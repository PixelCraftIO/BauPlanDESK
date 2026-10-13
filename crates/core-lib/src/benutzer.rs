//! Benutzer, Rollen und Rechte.
//!
//! Jede schreibende Operation in [`crate::CoreServices`] bekommt den
//! handelnden [`Benutzer`] und prüft dessen Rolle hier. So gelten dieselben
//! Regeln in der Desktop-App und im Server.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BenutzerId(pub Uuid);

impl BenutzerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BenutzerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BenutzerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Rollen in aufsteigender Reihenfolge: Jede Rolle hat alle Rechte der
/// vorherigen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rolle {
    /// Projekte und Dokumente ansehen, rechnen.
    Lesen,
    /// Zusätzlich Projektdaten ändern, Dokumente ablegen und sperren.
    Bearbeitung,
    /// Zusätzlich Projekte anlegen und Sperren anderer aufheben.
    Projektleitung,
    /// Zusätzlich Benutzer verwalten.
    Administrator,
}

impl Rolle {
    pub const ALLE: [Rolle; 4] = [
        Self::Lesen,
        Self::Bearbeitung,
        Self::Projektleitung,
        Self::Administrator,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Lesen => "lesen",
            Self::Bearbeitung => "bearbeitung",
            Self::Projektleitung => "projektleitung",
            Self::Administrator => "administrator",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Lesen => "Lesen",
            Self::Bearbeitung => "Bearbeitung",
            Self::Projektleitung => "Projektleitung",
            Self::Administrator => "Administrator",
        }
    }

    pub fn darf(self, recht: Recht) -> bool {
        self >= recht.mindestrolle()
    }
}

impl std::str::FromStr for Rolle {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALLE
            .into_iter()
            .find(|r| r.code() == s)
            .ok_or_else(|| CoreError::InvalidInput(format!("unbekannte Rolle: {s}")))
    }
}

/// Einzelne Rechte; welche Rolle sie mindestens braucht, steht in
/// [`Recht::mindestrolle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recht {
    ProjektAnlegen,
    ProjektBearbeiten,
    DokumentAblegen,
    DokumentSperren,
    FremdeSperreAufheben,
    BenutzerVerwalten,
}

impl Recht {
    pub fn mindestrolle(self) -> Rolle {
        match self {
            Self::ProjektBearbeiten | Self::DokumentAblegen | Self::DokumentSperren => {
                Rolle::Bearbeitung
            }
            Self::ProjektAnlegen | Self::FremdeSperreAufheben => Rolle::Projektleitung,
            Self::BenutzerVerwalten => Rolle::Administrator,
        }
    }

    fn beschreibung(self) -> &'static str {
        match self {
            Self::ProjektAnlegen => "Projekte anlegen",
            Self::ProjektBearbeiten => "Projektdaten ändern",
            Self::DokumentAblegen => "Dokumente ablegen",
            Self::DokumentSperren => "Dokumente sperren",
            Self::FremdeSperreAufheben => "Sperren anderer aufheben",
            Self::BenutzerVerwalten => "Benutzer verwalten",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Benutzer {
    pub id: BenutzerId,
    /// Eindeutig, ohne Groß-/Kleinschreibung (z. B. Windows-Anmeldename).
    pub anmeldename: String,
    pub anzeigename: String,
    pub rolle: Rolle,
    pub aktiv: bool,
    pub erstellt_am: DateTime<Utc>,
}

impl Benutzer {
    /// Fehler [`CoreError::Forbidden`], wenn die Rolle das Recht nicht hat
    /// oder der Benutzer deaktiviert ist.
    pub fn pruefe(&self, recht: Recht) -> CoreResult<()> {
        if !self.aktiv {
            return Err(CoreError::Forbidden(format!(
                "Das Benutzerkonto „{}“ ist deaktiviert.",
                self.anmeldename
            )));
        }
        if !self.rolle.darf(recht) {
            return Err(CoreError::Forbidden(format!(
                "{} dürfen nur Benutzer mit der Rolle „{}“ oder höher (Ihre Rolle: „{}“).",
                recht.beschreibung(),
                recht.mindestrolle().name(),
                self.rolle.name()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuerBenutzer {
    pub anmeldename: String,
    pub anzeigename: String,
    pub rolle: Rolle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenutzerAenderung {
    pub id: BenutzerId,
    pub anzeigename: String,
    pub rolle: Rolle,
    pub aktiv: bool,
}

/// Normalisiert Anmeldenamen, damit `MUSTER\Daniel` und `daniel` nicht
/// zwei Konten ergeben: Domänenpräfix entfernen, klein schreiben.
pub fn anmeldename_normalisieren(name: &str) -> CoreResult<String> {
    let ohne_domaene = name.rsplit(['\\', '/']).next().unwrap_or(name);
    let ohne_upn = ohne_domaene.split('@').next().unwrap_or(ohne_domaene);
    let n = ohne_upn.trim().to_lowercase();
    if n.is_empty() || n.len() > 128 || n.chars().any(char::is_control) {
        return Err(CoreError::InvalidInput(format!(
            "ungültiger Anmeldename: {name:?}"
        )));
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn benutzer(rolle: Rolle) -> Benutzer {
        Benutzer {
            id: BenutzerId::new(),
            anmeldename: "test".into(),
            anzeigename: "Test".into(),
            rolle,
            aktiv: true,
            erstellt_am: Utc::now(),
        }
    }

    #[test]
    fn rollen_sind_aufsteigend() {
        assert!(!Rolle::Lesen.darf(Recht::ProjektBearbeiten));
        assert!(Rolle::Bearbeitung.darf(Recht::DokumentAblegen));
        assert!(!Rolle::Bearbeitung.darf(Recht::ProjektAnlegen));
        assert!(Rolle::Projektleitung.darf(Recht::FremdeSperreAufheben));
        assert!(!Rolle::Projektleitung.darf(Recht::BenutzerVerwalten));
        assert!(Rolle::Administrator.darf(Recht::BenutzerVerwalten));
    }

    #[test]
    fn deaktivierte_benutzer_duerfen_nichts() {
        let mut b = benutzer(Rolle::Administrator);
        b.aktiv = false;
        assert!(matches!(
            b.pruefe(Recht::DokumentAblegen),
            Err(CoreError::Forbidden(_))
        ));
    }

    #[test]
    fn fehlermeldung_nennt_rolle() {
        let e = benutzer(Rolle::Lesen)
            .pruefe(Recht::DokumentAblegen)
            .unwrap_err();
        assert!(e.to_string().contains("Bearbeitung"), "{e}");
    }

    #[test]
    fn anmeldenamen() {
        assert_eq!(
            anmeldename_normalisieren("BUERO\\Daniel").unwrap(),
            "daniel"
        );
        assert_eq!(
            anmeldename_normalisieren("daniel@buero.local").unwrap(),
            "daniel"
        );
        assert_eq!(anmeldename_normalisieren(" Daniel ").unwrap(), "daniel");
        assert!(anmeldename_normalisieren("  ").is_err());
    }

    #[test]
    fn rollen_als_text() {
        for r in Rolle::ALLE {
            assert_eq!(r.code().parse::<Rolle>().unwrap(), r);
        }
    }
}
