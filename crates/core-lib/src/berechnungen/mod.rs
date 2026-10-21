//! Eigenständige Fachkerne mit gemeinsamem Ergebnisvertrag, ohne UI-/Datenbankabhängigkeit.
pub mod din277;
pub mod entwaesserung;
mod formulare;
pub mod freiflaechen;
mod geschossbindung;
pub mod hoehen;
mod raumbuch;
pub use geschossbindung::Einstufungsquelle;
mod service;
pub mod stellplaetze;
pub mod ueberbaubarkeit;
pub mod vollgeschosse;
pub mod wohnflaechen;
use crate::{CoreError, CoreResult, plattform::ObjektArt};
pub use formulare::{Auswahl, BerechnungsVorlage, Feld, FormTabelle, vorlagen};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum BerechnungsArt {
    Stellplaetze,
    Wohnflaechen,
    Din277,
    Vollgeschosse,
    Hoehen,
    Freiflaechen,
    Entwaesserung,
    Ueberbaubarkeit,
}
impl BerechnungsArt {
    pub const ALLE: [Self; 8] = [
        Self::Stellplaetze,
        Self::Wohnflaechen,
        Self::Din277,
        Self::Vollgeschosse,
        Self::Hoehen,
        Self::Freiflaechen,
        Self::Entwaesserung,
        Self::Ueberbaubarkeit,
    ];
    pub fn rechenstand(self) -> &'static str {
        match self {
            Self::Stellplaetze => stellplaetze::RECHENSTAND,
            Self::Wohnflaechen => wohnflaechen::RECHENSTAND,
            Self::Din277 => din277::RECHENSTAND,
            Self::Vollgeschosse => vollgeschosse::RECHENSTAND,
            Self::Hoehen => hoehen::RECHENSTAND,
            Self::Freiflaechen => freiflaechen::RECHENSTAND,
            Self::Entwaesserung => entwaesserung::RECHENSTAND,
            Self::Ueberbaubarkeit => ueberbaubarkeit::RECHENSTAND,
        }
    }
    pub fn objekt(self) -> ObjektArt {
        match self {
            Self::Stellplaetze => ObjektArt::Stellplaetze,
            Self::Wohnflaechen => ObjektArt::Wohnflaechen,
            Self::Din277 => ObjektArt::Din277,
            Self::Vollgeschosse => ObjektArt::Vollgeschosse,
            Self::Hoehen => ObjektArt::Hoehen,
            Self::Freiflaechen => ObjektArt::Freiflaechen,
            Self::Entwaesserung => ObjektArt::Entwaesserung,
            Self::Ueberbaubarkeit => ObjektArt::Ueberbaubarkeit,
        }
    }
    pub fn titel(self) -> &'static str {
        match self {
            Self::Stellplaetze => "Stellplatznachweis",
            Self::Wohnflaechen => "Wohnflächenberechnung",
            Self::Din277 => "Flächen und Rauminhalte nach DIN 277",
            Self::Vollgeschosse => "Vollgeschossprüfung",
            Self::Hoehen => "Gebäudehöhen und Dachneigung",
            Self::Freiflaechen => "Versiegelungs- und Freiflächenbilanz",
            Self::Entwaesserung => "Regenwasser und Rückhaltung",
            Self::Ueberbaubarkeit => "Überbaubare Grundstücksfläche",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct Regelkontext {
    pub bezeichnung: String,
    pub version: String,
    pub quelle: String,
    pub geltungsbereich: String,
    pub gueltig_ab: String,
    pub gueltig_bis: String,
    pub anwendbarkeit_bestaetigt: bool,
    pub pruefvermerk: String,
}
impl Default for Regelkontext {
    fn default() -> Self {
        Self {
            bezeichnung: String::new(),
            version: "1".into(),
            quelle: String::new(),
            geltungsbereich: String::new(),
            gueltig_ab: String::new(),
            gueltig_bis: String::new(),
            anwendbarkeit_bestaetigt: false,
            pruefvermerk: String::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Einheit {
    M,
    M2,
    M3,
    Anzahl,
    Grad,
    Prozent,
    LProS,
}
impl Einheit {
    pub fn text(self) -> &'static str {
        match self {
            Self::M => "m",
            Self::M2 => "m²",
            Self::M3 => "m³",
            Self::Anzahl => "Anzahl",
            Self::Grad => "°",
            Self::Prozent => "%",
            Self::LProS => "l/s",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Messgroesse {
    Wohnflaeche,
    Versiegelungsansatz,
    Gruenansatz,
    Nutzungsflaeche,
    Verkaufsflaeche,
    Grundflaeche,
    Wohneinheiten,
    Sitzplaetze,
    Betten,
    Beschaeftigte,
    Bgf,
    Nrf,
    Kgf,
    Tf,
    Vf,
    Bri,
    Hoehe,
    Volumen,
    Stellplaetze,
    Sonstiges,
}
impl Messgroesse {
    pub fn text(self) -> &'static str {
        match self {
            Self::Wohnflaeche => "Wohnfläche (m²)",
            Self::Versiegelungsansatz => "Gewichteter Versiegelungsansatz (m²)",
            Self::Gruenansatz => "Anrechenbarer Grünansatz (m²)",
            Self::Nutzungsflaeche => "Nutzungsfläche (m²)",
            Self::Verkaufsflaeche => "Verkaufsfläche (m²)",
            Self::Grundflaeche => "Körperliche Grundfläche (m²)",
            Self::Wohneinheiten => "Wohneinheiten",
            Self::Sitzplaetze => "Sitzplätze",
            Self::Betten => "Betten",
            Self::Beschaeftigte => "Beschäftigte",
            Self::Bgf => "Brutto-Grundfläche",
            Self::Nrf => "Netto-Raumfläche",
            Self::Kgf => "Konstruktions-Grundfläche",
            Self::Tf => "Technikfläche",
            Self::Vf => "Verkehrsfläche",
            Self::Bri => "Brutto-Rauminhalt",
            Self::Hoehe => "Höhe",
            Self::Volumen => "Volumen",
            Self::Stellplaetze => "Stellplätze",
            Self::Sonstiges => "Sonstige Größe",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Uebernahme {
    pub nachweis_id: Uuid,
    pub kennzahl: String,
    pub ziel: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fachquelle {
    pub art: ObjektArt,
    pub objekt: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FachPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<crate::planaufmass::Planbindung>>")]
    pub planbindungen: Vec<crate::planaufmass::Planbindung>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gebaeudedaten_uebernehmen: Option<bool>,
    pub modul: BerechnungsArt,
    pub bezeichnung: String,
    pub grundstueck_id: Option<Uuid>,
    pub gebaeude_id: Option<Uuid>,
    pub regelkontext: Regelkontext,
    pub quellen: Vec<crate::plattform::objekte::Dokumentbezug>,
    #[serde(default)]
    pub fachquellen: Vec<Fachquelle>,
    #[serde(default)]
    pub uebernahmen: Vec<Uebernahme>,
    #[ts(type = "JsonValue")]
    pub eingaben: serde_json::Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Kennzahl {
    pub id: String,
    pub bezeichnung: String,
    pub wert: Option<f64>,
    pub einheit: Einheit,
    pub groesse: Messgroesse,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachweistabelle {
    pub titel: String,
    pub spalten: Vec<String>,
    pub zeilen: Vec<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct FachErgebnis {
    pub rechenstand: String,
    pub modul: BerechnungsArt,
    pub vollstaendig: bool,
    pub offen: Vec<String>,
    pub hinweise: Vec<String>,
    pub tabellen: Vec<Nachweistabelle>,
    pub kennzahlen: Vec<Kennzahl>,
}
impl FachErgebnis {
    pub(super) fn neu(modul: BerechnungsArt) -> Self {
        Self {
            rechenstand: modul.rechenstand().into(),
            modul,
            vollstaendig: true,
            offen: vec![],
            hinweise: vec![],
            tabellen: vec![],
            kennzahlen: vec![],
        }
    }
    pub(super) fn offen(&mut self, t: impl Into<String>) {
        self.offen.push(t.into());
        self.vollstaendig = false;
    }
    pub(super) fn tabelle(&mut self, titel: &str, spalten: &[&str], zeilen: Vec<Vec<String>>) {
        self.tabellen.push(Nachweistabelle {
            titel: titel.into(),
            spalten: spalten.iter().map(|s| s.to_string()).collect(),
            zeilen,
        });
    }
    pub(super) fn kennzahl(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        wert: Option<f64>,
        einheit: Einheit,
        groesse: Messgroesse,
    ) {
        self.kennzahlen.push(Kennzahl {
            id: id.into(),
            bezeichnung: name.into(),
            wert,
            einheit,
            groesse,
        });
    }
}
pub(super) fn err(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
pub(super) fn zahl(v: f64) -> String {
    format!("{v:.4}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
pub(super) fn wert(v: f64, name: &str) -> CoreResult<()> {
    if !v.is_finite() || !(0. ..=1e12).contains(&v) {
        Err(err(format!(
            "{name}: endlicher Wert zwischen 0 und 10¹² erforderlich"
        )))
    } else {
        Ok(())
    }
}
pub(super) fn quote(v: f64, name: &str) -> CoreResult<()> {
    wert(v, name)?;
    if v > 1. {
        Err(err(format!("{name}: Faktor zwischen 0 und 1 erforderlich")))
    } else {
        Ok(())
    }
}
pub(super) fn text(s: &str) -> CoreResult<()> {
    if s.trim().is_empty() || s.len() > 4000 {
        Err(err(
            "Bezeichnung / Grundlage mit 1 bis 4.000 Zeichen erforderlich",
        ))
    } else {
        Ok(())
    }
}
/// Wie [`text`], die Meldung nennt Position und Feld (z. B. „Position „EG.01 Wohnen“: Wohnung“).
pub(super) fn text_feld(s: &str, wo: impl std::fmt::Display) -> CoreResult<()> {
    text(s).map_err(|_| err(format!("{wo}: Angabe mit 1 bis 4.000 Zeichen erforderlich")))
}
pub(super) fn ids<'a>(v: impl Iterator<Item = &'a str>) -> CoreResult<()> {
    let mut ids = std::collections::HashSet::new();
    for id in v {
        text(id)?;
        if id.len() > 100
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(err(
                "Positionskennung: nur Buchstaben, Ziffern, Bindestrich und Unterstrich",
            ));
        }
        if !ids.insert(id) {
            return Err(err("Doppelte Positionskennung"));
        }
        if ids.len() > 2000 {
            return Err(err("Höchstens 2.000 Positionen"));
        }
    }
    Ok(())
}
pub(super) fn decode<T: serde::de::DeserializeOwned>(v: &serde_json::Value) -> CoreResult<T> {
    serde_json::from_value(v.clone()).map_err(|e| err(format!("Berechnungseingaben: {e}")))
}
/// Nur Rechenlogik. Quellen-/Projekt- und Übernahmeprüfung liegt in CoreServices.
pub fn berechnen(p: &FachPlan) -> CoreResult<FachErgebnis> {
    text_feld(&p.bezeichnung, "Bezeichnung der Berechnung")?;
    text_feld(&p.regelkontext.bezeichnung, "Regelkontext: Bezeichnung")?;
    text_feld(&p.regelkontext.version, "Regelkontext: Version / Fassung")?;
    for s in [
        &p.regelkontext.quelle,
        &p.regelkontext.geltungsbereich,
        &p.regelkontext.pruefvermerk,
    ] {
        if s.len() > 10000 {
            return Err(err("Regelvermerk zu lang"));
        }
    }
    let mut r = match p.modul {
        BerechnungsArt::Stellplaetze => stellplaetze::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Wohnflaechen => wohnflaechen::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Din277 => din277::berechnen(&din277::eingabe(&p.eingaben)?)?,
        BerechnungsArt::Vollgeschosse => vollgeschosse::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Hoehen => hoehen::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Ueberbaubarkeit => ueberbaubarkeit::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Freiflaechen => freiflaechen::berechnen(&decode(&p.eingaben)?)?,
        BerechnungsArt::Entwaesserung => entwaesserung::berechnen(&decode(&p.eingaben)?)?,
    };
    if p.grundstueck_id.is_none() {
        r.offen("Berechnung einer aktuellen Baugrundstückseinheit zuordnen");
    }
    if !p.regelkontext.anwendbarkeit_bestaetigt
        || p.regelkontext.quelle.trim().is_empty()
        || p.regelkontext.pruefvermerk.trim().is_empty()
    {
        r.offen("Anwendbare Rechts-/Satzungs-/Methodenfassung, Quelle und Prüfvermerk durch Projektleitung bestätigen");
    }
    for datum in [&p.regelkontext.gueltig_ab, &p.regelkontext.gueltig_bis] {
        if !datum.is_empty() {
            chrono::NaiveDate::parse_from_str(datum, "%Y-%m-%d")
                .map_err(|_| err("Gültigkeitsdatum als JJJJ-MM-TT angeben"))?;
        }
    }
    if !p.regelkontext.gueltig_ab.is_empty()
        && !p.regelkontext.gueltig_bis.is_empty()
        && p.regelkontext.gueltig_ab > p.regelkontext.gueltig_bis
    {
        return Err(err("Gültigkeitszeitraum ist vertauscht"));
    }
    if r.kennzahlen
        .iter()
        .any(|k| k.wert.is_some_and(|v| !v.is_finite() || v.abs() > 1e15))
    {
        return Err(err("Ergebnis außerhalb des zulässigen Zahlenbereichs"));
    }
    Ok(r)
}
#[cfg(test)]
mod tests;
/// Einheitliche Zahlen-/Enumdarstellung für Web/Native-Requests und idempotente Nachweise.
pub(crate) fn normalisieren(p: &mut FachPlan) -> CoreResult<()> {
    p.eingaben = match p.modul {
        BerechnungsArt::Stellplaetze => {
            serde_json::to_value(decode::<stellplaetze::Eingabe>(&p.eingaben)?)
        }
        BerechnungsArt::Wohnflaechen => {
            serde_json::to_value(decode::<wohnflaechen::Eingabe>(&p.eingaben)?)
        }
        BerechnungsArt::Din277 => serde_json::to_value(din277::eingabe(&p.eingaben)?),
        BerechnungsArt::Vollgeschosse => {
            serde_json::to_value(decode::<vollgeschosse::Eingabe>(&p.eingaben)?)
        }
        BerechnungsArt::Hoehen => serde_json::to_value(decode::<hoehen::Eingabe>(&p.eingaben)?),
        BerechnungsArt::Ueberbaubarkeit => {
            serde_json::to_value(decode::<ueberbaubarkeit::Eingabe>(&p.eingaben)?)
        }
        BerechnungsArt::Freiflaechen => {
            serde_json::to_value(decode::<freiflaechen::Eingabe>(&p.eingaben)?)
        }
        BerechnungsArt::Entwaesserung => {
            serde_json::to_value(decode::<entwaesserung::Eingabe>(&p.eingaben)?)
        }
    }
    .map_err(|e| err(e.to_string()))?;
    Ok(())
}
