//! Betriebsart der Desktop-App, gelesen aus `<app_config_dir>/config.json`.
//! Fehlt die Datei, startet die App als Einzelplatz-Version.

use std::path::{Path, PathBuf};

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

/// Arbeitsplatzbezogene Auswahl der lokalen DMS-Ablage. Die Datenbank bleibt
/// am bisherigen Ort; Netzwerk-Installationen nutzen weiter die Büro-Konfiguration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateiablageConfig {
    pub root: std::path::PathBuf,
}

impl DateiablageConfig {
    pub fn load(dir: &Path, standard: std::path::PathBuf) -> Result<Self, String> {
        Self::load_named(dir, standard, "dateiablage.json")
    }
    pub fn load_named(
        dir: &Path,
        standard: std::path::PathBuf,
        datei: &str,
    ) -> Result<Self, String> {
        let file = dir.join(datei);
        match std::fs::read(&file) {
            Ok(data) => serde_json::from_slice(&data).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self { root: standard }),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        self.save_named(dir, "dateiablage.json")
    }
    pub fn save_named(&self, dir: &Path, datei: &str) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let tmp = dir.join(format!("dateiablage-{}.tmp", uuid::Uuid::new_v4()));
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        if let Err(e) = std::fs::rename(&tmp, dir.join(datei)) {
            let _ = std::fs::remove_file(tmp);
            return Err(e.to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DateiablageConfig;
    #[test]
    fn formularordner_bleibt_nach_neuladen_getrennt_von_dms() {
        let dir = std::env::temp_dir().join(format!("bauprojekt-config-{}", uuid::Uuid::new_v4()));
        let formular = dir.join("Formulare");
        let dms = dir.join("Dateien");
        DateiablageConfig { root: dms.clone() }.save(&dir).unwrap();
        DateiablageConfig {
            root: formular.clone(),
        }
        .save_named(&dir, "formularbibliothek.json")
        .unwrap();
        assert_eq!(
            DateiablageConfig::load_named(&dir, dir.join("Standard"), "formularbibliothek.json")
                .unwrap()
                .root,
            formular
        );
        assert_eq!(
            DateiablageConfig::load(&dir, dir.join("Standard"))
                .unwrap()
                .root,
            dms
        );
        std::fs::write(dir.join("formularbibliothek.json"), "ungültig").unwrap();
        assert!(
            DateiablageConfig::load_named(&dir, dir.join("Standard"), "formularbibliothek.json")
                .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Die Profilwahl liegt im globalen Konfigurationsordner; ein Profil besitzt eigene Einstellungen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilAuswahl {
    pub schema_version: u32,
    pub pfad: PathBuf,
}
pub fn profil_pfade(global: &Path, standard: &Path) -> Result<(PathBuf, PathBuf), String> {
    let raw = match std::fs::read(global.join("profil.json")) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((global.into(), standard.into()));
        }
        Err(e) => return Err(e.to_string()),
    };
    let profil: ProfilAuswahl = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if profil.schema_version != 1
        || !profil.pfad.is_absolute()
        || !profil.pfad.join("bauprojekt.db").is_file()
    {
        return Err("Gewähltes Profil ist nicht verfügbar. profil.json im globalen Konfigurationsordner prüfen.".into());
    }
    Ok((profil.pfad.join("einstellungen"), profil.pfad))
}
impl ProfilAuswahl {
    pub fn speichern(&self, global: &Path) -> Result<(), String> {
        std::fs::create_dir_all(global).map_err(|e| e.to_string())?;
        let tmp = global.join(format!(".profil-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            std::fs::write(
                &tmp,
                serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, global.join("profil.json")).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    }
}

#[cfg(test)]
mod profiltests {
    use super::*;
    #[test]
    fn profilwahl_haelt_daten_und_einstellungen_getrennt() {
        let root = std::env::temp_dir().join(format!("bauprojekt-profil-{}", uuid::Uuid::new_v4()));
        let global = root.join("global");
        let standard = root.join("standard");
        let neu = root.join("neu");
        std::fs::create_dir_all(&standard).unwrap();
        std::fs::create_dir_all(&neu).unwrap();
        std::fs::write(standard.join("bauprojekt.db"), b"Original").unwrap();
        std::fs::write(neu.join("bauprojekt.db"), b"Restauriert").unwrap();
        assert_eq!(
            profil_pfade(&global, &standard).unwrap(),
            (global.clone(), standard.clone())
        );
        ProfilAuswahl {
            schema_version: 1,
            pfad: neu.clone(),
        }
        .speichern(&global)
        .unwrap();
        let (config, data) = profil_pfade(&global, &standard).unwrap();
        assert_eq!(data, neu);
        DateiablageConfig {
            root: data.join("dateien"),
        }
        .save(&config)
        .unwrap();
        assert!(!global.join("dateiablage.json").exists());
        assert_eq!(
            std::fs::read(standard.join("bauprojekt.db")).unwrap(),
            b"Original"
        );
        std::fs::remove_file(global.join("profil.json")).unwrap();
        assert_eq!(
            profil_pfade(&global, &standard).unwrap(),
            (global, standard)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
