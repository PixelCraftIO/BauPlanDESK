//! Geometrischer Vergleich eines Gebäudeumrisses mit einem Baufenster.
use super::{Laengeneinheit, Punkt};
use crate::domain::DocumentId;
use crate::{CoreError, CoreResult};
use geo::{Area, BooleanOps, Distance, Euclidean, LineString, Point, Polygon, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaugrenzenGeometrie {
    pub dokument_id: DocumentId,
    pub dokument_version: i64,
    pub einheit: Laengeneinheit,
    pub baufenster: Vec<Punkt>,
    pub vorhaben: Vec<Punkt>,
    /// Baugrundstück, zu dem der Vorhabenumriss gehört (Schlüssel der
    /// Grundstückseinheit). Ohne Angabe gilt der Vergleich nur bei genau einem
    /// Baugrundstück; bei mehreren wird er keinem still zugerechnet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grundstueck_schluessel: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Aussenflaeche {
    pub aussenring: Vec<Punkt>,
    pub innenringe: Vec<Vec<Punkt>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaugrenzenPruefung {
    pub baufenster_m2: f64,
    pub vorhaben_m2: f64,
    pub ausserhalb_m2: f64,
    pub ausserhalb_prozent: f64,
    /// Größter Abstand unter den Gebäudeeckpunkten. Kein Maximum an allen Kanten.
    pub eckpunkt_ueberstand_m: f64,
    pub eingehalten: bool,
    /// Ergebnisgeometrie in den ursprünglichen Zeichnungseinheiten, einschließlich Löchern.
    pub aussenflaechen: Vec<Aussenflaeche>,
}

fn polygon(punkte: &[Punkt], origin: Punkt, faktor: f64, name: &str) -> CoreResult<Polygon<f64>> {
    if !(3..=2000).contains(&punkte.len())
        || punkte.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err(CoreError::InvalidInput(format!(
            "{name}: 3 bis 2000 gültige Punkte erforderlich"
        )));
    }
    let ring = LineString::from(
        punkte
            .iter()
            .map(|p| ((p.0 - origin.0) * faktor, (p.1 - origin.1) * faktor))
            .collect::<Vec<_>>(),
    );
    if ring.0.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err(CoreError::InvalidInput(format!(
            "{name}: Koordinaten außerhalb des darstellbaren Bereichs"
        )));
    }
    let p = Polygon::new(ring, vec![]);
    p.check_validation()
        .map_err(|e| CoreError::InvalidInput(format!("{name}: ungültiger Umriss ({e})")))?;
    if !p.unsigned_area().is_finite() || p.unsigned_area() <= 1e-10 {
        return Err(CoreError::InvalidInput(format!(
            "{name}: Umriss benötigt eine Fläche größer als 0"
        )));
    }
    Ok(p)
}

impl BaugrenzenGeometrie {
    pub fn validate(&self) -> CoreResult<()> {
        if self.dokument_version < 1 {
            return Err(CoreError::InvalidInput("Ungültige Planversion".into()));
        }
        let o = self.baufenster.first().copied().unwrap_or(Punkt(0.0, 0.0));
        polygon(&self.baufenster, o, self.einheit.meter(), "Baufenster")?;
        polygon(&self.vorhaben, o, self.einheit.meter(), "Vorhaben")?;
        Ok(())
    }
}

