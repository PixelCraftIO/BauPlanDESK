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
            ausschnitt_id: None,
            hoehenzone_von: None,
        }],
        ausschnitte: vec![],
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
#[test]
fn bindungsstand_ignoriert_ebenen_und_andere_markierungen() {
    let alt = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    let id = Uuid::from_u128(3);
    let mut neu = alt.clone();
    neu.ebenen[0].sichtbar = false;
    neu.ebenen[0].gesperrt = true;
    let mut weitere = neu.markierungen[0].clone();
    weitere.id = Uuid::from_u128(4);
    weitere.geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [20., 20.],
    };
    neu.markierungen.push(weitere);
    assert_eq!(bindungsstand(&alt, id), bindungsstand(&neu, id));
    // Kontur, Maßstab und Zuordnung der gebundenen Markierung sind maßgebend.
    let mut kontur = neu.clone();
    kontur.markierungen[0].geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 12.],
    };
    assert_ne!(bindungsstand(&alt, id), bindungsstand(&kontur, id));
    let mut massstab = neu.clone();
    massstab.kalibrierung = Some(Plankalibrierung::Massstab { nenner: 50. });
    assert_ne!(bindungsstand(&alt, id), bindungsstand(&massstab, id));
    let mut zuordnung = neu;
    zuordnung.markierungen[0].bedeutung = Planbedeutung::Nuf;
    assert_ne!(bindungsstand(&alt, id), bindungsstand(&zuordnung, id));
}
fn schnitt(
    id: u128,
    von: Planpunkt,
    bis: Planpunkt,
    linie: Option<(Planpunkt, Planpunkt, f64)>,
) -> Planausschnitt {
    Planausschnitt {
        id: Uuid::from_u128(id),
        bezeichnung: format!("Schnitt {id}"),
        art: Ausschnittart::Schnitt,
        von,
        bis,
        kalibrierung: None,
        bezugslinie: linie.map(|(von, bis, hoehe_m)| Planbezugslinie {
            von,
            bis,
            hoehe_m,
            system: "m ü. NHN".into(),
        }),
    }
}
fn hoehenpunkt(id: u128, p: Planpunkt, ausschnitt: Option<u128>) -> Planmarkierung {
    Planmarkierung {
        id: Uuid::from_u128(id),
        ebene_id: Uuid::from_u128(1),
        bezeichnung: format!("Punkt {id}"),
        bedeutung: Planbedeutung::Traufe,
        geometrie: Plangeometrie::Hoehenpunkt { punkt: p },
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: ausschnitt.map(Uuid::from_u128),
        hoehenzone_von: None,
    }
}
#[test]
fn ausschnitte_messen_hoehen_je_eigener_bezugslinie() {
    // 10 m je 100 Punkte; zwei Schnitte nebeneinander mit verschiedenen ±0,00-Linien.
    let mut p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    p.markierungen.clear();
    p.ausschnitte = vec![
        schnitt(
            10,
            [0., 0.],
            [200., 200.],
            Some(([0., 150.], [200., 150.], 72.64)),
        ),
        schnitt(
            11,
            [300., 0.],
            [500., 200.],
            Some(([300., 100.], [500., 100.], 0.)),
        ),
    ];
    // Je 50 Punkte über der jeweiligen Linie = 5 m.
    p.markierungen.push(hoehenpunkt(20, [100., 100.], Some(10)));
    p.markierungen.push(hoehenpunkt(21, [400., 50.], Some(11)));
    // Dieselbe Seitenhöhe wie Punkt 21, aber im ersten Schnitt: 10 m über dessen Linie.
    p.markierungen.push(hoehenpunkt(22, [50., 50.], Some(10)));
    let w = p.messen().unwrap();
    assert!((w[0].hoehe_m.unwrap() - 77.64).abs() < 1e-9);
    assert!((w[1].hoehe_m.unwrap() - 5.).abs() < 1e-9);
    assert!((w[2].hoehe_m.unwrap() - 82.64).abs() < 1e-9);
    assert!(w.iter().all(|w| w.offen.is_empty()));
}
#[test]
fn ausschnitt_mit_eigenem_massstab_und_gedrehter_bezugslinie() {
    let mut p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    p.markierungen.clear();
    // Seitenmaßstab 10 m / 100 pt; Ausschnitt 1:50 → 50·0,0254/72 m je Punkt.
    let mut a = schnitt(
        10,
        [0., 0.],
        [200., 200.],
        Some(([100., 0.], [100., 200.], 0.)),
    );
    a.kalibrierung = Some(Plankalibrierung::Massstab { nenner: 50. });
    p.ausschnitte = vec![a];
    // Senkrechte Bezugslinie: „oben“ zeigt nach links (negatives x).
    p.markierungen.push(hoehenpunkt(20, [20., 100.], Some(10)));
    p.markierungen.push(Planmarkierung {
        geometrie: Plangeometrie::Strecke {
            von: [100., 50.],
            bis: [60., 50.],
        },
        ..hoehenpunkt(21, [0., 0.], Some(10))
    });
    let w = p.messen().unwrap();
    let f = 50. * 0.0254 / 72.;
    assert!((w[0].hoehe_m.unwrap() - 80. * f).abs() < 1e-9);
    // Strecke entlang „oben“: reine Höhendifferenz, keine Horizontale.
    assert!((w[1].hoehe_m.unwrap() - 40. * f).abs() < 1e-9);
    assert!(w[1].horizontale_m.unwrap().abs() < 1e-9);
    assert!((w[1].neigung_grad.unwrap() - 90.).abs() < 1e-9);
}
#[test]
fn ausschnitte_melden_fehlende_bezuege_und_weisen_ungueltige_zurueck() {
    let mut p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    p.hoehenbezug = Some(Planhoehenbezug {
        punkt: [0., 0.],
        hoehe_m: 0.,
        system: "NHN".into(),
    });
    p.markierungen.clear();
    let mut grundriss = schnitt(10, [0., 0.], [100., 100.], None);
    grundriss.art = Ausschnittart::Grundriss;
    p.ausschnitte = vec![grundriss, schnitt(11, [200., 0.], [300., 100.], None)];
    p.markierungen.push(hoehenpunkt(20, [50., 50.], Some(10)));
    p.markierungen.push(hoehenpunkt(21, [250., 50.], Some(11)));
    p.markierungen.push(hoehenpunkt(22, [250., 150.], Some(11)));
    p.markierungen.push(hoehenpunkt(23, [250., 50.], None));
    let w = p.messen().unwrap();
    // Der Seitenbezug gilt nicht innerhalb eines Ausschnitts.
    assert!(w[0].hoehe_m.is_none() && w[0].offen.iter().any(|o| o.contains("Grundriss")));
    assert!(w[1].hoehe_m.is_none() && w[1].offen.iter().any(|o| o.contains("Bezugslinie")));
    assert!(w[2].offen.iter().any(|o| o.contains("außerhalb")));
    assert!(w[3].hoehe_m.is_some() && w[3].offen.is_empty());
    let mut fremd = p.clone();
    fremd.markierungen[0].ausschnitt_id = Some(Uuid::from_u128(99));
    assert!(fremd.messen().is_err());
    let mut falsch = p.clone();
    falsch.ausschnitte[0].bezugslinie = Some(Planbezugslinie {
        von: [0., 0.],
        bis: [1., 0.],
        hoehe_m: 0.,
        system: "NHN".into(),
    });
    assert!(falsch.messen().is_err(), "Grundriss ohne Bezugslinie");
    let mut leer = p.clone();
    leer.ausschnitte[0].bis = leer.ausschnitte[0].von;
    assert!(leer.messen().is_err());
}
#[test]
fn ausschnitte_sind_optional_und_werden_ohne_inhalt_nicht_serialisiert() {
    let p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.get("ausschnitte").is_none());
    assert!(v["markierungen"][0].get("ausschnitt_id").is_none());
    let zurueck: Planaufmass = serde_json::from_value(v).unwrap();
    assert_eq!(zurueck, p);
    let mut mit = p.clone();
    mit.ausschnitte = vec![schnitt(
        10,
        [0., 0.],
        [50., 50.],
        Some(([0., 40.], [50., 40.], 1.)),
    )];
    mit.markierungen[0].ausschnitt_id = Some(Uuid::from_u128(10));
    let v = serde_json::to_value(&mit).unwrap();
    assert_eq!(v["ausschnitte"][0]["art"], "schnitt");
    assert_eq!(serde_json::from_value::<Planaufmass>(v).unwrap(), mit);
    // Der Bindungsstand kennt den Ausschnitt der gebundenen Markierung.
    let id = mit.markierungen[0].id;
    assert_ne!(bindungsstand(&p, id), bindungsstand(&mit, id));
    let mut andere = mit.clone();
    andere
        .ausschnitte
        .push(schnitt(12, [60., 0.], [90., 50.], None));
    assert_eq!(bindungsstand(&mit, id), bindungsstand(&andere, id));
}
#[test]
fn hoehensystem_folgt_der_bezugslinie_des_ausschnitts() {
    let mut p = plan(Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [10., 10.],
    });
    p.markierungen.clear();
    p.hoehenbezug = Some(Planhoehenbezug {
        punkt: [0., 0.],
        hoehe_m: 0.,
        system: "lokal, OKFF EG = 0".into(),
    });
    p.ausschnitte = vec![
        schnitt(
            10,
            [0., 0.],
            [200., 200.],
            Some(([0., 150.], [200., 150.], 72.64)),
        ),
        schnitt(11, [300., 0.], [500., 200.], None),
    ];
    p.markierungen.push(hoehenpunkt(20, [100., 100.], Some(10)));
    p.markierungen.push(hoehenpunkt(21, [400., 50.], Some(11)));
    p.markierungen.push(hoehenpunkt(22, [600., 50.], None));
    let m = |i: usize| p.markierungen[i].clone();
    // Im Schnitt gilt dessen Bezugslinie, nicht der blattweite Höhenbezug.
    assert_eq!(p.hoehensystem(&m(0)), Some("m ü. NHN"));
    // Ausschnitt ohne Bezugslinie liefert keine absoluten Höhen (wie messen).
    assert_eq!(p.hoehensystem(&m(1)), None);
    assert!(p.messen().unwrap()[1].hoehe_m.is_none());
    // Ohne Ausschnitt: blattweiter Höhenbezug.
    assert_eq!(p.hoehensystem(&m(2)), Some("lokal, OKFF EG = 0"));
    p.hoehenbezug = None;
    assert_eq!(p.hoehensystem(&m(2)), None);
    assert_eq!(p.hoehensystem(&m(0)), Some("m ü. NHN"));
}
