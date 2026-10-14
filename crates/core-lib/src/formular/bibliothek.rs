//! Projektunabhängige PDF-Quellen. Name + Inhaltsstand identifizieren die
//! ausgewählte Vorlage; beim Übernehmen entsteht ein eigenständiges DMS-Dokument.
use crate::{
    CoreResult,
    benutzer::{Benutzer, Recht},
    storage::ContentHash,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibliotheksVorlage {
    pub name: String,
    pub sha256: ContentHash,
    pub size: u64,
}

/// Keine Formularfelder nötig: Auch unveränderte amtliche PDFs können später
/// mit dem bestehenden Editor um eigene Felder ergänzt werden.
pub fn import_pruefen(pdf: &[u8], wer: &Benutzer) -> CoreResult<()> {
    wer.pruefe(Recht::DokumentAblegen)?;
    super::editor::seiten(pdf)?;
    super::pdf::felder_lesen(pdf)?;
    Ok(())
}
