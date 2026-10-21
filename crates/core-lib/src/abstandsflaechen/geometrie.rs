use super::*;
use geo::{
    Area, BooleanOps, Distance, Euclidean, LineString, MultiPolygon, Polygon, Relate, Validation,
};
pub(super) fn punkt(p: &APunkt) -> CoreResult<()> {
    if !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 10_000_000. || p.y.abs() > 10_000_000. {
        Err(err(
            "Koordinaten müssen endlich und in Metern angegeben sein (maximal ±10.000.000)",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn polygon(p: &[APunkt]) -> CoreResult<Polygon<f64>> {
    if !(3..=2000).contains(&p.len()) {
        return Err(err("Flächen benötigen 3 bis 2000 Eckpunkte"));
    }
    for v in p {
        punkt(v)?;
    }
    let mut line = LineString::from(p.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>());
    line.close();
    let poly = Polygon::new(line, vec![]);
    poly.check_validation()
        .map_err(|e| err(format!("Ungültiger Flächenumriss: {e}")))?;
    if poly.unsigned_area() < 1e-8 {
        return Err(err("Flächenumriss benötigt eine positive Fläche"));
    }
    Ok(poly)
}
pub(super) fn grundstueck(g: &AbstandsGrundstueck) -> CoreResult<Polygon<f64>> {
    if g.innenringe.len() > 100 {
        return Err(err("Höchstens 100 Innenringe je Grundstück"));
    }
    let aussen = polygon(&g.grenze)?;
    let innen = g
        .innenringe
        .iter()
        .map(|r| polygon(r).map(|p| p.exterior().clone()))
        .collect::<CoreResult<Vec<_>>>()?;
    let p = Polygon::new(aussen.exterior().clone(), innen);
    p.check_validation()
        .map_err(|e| err(format!("Ungültige Grundstücksinnenringe: {e}")))?;
    Ok(p)
}
fn umrisse(m: &MultiPolygon<f64>) -> Vec<AbstandsPolygon> {
    m.0.iter()
        .map(|p| AbstandsPolygon {
            aussen: p
                .exterior()
                .0
                .iter()
                .map(|p| APunkt { x: p.x, y: p.y })
                .collect(),
            innen: p
                .interiors()
                .iter()
                .map(|l| l.0.iter().map(|p| APunkt { x: p.x, y: p.y }).collect())
                .collect(),
        })
        .collect()
}
/// Außennormale der Wand (Einheitsvektor).
fn normale(w: &AbstandsWand) -> Option<(f64, f64)> {
    let (dx, dy) = (w.bis.x - w.von.x, w.bis.y - w.von.y);
    let l = dx.hypot(dy);
    let n = match w.aussen {
        Aussenrichtung::Links => 1.,
        Aussenrichtung::Rechts => -1.,
    };
    (l >= 0.001).then(|| (-dy / l * n, dx / l * n))
}
/// `verlauf`: Tiefe je Station ab Wandanfang, aufsteigend; Sprünge mit
/// doppelter Station.
pub(super) fn wand(
    p: &AbstandsPlan,
    w: &AbstandsWand,
    e: &mut WandErgebnis,
    verlauf: &[(f64, f64)],
    beruehrung_eingehalten: bool,
) -> CoreResult<()> {
    if verlauf.iter().all(|v| v.1 == 0.) && w.polygon_manuell.is_none() {
        e.flaeche_m2 = Some(0.);
        e.ausserhalb_m2 = Some(0.);
        if let Some(g) = p
            .grundstuecke
            .iter()
            .find(|g| Some(g.id) == w.grundstueck_id && g.grenze.len() >= 3)
        {
            let grenze = grundstueck(g)?;
            let linie = LineString::from(vec![(w.von.x, w.von.y), (w.bis.x, w.bis.y)]);
            e.status = if grenze.relate(&linie).is_covers() {
                AbstandsStatus::Eingehalten
            } else {
                AbstandsStatus::Konflikt
            };
            if g.grundlage.trim().is_empty() {
                e.offen
                    .push("Grenzherkunft und rechtliche Grundstückseinheit dokumentieren".into());
            }
        } else {
            e.status = AbstandsStatus::Tiefenvorpruefung;
            e.offen
                .push("Grundstücksgrenze fehlt – Lage der privilegierten Anlage ungeklärt".into());
        }
        return Ok(());
    }
    let dx = w.bis.x - w.von.x;
    let dy = w.bis.y - w.von.y;
    let n = match w.aussen {
        Aussenrichtung::Links => 1.,
        Aussenrichtung::Rechts => -1.,
    };
    let points = if let Some(poly) = &w.polygon_manuell {
        if !w
            .sonderansatz
            .as_ref()
            .is_some_and(|s| s.wirksam && s.beleg.is_some() && !s.grundlage.trim().is_empty())
        {
            e.offen
                .push("Manuelles Flächenpolygon benötigt wirksamen belegten Sonderansatz".into());
            return Ok(());
        }
        e.herleitung
            .push("Abstandsflächenumriss aus dokumentiertem besonderen Nachweis übernommen".into());
        poly.clone()
    } else {
        // Wandenden exakt übernehmen, damit gleichmäßige Tiefen dasselbe
        // Rechteck ergeben wie vor Version 2.
        let fuss = |s: f64| {
            if s <= 0. {
                w.von.clone()
            } else if s >= e.laenge_m {
                w.bis.clone()
            } else {
                APunkt {
                    x: w.von.x + dx * s / e.laenge_m,
                    y: w.von.y + dy * s / e.laenge_m,
                }
            }
        };
        // Stellen ohne Tiefenänderung (z. B. Wandteilgrenze ohne Sprung) entfallen.
        let knicke = verlauf.iter().enumerate().filter(|(i, v)| {
            *i == 0 || *i == verlauf.len() - 1 || verlauf[i - 1].1 != v.1 || verlauf[i + 1].1 != v.1
        });
        let mut punkte = vec![w.von.clone(), w.bis.clone()];
        for (_, &(s, t)) in knicke.rev() {
            let f = fuss(s);
            let q = APunkt {
                x: f.x - dy / e.laenge_m * t * n,
                y: f.y + dx / e.laenge_m * t * n,
            };
            if punkte.last() != Some(&q) {
                punkte.push(q);
            }
        }
        if punkte.len() > 3 && punkte.first() == punkte.last() {
            punkte.pop();
        }
        punkte
    };
    let poly = polygon(&points)?;
    e.flaeche_m2 = Some(poly.unsigned_area());
    e.polygon = points;
    let Some(g) = p
        .grundstuecke
        .iter()
        .find(|g| Some(g.id) == w.grundstueck_id && g.grenze.len() >= 3)
    else {
        e.status = AbstandsStatus::Tiefenvorpruefung;
        e.offen
            .push("Grundstücksgrenze fehlt – ausschließlich Tiefenvorprüfung".into());
        return Ok(());
    };
    if g.grundlage.trim().is_empty() {
        e.offen
            .push("Grenzherkunft und rechtliche Grundstückseinheit dokumentieren".into());
    }
    let mut erlaubt = MultiPolygon(vec![grundstueck(g)?]);
    for f in p
        .flaechen
        .iter()
        .filter(|f| f.grundstueck_id == g.id && f.art != Flaechenart::Baukoerper)
    {
        if f.wirksam && f.beleg.is_some() && !f.grundlage.trim().is_empty() && f.polygon.len() >= 3
        {
            erlaubt = erlaubt.union(&MultiPolygon(vec![polygon(&f.polygon)?]));
            e.herleitung.push(format!(
                "Berücksichtigte Sicherungs-/öffentliche Teilfläche: {}; {}",
                f.bezeichnung, f.grundlage
            ));
        } else {
            e.offen.push(format!(
                "Fläche {} ungeklärt/nicht wirksam: nicht angerechnet",
                f.bezeichnung
            ));
        }
    }
    let ausserhalb = MultiPolygon(vec![poly.clone()]).difference(&erlaubt);
    let mut konflikte = ausserhalb.clone();
    for f in p
        .flaechen
        .iter()
        .filter(|f| f.art == Flaechenart::Baukoerper && f.polygon.len() >= 3)
    {
        konflikte = konflikte.union(
            &MultiPolygon(vec![poly.clone()])
                .intersection(&MultiPolygon(vec![polygon(&f.polygon)?])),
        );
    }
    let flaeche = konflikte.unsigned_area();
    e.ausserhalb_m2 = Some(ausserhalb.unsigned_area());
    e.konflikte = umrisse(&konflikte);
    let randabstand = erlaubt
        .0
        .iter()
        .map(|a| Euclidean.distance(poly.exterior(), a.exterior()))
        .fold(f64::INFINITY, f64::min);
    // Ab Rechenstand 3: Berührung der Grenze von innen ist der übliche
    // Mindestabstand und eingehalten; Überschreitungen bis 1 mm² gelten als
    // Rundung. Jede größere Fläche jenseits der Grenze bleibt ein Konflikt.
    e.status = if beruehrung_eingehalten {
        if flaeche > 1e-6 {
            AbstandsStatus::Konflikt
        } else {
            if flaeche > 0. || randabstand <= 0.000001 {
                e.herleitung.push("Abstandsfläche reicht bis an die maßgebliche Grenze (Berührung von innen, Rechentoleranz 1 mm²).".into());
            }
            AbstandsStatus::Eingehalten
        }
    } else if flaeche > 1e-8 {
        AbstandsStatus::Konflikt
    } else if flaeche > 0. || randabstand <= 0.000001 {
        AbstandsStatus::Grenzfall
    } else {
        AbstandsStatus::Eingehalten
    };
    if e.status == AbstandsStatus::Grenzfall {
        e.herleitung.push("Numerischer Grenzfall: Berührung oder Abstand ≤ 0,000001 m zur maßgeblichen Grenze. Keine automatische Einhaltung; Eingaben und Genauigkeit prüfen.".into());
    }
    Ok(())
}
pub(super) fn ueberdeckungen(
    p: &AbstandsPlan,
    waende: &mut [WandErgebnis],
) -> CoreResult<Vec<Ueberdeckung>> {
    let normale = |id: Uuid| p.waende.iter().find(|w| w.id == id).and_then(normale);
    let mut result = vec![];
    for a in 0..waende.len() {
        for b in a + 1..waende.len() {
            if waende[a].polygon.len() < 3 || waende[b].polygon.len() < 3 {
                continue;
            }
            let m = polygon(&waende[a].polygon)?.intersection(&polygon(&waende[b].polygon)?);
            let flaeche = m.unsigned_area();
            if flaeche > 1e-8 {
                // 0° = parallel gegenüber, 90° = Innenecke, 180° = gleiche Richtung.
                let winkel = normale(waende[a].id)
                    .zip(normale(waende[b].id))
                    .map(|(na, nb)| {
                        180. - (na.0 * nb.0 + na.1 * nb.1)
                            .clamp(-1., 1.)
                            .acos()
                            .to_degrees()
                    });
                let zulaessig = p.regeln.ueberdeckung.as_ref().and_then(|r| {
                    winkel.filter(|w| *w > r.winkel_ueber_grad).map(|w| {
                        format!(
                            "Wände stehen im Winkel von {:.1}° > {}° zueinander; {}",
                            w, r.winkel_ueber_grad, r.quelle
                        )
                    })
                });
                result.push(Ueberdeckung {
                    wand_a: waende[a].id,
                    wand_b: waende[b].id,
                    flaeche_m2: flaeche,
                    polygone: umrisse(&m),
                    winkel_grad: winkel,
                    zulaessig,
                });
            }
        }
    }
    Ok(result)
}
