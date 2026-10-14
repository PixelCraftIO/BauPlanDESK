//! Lesbarer PDF-Ordner, lokal oder eingebundene Freigabe. Kein Projekt-Fremdschlüssel
//! und kein privater Katalog: direkt abgelegte PDFs sind ebenfalls sichtbar.
use core_lib::{
    CoreError, CoreResult, formular::bibliothek::BibliotheksVorlage, storage::ContentHash,
};
use std::path::Path;

const MAX_BYTES: u64 = 100 * 1024 * 1024;
fn fehler(e: std::io::Error) -> CoreError {
    CoreError::Storage(e.to_string())
}
fn name_pruefen(name: &str) -> CoreResult<()> {
    if name.is_empty()
        || name.contains(['/', '\\', ':'])
        || name.chars().any(char::is_control)
        || !name.to_ascii_lowercase().ends_with(".pdf")
    {
        return Err(CoreError::InvalidInput(
            "Erwartet wird ein PDF-Dateiname ohne Verzeichnisse".into(),
        ));
    }
    Ok(())
}
pub async fn ordner_pruefen(root: &Path) -> CoreResult<()> {
    if !root.is_absolute() {
        return Err(CoreError::InvalidInput(
            "Formularordner muss ein absoluter Pfad sein".into(),
        ));
    }
    tokio::fs::create_dir_all(root).await.map_err(fehler)?;
    let _ = tokio::fs::read_dir(root).await.map_err(fehler)?;
    Ok(())
}
pub async fn lesen(root: &Path, name: &str, stand: Option<&ContentHash>) -> CoreResult<Vec<u8>> {
    name_pruefen(name)?;
    let path = root.join(name);
    let meta = tokio::fs::symlink_metadata(&path).await.map_err(fehler)?;
    if !meta.is_file() || meta.is_symlink() || meta.len() > MAX_BYTES {
        return Err(CoreError::InvalidInput(
            "Vorlage muss eine reguläre PDF-Datei bis 100 MB sein".into(),
        ));
    }
    let pdf = tokio::fs::read(path).await.map_err(fehler)?;
    if pdf.len() as u64 > MAX_BYTES {
        return Err(CoreError::InvalidInput("PDF ist größer als 100 MB".into()));
    }
    if let Some(hash) = stand {
        hash.verify(&pdf)?;
    }
    Ok(pdf)
}
pub async fn auflisten(root: &Path) -> CoreResult<Vec<BibliotheksVorlage>> {
    ordner_pruefen(root).await?;
    let mut result = Vec::new();
    let mut entries = tokio::fs::read_dir(root).await.map_err(fehler)?;
    while let Some(entry) = entries.next_entry().await.map_err(fehler)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.file_type().await.map_err(fehler)?;
        if !meta.is_file() || !name.to_ascii_lowercase().ends_with(".pdf") {
            continue;
        }
        let pdf = lesen(root, &name, None).await?;
        result.push(BibliotheksVorlage {
            name,
            sha256: ContentHash::of(&pdf),
            size: pdf.len() as u64,
        });
    }
    result.sort_by_key(|a| a.name.to_lowercase());
    Ok(result)
}
/// Neue Datei veröffentlichen; gleichnamige Vorlagen niemals ersetzen.
pub async fn ablegen(root: &Path, name: &str, pdf: &[u8]) -> CoreResult<BibliotheksVorlage> {
    name_pruefen(name)?;
    ordner_pruefen(root).await?;
    if pdf.len() as u64 > MAX_BYTES {
        return Err(CoreError::InvalidInput("PDF ist größer als 100 MB".into()));
    }
    let hash = ContentHash::of(pdf);
    let name = match tokio::fs::symlink_metadata(root.join(name)).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => name.to_string(),
        Ok(_) => format!(
            "{}--{}.pdf",
            name[..name.len() - 4].chars().take(100).collect::<String>(),
            uuid::Uuid::new_v4()
        ),
        Err(e) => return Err(fehler(e)),
    };
    super::projekt_export::unveraenderlich_schreiben(root, &name, pdf, &hash).await?;
    Ok(BibliotheksVorlage {
        name,
        sha256: hash,
        size: pdf.len() as u64,
    })
}
