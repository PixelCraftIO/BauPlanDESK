//! Aufmaß auf PDF- und DXF-Unterlagen: unveränderte Dokumentversion +
//! unabhängig gespeicherte Zeichenmaske. Koordinaten sind Seitenpunkte: bei
//! PDF physische Seitenpunkte (72 je Zoll, links oben, y nach unten), bei DXF
//! Zeichnungseinheiten ab der linken oberen Ecke des Zeichnungsumfangs, y nach
//! unten ([`cadmigration::seitenpunkt`]).
//! Öffentlicher Vertrag und Koordinatenkonvention: docs/planaufmass.md.
mod abgleich;
mod abstand;
mod aufgaben;
mod baugrenzen;
mod cadmigration;
mod export;
mod flurstueck;
mod gebaeude;
mod geometrie;
mod grundstueck;
mod protokoll;
mod pruefung;
mod raumbuch;
mod service;
mod zustand;
pub use abgleich::{
    Abgleichsfrage, Abgleichskandidat, Doppelerfassung, Zeilenzuordnung, abzugsdubletten,
    bauteildubletten, fast_gleich,
};
pub(crate) use abgleich::toleranzen;
pub use abstand::abstandsziele;
pub use aufgaben::{Aufgabengeometrie, Planaufgabe, Werkzeugvorlage, werkzeugkiste};
pub use baugrenzen::BaugrenzenUebernahme;
pub use cadmigration::{CadUebernahme, seitenpunkt};
pub use export::{Planexportformat, Planexportwunsch};
pub use flurstueck::flurstueckziele;
pub use gebaeude::gebaeudeziele;
pub use geometrie::{Flaechenoperation, flaechenoperation};
pub use grundstueck::{Aufmassstand, abzugsziele, bauteilziele};
pub use protokoll::planaenderungen;
pub use pruefung::{Markierungspruefung, Planpruefung};
pub(crate) use service::bindungsstand;
pub use service::{Planziel, planziele};
pub use zustand::Planquellenhinweis;
#[cfg(test)]
mod tests;

