use super::*;
use serde_json::{Value, json};
fn plan(a: BerechnungsArt) -> FachPlan {
    let mut p = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == a)
        .unwrap()
        .plan;
    p.grundstueck_id = Some(Uuid::new_v4());
    p.regelkontext.anwendbarkeit_bestaetigt = true;
    p.regelkontext.quelle = "Fiktiver unabhängig geprüfter Kontrollansatz".into();
    p.regelkontext.pruefvermerk = "Kontrollfall, keine reale Rechtsfreigabe".into();
    p
}
fn k(r: &FachErgebnis, id: &str) -> f64 {
    r.kennzahlen
        .iter()
        .find(|k| k.id == id)
        .unwrap()
        .wert
        .unwrap()
}
fn wohn() -> FachPlan {
    let mut p = plan(BerechnungsArt::Wohnflaechen);
    p.eingaben["positionen"][0]["aufmass_m2"] = json!(100);
    p.eingaben["positionen"][0]["lichte_hoehe_m"] = json!(2.5);
    p.eingaben["positionen"][0]["quelle"] = json!("Prüfaufmaß");
    p.eingaben["aufmass_vollstaendig"] = json!(true);
    p
}
#[test]
fn alle_vorlagen_sind_valide_und_ohne_freigabe_offen() {
    let v = vorlagen().unwrap();
    assert_eq!(v.len(), 8);
    for p in v {
        assert!(!berechnen(&p.plan).unwrap().vollstaendig);
    }
}
#[test]
fn wohnflaechen_hoehenzonen_grenzwerte_und_fehlende_hoehe() {
    let mut p = wohn();
    for (h, n) in [(0.99, 0.), (1., 50.), (1.99, 50.), (2., 100.)] {
        p.eingaben["positionen"][0]["lichte_hoehe_m"] = json!(h);
        assert_eq!(k(&berechnen(&p).unwrap(), "wohnflaeche"), n);
    }
    p.eingaben["positionen"][0]["lichte_hoehe_m"] = Value::Null;
    assert!(!berechnen(&p).unwrap().vollstaendig);
}
#[test]
fn wohnflaechen_balkon_sonderfaktor_und_abzuege() {
    let mut p = wohn();
    p.eingaben["positionen"][0]["typ"] = json!("balkon");
    p.eingaben["positionen"][0]["aufmass_m2"] = json!(12);
    assert_eq!(k(&berechnen(&p).unwrap(), "wohnflaeche"), 3.);
    p.eingaben["positionen"][0]["sonderfaktor"] = json!(0.5);
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["begruendung"] = json!("Qualität nachgewiesen");
    assert_eq!(k(&berechnen(&p).unwrap(), "wohnflaeche"), 6.);
    p.eingaben["positionen"][0]["sonderfaktor"] = json!(0.75);
    assert!(berechnen(&p).is_err());
    let mut p = wohn();
    p.eingaben["abzuege"] = json!([{"id":"a","teilflaeche":"flaeche1","bezeichnung":"Treppe","flaeche_m2":2,"typ":"treppe","hoehe_m":2,"stufen":3,"grundlage":"Aufmaß"}]);
    assert_eq!(k(&berechnen(&p).unwrap(), "wohnflaeche"), 100.);
    p.eingaben["abzuege"][0]["stufen"] = json!(4);
    assert_eq!(k(&berechnen(&p).unwrap(), "wohnflaeche"), 98.);
    p.eingaben["abzuege"][0]["flaeche_m2"] = json!(101);
    assert!(berechnen(&p).is_err());
}
#[test]
fn din277_klassensummen_und_keine_erfundene_bgf() {
    let mut p = plan(BerechnungsArt::Din277);
    let row = p.eingaben["positionen"][0].clone();
    p.eingaben["positionen"] = json!([]);
    for (i, typ, area) in [
        ("a", "nuf", 100.),
        ("b", "tf", 10.),
        ("c", "vf", 20.),
        ("d", "kgf", 30.),
    ] {
        let mut r = row.clone();
        r["id"] = json!(i);
        r["art"] = json!(typ);
        r["aufmass_m2"] = json!(area);
        r["quelle"] = json!("Aufmaß");
        p.eingaben["positionen"].as_array_mut().unwrap().push(r);
    }
    p.eingaben["klassifikation_geprueft"] = json!(true);
    let r = berechnen(&p).unwrap();
    assert_eq!(k(&r, "nrf"), 130.);
    assert_eq!(
        r.kennzahlen.iter().find(|k| k.id == "bgf").unwrap().wert,
        None
    );
    p.eingaben["kgf_vollstaendig"] = json!(true);
    assert_eq!(k(&berechnen(&p).unwrap(), "bgf"), 160.);
    assert_eq!(k(&berechnen(&p).unwrap(), "nuf"), 100.);
}
#[test]
fn vollgeschosse_strikte_flaechen_grenze_und_fehlender_bezug() {
    let mut p = plan(BerechnungsArt::Vollgeschosse);
    p.eingaben["geschosse"][0]["grundflaeche_m2"] = json!(100);
    p.eingaben["geschosse"][0]["unterliegend_m2"] = json!(100);
    p.eingaben["geschosse"][0]["hoehenquelle"] = json!("Schnitt");
    p.eingaben["teilflaechen"] = json!([{"id":"a","geschoss_id":"eg","bezeichnung":"Hoch","flaeche_m2":75,"lichte_hoehe_m":2.3},{"id":"b","geschoss_id":"eg","bezeichnung":"Niedrig","flaeche_m2":25,"lichte_hoehe_m":1}]);
    p.eingaben["aufmass_geprueft"] = json!(true);
    p.eingaben["sonderfaelle_geprueft"] = json!(true);
    assert_eq!(k(&berechnen(&p).unwrap(), "vollgeschosse"), 0.);
    p.eingaben["regeln"]["anteil_streng"] = json!(false);
    assert_eq!(k(&berechnen(&p).unwrap(), "vollgeschosse"), 1.);
    p.eingaben["geschosse"][0]["unterliegend_m2"] = Value::Null;
    assert!(!berechnen(&p).unwrap().vollstaendig);
}
#[test]
fn hoehen_und_dachneigung_mit_festsetzungsvergleich() {
    let mut p = plan(BerechnungsArt::Hoehen);
    p.eingaben["punkte"] = json!([{"id":"t","bezeichnung":"Traufe","art":"traufe","absolut_m":105,"bezug_m":100,"quelle":"Schnitt"},{"id":"f","bezeichnung":"First","art":"first","absolut_m":110,"bezug_m":100,"quelle":"Schnitt"}]);
    p.eingaben["daecher"] = json!([{"id":"d","bezeichnung":"Satteldach","traufe_id":"t","first_id":"f","horizontale_m":5,"min_neigung_grad":40,"max_neigung_grad":50,"quelle":"Aufmaß"}]);
    let r = berechnen(&p).unwrap();
    assert_eq!(k(&r, "hoehe:t"), 5.);
    assert!((k(&r, "dach:d") - 45.).abs() < 1e-9);
    p.eingaben["regeln"] = json!([{"art":"first","minimum_m":null,"maximum_m":9,"absolut":false,"quelle":"Planfestsetzung"}]);
    assert!(!berechnen(&p).unwrap().vollstaendig);
}
#[test]
fn freiflaechen_bilanz_und_bezugsflaechen() {
    let mut p = plan(BerechnungsArt::Freiflaechen);
    p.eingaben["grundstuecksflaeche_m2"] = json!(500);
    p.eingaben["bauland_m2"] = json!(400);
    p.eingaben["positionen"] = json!([{"id":"a","bezeichnung":"Haus","typ":"dach","flaeche_m2":200,"quelle":"Plan"},{"id":"b","bezeichnung":"Grün","typ":"gruen","flaeche_m2":300,"quelle":"Plan"}]);
    p.eingaben["bilanz_geprueft"] = json!(true);
    p.eingaben["regeln"]["min_gruenanteil"] = json!(0.6);
    let r = berechnen(&p).unwrap();
    assert!(r.vollstaendig);
    assert_eq!(k(&r, "gruenanteil"), 60.);
    p.eingaben["regeln"]["gruen_bezug"] = json!("bauland");
    assert_eq!(k(&berechnen(&p).unwrap(), "gruenanteil"), 75.);
    p.eingaben["positionen"][1]["flaeche_m2"] = json!(301);
    assert!(!berechnen(&p).unwrap().vollstaendig);
}
#[test]
fn rueckhaltung_kontrollrechnung_und_ungepruefte_versickerung() {
    let mut p = plan(BerechnungsArt::Entwaesserung);
    p.eingaben["positionen"][0]["flaeche_m2"] = json!(100);
    p.eingaben["positionen"][0]["quelle"] = json!("Dachaufmaß");
    p.eingaben["regeln"]["regen"] = json!([{"id":"a","dauer_min":10,"intensitaet_l_s_ha":100,"quelle":"Fiktiver Kontrollregen"},{"id":"b","dauer_min":60,"intensitaet_l_s_ha":30,"quelle":"Fiktiver Kontrollregen"}]);
    p.eingaben["flaechen_geprueft"] = json!(true);
    p.eingaben["regendaten_geprueft"] = json!(true);
    assert!((k(&berechnen(&p).unwrap(), "rueckhaltung") - 1.08).abs() < 1e-9);
    p.eingaben["regeln"]["drossel_l_s"] = json!(0.1);
    assert!((k(&berechnen(&p).unwrap(), "rueckhaltung") - 0.72).abs() < 1e-9);
    p.eingaben["regeln"]["kf_m_s"] = json!(0.001);
    p.eingaben["regeln"]["sickerflaeche_m2"] = json!(10);
    let r = berechnen(&p).unwrap();
    assert!(!r.vollstaendig);
    assert!((k(&r, "abfluss") - 0.1).abs() < 1e-9);
}
fn parking() -> FachPlan {
    let mut p = plan(BerechnungsArt::Stellplaetze);
    p.eingaben["positionen"][0]["wohneinheiten"] = json!(4);
    p.eingaben["angebot"] = json!([{"id":"a","bezeichnung":"Kfz","art":"kfz","anzahl":10,"besucher":5,"barrierefrei":0,"ausserhalb":false,"anerkannt":true,"lage_und_sicherung":"Lageplan"},{"id":"b","bezeichnung":"Rad","art":"fahrrad","anzahl":10,"besucher":0,"barrierefrei":0,"ausserhalb":false,"anerkannt":true,"lage_und_sicherung":"Lageplan"}]);
    p
}
#[test]
fn stellplaetze_faktoren_rundung_besucher_und_ambiguitaet() {
    let mut p = parking();
    let r = berechnen(&p).unwrap();
    assert!(r.vollstaendig);
    assert_eq!(k(&r, "kfz_bedarf"), 3.);
    assert_eq!(k(&r, "fahrrad_bedarf"), 8.);
    p.eingaben["regeln"]["richtzahlen"][0]["faktor"] = json!(1.5);
    assert_eq!(k(&berechnen(&p).unwrap(), "kfz_bedarf"), 6.);
    let mut p = parking();
    let mut rows = vec![];
    for i in 0..4 {
        let mut row = p.eingaben["positionen"][0].clone();
        row["id"] = json!(format!("n{i}"));
        row["wohneinheiten"] = json!(1);
        rows.push(row);
    }
    p.eingaben["positionen"] = json!(rows);
    p.eingaben["regeln"]["rundungsstufe"] = json!("je_position");
    assert_eq!(k(&berechnen(&p).unwrap(), "kfz_bedarf"), 4.);
    p.eingaben["regeln"]["rundungsstufe"] = json!("gesamt");
    assert_eq!(k(&berechnen(&p).unwrap(), "kfz_bedarf"), 3.);
    p.eingaben["regeln"]["richtzahlen"][0]["besucheranteil"] = json!(0.2);
    p.eingaben["regeln"]["richtzahlen"][0]["besucher_zusaetzlich"] = json!(true);
    assert_eq!(k(&berechnen(&p).unwrap(), "kfz_bedarf"), 4.);
    let mut d = p.eingaben["regeln"]["richtzahlen"][0].clone();
    d["id"] = json!("duplikat");
    p.eingaben["regeln"]["richtzahlen"]
        .as_array_mut()
        .unwrap()
        .push(d);
    assert!(!berechnen(&p).unwrap().vollstaendig);
}
#[test]
fn uebernahmen_ueber_stabile_ids_und_fremde_felder() {
    let p = parking();
    assert_eq!(
        service::importgroesse(&p, "positionen.nutzung1.wohnflaeche_m2").unwrap(),
        (Einheit::M2, Messgroesse::Wohnflaeche)
    );
    assert!(service::importgroesse(&p, "positionen.0.wohnflaeche_m2").is_err());
    let mut p = wohn();
    p.eingaben["unbekannt"] = json!(1);
    assert!(berechnen(&p).is_err());
    let mut e: wohnflaechen::Eingabe = decode(&wohn().eingaben).unwrap();
    e.positionen[0].laenge_m = f64::NAN;
    assert!(wohnflaechen::berechnen(&e).is_err());
}

