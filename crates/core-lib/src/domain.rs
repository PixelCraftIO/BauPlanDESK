//! Domänenmodell. Reine Datentypen ohne Persistenz-Annahmen.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::benutzer::BenutzerId;
use crate::error::{CoreError, CoreResult};
use crate::storage::ContentHash;

/// Die 16 Bundesländer, jeweils mit eigener Landesbauordnung (LBO).
/// Der Code entspricht dem ISO-3166-2:DE-Suffix und ist zugleich der
/// Dateiname des LBO-Regelpakets (`by.rhai`, `bw.rhai`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(ts_rs::TS)]
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
#[derive(ts_rs::TS)]
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

/// Planungsrechtliche Einordnung und Eckdaten, vom Projektteam erfasst.
/// Diese Angaben ersetzen keine Prüfung der bauplanungsrechtlichen Zulässigkeit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rechtsgrundlage {
    Par30,
    Par34,
    Par35,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BebauungsplanArt {
    #[default]
    Ungeklaert,
    Kein,
    Qualifiziert,
    Einfach,
    Vorhabenbezogen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AussenbereichsVorhaben {
    Privilegiert,
    Sonstiges,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Planungsrecht {
    pub bau_nvo: Option<crate::flaechen::BauNvo>,
    pub rechtsgrundlage: Option<Rechtsgrundlage>,
    pub bebauungsplan_art: BebauungsplanArt,
    pub ergaenzende_grundlage: Option<Rechtsgrundlage>,
    pub aussenbereich_vorhaben: Option<AussenbereichsVorhaben>,
    pub plan_bezeichnung: String,
    pub plan_nummer: String,
    /// Datum im Format YYYY-MM-DD; leer, solange nicht bekannt.
    pub plan_rechtskraft: String,
    pub gemeinde: String,
    pub standort: String,
    pub vorhaben: String,
    pub nutzung: String,
    pub festsetzungen: String,
    pub begruendung: String,
    pub festgesetzte_werte: crate::festsetzungen::Festsetzungen,
    /// Vergleichswerte der näheren Umgebung für Vorhaben nach § 34 BauGB.
    pub umgebung: Option<crate::festsetzungen::Umgebungsreferenz>,
    pub berechnung: Option<crate::festsetzungen::NutzungsEingabe>,
    /// Bestandsformat, wird beim getrennten Speichern in Fakten und Ansätze überführt.
    pub baugrundstuecke: Option<crate::baugrundstuecke::BaugrundstuecksPlan>,
    pub grundstuecksinformationen:
        Option<crate::grundstuecksinformationen::Grundstuecksinformationen>,
    pub baugrundstuecksermittlung:
        Option<crate::grundstuecksinformationen::BaugrundstuecksErmittlung>,
    pub baugrenzen: Option<crate::cad::baugrenzen::BaugrenzenGeometrie>,
}

impl Planungsrecht {
    pub fn validate(&self) -> CoreResult<()> {
        self.festgesetzte_werte.validate()?;
        if let Some(u) = &self.umgebung {
            u.validate()?;
        }
        if let Some(i) = &self.grundstuecksinformationen {
            i.validate()?;
        }
        if let Some(e) = &self.baugrundstuecksermittlung
            && crate::grundstuecksinformationen::quellen_aktuell(self)?
        {
            crate::grundstuecksinformationen::validiere_ansaetze(self, e)?;
            if let Some(plan) = crate::grundstuecksinformationen::plan(self)? {
                plan.pruefe(self, 0)?;
            }
        }
        if let Some(b) = &self.baugrundstuecke {
            b.pruefe(self, 0)?;
        }
        if let Some(e) = &self.berechnung {
            if e.aufstellung.is_some() {
                crate::flaechen::berechne_aufstellung(e, self)?;
            } else {
                crate::calc::kennzahlen(&e.mass)?;
            }
        }
        if let Some(g) = &self.baugrenzen {
            g.validate()?;
        }
        if self.ergaenzende_grundlage == Some(Rechtsgrundlage::Par30) {
            return Err(CoreError::InvalidInput(
                "Ergänzend ist § 34 oder § 35 BauGB anzugeben".into(),
            ));
        }
        if !self.plan_rechtskraft.is_empty()
            && (self.plan_rechtskraft.len() != 10
                || chrono::NaiveDate::parse_from_str(&self.plan_rechtskraft, "%Y-%m-%d").is_err())
        {
            return Err(CoreError::InvalidInput(
                "Rechtskraft: gültiges Datum im Format YYYY-MM-DD erforderlich".into(),
            ));
        }
        for (name, value, max) in [
            ("Planbezeichnung", &self.plan_bezeichnung, 500),
            ("Plannummer", &self.plan_nummer, 100),
            ("Gemeinde", &self.gemeinde, 500),
            ("Standort", &self.standort, 1000),
            ("Vorhaben", &self.vorhaben, 10000),
            ("Nutzung", &self.nutzung, 1000),
            ("Festsetzungen", &self.festsetzungen, 10000),
            ("Begründung", &self.begruendung, 10000),
        ] {
            if value.chars().count() > max {
                return Err(CoreError::InvalidInput(format!(
                    "{name}: höchstens {max} Zeichen"
                )));
            }
        }
        Ok(())
    }
}

/// Ein Bauvorhaben. `version` dient dem Optimistic Locking im Mehrbenutzerbetrieb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub bundesland: Bundesland,
    /// Grundstücksfläche in m². Mit erfassten Flurstücken abgeleitet als Summe
    /// der Vorhabenflurstücke (siehe
    /// [`crate::grundstuecksinformationen::projektflaeche_ableiten`]); die
    /// maßgebende Fläche nach § 19 Abs. 3 BauNVO liefert dann je Baugrundstück
    /// [`crate::grundstueckskontext`]. Ohne Flurstücke die Angabe des Bestandsprojekts.
    pub grundstuecksflaeche_m2: Option<f64>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub planungsrecht: Planungsrecht,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub bundesland: Bundesland,
    pub grundstuecksflaeche_m2: Option<f64>,
    #[serde(default)]
    pub planungsrecht: Planungsrecht,
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
    #[serde(default)]
    pub planungsrecht: Option<Planungsrecht>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[derive(ts_rs::TS)]
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
    AusgabeauftragGeaendert,
    ProjektGeaendert,
    DokumentAbgelegt,
    DokumentKategorieGeaendert,
    DokumentGesperrt,
    DokumentEntsperrt,
    SperreAufgehoben,
    FormularAusgefuellt,
    BenutzerAngelegt,
    BenutzerGeaendert,
    LizenzEingespielt,
    BauantragGeaendert,
    XbauExportiert,
    EinreichungErstellt,
    EinreichungBestaetigt,
    KundeAngelegt,
    KundeGeaendert,
    KundeGeloescht,
    BauherrenGeaendert,
    BeteiligteGeaendert,
}

impl Aktion {
    const ALLE: [Aktion; 21] = [
        Self::ProjektAngelegt,
        Self::AusgabeauftragGeaendert,
        Self::ProjektGeaendert,
        Self::DokumentAbgelegt,
        Self::DokumentKategorieGeaendert,
        Self::DokumentGesperrt,
        Self::DokumentEntsperrt,
        Self::SperreAufgehoben,
        Self::FormularAusgefuellt,
        Self::BenutzerAngelegt,
        Self::BenutzerGeaendert,
        Self::LizenzEingespielt,
        Self::BauantragGeaendert,
        Self::XbauExportiert,
        Self::EinreichungErstellt,
        Self::EinreichungBestaetigt,
        Self::KundeAngelegt,
        Self::KundeGeaendert,
        Self::KundeGeloescht,
        Self::BauherrenGeaendert,
        Self::BeteiligteGeaendert,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::AusgabeauftragGeaendert => "ausgabeauftrag_geaendert",
            Self::ProjektAngelegt => "projekt_angelegt",
            Self::ProjektGeaendert => "projekt_geaendert",
            Self::DokumentKategorieGeaendert => "dokument_kategorie_geaendert",
            Self::DokumentAbgelegt => "dokument_abgelegt",
            Self::DokumentGesperrt => "dokument_gesperrt",
            Self::DokumentEntsperrt => "dokument_entsperrt",
            Self::SperreAufgehoben => "sperre_aufgehoben",
            Self::FormularAusgefuellt => "formular_ausgefuellt",
            Self::BenutzerAngelegt => "benutzer_angelegt",
            Self::BenutzerGeaendert => "benutzer_geaendert",
            Self::LizenzEingespielt => "lizenz_eingespielt",
            Self::BauantragGeaendert => "bauantrag_geaendert",
            Self::EinreichungErstellt => "einreichung_erstellt",
            Self::EinreichungBestaetigt => "einreichung_bestaetigt",
            Self::XbauExportiert => "xbau_exportiert",
            Self::KundeAngelegt => "kunde_angelegt",
            Self::KundeGeaendert => "kunde_geaendert",
            Self::KundeGeloescht => "kunde_geloescht",
            Self::BauherrenGeaendert => "bauherren_geaendert",
            Self::BeteiligteGeaendert => "beteiligte_geaendert",
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

#[cfg(test)]
mod planungsrecht_tests {
    use super::*;

    #[test]
    fn bestehende_api_payloads_bleiben_kompatibel() {
        let p: NewProject = serde_json::from_str(
            r#"{"name":"Alt","bundesland":"BY","grundstuecksflaeche_m2":800}"#,
        )
        .unwrap();
        assert_eq!(p.planungsrecht, Planungsrecht::default());
        let u: ProjectUpdate = serde_json::from_value(serde_json::json!({
            "id": ProjectId::new(), "expected_version": 1, "name": "Alt", "bundesland": "BY",
            "grundstuecksflaeche_m2": 900
        }))
        .unwrap();
        assert!(u.planungsrecht.is_none());
    }

    #[test]
    fn rechtskraft_muss_ein_gueltiges_datum_sein() {
        let mut p = Planungsrecht::default();
        assert!(p.validate().is_ok());
        p.plan_rechtskraft = "2024-02-29".into();
        assert!(p.validate().is_ok());
        p.plan_rechtskraft = "2025-02-29".into();
        assert!(p.validate().is_err());
        p.plan_rechtskraft = "2025-2-01".into();
        assert!(p.validate().is_err());
    }
}
