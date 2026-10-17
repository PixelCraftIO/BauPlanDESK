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
pub mod paket;
pub mod postfach;
mod xml;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Bundesland, DocumentId};
use crate::error::{CoreError, CoreResult};

pub(crate) use antrag::mime_pruefen;
pub use antrag::{Anlagendatei, Nachrichtendaten, baugenehmigung_aenderung, baugenehmigung_antrag};
pub use codeliste::{Codeeintrag, Codeliste, Codelisten};

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

/// Bauherr oder Gebührenpflichtiger: natürliche Person oder Organisation.
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
    },
    Organisation {
        #[serde(default = "Uuid::new_v4")]
        id: Uuid,
        #[serde(default)]
        name: String,
        #[serde(default)]
        anschrift: Anschrift,
    },
}

impl Beteiligter {
    fn fehlende(&self, wer: &str, fehlt: &mut Vec<String>) {
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
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Flurstueck {
    /// Name der Gemarkung oder Gemarkungsnummer (nur Ziffern).
    pub gemarkung: String,
    pub flur: String,
    pub zaehler: String,
    pub nenner: String,
}

impl Flurstueck {
    /// XBau-Kodierung eines zentral erfassten Flurstücks. Die Flurstücksnummer
    /// „Zähler/Nenner“ wird am Schrägstrich getrennt.
    pub fn aus_info(f: &crate::grundstuecksinformationen::FlurstuecksInfo) -> Self {
        let (zaehler, nenner) = f.nummer.split_once('/').unwrap_or((&f.nummer, ""));
        Self {
            gemarkung: f.gemarkung.trim().into(),
            flur: f.flur.trim().into(),
            zaehler: zaehler.trim().into(),
            nenner: nenner.trim().into(),
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
    /// `bauherren` sind die dem Projekt zugeordneten Bauherren.
    pub fn fehlende_angaben(&self, buero: &Buero, bauherren: &[Beteiligter]) -> Vec<String> {
        let mut fehlt = Vec::new();
        if buero.name.trim().is_empty() {
            fehlt.push("Büro: Name".into());
        }
        if buero.kennung.trim().is_empty() {
            fehlt.push("Büro: Absenderkennung".into());
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
        }
        if self.anlagen.is_empty() {
            fehlt.push("mindestens eine Anlage (Bauvorlage)".into());
        }
        for a in &self.anlagen {
            if a.anlageart.trim().is_empty() {
                fehlt.push(format!("Anlage {}: Art der Anlage", a.bezeichnung.trim()));
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
