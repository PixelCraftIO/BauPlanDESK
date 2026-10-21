use super::{testdaten::*, *};
#[test]
fn tiefe_und_rechteckflaeche_unabhaengiger_kontrollfall() {
    let p = plan();
    let r = berechnen(&p).unwrap();
    assert!(r.konfliktfrei);
    let w = &r.waende[0];
    assert_eq!(w.h_m, Some(8.));
    assert_eq!(w.tiefe_m, Some(3.2));
    assert!((w.flaeche_m2.unwrap() - 32.).abs() < 1e-8);
    assert_eq!(w.ausserhalb_m2, Some(0.));
}
#[test]
fn dachschwellen_giebel_und_fehlender_ansatz() {
    let mut p = plan();
    p.waende[0].dachart = Dachart::Dachflaeche;
    p.waende[0].dachhoehe_m = Some(3.);
    for (n, anteil) in [(45., 0.), (45.01, 1.), (70., 1.), (70.01, 3.)] {
        p.waende[0].dachneigung_grad = Some(n);
        assert_eq!(berechnen(&p).unwrap().waende[0].dachanteil_m, Some(anteil));
    }
    p.regeln.dach.clear();
    assert!(!berechnen(&p).unwrap().vollstaendig);
    // Pakete ohne Giebelregel: fachlich hergeleiteter Anteil.
    p.regeln.giebel = None;
    p.waende[0].dachart = Dachart::Giebel;
    p.waende[0].dachfaktor_manuell = Some(0.5);
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.waende[0].dachgrundlage = "Geprüfter Giebelansatz für den Kontrollfall".into();
    assert_eq!(berechnen(&p).unwrap().waende[0].dachanteil_m, Some(1.5));
}
#[test]
fn fremde_einheit_darf_abweichung_nicht_decken() {
    let mut p = plan();
    p.grundstuecke[0].grenze = rechteck(0., 3., 20., 17.);
    let mut fremd = p.grundstuecke[0].clone();
    fremd.id = Uuid::from_u128(8);
    fremd.grenze = rechteck(0., 0., 20., 3.);
    p.grundstuecke.push(fremd);
    let r = berechnen(&p).unwrap();
    assert_eq!(r.waende[0].status, AbstandsStatus::Konflikt);
    assert!(
        (r.waende[0].ausserhalb_m2.unwrap() - 12.).abs() < 1e-6,
        "{:?}",
        r.waende[0]
    );
    assert!(!r.konfliktfrei);
}
#[test]
fn grundbuch_und_grz_sind_keine_voraussetzung() {
    let mut p = plan();
    p.grundstuecke.clear();
    p.waende[0].grundstueck_id = None;
    let r = berechnen(&p).unwrap();
    assert_eq!(r.waende[0].tiefe_m, Some(3.2));
    assert!(!r.vollstaendig);
    assert!(!r.konfliktfrei);
}
#[test]
fn sicherung_nur_mit_beleg_wirksam_und_richtiger_einheit() {
    let mut p = plan();
    p.grundstuecke[0].grenze = rechteck(0., 3., 20., 17.);
    let f = AbstandsFlaeche {
        id: Uuid::from_u128(5),
        grundstueck_id: p.grundstuecke[0].id,
        art: Flaechenart::Sicherung,
        bezeichnung: "Baulastfläche".into(),
        polygon: rechteck(0., 0., 20., 3.),
        grundlage: "Fiktive wirksame Sicherung dieses Umrisses".into(),
        wirksam: true,
        beleg: None,
    };
    p.flaechen.push(f);
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.flaechen[0].beleg = Some(Dokumentbezug {
        dokument_id: crate::domain::DocumentId(Uuid::from_u128(10)),
        version: 1,
    });
    assert!(berechnen(&p).unwrap().konfliktfrei);
    p.flaechen[0].wirksam = false;
    assert!(!berechnen(&p).unwrap().konfliktfrei);
}
#[test]
fn grenzgarage_zaehlt_nur_wandlaengen_an_der_grenze() {
    // Garage 6 × 3,5 m an der Grenze: eine Wand an der Grenze, drei weitere
    // Wände ohne eigene Abstandsfläche; Gesamtwandlänge 19 m > 9 m ist unerheblich.
    let mut p = plan();
    let w = &mut p.waende[0];
    w.fall = Wandfall::Garage;
    w.gebaeude = "Garage".into();
    w.abschluss_a_m = Some(102.5);
    w.abschluss_b_m = Some(102.5);
    w.bis.x = w.von.x + 6.;
    w.nutzung_geprueft = true;
    w.grenzkennung = "Sued".into();
    let grenze = w.clone();
    for (i, (dx, dy)) in [(6., 3.5), (-6., 0.), (0., -3.5)].into_iter().enumerate() {
        let mut a = grenze.clone();
        a.id = Uuid::from_u128(20 + i as u128);
        a.grenzkennung = String::new();
        a.von = p.waende.last().unwrap().bis.clone();
        a.bis = APunkt {
            x: a.von.x + if dx == 6. { 0. } else { dx },
            y: a.von.y + if dx == 6. { dy } else { dy },
        };
        p.waende.push(a);
    }
    let r = berechnen(&p).unwrap();
    assert!(
        r.waende.iter().all(|w| w.tiefe_m == Some(0.)),
        "{:?}",
        r.waende
            .iter()
            .map(|w| (&w.offen, w.tiefe_m))
            .collect::<Vec<_>>()
    );
    assert!(
        r.waende[1]
            .herleitung
            .iter()
            .any(|h| h.contains("ohne eigene Abstandsfläche"))
    );
    // Rechenstand 2: wie bisher Summe aller Wände, Privileg entfällt.
    let alt = berechnen_stand(&p, "abstand-projekt-2").unwrap();
    assert!(alt.waende.iter().any(|w| w.tiefe_m != Some(0.)));
}
#[test]
fn garage_summen_und_grundstueckslage() {
    let mut p = plan();
    let w = &mut p.waende[0];
    w.fall = Wandfall::Garage;
    w.abschluss_a_m = Some(102.5);
    w.abschluss_b_m = Some(102.5);
    w.bis.x = 11.;
    w.nutzung_geprueft = true;
    w.grenzkennung = "Sued".into();
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(0.));
    let mut andere = p.waende[0].clone();
    andere.id = Uuid::from_u128(6);
    andere.von.x = 11.;
    andere.bis.x = 17.;
    p.waende.push(andere);
    let r = berechnen(&p).unwrap();
    assert!(!r.vollstaendig);
    assert!(r.waende.iter().all(|w| w.tiefe_m == Some(3.)));
    p.waende.pop();
    p.waende[0].von.y = -1.;
    p.waende[0].bis.y = -1.;
    assert_eq!(
        berechnen(&p).unwrap().waende[0].status,
        AbstandsStatus::Konflikt
    );
}
#[test]
fn andere_baukoerper_und_ueberdeckungen() {
    let mut p = plan();
    let mut a = p.waende[0].clone();
    a.id = Uuid::from_u128(6);
    a.von.x = 8.;
    a.bis.x = 12.;
    p.waende.push(a);
    assert!(!berechnen(&p).unwrap().ueberdeckungen.is_empty());
    p.waende.pop();
    p.flaechen.push(AbstandsFlaeche {
        id: Uuid::from_u128(7),
        grundstueck_id: p.grundstuecke[0].id,
        art: Flaechenart::Baukoerper,
        bezeichnung: "Nachbarbaukörper".into(),
        polygon: rechteck(6., 2., 2., 2.),
        grundlage: "Plan".into(),
        wirksam: false,
        beleg: None,
    });
    let r = berechnen(&p).unwrap();
    assert_eq!(r.waende[0].status, AbstandsStatus::Konflikt);
    assert_eq!(r.waende[0].ausserhalb_m2, Some(0.));
    assert!(!r.waende[0].konflikte.is_empty());
}
#[test]
fn rotation_translation_aendert_flaechen_nicht() {
    let mut p = plan();
    p.grundstuecke[0].grenze = rechteck(0., 3., 20., 17.);
    let vorher = berechnen(&p).unwrap();
    let drehen = |v: &mut APunkt| {
        let (x, y) = (v.x, v.y);
        v.x = -y + 10000.;
        v.y = x + 50000.;
    };
    for g in &mut p.grundstuecke {
        for v in &mut g.grenze {
            drehen(v);
        }
    }
    for w in &mut p.waende {
        drehen(&mut w.von);
        drehen(&mut w.bis);
    }
    let nachher = berechnen(&p).unwrap();
    assert!(
        (vorher.waende[0].ausserhalb_m2.unwrap() - nachher.waende[0].ausserhalb_m2.unwrap()).abs()
            < 1e-6
    );
}
#[test]
fn unvollstaendig_nan_und_selbstschnitt() {
    let mut p = plan();
    p.recht.geprueft = false;
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.waende[0].gelaende_a_m = None;
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, None);
    p.waende[0].von.x = f64::NAN;
    assert!(berechnen(&p).is_err());
    p = plan();
    p.grundstuecke[0].grenze = vec![
        punkt(0., 0.),
        punkt(10., 10.),
        punkt(0., 10.),
        punkt(10., 0.),
    ];
    assert!(berechnen(&p).is_err());
}

