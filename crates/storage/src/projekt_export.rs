//! Exportansicht mit lesbaren Projektordnern. Bestehende fremde/geänderte Dateien
//! werden niemals überschrieben; aktuelle DMS-Versionen bleiben unverändert.
use core_lib::{CoreError, CoreResult, storage::ContentHash};
use std::path::Path;
use tokio::io::AsyncWriteExt;
pub async fn unveraenderlich_schreiben(
    root: &Path,
    relative: &str,
    data: &[u8],
    hash: &ContentHash,
) -> CoreResult<bool> {
    let key = core_lib::storage::StorageKey::new(relative)?;
    hash.verify(data)?;
    let mut parent = root.to_path_buf();
    tokio::fs::create_dir_all(&parent)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    let parts: Vec<_> = key.segments().collect();
    for segment in parts.iter().take(parts.len() - 1) {
        parent.push(segment);
        match tokio::fs::symlink_metadata(&parent).await {
            Ok(m) if m.is_symlink() || !m.is_dir() => return Err(CoreError::Storage(
                "Projektordner enthält einen Link oder eine Datei anstelle eines Verzeichnisses"
                    .into(),
            )),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => tokio::fs::create_dir(&parent)
                .await
                .map_err(|e| CoreError::Storage(e.to_string()))?,
            Err(e) => return Err(CoreError::Storage(e.to_string())),
        }
    }
    let target = parent.join(
        parts
            .last()
            .ok_or_else(|| CoreError::InvalidInput("Dateiname fehlt".into()))?,
    );
    if tokio::fs::symlink_metadata(&target).await.is_ok() {
        let meta = tokio::fs::symlink_metadata(&target)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        if meta.is_symlink() || !meta.is_file() {
            return Err(CoreError::Storage(
                "Exportziel ist keine reguläre Datei".into(),
            ));
        }
        let old = tokio::fs::read(&target)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        hash.verify(&old).map_err(|_| {
            CoreError::Storage(format!(
                "{} wurde außerhalb der Anwendung verändert; wird nicht überschrieben",
                target.display()
            ))
        })?;
        return Ok(false);
    }
    // Temp + hard_link publishes only a complete file, with create-new semantics.
    let tmp = parent.join(format!(".export-{}.tmp", uuid::Uuid::new_v4()));
    let result = async {
        let mut f = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        f.write_all(data)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        f.sync_all()
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        drop(f);
        if tokio::fs::hard_link(&tmp, &target).await.is_err() {
            // Some removable drives and SMB shares do not support hard links.
            // create_new still guarantees that another file is never overwritten.
            exklusiv_schreiben(&target, data).await?;
        }
        Ok(true)
    }
    .await;
    let _ = tokio::fs::remove_file(tmp).await;
    result
}
async fn exklusiv_schreiben(target: &Path, data: &[u8]) -> CoreResult<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .await
        .map_err(|e| CoreError::Storage(format!("Exportdatei exklusiv anlegen: {e}")))?;
    let result = async {
        file.write_all(data).await?;
        file.sync_all().await
    }
    .await;
    drop(file);
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(target).await;
        return Err(CoreError::Storage(format!("Exportdatei schreiben: {e}")));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn portabler_export_fallback_ueberschreibt_nichts() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("export.pdf");
        exklusiv_schreiben(&target, b"PDF").await.unwrap();
        assert!(exklusiv_schreiben(&target, b"Andere Daten").await.is_err());
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"PDF");
    }
    #[tokio::test]
    async fn wiederholung_und_fremde_aenderung() {
        let root = tempfile::tempdir().unwrap();
        let hash = ContentHash::of(b"PDF");
        assert!(
            unveraenderlich_schreiben(root.path(), "Projekt/Dokumente/a.pdf", b"PDF", &hash)
                .await
                .unwrap()
        );
        assert!(
            !unveraenderlich_schreiben(root.path(), "Projekt/Dokumente/a.pdf", b"PDF", &hash)
                .await
                .unwrap()
        );
        std::fs::write(root.path().join("Projekt/Dokumente/a.pdf"), b"Andere Daten").unwrap();
        assert!(
            unveraenderlich_schreiben(root.path(), "Projekt/Dokumente/a.pdf", b"PDF", &hash)
                .await
                .is_err()
        );
        assert!(
            unveraenderlich_schreiben(root.path(), "../ausserhalb.pdf", b"PDF", &hash)
                .await
                .is_err()
        );
    }
}
