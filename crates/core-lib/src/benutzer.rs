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
#[derive(ts_rs::TS)]
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
    ProjektLoeschen,
    DokumentAblegen,
    DokumentSperren,
    FremdeSperreAufheben,
    BenutzerVerwalten,
    LizenzVerwalten,
    BueroangabenAendern,
    KundenBearbeiten,
    KundenLoeschen,
}

impl Recht {
    pub fn mindestrolle(self) -> Rolle {
        match self {
            Self::ProjektBearbeiten
            | Self::DokumentAblegen
            | Self::DokumentSperren
            | Self::KundenBearbeiten => Rolle::Bearbeitung,
            Self::ProjektAnlegen
            | Self::ProjektLoeschen
            | Self::FremdeSperreAufheben
            | Self::BueroangabenAendern
            | Self::KundenLoeschen => Rolle::Projektleitung,
            Self::BenutzerVerwalten | Self::LizenzVerwalten => Rolle::Administrator,
        }
    }

    fn beschreibung(self) -> &'static str {
        match self {
            Self::ProjektAnlegen => "Projekte anlegen",
            Self::ProjektBearbeiten => "Projektdaten ändern",
            Self::ProjektLoeschen => "Projekte löschen",
            Self::DokumentAblegen => "Dokumente ablegen",
            Self::DokumentSperren => "Dokumente sperren",
            Self::FremdeSperreAufheben => "Sperren anderer aufheben",
            Self::BenutzerVerwalten => "Benutzer verwalten",
            Self::LizenzVerwalten => "Lizenzen einspielen",
            Self::BueroangabenAendern => "Büroangaben ändern",
            Self::KundenBearbeiten => "Kunden anlegen und ändern",
            Self::KundenLoeschen => "Kunden löschen",
        }
    }
}

/// Stand des Anmeldekennworts eines Kontos. Der Hash selbst gehört nicht
/// dazu und verlässt die Datenbankschicht nur über [`Kennwortdaten`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kennwortstatus {
    /// Noch kein Kennwort (Bestand vor der Kennwortanmeldung, Einzelplatz).
    #[default]
    Keines,
    /// Von einem Administrator vergeben; muss bei der Anmeldung geändert werden.
    Erstkennwort,
    /// Vom Benutzer selbst gesetzt.
    Gesetzt,
}

impl Kennwortstatus {
    /// Ein Hash ist gespeichert (Erst- oder eigenes Kennwort).
    pub fn vorhanden(self) -> bool {
        self != Self::Keines
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
    /// Ergänzt mit der Kennwortanmeldung; fehlt in älteren Daten (= keines).
    #[serde(default)]
    pub kennwort: Kennwortstatus,
    /// Nach zu vielen Fehlversuchen keine Anmeldung bis zu diesem Zeitpunkt.
    #[serde(default)]
    pub anmeldesperre_bis: Option<DateTime<Utc>>,
    #[serde(default)]
    pub letzte_anmeldung: Option<DateTime<Utc>>,
}

/// Gespeicherte Kennwortangaben eines Kontos, nur für [`crate::anmeldung`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kennwortdaten {
    /// Argon2id im PHC-Format, `None` ohne Kennwort.
    pub hash: Option<String>,
    /// Fehleingaben in Folge seit der letzten erfolgreichen Anmeldung.
    pub fehlversuche: u32,
    pub gesperrt_bis: Option<DateTime<Utc>>,
}

impl Benutzer {
    /// Platzhalter für „niemand angemeldet“ (Anmeldedialog im Netzwerkmodus).
    /// Jede Rechteprüfung im Core lehnt ihn ab; er steht in keiner Datenbank.
    pub fn nicht_angemeldet() -> Self {
        Self {
            id: BenutzerId(Uuid::nil()),
            anmeldename: String::new(),
            anzeigename: "Nicht angemeldet".into(),
            rolle: Rolle::Lesen,
            aktiv: false,
            erstellt_am: DateTime::<Utc>::UNIX_EPOCH,
            kennwort: Kennwortstatus::Keines,
            anmeldesperre_bis: None,
            letzte_anmeldung: None,
        }
    }

    pub fn ist_angemeldet(&self) -> bool {
        !self.id.0.is_nil()
    }

    /// Fehler [`CoreError::Forbidden`], wenn die Rolle das Recht nicht hat
    /// oder der Benutzer deaktiviert ist.
    pub fn pruefe(&self, recht: Recht) -> CoreResult<()> {
        if !self.ist_angemeldet() {
            return Err(CoreError::Forbidden(NICHT_ANGEMELDET.into()));
        }
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

pub(crate) const NICHT_ANGEMELDET: &str = "Bitte melden Sie sich zuerst an.";

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
            kennwort: Kennwortstatus::Keines,
            anmeldesperre_bis: None,
            letzte_anmeldung: None,
        }
    }

    #[test]
    fn nicht_angemeldet_darf_nichts() {
        let b = Benutzer::nicht_angemeldet();
        assert!(!b.ist_angemeldet());
        let e = b.pruefe(Recht::ProjektBearbeiten).unwrap_err();
        assert!(e.to_string().contains("melden Sie sich"), "{e}");
    }

    #[test]
    fn aeltere_benutzerdaten_ohne_kennwortfelder_bleiben_lesbar() {
        let b: Benutzer = serde_json::from_value(serde_json::json!({
            "id": "6f1c8f9e-2d43-4f0a-9a33-0b8f8e0f2c11",
            "anmeldename": "anna",
            "anzeigename": "Anna",
            "rolle": "bearbeitung",
            "aktiv": true,
            "erstellt_am": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(b.kennwort, Kennwortstatus::Keines);
        assert_eq!(b.anmeldesperre_bis, None);
        let json = serde_json::to_value(&b).unwrap();
        assert_eq!(json["kennwort"], "keines");
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
