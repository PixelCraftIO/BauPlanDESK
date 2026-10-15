//! Einreichungen besitzen unveränderliche Dateistände. Export ist keine Einreichung:
//! Erst eine ausdrückliche Bestätigung mit Datum verändert den Empfängerstand.
use crate::{
    CoreError, CoreResult,
    domain::{DocumentId, DocumentVersion, ProjectId},
    storage::ContentHash,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Write},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DokumentAuswahl {
    pub dokument_id: DocumentId,
    pub version: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuesPaket {
    pub titel: String,
    pub empfaenger: String,
    pub dokumente: Vec<DokumentAuswahl>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaketStand {
    /// Kategorien zum Zeitpunkt der Paketerstellung, unabhängig von späteren Änderungen.
    /// Fehlend bei älteren Paketen: damaliger Stand nicht erfasst.
    #[serde(default)]
    pub kategorien: BTreeMap<String, Option<crate::dokumentkategorien::KategorieStand>>,
    #[serde(default)]
    pub pruefstand: Option<PaketPruefstand>,
    pub id: Uuid,
    pub projekt_id: ProjectId,
    pub titel: String,
    pub empfaenger: String,
    pub erstellt_am: DateTime<Utc>,
    pub erstellt_von: String,
    pub dokumente: Vec<DocumentVersion>,
}
/// Der Paketstand enthält die tatsächlich gelesenen Fachrevisionen. Er wird nie aus Live-Daten nachgebaut.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaketPruefstand {
    pub schema_version: u32,
    pub ermittelt_am: DateTime<Utc>,
    pub unterlagen: Vec<crate::commit::Fachstand>,
    pub nachweise: Vec<crate::plattform::nachweise::NachweisAnsicht>,
    pub nicht_enthaltene_pflichtunterlagen: Vec<String>,
    pub ungepruefte_unterlagen: Vec<String>,
    pub hinweise: Vec<String>,
}
/// Technische Wiederholung desselben Pakets: Zeitstempel und ZIP-Dateiname sind kein neuer Auftrag.
pub fn gleicher_auftrag(a: &PaketStand, b: &PaketStand) -> bool {
    a.projekt_id == b.projekt_id
        && a.titel == b.titel
        && a.empfaenger == b.empfaenger
        && a.erstellt_von == b.erstellt_von
        && a.dokumente.len() == b.dokumente.len()
        && a.dokumente
            .iter()
            .zip(&b.dokumente)
            .all(|(a, b)| a.document_id == b.document_id && a.version == b.version)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EinreichungsBestaetigung {
    pub datum: NaiveDate,
    pub notiz: String,
    pub bestaetigt_am: DateTime<Utc>,
    pub bestaetigt_von: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Einreichung {
    pub stand: PaketStand,
    pub zip_key: String,
    pub zip_hash: ContentHash,
    pub size: u64,
    pub version: i64,
    pub einreichung: Option<EinreichungsBestaetigung>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VergleichsStatus {
    Neu,
    BereitsEingereicht,
    GleicherInhalt,
    Geaendert,
    Aelter,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DokumentVergleich {
    #[serde(default)]
    pub kategorie: Option<crate::dokumentkategorien::KategorieStand>,
    #[serde(default)]
    pub eingereichte_kategorie: Option<crate::dokumentkategorien::KategorieStand>,
    pub aktuell: DocumentVersion,
    pub zuletzt_eingereicht: Option<DocumentVersion>,
    pub paket_id: Option<Uuid>,
    pub eingereicht_am: Option<NaiveDate>,
    pub status: VergleichsStatus,
}
/// Nach tatsächlichem Einreichungsdatum; bei gleichem Tag nach Bestätigungszeit.
/// Nur bestätigte Pakete desselben Empfängers und Projekts beeinflussen den Stand.
pub fn vergleichen(
    projekt: ProjectId,
    empfaenger: &str,
    docs: Vec<DocumentVersion>,
    chronik: &[Einreichung],
) -> Vec<DokumentVergleich> {
    let mut stand = BTreeMap::new();
    let mut pakete: Vec<_> = chronik
        .iter()
        .filter(|e| {
            e.stand.projekt_id == projekt
                && e.stand
                    .empfaenger
                    .trim()
                    .eq_ignore_ascii_case(empfaenger.trim())
                && e.einreichung.is_some()
        })
        .collect();
    pakete.sort_by_key(|e| {
        let b = e.einreichung.as_ref().unwrap();
        (b.datum, b.bestaetigt_am, e.stand.id)
    });
    for e in pakete {
        for d in &e.stand.dokumente {
            stand.insert(d.document_id.0, (d, e));
        }
    }
    docs.into_iter()
        .map(|aktuell| {
            let alt = stand.get(&aktuell.document_id.0);
            let status = match alt {
                None => VergleichsStatus::Neu,
                Some((d, _)) if d.version == aktuell.version => {
                    VergleichsStatus::BereitsEingereicht
                }
                Some((d, _)) if aktuell.version < d.version => VergleichsStatus::Aelter,
                Some((d, _)) if d.sha256 == aktuell.sha256 => VergleichsStatus::GleicherInhalt,
                Some(_) => VergleichsStatus::Geaendert,
            };
            DokumentVergleich {
                kategorie: None,
                eingereichte_kategorie: alt.and_then(|(_, e)| {
                    e.stand
                        .kategorien
                        .get(&aktuell.document_id.0.to_string())
                        .cloned()
                        .flatten()
                }),
                zuletzt_eingereicht: alt.map(|(d, _)| (*d).clone()),
                paket_id: alt.map(|(_, e)| e.stand.id),
                eingereicht_am: alt.map(|(_, e)| e.einreichung.as_ref().unwrap().datum),
                aktuell,
                status,
            }
        })
        .collect()
}
pub fn pruefen(input: &NeuesPaket) -> CoreResult<()> {
    if input.titel.trim().is_empty()
        || input.titel.len() > 300
        || input.empfaenger.trim().is_empty()
        || input.empfaenger.len() > 300
        || input.dokumente.is_empty()
        || input.dokumente.len() > 500
    {
        return Err(CoreError::InvalidInput(
            "Titel, Empfänger und 1 bis 500 Dokumente sind erforderlich".into(),
        ));
    }
    let mut ids = HashSet::new();
    if input
        .dokumente
        .iter()
        .any(|d| d.version < 1 || !ids.insert(d.dokument_id.0))
    {
        return Err(CoreError::InvalidInput(
            "Je Dokument genau eine gültige Version auswählen".into(),
        ));
    }
    Ok(())
}
/// Manifest enthält verbindliche DMS-Identitäten und Hashes. ZIP-Dateinamen sind
/// unabhängig von Quelldateinamen sicher und eindeutig, auch bei Namensgleichheit.
pub fn packen(stand: &PaketStand, inhalte: &[Vec<u8>]) -> CoreResult<Vec<u8>> {
    if stand.dokumente.len() != inhalte.len() {
        return Err(CoreError::InvalidInput(
            "Unvollständiger Paketinhalt".into(),
        ));
    }
    let err = |e| CoreError::Storage(format!("ZIP: {e}"));
    let mut zip = zip::write::ZipWriter::new(Cursor::new(Vec::new()));
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("Einreichungsstand.json", opt).map_err(err)?;
    zip.write_all(
        &serde_json::to_vec_pretty(stand).map_err(|e| CoreError::Storage(e.to_string()))?,
    )
    .map_err(|e| CoreError::Storage(e.to_string()))?;
    for (d, bytes) in stand.dokumente.iter().zip(inhalte) {
        d.sha256.verify(bytes)?;
        let name: String = d
            .file_name
            .chars()
            .map(|c| {
                if c.is_control() || "/\\:*?\"<>|".contains(c) {
                    '_'
                } else {
                    c
                }
            })
            .take(150)
            .collect();
        zip.start_file(
            format!("Unterlagen/{}/v{}-{}", d.document_id.0, d.version, name),
            opt,
        )
        .map_err(err)?;
        zip.write_all(bytes)
            .map_err(|e| CoreError::Storage(e.to_string()))?;
    }
    Ok(zip.finish().map_err(err)?.into_inner())
}
mod service;
