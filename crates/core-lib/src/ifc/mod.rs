//! IFC-Modelle (ISO 16739) als Unterlage: lesen, prüfen, nacharbeiten,
//! auswerten und schreiben.
//!
//! Öffentlicher Vertrag (Einzelheiten in docs/ifc.md):
//!
//! * Die IFC-Datei bleibt unverändert als Dokumentversion im DMS. Eine
//!   [`Modellakte`] (Fachobjekt `ifcmodell`) verweist auf sie.
//! * [`Modellauszug`] ist die schemaunabhängige Sicht auf ein Modell
//!   (IFC2X3, IFC4, IFC4X3): Struktur, Einheiten, Eigenschaften, Mengen und
//!   ausgewertete Geometrie in Metern. Er wird aus der Datei berechnet und nie
//!   gespeichert.
//! * Rechtliche Anrechnungen (Vollgeschoss, GRZ/GFZ, Abstandsflächen) bleiben
//!   bei ihren Fachmodulen. Das Modell liefert Flächen, Höhen und Herkunft.
mod abgleich;
mod auswertung;
mod auszug;
mod export;
pub mod geometrie;
mod grundriss;
pub mod nacharbeit;
mod pruefung;
mod service;
pub mod step;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;
#[cfg(test)]
mod tests;
mod uebernahme;

pub use abgleich::{
    Elementaenderung, Elementverweis, Modellabgleich, Nacharbeitsstand, Reimportwunsch, abgleichen,
};
pub use auswertung::{
    Bezugsflaeche, Geschossauswertung, Modellauswertung, Modellhoehen, Nutzungsflaeche, auswerten,
};
pub use auszug::{auszug, klassentitel};
pub use export::{
    Entwurf, Entwurfsgeschoss, Entwurfsraum, Entwurfswand, Entwurfszuordnung, Exportbericht,
    Exportergebnis, Exportwunsch, entwurf_kontrolle, entwurf_schreiben, kontrolle,
    nacharbeit_schreiben,
};
pub use grundriss::{
    Grundrissteil, Modellgeschoss, Modellgrundriss, SCHNITTHOEHE_M, als_plan, geschossliste,
    grundriss, modellumfang,
};
pub use nacharbeit::{Eigenschaftswert, Modellaenderung, Raumvorschlag, raumvorschlaege};
pub use pruefung::{
    BAUANTRAG_IDS, Befund, Befundaktion, Befundstufe, Modellpruefung, eingebaute_regeln,
    ids_pruefen, pruefen,
};
pub use service::{Modell, ist_ifc};
pub use uebernahme::{
    Geschossuebernahme, Modellbezug, Modelluebernahme, Modelluebernahmeergebnis,
    Modelluebernahmewunsch, RAND_M, raumhoehe, zeichnungspunkt,
};

use crate::planaufmass::Planpolygon;
use crate::plattform::objekte::Dokumentbezug;
use serde::{Deserialize, Serialize};

pub const RECHENSTAND: &str = "ifc-1";

/// Fachobjekt `ifcmodell`: ein importiertes IFC-Modell des Projekts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Modellakte {
    pub bezeichnung: String,
    /// IFC-Datei (`.ifc` oder `.ifczip`) im DMS.
    pub dokument: Dokumentbezug,
    /// Letzte Übernahme in Gebäude, Zeichnungen und Raumbücher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub uebernahme: Option<Modelluebernahme>,
    /// Nacharbeit: Änderungen über die GlobalId, in dieser Reihenfolge angewendet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Modellaenderung>>")]
    pub nacharbeit: Vec<Modellaenderung>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Georeferenz {
    /// Koordinatenreferenzsystem, z. B. `EPSG:25832`.
    pub crs: Option<String>,
    pub ostwert: Option<f64>,
    pub nordwert: Option<f64>,
    pub hoehe: Option<f64>,
    /// Drehung der Modell-x-Achse gegen Ost, Grad gegen den Uhrzeigersinn.
    pub drehung_grad: Option<f64>,
    /// IFC2X3: geographische Lage des Grundstücks in Dezimalgrad.
    pub breite: Option<f64>,
    pub laenge: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Mengenart {
    Laenge,
    Flaeche,
    Volumen,
    Anzahl,
    Gewicht,
    Zeit,
}

/// Eigenschaft aus einem Eigenschaftssatz (Pset), Zahlen in SI-Einheiten.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Modelleigenschaft {
    pub satz: String,
    pub name: String,
    /// Anzeigewert (z. B. „Ja“, „2,50 m“, „Wohnen“).
    pub wert: String,
    /// Zahlenwert in SI-Einheit (m, m², m³), sofern es eine Zahl ist.
    pub zahl: Option<f64>,
    /// Vom Typ (`IfcRelDefinesByType`) geerbt statt am Element gesetzt.
    pub vom_typ: bool,
}

