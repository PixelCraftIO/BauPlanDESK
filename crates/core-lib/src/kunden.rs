//! Kundendatenbank: Bauherren, Bauträger, Investoren, Baufirmen usw. werden
//! einmal zentral gepflegt und Projekten als Bauherren oder in weiteren Rollen
//! ([`Beteiligtenrolle`]) zugeordnet. Bauantrag,
//! XBau-Export und Formulare lesen die Angaben von dort, sodass eine neue
//! Anschrift sofort in allen Projekten gilt.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::xbau::{Anschrift, Beteiligter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KundeId(pub Uuid);

impl KundeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for KundeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KundeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Natürliche Person oder Firma/Organisation (so unterscheidet auch XBau).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kundentyp {
    #[default]
    Person,
    Organisation,
}

impl Kundentyp {
    pub fn code(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Organisation => "organisation",
        }
    }
}

impl std::str::FromStr for Kundentyp {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "person" => Ok(Self::Person),
            "organisation" => Ok(Self::Organisation),
            _ => Err(CoreError::Database(format!("unbekannter Kundentyp: {s}"))),
        }
    }
}

/// Wofür der Kunde steht; dient zum Filtern der Kundenliste.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kundenkategorie {
    #[default]
    Privat,
    Bautraeger,
    Baufirma,
    Investor,
    Planungsbuero,
    Wohnungsunternehmen,
    OeffentlicheHand,
    Sonstige,
}

impl Kundenkategorie {
    pub const ALLE: [Kundenkategorie; 8] = [
        Self::Privat,
        Self::Bautraeger,
        Self::Baufirma,
        Self::Investor,
        Self::Planungsbuero,
        Self::Wohnungsunternehmen,
        Self::OeffentlicheHand,
        Self::Sonstige,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Privat => "privat",
            Self::Bautraeger => "bautraeger",
            Self::Baufirma => "baufirma",
            Self::Investor => "investor",
            Self::Planungsbuero => "planungsbuero",
            Self::Wohnungsunternehmen => "wohnungsunternehmen",
            Self::OeffentlicheHand => "oeffentliche_hand",
            Self::Sonstige => "sonstige",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Privat => "Privatperson",
            Self::Bautraeger => "Bauträger",
            Self::Baufirma => "Baufirma",
            Self::Investor => "Investor",
            Self::Planungsbuero => "Planungsbüro",
            Self::Wohnungsunternehmen => "Wohnungsunternehmen",
            Self::OeffentlicheHand => "Öffentliche Hand",
            Self::Sonstige => "Sonstige",
        }
    }
}

impl std::str::FromStr for Kundenkategorie {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALLE
            .into_iter()
            .find(|k| k.code() == s)
            .ok_or_else(|| CoreError::Database(format!("unbekannte Kundenkategorie: {s}")))
    }
}

/// Die bearbeitbaren Angaben eines Kunden.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kundendaten {
    pub typ: Kundentyp,
    pub kategorie: Kundenkategorie,
    /// Eigene Kundennummer des Büros, z. B. aus der Buchhaltung.
    pub kundennummer: String,
    /// Nur bei Personen.
    pub vorname: String,
    /// Nur bei Personen.
    pub familienname: String,
    /// Name der Firma oder Organisation; nur bei Organisationen.
    pub firma: String,
    /// Ansprechpartner bei einer Organisation.
    pub ansprechpartner: String,
    pub anschrift: Anschrift,
    pub telefon: String,
    pub email: String,
    pub notizen: String,
}

impl Kundendaten {
    /// Entfernt Leerraum am Rand und leert die Felder, die zum anderen
    /// Kundentyp gehören.
    pub fn bereinigt(mut self) -> Self {
        for feld in [
            &mut self.kundennummer,
            &mut self.vorname,
            &mut self.familienname,
            &mut self.firma,
            &mut self.ansprechpartner,
            &mut self.anschrift.strasse,
            &mut self.anschrift.hausnummer,
            &mut self.anschrift.postleitzahl,
            &mut self.anschrift.ort,
            &mut self.telefon,
            &mut self.email,
        ] {
            *feld = feld.trim().to_string();
        }
        self.notizen = self.notizen.trim_end().to_string();
        match self.typ {
            Kundentyp::Person => {
                self.firma.clear();
                self.ansprechpartner.clear();
            }
            Kundentyp::Organisation => {
                self.vorname.clear();
                self.familienname.clear();
            }
        }
        self
    }

