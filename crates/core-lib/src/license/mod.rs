//! Lizenzverwaltung.
//!
//! * [`datei`]: signierte Lizenzdateien prüfen (und beim Hersteller ausstellen).
//! * [`DatenbankLizenz`]: Lizenz und belegte Arbeitsplätze liegen in der
//!   gemeinsamen Datenbank. Der Administrator spielt die Lizenz einmal ein,
//!   alle Arbeitsplätze sehen sie.
//!
//! Ohne Lizenz läuft BauProjektIO als Testversion. Ist die Testversion oder
//! Lizenz abgelaufen, bleibt alles lesbar und exportierbar; nur Änderungen
//! sind gesperrt. Die Prüfung dafür sitzt in [`crate::CoreServices`].

pub mod datei;
mod datenbank;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub use datei::{Lizenz, Lizenzpruefer};
pub use datenbank::DatenbankLizenz;

use crate::error::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    Einzelplatz,
    Buero,
    Cloud,
}

impl Edition {
    pub fn name(self) -> &'static str {
        match self {
            Self::Einzelplatz => "Einzelplatz",
            Self::Buero => "Büro",
            Self::Cloud => "Cloud",
        }
    }
}

/// Ein belegter Arbeitsplatz (Rechner).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arbeitsplatzbelegung {
    pub arbeitsplatz: String,
    pub benutzer_name: String,
    pub belegt_seit: DateTime<Utc>,
    pub letzte_meldung: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Lizenzzustand {
    Lizenziert,
    Testversion {
        tage_uebrig: i64,
    },
    TestversionAbgelaufen,
    Abgelaufen {
        seit: NaiveDate,
    },
    Ungueltig {
        grund: String,
    },
    /// Lizenz gültig, aber alle Arbeitsplätze sind belegt.
    KeinArbeitsplatz,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lizenzstatus {
    pub zustand: Lizenzzustand,
    /// Dürfen auf diesem Arbeitsplatz Daten geändert werden?
    pub schreiben_erlaubt: bool,
    /// Verständliche Beschreibung für die Oberfläche und Fehlermeldungen.
    pub meldung: String,
    pub lizenz: Option<Lizenz>,
    pub arbeitsplaetze_max: u32,
    pub arbeitsplaetze: Vec<Arbeitsplatzbelegung>,
    /// Name dieses Arbeitsplatzes, wenn er einen Platz belegt hat.
    pub eigener_arbeitsplatz: Option<String>,
}

#[async_trait]
pub trait LicenseProvider: Send + Sync {
    async fn status(&self) -> CoreResult<Lizenzstatus>;

    /// Prüft eine Lizenzdatei und speichert sie. Die Rechteprüfung macht
    /// [`crate::CoreServices::lizenz_einspielen`].
    async fn einspielen(&self, datei: &[u8]) -> CoreResult<Lizenzstatus>;

    /// Belegt einen Arbeitsplatz oder meldet ihn als weiterhin aktiv
    /// (regelmäßig aufrufen). Sind alle Plätze belegt, zeigt der Status das.
    async fn arbeitsplatz_belegen(
        &self,
        arbeitsplatz: &str,
        benutzer_name: &str,
    ) -> CoreResult<Lizenzstatus>;

    async fn arbeitsplatz_freigeben(&self, arbeitsplatz: &str) -> CoreResult<()>;
}

/// Lizenz für Entwicklung und Tests: immer gültig.
pub struct DevelopmentLicense;

impl DevelopmentLicense {
    fn status() -> Lizenzstatus {
        Lizenzstatus {
            zustand: Lizenzzustand::Lizenziert,
            schreiben_erlaubt: true,
            meldung: "Entwicklungslizenz".into(),
            lizenz: None,
            arbeitsplaetze_max: u32::MAX,
            arbeitsplaetze: Vec::new(),
            eigener_arbeitsplatz: None,
        }
    }
}

#[async_trait]
impl LicenseProvider for DevelopmentLicense {
    async fn status(&self) -> CoreResult<Lizenzstatus> {
        Ok(Self::status())
    }

    async fn einspielen(&self, _datei: &[u8]) -> CoreResult<Lizenzstatus> {
        Ok(Self::status())
    }

    async fn arbeitsplatz_belegen(&self, _: &str, _: &str) -> CoreResult<Lizenzstatus> {
        Ok(Self::status())
    }

    async fn arbeitsplatz_freigeben(&self, _: &str) -> CoreResult<()> {
        Ok(())
    }
}
