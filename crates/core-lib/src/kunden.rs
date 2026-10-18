//! Kundendatenbank: Bauherren, Bauträger, Investoren, Baufirmen usw. werden
//! einmal zentral gepflegt und Projekten als Bauherren oder in weiteren Rollen
//! ([`Beteiligtenrolle`]) zugeordnet. Bauantrag,
//! XBau-Export und Formulare lesen die Angaben von dort, sodass eine neue
//! Anschrift sofort in allen Projekten gilt.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::xbau::{Anschrift, Beteiligter, Kommunikation, WeitereRolle, WeitererBeteiligter};

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
        // Diese Angaben gehen in Behördennachrichten (XBau), deren Schemas
        // nur Zeichen nach DIN 91379 zulassen. Früh melden statt erst beim Export.
        for (feld, wert) in [
            ("Vorname", &self.vorname),
            ("Familienname", &self.familienname),
            ("Firma", &self.firma),
            ("Ansprechpartner", &self.ansprechpartner),
            ("Straße", &self.anschrift.strasse),
            ("Hausnummer", &self.anschrift.hausnummer),
            ("Postleitzahl", &self.anschrift.postleitzahl),
            ("Ort", &self.anschrift.ort),
            ("Telefon", &self.telefon),
            ("E-Mail", &self.email),
        ] {
            if let Some(m) = crate::xbau::din91379::meldung(feld, wert) {
                return Err(CoreError::InvalidInput(m));
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
                kommunikation,
                ..
            } => Self {
                typ: Kundentyp::Person,
                vorname: vorname.clone(),
                familienname: familienname.clone(),
                anschrift: anschrift.clone(),
                telefon: kommunikation.telefon.clone(),
                email: kommunikation.email.clone(),
                ..Self::default()
            },
            Beteiligter::Organisation {
                name,
                anschrift,
                ansprechpartner,
                kommunikation,
                ..
            } => Self {
                typ: Kundentyp::Organisation,
                kategorie: Kundenkategorie::Sonstige,
                firma: name.clone(),
                ansprechpartner: ansprechpartner.clone(),
                anschrift: anschrift.clone(),
                telefon: kommunikation.telefon.clone(),
                email: kommunikation.email.clone(),
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
        let kommunikation = Kommunikation {
            telefon: d.telefon.trim().into(),
            email: d.email.trim().into(),
        };
        match d.typ {
            Kundentyp::Person => Beteiligter::Person {
                id: self.id.0,
                vorname: d.vorname.clone(),
                familienname: d.familienname.clone(),
                anschrift: d.anschrift.clone(),
                kommunikation,
            },
            Kundentyp::Organisation => Beteiligter::Organisation {
                id: self.id.0,
                name: d.firma.clone(),
                anschrift: d.anschrift.clone(),
                ansprechpartner: d.ansprechpartner.trim().into(),
                kommunikation,
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

/// Bauherren und Beteiligte mit Verfügungsrecht über das Grundstück.
pub fn verfuegungsberechtigte(
    bauherren: &[KundeId],
    beteiligte: &[Projektbeteiligter],
) -> std::collections::HashSet<KundeId> {
    bauherren
        .iter()
        .copied()
        .chain(
            beteiligte
                .iter()
                .filter(|b| b.rolle.verfuegungsberechtigt())
                .map(|b| b.kunde.id),
        )
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

/// Weitere Beteiligte der XBau-Antragsnachricht aus einem eingefrorenen Stand:
/// Bevollmächtigte, Eigentümer der Vorhabenflurstücke
/// (`eigentuemer_kunden`) und Projektbeteiligte als Grundstückseigentümer,
/// Erbbauberechtigte und Fachplaner, je Rolle und Kunde einmal.
/// `bauherren` liefert die Angaben von Eigentümern, die zugleich Bauherr
/// sind. Ein Eigentümer ohne Kundenangaben im Stand ist ein Fehler, keine
/// stille Lücke.
pub fn antragsbeteiligte(
    projekt: &crate::domain::Project,
    bauherren: &[Beteiligter],
    beteiligte: &[Projektbeteiligter],
) -> CoreResult<Vec<WeitererBeteiligter>> {
    let mut liste: Vec<(WeitereRolle, Uuid, Beteiligter)> = Vec::new();
    let mut aufnehmen = |rolle, b: Beteiligter| {
        if !liste.iter().any(|(r, id, _)| *r == rolle && *id == b.id()) {
            liste.push((rolle, b.id(), b));
        }
    };
    for b in beteiligte
        .iter()
        .filter(|b| b.rolle == Beteiligtenrolle::Bevollmaechtigter)
    {
        aufnehmen(WeitereRolle::Bevollmaechtigter, b.kunde.als_beteiligter());
    }
    let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
    for (f, k) in infos.vorhabeneigentuemer() {
        let b = beteiligte
            .iter()
            .find(|b| b.kunde.id == k)
            .map(|b| b.kunde.als_beteiligter())
            .or_else(|| bauherren.iter().find(|b| b.id() == k.0).cloned())
            .ok_or_else(|| {
                CoreError::InvalidInput(format!(
                    "Eigentümer von Flurstück {} ist weder Bauherr noch Projektbeteiligter. Unter Beteiligte oder Flurstücke die Zuordnung prüfen.",
                    f.bezeichnung()
                ))
            })?;
        aufnehmen(WeitereRolle::Grundstueckseigentuemer, b);
    }
    for b in beteiligte {
        let rolle = match b.rolle {
            Beteiligtenrolle::Grundstueckseigentuemer => WeitereRolle::Grundstueckseigentuemer,
            Beteiligtenrolle::Erbbauberechtigter => WeitereRolle::Erbbauberechtigter,
            Beteiligtenrolle::Fachplaner => WeitereRolle::Fachplaner,
            Beteiligtenrolle::Bevollmaechtigter => continue,
        };
        aufnehmen(rolle, b.kunde.als_beteiligter());
    }
    Ok(liste
        .into_iter()
        .map(|(rolle, _, beteiligter)| WeitererBeteiligter { rolle, beteiligter })
        .collect())
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
    fn zeichen_ausserhalb_din_91379_werden_beim_speichern_genannt() {
        let mut k = person("Иван", "Petrov");
        let e = k.pruefen().unwrap_err().to_string();
        assert!(e.contains("Vorname:") && e.contains("DIN 91379"), "{e}");
        k.vorname = "Iwan".into();
        assert!(k.pruefen().is_ok());
        // Notizen gehen in keine Nachricht und bleiben frei.
        k.notizen = "Grüße – Ж".into();
        assert!(k.pruefen().is_ok());
    }

    fn kunde(daten: Kundendaten) -> Kunde {
        Kunde {
            id: KundeId::new(),
            daten,
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn antragsbeteiligte_aus_rollen_und_flurstuecken() {
        use crate::grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen};
        let bauherr = kunde(person("Anna", "Bau"));
        let eigen = kunde(person("Erna", "Eigen"));
        let vollmacht = kunde(person("Otto", "Vollmacht"));
        let planer = kunde(person("Frieda", "Statik"));
        let fl = |id: &str, eigentuemer: Vec<KundeId>| FlurstuecksInfo {
            id: id.into(),
            gemarkung: "G".into(),
            gemarkungsnummer: String::new(),
            flur: "1".into(),
            nummer: "1".into(),
            flaeche_m2: 100.,
            grundbuchbezirk: String::new(),
            grundbuchblatt: String::new(),
            laufende_nummer: None,
            grundbuchauszug: None,
            eigentuemer: String::new(),
            eigentuemer_kunden: eigentuemer,
            bemerkung: String::new(),
            rolle: Default::default(),
        };
        let mut projekt = crate::domain::Project {
            id: crate::domain::ProjectId::new(),
            name: "P".into(),
            bundesland: crate::domain::Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        projekt.planungsrecht.grundstuecksinformationen = Some(Grundstuecksinformationen {
            flurstuecke: vec![fl("a", vec![bauherr.id, eigen.id]), fl("b", vec![eigen.id])],
        });
        let beteiligte = vec![
            Projektbeteiligter {
                rolle: Beteiligtenrolle::Fachplaner,
                kunde: planer.clone(),
            },
            Projektbeteiligter {
                rolle: Beteiligtenrolle::Grundstueckseigentuemer,
                kunde: eigen.clone(),
            },
            Projektbeteiligter {
                rolle: Beteiligtenrolle::Bevollmaechtigter,
                kunde: vollmacht.clone(),
            },
        ];
        let bauherren = vec![bauherr.als_beteiligter()];
        let liste = antragsbeteiligte(&projekt, &bauherren, &beteiligte).unwrap();
        let ids: Vec<_> = liste
            .iter()
            .map(|b| (b.rolle, b.beteiligter.id()))
            .collect();
        assert_eq!(
            ids,
            [
                (WeitereRolle::Bevollmaechtigter, vollmacht.id.0),
                (WeitereRolle::Grundstueckseigentuemer, bauherr.id.0),
                (WeitereRolle::Grundstueckseigentuemer, eigen.id.0),
                (WeitereRolle::Fachplaner, planer.id.0),
            ]
        );
        // Eigentümer ohne Kundenangaben im Stand: Fehler statt Lücke.
        let err = antragsbeteiligte(&projekt, &[], &beteiligte).unwrap_err();
        assert!(err.to_string().contains("Flurstück"), "{err}");
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