    /// Prüft die Pflichtangaben (nach [`Self::bereinigt`]).
    pub fn pruefen(&self) -> CoreResult<()> {
        match self.typ {
            Kundentyp::Person if self.familienname.is_empty() => {
                return Err(CoreError::InvalidInput("Familienname fehlt".into()));
            }
            Kundentyp::Organisation if self.firma.is_empty() => {
                return Err(CoreError::InvalidInput(
                    "Name der Firma oder Organisation fehlt".into(),
                ));
            }
            _ => {}
        }
        if !self.email.is_empty() {
            let gueltig = self.email.split_once('@').is_some_and(|(lokal, domain)| {
                !lokal.is_empty()
                    && domain.contains('.')
                    && !self.email.contains(char::is_whitespace)
            });
            if !gueltig {
                return Err(CoreError::InvalidInput(format!(
                    "„{}“ ist keine gültige E-Mail-Adresse",
                    self.email
                )));
            }
        }
        Ok(())
    }

    /// Name, wie er in Listen und im Protokoll erscheint.
    pub fn anzeigename(&self) -> String {
        match self.typ {
            Kundentyp::Person => [self.vorname.trim(), self.familienname.trim()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            Kundentyp::Organisation => self.firma.trim().to_string(),
        }
    }

    /// Übernimmt einen Bauherrn, der bisher direkt im Bauantrag stand.
    pub fn aus_beteiligter(b: &Beteiligter) -> Self {
        match b {
            Beteiligter::Person {
                vorname,
                familienname,
                anschrift,
                ..
            } => Self {
                typ: Kundentyp::Person,
                vorname: vorname.clone(),
                familienname: familienname.clone(),
                anschrift: anschrift.clone(),
                ..Self::default()
            },
            Beteiligter::Organisation {
                name, anschrift, ..
            } => Self {
                typ: Kundentyp::Organisation,
                kategorie: Kundenkategorie::Sonstige,
                firma: name.clone(),
                anschrift: anschrift.clone(),
                ..Self::default()
            },
        }
        .bereinigt()
    }

    /// Gleiche Person bzw. Organisation unter gleicher Anschrift (Groß- und
    /// Kleinschreibung egal). Dient dazu, beim Übernehmen keine Dubletten
    /// anzulegen.
    pub fn gleiche_identitaet(&self, andere: &Kundendaten) -> bool {
        let n = |s: &str| s.trim().to_lowercase();
        let a = (&self.anschrift, &andere.anschrift);
        self.typ == andere.typ
            && n(&self.anzeigename()) == n(&andere.anzeigename())
            && n(&a.0.strasse) == n(&a.1.strasse)
            && n(&a.0.hausnummer) == n(&a.1.hausnummer)
            && n(&a.0.postleitzahl) == n(&a.1.postleitzahl)
            && n(&a.0.ort) == n(&a.1.ort)
    }

    /// `true`, wenn weder Name noch Anschrift angegeben sind.
    pub fn ist_leer(&self) -> bool {
        self.anzeigename().is_empty() && self.anschrift.ist_leer()
    }
}

/// Ein Kunde. `version` dient dem Optimistic Locking im Mehrbenutzerbetrieb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kunde {
    pub id: KundeId,
    #[serde(flatten)]
    pub daten: Kundendaten,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Kunde {
    pub fn anzeigename(&self) -> String {
        self.daten.anzeigename()
    }

    /// Der Kunde als Beteiligter im XBau-Antrag. Die Kunden-ID dient als
    /// Akteur-ID und bleibt damit über alle Nachrichten hinweg gleich.
    pub fn als_beteiligter(&self) -> Beteiligter {
        let d = &self.daten;
        match d.typ {
            Kundentyp::Person => Beteiligter::Person {
                id: self.id.0,
                vorname: d.vorname.clone(),
                familienname: d.familienname.clone(),
                anschrift: d.anschrift.clone(),
            },
            Kundentyp::Organisation => Beteiligter::Organisation {
                id: self.id.0,
                name: d.firma.clone(),
                anschrift: d.anschrift.clone(),
            },
        }
    }
}

/// Rolle eines Kunden im Projekt neben dem Bauherrn. Bauherren stehen
/// weiter in ihrer eigenen Zuordnung ([`crate::repository::KundenRepository::bauherren`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Beteiligtenrolle {
    Grundstueckseigentuemer,
    Erbbauberechtigter,
    Bevollmaechtigter,
    Fachplaner,
}

impl Beteiligtenrolle {
    pub const ALLE: [Beteiligtenrolle; 4] = [
        Self::Grundstueckseigentuemer,
        Self::Erbbauberechtigter,
        Self::Bevollmaechtigter,
        Self::Fachplaner,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Grundstueckseigentuemer => "grundstueckseigentuemer",
            Self::Erbbauberechtigter => "erbbauberechtigter",
            Self::Bevollmaechtigter => "bevollmaechtigter",
            Self::Fachplaner => "fachplaner",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Grundstueckseigentuemer => "Grundstückseigentümer",
            Self::Erbbauberechtigter => "Erbbauberechtigter",
            Self::Bevollmaechtigter => "Bevollmächtigter",
            Self::Fachplaner => "Fachplaner",
        }
    }

    /// Verfügungsberechtigt über das Grundstück.
    pub fn verfuegungsberechtigt(self) -> bool {
        matches!(
            self,
            Self::Grundstueckseigentuemer | Self::Erbbauberechtigter
        )
    }
}

