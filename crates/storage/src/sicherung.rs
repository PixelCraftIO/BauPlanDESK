//! Integritätsgeprüfte Sicherungsarchive. Wiederherstellung ausschließlich in ein neues Verzeichnis.
//!
//! Archivnamen sind immer portabel. Dateien, deren Name unter Windows oder in
//! ZIP-Archiven nicht zulässig ist (z. B. ältere DMS-Schlüssel mit `:` oder
//! Punkt am Ende), werden im Archiv prozentkodiert abgelegt
//! (Manifest-Schema 2) und beim Wiederherstellen unter ihrem ursprünglichen
//! Namen geschrieben. Der Abgleich mit der Datenbank erfolgt über den
//! ursprünglichen Storage-Schlüssel; es geht keine Datei verloren.
use core_lib::{
    CoreError, CoreResult,
    repository::Repositories,
    storage::{ContentHash, StorageKey},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
#[derive(Debug, Clone)]
pub struct Sicherungsquelle {
    /// Ursprünglicher relativer Name, z. B. `dateien/data/projekte/…`. Ein
    /// nicht portabler Name wird erst beim Archivieren kodiert.
    pub name: String,
    pub pfad: PathBuf,
    pub erwartet: Option<ContentHash>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sicherungsdatei {
    /// Name im Archiv; ab Schema 2 mit [`archivname`] kodiert.
    pub name: String,
    pub sha256: ContentHash,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sicherungsmanifest {
    /// 1: Archivnamen sind die ursprünglichen Namen. 2: Archivnamen sind mit
    /// [`archivname`] kodiert. Archive ohne kodierungsbedürftige Namen werden
    /// weiterhin mit Schema 1 geschrieben und bleiben für ältere Versionen lesbar.
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
/// Zeichen, die im Archivnamen als `%XX` stehen. `%` selbst gehört dazu,
/// damit die Kodierung umkehrbar bleibt.
const KODIERT: &[char] = &['%', ':', '*', '?', '"', '<', '>', '|'];
fn hex(c: char, out: &mut String) {
    out.push_str(&format!("%{:02X}", c as u32));
}
/// Portabler Archivname: kodiert unzulässige Zeichen, Punkt/Leerzeichen am
/// Segmentende und reservierte Gerätenamen. Für portable Namen ohne `%`
/// ist das Ergebnis unverändert.
pub fn archivname(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, segment) in name.split('/').enumerate() {
        if i > 0 {
            out.push('/');
        }
        let reserviert = core_lib::dateiname::reservierter_name(segment);
        let letzter = segment.char_indices().last().map(|(p, _)| p);
        for (p, c) in segment.char_indices() {
            let kodieren = KODIERT.contains(&c)
                || c.is_control()
                || (reserviert && p == 0)
                || (Some(p) == letzter && matches!(c, ' ' | '.'));
            if kodieren && c.is_ascii() {
                hex(c, &mut out);
            } else {
                out.push(c);
            }
        }
    }
    out
}
/// Umkehrung von [`archivname`]. Nur die von [`archivname`] erzeugten
/// Kodierungen werden akzeptiert, insbesondere kein kodierter Pfadtrenner.
pub fn originalname(archiv: &str) -> CoreResult<String> {
    let mut out = String::with_capacity(archiv.len());
    let mut it = archiv.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let code: String = it.by_ref().take(2).collect();
        let zeichen = (code.len() == 2 && code.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| u8::from_str_radix(&code, 16).ok())
            .flatten()
            .map(char::from)
            .filter(|c| *c != '/' && *c != '\\' && c.is_ascii())
            .ok_or_else(|| err("Ungültige Kodierung im Sicherungspfad"))?;
        out.push(zeichen);
    }
    StorageKey::new(&out)?;
    Ok(out)
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
            // Nur Pfadsicherheit; nicht portable Namen kodiert `erstellen`.
            StorageKey::new(&name)?;
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
/// Unterverzeichnisse einer [`crate::LocalStorage`]-Ablage, die gesichert
/// werden. `tmp/` enthält nur Zwischendateien laufender Speichervorgänge und
/// wird ausgelassen, damit gleichzeitiges Arbeiten die Sicherung nicht abbricht.
pub const ABLAGE_INHALT: [&str; 2] = ["data", "meta"];

/// Sicherungsquellen einer Dateiablage im Layout von [`crate::LocalStorage`]
/// (`data/<schlüssel>`, `meta/<schlüssel>.json`) unter `prefix/data/…` bzw.
/// `prefix/meta/…`. Jeder Storage-Schlüssel aus `erwartet` muss als Datei
/// vorliegen; sein SHA-256 wird beim Archivieren geprüft. Fehlende Referenzen
/// sind ein Fehler, damit keine unvollständige Sicherung entsteht.
pub fn dateiablage(
    root: &Path,
    prefix: &str,
    mut erwartet: BTreeMap<String, ContentHash>,
) -> CoreResult<Vec<Sicherungsquelle>> {
    let mut quellen = Vec::new();
    for teil in ABLAGE_INHALT {
        let pfad = root.join(teil);
        if pfad.is_dir() {
            quellen.extend(verzeichnis(&pfad, &format!("{prefix}/{teil}"))?);
        }
    }
    quellen.retain(|q| {
        !q.pfad
            .file_name()
            .is_some_and(|n| crate::atomare_datei::ist_temporaer(&n.to_string_lossy()))
    });
    let daten = format!("{prefix}/data/");
    for q in &mut quellen {
        if let Some(schluessel) = q.name.strip_prefix(&daten)
            && let Some(h) = erwartet.remove(schluessel)
        {
            q.erwartet = Some(h);
        }
    }
    if let Some(erster) = erwartet.keys().next() {
        return Err(err(format!(
            "{} Datenbankreferenzen fehlen in der Dateiablage (z. B. {erster}). Die Sicherung wird nicht als vollständig ausgegeben.",
            erwartet.len()
        )));
    }
    Ok(quellen)
}

/// Storage-Schlüssel und SHA-256 aller Dateien, auf die die Datenbank
/// verweist: sämtliche Dokumentversionen, die inhaltsadressierten
/// Formularquellen der Formularinstanzen und Einreichungspakete.
pub async fn dms_referenzen(repos: &Repositories) -> CoreResult<BTreeMap<String, ContentHash>> {
    let mut erwartet = BTreeMap::new();
    for p in repos.projects.list().await? {
        for d in repos.documents.list_for_project(p.id).await? {
            erwartet.insert(d.storage_key.clone(), d.sha256.clone());
            if let Some(z) = repos
                .commit
                .dokumentzusatz(d.document_id, d.version, "formularinstanz")
                .await?
            {
                if z.schema_version != 1 {
                    return Err(CoreError::InvalidInput(
                        "Unbekannte Formularinstanzversion; Sicherung mit neuerer Anwendung erstellen"
                            .into(),
                    ));
                }
                let i: core_lib::formular::instanz::FormularInstanz =
                    serde_json::from_value(z.daten)
                        .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
                erwartet.insert(i.vorlagen_storage_key, i.vorlagen_hash);
            }
        }
        for e in repos.einreichungen.auflisten(p.id).await? {
            erwartet.insert(e.zip_key, e.zip_hash);
        }
    }
    Ok(erwartet)
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
        let namen: Vec<String> = quellen.iter().map(|q| archivname(&q.name)).collect();
        let kodiert = quellen.iter().zip(&namen).any(|(q, n)| &q.name != n);
        let mut manifest = Sicherungsmanifest {
            schema_version: if kodiert { 2 } else { 1 },
            erstellt_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_secs(),
            dateien: vec![],
        };
        for (q, name) in quellen.iter().zip(&namen) {
            StorageKey::new(&q.name)?;
            name_pruefen(name)?;
            if !ids.insert(name.as_str()) {
                return Err(err("Doppelter Sicherungspfad"));
            }
            let m = fs::symlink_metadata(&q.pfad).map_err(err)?;
            if !m.is_file() || m.is_symlink() {
                return Err(err("Sicherungsquelle ist keine reguläre Datei"));
            }
            zip.start_file(name.as_str(), optionen).map_err(err)?;
            let (bytes, sha256) = kopieren(File::open(&q.pfad).map_err(err)?, &mut zip, m.len())?;
            if bytes != m.len() || q.erwartet.as_ref().is_some_and(|h| h != &sha256) {
                return Err(err(format!(
                    "Sicherungsquelle hat sich geändert: {}",
                    q.name
                )));
            }
            manifest.dateien.push(Sicherungsdatei {
                name: name.clone(),
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
    if !matches!(m.schema_version, 1 | 2)
        || m.dateien.len() > 100_000
        || zip.len() != m.dateien.len() + 1
    {
        return Err(err("Unbekanntes oder unvollständiges Sicherungsformat"));
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for f in &m.dateien {
        name_pruefen(&f.name)?;
        if m.schema_version == 2 {
            originalname(&f.name)?;
        }
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
            let original = if m.schema_version == 2 {
                originalname(&f.name)?
            } else {
                f.name.clone()
            };
            // Unter Windows lässt sich ein nicht portabler Name nicht anlegen
            // (z. B. würde `:` einen alternativen Datenstrom erzeugen).
            if cfg!(windows) && original != f.name {
                return Err(err(format!(
                    "{original} kann unter Windows nicht wiederhergestellt werden. Sicherung auf Linux/macOS wiederherstellen."
                )));
            }
            let target = original
                .split('/')
                .fold(tmp.clone(), |p, segment| p.join(segment));
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
