//! Externe Programme erhalten Arbeitskopien; sie schreiben niemals DMS-Versionen.
use crate::{CoreError, CoreResult, domain::DocumentVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arbeitsstand {
    pub id: Uuid,
    pub dokument: DocumentVersion,
}
pub fn extern_oeffnen_erlaubt(name: &str) -> CoreResult<()> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if [
        "pdf", "dwg", "dxf", "xls", "xlsx", "xlsm", "xlsb", "doc", "docx", "docm", "odt", "ods",
        "ppt", "pptx", "odp", "txt", "csv", "rtf", "jpg", "jpeg", "png", "gif", "tif", "tiff",
        "bmp", "webp", "heic", "zip", "7z", "svg", "xml", "eml", "msg",
    ]
    .contains(&ext.as_str())
    {
        Ok(())
    } else {
        Err(CoreError::InvalidInput("Für diesen Dateityp ist kein direktes Öffnen vorgesehen. Die Datei kann exportiert werden.".into()))
    }
}

impl crate::CoreServices {
    /// Prüft Typ und Größe vor dem Download einer lokalen Arbeitskopie.
    pub async fn externe_datei(
        &self,
        id: crate::domain::DocumentId,
        version: i64,
    ) -> CoreResult<(DocumentVersion, bytes::Bytes)> {
        let d = self
            .repos
            .documents
            .history(id)
            .await?
            .into_iter()
            .find(|d| d.version == version)
            .ok_or_else(|| CoreError::NotFound(format!("Dokumentversion {version}")))?;
        extern_oeffnen_erlaubt(&d.file_name)?;
        if d.size_bytes > 512 * 1024 * 1024 {
            return Err(CoreError::InvalidInput(
                "Externe Arbeitskopien sind bis 512 MB möglich".into(),
            ));
        }
        self.load_version(id, version).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn programme_oeffnen_dokumente_aber_keine_programmdateien() {
        for name in [
            "Plan.DWG",
            "Tabelle.xlsx",
            "Brief.docx",
            "Foto.jpg",
            "Datei.odt",
            "Bauamt.eml",
            "Nachforderung.MSG",
        ] {
            assert!(extern_oeffnen_erlaubt(name).is_ok());
        }
        for name in [
            "Programm.exe",
            "Skript.cmd",
            "Skript.ps1",
            "Skript.sh",
            "Link.desktop",
        ] {
            assert!(extern_oeffnen_erlaubt(name).is_err());
        }
    }
}