#[test]
fn baulinie_zuruecktreten_und_baugrenze_vorbau_benoetigen_belegte_entscheidung() {
    let mut p = plan(BerechnungsArt::Ueberbaubarkeit);
    p.eingaben["aufmass_geprueft"] = json!(true);
    p.eingaben["flaechen_abgegrenzt"] = json!(true);
    p.eingaben["positionen"][0]["quelle"] = json!("Plan Kontrollfall / Schnitt A");
    p.eingaben["positionen"][0]["abstand_min_m"] = json!(-0.5);
    p.eingaben["positionen"][0]["abstand_max_m"] = json!(-0.2);
    assert!(berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["art"] = json!("baulinie");
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["entscheidung_bestaetigt"] = json!(true);
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["entscheidung"] =
        json!("Fiktive belegte Entscheidung, kein reales Baurecht");
    assert!(berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["art"] = json!("vorbau");
    p.eingaben["positionen"][0]["abstand_max_m"] = json!(0.8);
    assert!(!berechnen(&p).unwrap().vollstaendig);
    p.eingaben["positionen"][0]["ausserhalb_m2"] = json!(2.4);
    let r = berechnen(&p).unwrap();
    assert!(r.vollstaendig);
    assert_eq!(k(&r, "ueberstand_m"), 0.8);
    assert_eq!(k(&r, "ausserhalb_m2"), 2.4);
    p.eingaben["positionen"][0]["abstand_min_m"] = json!(1.);
    assert!(berechnen(&p).is_err());
}

#[test]
fn vollgeschoss_unvollstaendige_hoehenzone_ist_keine_negative_einstufung() {
    let mut p = plan(BerechnungsArt::Vollgeschosse);
    p.eingaben["regeln"]["vergleich_unterliegend"] = json!(false);
    p.eingaben["geschosse"][0]["grundflaeche_m2"] = json!(100);
    p.eingaben["geschosse"][0]["hoehenquelle"] = json!("Kontrollaufmaß");
    let r = berechnen(&p).unwrap();
    assert_eq!(
        r.kennzahlen
            .iter()
            .find(|k| k.id == "geschoss:eg")
            .unwrap()
            .wert,
        None
    );
    assert_eq!(
        r.kennzahlen
            .iter()
            .find(|k| k.id == "vollgeschosse")
            .unwrap()
            .wert,
        None
    );
    assert!(!r.vollstaendig);
}