/// Basismenge (z. B. `Qto_SpaceBaseQuantities.NetFloorArea`) in SI-Einheit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Modellmenge {
    pub satz: String,
    pub name: String,
    pub art: Mengenart,
    pub wert: f64,
}

/// Ausgewertete Geometrie eines Elements in Metern im Weltsystem des Modells
/// (x, y waagerecht, y nach Norden bzw. Modell-y, z nach oben).
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellgeometrie {
    /// Senkrechte Projektion (Grundriss) aller Körperteile.
    pub grundriss: Vec<Planpolygon>,
    pub grundflaeche_m2: f64,
    pub unten_m: f64,
    pub oben_m: f64,
    pub volumen_m3: Option<f64>,
    /// Boolesche Operationen oder schräge Körper angenähert.
    pub naeherung: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Strukturelement {
    pub global_id: String,
    pub name: String,
    pub langname: Option<String>,
    /// Übergeordnetes Strukturelement (Grundstück → Gebäude → Geschoss).
    pub teil_von: Option<String>,
    /// Geschosse: Höhe der Geschossebene über dem Projektnullpunkt (m).
    pub hoehe_m: Option<f64>,
    pub eigenschaften: Vec<Modelleigenschaft>,
    pub mengen: Vec<Modellmenge>,
    pub geometrie: Option<Modellgeometrie>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellelement {
    pub global_id: String,
    /// Entitätsname in Großbuchstaben, z. B. `IFCWALL`, `IFCSPACE`.
    pub klasse: String,
    pub name: String,
    pub langname: Option<String>,
    pub objekttyp: Option<String>,
    /// `PredefinedType` (z. B. `FLOOR`, `ROOF`) oder Innen/Außen bei IFC2X3-Räumen.
    pub vordefiniert: Option<String>,
    /// Geschoss (GlobalId), dem das Element über die räumliche Struktur angehört.
    pub geschoss: Option<String>,
    /// Gebäude (GlobalId) des Elements, auch ohne Geschoss.
    pub gebaeude: Option<String>,
    /// Übergeordnetes Element (z. B. Dach → Dachplatten) oder Raum.
    pub teil_von: Option<String>,
    pub eigenschaften: Vec<Modelleigenschaft>,
    pub mengen: Vec<Modellmenge>,
    pub geometrie: Option<Modellgeometrie>,
    /// Nicht ausgewertete Darstellungsarten der Geometrie.
    pub geometrie_offen: Vec<String>,
    #[serde(skip)]
    #[ts(skip)]
    pub step_id: u64,
    #[serde(skip)]
    #[ts(skip)]
    pub teile: Vec<geometrie::Teil>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Klassenzahl {
    pub klasse: String,
    pub titel: String,
    pub anzahl: usize,
}

/// „Was steckt im Modell“: Übersicht für den Import.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellbericht {
    pub klassen: Vec<Klassenzahl>,
    pub elemente: usize,
    pub mit_geometrie: usize,
    pub ohne_geometrie: usize,
    pub naeherungen: usize,
    pub raeume: usize,
    pub raeume_mit_flaechenmenge: usize,
    /// Darstellungsarten, die nicht ausgewertet wurden, mit Anzahl Elemente.
    pub offene_darstellungen: Vec<Klassenzahl>,
    pub hinweise: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellauszug {
    pub rechenstand: String,
    /// `IFC2X3`, `IFC4`, `IFC4X3_ADD2` …
    pub schema: String,
    /// Erzeugende Software laut Dateikopf.
    pub system: Option<String>,
    pub projekt: Option<String>,
    /// Längeneinheit der Datei, z. B. „Millimeter“.
    pub einheit: String,
    pub meter_je_einheit: f64,
    pub georeferenz: Option<Georeferenz>,
    pub grundstuecke: Vec<Strukturelement>,
    pub gebaeude: Vec<Strukturelement>,
    /// Geschosse nach Höhe sortiert.
    pub geschosse: Vec<Strukturelement>,
    pub elemente: Vec<Modellelement>,
    pub bericht: Modellbericht,
}

impl Modellauszug {
    pub fn element(&self, global_id: &str) -> Option<&Modellelement> {
        self.elemente.iter().find(|e| e.global_id == global_id)
    }
    pub fn geschoss(&self, global_id: &str) -> Option<&Strukturelement> {
        self.geschosse.iter().find(|g| g.global_id == global_id)
    }
}
