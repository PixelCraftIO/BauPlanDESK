//! Arbeitsplatzbezogene, bearbeitbare Kopien mit dauerhaft gespeicherter Basis.
use core_lib::{
    CoreError, CoreResult,
    dateiverwaltung::Arbeitsstand,
    domain::DocumentVersion,
    storage::{ContentHash, StorageKey},
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize)]
pub struct Arbeitskopie {
    pub stand: Arbeitsstand,
    pub pfad: PathBuf,
    pub geaendert: bool,
}
fn err(e: std::io::Error) -> CoreError {
    CoreError::Storage(e.to_string())
}
fn dateiname(d: &DocumentVersion) -> CoreResult<&str> {
    let key = StorageKey::new(d.file_name.clone())?;
    if key.segments().count() != 1 || d.file_name.contains(':') {
        return Err(CoreError::InvalidInput(
            "Ungültiger Arbeitsdateiname".into(),
        ));
    }
    Ok(&d.file_name)
}
pub async fn erstellen(
    root: &Path,
    dokument: DocumentVersion,
    pdf: &[u8],
) -> CoreResult<Arbeitskopie> {
    if pdf.len() > 512 * 1024 * 1024 {
        return Err(CoreError::InvalidInput(
            "Arbeitsdatei ist größer als 512 MB".into(),
        ));
    }
    dokument.sha256.verify(pdf)?;
    let name = dateiname(&dokument)?.to_string();
    let stand = Arbeitsstand {
        id: Uuid::new_v4(),
        dokument,
    };
    let dir = root.join(stand.id.to_string());
    super::projekt_export::unveraenderlich_schreiben(&dir, &name, pdf, &stand.dokument.sha256)
        .await?;
    let data = serde_json::to_vec_pretty(&stand).map_err(|e| CoreError::Storage(e.to_string()))?;
    super::projekt_export::unveraenderlich_schreiben(
        &dir,
        ".bauprojekt-arbeitskopie.json",
        &data,
        &ContentHash::of(&data),
    )
    .await?;
    laden(root, stand.id).await
}
pub async fn laden(root: &Path, id: Uuid) -> CoreResult<Arbeitskopie> {
    let dir = root.join(id.to_string());
    let bytes = tokio::fs::read(dir.join(".bauprojekt-arbeitskopie.json"))
        .await
        .map_err(err)?;
    let stand: Arbeitsstand =
        serde_json::from_slice(&bytes).map_err(|e| CoreError::Storage(e.to_string()))?;
    if stand.id != id {
        return Err(CoreError::InvalidInput(
            "Arbeitskopie hat eine ungültige Kennung".into(),
        ));
    }
    let pfad = dir.join(dateiname(&stand.dokument)?);
    let data = inhalt(&pfad).await?;
    let geaendert = ContentHash::of(&data) != stand.dokument.sha256;
    Ok(Arbeitskopie {
        stand,
        pfad,
        geaendert,
    })
}
pub async fn inhalt(pfad: &Path) -> CoreResult<Vec<u8>> {
    let meta = tokio::fs::symlink_metadata(pfad).await.map_err(err)?;
    if !meta.is_file() || meta.is_symlink() || meta.len() > 512 * 1024 * 1024 {
        return Err(CoreError::InvalidInput(
            "Arbeitsdatei muss eine reguläre Datei bis 512 MB sein".into(),
        ));
    }
    tokio::fs::read(pfad).await.map_err(err)
}
pub async fn auflisten(
    root: &Path,
    projekt: core_lib::domain::ProjectId,
) -> CoreResult<Vec<Arbeitskopie>> {
    tokio::fs::create_dir_all(root).await.map_err(err)?;
    let mut entries = tokio::fs::read_dir(root).await.map_err(err)?;
    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(err)? {
        if !entry.file_type().await.map_err(err)?.is_dir() {
            continue;
        }
        let Some(id) = entry
            .file_name()
            .to_str()
            .and_then(|s| Uuid::parse_str(s).ok())
        else {
            continue;
        };
        let copy = laden(root, id).await?;
        if copy.stand.dokument.project_id == projekt {
            result.push(copy);
        }
    }
    result.sort_by_key(|c| (c.stand.dokument.file_name.clone(), c.stand.id));
    Ok(result)
}
/// DMS-Commit und lokale Basis sind getrennte Grenzen: Eine neue Basis wird erst
/// nach erfolgreichem Commit atomar veröffentlicht. Die Arbeitsdatei bleibt frei
/// bearbeitbar, auch wenn sie inzwischen erneut geändert wurde.
pub async fn basis_aktualisieren(
    root: &Path,
    id: Uuid,
    dokument: DocumentVersion,
) -> CoreResult<()> {
    let alt = laden(root, id).await?;
    if alt.stand.dokument.document_id != dokument.document_id
        || alt.stand.dokument.project_id != dokument.project_id
        || alt.stand.dokument.file_name != dokument.file_name
        || dokument.version <= alt.stand.dokument.version
    {
        return Err(CoreError::InvalidInput(
            "Neue Basis gehört nicht zur Arbeitskopie".into(),
        ));
    }
    let stand = Arbeitsstand { id, dokument };
    let metadata = root
        .join(id.to_string())
        .join(".bauprojekt-arbeitskopie.json");
    let temp = metadata.with_extension(format!("{}.tmp", Uuid::new_v4()));
    tokio::fs::write(
        &temp,
        serde_json::to_vec_pretty(&stand).map_err(|e| CoreError::Storage(e.to_string()))?,
    )
    .await
    .map_err(err)?;
    if let Err(e) = tokio::fs::rename(&temp, &metadata).await {
        let _ = tokio::fs::remove_file(temp).await;
        return Err(err(e));
    }
    Ok(())
}
