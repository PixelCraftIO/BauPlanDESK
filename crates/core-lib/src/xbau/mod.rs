//! XBau-Export (XBau-Hochbau 2.7 mit Kernmodul 1.4.0).
//!
//! Ein Projekt bekommt einen [`Bauantrag`] mit den Angaben, die nicht schon im
//! Projekt stehen (Grundstück, Bauaufsichtsbehörde, ausgewählte
//! Bauvorlagen). Daraus entsteht die Nachricht `baugenehmigung.antrag.0200`
//! und ein ZIP-Paket mit der Nachricht und den Anlagen ([`paket`]).
//!
//! `austausch` prüft XML/ZIP über einen injizierten XSD-Adapter, `postfach`
//! archiviert Originalbezüge und manuelle Belege. Kein Transportdienst.
//! Die Schemas liegen unter `crates/core-lib/xbau/2.7`; Tests und Laufzeit
//! prüfen erzeugte Nachrichten dagegen.

mod antrag;
pub mod austausch;
pub mod codeliste;
pub mod din91379;
mod inhalt;
pub mod paket;
pub mod postfach;
mod xml;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Bundesland, DocumentId};
use crate::error::{CoreError, CoreResult};

pub(crate) use antrag::mime_pruefen;
pub use antrag::{
    Anlagendatei, Antragsart, Nachrichtendaten, antrag, antrag_geaendert, baugenehmigung_aenderung,
    baugenehmigung_antrag,
};
pub use codeliste::{Codeeintrag, Codeliste, Codelisten};
pub use inhalt::{FlaechenBauNvo, Vorhabenangaben, WeitereRolle, WeitererBeteiligter};

/// Version des XBau-Standards, die der Export erzeugt.
pub const XBAU_VERSION: &str = "2.7";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Anschrift {
    pub strasse: String,
    pub hausnummer: String,
    pub postleitzahl: String,
    pub ort: String,
}

impl Anschrift {
    pub fn ist_leer(&self) -> bool {
        [
            &self.strasse,
            &self.hausnummer,
            &self.postleitzahl,
            &self.ort,
        ]
        .iter()
        .all(|s| s.trim().is_empty())
    }
}

/// Erreichbarkeit eines Beteiligten (XBau `Kommunikation`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kommunikation {
    pub telefon: String,
    pub email: String,
}

impl Kommunikation {
    pub fn ist_leer(&self) -> bool {
        self.telefon.trim().is_empty() && self.email.trim().is_empty()
    }
}

/// Bauherr, Gebührenpflichtiger oder weiterer Beteiligter: natürliche Person
/// oder Organisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Beteiligter {
    Person {
        /// Technische ID, gilt innerhalb des Antragsvorgangs.
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        #[serde(default)]
        vorname: String,
        #[serde(default)]
        familienname: String,
        #[serde(default)]
        anschrift: Anschrift,
        /// Fehlt in älteren Bauanträgen und Abgabeständen.
        #[serde(default, skip_serializing_if = "Kommunikation::ist_leer")]
        kommunikation: Kommunikation,
    },
    Organisation {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        #[serde(default)]
        name: String,
        #[serde(default)]
        anschrift: Anschrift,
        /// Ansprechpartner als Freitext (Name der Person); fehlt in älteren
        /// Bauanträgen und Abgabeständen.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        ansprechpartner: String,
        #[serde(default, skip_serializing_if = "Kommunikation::ist_leer")]
        kommunikation: Kommunikation,
    },
}

