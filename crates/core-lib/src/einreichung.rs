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
    #[serde(default)]
    pub eingereichte_kategorie_erfasst: bool,
    pub aktuell: DocumentVersion,
    pub zuletzt_eingereicht: Option<DocumentVersion>,
    /// Einreichungspaket der zuletzt eingereichten Version.
    pub paket_id: Option<Uuid>,
    /// Verfahrensakte, wenn die zuletzt eingereichte Version aus einem
    /// versendeten Abgabestand oder einer externen Einreichung stammt.
    #[serde(default)]
    pub verfahren: Option<Uuid>,
    pub eingereicht_am: Option<NaiveDate>,
    pub status: VergleichsStatus,
}
/// Einreichung aus einer Verfahrensakte an denselben Empfänger: versendeter
/// Abgabestand oder nachgetragene externe Einreichung (siehe
/// [`crate::verfahren::externe_einreichung::einreichungsfolge`]).
#[derive(Debug, Clone)]
pub struct Verfahrenseinreichung {
    /// Verfahrensakte der Einreichung.
    pub verfahren: Uuid,
    /// Kennung des Abgabestands bzw. der externen Einreichung.
    pub id: Uuid,
    pub datum: NaiveDate,
    /// Ordnet Einreichungen desselben Tages.
    pub erfasst_am: DateTime<Utc>,
    /// Eingereichte Versionen; Kategorie zum Abgabezeitpunkt, `None` = nicht erfasst.
    pub dokumente: Vec<(
        DocumentVersion,
        Option<Option<crate::dokumentkategorien::KategorieStand>>,
    )>,
}
/// Nach tatsächlichem Einreichungsdatum; bei gleichem Tag nach Bestätigungszeit.
/// Nur bestätigte Pakete desselben Empfängers und Projekts beeinflussen den Stand.
pub fn vergleichen(
    projekt: ProjectId,
    empfaenger: &str,
    docs: Vec<DocumentVersion>,
    chronik: &[Einreichung],
) -> Vec<DokumentVergleich> {
    vergleichen_mit_verfahren(projekt, empfaenger, docs, chronik, &[])
}
/// Wie [`vergleichen`], zusätzlich mit den Einreichungen der Verfahrensakten
/// an diesen Empfänger. Maßgebend ist die jüngste Einreichung (Datum, dann
/// Erfassungs- bzw. Bestätigungszeit) über beide Quellen. `paket_id` nennt
/// nur Einreichungspakete; bei Akteneinreichungen steht die Akte in
/// `verfahren`.
pub fn vergleichen_mit_verfahren(
    projekt: ProjectId,
    empfaenger: &str,
    docs: Vec<DocumentVersion>,
    chronik: &[Einreichung],
    verfahren: &[Verfahrenseinreichung],
) -> Vec<DokumentVergleich> {
    struct Stand<'a> {
        version: &'a DocumentVersion,
        datum: NaiveDate,
        paket: Option<Uuid>,
        verfahren: Option<Uuid>,
        kategorie: Option<Option<crate::dokumentkategorien::KategorieStand>>,
    }
    type Reihung = (NaiveDate, DateTime<Utc>, Uuid);
    let mut eintraege: Vec<(Reihung, Vec<Stand>)> = chronik
        .iter()
        .filter(|e| {
            e.stand.projekt_id == projekt
                && e.stand
                    .empfaenger
                    .trim()
                    .eq_ignore_ascii_case(empfaenger.trim())
        })
        .filter_map(|e| {
            let b = e.einreichung.as_ref()?;
            Some((
                (b.datum, b.bestaetigt_am, e.stand.id),
                e.stand
                    .dokumente
                    .iter()
                    .map(|d| Stand {
                        version: d,
                        datum: b.datum,
                        paket: Some(e.stand.id),
                        verfahren: None,
                        kategorie: e
                            .stand
                            .kategorien
                            .get(&d.document_id.0.to_string())
                            .cloned(),
                    })
                    .collect(),
            ))
        })
        .collect();
    eintraege.extend(verfahren.iter().map(|v| {
        (
            (v.datum, v.erfasst_am, v.id),
            v.dokumente
                .iter()
                .map(|(d, k)| Stand {
                    version: d,
                    datum: v.datum,
                    paket: None,
                    verfahren: Some(v.verfahren),
                    kategorie: k.clone(),
                })
                .collect(),
        )
    }));
    eintraege.sort_by_key(|(k, _)| *k);
    let mut stand = BTreeMap::new();
    for (_, dokumente) in eintraege {
        for d in dokumente {
            stand.insert(d.version.document_id.0, d);
        }
    }
    docs.into_iter()
        .map(|aktuell| {
            let alt = stand.get(&aktuell.document_id.0);
            let status = match alt.map(|s| s.version) {
                None => VergleichsStatus::Neu,
                Some(d) if d.version == aktuell.version => VergleichsStatus::BereitsEingereicht,
                Some(d) if aktuell.version < d.version => VergleichsStatus::Aelter,
                Some(d) if d.sha256 == aktuell.sha256 => VergleichsStatus::GleicherInhalt,
                Some(_) => VergleichsStatus::Geaendert,
            };
            DokumentVergleich {
                kategorie: None,
                eingereichte_kategorie_erfasst: alt.is_some_and(|s| s.kategorie.is_some()),
                eingereichte_kategorie: alt.and_then(|s| s.kategorie.clone().flatten()),
                zuletzt_eingereicht: alt.map(|s| s.version.clone()),
                paket_id: alt.and_then(|s| s.paket),
                verfahren: alt.and_then(|s| s.verfahren),
                eingereicht_am: alt.map(|s| s.datum),
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
        let name =
            crate::dateiname::portabel(&d.file_name, 150).unwrap_or_else(|| "Unterlage".into());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn version(d: DocumentId, p: ProjectId, v: i64, inhalt: &[u8]) -> DocumentVersion {
        DocumentVersion {
            document_id: d,
            project_id: p,
            version: v,
            file_name: "Lageplan.pdf".into(),
            storage_key: String::new(),
            sha256: ContentHash::of(inhalt),
            size_bytes: inhalt.len() as u64,
            created_by: String::new(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn verfahrenseinreichung_zaehlt_als_eingereicht() {
        let p = ProjectId(Uuid::new_v4());
        let (lage, ansicht) = (DocumentId(Uuid::new_v4()), DocumentId(Uuid::new_v4()));
        let vid = Uuid::new_v4();
        let tag = |t| NaiveDate::from_ymd_opt(2026, 9, t).unwrap();
        let extern_ = Verfahrenseinreichung {
            verfahren: vid,
            id: Uuid::new_v4(),
            datum: tag(15),
            erfasst_am: Utc::now(),
            dokumente: vec![(version(lage, p, 1, b"a"), None)],
        };
        let docs = vec![version(lage, p, 2, b"b"), version(ansicht, p, 1, b"c")];
        // Ohne Akteneinreichungen: alles neu (bisheriges Verhalten).
        let alt = vergleichen(p, "Bauamt", docs.clone(), &[]);
        assert!(alt.iter().all(|v| v.status == VergleichsStatus::Neu));
        let v = vergleichen_mit_verfahren(p, "Bauamt", docs, &[], &[extern_]);
        assert_eq!(v[0].status, VergleichsStatus::Geaendert);
        assert_eq!(v[0].eingereicht_am, Some(tag(15)));
        assert_eq!((v[0].verfahren, v[0].paket_id), (Some(vid), None));
        assert!(!v[0].eingereichte_kategorie_erfasst);
        assert_eq!(v[1].status, VergleichsStatus::Neu);
    }
}
