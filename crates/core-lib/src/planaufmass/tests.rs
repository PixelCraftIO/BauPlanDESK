use super::*;
fn plan(g: Plangeometrie) -> Planaufmass {
    let ebene = Uuid::from_u128(1);
    Planaufmass {
        bezeichnung: "EG".into(),
        dokument: Dokumentbezug {
            dokument_id: crate::domain::DocumentId(Uuid::from_u128(2)),
            version: 1,
        },
        seite: 1,
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Räume".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen: vec![Planmarkierung {
            id: Uuid::from_u128(3),
            ebene_id: ebene,
            bezeichnung: "Raum 01".into(),
            bedeutung: Planbedeutung::Raum,
            geometrie: g,
            grundstueck_id: None,
            gebaeude_id: None,
            geschoss_id: None,
        }],
    }
}
#[test]
fn kalibrierung_flaeche_und_umkehrung() {
    let mut p = plan(Plangeometrie::Polygon {
        punkte: vec![[0., 0.], [100., 0.], [100., 50.], [0., 50.]],
    });
    let r = p.messen().unwrap().remove(0);
    assert!((r.flaeche_m2.unwrap() - 50.).abs() < 1e-9);
    assert!((r.laenge_m.unwrap() - 30.).abs() < 1e-9);
    if let Plangeometrie::Polygon { punkte } = &mut p.markierungen[0].geometrie {
        punkte.reverse();
    }
    assert_eq!(r.flaeche_m2, p.messen().unwrap()[0].flaeche_m2);
    p.kalibrierung = None;
    assert_eq!(p.messen().unwrap()[0].flaeche_m2, None);
}
#[test]
fn massstab_physische_punkte_und_kreis() {
    let mut p = plan(Plangeometrie::Kreis {
        mitte: [0., 0.],
        rand: [72., 0.],
    });
    p.kalibrierung = Some(Plankalibrierung::Massstab { nenner: 100. });
    let r = p.messen().unwrap().remove(0);
    assert!((r.flaeche_m2.unwrap() - std::f64::consts::PI * 2.54 * 2.54).abs() < 1e-10);
}
#[test]
fn hoehen_y_achse_nach_unten_und_neigung() {
    let mut p = plan(Plangeometrie::Hoehenpunkt { punkt: [100., 50.] });
    assert_eq!(p.messen().unwrap()[0].hoehe_m, None);
    p.hoehenbezug = Some(Planhoehenbezug {
        punkt: [0., 100.],
        hoehe_m: 100.,
        system: "NHN".into(),
    });
    assert_eq!(p.messen().unwrap()[0].hoehe_m, Some(105.));
    p.markierungen[0].geometrie = Plangeometrie::Strecke {
        von: [0., 100.],
        bis: [100., 0.],
    };
    let r = p.messen().unwrap().remove(0);
    assert_eq!(r.neigung_grad, Some(45.));
    assert_eq!(r.hoehe_m, Some(10.));
    assert_eq!(r.horizontale_m, Some(10.));
}
#[test]
fn degeneriert_selbstschneidend_und_nan_werden_abgewiesen() {
    for g in [
        Plangeometrie::Polygon {
            punkte: vec![[0., 0.], [10., 10.], [0., 10.], [10., 0.]],
        },
        Plangeometrie::Rechteck {
            von: [0., 0.],
            bis: [0., 10.],
        },
        Plangeometrie::Kreis {
            mitte: [0., 0.],
            rand: [f64::NAN, 0.],
        },
    ] {
        assert!(plan(g).messen().is_err());
    }
    let mut p = plan(Plangeometrie::Strecke {
        von: [0., 0.],
        bis: [10., 0.],
    });
    p.kalibrierung = Some(Plankalibrierung::Strecke {
        von: [0., 0.],
        bis: [0., 0.],
        meter: 10.,
    });
    assert!(p.messen().is_err());
}
#[test]
fn altprojekte_ohne_bindung_bleiben_unveraendert() {
    for v in crate::berechnungen::vorlagen().unwrap() {
        assert!(v.plan.planbindungen.is_empty());
        assert!(
            serde_json::to_value(&v.plan)
                .unwrap()
                .get("planbindungen")
                .is_none()
        );
    }
    let p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    assert_eq!(
        p,
        serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap()
    );
}
#[test]
fn rohflaechen_sind_keine_wohnflaechen_oder_regelwerte() {
    use crate::berechnungen::BerechnungsArt::*;
    assert_eq!(
        service::zielgroesse(Din277, "positionen", "aufmass_m2"),
        Some(Planmessgroesse::Flaeche)
    );
    assert_eq!(
        service::zielgroesse(Stellplaetze, "positionen", "wohnflaeche_m2"),
        None
    );
    assert_eq!(
        service::zielgroesse(Freiflaechen, "regeln", "min_gruenanteil"),
        None
    );
}
#[test]
fn unbenannte_geschosse_bleiben_als_zielfeld_erkennbar() {
    let g: crate::plattform::objekte::Gebaeude = serde_json::from_value(serde_json::json!({
        "bezeichnung": "Lagerhalle", "grundstueck_ids": [], "quelle": null,
        "geschosse": [
            {"id": Uuid::from_u128(7), "bezeichnung": " ", "vollgeschoss": null, "grundlage": ""},
            {"id": Uuid::from_u128(8), "bezeichnung": "OG", "vollgeschoss": null, "grundlage": ""}
        ]
    }))
    .unwrap();
    let z = gebaeudeziele(&g);
    let titel = |ziel: String| z.iter().find(|x| x.ziel == ziel).unwrap().titel.clone();
    assert_eq!(
        titel(format!("geschosse.{}.flaeche_m2", Uuid::from_u128(7))),
        "Geschoss 1 · Fläche nach Außenmaßen"
    );
    assert_eq!(
        titel(format!("geschosse.{}.flaeche_m2", Uuid::from_u128(8))),
        "OG · Fläche nach Außenmaßen"
    );
}
