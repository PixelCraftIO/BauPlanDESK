//! PDF-Aufmaß: unveränderte Dokumentversion + unabhängig gespeicherte Zeichenmaske.
//! Öffentlicher Vertrag und Koordinatenkonvention: docs/planaufmass.md.
mod abstand;
mod gebaeude;
mod raumbuch;
mod service;
pub use abstand::abstandsziele;
pub use gebaeude::gebaeudeziele;
pub use service::{Planziel, planziele};
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
}
impl Plankalibrierung {
    pub fn meter_pro_punkt(&self) -> CoreResult<f64> {
        let faktor = match self {
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
    Polygon { punkte: Vec<Planpunkt> },
    Rechteck { von: Planpunkt, bis: Planpunkt },
    Kreis { mitte: Planpunkt, rand: Planpunkt },
    Strecke { von: Planpunkt, bis: Planpunkt },
    Hoehenpunkt { punkt: Planpunkt },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Planbedeutung {
    Unbestimmt,
    Flurstueck,
    PrivateGruenflaeche,
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
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planhoehenbezug {
    pub punkt: Planpunkt,
    pub hoehe_m: f64,
    /// Zum Beispiel „m ü. NHN“ oder „lokal, OKFF EG = 0“.
    pub system: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planaufmass {
    pub bezeichnung: String,
    pub dokument: Dokumentbezug,
    /// Einsbasiert. Mehrere Aufmaße derselben Seite dürfen verschiedene Maßstäbe haben.
    pub seite: u32,
    pub kalibrierung: Option<Plankalibrierung>,
    pub hoehenbezug: Option<Planhoehenbezug>,
    pub ebenen: Vec<Planebene>,
    pub markierungen: Vec<Planmarkierung>,
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
    pub fn messen(&self) -> CoreResult<Vec<Planmesswert>> {
        crate::plattform::text(&self.bezeichnung, 500)?;
        if self.seite == 0
            || self.dokument.version < 1
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
        let faktor = self
            .kalibrierung
            .as_ref()
            .map(Plankalibrierung::meter_pro_punkt)
            .transpose()?;
        let mut ids = std::collections::HashSet::new();
        self.markierungen
            .iter()
            .map(|m| {
                crate::plattform::text(&m.bezeichnung, 500)?;
                if !ids.insert(m.id) || !ebenen.contains(&m.ebene_id) {
                    return Err(fehler("Markierung doppelt oder Zeichenebene fehlt"));
                }
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
                        let dx = (a[0] - b[0]).abs();
                        let dy = (a[1] - b[1]).abs();
                        r.horizontale_m = faktor.map(|f| dx * f);
                        // Positive Höhendifferenz; absolute Höhen ausschließlich mit Höhenbezug.
                        r.hoehe_m = faktor.map(|f| dy * f);
                        r.neigung_grad = faktor.map(|_| dy.atan2(dx).to_degrees());
                        (None, Some(l))
                    }
                    Plangeometrie::Hoehenpunkt { punkt: p } => {
                        punkt(p)?;
                        r.hoehe_m = faktor
                            .zip(self.hoehenbezug.as_ref())
                            .map(|(f, h)| h.hoehe_m + (h.punkt[1] - p[1]) * f);
                        if self.hoehenbezug.is_none() {
                            r.offen.push("Höhenbezug fehlt".into());
                        }
                        (None, None)
                    }
                };
                r.flaeche_m2 = area.zip(faktor).map(|(a, f)| a * f * f);
                r.laenge_m = length.zip(faktor).map(|(l, f)| l * f);
                if faktor.is_none() {
                    r.offen.push("Maßstab / Kalibrierung fehlt".into());
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
