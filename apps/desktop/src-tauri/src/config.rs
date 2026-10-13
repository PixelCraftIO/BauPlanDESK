//! Betriebsart der Desktop-App, gelesen aus `<app_config_dir>/config.json`.
//! Fehlt die Datei, startet die App als Einzelplatz-Version.

use std::path::Path;

use serde::{Deserialize, Serialize};
use storage::StorageConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "modus", rename_all = "lowercase")]
pub enum AppConfig {
    /// SQLite + lokales Verzeichnis im App-Datenordner.
    #[default]
    Einzelplatz,
    /// Gemeinsame PostgreSQL-Datenbank + NAS/S3, mit lokalem Datei-Cache.
    ///
    /// ```json
    /// { "modus": "netzwerk",
    ///   "database_url": "postgres://bp:geheim@server/bauprojekt",
    ///   "storage": { "type": "nas", "root": "\\\\nas\\bauprojekt" },
    ///   "cache_max_gb": 20 }
    /// ```
    Netzwerk {
        database_url: String,
        storage: StorageConfig,
        #[serde(default = "default_cache_gb")]
        cache_max_gb: u64,
    },
}

fn default_cache_gb() -> u64 {
    10
}

impl AppConfig {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join("config.json");
        match std::fs::read(&path) {
            Ok(raw) => serde_json::from_slice(&raw).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Einzelplatz => "Einzelplatz",
            Self::Netzwerk { .. } => "Netzwerk",
        }
    }
}