impl std::str::FromStr for Beteiligtenrolle {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALLE
            .into_iter()
            .find(|r| r.code() == s)
            .ok_or_else(|| CoreError::Database(format!("unbekannte Beteiligtenrolle: {s}")))
    }
}

/// Zuordnung eines Kunden in einer Rolle, wie sie gespeichert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beteiligtenzuordnung {
    pub rolle: Beteiligtenrolle,
    pub kunde: KundeId,
}

/// Ein Kunde in seiner Rolle im Projekt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projektbeteiligter {
    pub rolle: Beteiligtenrolle,
    pub kunde: Kunde,
}

/// Grundstückseigentümer und Erbbauberechtigte, die nicht zugleich Bauherr
/// sind. Ihre Zustimmung zum Vorhaben ist als Bauvorlage nachzuweisen; ob und
/// in welcher Form, regelt das Landesrecht.
pub fn zustimmung_erforderlich<'a>(
    bauherren: &[Kunde],
    beteiligte: &'a [Projektbeteiligter],
) -> Vec<&'a Projektbeteiligter> {
    let mut gesehen = std::collections::HashSet::new();
    beteiligte
        .iter()
        .filter(|b| b.rolle.verfuegungsberechtigt())
        .filter(|b| !bauherren.iter().any(|k| k.id == b.kunde.id))
        .filter(|b| gesehen.insert(b.kunde.id))
        .collect()
}

/// Hinweistexte zu [`zustimmung_erforderlich`].
pub fn zustimmungshinweise(bauherren: &[Kunde], beteiligte: &[Projektbeteiligter]) -> Vec<String> {
    zustimmung_erforderlich(bauherren, beteiligte)
        .into_iter()
        .map(|b| {
            format!(
                "Zustimmungserklärung von {} ({}), nicht Bauherr",
                b.kunde.anzeigename(),
                b.rolle.name()
            )
        })
        .collect()
}

/// Sortiert nach Anzeigename, ohne Rücksicht auf Groß- und Kleinschreibung.
pub fn sortieren(kunden: &mut [Kunde]) {
    kunden.sort_by_cached_key(|k| (k.anzeigename().to_lowercase(), k.id));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(vorname: &str, familienname: &str) -> Kundendaten {
        Kundendaten {
            vorname: vorname.into(),
            familienname: familienname.into(),
            ..Kundendaten::default()
        }
    }

    #[test]
    fn pflichtangaben() {
        assert!(person("Erika", "").bereinigt().pruefen().is_err());
        assert!(person("", " Mustermann ").bereinigt().pruefen().is_ok());
        let firma = Kundendaten {
            typ: Kundentyp::Organisation,
            vorname: "bleibt nicht".into(),
            ..Kundendaten::default()
        }
        .bereinigt();
        assert!(firma.vorname.is_empty());
        assert!(firma.pruefen().is_err());
    }

    #[test]
    fn email() {
        let mut k = person("Erika", "Mustermann");
        for gut in ["", "erika@example.de"] {
            k.email = gut.into();
            assert!(k.pruefen().is_ok(), "{gut}");
        }
        for schlecht in ["erika", "@example.de", "erika@example", "er ika@example.de"] {
            k.email = schlecht.into();
            assert!(k.pruefen().is_err(), "{schlecht}");
        }
    }

    #[test]
    fn dubletten_erkennen() {
        let mut a = person("Erika", "Mustermann");
        a.anschrift.ort = "München".into();
        let mut b = person(" erika", "MUSTERMANN ");
        b.anschrift.ort = "münchen".into();
        assert!(a.gleiche_identitaet(&b));
        b.anschrift.ort = "Augsburg".into();
        assert!(!a.gleiche_identitaet(&b));
    }

    #[test]
    fn als_beteiligter() {
        let kunde = Kunde {
            id: KundeId::new(),
            daten: Kundendaten {
                typ: Kundentyp::Organisation,
                firma: "Wohnbau Süd GmbH".into(),
                ..Kundendaten::default()
            },
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        match kunde.als_beteiligter() {
            Beteiligter::Organisation { id, name, .. } => {
                assert_eq!(id, kunde.id.0);
                assert_eq!(name, "Wohnbau Süd GmbH");
            }
            b => panic!("{b:?}"),
        }
    }
}
