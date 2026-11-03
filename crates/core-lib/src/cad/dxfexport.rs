//! DXF-Ausgabe einer Zeichnung oder eines kalibrierten Aufmaßes für das
//! Büro-CAD: Meter, y nach oben, eine DXF-Ebene je Zeichenebene mit Farbe und
//! Strichstärke. Linienarten bleiben „Continuous“ (keine Linientyp-Tabellen).
use crate::{
    CoreError, CoreResult,
    druckausgabe::planlayout::{Planlinie, Plantext},
};
use ::dxf::{
    Color, Drawing, LwPolylineVertex, Point,
    entities::{Entity, EntityType, LwPolyline, Text},
    enums::{AcadVersion, Units},
    tables::Layer,
};
use std::collections::BTreeSet;

fn rgb(c: [f64; 3]) -> i32 {
    let k = |v: f64| (v.clamp(0., 1.) * 255.).round() as i32;
    (k(c[0]) << 16) | (k(c[1]) << 8) | k(c[2])
}
/// DXF-Strichstärke in 1/100 mm auf die nächste Normstufe.
fn strichstaerke(mm: f64) -> i16 {
    const STUFEN: [i16; 14] = [13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 100];
    let h = (mm * 100.).round() as i16;
    *STUFEN.iter().min_by_key(|s| (**s - h).abs()).unwrap_or(&25)
}
/// DXF-Ebenennamen dürfen keine Steuer- und Sonderzeichen tragen.
fn layername(s: &str) -> String {
    let n: String = s
        .chars()
        .map(|c| {
            if c.is_control() || "<>/\\\":;?*|,=`".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let n = n.trim();
    if n.is_empty() {
        "0".into()
    } else {
        n.chars().take(200).collect()
    }
}

/// `faktor`: Meter je Blatteinheit; `massstab_text`: Maßstab, mit dem die Texthöhen (Papier-mm) gerechnet wurden.
pub fn schreiben(
    linien: &[Planlinie],
    texte: &[Plantext],
    faktor: f64,
    massstab_text: f64,
) -> CoreResult<Vec<u8>> {
    if !(faktor.is_finite() && faktor > 0.) {
        return Err(CoreError::InvalidInput(
            "Maßstab fehlt für die DXF-Ausgabe".into(),
        ));
    }
    let mut d = Drawing::new();
    d.header.version = AcadVersion::R2018;
    d.header.default_drawing_units = Units::Meters;
    // Farbe und Strichstärke stehen am Objekt (24-Bit-Farbe, Strichstärke).
    let ebenen: BTreeSet<String> = linien
        .iter()
        .map(|l| layername(&l.ebene))
        .chain(texte.iter().map(|t| layername(&t.ebene)))
        .collect();
    for name in ebenen.into_iter().filter(|n| n != "0") {
        d.add_layer(Layer {
            name,
            color: Color::from_index(7),
            ..Default::default()
        });
    }
    let p = |q: &[f64; 2]| (q[0] * faktor, -q[1] * faktor);
    for l in linien.iter().filter(|l| l.punkte.len() >= 2) {
        let mut pl = LwPolyline::default();
        pl.set_is_closed(l.geschlossen);
        pl.vertices = l
            .punkte
            .iter()
            .map(|q| {
                let (x, y) = p(q);
                LwPolylineVertex {
                    x,
                    y,
                    ..Default::default()
                }
            })
            .collect();
        let mut e = Entity::new(EntityType::LwPolyline(pl));
        e.common.layer = layername(&l.ebene);
        e.common.color_24_bit = rgb(l.stift.farbe);
        e.common.lineweight_enum_value = strichstaerke(l.stift.breite_mm);
        d.add_entity(e);
    }
    for t in texte.iter().filter(|t| !t.text.trim().is_empty()) {
        let (x, y) = p(&t.punkt);
        let mut e = Entity::new(EntityType::Text(Text {
            location: Point::new(x, y, 0.),
            // Papierhöhe (mm) im Ausgabemaßstab zurück in Meter.
            text_height: (t.hoehe_mm * massstab_text / 1000.).max(0.01),
            value: t.text.clone(),
            rotation: t.drehung_grad,
            ..Default::default()
        }));
        e.common.layer = layername(&t.ebene);
        e.common.color_24_bit = rgb(t.farbe);
        d.add_entity(e);
    }
    let mut out = Vec::new();
    d.save(&mut out)
        .map_err(|e| CoreError::Storage(format!("DXF schreiben: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::druckausgabe::planlayout::{Fuellung, Stift};
    use std::io::Cursor;

    #[test]
    fn dxf_ist_lesbar_und_in_metern() {
        let linien = vec![Planlinie {
            ebene: "Wände/Neubau".into(),
            punkte: vec![[0., 0.], [100., 0.], [100., 50.]],
            geschlossen: true,
            stift: Stift {
                breite_mm: 0.5,
                farbe: [0.8, 0., 0.],
                muster: vec![],
            },
            fuellung: Fuellung::Keine,
        }];
        let texte = vec![Plantext {
            ebene: "Text".into(),
            punkt: [10., 10.],
            text: "Wohnen".into(),
            hoehe_mm: 3.5,
            drehung_grad: 0.,
            farbe: [0., 0., 0.],
        }];
        let b = schreiben(&linien, &texte, 0.01, 100.).unwrap();
        let d = Drawing::load(&mut Cursor::new(&b)).unwrap();
        let es: Vec<_> = d.entities().collect();
        assert_eq!(es.len(), 2);
        let EntityType::LwPolyline(pl) = &es[0].specific else {
            panic!()
        };
        assert!(
            (pl.vertices[1].x - 1.).abs() < 1e-9,
            "100 Einheiten × 0,01 = 1 m"
        );
        assert!((pl.vertices[2].y + 0.5).abs() < 1e-9, "y nach oben");
        assert_eq!(es[0].common.layer, "Wände_Neubau");
        assert_eq!(es[0].common.lineweight_enum_value, 50);
        assert!(d.layers().any(|l| l.name == "Text"));
        assert!(schreiben(&linien, &texte, 0., 100.).is_err());
    }
}
