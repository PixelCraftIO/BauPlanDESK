//! Geometrische Bearbeitung in Seitenpunkten; keine rechtliche Anrechnung.
use super::*;
use geo::{BooleanOps, MultiPolygon};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Flaechenoperation {
    Vereinen,
    Abziehen,
    Schneiden,
    Teilen,
}

fn ring(ps: &[Planpunkt]) -> CoreResult<LineString<f64>> {
    polygon(ps)?;
    Ok(LineString::from(
        ps.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>(),
    ))
}
fn umwandeln(ps: &[Planpolygon]) -> CoreResult<MultiPolygon<f64>> {
    if ps.is_empty()
        || ps.len() > 100
        || ps
            .iter()
            .map(|p| p.aussen.len() + p.innen.iter().map(Vec::len).sum::<usize>())
            .sum::<usize>()
            > 10000
    {
        return Err(fehler(
            "Fläche benötigt 1 bis 100 Teilflächen und höchstens 10000 Punkte",
        ));
    }
    let mut result = Vec::new();
    for p in ps {
        if p.innen.len() > 100 {
            return Err(fehler("Zu viele Aussparungen"));
        }
        let p = Polygon::new(
            ring(&p.aussen)?,
            p.innen
                .iter()
                .map(|p| ring(p))
                .collect::<CoreResult<Vec<_>>>()?,
        );
        p.check_validation()
            .map_err(|e| fehler(format!("Ungültige Aussparung: {e}")))?;
        result.push(p);
    }
    let m = MultiPolygon(result);
    m.check_validation()
        .map_err(|e| fehler(format!("Ungültige Teilflächen: {e}")))?;
    Ok(m)
}
pub(super) fn flaechenmasse(ps: &[Planpolygon]) -> CoreResult<(f64, f64)> {
    let m = umwandeln(ps)?;
    let area = m.unsigned_area();
    positiv(area)?;
    let length =
        m.0.iter()
            .flat_map(|p| std::iter::once(p.exterior()).chain(p.interiors().iter()))
            .map(|l| {
                l.0.windows(2)
                    .map(|p| (p[0].x - p[1].x).hypot(p[0].y - p[1].y))
                    .sum::<f64>()
            })
            .sum();
    Ok((area, length))
}
fn als_flaeche(g: &Plangeometrie) -> CoreResult<MultiPolygon<f64>> {
    let punkte = match g {
        Plangeometrie::Flaeche { polygone } => return umwandeln(polygone),
        Plangeometrie::Polygon { punkte } => punkte.clone(),
        Plangeometrie::Rechteck { von, bis } => {
            vec![*von, [bis[0], von[1]], *bis, [von[0], bis[1]]]
        }
        Plangeometrie::Kreis { mitte, rand } => {
            punkt(mitte)?;
            punkt(rand)?;
            let r = abstand(mitte, rand);
            positiv(r)?;
            // Explizite Polygonisierung ausschließlich bei Flächenoperationen.
            (0..512)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 512.;
                    [mitte[0] + r * a.cos(), mitte[1] + r * a.sin()]
                })
                .collect()
        }
        _ => return Err(fehler("Flächenoperation benötigt flächige Markierungen")),
    };
    umwandeln(&[Planpolygon {
        aussen: punkte,
        innen: vec![],
    }])
}
fn zurueck(m: MultiPolygon<f64>) -> CoreResult<Plangeometrie> {
    if m.0.is_empty() {
        return Err(fehler("Die Operation ergibt keine Fläche"));
    }
    let punkte = |l: &LineString<f64>| {
        l.0.iter()
            .take(l.0.len().saturating_sub(1))
            .map(|c| [c.x, c.y])
            .collect()
    };
    let polygone: Vec<_> =
        m.0.iter()
            .map(|p| Planpolygon {
                aussen: punkte(p.exterior()),
                innen: p.interiors().iter().map(punkte).collect(),
            })
            .collect();
    flaechenmasse(&polygone)?;
    Ok(Plangeometrie::Flaeche { polygone })
}
/// Vereinen/Abziehen/Schnitt liefert eine Geometrie; Teilen liefert den
/// gemeinsamen und den verbleibenden Teil. Leere Teilungen werden abgewiesen.
pub fn flaechenoperation(
    a: &Plangeometrie,
    b: &Plangeometrie,
    op: Flaechenoperation,
) -> CoreResult<Vec<Plangeometrie>> {
    let a = als_flaeche(a)?;
    let b = als_flaeche(b)?;
    match op {
        Flaechenoperation::Vereinen => Ok(vec![zurueck(a.union(&b))?]),
        Flaechenoperation::Abziehen => Ok(vec![zurueck(a.difference(&b))?]),
        Flaechenoperation::Schneiden => Ok(vec![zurueck(a.intersection(&b))?]),
        Flaechenoperation::Teilen => Ok(vec![
            zurueck(a.intersection(&b))?,
            zurueck(a.difference(&b))?,
        ]),
    }
}