#[test]
fn beruehrung_der_grenze_ist_eingehalten_ueberschreitung_nicht() {
    // Wand genau im Mindestabstand: Die Abstandsfläche reicht bis an die Grenze.
    let mut p = plan();
    p.waende[0].von.y = 3.2;
    p.waende[0].bis.y = 3.2;
    let r = berechnen(&p).unwrap();
    assert_eq!(r.waende[0].status, AbstandsStatus::Eingehalten);
    assert!(r.konfliktfrei);
    assert!(
        r.waende[0]
            .herleitung
            .iter()
            .any(|h| h.contains("bis an die maßgebliche Grenze"))
    );
    // Archivierte Stände nach Rechenstand 2 bleiben unverändert reproduzierbar.
    let alt = berechnen_stand(&p, "abstand-projekt-2").unwrap();
    assert_eq!(alt.waende[0].status, AbstandsStatus::Grenzfall);
    assert!(!alt.konfliktfrei);
    // Messbare Überschreitung (1 cm über die ganze Wand) bleibt ein Konflikt.
    p.waende[0].von.y = 3.19;
    p.waende[0].bis.y = 3.19;
    assert_eq!(
        berechnen(&p).unwrap().waende[0].status,
        AbstandsStatus::Konflikt
    );
}
#[test]
fn manueller_umriss_braucht_wirksamen_sonderbeleg() {
    let mut p = plan();
    p.waende[0].polygon_manuell = Some(rechteck(5., 2., 10., 3.));
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.waende[0].sonderansatz = Some(Sonderansatz {
        tiefe_m: 3.,
        grundlage: "Dokumentierter besonderer Flächenansatz und dessen Geltung".into(),
        wirksam: true,
        beleg: Some(Dokumentbezug {
            dokument_id: crate::domain::DocumentId(Uuid::from_u128(45)),
            version: 1,
        }),
    });
    assert!(berechnen(&p).unwrap().konfliktfrei);
}

