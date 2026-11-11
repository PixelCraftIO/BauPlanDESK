//! Gegenproben zwischen Zeichnungen: Was in zwei Plänen gezeichnet ist, soll
//! übereinstimmen. Abweichungen über der Toleranz sind Hinweise, keine Sperren.
use super::*;
use crate::plattform::objekte::Gebaeude;
use std::collections::BTreeMap;

/// Eine Abweichung zwischen zwei Zeichnungen desselben Gegenstands.
#[derive(Debug, Clone, PartialEq)]
pub struct Gegenprobe {
    /// Stabil je Gegenstand, für die Projektübersicht.
    pub schluessel: String,
    pub titel: String,
    pub text: String,
    /// Aufmaß, in dem die Prüfung beginnt.
    pub aufmass: Uuid,
}

/// Eine Zeichnung eines Gegenstands: Aufmaß und Ausschnitt.
type Quelle = (Uuid, Option<Uuid>);
/// Je Zeichnung: Ort als Text und der dort gezeichnete Wert.
type Zeichnungen<T> = BTreeMap<Quelle, (String, T)>;

fn zahl(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// Das Geschoss am Gelände: dem Namen nach, sonst mit dem Fußboden nächst ±0.
fn erdgeschoss(g: &Gebaeude) -> Option<(Uuid, String)> {
    let name = |s: &crate::plattform::objekte::GebaeudeGeschoss| {
        let b = s.bezeichnung.trim().to_lowercase();
        b == "eg" || b.starts_with("eg ") || b.contains("erdgeschoss")
    };
    g.geschosse
        .iter()
        .find(|s| name(s))
        .or_else(|| {
            g.geschosse
                .iter()
                .filter_map(|s| {
                    s.geometrie
                        .as_ref()
                        .and_then(|x| x.fussboden_m)
                        .map(|f| (f.abs(), s))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .filter(|(f, _)| *f <= 1.5)
                .map(|(_, s)| s)
        })
        .map(|s| (s.id, s.bezeichnung.trim().to_string()))
}

/// Vergleicht doppelt Gezeichnetes über alle Aufmaße eines Projekts:
/// Flurstück, Gebäudegrundfläche und Geschoss-Außenkante je Zeichnung, die
/// Gebäudegrundfläche gegen die Außenkante des Erdgeschosses sowie First und
/// Traufe zwischen Schnitten und Ansichten gleichen Höhensystems.
/// `flurstuecke` liefert die Bezeichnung zu einer Flurstückskennung.
pub fn gegenproben(
    aufmasse: &[(Uuid, Planaufmass)],
    gebaeude: &[(Uuid, Gebaeude)],
    flurstuecke: &BTreeMap<String, String>,
) -> Vec<Gegenprobe> {
    // Gegenstand → Zeichnung → (Ort, Summe).
    let mut flaechen: BTreeMap<(u8, String, String), Zeichnungen<f64>> = BTreeMap::new();
    // (Bedeutung, Gebäude, Höhensystem) → Zeichnung → (Ort, Höhen).
    let mut hoehen: BTreeMap<(u8, Uuid, String), Zeichnungen<Vec<f64>>> = BTreeMap::new();
    for (id, a) in aufmasse {
        let Ok(werte) = a.messen() else { continue };
        for m in &a.markierungen {
            let Some(w) = werte.iter().find(|w| w.id == m.id) else {
                continue;
            };
            let ausschnitt = m
                .ausschnitt_id
                .and_then(|x| a.ausschnitte.iter().find(|s| s.id == x));
            let ort = match ausschnitt {
                Some(s) if s.bezeichnung.trim() != a.bezeichnung.trim() => {
                    format!("{} · {}", a.bezeichnung.trim(), s.bezeichnung.trim())
                }
                _ => a.bezeichnung.trim().to_string(),
            };
            let quelle = (*id, m.ausschnitt_id);
            let gegenstand = match m.bedeutung {
                Planbedeutung::Flurstueck => m
                    .flurstueck_id
                    .as_ref()
                    .map(|f| (0, f.clone(), String::new())),
                Planbedeutung::Gebaeudegrundflaeche => {
                    m.gebaeude_id.map(|g| (1, g.to_string(), String::new()))
                }
                Planbedeutung::GeschossAussenkante => m
                    .gebaeude_id
                    .zip(m.geschoss_id)
                    .map(|(g, s)| (2, g.to_string(), s.to_string())),
                _ => None,
            };
            if let (Some(k), Some(f)) = (gegenstand, w.flaeche_m2) {
                let e = flaechen
                    .entry(k)
                    .or_default()
                    .entry(quelle)
                    .or_insert((ort.clone(), 0.));
                e.1 += f;
            }
            let art = match m.bedeutung {
                Planbedeutung::First => Some(0),
                Planbedeutung::Traufe => Some(1),
                _ => None,
            };
            let system = ausschnitt
                .and_then(|s| s.bezugslinie.as_ref().map(|b| b.system.clone()))
                .or_else(|| a.hoehenbezug.as_ref().map(|h| h.system.clone()));
            if let (Some(art), Some(g), Some(h), Some(system), Plangeometrie::Hoehenpunkt { .. }) =
                (art, m.gebaeude_id, w.hoehe_m, system, &m.geometrie)
            {
                hoehen
                    .entry((art, g, system.trim().to_lowercase()))
                    .or_default()
                    .entry(quelle)
                    .or_insert((ort, Vec::new()))
                    .1
                    .push(h);
            }
        }
    }
    let gebaeudename = |id: &str| {
        gebaeude
            .iter()
            .find(|(g, _)| g.to_string() == id)
            .map(|(_, g)| g.bezeichnung.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Gebäude".into())
    };
    let geschossname = |gid: &str, sid: &str| {
        gebaeude
            .iter()
            .find(|(g, _)| g.to_string() == gid)
            .and_then(|(_, g)| g.geschosse.iter().find(|s| s.id.to_string() == sid))
            .map(|s| s.bezeichnung.trim().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Geschoss".into())
    };
    let liste = |q: &BTreeMap<Quelle, (String, f64)>, einheit: &str| {
        q.values()
            .map(|(ort, v)| format!("{ort}: {} {einheit}", zahl(*v)))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let mut out = Vec::new();
    for ((art, a, b), quellen) in &flaechen {
        if quellen.len() < 2 {
            continue;
        }
        let min = quellen.values().map(|v| v.1).fold(f64::INFINITY, f64::min);
        let max = quellen.values().map(|v| v.1).fold(0., f64::max);
        if abgleich::fast_gleich(max, min) {
            continue;
        }
        let was = match art {
            0 => format!(
                "Flurstück {}",
                flurstuecke.get(a).map(String::as_str).unwrap_or("")
            )
            .trim()
            .to_string(),
            1 => format!("Gebäudegrundfläche {}", gebaeudename(a)),
            _ => format!("Außenkante {} · {}", gebaeudename(a), geschossname(a, b)),
        };
        out.push(Gegenprobe {
            schluessel: format!("flaeche:{art}:{a}:{b}"),
            titel: format!("Gegenprobe: {was}"),
            text: format!(
                "In zwei Zeichnungen unterschiedlich groß: {} (Unterschied {} m²). Kontur oder Maßstab prüfen.",
                liste(quellen, "m²"),
                zahl(max - min)
            ),
            aufmass: quellen.keys().next().map(|q| q.0).unwrap_or_default(),
        });
    }
    // Gebäudegrundfläche im Lageplan gegen die Außenkante des Erdgeschosses.
    for (gid, g) in gebaeude {
        let Some((eg, eg_name)) = erdgeschoss(g) else {
            continue;
        };
        let grund = flaechen.get(&(1, gid.to_string(), String::new()));
        let kante = flaechen.get(&(2, gid.to_string(), eg.to_string()));
        let (Some(grund), Some(kante)) = (grund, kante) else {
            continue;
        };
        let (Some((q, (ort_g, fg))), Some((_, (ort_k, fk)))) =
            (grund.iter().next(), kante.iter().next())
        else {
            continue;
        };
        if abgleich::fast_gleich(*fg, *fk) {
            continue;
        }
        out.push(Gegenprobe {
            schluessel: format!("grundflaeche:{gid}"),
            titel: format!("Gegenprobe: Grundfläche {}", gebaeudename(&gid.to_string())),
            text: format!(
                "Gebäudegrundfläche ({ort_g}: {} m²) und Außenkante {eg_name} ({ort_k}: {} m²) weichen um {} m² ab. Bei Vorsprüngen oder Auskragungen ist das richtig, sonst Kontur oder Maßstab prüfen.",
                zahl(*fg),
                zahl(*fk),
                zahl((fg - fk).abs())
            ),
            aufmass: q.0,
        });
    }
    let toleranz = abgleich::toleranzen().gegenprobe_hoehe.m;
    for ((art, gid, _), quellen) in &hoehen {
        // Eine Traufe je Zeichnung ist vergleichbar; mehrere können verschiedene Dachseiten sein.
        let werte: BTreeMap<Quelle, (String, f64)> = quellen
            .iter()
            .filter(|(_, (_, h))| *art == 0 || h.len() == 1)
            .map(|(q, (ort, h))| {
                (
                    *q,
                    (
                        ort.clone(),
                        h.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    ),
                )
            })
            .collect();
        if werte.len() < 2 {
            continue;
        }
        let min = werte.values().map(|v| v.1).fold(f64::INFINITY, f64::min);
        let max = werte
            .values()
            .map(|v| v.1)
            .fold(f64::NEG_INFINITY, f64::max);
        if max - min <= toleranz {
            continue;
        }
        let was = if *art == 0 { "First" } else { "Traufe" };
        out.push(Gegenprobe {
            schluessel: format!("hoehe:{art}:{gid}"),
            titel: format!("Gegenprobe: {was} {}", gebaeudename(&gid.to_string())),
            text: format!(
                "{was} liegt in zwei Zeichnungen unterschiedlich hoch: {} (Unterschied {} m). Höhenpunkt oder Bezugslinie prüfen.",
                liste(&werte, "m"),
                zahl(max - min)
            ),
            aufmass: werte.keys().next().map(|q| q.0).unwrap_or_default(),
        });
    }
    out
}
