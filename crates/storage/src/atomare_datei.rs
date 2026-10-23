//! Veröffentlichung einer vollständigen Datei im Zielverzeichnis. Fehler vor
//! dem Ersetzen erhalten die bisherige Datei; NamedTempFile räumt auch bei
//! vorzeitigem Fehler auf. Keine copy/delete-Ausweichlösung auf Netzlaufwerken.
use core_lib::{CoreError, CoreResult};
use std::{io::Write, path::PathBuf};
pub async fn ersetzen(ziel: PathBuf, daten: bytes::Bytes) -> CoreResult<()> {
    tokio::task::spawn_blocking(move || schreiben(ziel, |f| f.write_all(&daten)))
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?
}
fn schreiben(
    ziel: PathBuf,
    schreiben: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> CoreResult<()> {
    let parent = ziel
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut builder = tempfile::Builder::new();
    builder.prefix(".bauplan-write-").suffix(".tmp");
    // NAS-Dateien behalten die üblichen Verzeichnis-/Umaskrechte. Private
    // Druckartefakte verwenden getrennt die restriktiven Tempfile-Vorgaben.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o666));
    }
    let mut tmp = builder.tempfile_in(parent).map_err(err)?;
    match std::fs::symlink_metadata(&ziel) {
        Ok(m) if m.is_file() => tmp
            .as_file()
            .set_permissions(m.permissions())
            .map_err(err)?,
        Ok(_) => {
            return Err(CoreError::Storage(
                "Exportziel ist keine reguläre Datei".into(),
            ));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(err(e)),
    }
    schreiben(tmp.as_file_mut()).map_err(err)?;
    tmp.as_file().sync_all().map_err(err)?;
    tmp.persist(&ziel).map_err(|e| err(e.error))?;
    Ok(())
}
/// Reservierter temporärer Namensraum, der nie als DMS-Objekt veröffentlicht wird.
pub(crate) fn ist_temporaer(name: &str) -> bool {
    name.starts_with(".bauplan-write-") && name.ends_with(".tmp")
}
fn err(e: std::io::Error) -> CoreError {
    CoreError::Storage(format!("Datei sicher schreiben: {e}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ersetzt_vollstaendig_und_erhaelt_alte_datei_bei_fehler() {
        let dir = tempfile::tempdir().unwrap();
        let ziel = dir.path().join("a.pdf");
        ersetzen(ziel.clone(), bytes::Bytes::from_static(b"alt"))
            .await
            .unwrap();
        assert!(
            schreiben(ziel.clone(), |f| {
                f.write_all(b"halb")?;
                Err(std::io::Error::other("Datentraeger voll"))
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&ziel).unwrap(), b"alt");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        ersetzen(ziel.clone(), bytes::Bytes::from_static(b"neu"))
            .await
            .unwrap();
        assert_eq!(std::fs::read(ziel).unwrap(), b"neu");
    }
    #[tokio::test]
    async fn gesperrtes_zielverzeichnis_hinterlaesst_keine_tempdatei() {
        let dir = tempfile::tempdir().unwrap();
        let ziel = dir.path().join("ordner");
        std::fs::create_dir(&ziel).unwrap();
        assert!(
            ersetzen(ziel, bytes::Bytes::from_static(b"PDF"))
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
