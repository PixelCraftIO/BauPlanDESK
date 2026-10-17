//! Integritätsgeprüfte Sicherungsarchive. Wiederherstellung ausschließlich in ein neues Verzeichnis.
use core_lib::{
    CoreError, CoreResult,
    storage::{ContentHash, StorageKey},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
#[derive(Debug, Clone)]
pub struct Sicherungsquelle {
    pub name: String,
    pub pfad: PathBuf,
    pub erwartet: Option<ContentHash>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sicherungsdatei {
    pub name: String,
    pub sha256: ContentHash,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sicherungsmanifest {
    pub schema_version: u32,
    pub erstellt_unix: u64,
    pub dateien: Vec<Sicherungsdatei>,
}
fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(e.to_string())
}
fn name_pruefen(name: &str) -> CoreResult<()> {
    StorageKey::new(name)?;
    let reserviert = name.split('/').any(|s| {
        let basis = s.split('.').next().unwrap_or(s).to_ascii_uppercase();
        matches!(basis.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|p| {
                basis.strip_prefix(p).is_some_and(|s| {
                    matches!(s, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                })
            })
    });
    if reserviert
        || name.contains(':')
        || name == "manifest.json"
        || name.split('/').any(|s| s.ends_with([' ', '.']))
    {
        return Err(CoreError::InvalidInput(
            "Nicht portabler Sicherungspfad".into(),
        ));
    }
    Ok(())
}
fn neu(p: &Path) -> CoreResult<File> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(p).map_err(err)
}
fn kopieren(
    mut quelle: impl Read,
    mut ziel: impl Write,
    max: u64,
) -> CoreResult<(u64, ContentHash)> {
    let mut hash = Sha256::new();
    let mut summe = 0u64;
    let mut buf = [0u8; 128 * 1024];
    loop {
        let n = quelle.read(&mut buf).map_err(err)?;
        if n == 0 {
            break;
        }
        summe = summe
            .checked_add(n as u64)
            .ok_or_else(|| err("Datei zu groß"))?;
        if summe > max {
            return Err(err("Sicherungsdatei überschreitet die deklarierte Größe"));
        }
        hash.update(&buf[..n]);
        ziel.write_all(&buf[..n]).map_err(err)?;
    }
    Ok((
        summe,
        ContentHash::try_from(format!("{:x}", hash.finalize()))?,
    ))
}
pub fn verzeichnis(root: &Path, prefix: &str) -> CoreResult<Vec<Sicherungsquelle>> {
    fn walk(
        root: &Path,
        pfad: &Path,
        prefix: &str,
        out: &mut Vec<Sicherungsquelle>,
    ) -> CoreResult<()> {
        let m = fs::symlink_metadata(pfad).map_err(err)?;
        if m.is_symlink() {
            return Err(err(format!(
                "Verknüpfung wird nicht gesichert: {}",
                pfad.display()
            )));
        }
        if m.is_dir() {
            for e in fs::read_dir(pfad).map_err(err)? {
                walk(root, &e.map_err(err)?.path(), prefix, out)?;
            }
        } else if m.is_file() {
            let rel = pfad
                .strip_prefix(root)
                .map_err(err)?
                .to_str()
                .ok_or_else(|| err("Dateiname ist nicht UTF-8"))?
                .replace('\\', "/");
            let name = format!("{prefix}/{rel}");
            name_pruefen(&name)?;
            out.push(Sicherungsquelle {
                name,
                pfad: pfad.into(),
                erwartet: None,
            });
        } else {
            return Err(err("Sicherung enthält keine reguläre Datei"));
        }
        if out.len() > 100_000 {
            return Err(err("Sicherung enthält mehr als 100000 Dateien"));
        }
        Ok(())
    }
    let mut out = vec![];
    walk(root, root, prefix, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
pub fn erstellen(ziel: &Path, quellen: &[Sicherungsquelle]) -> CoreResult<Sicherungsmanifest> {
    if ziel.exists() {
        return Err(err(
            "Sicherungsziel existiert bereits; bitte einen neuen Dateinamen wählen",
        ));
    }
    if quellen.len() > 100000 {
        return Err(err("Sicherung enthält mehr als 100000 Dateien"));
    }
    let parent = ziel.parent().ok_or_else(|| err("Zielverzeichnis fehlt"))?;
    fs::create_dir_all(parent).map_err(err)?;
    let tmp = parent.join(format!(".sicherung-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut zip = ZipWriter::new(neu(&tmp)?);
        let optionen =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut ids = HashSet::new();
        let mut manifest = Sicherungsmanifest {
            schema_version: 1,
            erstellt_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_secs(),
            dateien: vec![],
        };
        for q in quellen {
            name_pruefen(&q.name)?;
            if !ids.insert(&q.name) {
                return Err(err("Doppelter Sicherungspfad"));
            }
            let m = fs::symlink_metadata(&q.pfad).map_err(err)?;
            if !m.is_file() || m.is_symlink() {
                return Err(err("Sicherungsquelle ist keine reguläre Datei"));
            }
            zip.start_file(&q.name, optionen).map_err(err)?;
            let (bytes, sha256) = kopieren(File::open(&q.pfad).map_err(err)?, &mut zip, m.len())?;
            if bytes != m.len() || q.erwartet.as_ref().is_some_and(|h| h != &sha256) {
                return Err(err(format!(
                    "Sicherungsquelle hat sich geändert: {}",
                    q.name
                )));
            }
            manifest.dateien.push(Sicherungsdatei {
                name: q.name.clone(),
                sha256,
                bytes,
            });
        }
        zip.start_file("manifest.json", optionen).map_err(err)?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest).map_err(err)?)
            .map_err(err)?;
        zip.finish().map_err(err)?.sync_all().map_err(err)?;
        if fs::hard_link(&tmp, ziel).is_err() {
            let mut out = neu(ziel)?;
            let result = std::io::copy(&mut File::open(&tmp).map_err(err)?, &mut out)
                .and_then(|_| out.sync_all());
            if let Err(e) = result {
                let _ = fs::remove_file(ziel);
                return Err(err(e));
            }
        }
        Ok(manifest)
    })();
    let _ = fs::remove_file(tmp);
    result
}
fn manifest(zip: &mut ZipArchive<File>) -> CoreResult<Sicherungsmanifest> {
    let mut raw = Vec::new();
    zip.by_name("manifest.json")
        .map_err(err)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(err)?;
    if raw.len() > 8 * 1024 * 1024 {
        return Err(err("Sicherungsmanifest ist zu groß"));
    }
    let m: Sicherungsmanifest = serde_json::from_slice(&raw).map_err(err)?;
    if m.schema_version != 1 || m.dateien.len() > 100_000 || zip.len() != m.dateien.len() + 1 {
        return Err(err("Unbekanntes oder unvollständiges Sicherungsformat"));
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for f in &m.dateien {
        name_pruefen(&f.name)?;
        if !names.insert(&f.name) {
            return Err(err("Doppelte Manifestdatei"));
        }
        total = total
            .checked_add(f.bytes)
            .ok_or_else(|| err("Sicherung zu groß"))?;
        if total > 1024u64.pow(4) {
            return Err(err("Sicherung überschreitet 1 TB"));
        }
    }
    let mut archive_names = HashSet::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(err)?;
        if !archive_names.insert(f.name().map_err(err)?.into_owned()) {
            return Err(err("Doppelte ZIP-Datei"));
        }
    }
    Ok(m)
}
pub fn pruefen(pfad: &Path) -> CoreResult<Sicherungsmanifest> {
    let mut zip = ZipArchive::new(File::open(pfad).map_err(err)?).map_err(err)?;
    let m = manifest(&mut zip)?;
    for f in &m.dateien {
        let entry = zip.by_name(&f.name).map_err(err)?;
        if entry.size() != f.bytes {
            return Err(err("Sicherungsgröße stimmt nicht"));
        }
        let (bytes, hash) = kopieren(entry, std::io::sink(), f.bytes)?;
        if bytes != f.bytes || hash != f.sha256 {
            return Err(err(format!(
                "Integritätsprüfung fehlgeschlagen: {}",
                f.name
            )));
        }
    }
    Ok(m)
}
pub fn wiederherstellen(pfad: &Path, ziel: &Path) -> CoreResult<Sicherungsmanifest> {
    if ziel.exists() {
        return Err(err(
            "Wiederherstellungsziel muss neu sein. Vorhandene Daten werden nicht überschrieben",
        ));
    }
    let parent = ziel.parent().ok_or_else(|| err("Zielverzeichnis fehlt"))?;
    fs::create_dir_all(parent).map_err(err)?;
    let tmp = parent.join(format!(".wiederherstellung-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&tmp).map_err(err)?;
    let result = (|| {
        let mut zip = ZipArchive::new(File::open(pfad).map_err(err)?).map_err(err)?;
        let m = manifest(&mut zip)?;
        for f in &m.dateien {
            let target = tmp.join(&f.name);
            fs::create_dir_all(target.parent().ok_or_else(|| err("Verzeichnis fehlt"))?)
                .map_err(err)?;
            let mut out = neu(&target)?;
            let entry = zip.by_name(&f.name).map_err(err)?;
            if entry.size() != f.bytes {
                return Err(err("Sicherungsgröße stimmt nicht"));
            }
            let (bytes, hash) = kopieren(entry, &mut out, f.bytes)?;
            out.sync_all().map_err(err)?;
            if bytes != f.bytes || hash != f.sha256 {
                return Err(err(format!(
                    "Integritätsprüfung fehlgeschlagen: {}",
                    f.name
                )));
            }
        }
        // Publication happens only after every file has been verified.
        if ziel.exists() {
            return Err(err(
                "Wiederherstellungsziel wurde zwischenzeitlich angelegt",
            ));
        }
        fs::rename(&tmp, ziel).map_err(err)?;
        Ok(m)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(tmp);
    }
    result
}
