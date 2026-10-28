//! Grundriss-Schnitt eines Geschosses: alle Körperteile, die die Schnittebene
//! (Geschosshöhe + Schnitthöhe) schneiden, und die Räume des Geschosses.
//! Hüllen (Flächenmodelle, schräge Körper) gehen mit ihrer Projektion ein.
use super::geometrie::{Teil, multipolygon, planpolygone, vereinen};
use super::{Modellauszug, klassentitel};
use crate::cad::{Ebene, Laengeneinheit, Linienzug, Plan, Punkt, Rechteck};
use crate::planaufmass::Planpolygon;
use crate::{CoreError, CoreResult};
use serde::Serialize;

/// Übliche Schnitthöhe über der Geschossebene (m).
pub const SCHNITTHOEHE_M: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Grundrissteil {
    pub global_id: String,
    pub klasse: String,
    pub titel: String,
    pub name: String,
    /// Meter im Weltsystem des Modells (y nach Modell-y, nicht gespiegelt).
    pub polygone: Vec<Planpolygon>,
    pub raum: bool,
    /// Projektion statt Schnitt (Flächenmodell oder schräger Körper).
    pub projektion: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellgrundriss {
    pub geschoss: String,
    pub schnitthoehe_m: f64,
    pub teile: Vec<Grundrissteil>,
    /// Umfang aller Elemente des Modells (nicht nur dieses Geschosses), damit
    /// die Grundrisse aller Geschosse deckungsgleich liegen.
    pub min: [f64; 2],
    pub max: [f64; 2],
}

/// Geschoss zur Auswahl (z. B. für IFC-Hintergründe).
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellgeschoss {
    pub global_id: String,
    pub name: String,
    pub hoehe_m: Option<f64>,
}

pub fn geschossliste(a: &Modellauszug) -> Vec<Modellgeschoss> {
    a.geschosse
        .iter()
        .map(|g| Modellgeschoss {
            global_id: g.global_id.clone(),
            name: if g.name.is_empty() {
                g.langname.clone().unwrap_or_else(|| "Geschoss".into())
            } else {
                g.name.clone()
            },
            hoehe_m: g.hoehe_m,
        })
        .collect()
}

/// Umfang aller Elementgrundrisse des Modells.
pub fn modellumfang(a: &Modellauszug) -> ([f64; 2], [f64; 2]) {
    let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for e in &a.elemente {
        for p in e.geometrie.iter().flat_map(|g| g.grundriss.iter()) {
            for q in &p.aussen {
                for k in 0..2 {
                    min[k] = min[k].min(q[k]);
                    max[k] = max[k].max(q[k]);
                }
            }
        }
    }
    if min[0].is_finite() {
        (min, max)
    } else {
        ([0., 0.], [1., 1.])
    }
}

pub fn grundriss(
    a: &Modellauszug,
    geschoss: &str,
    schnitthoehe_m: Option<f64>,
) -> CoreResult<Modellgrundriss> {
    let g = a
        .geschoss(geschoss)
        .ok_or_else(|| CoreError::InvalidInput("Geschoss ist nicht im Modell".into()))?;
    let schnitt = schnitthoehe_m.unwrap_or(SCHNITTHOEHE_M);
    if !schnitt.is_finite() || !(-10. ..=50.).contains(&schnitt) {
        return Err(CoreError::InvalidInput(
            "Schnitthöhe zwischen −10 und 50 m angeben".into(),
        ));
    }
    let z = g.hoehe_m.unwrap_or(0.) + schnitt;
    let mut teile = Vec::new();
    for e in &a.elemente {
        let raum = e.klasse == "IFCSPACE";
        let (polys, projektion): (Vec<geo::Polygon<f64>>, bool) = if raum {
            if e.geschoss.as_deref() != Some(geschoss) {
                continue;
            }
            (
                e.geometrie
                    .as_ref()
                    .map(|g| multipolygon(&g.grundriss).0)
                    .unwrap_or_default(),
                false,
            )
        } else {
            let mut proj = false;
            let p = e
                .teile
                .iter()
                .filter(|t| {
                    let (u, o) = t.hoehen();
                    u - 1e-6 <= z && z <= o + 1e-6
                })
                .inspect(|t| proj |= matches!(t, Teil::Huelle { .. }))
                .flat_map(|t| t.grundriss().0.clone())
                .collect();
            (p, proj)
        };
        if polys.is_empty() {
            continue;
        }
        teile.push(Grundrissteil {
            global_id: e.global_id.clone(),
            klasse: e.klasse.clone(),
            titel: klassentitel(&e.klasse),
            name: e.name.clone(),
            polygone: planpolygone(&vereinen(polys)),
            raum,
            projektion,
        });
    }
    let (min, max) = modellumfang(a);
    Ok(Modellgrundriss {
        geschoss: geschoss.into(),
        schnitthoehe_m: schnitt,
        teile,
        min,
        max,
    })
}

/// Grundriss als DXF-artiger Plan für den Zeicheneditor: Meter, eine Ebene je
/// Klasse, Ringe als geschlossene Linienzüge. So zeichnet und fängt der Editor
/// IFC-Hintergründe wie DXF-Hintergründe.
pub fn als_plan(g: &Modellgrundriss) -> Plan {
    let mut ebenen: Vec<Ebene> = Vec::new();
    let mut linien = Vec::new();
    for t in &g.teile {
        let name = if t.raum {
            "Räume".to_string()
        } else {
            t.titel.clone()
        };
        let ebene = match ebenen.iter().position(|e| e.name == name) {
            Some(i) => i,
            None => {
                ebenen.push(Ebene {
                    farbe: Some(if t.raum { "#7a8a99" } else { "#222222" }.into()),
                    name,
                    sichtbar: true,
                });
                ebenen.len() - 1
            }
        };
        for p in &t.polygone {
            for r in std::iter::once(&p.aussen).chain(p.innen.iter()) {
                linien.push(Linienzug {
                    ebene,
                    farbe: None,
                    punkte: r.iter().map(|q| Punkt(q[0], q[1])).collect(),
                    geschlossen: true,
                });
            }
        }
    }
    Plan {
        einheit: Some(Laengeneinheit::Meter),
        ebenen,
        linien,
        texte: Vec::new(),
        raumpunkte: Vec::new(),
        grenzen: Some(Rechteck {
            min: Punkt(g.min[0], g.min[1]),
            max: Punkt(g.max[0], g.max[1]),
        }),
        hinweise: vec![format!(
            "IFC-Grundriss, Schnitt {} m über der Geschossebene",
            cad_zahl(g.schnitthoehe_m)
        )],
    }
}

fn cad_zahl(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}