impl Beteiligter {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Person { id, .. } | Self::Organisation { id, .. } => *id,
        }
    }

    pub fn anschrift(&self) -> &Anschrift {
        match self {
            Self::Person { anschrift, .. } | Self::Organisation { anschrift, .. } => anschrift,
        }
    }

    /// Name, Anschrift und Zeichenvorrat. Die Anschrift muss zustellfähig
    /// sein (XBau verlangt sie für Bauherren und Gebührenpflichtige).
    fn fehlende(&self, wer: &str, fehlt: &mut Vec<String>) {
        self.namen_pruefen(wer, fehlt);
        let a = self.anschrift();
        for (feld, wert) in [
            ("Straße", &a.strasse),
            ("Postleitzahl", &a.postleitzahl),
            ("Ort", &a.ort),
        ] {
            if wert.trim().is_empty() {
                fehlt.push(format!("{wer}: Anschrift ({feld})"));
            }
        }
    }

    /// Name und Zeichenvorrat (DIN 91379) ohne Pflicht zur Anschrift.
    fn namen_pruefen(&self, wer: &str, fehlt: &mut Vec<String>) {
        match self {
            Self::Person {
                vorname,
                familienname,
                ..
            } => {
                if vorname.trim().is_empty() {
                    fehlt.push(format!("{wer}: Vorname"));
                }
                if familienname.trim().is_empty() {
                    fehlt.push(format!("{wer}: Familienname"));
                }
            }
            Self::Organisation { name, .. } => {
                if name.trim().is_empty() {
                    fehlt.push(format!("{wer}: Name der Organisation"));
                }
            }
        }
        for (feld, wert) in self.texte() {
            fehlt.extend(din91379::meldung(&format!("{wer}: {feld}"), wert));
        }
    }

    /// Alle Texte, die in die XBau-Nachricht gehen, mit Feldnamen.
    fn texte(&self) -> Vec<(&'static str, &str)> {
        let mut t = match self {
            Self::Person {
                vorname,
                familienname,
                ..
            } => vec![
                ("Vorname", vorname.as_str()),
                ("Familienname", familienname.as_str()),
            ],
            Self::Organisation {
                name,
                ansprechpartner,
                ..
            } => vec![
                ("Name der Organisation", name.as_str()),
                ("Ansprechpartner", ansprechpartner.as_str()),
            ],
        };
        t.extend(anschrift_texte(self.anschrift()));
        let k = match self {
            Self::Person { kommunikation, .. } | Self::Organisation { kommunikation, .. } => {
                kommunikation
            }
        };
        t.push(("Telefon", k.telefon.as_str()));
        t.push(("E-Mail", k.email.as_str()));
        t
    }
}

