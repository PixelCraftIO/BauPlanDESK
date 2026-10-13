//! Testdaten für CAD-Tests (auch für andere Crates, Feature `testdaten`).

use ::dxf::entities::{Circle, Entity, EntityType, Insert, LwPolyline, MText};
use ::dxf::enums::{AcadVersion, Units};
use ::dxf::tables::Layer;
use ::dxf::{Block, Color, Drawing, LwPolylineVertex, Point};

/// Kleiner Lageplan in Metern:
/// * Grundstück 20 × 30 m (Ebene „Grundstück“, rot),
/// * Gebäude 12 × 10 m mit halbrundem Erker (Radius 2 m) und Firstlinie,
/// * zwei Bäume als Blockreferenz (Radius 1 m und 2 m),
/// * Beschriftung des Flurstücks als MTEXT.
pub fn musterplan() -> Vec<u8> {
    let mut d = Drawing::new();
    d.header.version = AcadVersion::R2018;
    d.header.default_drawing_units = Units::Meters;

    for (name, farbe) in [
        ("Grundstück", 1),
        ("Gebäude", 7),
        ("Grün", 3),
        ("Beschriftung", 7),
    ] {
        d.add_layer(Layer {
            name: name.into(),
            color: Color::from_index(farbe),
            ..Default::default()
        });
    }

    let auf = |ebene: &str, specific: EntityType, d: &mut Drawing| {
        let mut e = Entity::new(specific);
        e.common.layer = ebene.into();
        d.add_entity(e);
    };

    auf(
        "Grundstück",
        polylinie(
            &[
                (0.0, 0.0, 0.0),
                (20.0, 0.0, 0.0),
                (20.0, 30.0, 0.0),
                (0.0, 30.0, 0.0),
            ],
            true,
        ),
        &mut d,
    );
    auf(
        "Gebäude",
        polylinie(
            &[
                (4.0, 8.0, 0.0),
                (8.0, 8.0, 1.0), // Halbkreis nach Süden bis (12, 8)
                (12.0, 8.0, 0.0),
                (16.0, 8.0, 0.0),
                (16.0, 18.0, 0.0),
                (4.0, 18.0, 0.0),
            ],
            true,
        ),
        &mut d,
    );
    auf(
        "Gebäude",
        polylinie(&[(4.0, 13.0, 0.0), (16.0, 13.0, 0.0)], false),
        &mut d,
    );

    let mut kreis = Entity::new(EntityType::Circle(Circle {
        center: Point::new(0.0, 0.0, 0.0),
        radius: 1.0,
        ..Default::default()
    }));
    kreis.common.layer = "0".into();
    kreis.common.color = Color::by_block();
    d.add_block(Block {
        name: "Baum".into(),
        entities: vec![kreis],
        ..Default::default()
    });
    for (x, y, faktor) in [(3.0, 3.0, 1.0), (17.0, 25.0, 2.0)] {
        let mut e = Entity::new(EntityType::Insert(Insert {
            name: "Baum".into(),
            location: Point::new(x, y, 0.0),
            x_scale_factor: faktor,
            y_scale_factor: faktor,
            ..Default::default()
        }));
        e.common.layer = "Grün".into();
        e.common.color = Color::from_index(3);
        d.add_entity(e);
    }

    auf(
        "Beschriftung",
        EntityType::MText(MText {
            insertion_point: Point::new(2.0, 27.0, 0.0),
            initial_text_height: 0.8,
            text: r"Flurstück 123/4\PGemarkung Lindenweg".into(),
            ..Default::default()
        }),
        &mut d,
    );

    let mut aus = Vec::new();
    d.save(&mut aus).expect("DXF schreiben");
    aus
}

fn polylinie(ecken: &[(f64, f64, f64)], geschlossen: bool) -> EntityType {
    let mut l = LwPolyline::default();
    l.set_is_closed(geschlossen);
    l.vertices = ecken
        .iter()
        .map(|&(x, y, bulge)| LwPolylineVertex {
            x,
            y,
            bulge,
            ..Default::default()
        })
        .collect();
    EntityType::LwPolyline(l)
}
