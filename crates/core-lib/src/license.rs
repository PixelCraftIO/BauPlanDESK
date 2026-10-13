//! Schnittstelle für das Lizenzschlüssel-Management.
//!
//! Konkrete Anbieter (lokale signierte Lizenzdatei, Lizenzserver im
//! Büronetz, Online-Aktivierung) implementieren [`LicenseProvider`] und
//! werden wie Datenbank und Storage injiziert.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edition {
    Einzelplatz,
    Buero,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub licensee: String,
    pub edition: Edition,
    pub seats: u32,
    pub valid_until: Option<DateTime<Utc>>,
    /// Zeitpunkt der letzten erfolgreichen Prüfung, z. B. für Offline-Kulanz.
    pub checked_at: DateTime<Utc>,
}

#[async_trait]
pub trait LicenseProvider: Send + Sync {
    async fn status(&self) -> CoreResult<LicenseStatus>;

    /// Belegt einen Arbeitsplatz (Netzwerklizenz). Rückgabe ist ein Token,
    /// das mit [`LicenseProvider::release_seat`] wieder freigegeben wird.
    async fn acquire_seat(&self, workstation: &str) -> CoreResult<String>;

    async fn release_seat(&self, token: &str) -> CoreResult<()>;
}

/// Lizenz für Entwicklung und Tests: immer gültig.
pub struct DevelopmentLicense;

#[async_trait]
impl LicenseProvider for DevelopmentLicense {
    async fn status(&self) -> CoreResult<LicenseStatus> {
        Ok(LicenseStatus {
            licensee: "Entwicklung".into(),
            edition: Edition::Einzelplatz,
            seats: 1,
            valid_until: None,
            checked_at: Utc::now(),
        })
    }

    async fn acquire_seat(&self, workstation: &str) -> CoreResult<String> {
        Ok(format!("dev-{workstation}"))
    }

    async fn release_seat(&self, _token: &str) -> CoreResult<()> {
        Ok(())
    }
}