fn anschrift_texte(a: &Anschrift) -> [(&'static str, &str); 4] {
    [
        ("Straße", a.strasse.as_str()),
        ("Hausnummer", a.hausnummer.as_str()),
        ("Postleitzahl", a.postleitzahl.as_str()),
        ("Ort", a.ort.as_str()),
    ]
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Flurstueck {
    /// Name der Gemarkung oder Gemarkungsnummer (nur Ziffern).
    pub gemarkung: String,
    /// Gemarkungsnummer laut Kataster; geht dem Namen in XBau vor.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub gemarkungsnummer: String,
    pub flur: String,
    pub zaehler: String,
    pub nenner: String,
}

impl Flurstueck {
    /// XBau-Kodierung eines zentral erfassten Flurstücks.
    pub fn aus_info(f: &crate::grundstuecksinformationen::FlurstuecksInfo) -> Self {
        let (zaehler, nenner) = f.zaehler_nenner();
        Self {
            gemarkung: f.gemarkung.trim().into(),
            gemarkungsnummer: f.gemarkungsnummer.trim().into(),
            flur: f.flur.trim().into(),
            zaehler: zaehler.into(),
            nenner: nenner.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Baugrundstueck {
    pub anschrift: Anschrift,
    pub flurstuecke: Vec<Flurstueck>,
}

/// Empfangende Bauaufsichtsbehörde.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Bauaufsicht {
    pub name: String,
    /// Behördenkennung im DVDV, z. B. `bab:091620000`.
    pub kennung: String,
}

/// Eine Bauvorlage aus dem DMS, die dem Antrag beiliegt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anlagenwahl {
    pub dokument: DocumentId,
    /// Code aus der Codeliste [`codeliste::ANLAGEN`].
    #[serde(default)]
    pub anlageart: String,
    /// Bezeichnung für die Behörde; leer = Dateiname.
    #[serde(default)]
    pub bezeichnung: String,
}

/// Angaben für den Bauantrag, die nicht schon im Projekt stehen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Bauantrag {
    /// Kennzeichnet den Antragsvorgang über alle Nachrichten hinweg
    /// (`referenzAntragsservice`). Wird einmal vergeben und bleibt.
    pub vorgang: Uuid,
    pub bauaufsicht: Bauaufsicht,
    /// Bezeichnung des Bauvorhabens; leer = Projektname.
    pub bezeichnung: String,
    /// Veraltet: Bauherren stehen in der Kundendatenbank und sind dem Projekt
    /// zugeordnet ([`crate::kunden`]). Das Feld wird nur noch gelesen, um alte
    /// Bauanträge zu übernehmen, und nie mehr geschrieben.
    #[serde(skip_serializing)]
    pub bauherren: Vec<Beteiligter>,
    /// Wer die Gebühren zahlt; `None` = der erste Bauherr.
    pub gebuehrenpflichtiger: Option<Beteiligter>,
    /// Gewählter Entwurfsverfasser aus [`Buero::entwurfsverfasser`];
    /// `None` = die einzige eingetragene Person des Büros.
    pub entwurfsverfasser: Option<Uuid>,
    /// Codes der Art der Maßnahme, siehe [`MASSNAHMEN`].
    pub massnahmen: Vec<String>,
    pub anzahl_gebaeude: u32,
    pub grundstueck: Baugrundstueck,
    pub anlagen: Vec<Anlagenwahl>,
    /// Bauordnungsrechtliche Verfahrensart als Code der Landes-Codeliste
    /// [`codeliste::verfahrensart_uri`]; leer = nicht angegeben. Fehlt in
    /// älteren Bauanträgen.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verfahrensart: String,
}

impl Default for Bauantrag {
    fn default() -> Self {
        Self {
            vorgang: Uuid::new_v4(),
            bauaufsicht: Bauaufsicht::default(),
            bezeichnung: String::new(),
            bauherren: Vec::new(),
            gebuehrenpflichtiger: None,
            entwurfsverfasser: None,
            massnahmen: vec!["11".into()],
            anzahl_gebaeude: 1,
            grundstueck: Baugrundstueck {
                anschrift: Anschrift::default(),
                flurstuecke: vec![Flurstueck::default()],
            },
            anlagen: Vec::new(),
            verfahrensart: String::new(),
        }
    }
}

impl Bauantrag {
    /// Entwurfsverfasser dieses Antrags: die gewählte Person oder, ohne Wahl,
    /// die einzige des Büros.
    pub fn entwurfsverfasser_von<'a>(&self, buero: &'a Buero) -> Option<&'a Entwurfsverfasser> {
        match self.entwurfsverfasser {
            Some(id) => buero.entwurfsverfasser.iter().find(|e| e.id == id),
            None => match buero.entwurfsverfasser.as_slice() {
                [e] => Some(e),
                _ => None,
            },
        }
    }

    /// Pflichtangaben, die für den Export noch fehlen (leer = vollständig).
    /// `bauherren` sind die dem Projekt zugeordneten Bauherren. Genannt werden
    /// auch Texte mit Zeichen außerhalb DIN 91379, die das Schema ablehnt.
    pub fn fehlende_angaben(&self, buero: &Buero, bauherren: &[Beteiligter]) -> Vec<String> {
        let mut fehlt = Vec::new();
        if buero.name.trim().is_empty() {
            fehlt.push("Büro: Name".into());
        }
        if buero.kennung.trim().is_empty() {
            fehlt.push("Büro: Absenderkennung".into());
        }
        // Die Büroanschrift ist die Geschäftsanschrift des Entwurfsverfassers.
        for (feld, wert) in [
            ("Straße", &buero.anschrift.strasse),
            ("Postleitzahl", &buero.anschrift.postleitzahl),
            ("Ort", &buero.anschrift.ort),
        ] {
            if wert.trim().is_empty() {
                fehlt.push(format!("Büro: Anschrift ({feld})"));
            }
        }
        for (feld, wert) in [("Name", buero.name.as_str())]
            .into_iter()
            .chain(anschrift_texte(&buero.anschrift))
        {
            fehlt.extend(din91379::meldung(&format!("Büro: {feld}"), wert));
        }
        match self.entwurfsverfasser_von(buero) {
            Some(e) => e.fehlende(&mut fehlt),
            None if self.entwurfsverfasser.is_some() => fehlt.push(
                "Entwurfsverfasser: gewählte Person steht nicht mehr in den Büroangaben".into(),
            ),
            None => fehlt.push("Entwurfsverfasser (bauvorlageberechtigte Person)".into()),
        }
        if self.bauaufsicht.name.trim().is_empty() {
            fehlt.push("Bauaufsichtsbehörde: Name".into());
        }
        if self.bauaufsicht.kennung.trim().is_empty() {
            fehlt.push("Bauaufsichtsbehörde: Behördenkennung (DVDV)".into());
        }
        fehlt.extend(din91379::meldung(
            "Bauaufsichtsbehörde: Name",
            &self.bauaufsicht.name,
        ));
        fehlt.extend(din91379::meldung(
            "Bezeichnung des Bauvorhabens",
            &self.bezeichnung,
        ));
        if bauherren.is_empty() {
            fehlt.push("Bauherr".into());
        }
        for (i, b) in bauherren.iter().enumerate() {
            let wer = if bauherren.len() == 1 {
                "Bauherr".to_string()
            } else {
                format!("{}. Bauherr", i + 1)
            };
            b.fehlende(&wer, &mut fehlt);
        }
        if let Some(g) = &self.gebuehrenpflichtiger {
            g.fehlende("Gebührenpflichtiger", &mut fehlt);
        }
        if self.massnahmen.is_empty() {
            fehlt.push("Art der Maßnahme".into());
        }
        for m in &self.massnahmen {
            if !MASSNAHMEN.iter().any(|(c, _)| c == m) {
                fehlt.push(format!("Art der Maßnahme: unbekannter Code {m}"));
            }
        }
        for (feld, wert) in anschrift_texte(&self.grundstueck.anschrift) {
            fehlt.extend(din91379::meldung(
                &format!("Grundstück: Anschrift ({feld})"),
                wert,
            ));
        }
        if self.grundstueck.flurstuecke.is_empty() {
            fehlt.push("Flurstück".into());
        }
        for (i, f) in self.grundstueck.flurstuecke.iter().enumerate() {
            let wer = if self.grundstueck.flurstuecke.len() == 1 {
                "Flurstück".to_string()
            } else {
                format!("{}. Flurstück", i + 1)
            };
            if f.gemarkung.trim().is_empty() {
                fehlt.push(format!("{wer}: Gemarkung"));
            }
            if f.zaehler.trim().is_empty() {
                fehlt.push(format!("{wer}: Flurstücksnummer"));
            }
            for (feld, wert) in [
                ("Gemarkung", &f.gemarkung),
                ("Flur", &f.flur),
                ("Flurstücksnummer", &f.zaehler),
                ("Nenner", &f.nenner),
            ] {
                fehlt.extend(din91379::meldung(&format!("{wer}: {feld}"), wert));
            }
        }
        if self.anlagen.is_empty() {
            fehlt.push("mindestens eine Anlage (Bauvorlage)".into());
        }
        for (i, a) in self.anlagen.iter().enumerate() {
            if a.anlageart.trim().is_empty() {
                fehlt.push(match a.bezeichnung.trim() {
                    "" => format!("{}. Anlage: Art der Anlage wählen", i + 1),
                    b => format!("Anlage „{b}“: Art der Anlage wählen"),
                });
            }
        }
        fehlt
    }
}

