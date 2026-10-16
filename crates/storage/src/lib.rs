//! Konkrete Implementierungen von [`core_lib::storage::Storage`].
//!
//! Welche verwendet wird, entscheidet allein die [`StorageConfig`] der
//! jeweiligen Installation – core-lib sieht immer nur `Arc<dyn Storage>`.

pub mod arbeitskopie;
pub mod formularbibliothek;
mod local;
pub mod projekt_export;
pub mod sicherung;
mod wechselbar;
pub use wechselbar::WechselbareAblage;
#[cfg(feature = "s3")]
mod s3;

use std::path::PathBuf;
use std::sync::Arc;

use core_lib::CoreResult;
use core_lib::storage::Storage;
use serde::{Deserialize, Serialize};

pub use local::LocalStorage;
#[cfg(feature = "s3")]
pub use s3::{S3Config, S3Storage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    /// Lokales Verzeichnis (Einzelplatz).
    Local { root: PathBuf },
    /// Eingebundenes NAS-Laufwerk, z. B. `\\nas\bauprojekte` oder `/mnt/nas`.
    /// Technisch ein Dateisystempfad, aber im Client-Modus mit Cache betrieben.
    Nas { root: PathBuf },
    /// S3-kompatibler Objektspeicher (AWS, MinIO, Hetzner, IONOS, …).
    #[cfg(feature = "s3")]
    S3(S3Config),
}

impl StorageConfig {
    /// Muss der Client große Dateien lokal zwischenspeichern?
    pub fn is_remote(&self) -> bool {
        !matches!(self, Self::Local { .. })
    }
}

pub fn build(config: &StorageConfig) -> CoreResult<Arc<dyn Storage>> {
    Ok(match config {
        StorageConfig::Local { root } => Arc::new(LocalStorage::new(root.clone(), "local")?),
        StorageConfig::Nas { root } => Arc::new(LocalStorage::new(root.clone(), "nas")?),
        #[cfg(feature = "s3")]
        StorageConfig::S3(cfg) => Arc::new(S3Storage::new(cfg)?),
    })
}

pub mod buero_sicherung;

pub mod xbau;
