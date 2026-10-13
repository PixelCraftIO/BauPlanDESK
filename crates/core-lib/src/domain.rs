//! Domänenmodell. Reine Datentypen ohne Persistenz-Annahmen.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::benutzer::BenutzerId;
use crate::error::CoreError;
use crate::storage::ContentHash;

/// Die 16 Bundesländer, jeweils mit eigener Landesbauordnung (LBO).
/// Der Code entspricht dem ISO-3166-2:DE-Suffix und ist zugleich der
/// Dateiname des LBO-Regelpakets (`by.rhai`, `bw.rhai`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Bundesland {
    Bw,
    By,
    Be,
    Bb,
    Hb,
    Hh,
    He,
    Mv,
    Ni,
    Nw,
    Rp,
    Sl,
    Sn,
    St,
    Sh,
    Th,
}

impl Bundesland {
    pub const ALL: [Bundesland; 16] = [
        Self::Bw,
        Self::By,
        Self::Be,
        Self::Bb,
        Self::Hb,
        Self::Hh,
        Self::He,
        Self::Mv,
        Self::Ni,
        Self::Nw,
        Self::Rp,
        Self::Sl,
        Self::Sn,
        Self::St,
        Self::Sh,
        Self::Th,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Bw => "BW",
            Self::By => "BY",
            Self::Be => "BE",
            Self::Bb => "BB",
            Self::Hb => "HB",
            Self::Hh => "HH",
            Self::He => "HE",
            Self::Mv => "MV",
            Self::Ni => "NI",
            Self::Nw => "NW",
            Self::Rp => "RP",
            Self::Sl => "SL",
            Self::Sn => "SN",
            Self::St => "ST",
            Self::Sh => "SH",
            Self::Th => "TH",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bw => "Baden-Württemberg",
            Self::By => "Bayern",
            Self::Be => "Berlin",
            Self::Bb => "Brandenburg",
            Self::Hb => "Bremen",
            Self::Hh => "Hamburg",
            Self::He => "Hessen",
            Self::Mv => "Mecklenburg-Vorpommern",
            Self::Ni => "Niedersachsen",
            Self::Nw => "Nordrhein-Westfalen",
            Self::Rp => "Rheinland-Pfalz",
            Self::Sl => "Saarland",
            Self::Sn => "Sachsen",
            Self::St => "Sachsen-Anhalt",
            Self::Sh => "Schleswig-Holstein",
            Self::Th => "Thüringen",
        }
    }
}

impl std::str::FromStr for Bundesland {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|land| land.code().eq_ignore_ascii_case(s.trim()))
            .ok_or_else(|| CoreError::InvalidInput(format!("unbekanntes Bundesland: {s}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Ein Bauvorhaben. `version` dient dem Optimistic Locking im Mehrbenutzerbetrieb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub bundesland: Bundesland,
    /// Grundstücksfläche in m² (maßgebend nach § 19 Abs. 3 BauNVO).
    pub grundstuecksflaeche_m2: Option<f64>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub bundesland: Bundesland,
    pub grundstuecksflaeche_m2: Option<f64>,
}

/// Änderung an einem Projekt. `expected_version` ist die Version, die der
/// Client zuletzt gelesen hat; passt sie nicht, schlägt das Update mit
/// [`CoreError::VersionConflict`] fehl.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectUpdate {
    pub id: ProjectId,
    pub expected_version: i64,
    pub name: String,
    pub bundesland: Bundesland,
    pub grundstuecksflaeche_m2: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Eine unveränderliche Version eines DMS-Dokuments (Änderungshistorie).
/// Der Inhalt liegt im [`crate::storage::Storage`] unter `storage_key`;
/// `sha256` ist die maßgebliche Prüfsumme, gegen die auch der Client-Cache
/// validiert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub version: i64,
    pub file_name: String,
    pub storage_key: String,
    pub sha256: ContentHash,
    pub size_bytes: u64,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

/// Sperre eines Dokuments zur Bearbeitung (Check-out). Solange sie besteht,
/// können nur ihr Inhaber neue Versionen ablegen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLock {
    pub document_id: DocumentId,
    pub project_id: ProjectId,
    pub benutzer_id: BenutzerId,
    pub benutzer_name: String,
    pub seit: DateTime<Utc>,
}

/// Protokollierte Änderungen (Änderungshistorie über alle Dokumente und
/// Projektdaten hinweg).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aktion {
    ProjektAngelegt,
    ProjektGeaendert,
    DokumentAbgelegt,
    DokumentGesperrt,
    DokumentEntsperrt,
    SperreAufgehoben,
    FormularAusgefuellt,
    BenutzerAngelegt,
    BenutzerGeaendert,
    LizenzEingespielt,
}

impl Aktion {
    const ALLE: [Aktion; 10] = [
        Self::ProjektAngelegt,
        Self::ProjektGeaendert,
        Self::DokumentAbgelegt,
        Self::DokumentGesperrt,
        Self::DokumentEntsperrt,
        Self::SperreAufgehoben,
        Self::FormularAusgefuellt,
        Self::BenutzerAngelegt,
        Self::BenutzerGeaendert,
        Self::LizenzEingespielt,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::ProjektAngelegt => "projekt_angelegt",
            Self::ProjektGeaendert => "projekt_geaendert",
            Self::DokumentAbgelegt => "dokument_abgelegt",
            Self::DokumentGesperrt => "dokument_gesperrt",
            Self::DokumentEntsperrt => "dokument_entsperrt",
            Self::SperreAufgehoben => "sperre_aufgehoben",
            Self::FormularAusgefuellt => "formular_ausgefuellt",
            Self::BenutzerAngelegt => "benutzer_angelegt",
            Self::BenutzerGeaendert => "benutzer_geaendert",
            Self::LizenzEingespielt => "lizenz_eingespielt",
        }
    }
}

impl std::str::FromStr for Aktion {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALLE
            .into_iter()
            .find(|a| a.code() == s)
            .ok_or_else(|| CoreError::Database(format!("unbekannte Aktion im Protokoll: {s}")))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuerProtokolleintrag {
    pub zeit: DateTime<Utc>,
    pub benutzer_id: BenutzerId,
    pub benutzer_name: String,
    pub aktion: Aktion,
    pub project_id: Option<ProjectId>,
    pub document_id: Option<DocumentId>,
    /// Menschlich lesbare Beschreibung, z. B. „Lageplan.dxf, Version 3“.
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Protokolleintrag {
    pub id: i64,
    #[serde(flatten)]
    pub eintrag: NeuerProtokolleintrag,
}
