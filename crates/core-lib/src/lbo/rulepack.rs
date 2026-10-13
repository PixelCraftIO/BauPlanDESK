use serde::{Deserialize, Serialize};

use crate::domain::Bundesland;
use crate::error::{CoreError, CoreResult};

/// Quelltext eines Regelpakets. Woher er stammt (Installationsverzeichnis,
/// Update-Server, Datenbank) entscheidet die Laufzeitumgebung.
#[derive(Debug, Clone)]
pub struct RulePackSource {
    pub land: Bundesland,
    pub script: String,
    /// Herkunft für Logs und Fehlermeldungen, z. B. ein Dateipfad.
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulePackInfo {
    pub land: Bundesland,
    pub version: String,
    /// Fassung der Landesbauordnung, auf der das Paket beruht.
    pub rechtsstand: String,
    pub origin: String,
}

/// Rückgabe von `meta()` im Skript.
#[derive(Debug, Deserialize)]
pub(super) struct Meta {
    land: String,
    version: String,
    rechtsstand: String,
}

impl RulePackInfo {
    pub(super) fn from_meta(meta: Meta, source: &RulePackSource) -> CoreResult<Self> {
        let declared: Bundesland = meta.land.parse()?;
        if declared != source.land {
            return Err(CoreError::Script {
                land: source.land,
                message: format!(
                    "{}: Paket deklariert {:?}, erwartet {:?}",
                    source.origin, declared, source.land
                ),
            });
        }
        if meta.version.trim().is_empty() {
            return Err(CoreError::Script {
                land: source.land,
                message: format!("{}: meta().version fehlt", source.origin),
            });
        }
        Ok(Self {
            land: declared,
            version: meta.version,
            rechtsstand: meta.rechtsstand,
            origin: source.origin.clone(),
        })
    }
}
