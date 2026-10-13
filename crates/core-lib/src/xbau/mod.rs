//! XBau-Export (XBau-Hochbau 2.7 mit Kernmodul 1.4.0).
//!
//! Ein Projekt bekommt einen [`Bauantrag`] mit den Angaben, die nicht schon im
//! Projekt stehen (Bauherr, Grundstück, Bauaufsichtsbehörde, ausgewählte
//! Bauvorlagen). Daraus entsteht die Nachricht `baugenehmigung.antrag.0200`
//! und ein ZIP-Paket mit der Nachricht und den Anlagen ([`paket`]).
//!
//! Die Schemas liegen unter `crates/core-lib/xbau/2.7`; die Tests prüfen jede
//! erzeugte Nachricht dagegen.

mod antrag;
pub mod codeliste;
pub mod paket;
mod xml;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{Bundesland, DocumentId};

pub(crate) use antrag::mime_pruefen;
pub use antrag::{Anlagendatei, Nachrichtendaten, baugenehmigung_antrag};
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
    fn ist_leer(&self) -> bool {
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
    pub bauherren: Vec<Beteiligter>,
    /// Wer die Gebühren zahlt; `None` = der erste Bauherr.
    pub gebuehrenpflichtiger: Option<Beteiligter>,
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
    /// Pflichtangaben, die für den Export noch fehlen (leer = vollständig).
    pub fn fehlende_angaben(&self, buero: &Buero) -> Vec<String> {
        let mut fehlt = Vec::new();
        if buero.name.trim().is_empty() {
            fehlt.push("Büro: Name".into());
        }
        if buero.kennung.trim().is_empty() {
            fehlt.push("Büro: Absenderkennung".into());
        }
        if self.bauaufsicht.name.trim().is_empty() {
            fehlt.push("Bauaufsichtsbehörde: Name".into());
        }
        if self.bauaufsicht.kennung.trim().is_empty() {
            fehlt.push("Bauaufsichtsbehörde: Behördenkennung (DVDV)".into());
        }
        if self.bauherren.is_empty() {
            fehlt.push("Bauherr".into());
        }
        for (i, b) in self.bauherren.iter().enumerate() {
            let wer = if self.bauherren.len() == 1 {
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
/// der Nachricht und Entwurfsverfasser.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Buero {
    pub name: String,
    pub anschrift: Anschrift,
    /// Kennung des Absenders, wie sie Antragsportal oder Behörde vergeben
    /// (z. B. DVDV-Kennung).
    pub kennung: String,
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

/// MIME-Typ einer Datei nach ihrer Endung.
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
        "ifc" => "application/x-step",
        "zip" => "application/zip",
        _ => return None,
    })
}