/// Angaben zum eigenen Büro, für alle Projekte gleich. Das Büro ist Absender
/// der Nachricht; Entwurfsverfasser ist eine seiner bauvorlageberechtigten
/// Personen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Buero {
    pub name: String,
    pub anschrift: Anschrift,
    /// Kennung des Absenders, wie sie Antragsportal oder Behörde vergeben
    /// (z. B. DVDV-Kennung).
    pub kennung: String,
    /// Bauvorlageberechtigte Personen des Büros, aus denen der Bauantrag den
    /// Entwurfsverfasser wählt.
    pub entwurfsverfasser: Vec<Entwurfsverfasser>,
}

impl Buero {
    pub fn validate(&self) -> CoreResult<()> {
        let mut ids = std::collections::HashSet::new();
        for e in &self.entwurfsverfasser {
            if !ids.insert(e.id) {
                return Err(CoreError::InvalidInput(
                    "Entwurfsverfasser: doppelte ID".into(),
                ));
            }
            for (feld, wert) in [
                ("Vorname", &e.vorname),
                ("Familienname", &e.familienname),
                ("Berufsbezeichnung", &e.berufsbezeichnung),
                ("Mitgliedsnummer", &e.mitgliedsnummer),
            ] {
                if wert.chars().count() > 200 {
                    return Err(CoreError::InvalidInput(format!(
                        "Entwurfsverfasser: {feld} ist zu lang"
                    )));
                }
            }
            if e.nachweis.chars().count() > 2000 {
                return Err(CoreError::InvalidInput(
                    "Entwurfsverfasser: Nachweis ist zu lang".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Bauvorlageberechtigte Person (XBau `Entwurfsverfasser`). Die Berechtigung
/// belegt entweder die Kammermitgliedschaft oder ein freier Nachweis, etwa
/// eine Bescheinigung nach Landesrecht.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entwurfsverfasser {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub vorname: String,
    #[serde(default)]
    pub familienname: String,
    /// Etwa „Architektin“ oder „Beratender Ingenieur“.
    #[serde(default)]
    pub berufsbezeichnung: String,
    /// Mitgliedsnummer in der Architekten- oder Ingenieurkammer.
    #[serde(default)]
    pub mitgliedsnummer: String,
    /// Land, in dem die Kammer ihren Sitz hat.
    #[serde(default)]
    pub kammerland: Option<Bundesland>,
    /// Nachweis der Bauvorlageberechtigung ohne Kammermitgliedschaft.
    #[serde(default)]
    pub nachweis: String,
}

impl Entwurfsverfasser {
    pub fn anzeigename(&self) -> String {
        format!("{} {}", self.vorname.trim(), self.familienname.trim())
            .trim()
            .to_string()
    }

    fn fehlende(&self, fehlt: &mut Vec<String>) {
        for (feld, wert) in [
            ("Vorname", &self.vorname),
            ("Familienname", &self.familienname),
            ("Berufsbezeichnung", &self.berufsbezeichnung),
        ] {
            fehlt.extend(din91379::meldung(
                &format!("Entwurfsverfasser: {feld}"),
                wert,
            ));
        }
        if self.vorname.trim().is_empty() {
            fehlt.push("Entwurfsverfasser: Vorname".into());
        }
        if self.familienname.trim().is_empty() {
            fehlt.push("Entwurfsverfasser: Familienname".into());
        }
        if self.mitgliedsnummer.trim().is_empty() {
            if self.nachweis.trim().is_empty() {
                fehlt.push(
                    "Entwurfsverfasser: Kammer-Mitgliedsnummer oder Nachweis der Bauvorlageberechtigung"
                        .into(),
                );
            }
        } else if self.kammerland.is_none() {
            fehlt.push("Entwurfsverfasser: Land der Kammer".into());
        }
    }
}

/// Gespeicherter Bauantrag mit Version für das Optimistic Locking.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GespeicherterBauantrag {
    pub antrag: Bauantrag,
    /// 0 = noch nie gespeichert.
    pub version: i64,
    pub geaendert_von: Option<String>,
    pub geaendert_am: Option<DateTime<Utc>>,
    /// Was für den Export noch fehlt (siehe [`Bauantrag::fehlende_angaben`]
    /// und [`Codelisten::fehlende`]).
    pub fehlende_angaben: Vec<String>,
    /// Prüfhinweise, die den Export nicht sperren, etwa eine nötige
    /// Zustimmungserklärung (siehe [`crate::kunden::zustimmungshinweise`]).
    pub hinweise: Vec<String>,
    /// `true`: Die Flurstücke stammen aus den Grundstücksinformationen des
    /// Projekts und werden dort bearbeitet; die Liste im Antrag ist nur Anzeige.
    pub flurstuecke_zentral: bool,
    /// `true`: [`Bauantrag::anzahl_gebaeude`] ist die Zahl der unter
    /// Gebäude/Geschosse erfassten Gebäude und wird dort gepflegt.
    pub gebaeude_zentral: bool,
    /// Landes-Codeliste der Verfahrensarten für das Bundesland des Projekts
    /// ([`codeliste::verfahrensart_uri`]), sofern geladen; daraus wird
    /// [`Bauantrag::verfahrensart`] gewählt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verfahrensarten: Option<Codeliste>,
}

/// Art der Maßnahme (XBau-interne Codeliste `baumassnahmeart`, Version 2.0).
pub const MASSNAHMEN: [(&str, &str); 7] = [
    ("10", "Baumaßnahme"),
    ("11", "Errichtung"),
    ("12", "Änderung"),
    ("13", "Nutzungsänderung (ohne bauliche Änderung)"),
    ("14", "Nutzungsänderung (mit baulicher Änderung)"),
    ("20", "Abgrabung"),
    ("30", "Beseitigung"),
];

/// Schlüssel des Bundeslands nach dem Gemeindeverzeichnis von Destatis.
pub fn land_schluessel(land: Bundesland) -> &'static str {
    match land {
        Bundesland::Sh => "01",
        Bundesland::Hh => "02",
        Bundesland::Ni => "03",
        Bundesland::Hb => "04",
        Bundesland::Nw => "05",
        Bundesland::He => "06",
        Bundesland::Rp => "07",
        Bundesland::Bw => "08",
        Bundesland::By => "09",
        Bundesland::Sl => "10",
        Bundesland::Be => "11",
        Bundesland::Bb => "12",
        Bundesland::Mv => "13",
        Bundesland::Sn => "14",
        Bundesland::St => "15",
        Bundesland::Th => "16",
    }
}

/// MIME-Typ einer Datei nach ihrer Endung. PDF, IFC und BCF stehen so in der
/// Codeliste XBau MIME-Types; die übrigen Typen nennt die Fehlermeldung, wenn
/// die Codeliste sie nicht enthält.
pub fn mime_typ(dateiname: &str) -> Option<&'static str> {
    let endung = dateiname.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match endung.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "tif" | "tiff" => "image/tiff",
        "xml" => "application/xml",
        "gml" => "application/gml+xml",
        "dxf" => "image/vnd.dxf",
        "ifc" => "application/x-extension-ifc",
        "ifcxml" => "application/x-extension-ifcxml",
        "ifczip" => "application/x-extension-ifczip",
        "bcf" => "application/x-extension-bcf",
        "zip" => "application/zip",
        _ => return None,
    })
}
