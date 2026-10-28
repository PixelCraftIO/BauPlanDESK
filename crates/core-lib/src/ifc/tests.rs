use super::testdaten::{efh, efh_ifc4, wand_ifc2x3};
use super::*;

fn modell(text: &str) -> Modell {
    Modell::lesen("modell.ifc", text.as_bytes()).unwrap()
}
fn nah(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn efh_struktur_einheiten_und_georeferenz() {
    let m = modell(&efh_ifc4());
    let a = &m.auszug;
    assert_eq!(a.schema, "IFC4");
    assert_eq!(a.einheit, "Millimeter");
    assert!(nah(a.meter_je_einheit, 0.001));
    assert_eq!(a.projekt.as_deref(), Some("Neubau Einfamilienhaus"));
    assert_eq!(a.system.as_deref(), Some("BauPlanIO Testdaten"));
    let geo = a.georeferenz.as_ref().unwrap();
    assert_eq!(geo.crs.as_deref(), Some("EPSG:25832"));
    assert_eq!(geo.ostwert, Some(356000.));
    assert!(nah(geo.drehung_grad.unwrap(), 0.));
    assert_eq!(a.gebaeude.len(), 1);
    assert_eq!(a.geschosse.len(), 2);
    assert_eq!(a.geschosse[0].name, "EG");
    assert_eq!(a.geschosse[1].hoehe_m, Some(efh::GESCHOSSHOEHE));
    assert_eq!(
        a.geschosse[0].teil_von.as_deref(),
        Some(a.gebaeude[0].global_id.as_str())
    );
    // 5 Wände + Decke + Bodenplatte + Schornstein + Proxy im EG; 5 Wände + Dach + Dachplatte im OG; 4 Räume.
    assert_eq!(a.bericht.elemente, 20, "{:?}", a.bericht.klassen);
    assert_eq!(a.bericht.raeume, 4);
    assert_eq!(a.bericht.raeume_mit_flaechenmenge, 4);
    assert!(a.bericht.hinweise.is_empty(), "{:?}", a.bericht.hinweise);
    let proxy = a
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCBUILDINGELEMENTPROXY")
        .unwrap();
    assert_eq!(
        proxy.geschoss.as_deref(),
        Some(a.geschosse[0].global_id.as_str())
    );
    // Dachplatte gehört über das Dach zum OG.
    let platte = a.elemente.iter().find(|e| e.name == "Dachplatte").unwrap();
    let dach = a.elemente.iter().find(|e| e.klasse == "IFCROOF").unwrap();
    assert_eq!(platte.teil_von.as_deref(), Some(dach.global_id.as_str()));
    assert_eq!(
        platte.geschoss.as_deref(),
        Some(a.geschosse[1].global_id.as_str())
    );
    assert_eq!(platte.vordefiniert.as_deref(), Some("ROOF"));
    assert!(dach.geometrie.is_none());
}

#[test]
fn efh_eigenschaften_und_mengen() {
    let m = modell(&efh_ifc4());
    let w = m
        .auszug
        .elemente
        .iter()
        .find(|e| e.name == "Außenwand Süd")
        .unwrap();
    let ext = w
        .eigenschaften
        .iter()
        .find(|p| p.name == "IsExternal")
        .unwrap();
    assert_eq!(
        (ext.satz.as_str(), ext.wert.as_str()),
        ("Pset_WallCommon", "Ja")
    );
    let r = m
        .auszug
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCSPACE")
        .unwrap();
    assert_eq!(r.name, "0.01");
    assert_eq!(r.langname.as_deref(), Some("Wohnen"));
    assert_eq!(r.vordefiniert.as_deref(), Some("INTERNAL"));
    let f = r.mengen.iter().find(|q| q.name == "NetFloorArea").unwrap();
    assert!(nah(f.wert, efh::RAUM_MENGE));
    // Längenmengen in Millimetern werden in Meter umgerechnet.
    let h = r.mengen.iter().find(|q| q.name == "Height").unwrap();
    assert!(nah(h.wert, efh::LICHTE_HOEHE));
}

#[test]
fn efh_geometrie_in_metern() {
    let m = modell(&efh_ifc4());
    let a = &m.auszug;
    let r = a.elemente.iter().find(|e| e.klasse == "IFCSPACE").unwrap();
    let g = r.geometrie.as_ref().unwrap();
    assert!(
        nah(g.grundflaeche_m2, efh::raumflaeche()),
        "{}",
        g.grundflaeche_m2
    );
    assert!(nah(g.unten_m, 0.) && nah(g.oben_m, efh::LICHTE_HOEHE));
    assert!(nah(
        g.volumen_m3.unwrap(),
        efh::raumflaeche() * efh::LICHTE_HOEHE
    ));
    assert!(!g.naeherung);
    // Wand Ost: gedrehte Platzierung, 36,5 cm × 7,27 m.
    let w = a
        .elemente
        .iter()
        .find(|e| e.name == "Außenwand Ost")
        .unwrap();
    let g = w.geometrie.as_ref().unwrap();
    assert!(nah(
        g.grundflaeche_m2,
        efh::WAND * (efh::BREITE - 2. * efh::WAND)
    ));
    let xs: Vec<f64> = g.grundriss[0].aussen.iter().map(|p| p[0]).collect();
    assert!(
        xs.iter()
            .all(|x| *x >= efh::LAENGE - efh::WAND - 1e-6 && *x <= efh::LAENGE + 1e-6),
        "{xs:?}"
    );
    // OG-Wände stehen auf Geschosshöhe.
    let og = a
        .elemente
        .iter()
        .filter(|e| e.name == "Außenwand Süd")
        .nth(1)
        .unwrap();
    assert!(nah(
        og.geometrie.as_ref().unwrap().unten_m,
        efh::GESCHOSSHOEHE
    ));
    // Schornstein als Flächenmodell: Projektion 0,25 m², Volumen aus den Flächen.
    let s = a
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCCHIMNEY")
        .unwrap();
    let g = s.geometrie.as_ref().unwrap();
    assert!(nah(g.grundflaeche_m2, 0.25), "{g:?}");
    assert!(nah(g.oben_m, efh::FIRST + 0.9));
    assert!(nah(g.volumen_m3.unwrap(), 0.25 * (efh::FIRST + 0.9)));
    // Bodenplatte unter ±0.
    let bp = a.elemente.iter().find(|e| e.name == "Bodenplatte").unwrap();
    assert!(nah(bp.geometrie.as_ref().unwrap().unten_m, -0.25));
    assert!(nah(
        bp.geometrie.as_ref().unwrap().grundflaeche_m2,
        efh::LAENGE * efh::BREITE
    ));
}

#[test]
fn ifc2x3_mit_platzierung_ohne_elevation() {
    let m = modell(&wand_ifc2x3());
    let a = &m.auszug;
    assert_eq!(a.schema, "IFC2X3");
    assert_eq!(a.einheit, "Meter");
    assert_eq!(a.geschosse[0].hoehe_m, Some(3.));
    let w = &a.elemente[0];
    assert_eq!(w.klasse, "IFCWALLSTANDARDCASE");
    assert_eq!(klassentitel(&w.klasse), "Wand");
    assert_eq!(w.vordefiniert, None);
    let g = w.geometrie.as_ref().unwrap();
    assert!(nah(g.grundflaeche_m2, 1.2) && nah(g.unten_m, 3.) && nah(g.oben_m, 5.5));
    let geo = a.georeferenz.as_ref().unwrap();
    assert!(nah(geo.breite.unwrap(), 51.5) && nah(geo.laenge.unwrap(), 7.));
}

#[test]
fn ifczip_und_fehlerfaelle() {
    use std::io::Write;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        z.start_file("haus.ifc", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(wand_ifc2x3().as_bytes()).unwrap();
        z.finish().unwrap();
    }
    let m = Modell::lesen("haus.ifczip", buf.get_ref()).unwrap();
    assert_eq!(m.auszug.elemente.len(), 1);
    assert!(
        Modell::lesen("a.ifcxml", b"<x/>")
            .unwrap_err()
            .to_string()
            .contains("IFC-XML")
    );
    let ohne_schema = wand_ifc2x3().replace("FILE_SCHEMA(('IFC2X3'))", "FILE_SCHEMA(('AP214'))");
    assert!(Modell::lesen("a.ifc", ohne_schema.as_bytes()).is_err());
}

#[test]
fn unbekannte_darstellung_wird_berichtet() {
    let t = wand_ifc2x3().replace(
        "#20=IFCEXTRUDEDAREASOLID(#18,#5,#19,2.5);",
        "#20=IFCREVOLVEDAREASOLID(#18,#5,$,1.57);",
    );
    let m = modell(&t);
    assert!(m.auszug.elemente[0].geometrie.is_none());
    assert_eq!(
        m.auszug.elemente[0].geometrie_offen,
        vec!["IFCREVOLVEDAREASOLID".to_string()]
    );
    assert_eq!(m.auszug.bericht.offene_darstellungen[0].anzahl, 1);
    assert_eq!(m.auszug.bericht.ohne_geometrie, 1);
}

#[test]
fn bogen_und_indizierte_kurven() {
    // Kreisprofil r = 0,5 m und indizierte Kurve mit Bogen: Flächen nahe dem Sollwert.
    let kreis = wand_ifc2x3().replace(
        "#18=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#17);",
        "#18=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.5);",
    );
    let g = modell(&kreis).auszug.elemente[0].geometrie.clone().unwrap();
    assert!(
        (g.grundflaeche_m2 - std::f64::consts::PI * 0.25).abs() < 0.01,
        "{}",
        g.grundflaeche_m2
    );
    let halbkreis = wand_ifc2x3().replace(
        "#18=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#17);",
        "#30=IFCCARTESIANPOINTLIST2D(((0.,0.),(2.,0.),(1.,1.)));\n#31=IFCINDEXEDPOLYCURVE(#30,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,1))),$);\n#18=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#31);",
    );
    let g = modell(&halbkreis).auszug.elemente[0]
        .geometrie
        .clone()
        .unwrap();
    assert!(
        (g.grundflaeche_m2 - std::f64::consts::PI / 2.).abs() < 0.01,
        "{}",
        g.grundflaeche_m2
    );
}

#[test]
fn grundriss_schnitt_je_geschoss() {
    let m = modell(&efh_ifc4());
    let a = &m.auszug;
    let eg = grundriss(a, &a.geschosse[0].global_id, None).unwrap();
    let klassen: Vec<&str> = eg.teile.iter().map(|t| t.klasse.as_str()).collect();
    assert_eq!(klassen.iter().filter(|k| **k == "IFCWALL").count(), 5);
    assert_eq!(klassen.iter().filter(|k| **k == "IFCSPACE").count(), 2);
    assert!(klassen.contains(&"IFCCHIMNEY"));
    // Lichtschacht liegt unter der Geschossebene, Decke über der Schnitthöhe.
    assert!(!klassen.contains(&"IFCBUILDINGELEMENTPROXY") && !klassen.contains(&"IFCSLAB"));
    let og = grundriss(a, &a.geschosse[1].global_id, Some(1.2)).unwrap();
    assert!(
        og.teile
            .iter()
            .any(|t| t.klasse == "IFCCHIMNEY" && t.projektion)
    );
    assert_eq!(og.min, eg.min);
    assert!(nah(eg.max[0] - eg.min[0], efh::LAENGE));
    // Als Plan in Metern: eine Ebene je Klasse, geschlossene Ringe.
    let p = als_plan(&eg);
    let namen: Vec<&str> = p.ebenen.iter().map(|e| e.name.as_str()).collect();
    assert!(
        namen.contains(&"Wand") && namen.contains(&"Räume"),
        "{namen:?}"
    );
    assert!(
        p.linien
            .iter()
            .all(|l| l.geschlossen && l.punkte.len() >= 4)
    );
    assert!(grundriss(a, "gibt-es-nicht", None).is_err());
    assert!(grundriss(a, &a.geschosse[0].global_id, Some(f64::NAN)).is_err());
}