use crate::{CoreError, CoreResult, plattform::objekte::Dokumentbezug};
use geo::{Area, LineString, Polygon, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const RECHENSTAND: &str = "planaufmass-1";
pub type Planpunkt = [f64; 2];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Plankalibrierung {
    /// Physische PDF-Seitenpunkte: 72 Punkte = 1 Zoll; unabhängig vom Bildschirmzoom.
    Massstab { nenner: f64 },
    Strecke {
        von: Planpunkt,
        bis: Planpunkt,
        meter: f64,
    },
    /// DXF-Unterlagen: Seitenpunkte sind Zeichnungseinheiten (Ursprung links
    /// oben am Zeichnungsumfang, y nach unten); die Einheit stammt aus dem
    /// DXF-Kopf (`$INSUNITS`) oder wird gewählt, wenn die Datei keine nennt.
    Einheit { einheit: crate::cad::Laengeneinheit },
}
impl Plankalibrierung {
    pub fn meter_pro_punkt(&self) -> CoreResult<f64> {
        let faktor = match self {
            Self::Einheit { einheit } => einheit.meter(),
            Self::Massstab { nenner } => {
                positiv(*nenner)?;
                nenner * 0.0254 / 72.
            }
            Self::Strecke { von, bis, meter } => {
                punkt(von)?;
                punkt(bis)?;
                positiv(*meter)?;
                let l = abstand(von, bis);
                positiv(l)?;
                meter / l
            }
        };
        positiv(faktor)?;
        Ok(faktor)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planebene {
    pub id: Uuid,
    pub bezeichnung: String,
    pub sichtbar: bool,
    pub gesperrt: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Plangeometrie {
    Polygon {
        punkte: Vec<Planpunkt>,
    },
    Rechteck {
        von: Planpunkt,
        bis: Planpunkt,
    },
    Kreis {
        mitte: Planpunkt,
        rand: Planpunkt,
    },
    Strecke {
        von: Planpunkt,
        bis: Planpunkt,
    },
    Hoehenpunkt {
        punkt: Planpunkt,
    },
    /// Zusammengesetzte Fläche mit echten Aussparungen. Bestehende einfache
    /// Geometrien behalten ihre Serialisierung und Berechnung.
    Flaeche {
        polygone: Vec<Planpolygon>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planpolygon {
    pub aussen: Vec<Planpunkt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Vec<Planpunkt>>>")]
    pub innen: Vec<Vec<Planpunkt>>,
}
impl Plangeometrie {
    /// Stützpunkte der Geometrie in Seitenkoordinaten.
    pub fn punkte(&self) -> Vec<Planpunkt> {
        match self {
            Self::Flaeche { polygone } => polygone
                .iter()
                .flat_map(|p| p.aussen.iter().chain(p.innen.iter().flatten()))
                .copied()
                .collect(),
            Self::Polygon { punkte } => punkte.clone(),
            Self::Rechteck { von, bis } | Self::Strecke { von, bis } => vec![*von, *bis],
            Self::Kreis { mitte, rand } => vec![*mitte, *rand],
            Self::Hoehenpunkt { punkt } => vec![*punkt],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Planbedeutung {
    Unbestimmt,
    Flurstueck,
    /// Baufenster (Baugrenzen / Baulinien) und Umriss des Vorhabens im
    /// Lageplan; Grundlage der Lageprüfung nach § 23 BauNVO, siehe
    /// [`crate::CoreServices::planaufmass_baugrenzen`].
    Baufenster,
    Vorhabenumriss,
    PrivateGruenflaeche,
    /// Abzugsflächen der Baugrundstücksermittlung (Arten aus
    /// `regelwerke/grundstueck-abzuege.json`), siehe [`Self::abzugsart`].
    OeffentlicheGruenflaeche,
    Verkehrsflaeche,
    AusserhalbBauland,
    SonstigerAbzug,
    /// GRZ-Bauteile ohne Gebäude (§ 19 Abs. 4 BauNVO), siehe [`Self::bauteiltyp`].
    Zufahrt,
    Stellplatz,
    Garage,
    Nebenanlage,
    Terrasse,
    Unterirdisch,
    Versiegelung,
    GeschossAussenkante,
    Gebaeudegrundflaeche,
    Raum,
    Aufenthaltsraum,
    Nuf,
    Tf,
    Vf,
    Kgf,
    Aussenwand,
    Gelaende,
    Traufe,
    First,
    Dach,
    /// Gebäudenullpunkt ±0,00 (Höhenpunkt in NHN).
    Nullpunkt,
    /// OK Fertigfußboden eines Geschosses.
    Fussboden,
    /// OK Decke eines Geschosses.
    Decke,
    Fundamentsohle,
    /// Senkrechte Strecke: lichte Höhe, Geschosshöhe, Deckendicke.
    Hoehenmass,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planmarkierung {
    pub id: Uuid,
    pub ebene_id: Uuid,
    pub bezeichnung: String,
    pub bedeutung: Planbedeutung,
    pub geometrie: Plangeometrie,
    pub grundstueck_id: Option<Uuid>,
    pub gebaeude_id: Option<Uuid>,
    pub geschoss_id: Option<Uuid>,
    /// Ausschnitt, in dem die Markierung gezeichnet wurde; fehlt bei Altdaten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ausschnitt_id: Option<Uuid>,
    /// Raummarkierung, deren Höhenzone (Teilfläche mit abweichender lichter
    /// Höhe, z. B. unter einer Dachschräge) diese Fläche ist. Das Raumbuch
    /// übernimmt sie als Höhenzone des Raums, nicht als eigenen Raum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hoehenzone_von: Option<Uuid>,
    /// Flurstück (ID aus den Grundstücksinformationen), dessen Grenze diese
    /// Markierung mit Bedeutung „Flurstück“ zeigt. Grundlage der Übernahme in
    /// die Flurstücksfläche; ohne Zuordnung bleibt die Fläche eine Kontrolle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flurstueck_id: Option<String>,
}
impl Planbedeutung {
    pub fn titel(self) -> &'static str {
        match self {
            Self::Unbestimmt => "Noch nicht zugeordnet",
            Self::Flurstueck => "Flurstück",
            Self::Baufenster => "Baufenster",
            Self::Vorhabenumriss => "Vorhabenumriss",
            Self::PrivateGruenflaeche => "Private Grünfläche",
            Self::OeffentlicheGruenflaeche => "Öffentliche Grünfläche",
            Self::Verkehrsflaeche => "Verkehrsfläche",
            Self::AusserhalbBauland => "Fläche außerhalb des Baulands",
            Self::SonstigerAbzug => "Sonstige nicht maßgebende Fläche",
            Self::Zufahrt => "Zufahrt",
            Self::Stellplatz => "Stellplatz",
            Self::Garage => "Garage / Carport",
            Self::Nebenanlage => "Nebenanlage (§ 14 BauNVO)",
            Self::Terrasse => "Terrasse",
            Self::Unterirdisch => "Unterirdische Anlage",
            Self::Versiegelung => "Versiegelung",
            Self::GeschossAussenkante => "Geschoss-Außenkante",
            Self::Gebaeudegrundflaeche => "Gebäudegrundfläche",
            Self::Raum => "Raum",
            Self::Aufenthaltsraum => "Aufenthaltsraum",
            Self::Nuf => "Nutzungsfläche (NUF)",
            Self::Tf => "Technikfläche (TF)",
            Self::Vf => "Verkehrsfläche (VF)",
            Self::Kgf => "Konstruktionsfläche (KGF)",
            Self::Aussenwand => "Außenwand",
            Self::Gelaende => "Gelände",
            Self::Traufe => "Traufe",
            Self::First => "First",
            Self::Dach => "Dach",
            Self::Nullpunkt => "Gebäudenullpunkt",
            Self::Fussboden => "OK Fertigfußboden",
            Self::Decke => "OK Decke",
            Self::Fundamentsohle => "Fundamentsohle",
            Self::Hoehenmass => "Höhenmaß",
        }
    }
    /// Flächen, die das Raumbuch als Raum (oder Höhenzone) übernimmt.
    pub fn raumartig(self) -> bool {
        matches!(
            self,
            Self::Raum | Self::Aufenthaltsraum | Self::Nuf | Self::Tf | Self::Vf
        )
    }
    /// Abzugsart der Baugrundstücksermittlung, die diese Bedeutung liefert.
    pub fn abzugsart(self) -> Option<&'static str> {
        Some(match self {
            Self::PrivateGruenflaeche => "private_gruenflaeche",
            Self::OeffentlicheGruenflaeche => "oeffentliche_gruenflaeche",
            Self::Verkehrsflaeche => "verkehrsflaeche",
            Self::AusserhalbBauland => "ausserhalb_bauland",
            Self::SonstigerAbzug => "sonstiger_abzug",
            _ => return None,
        })
    }
    /// Bedeutung zu einer Abzugsart.
    pub fn fuer_abzugsart(art: &str) -> Option<Self> {
        [
            Self::PrivateGruenflaeche,
            Self::OeffentlicheGruenflaeche,
            Self::Verkehrsflaeche,
            Self::AusserhalbBauland,
            Self::SonstigerAbzug,
        ]
        .into_iter()
        .find(|b| b.abzugsart() == Some(art))
    }
    /// Vorbelegter GRZ-Bauteiltyp für ein gezeichnetes Bauteil ohne Gebäude;
    /// die Hauptanlage kommt über die Gebäudegrundfläche.
    pub fn bauteiltyp(self) -> Option<crate::flaechen::Bauteiltyp> {
        use crate::flaechen::Bauteiltyp as T;
        Some(match self {
            Self::Zufahrt => T::Zufahrt,
            Self::Stellplatz => T::Stellplatz,
            Self::Garage => T::Garage,
            Self::Nebenanlage => T::Nebenanlage14,
            Self::Terrasse => T::TerrasseHauptanlage,
            Self::Unterirdisch => T::Unterirdisch,
            _ => return None,
        })
    }
    /// Bedeutung, die ein Bauteiltyp im Aufmaß erwartet (Garagen in der
    /// Abstandsfläche und Terrassen an Nebenanlagen sind Varianten der Zeichnung).
    pub fn fuer_bauteiltyp(typ: crate::flaechen::Bauteiltyp) -> Option<Self> {
        use crate::flaechen::Bauteiltyp as T;
        Some(match typ {
            T::Zufahrt => Self::Zufahrt,
            T::Stellplatz => Self::Stellplatz,
            T::Garage | T::GarageAbstandsflaeche => Self::Garage,
            T::Nebenanlage14 => Self::Nebenanlage,
            T::TerrasseHauptanlage | T::TerrasseNebenanlage => Self::Terrasse,
            T::Unterirdisch => Self::Unterirdisch,
            T::Hauptanlage | T::BalkonLoggia => return None,
        })
    }
}
/// Zeichnungsart eines Ausschnitts; bestimmt, ob Höhen gemessen werden können.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Ausschnittart {
    Grundriss,
    Lageplan,
    Schnitt,
    Ansicht,
    Detail,
}
impl Ausschnittart {
    /// Nur Schnitte, Ansichten und Details zeigen Höhen lotrecht zur Bezugslinie.
    pub fn mit_hoehen(self) -> bool {
        matches!(self, Self::Schnitt | Self::Ansicht | Self::Detail)
    }
}
/// Bezugslinie eines Schnitts/einer Ansicht (z. B. die ±0,00-Linie). Höhen sind
/// der lotrechte Abstand zu dieser Linie; „oben“ ist die Seite der Linie mit
/// kleinerem y (Seitenoben). Damit darf ein Schnitt auch gedreht liegen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planbezugslinie {
    pub von: Planpunkt,
    pub bis: Planpunkt,
    pub hoehe_m: f64,
    pub system: String,
}
/// Gespeicherte Sicht auf einen Teil der Seite (Grundriss, Schnitt, …) mit
/// eigenem Maßstab und eigener Bezugslinie. Die PDF bleibt unverändert;
/// Markierungen behalten Seitenkoordinaten. Ohne eigenen Maßstab gilt der
/// Maßstab der Seite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planausschnitt {
    pub id: Uuid,
    pub bezeichnung: String,
    pub art: Ausschnittart,
    pub von: Planpunkt,
    pub bis: Planpunkt,
    /// Stabile Zeichenebene; Altbestände verwenden weiterhin die Namenszuordnung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ebene_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kalibrierung: Option<Plankalibrierung>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bezugslinie: Option<Planbezugslinie>,
}
impl Planausschnitt {
    fn enthaelt(&self, p: &Planpunkt) -> bool {
        let (x0, x1) = (self.von[0].min(self.bis[0]), self.von[0].max(self.bis[0]));
        let (y0, y1) = (self.von[1].min(self.bis[1]), self.von[1].max(self.bis[1]));
        p[0] >= x0 - 1. && p[0] <= x1 + 1. && p[1] >= y0 - 1. && p[1] <= y1 + 1.
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planhoehenbezug {
    pub punkt: Planpunkt,
    pub hoehe_m: f64,
    /// Zum Beispiel „m ü. NHN“ oder „lokal, OKFF EG = 0“.
    pub system: String,
}
/// Unterlage als eingepasster Hintergrund einer eigenen Zeichnung. Der
/// Seitenpunkt `ursprung` der Unterlage liegt bei `lage` (Meter) der Zeichnung;
/// die Unterlage ist um `drehung_grad` (im Uhrzeigersinn, Blattkoordinaten)
/// gedreht und mit `meter_pro_punkt` skaliert. Die Unterlage selbst bleibt
/// unverändert; Markierungen werden in Metern der Zeichnung gespeichert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planhintergrund {
    pub id: Uuid,
    pub dokument: Dokumentbezug,
    /// Einsbasiert; DXF immer 1.
    pub seite: u32,
    pub ursprung: Planpunkt,
    pub lage: Planpunkt,
    pub drehung_grad: f64,
    pub meter_pro_punkt: f64,
    /// 0 … 1.
    pub deckkraft: f64,
    pub sichtbar: bool,
    /// IFC-Hintergrund: Geschoss (GlobalId), dessen Grundriss-Schnitt gezeigt
    /// wird; Seitenpunkte sind dann Meter des Modells (wie eine DXF in Metern).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geschoss: Option<String>,
}
/// Zeichenelemente ohne Messwert: Wand mit Dicke, Maßkette, Text. Sie
/// gehören zur Darstellung und zum Fang, nicht zu den Messwerten.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Zeichenform {
    Wand {
        von: Planpunkt,
        bis: Planpunkt,
        dicke_m: f64,
    },
    Masskette {
        von: Planpunkt,
        bis: Planpunkt,
        /// Abstand der Maßlinie von der Strecke in Seitenpunkten; positiv = rechts
        /// der Richtung von→bis (bei y nach unten also im Uhrzeigersinn gedreht).
        abstand: f64,
    },
    Text {
        punkt: Planpunkt,
        inhalt: String,
        /// Schrifthöhe in Seitenpunkten.
        hoehe: f64,
        drehung_grad: f64,
    },
}
// Kein `deny_unknown_fields`: serde kann es nicht mit `flatten` verbinden;
// die Form selbst lehnt unbekannte Felder ab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Zeichenelement {
    pub id: Uuid,
    pub ebene_id: Uuid,
    #[serde(flatten)]
    #[ts(flatten)]
    pub form: Zeichenform,
}
impl Zeichenform {
    /// Stützpunkte in Seitenkoordinaten.
    pub fn punkte(&self) -> Vec<Planpunkt> {
        match self {
            Self::Wand { von, bis, .. } | Self::Masskette { von, bis, .. } => vec![*von, *bis],
            Self::Text { punkt, .. } => vec![*punkt],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planaufmass {
    pub bezeichnung: String,
    /// Unterlage (PDF-Seite oder DXF). Fehlt sie, ist das Aufmaß eine eigene
    /// Zeichnung: Seitenpunkte sind Meter (y nach unten), Unterlagen liegen
    /// als `hintergruende` eingepasst darunter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dokument: Option<Dokumentbezug>,
    /// Einsbasiert. Mehrere Aufmaße derselben Seite dürfen verschiedene Maßstäbe haben.
    pub seite: u32,
    pub kalibrierung: Option<Plankalibrierung>,
    pub hoehenbezug: Option<Planhoehenbezug>,
    pub ebenen: Vec<Planebene>,
    pub markierungen: Vec<Planmarkierung>,
    /// Optionale Ausschnitte; ohne sie gelten Maßstab und Höhenbezug der Seite.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Planausschnitt>>")]
    pub ausschnitte: Vec<Planausschnitt>,
    /// Nur eigene Zeichnungen: eingepasste Unterlagen als Hintergrund.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Planhintergrund>>")]
    pub hintergruende: Vec<Planhintergrund>,
    /// Zeichenelemente ohne Messwert (Wand, Maßkette, Text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Zeichenelement>>")]
    pub elemente: Vec<Zeichenelement>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Planmesswert {
    pub id: Uuid,
    pub laenge_m: Option<f64>,
    pub flaeche_m2: Option<f64>,
    pub horizontale_m: Option<f64>,
    pub hoehe_m: Option<f64>,
    pub neigung_grad: Option<f64>,
    pub offen: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Planmessgroesse {
    Flaeche,
    Laenge,
    Horizontale,
    Hoehe,
    Neigung,
}
impl Planmesswert {
    pub fn wert(&self, art: Planmessgroesse) -> CoreResult<f64> {
        match art {
            Planmessgroesse::Flaeche => self.flaeche_m2,
            Planmessgroesse::Laenge => self.laenge_m,
            Planmessgroesse::Horizontale => self.horizontale_m,
            Planmessgroesse::Hoehe => self.hoehe_m,
            Planmessgroesse::Neigung => self.neigung_grad,
        }
        .ok_or_else(|| fehler("Messgröße ist für diese Markierung noch nicht ermittelt"))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planbindung {
    pub aufmass_id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    pub markierung_id: Uuid,
    pub groesse: Planmessgroesse,
    /// Tabelle.Zeilen-ID.Feld; niemals Arraypositionen.
    pub ziel: String,
}
pub(crate) fn fehler(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn positiv(n: f64) -> CoreResult<()> {
    if !n.is_finite() || n <= 1e-9 || n > 1e12 {
        return Err(fehler(
            "Maßstab, Strecke oder Abmessung muss endlich und positiv sein",
        ));
    }
    Ok(())
}
fn punkt(p: &Planpunkt) -> CoreResult<()> {
    if p.iter().any(|n| !n.is_finite() || n.abs() > 1e8) {
        return Err(fehler("Ungültige Plankoordinate"));
    }
    Ok(())
}
fn abstand(a: &Planpunkt, b: &Planpunkt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn polygon(punkte: &[Planpunkt]) -> CoreResult<(f64, f64)> {
    if !(3..=10000).contains(&punkte.len()) {
        return Err(fehler("Polygon benötigt 3 bis 10000 Punkte"));
    }
    for p in punkte {
        punkt(p)?;
    }
    let p = Polygon::new(
        LineString::from(punkte.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>()),
        vec![],
    );
    if !p.is_valid() {
        return Err(fehler(
            "Polygon überschneidet sich oder besitzt ungültige Kanten",
        ));
    }
    let area = p.unsigned_area();
    positiv(area)?;
    let length = punkte
        .iter()
        .zip(punkte.iter().cycle().skip(1))
        .take(punkte.len())
        .map(|(a, b)| abstand(a, b))
        .sum();
    Ok((area, length))
}
impl Planaufmass {
    /// Eigene Zeichnung ohne Unterlage: Seitenpunkte sind Meter.
    pub fn ist_zeichnung(&self) -> bool {
        self.dokument.is_none()
    }
    /// Kalibrierung des Blattes; eine eigene Zeichnung rechnet immer in Metern.
    pub fn kalibrierung_wirksam(&self) -> Option<Plankalibrierung> {
        if self.ist_zeichnung() {
            Some(Plankalibrierung::Einheit {
                einheit: crate::cad::Laengeneinheit::Meter,
            })
        } else {
            self.kalibrierung.clone()
        }
    }
    fn zeichnung_pruefen(&self) -> CoreResult<()> {
        if self.ist_zeichnung() {
            if self.seite != 1 {
                return Err(fehler("Eine eigene Zeichnung hat genau ein Blatt"));
            }
            let meter = |k: &Plankalibrierung| {
                matches!(
                    k,
                    Plankalibrierung::Einheit {
                        einheit: crate::cad::Laengeneinheit::Meter
                    }
                )
            };
            if self.kalibrierung.as_ref().is_some_and(|k| !meter(k))
                || self.ausschnitte.iter().any(|a| a.kalibrierung.is_some())
            {
                return Err(fehler(
                    "Eine eigene Zeichnung rechnet in Metern; Maßstab und Referenzstrecke gelten nur für Unterlagen",
                ));
            }
            if self.hintergruende.len() > 20 {
                return Err(fehler("Zu viele Hintergründe"));
            }
            let mut ids = std::collections::HashSet::new();
            for h in &self.hintergruende {
                if !ids.insert(h.id) {
                    return Err(fehler("Hintergrund doppelt"));
                }
                if h.seite == 0 || h.dokument.version < 1 {
                    return Err(fehler(
                        "Ungültige Seite oder Dokumentversion des Hintergrunds",
                    ));
                }
                punkt(&h.ursprung)?;
                punkt(&h.lage)?;
                positiv(h.meter_pro_punkt)?;
                if !h.drehung_grad.is_finite()
                    || !(0. ..=1.).contains(&h.deckkraft)
                    || h.meter_pro_punkt > 1e6
                {
                    return Err(fehler("Ungültige Einpassung des Hintergrunds"));
                }
            }
        } else if !self.hintergruende.is_empty() {
            return Err(fehler(
                "Hintergründe gibt es nur in eigenen Zeichnungen; ein Aufmaß zeichnet direkt auf seiner Unterlage",
            ));
        }
        if self.elemente.len() > 2000 {
            return Err(fehler("Zu viele Zeichenelemente"));
        }
        let ebenen: std::collections::HashSet<Uuid> = self.ebenen.iter().map(|e| e.id).collect();
        let mut ids = std::collections::HashSet::new();
        for e in &self.elemente {
            if !ids.insert(e.id) || !ebenen.contains(&e.ebene_id) {
                return Err(fehler("Zeichenelement doppelt oder Zeichenebene fehlt"));
            }
            for p in e.form.punkte() {
                punkt(&p)?;
            }
            match &e.form {
                Zeichenform::Wand { von, bis, dicke_m } => {
                    positiv(abstand(von, bis))?;
                    positiv(*dicke_m)?;
                    if *dicke_m > 10. {
                        return Err(fehler("Wanddicke über 10 m"));
                    }
                }
                Zeichenform::Masskette {
                    von,
                    bis,
                    abstand: a,
                } => {
                    positiv(abstand(von, bis))?;
                    if !a.is_finite() || a.abs() > 1e6 {
                        return Err(fehler("Ungültiger Abstand der Maßkette"));
                    }
                }
                Zeichenform::Text {
                    inhalt,
                    hoehe,
                    drehung_grad,
                    ..
                } => {
                    crate::plattform::text(inhalt, 500)?;
                    positiv(*hoehe)?;
                    if !drehung_grad.is_finite() {
                        return Err(fehler("Ungültige Textdrehung"));
                    }
                }
            }
        }
        Ok(())
    }
    /// Höhensystem, in dem die Markierung absolute Höhen liefert: die
    /// Bezugslinie ihres Ausschnitts, ohne Ausschnitt der blattweite
    /// Höhenbezug. Entspricht der Höhenermittlung in [`Self::messen`]; ein
    /// Ausschnitt ohne Bezugslinie liefert keine absoluten Höhen (`None`).
    pub fn hoehensystem(&self, m: &Planmarkierung) -> Option<&str> {
        match m
            .ausschnitt_id
            .and_then(|id| self.ausschnitte.iter().find(|a| a.id == id))
        {
            Some(a) => a.bezugslinie.as_ref().map(|b| b.system.as_str()),
            None => self.hoehenbezug.as_ref().map(|h| h.system.as_str()),
        }
    }
    pub fn messen(&self) -> CoreResult<Vec<Planmesswert>> {
        crate::plattform::text(&self.bezeichnung, 500)?;
        if self.seite == 0
            || self.dokument.as_ref().is_some_and(|d| d.version < 1)
            || self.ebenen.len() > 100
            || self.markierungen.len() > 2000
        {
            return Err(fehler(
                "Ungültige Seite, Dokumentversion oder zu viele Zeichenobjekte",
            ));
        }
        let mut ebenen = std::collections::HashSet::new();
        for e in &self.ebenen {
            crate::plattform::text(&e.bezeichnung, 200)?;
            if !ebenen.insert(e.id) {
                return Err(fehler("Zeichenebene doppelt"));
            }
        }
        if let Some(h) = &self.hoehenbezug {
            punkt(&h.punkt)?;
            crate::plattform::text(&h.system, 200)?;
            if !h.hoehe_m.is_finite() || h.hoehe_m.abs() > 1e6 {
                return Err(fehler("Ungültiger Höhenbezug"));
            }
        }
        self.zeichnung_pruefen()?;
        let seitenfaktor = self
            .kalibrierung_wirksam()
            .as_ref()
            .map(Plankalibrierung::meter_pro_punkt)
            .transpose()?;
        if self.ausschnitte.len() > 100 {
            return Err(fehler("Zu viele Ausschnitte"));
        }
        let mut ausschnitte = std::collections::HashSet::new();
        for a in &self.ausschnitte {
            if a.ebene_id.is_some_and(|id| !ebenen.contains(&id)) {
                return Err(fehler("Zeichenebene des Ausschnitts fehlt"));
            }
            crate::plattform::text(&a.bezeichnung, 200)?;
            punkt(&a.von)?;
            punkt(&a.bis)?;
            positiv((a.von[0] - a.bis[0]).abs())?;
            positiv((a.von[1] - a.bis[1]).abs())?;
            if !ausschnitte.insert(a.id) {
                return Err(fehler("Ausschnitt doppelt"));
            }
            if let Some(k) = &a.kalibrierung {
                k.meter_pro_punkt()?;
            }
            if let Some(b) = &a.bezugslinie {
                if !a.art.mit_hoehen() {
                    return Err(fehler(
                        "Grundriss und Lageplan besitzen keine Bezugslinie für Höhen",
                    ));
                }
                punkt(&b.von)?;
                punkt(&b.bis)?;
                positiv(abstand(&b.von, &b.bis))?;
                crate::plattform::text(&b.system, 200)?;
                if !b.hoehe_m.is_finite() || b.hoehe_m.abs() > 1e6 {
                    return Err(fehler("Ungültige Bezugslinie"));
                }
            }
        }
        for m in &self.markierungen {
            let Some(raum) = m.hoehenzone_von else {
                continue;
            };
            let eltern = self
                .markierungen
                .iter()
                .find(|e| e.id == raum && e.id != m.id)
                .ok_or_else(|| fehler(format!("{}: Raum der Höhenzone fehlt", m.bezeichnung)))?;
            if !m.bedeutung.raumartig()
                || !eltern.bedeutung.raumartig()
                || eltern.hoehenzone_von.is_some()
                || eltern.gebaeude_id != m.gebaeude_id
                || eltern.geschoss_id != m.geschoss_id
            {
                return Err(fehler(format!(
                    "{}: Eine Höhenzone ist eine Raumfläche im selben Gebäude und Geschoss wie ihr Raum „{}“, der selbst keine Höhenzone ist",
                    m.bezeichnung, eltern.bezeichnung
                )));
            }
        }
        let mut ids = std::collections::HashSet::new();
        self.markierungen
            .iter()
            .map(|m| {
                crate::plattform::text(&m.bezeichnung, 500)?;
                if let Some(f) = &m.flurstueck_id
                    && (f.trim().is_empty() || f.len() > 500)
                {
                    return Err(fehler("Flurstückszuordnung ungültig"));
                }
                if !ids.insert(m.id) || !ebenen.contains(&m.ebene_id) {
                    return Err(fehler("Markierung doppelt oder Zeichenebene fehlt"));
                }
                let ausschnitt = m
                    .ausschnitt_id
                    .map(|id| {
                        self.ausschnitte
                            .iter()
                            .find(|a| a.id == id)
                            .ok_or_else(|| fehler("Ausschnitt der Markierung fehlt"))
                    })
                    .transpose()?;
                let faktor = match ausschnitt.and_then(|a| a.kalibrierung.as_ref()) {
                    Some(k) => Some(k.meter_pro_punkt()?),
                    None => seitenfaktor,
                };
                // Höhenrichtung und Bezug: Bezugslinie des Ausschnitts, sonst die
                // Seitenvertikale mit dem Höhenbezug der Seite. Ein Ausschnitt ohne
                // Bezugslinie liefert keine absoluten Höhen.
                let (aufwaerts, bezug): ([f64; 2], Option<(Planpunkt, f64)>) =
                    match ausschnitt {
                        Some(a) => match &a.bezugslinie {
                            Some(b) => {
                                let l = abstand(&b.von, &b.bis);
                                let d = [(b.bis[0] - b.von[0]) / l, (b.bis[1] - b.von[1]) / l];
                                let mut n = [d[1], -d[0]];
                                if n[1] > 0. || (n[1] == 0. && n[0] > 0.) {
                                    n = [-n[0], -n[1]];
                                }
                                (n, Some((b.von, b.hoehe_m)))
                            }
                            None => ([0., -1.], None),
                        },
                        None => (
                            [0., -1.],
                            self.hoehenbezug.as_ref().map(|h| (h.punkt, h.hoehe_m)),
                        ),
                    };
                let waagerecht = [-aufwaerts[1], aufwaerts[0]];
                let mut r = Planmesswert {
                    id: m.id,
                    laenge_m: None,
                    flaeche_m2: None,
                    horizontale_m: None,
                    hoehe_m: None,
                    neigung_grad: None,
                    offen: vec![],
                };
                let (area, length) = match &m.geometrie {
                    Plangeometrie::Flaeche { polygone } => {
                        let (a, l) = geometrie::flaechenmasse(polygone)?;
                        (Some(a), Some(l))
                    }
                    Plangeometrie::Polygon { punkte } => {
                        let (a, l) = polygon(punkte)?;
                        (Some(a), Some(l))
                    }
                    Plangeometrie::Rechteck { von: a, bis: b } => {
                        punkt(a)?;
                        punkt(b)?;
                        let w = (a[0] - b[0]).abs();
                        let h = (a[1] - b[1]).abs();
                        positiv(w)?;
                        positiv(h)?;
                        (Some(w * h), Some(2. * (w + h)))
                    }
                    Plangeometrie::Kreis { mitte: a, rand: b } => {
                        punkt(a)?;
                        punkt(b)?;
                        let radius = abstand(a, b);
                        positiv(radius)?;
                        (
                            Some(std::f64::consts::PI * radius * radius),
                            Some(2. * std::f64::consts::PI * radius),
                        )
                    }
                    Plangeometrie::Strecke { von: a, bis: b } => {
                        punkt(a)?;
                        punkt(b)?;
                        let l = abstand(a, b);
                        positiv(l)?;
                        let v = [b[0] - a[0], b[1] - a[1]];
                        let dx = (v[0] * waagerecht[0] + v[1] * waagerecht[1]).abs();
                        let dy = (v[0] * aufwaerts[0] + v[1] * aufwaerts[1]).abs();
                        r.horizontale_m = faktor.map(|f| dx * f);
                        // Positive Höhendifferenz; absolute Höhen ausschließlich mit Höhenbezug.
                        r.hoehe_m = faktor.map(|f| dy * f);
                        r.neigung_grad = faktor.map(|_| dy.atan2(dx).to_degrees());
                        (None, Some(l))
                    }
                    Plangeometrie::Hoehenpunkt { punkt: p } => {
                        punkt(p)?;
                        r.hoehe_m = faktor.zip(bezug).map(|(f, (o, h))| {
                            h + ((p[0] - o[0]) * aufwaerts[0] + (p[1] - o[1]) * aufwaerts[1]) * f
                        });
                        if bezug.is_none() {
                            r.offen.push(match ausschnitt {
                                Some(a) if !a.art.mit_hoehen() => format!(
                                    "Ausschnitt „{}“ ist ein {}; Höhen nur in Schnitt, Ansicht oder Detail",
                                    a.bezeichnung,
                                    match a.art {
                                        Ausschnittart::Lageplan => "Lageplan",
                                        _ => "Grundriss",
                                    }
                                ),
                                Some(a) => format!("Bezugslinie des Ausschnitts „{}“ fehlt", a.bezeichnung),
                                None => "Höhenbezug fehlt".into(),
                            });
                        }
                        (None, None)
                    }
                };
                r.flaeche_m2 = area.zip(faktor).map(|(a, f)| a * f * f);
                r.laenge_m = length.zip(faktor).map(|(l, f)| l * f);
                if faktor.is_none() {
                    r.offen.push("Maßstab / Kalibrierung fehlt".into());
                }
                if let Some(a) = ausschnitt
                    && !m.geometrie.punkte().iter().all(|p| a.enthaelt(p))
                {
                    r.offen.push(format!("Liegt außerhalb des Ausschnitts „{}“", a.bezeichnung));
                }
                if m.bedeutung == Planbedeutung::Unbestimmt {
                    r.offen.push("Fachliche Bedeutung fehlt".into());
                }
                if m.geschoss_id.is_some() && m.gebaeude_id.is_none() {
                    return Err(fehler("Geschoss benötigt eine Gebäudezuordnung"));
                }
                for n in [
                    r.flaeche_m2,
                    r.laenge_m,
                    r.hoehe_m,
                    r.horizontale_m,
                    r.neigung_grad,
                ]
                .into_iter()
                .flatten()
                {
                    if !n.is_finite() || n.abs() > 1e12 {
                        return Err(fehler("Messwert außerhalb des Wertebereichs"));
                    }
                }
                Ok(r)
            })
            .collect()
    }
}
