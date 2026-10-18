use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::domain::Bundesland;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Datensatz nicht gefunden: {0}")]
    NotFound(String),

    /// Optimistic Locking: jemand anderes hat den Datensatz zwischenzeitlich geändert.
    #[error("Konflikt: erwartete Version {expected}, aktuell {actual}")]
    VersionConflict { expected: i64, actual: i64 },

    /// Die Rolle des Benutzers reicht nicht aus.
    #[error("Keine Berechtigung: {0}")]
    Forbidden(String),

    /// Ein anderer Benutzer hat das Dokument zur Bearbeitung gesperrt.
    #[error("Das Dokument ist seit {} von {von} gesperrt", seit.with_timezone(&chrono::Local).format("%d.%m.%Y %H:%M"))]
    Gesperrt { von: String, seit: DateTime<Utc> },

    #[error("Ungültige Eingabe: {0}")]
    InvalidInput(String),

    #[error("Für {0:?} ist kein LBO-Regelpaket geladen")]
    RulePackMissing(Bundesland),

    #[error("LBO-Skriptfehler ({land:?}): {message}")]
    Script { land: Bundesland, message: String },

    #[error("Integritätsfehler: erwarteter Hash {expected}, erhalten {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Datenbankfehler: {0}")]
    Database(String),

    #[error("Storage-Fehler: {0}")]
    Storage(String),

    #[error("Lizenzfehler: {0}")]
    License(String),
}

impl CoreError {
    /// Start gegen eine gemeinsame Datenbank, die bereits einen Datenbankstand
    /// (Migration) kennt, den diese Programmversion nicht hat. Die Datenbank wird
    /// dabei nicht verändert; ältere Arbeitsplätze müssen erst aktualisiert werden.
    pub fn datenbank_neuer(datenbankstand: i64) -> Self {
        Self::Database(format!(
            "Diese Datenbank wurde mit einer neueren BauProjektIO-Version aktualisiert \
             (Datenbankstand {datenbankstand}). Bitte BauProjektIO auf diesem Arbeitsplatz \
             bzw. Server aktualisieren. Die Datenbank wurde nicht verändert."
        ))
    }
    /// Ein bereits angewendeter Datenbankstand weicht inhaltlich von dem dieser
    /// Programmversion ab (veränderte Migration).
    pub fn datenbankstand_abweichend(datenbankstand: i64) -> Self {
        Self::Database(format!(
            "Der Datenbankstand {datenbankstand} dieser Datenbank passt nicht zu dieser \
             BauProjektIO-Version. Bitte die Administration bzw. den Support verständigen; \
             die Datenbank wurde nicht verändert."
        ))
    }
}