#[test]
fn grundstuecksinnenring_bleibt_konfliktflaeche() {
    let mut p = plan();
    p.grundstuecke[0].innenringe.push(rechteck(6., 2., 2., 2.));
    let r = berechnen(&p).unwrap();
    assert!(!r.konfliktfrei);
    assert!((r.waende[0].ausserhalb_m2.unwrap() - 4.).abs() < 1e-6);
    assert!(!r.waende[0].konflikte.is_empty());
}

/// Der Schnellrechner (Rhai-Regelpakete) und der Abstandsnachweis
/// (JSON-Katalog) verwenden dieselben Tiefenregeln. Maßgebend ist der Katalog;
/// ein ausgeliefertes Regelpaket darf nicht davon abweichen.
#[test]
fn shipped_lbo_pakete_entsprechen_dem_katalog() {
    use crate::calc::Baugebiet::*;
    use crate::domain::Bundesland;
    use crate::lbo::{AbstandsflaecheInput, LboEngine, RulePackSource};
    let engine = LboEngine::new();
    for (land, script) in [
        (
            Bundesland::Bw,
            include_str!("../../../../lbo-rules/bw.rhai"),
        ),
        (
            Bundesland::By,
            include_str!("../../../../lbo-rules/by.rhai"),
        ),
        (
            Bundesland::Nw,
            include_str!("../../../../lbo-rules/nw.rhai"),
        ),
    ] {
        engine
            .load_rule_pack(RulePackSource {
                land,
                script: script.into(),
                origin: "test".into(),
            })
            .unwrap();
    }
    let katalog = katalog().unwrap();
    assert_eq!(katalog.len(), engine.rule_packs().len());
    for regeln in &katalog {
        for gebiet in [WS, WR, WA, WB, MD, MDW, MI, MU, MK, GE, GI, SO] {
            let regel = regeln
                .tiefen
                .iter()
                .find(|t| t.fall.is_none() && t.gebiete.iter().any(|g| g == gebiet.code()))
                .unwrap_or_else(|| panic!("{:?}: {} fehlt im Katalog", regeln.land, gebiet.code()));
            for h in [0.0, 2.0, 6.5, 10.0, 25.0] {
                let r = engine
                    .abstandsflaeche(
                        regeln.land,
                        &AbstandsflaecheInput {
                            wandhoehe_m: h,
                            gebiet,
                        },
                    )
                    .unwrap()
                    .ergebnis;
                assert_eq!(
                    (r.faktor, r.mindesttiefe_m),
                    (regel.faktor, regel.mindesttiefe_m),
                    "{:?} {}",
                    regeln.land,
                    gebiet.code()
                );
                let erwartet = (h * regel.faktor).max(regel.mindesttiefe_m);
                assert!(
                    (r.tiefe_m - erwartet).abs() < 0.01,
                    "{:?} {} H={h}",
                    regeln.land,
                    gebiet.code()
                );
            }
        }
    }
}
fn hp(station_m: f64, hoehe_m: f64) -> Hoehenpunkt {
    Hoehenpunkt {
        station_m,
        hoehe_m,
        quelle: String::new(),
    }
}
/// Fläche des Abstandsflächenpolygons (Gaußsche Formel).
fn flaeche(p: &[APunkt]) -> f64 {
    (0..p.len())
        .map(|i| {
            let (a, b) = (&p[i], &p[(i + 1) % p.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        .abs()
        / 2.
}
fn profilplan(gelaende: &[(f64, f64)], oberkante: &[(f64, f64)]) -> AbstandsPlan {
    let mut p = plan();
    let w = &mut p.waende[0];
    (
        w.gelaende_a_m,
        w.gelaende_b_m,
        w.abschluss_a_m,
        w.abschluss_b_m,
    ) = (None, None, None, None);
    w.gelaende = gelaende.iter().map(|&(s, h)| hp(s, h)).collect();
    w.oberkante = oberkante.iter().map(|&(s, h)| hp(s, h)).collect();
    p
}
#[test]
fn altplaene_rechnen_unveraendert_mit_maximalansatz() {
    let mut p = plan();
    p.regeln.wandhoehe = None;
    p.waende[0].gelaende_b_m = Some(101.);
    let r = berechnen(&p).unwrap();
    let w = &r.waende[0];
    assert_eq!(r.rechenstand, RECHENSTAND);
    assert_eq!(
        (w.wandhoehe_m, w.h_m, w.tiefe_m),
        (Some(8.), Some(8.), Some(3.2))
    );
    assert_eq!(
        w.polygon,
        vec![
            punkt(5., 5.),
            punkt(15., 5.),
            punkt(15., 5. - 3.2),
            punkt(5., 5. - 3.2)
        ]
    );
    assert_eq!(w.ansatz, Some(Wandhoehenansatz::Maximal));
}
#[test]
fn nrw_mittel_aus_den_kanten_und_hinweis_bei_ungleichmaessigem_gelaende() {
    // Gelände steigt von 100 auf 101 m: H = (8 + 7) / 2 = 7,5 m, T = 3 m.
    let p = profilplan(&[(0., 100.), (10., 101.)], &[(0., 108.), (10., 108.)]);
    assert_eq!(p.regeln.wandhoehe, Some(Wandhoehenansatz::MittelKanten));
    let r = berechnen(&p).unwrap();
    let w = &r.waende[0];
    assert_eq!(w.wandhoehe_m, Some(7.5));
    assert!((w.tiefe_m.unwrap() - 3.).abs() < 1e-9);
    assert!(r.konfliktfrei, "{:?}", w.offen);
    assert!(!w.herleitung.iter().any(|h| h.starts_with("Hinweis")));
    // Senke in der Wandmitte: Mittel aus den Kanten bleibt, Hinweis erscheint.
    let p = profilplan(
        &[(0., 100.), (5., 99.), (10., 101.)],
        &[(0., 108.), (10., 108.)],
    );
    let w = &berechnen(&p).unwrap().waende[0];
    assert_eq!(w.wandhoehe_m, Some(7.5));
    assert!(w.herleitung.iter().any(|h| h.starts_with("Hinweis")));
    assert_eq!(w.profil.len(), 3);
}
#[test]
fn hoehensprung_der_oberkante_bildet_wandteile() {
    // 0–4 m: H = 8 m → T = 3,2 m; 4–10 m: H = 5 m → T = 3 m (Mindesttiefe).
    let p = profilplan(
        &[(0., 100.), (10., 100.)],
        &[(0., 108.), (4., 108.), (4., 105.), (10., 105.)],
    );
    let w = &berechnen(&p).unwrap().waende[0];
    assert_eq!(w.wandteile.len(), 2);
    assert_eq!(
        (w.wandteile[0].wandhoehe_m, w.wandteile[1].wandhoehe_m),
        (8., 5.)
    );
    assert_eq!(w.wandteile[1].tiefe_m, Some(3.));
    assert!((flaeche(&w.polygon) - (4. * 3.2 + 6. * 3.)).abs() < 1e-9);
    assert_eq!(w.polygon.len(), 6, "{:?}", w.polygon);
    // Ausdrückliche Wandteilgrenze ohne Sprung: zwei gleiche Wandteile.
    let mut p = profilplan(&[(0., 100.), (10., 100.)], &[(0., 108.), (10., 108.)]);
    p.waende[0].wandteilgrenzen = vec![4.];
    let w = &berechnen(&p).unwrap().waende[0];
    assert_eq!(w.wandteile.len(), 2);
    assert_eq!(w.polygon.len(), 4);
}
#[test]
fn punktweise_tiefe_mit_knick_an_der_mindesttiefe() {
    // H fällt linear von 10 auf 5 m; 0,4 H = 3 m bei H = 7,5 m, also bei 5 m.
    let mut p = profilplan(&[(0., 100.), (10., 105.)], &[(0., 110.), (10., 110.)]);
    p.regeln.wandhoehe = Some(Wandhoehenansatz::Punktweise);
    let w = &berechnen(&p).unwrap().waende[0];
    assert_eq!(w.tiefe_m, Some(4.));
    assert!((flaeche(&w.polygon) - (5. * (4. + 3.) / 2. + 5. * 3.)).abs() < 1e-9);
    assert!(
        w.polygon
            .iter()
            .any(|q| (q.x - 10.).abs() < 1e-9 && (q.y - 2.).abs() < 1e-9)
    );
}
#[test]
fn flaechengemittelte_wandhoehe() {
    let mut p = profilplan(
        &[(0., 100.), (5., 98.), (10., 100.)],
        &[(0., 108.), (10., 108.)],
    );
    p.regeln.wandhoehe = Some(Wandhoehenansatz::MittelProfil);
    assert_eq!(berechnen(&p).unwrap().waende[0].wandhoehe_m, Some(9.));
}
#[test]
fn unvollstaendiges_oder_negatives_profil_bleibt_offen() {
    let p = profilplan(&[(0., 100.), (6., 100.)], &[(0., 108.), (10., 108.)]);
    let w = &berechnen(&p).unwrap().waende[0];
    assert!(w.tiefe_m.is_none());
    assert!(w.offen.iter().any(|o| o.contains("deckt die Wand nicht")));
    let p = profilplan(&[(0., 100.), (10., 109.)], &[(0., 108.), (10., 108.)]);
    let w = &berechnen(&p).unwrap().waende[0];
    assert!(w.offen.iter().any(|o| o.contains("unter dem Gelände")));
}
#[test]
fn nrw_giebel_und_gauben_automatisch() {
    let mut p = plan();
    let w = &mut p.waende[0];
    w.dachart = Dachart::Giebel;
    w.dachhoehe_m = Some(4.5);
    for (a, b, anteil) in [
        (Some(40.), Some(40.), Some(1.5)),
        (Some(75.), Some(75.), Some(4.5)),
        (Some(60.), None, Some(1.5)),
        (Some(75.), None, None),
    ] {
        p.waende[0].dachneigung_grad = a;
        p.waende[0].dachneigung_b_grad = b;
        let r = berechnen(&p).unwrap();
        assert_eq!(r.waende[0].dachanteil_m, anteil, "{a:?}/{b:?}");
    }
    let w = &mut p.waende[0];
    w.dachart = Dachart::Dachflaeche;
    (w.dachhoehe_m, w.dachneigung_grad) = (Some(3.), Some(30.));
    w.gauben_laenge_m = Some(6.);
    assert!((berechnen(&p).unwrap().waende[0].dachanteil_m.unwrap() - 1.).abs() < 1e-9);
    p.waende[0].gauben_laenge_m = Some(4.);
    assert_eq!(berechnen(&p).unwrap().waende[0].dachanteil_m, Some(0.));
}
#[test]
fn nrw_tiefenfaelle() {
    // H = 10 m: Regeltiefe 4 m.
    let mut p = profilplan(&[(0., 100.), (10., 100.)], &[(0., 110.), (10., 110.)]);
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(4.));
    p.waende[0].tiefenfall = Some(Tiefenfall::WohngebaeudeGk12);
    let w = &berechnen(&p).unwrap().waende[0];
    assert_eq!(w.tiefe_m, Some(3.));
    assert!(w.offen.iter().any(|o| o.contains("begründen")));
    p.waende[0].tiefenfall_grundlage = "Einfamilienhaus, GK 1, zwei Geschosse".into();
    assert!(berechnen(&p).unwrap().waende[0].offen.is_empty());
    // Öffentliche Fläche im WA: keine abweichende Tiefe, Regeltiefe.
    p.waende[0].tiefenfall = Some(Tiefenfall::OeffentlicheFlaeche);
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(4.));
    p.recht.gebiet = "MK".into();
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(3.));
    p.waende[0].tiefenfall = None;
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(3.));
    p.waende[0].oberkante = vec![hp(0., 120.), hp(10., 120.)];
    assert_eq!(berechnen(&p).unwrap().waende[0].tiefe_m, Some(5.));
}
fn umriss(ecken: &[(f64, f64, Option<f64>)]) -> AbstandsUmriss {
    AbstandsUmriss {
        id: Uuid::from_u128(50),
        bezeichnung: "Haus".into(),
        gebaeude_id: None,
        grundstueck_id: Some(Uuid::from_u128(1)),
        ecken: ecken
            .iter()
            .map(|&(x, y, gelaende_m)| UmrissEcke { x, y, gelaende_m })
            .collect(),
        oberkante_m: Some(106.),
        quelle: "Lageplan".into(),
    }
}
#[test]
fn umriss_erzeugt_und_fuehrt_die_waende() {
    let mut p = plan();
    p.waende.clear();
    p.umrisse = vec![umriss(&[
        (5., 5., Some(100.)),
        (15., 5., Some(100.5)),
        (15., 12., Some(101.)),
        (5., 12., None),
    ])];
    assert!(umrisse_einbinden(&mut p).is_empty());
    assert_eq!(p.waende.len(), 4);
    let sued = &p.waende[0];
    assert_eq!(
        (sued.von.clone(), sued.bis.clone()),
        (punkt(5., 5.), punkt(15., 5.))
    );
    assert_eq!(sued.aussen, Aussenrichtung::Rechts);
    assert_eq!(
        sued.gelaende
            .iter()
            .map(|h| (h.station_m, h.hoehe_m))
            .collect::<Vec<_>>(),
        vec![(0., 100.), (10., 100.5)]
    );
    assert_eq!(sued.oberkante.len(), 2);
    // West: Ecke 4 ohne Höhe – nur der Endpunkt an Ecke 1 ist belegt.
    assert_eq!(p.waende[3].gelaende.len(), 1);
    // Wiederholtes Einbinden ändert nichts; Wandangaben bleiben erhalten.
    p.waende[0].dachart = Dachart::Flach;
    let vorher = p.clone();
    umrisse_einbinden(&mut p);
    assert_eq!(p, vorher);
    // Abstandsfläche der Südseite liegt außen (unterhalb).
    let r = berechnen(&p).unwrap();
    assert!(r.waende[0].polygon.iter().all(|q| q.y <= 5.));
    // Umlaufrichtung im Uhrzeigersinn: Außenseite links.
    p.umrisse[0].ecken.reverse();
    umrisse_einbinden(&mut p);
    assert!(p.waende.iter().all(|w| w.aussen == Aussenrichtung::Links));
    // Ecke entfernt: die vierte Seite entfällt mit Hinweis.
    p.umrisse[0].ecken.pop();
    let hinweise = umrisse_einbinden(&mut p);
    assert_eq!(p.waende.len(), 3);
    assert_eq!(hinweise.len(), 1, "{hinweise:?}");
    // Abweichende Wand wird erkannt.
    p.waende[0].von.x += 1.;
    assert!(
        berechnen(&p).unwrap().waende[0]
            .offen
            .iter()
            .any(|o| o.contains("weicht vom Gebäudeumriss"))
    );
}
#[test]
fn ueberdeckung_an_der_innenecke_nach_winkelregel() {
    let mut p = plan();
    let mut a = p.waende[0].clone();
    (a.von, a.bis, a.aussen) = (punkt(2., 10.), punkt(10., 10.), Aussenrichtung::Links);
    let mut b = a.clone();
    b.id = Uuid::from_u128(3);
    (b.von, b.bis) = (punkt(10., 10.), punkt(10., 18.));
    p.waende = vec![a, b];
    let r = berechnen(&p).unwrap();
    assert_eq!(r.ueberdeckungen.len(), 1);
    let u = &r.ueberdeckungen[0];
    assert!((u.winkel_grad.unwrap() - 90.).abs() < 1e-9);
    assert!(u.zulaessig.is_some());
    assert!(r.konfliktfrei, "{:?}", r.offen);
    p.regeln.ueberdeckung = None;
    let r = berechnen(&p).unwrap();
    assert!(r.ueberdeckungen[0].zulaessig.is_none());
    assert!(!r.konfliktfrei);
}
#[test]
fn gelaende_aus_cad_hoehen() {
    let w = &plan().waende[0];
    let quelle = Dokumentbezug {
        dokument_id: crate::domain::DocumentId(Uuid::from_u128(9)),
        version: 1,
    };
    let h = |x: f64, y: f64, hoehe_m: f64| CadHoehe {
        x,
        y,
        hoehe_m,
        ebene: "Höhen".into(),
        art: "Höhenkote".into(),
        quelle: quelle.clone(),
    };
    let v = gelaende_vorschlag(
        w,
        &[
            h(5.1, 5.3, 100.),
            h(10., 4.8, 100.5),
            h(10.02, 5.5, 100.7),
            h(17., 5., 99.),
            h(10., 9., 98.),
        ],
        1.,
    )
    .unwrap();
    assert_eq!(
        v.iter()
            .map(|p| (p.station_m, p.hoehe_m))
            .collect::<Vec<_>>(),
        vec![(0.1, 100.), (5., 100.5)]
    );
    assert!(gelaende_vorschlag(w, &[], 50.).is_err());
}

#[test]
fn rechenstand_1_wird_unveraendert_wiederholt() {
    // Ergebnis eines Altplans, wie es Rechenstand 1 archiviert hat.
    let mut p = plan();
    p.regeln.wandhoehe = None;
    p.regeln.giebel = None;
    p.regeln.gauben = None;
    p.regeln.ueberdeckung = None;
    p.waende[0].gelaende_b_m = Some(101.);
    let r = berechnen_stand(&p, "abstand-projekt-1").unwrap();
    assert_eq!(r.rechenstand, "abstand-projekt-1");
    let w = &r.waende[0];
    assert_eq!(
        w.herleitung[0],
        "Konservativer Maximalansatz: max(108 − 100, 108 − 101) = 8 m. Bei nichtlinearem Verlauf Wand in Abschnitte aufteilen."
    );
    assert!(w.ansatz.is_none() && w.profil.is_empty() && w.wandteile.is_empty());
    let json = serde_json::to_value(&r).unwrap();
    assert!(json["waende"][0].get("profil").is_none());
    // Gleiche Geometrie wie im aktuellen Stand.
    assert_eq!(w.polygon, berechnen(&p).unwrap().waende[0].polygon);
    assert!(berechnen_stand(&p, "abstand-projekt-9").is_err());
}
