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
fn numerische_grenznaehe_ist_keine_freigabe() {
    let mut p = plan();
    p.waende[0].von.y = 3.2;
    p.waende[0].bis.y = 3.2;
    let r = berechnen(&p).unwrap();
    assert!(!r.konfliktfrei);
    assert_eq!(r.waende[0].status, AbstandsStatus::Grenzfall);
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