pub fn pruefe(g: &BaugrenzenGeometrie) -> CoreResult<BaugrenzenPruefung> {
    g.validate()?;
    let o = g.baufenster[0];
    let faktor = g.einheit.meter();
    let b = polygon(&g.baufenster, o, faktor, "Baufenster")?;
    let v = polygon(&g.vorhaben, o, faktor, "Vorhaben")?;
    let aussen = v.difference(&b);
    let area = aussen.unsigned_area();
    let vorhaben_m2 = v.unsigned_area();
    let eckpunkt_ueberstand_m = v
        .exterior()
        .points()
        .map(|p| Euclidean.distance(&p, &b))
        .fold(0.0_f64, f64::max);
    let zurueck = |ring: &LineString<f64>| {
        ring.points()
            .map(|p: Point<f64>| Punkt(p.x() / faktor + o.0, p.y() / faktor + o.1))
            .collect()
    };
    Ok(BaugrenzenPruefung {
        baufenster_m2: b.unsigned_area(),
        vorhaben_m2,
        ausserhalb_m2: area,
        ausserhalb_prozent: area / vorhaben_m2 * 100.0,
        eckpunkt_ueberstand_m,
        eingehalten: area <= 1e-8,
        aussenflaechen: aussen
            .0
            .iter()
            .map(|p| Aussenflaeche {
                aussenring: zurueck(p.exterior()),
                innenringe: p.interiors().iter().map(zurueck).collect(),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rechteck(x: f64, y: f64, w: f64, h: f64) -> Vec<Punkt> {
        vec![
            Punkt(x, y),
            Punkt(x + w, y),
            Punkt(x + w, y + h),
            Punkt(x, y + h),
        ]
    }
    fn geo(b: Vec<Punkt>, v: Vec<Punkt>) -> BaugrenzenGeometrie {
        BaugrenzenGeometrie {
            dokument_id: DocumentId::new(),
            dokument_version: 1,
            einheit: Laengeneinheit::Meter,
            baufenster: b,
            vorhaben: v,
            grundstueck_schluessel: None,
        }
    }
    #[test]
    fn rechteck_ueberstand_flaeche_und_eckpunktabstand() {
        let r = pruefe(&geo(rechteck(0., 0., 10., 10.), rechteck(8., 2., 4., 4.))).unwrap();
        assert!((r.ausserhalb_m2 - 8.).abs() < 1e-8);
        assert!((r.ausserhalb_prozent - 50.).abs() < 1e-8);
        assert!((r.eckpunkt_ueberstand_m - 2.).abs() < 1e-8);
        assert!(!r.eingehalten);
    }
    #[test]
    fn randberuehrung_ist_keine_flaechenueberschreitung() {
        let r = pruefe(&geo(rechteck(0., 0., 10., 10.), rechteck(0., 0., 10., 10.))).unwrap();
        assert!(r.eingehalten);
        assert_eq!(r.ausserhalb_m2, 0.);
    }
    #[test]
    fn millimeter_und_landeskoodinaten() {
        let mut g = geo(
            rechteck(500000., 5400000., 10000., 10000.),
            rechteck(508000., 5402000., 4000., 4000.),
        );
        g.einheit = Laengeneinheit::Millimeter;
        assert!((pruefe(&g).unwrap().ausserhalb_m2 - 8.).abs() < 1e-8);
    }
    #[test]
    fn konkaves_baufenster_prueft_auch_kanten_trotz_innerer_eckpunkte() {
        let b = vec![
            Punkt(0., 0.),
            Punkt(10., 0.),
            Punkt(10., 10.),
            Punkt(6., 10.),
            Punkt(6., 4.),
            Punkt(4., 4.),
            Punkt(4., 10.),
            Punkt(0., 10.),
        ];
        let r = pruefe(&geo(b, rechteck(2., 5., 6., 2.))).unwrap();
        assert!((r.ausserhalb_m2 - 4.).abs() < 1e-8);
        assert_eq!(r.eckpunkt_ueberstand_m, 0.);
        assert!(!r.eingehalten);
    }
    #[test]
    fn loch_im_differenzpolygon_bleibt_erhalten() {
        let r = pruefe(&geo(rechteck(2., 2., 6., 6.), rechteck(0., 0., 10., 10.))).unwrap();
        assert!((r.ausserhalb_m2 - 64.).abs() < 1e-8);
        assert_eq!(r.aussenflaechen[0].innenringe.len(), 1);
    }
    #[test]
    fn selbstschneidende_und_degenerierte_umrisse_werden_abgelehnt() {
        let b = rechteck(0., 0., 10., 10.);
        assert!(
            pruefe(&geo(
                b.clone(),
                vec![Punkt(0., 0.), Punkt(2., 2.), Punkt(0., 2.), Punkt(2., 0.)]
            ))
            .is_err()
        );
        assert!(pruefe(&geo(b, vec![Punkt(0., 0.), Punkt(1., 0.), Punkt(2., 0.)])).is_err());
    }
}
