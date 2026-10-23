//! Nicht destruktiver Abgleich von DMS-Dateien, Sidecars und Datenbankreferenzen.
//! Unbekannte Dateien werden gemeldet, niemals automatisch gelöscht.
use core_lib::{CoreError, CoreResult, storage::ContentHash};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Serialize)]
pub struct Befund {
    pub datei: String,
    pub art: String,
    pub hinweis: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Bericht {
    pub geprueft: usize,
    pub befunde: Vec<Befund>,
}
#[derive(Serialize, Deserialize)]
struct Sidecar {
    size_bytes: u64,
    sha256: ContentHash,
}
fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(e.to_string())
}
fn dateien(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    befunde: &mut Vec<Befund>,
) -> CoreResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(dir).map_err(err)?.is_symlink() {
        befunde.push(Befund {
            datei: dir.strip_prefix(root).unwrap_or(dir).display().to_string(),
            art: "link".into(),
            hinweis: "Verknüpfung wird nicht verfolgt".into(),
        });
        return Ok(());
    }
    for e in fs::read_dir(dir).map_err(err)? {
        let e = e.map_err(err)?;
        let typ = e.file_type().map_err(err)?;
        if typ.is_symlink() {
            befunde.push(Befund {
                datei: e
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&e.path())
                    .display()
                    .to_string(),
                art: "link".into(),
                hinweis: "Verknüpfung wird nicht verfolgt".into(),
            });
        } else if typ.is_dir() {
            dateien(root, &e.path(), out, befunde)?;
        } else if typ.is_file() {
            out.push(e.path());
        }
    }
    Ok(())
}
fn hash(path: &Path) -> CoreResult<(u64, ContentHash)> {
    let mut file = fs::File::open(path).map_err(err)?;
    let mut h = Sha256::new();
    let mut size = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(err)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        size += n as u64;
    }
    Ok((size, ContentHash::try_from(format!("{:x}", h.finalize()))?))
}
pub fn pruefen(root: &Path, referenzen: &BTreeMap<String, ContentHash>) -> CoreResult<Bericht> {
    let mut r = Bericht {
        geprueft: 0,
        befunde: vec![],
    };
    let mut data = vec![];
    dateien(root, &root.join("data"), &mut data, &mut r.befunde)?;
    let mut fehlt = referenzen.clone();
    for p in data {
        if p.file_name()
            .is_some_and(|n| crate::atomare_datei::ist_temporaer(&n.to_string_lossy()))
        {
            r.befunde.push(Befund{datei:p.strip_prefix(root).map_err(err)?.display().to_string(),art:"temporaer".into(),hinweis:"Temporäre Schreibdatei; laufende Vorgänge vor manueller Bereinigung ausschließen".into()});
            continue;
        }
        r.geprueft += 1;
        let key = p
            .strip_prefix(root.join("data"))
            .map_err(err)?
            .to_string_lossy()
            .replace('\\', "/");
        let erwartet = fehlt.remove(&key);
        let (size, h) = match hash(&p) {
            Ok(v) => v,
            Err(e) => {
                r.befunde.push(Befund {
                    datei: key,
                    art: "lesefehler".into(),
                    hinweis: e.to_string(),
                });
                continue;
            }
        };
        if erwartet.as_ref().is_some_and(|e| e != &h) {
            r.befunde.push(Befund {
                datei: key.clone(),
                art: "hashabweichung".into(),
                hinweis: "Inhalt weicht von der Datenbankreferenz ab. Sicherung prüfen.".into(),
            });
        }
        if erwartet.is_none() {
            r.befunde.push(Befund{datei:key.clone(),art:"ohne_referenz".into(),hinweis:"Keine Referenz im geprüften Datenbankstand. Kann zu laufender Speicherung oder weiteren Fachartefakten gehören; nicht automatisch löschen.".into()});
        }
        let meta = root.join("meta").join(format!("{key}.json"));
        match fs::read(&meta) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => r.befunde.push(Befund {
                datei: key,
                art: "metadaten_fehlen".into(),
                hinweis: "Datei vorhanden, Metadaten fehlen".into(),
            }),
            Err(e) => r.befunde.push(Befund {
                datei: key,
                art: "metadaten_lesefehler".into(),
                hinweis: e.to_string(),
            }),
            Ok(raw) => match serde_json::from_slice::<Sidecar>(&raw) {
                Ok(m) if m.size_bytes == size && m.sha256 == h => {}
                _ => r.befunde.push(Befund {
                    datei: key,
                    art: "metadaten_abweichung".into(),
                    hinweis: "Metadaten sind ungültig oder stimmen nicht zum Inhalt".into(),
                }),
            },
        }
    }
    for (key, _) in fehlt {
        r.befunde.push(Befund {
            datei: key,
            art: "datei_fehlt".into(),
            hinweis: "Referenzierte Datei fehlt. Sicherung prüfen.".into(),
        });
    }
    let mut meta = vec![];
    dateien(root, &root.join("meta"), &mut meta, &mut r.befunde)?;
    for p in meta {
        let key = p
            .strip_prefix(root.join("meta"))
            .map_err(err)?
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(key) = key.strip_suffix(".json")
            && !root.join("data").join(key).is_file()
        {
            r.befunde.push(Befund {
                datei: key.into(),
                art: "metadaten_ohne_datei".into(),
                hinweis: "Metadaten ohne zugehörige Datei".into(),
            });
        }
    }
    let mut tmp = vec![];
    dateien(root, &root.join("tmp"), &mut tmp, &mut r.befunde)?;
    for p in tmp {
        r.befunde.push(Befund {
            datei: p.strip_prefix(root).map_err(err)?.display().to_string(),
            art: "temporaer".into(),
            hinweis:
                "Temporäre Datei gefunden; laufende Vorgänge vor manueller Bereinigung ausschließen"
                    .into(),
        });
    }
    r.befunde
        .sort_by(|a, b| (&a.datei, &a.art).cmp(&(&b.datei, &b.art)));
    Ok(r)
}
fn ohne_links(root: &Path, relativ: &str) -> bool {
    let mut pfad = root.to_path_buf();
    for teil in relativ.split('/') {
        pfad.push(teil);
        match fs::symlink_metadata(&pfad) {
            Ok(m) if m.is_symlink() => return false,
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return false,
            _ => {}
        }
    }
    true
}
/// Rekonstruiert ausschließlich fehlende Sidecars referenzierter, hashgleicher
/// Dateien. Fremde/abweichende Metadaten und Inhalte bleiben unverändert.
pub async fn metadaten_ergaenzen(
    root: PathBuf,
    referenzen: BTreeMap<String, ContentHash>,
) -> CoreResult<usize> {
    let kandidaten = tokio::task::spawn_blocking(move || {
        let mut kandidaten = Vec::new();
        for (key, erwartet) in referenzen {
            core_lib::storage::StorageKey::new(key.clone())?;
            let meta = root.join("meta").join(format!("{key}.json"));
            if meta.exists()
                || !ohne_links(&root, &format!("data/{key}"))
                || !ohne_links(&root, &format!("meta/{key}.json"))
            {
                continue;
            }
            let datei = root.join("data").join(&key);
            if !datei.is_file() || fs::symlink_metadata(&datei).map_err(err)?.is_symlink() {
                continue;
            }
            let (size, h) = hash(&datei)?;
            if h != erwartet {
                continue;
            }
            let daten = serde_json::to_vec(&Sidecar {
                size_bytes: size,
                sha256: h,
            })
            .map_err(err)?;
            kandidaten.push((meta, daten));
        }
        Ok::<_, CoreError>(kandidaten)
    })
    .await
    .map_err(err)??;
    let mut n = 0;
    for (ziel, daten) in kandidaten {
        // create_new publication; eine inzwischen angelegte Metadatei nie ersetzen.
        let parent = ziel.parent().ok_or_else(|| err("Metadatenpfad fehlt"))?;
        let name = ziel
            .file_name()
            .ok_or_else(|| err("Metadatenname fehlt"))?
            .to_string_lossy();
        if ziel.exists() {
            continue;
        }
        let ergaenzt = crate::projekt_export::unveraenderlich_schreiben(
            parent,
            &name,
            &daten,
            &ContentHash::of(&daten),
        )
        .await?;
        if ergaenzt {
            n += 1;
        }
    }
    Ok(n)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fehlende_metadaten_reparieren_abweichende_datei_erhalten() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("data/a")).unwrap();
        fs::write(root.join("data/a/gut"), b"gut").unwrap();
        fs::write(root.join("data/a/falsch"), b"anders").unwrap();
        let refs = BTreeMap::from([
            ("a/gut".into(), ContentHash::of(b"gut")),
            ("a/falsch".into(), ContentHash::of(b"original")),
            ("fehlt".into(), ContentHash::of(b"x")),
        ]);
        let b = pruefen(root, &refs).unwrap();
        assert!(b.befunde.iter().any(|b| b.art == "datei_fehlt"));
        assert!(b.befunde.iter().any(|b| b.art == "hashabweichung"));
        assert_eq!(
            metadaten_ergaenzen(root.to_path_buf(), refs.clone())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            metadaten_ergaenzen(root.to_path_buf(), refs.clone())
                .await
                .unwrap(),
            0
        );
        assert!(!root.join("meta/a/falsch.json").exists());
        assert!(
            !pruefen(root, &refs)
                .unwrap()
                .befunde
                .iter()
                .any(|b| b.datei == "a/gut")
        );
        assert_eq!(fs::read(root.join("data/a/falsch")).unwrap(), b"anders");
    }
}
