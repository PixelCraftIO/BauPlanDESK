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
    assert_eq!(a.system.as_deref(), Some("BauPlanDESK Testdaten"));
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

fn regeln(p: &Modellpruefung) -> Vec<&str> {
    p.befunde.iter().map(|b| b.regel.as_str()).collect()
}

#[test]
fn pruefung_efh_gegen_bauantrag_ids() {
    let m = modell(&efh_ifc4());
    let p = pruefen(&m.auszug, BAUANTRAG_IDS).unwrap();
    assert_eq!(p.regelwerk, "BauPlanDESK – Modell für den Bauantrag");
    // Das Testhaus erfüllt das IDS; eingebaute Regeln finden Proxy und Mengen.
    assert!(
        p.befunde.iter().all(|b| !b.regel.starts_with("IDS:")),
        "{:#?}",
        p.befunde
    );
    // NetFloorArea 33,00 gegen 33,28 m² Geometrie liegt in der Toleranz.
    assert_eq!(regeln(&p), ["proxy"], "{:#?}", p.befunde);
    let proxy = &p.befunde[0];
    assert_eq!(proxy.stufe, Befundstufe::Warnung);
    assert_eq!(proxy.aktion, Some(Befundaktion::Klassifizieren));
    assert_eq!(proxy.elemente.len(), 1);
    assert!(p.nicht_geprueft.is_empty(), "{:?}", p.nicht_geprueft);
    assert_eq!((p.fehler, p.warnungen, p.hinweise), (0, 1, 0));

    // Veraltete Raummenge (30 m²) und fehlender Raumname.
    let mut a = m.auszug.clone();
    let r = a
        .elemente
        .iter_mut()
        .find(|e| e.klasse == "IFCSPACE")
        .unwrap();
    r.mengen
        .iter_mut()
        .find(|q| q.name == "NetFloorArea")
        .unwrap()
        .wert = 30.;
    r.name.clear();
    r.langname = None;
    let p = pruefen(&a, BAUANTRAG_IDS).unwrap();
    let menge = p.befunde.iter().find(|b| b.regel == "raummenge").unwrap();
    assert_eq!(menge.elemente.len(), 1);
    assert!(
        menge.text.contains("NetFloorArea 30,00 m²"),
        "{}",
        menge.text
    );
    assert!(regeln(&p).contains(&"raumname"));
    assert!(regeln(&p).contains(&"IDS:W-RAUM"));
}

#[test]
fn pruefung_ifc2x3_ohne_raeume_und_eigene_ids() {
    let m = modell(&wand_ifc2x3());
    let p = pruefen(&m.auszug, BAUANTRAG_IDS).unwrap();
    let r = regeln(&p);
    assert!(r.contains(&"IDS:W-RAUM"), "{r:?}");
    assert!(r.contains(&"IDS:W-AUSSENWAND"), "{r:?}");
    assert!(r.contains(&"georeferenz") == m.auszug.georeferenz.is_none());
    let wand = p
        .befunde
        .iter()
        .find(|b| b.regel == "IDS:W-AUSSENWAND")
        .unwrap();
    assert_eq!(wand.aktion, Some(Befundaktion::EigenschaftSetzen));
    assert_eq!(
        wand.eigenschaft.as_deref(),
        Some("Pset_WallCommon.IsExternal")
    );

    // Eigenes IDS: Wert, Verbot, Muster (nicht geprüft), fremdes Schema.
    let ids = r#"<?xml version="1.0"?>
<ids xmlns="http://standards.buildingsmart.org/IDS" xmlns:xs="http://www.w3.org/2001/XMLSchema">
 <info><title>Büro &amp; Test</title></info>
 <specifications>
  <specification name="Außenwände" identifier="F-1" ifcVersion="IFC4">
   <applicability><entity><name><simpleValue>IFCWALL</simpleValue></name></entity></applicability>
   <requirements><property><propertySet><simpleValue>Pset_WallCommon</simpleValue></propertySet><baseName><simpleValue>IsExternal</simpleValue></baseName><value><simpleValue>TRUE</simpleValue></value></property></requirements>
  </specification>
  <specification name="Keine Proxies" identifier="F-2" ifcVersion="IFC4">
   <applicability maxOccurs="0"><entity><name><simpleValue>IFCBUILDINGELEMENTPROXY</simpleValue></name></entity></applicability>
  </specification>
  <specification name="Raumnummern" identifier="H-3" ifcVersion="IFC4">
   <applicability><entity><name><simpleValue>IFCSPACE</simpleValue></name></entity></applicability>
   <requirements><attribute><name><simpleValue>Name</simpleValue></name><value><xs:restriction base="xs:string"><xs:pattern value="[0-9]\.[0-9]{2}"/></xs:restriction></value></attribute>
   <attribute cardinality="prohibited"><name><simpleValue>ObjectType</simpleValue></name></attribute></requirements>
  </specification>
  <specification name="Nur IFC2X3" identifier="F-4" ifcVersion="IFC2X3">
   <applicability minOccurs="1"><entity><name><simpleValue>IFCROOF</simpleValue></name></entity></applicability>
  </specification>
 </specifications>
</ids>"#;
    let efh = modell(&efh_ifc4());
    let (titel, befunde, offen) = ids_pruefen(&efh.auszug, ids).unwrap();
    assert_eq!(titel, "Büro & Test");
    let r: Vec<_> = befunde.iter().map(|b| b.regel.as_str()).collect();
    // Innenwände: IsExternal = FALSE verletzt die Wertvorgabe.
    assert_eq!(r, ["IDS:F-1", "IDS:F-2"], "{befunde:#?}");
    assert_eq!(befunde[0].elemente.len(), 2, "zwei Innenwände");
    assert_eq!(befunde[1].elemente.len(), 1);
    assert!(offen.iter().any(|o| o.contains("pattern")), "{offen:?}");
    assert!(
        offen.iter().any(|o| o.contains("gilt nicht für IFC4")),
        "{offen:?}"
    );
    assert!(ids_pruefen(&efh.auszug, "<kein/>").is_err());
    assert!(ids_pruefen(&efh.auszug, "<ids><specifications>").is_err());
}

#[test]
fn nacharbeit_klassifizieren_zuordnen_ausblenden_und_raum_bilden() {
    use nacharbeit::{anwenden, pruefen as liste_pruefen};
    let m = modell(&efh_ifc4());
    let roh = &m.auszug;
    let gid = |k: &str| {
        roh.elemente
            .iter()
            .find(|e| e.klasse == k)
            .unwrap()
            .global_id
            .clone()
    };
    let (eg, og) = (
        roh.geschosse[0].global_id.clone(),
        roh.geschosse[1].global_id.clone(),
    );
    let proxy = gid("IFCBUILDINGELEMENTPROXY");
    let kamin = gid("IFCCHIMNEY");
    let innen = roh
        .elemente
        .iter()
        .find(|e| {
            e.klasse == "IFCWALL"
                && e.eigenschaften
                    .iter()
                    .any(|p| p.name == "IsExternal" && p.wert == "Nein")
        })
        .unwrap()
        .global_id
        .clone();
    let raum = roh
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCSPACE")
        .unwrap()
        .global_id
        .clone();
    let liste = vec![
        Modellaenderung::Klassifizieren {
            global_id: proxy.clone(),
            klasse: "IFCWALL".into(),
            vordefiniert: None,
        },
        Modellaenderung::GeschossZuordnen {
            global_id: proxy.clone(),
            geschoss: og.clone(),
        },
        Modellaenderung::EigenschaftSetzen {
            global_id: innen.clone(),
            satz: "Pset_WallCommon".into(),
            name: "IsExternal".into(),
            wert: Eigenschaftswert::Logisch { wert: true },
        },
        Modellaenderung::Benennen {
            global_id: raum.clone(),
            name: "0.01".into(),
            langname: Some("Wohnzimmer".into()),
        },
        Modellaenderung::Ausblenden {
            global_id: kamin.clone(),
        },
        Modellaenderung::Ausblenden {
            global_id: "0000000000000000000000".into(),
        },
    ];
    liste_pruefen(roh, &liste).unwrap();
    let mut a = roh.clone();
    let verwaist = anwenden(&mut a, &liste);
    assert_eq!(verwaist, ["0000000000000000000000"]);
    let p = a.element(&proxy).unwrap();
    assert_eq!(
        (p.klasse.as_str(), p.geschoss.as_deref()),
        ("IFCWALL", Some(og.as_str()))
    );
    assert!(a.element(&kamin).is_none());
    assert!(
        a.bericht
            .klassen
            .iter()
            .all(|k| k.klasse != "IFCBUILDINGELEMENTPROXY" && k.klasse != "IFCCHIMNEY")
    );
    assert_eq!(a.bericht.elemente, roh.bericht.elemente - 1);
    let w = a.element(&innen).unwrap();
    assert_eq!(
        w.eigenschaften
            .iter()
            .filter(|p| p.name == "IsExternal")
            .map(|p| p.wert.as_str())
            .collect::<Vec<_>>(),
        ["Ja"]
    );
    assert_eq!(
        a.element(&raum).unwrap().langname.as_deref(),
        Some("Wohnzimmer")
    );
    assert!(
        a.bericht
            .hinweise
            .iter()
            .any(|h| h.contains("nicht enthält"))
    );
    assert!(
        super::pruefen(&a, BAUANTRAG_IDS)
            .unwrap()
            .befunde
            .iter()
            .all(|b| b.regel != "proxy")
    );

    // Räume aus Wänden: zwei Flächen je Geschoss, vorhanden erkannt.
    let v = raumvorschlaege(roh, &eg).unwrap();
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(
        v.iter()
            .all(|r| (r.flaeche_m2 - efh::raumflaeche()).abs() < 1e-3 && r.vorhanden.is_some())
    );
    // Räume ausblenden und neu bilden.
    let mut ohne = roh.clone();
    let raeume: Vec<_> = roh
        .elemente
        .iter()
        .filter(|e| e.klasse == "IFCSPACE" && e.geschoss.as_deref() == Some(eg.as_str()))
        .map(|e| Modellaenderung::Ausblenden {
            global_id: e.global_id.clone(),
        })
        .collect();
    anwenden(&mut ohne, &raeume);
    let v = raumvorschlaege(&ohne, &eg).unwrap();
    assert!(v.iter().all(|r| r.vorhanden.is_none()));
    let mut liste = raeume.clone();
    liste.push(Modellaenderung::RaumBilden {
        global_id: v[0].global_id.clone(),
        geschoss: eg.clone(),
        name: "0.01".into(),
        langname: Some("Wohnen".into()),
        umriss: v[0].umriss.clone(),
        hoehe_m: None,
    });
    liste_pruefen(roh, &liste).unwrap();
    let mut b = roh.clone();
    anwenden(&mut b, &liste);
    let neu = b.element(&v[0].global_id).unwrap();
    let g = neu.geometrie.as_ref().unwrap();
    assert!((g.grundflaeche_m2 - efh::raumflaeche()).abs() < 1e-3);
    assert!(nah(g.oben_m - g.unten_m, efh::GESCHOSSHOEHE));
    assert_eq!(neu.geschoss.as_deref(), Some(eg.as_str()));
    assert_eq!(b.bericht.raeume, 3);

    // Ungültige Nacharbeit.
    let falsch = |c: Modellaenderung| liste_pruefen(roh, &[c]).unwrap_err().to_string();
    assert!(
        falsch(Modellaenderung::Klassifizieren {
            global_id: proxy.clone(),
            klasse: "IFCSPACE".into(),
            vordefiniert: None
        })
        .contains("nicht vorgesehen")
    );
    assert!(
        falsch(Modellaenderung::GeschossZuordnen {
            global_id: proxy.clone(),
            geschoss: "1111111111111111111111".into()
        })
        .contains("Geschoss")
    );
    assert!(
        falsch(Modellaenderung::Ausblenden {
            global_id: "kurz".into()
        })
        .contains("GlobalId")
    );
    assert!(
        falsch(Modellaenderung::RaumBilden {
            global_id: raum.clone(),
            geschoss: eg.clone(),
            name: "x".into(),
            langname: None,
            umriss: v[0].umriss.clone(),
            hoehe_m: None
        })
        .contains("neue GlobalId")
    );
    assert!(
        falsch(Modellaenderung::RaumBilden {
            global_id: v[1].global_id.clone(),
            geschoss: eg.clone(),
            name: "x".into(),
            langname: None,
            umriss: vec![[0., 0.], [0.1, 0.], [0., 0.1]],
            hoehe_m: None
        })
        .contains("0,1 m²")
    );
}

#[test]
fn reimport_abgleich_ueber_global_id() {
    use super::testdaten::efh_ifc4_v2;
    let alt = modell(&efh_ifc4()).auszug;
    let neu = modell(&efh_ifc4_v2()).auszug;
    let gid = |k: &str| {
        alt.elemente
            .iter()
            .find(|e| e.klasse == k)
            .unwrap()
            .global_id
            .clone()
    };
    let (proxy, kamin) = (gid("IFCBUILDINGELEMENTPROXY"), gid("IFCCHIMNEY"));
    let wohnen = alt
        .elemente
        .iter()
        .find(|e| e.langname.as_deref() == Some("Wohnen"))
        .unwrap()
        .global_id
        .clone();
    let liste = vec![
        Modellaenderung::Klassifizieren {
            global_id: proxy.clone(),
            klasse: "IFCWALL".into(),
            vordefiniert: None,
        },
        Modellaenderung::Ausblenden {
            global_id: kamin.clone(),
        },
        Modellaenderung::Benennen {
            global_id: wohnen.clone(),
            name: "0.01".into(),
            langname: Some("Wohnzimmer".into()),
        },
    ];
    let v = abgleichen(&alt, &neu, &liste);
    assert!(v.neu.is_empty());
    assert_eq!(
        v.entfernt
            .iter()
            .map(|e| e.global_id.as_str())
            .collect::<Vec<_>>(),
        [kamin.as_str()]
    );
    assert_eq!(v.geaendert.len(), 2, "{:#?}", v.geaendert);
    let p = v.geaendert.iter().find(|g| g.global_id == proxy).unwrap();
    assert_eq!(p.aenderungen, ["Klasse Unklassifiziertes Bauteil → Wand"]);
    let w = v.geaendert.iter().find(|g| g.global_id == wohnen).unwrap();
    assert_eq!(w.aenderungen, ["Langname „Wohnen“ → „Wohnen/Essen“"]);
    assert_eq!(v.unveraendert, alt.elemente.len() - 3);
    assert_eq!(
        v.nacharbeit,
        [
            Nacharbeitsstand::Ueberholt,
            Nacharbeitsstand::Verwaist,
            Nacharbeitsstand::Wirksam
        ]
    );
    assert!(v.geschosse.is_empty());
    assert_eq!(v.hinweise.len(), 1);
}

#[test]
fn auswertung_grundflaeche_bgf_bri_und_hoehen() {
    let a = modell(&efh_ifc4()).auszug;
    let g = Bezugsflaeche {
        grundstueck_id: uuid::Uuid::nil(),
        bezeichnung: "Baugrundstück".into(),
        flaeche_m2: 400.,
    };
    let v = auswerten(&a, Some(g));
    let aussen = efh::LAENGE * efh::BREITE;
    assert!(nah(v.grundflaeche_m2, aussen), "{}", v.grundflaeche_m2);
    assert_eq!(v.geschosse.len(), 2);
    let (eg, og) = (&v.geschosse[0], &v.geschosse[1]);
    assert!(eg.oberirdisch && og.oberirdisch);
    assert!(nah(eg.bgf_m2, aussen) && nah(og.bgf_m2, aussen));
    // EG ab Unterkante Bodenplatte (−0,25 m) bis Ebene OG; OG bis Oberkante Dach.
    assert!(nah(eg.hoehe_m, efh::GESCHOSSHOEHE + 0.25), "{}", eg.hoehe_m);
    assert!(
        nah(og.hoehe_m, efh::FIRST - efh::GESCHOSSHOEHE),
        "{}",
        og.hoehe_m
    );
    assert!(nah(v.bri_m3, aussen * (efh::FIRST + 0.25)), "{}", v.bri_m3);
    assert!(nah(v.bgf_m2, 2. * aussen) && nah(v.bgf_oberirdisch_m2, 2. * aussen));
    // Netto-Raumfläche aus der Geometrie, die Mengen der Datei daneben.
    assert!(nah(v.nrf_m2, 4. * efh::raumflaeche()), "{}", v.nrf_m2);
    assert_eq!(eg.nrf_menge_m2, Some(2. * efh::RAUM_MENGE));
    assert_eq!(eg.raeume, 2);
    let n: Vec<&str> = v.nutzungen.iter().map(|n| n.nutzung.as_str()).collect();
    assert_eq!(n, ["Kind", "Küche", "Schlafen", "Wohnen"]);
    // Höhen über EG: Flachdach, Schornstein ragt 0,9 m darüber.
    assert!(v.hoehen.flachdach);
    assert_eq!(v.hoehen.first_m, Some(efh::FIRST));
    assert_eq!(v.hoehen.traufe_m, Some(efh::FIRST));
    assert!(nah(v.hoehen.oberkante_m.unwrap(), efh::FIRST + 0.9));
    assert_eq!(v.hoehen.bezug_nhn_m, Some(92.5));
    assert_eq!(v.grz, Some(0.2));
    assert_eq!(v.gfz, Some(0.4));

    // Ohne Baugrundstück keine Verhältniszahlen; ausgeblendete Räume fehlen.
    let mut b = a.clone();
    let raum = b
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCSPACE")
        .unwrap()
        .global_id
        .clone();
    nacharbeit::anwenden(&mut b, &[Modellaenderung::Ausblenden { global_id: raum }]);
    let w = auswerten(&b, None);
    assert_eq!((w.grz, w.gfz), (None, None));
    assert!(w.hinweise.iter().any(|h| h.contains("Kein Baugrundstück")));
    assert!(nah(w.nrf_m2, 3. * efh::raumflaeche()));
    assert!(nah(w.bgf_m2, v.bgf_m2));
}

#[test]
fn export_schreibt_nacharbeit_in_die_datei() {
    let m = modell(&efh_ifc4());
    let a = &m.auszug;
    let (eg, og) = (
        a.geschosse[0].global_id.clone(),
        a.geschosse[1].global_id.clone(),
    );
    let gid = |k: &str| {
        a.elemente
            .iter()
            .find(|e| e.klasse == k)
            .unwrap()
            .global_id
            .clone()
    };
    let wand = a
        .elemente
        .iter()
        .find(|e| e.name == "Außenwand Süd")
        .unwrap()
        .global_id
        .clone();
    let raeume: Vec<String> = a
        .elemente
        .iter()
        .filter(|e| e.klasse == "IFCSPACE")
        .map(|e| e.global_id.clone())
        .collect();
    let neu_raum = step::global_id(uuid::Uuid::from_u128(7));
    let liste = vec![
        Modellaenderung::Klassifizieren {
            global_id: gid("IFCBUILDINGELEMENTPROXY"),
            klasse: "IFCWALL".into(),
            vordefiniert: Some("PARAPET".into()),
        },
        // Geteilter Satz (vier Außenwände): Wand bekommt eine eigene Kopie.
        Modellaenderung::EigenschaftSetzen {
            global_id: wand.clone(),
            satz: "Pset_WallCommon".into(),
            name: "IsExternal".into(),
            wert: Eigenschaftswert::Logisch { wert: false },
        },
        Modellaenderung::EigenschaftSetzen {
            global_id: wand.clone(),
            satz: "Pset_WallCommon".into(),
            name: "FireRating".into(),
            wert: Eigenschaftswert::Text { wert: "F90".into() },
        },
        Modellaenderung::EigenschaftSetzen {
            global_id: wand.clone(),
            satz: "BauProjektIO".into(),
            name: "Dicke".into(),
            wert: Eigenschaftswert::Zahl { wert: 0.365 },
        },
        Modellaenderung::Benennen {
            global_id: raeume[0].clone(),
            name: "0.01".into(),
            langname: Some("Wohnen/Essen".into()),
        },
        Modellaenderung::Benennen {
            global_id: og.clone(),
            name: "1. OG".into(),
            langname: Some("Obergeschoss".into()),
        },
        Modellaenderung::GeschossZuordnen {
            global_id: gid("IFCCHIMNEY"),
            geschoss: og.clone(),
        },
        Modellaenderung::GeschossZuordnen {
            global_id: raeume[1].clone(),
            geschoss: og.clone(),
        },
        Modellaenderung::Ausblenden {
            global_id: gid("IFCROOF"),
        },
        Modellaenderung::Ausblenden {
            global_id: raeume[3].clone(),
        },
        Modellaenderung::RaumBilden {
            global_id: neu_raum.clone(),
            geschoss: eg.clone(),
            name: "0.03".into(),
            langname: Some("Abstellraum".into()),
            umriss: vec![[1., 1.], [3., 1.], [3., 2.5], [1., 2.5]],
            hoehe_m: Some(2.4),
        },
    ];
    nacharbeit::pruefen(a, &liste).unwrap();
    let mut soll = a.clone();
    nacharbeit::anwenden(&mut soll, &liste);
    let (text, b) = nacharbeit_schreiben(
        &m,
        &soll,
        &liste,
        "EFH-nachgearbeitet.ifc",
        "2026-10-28T12:00:00",
    )
    .unwrap();
    assert!(b.abweichungen.is_empty(), "{:#?}", b.abweichungen);
    assert_eq!((b.neu, b.entfernt), (1, 2));
    assert_eq!(b.elemente, soll.elemente.len());
    assert!(text.contains("FILE_NAME('EFH-nachgearbeitet.ifc'"));
    // Unberührte Instanzen bleiben zeichengleich.
    let erste = efh_ifc4()
        .lines()
        .find(|l| l.contains("=IFCSIUNIT("))
        .unwrap()
        .to_string();
    assert!(text.contains(&erste));

    let ist = modell(&text).auszug;
    let w = ist.element(&gid("IFCBUILDINGELEMENTPROXY")).unwrap();
    assert_eq!(
        (w.klasse.as_str(), w.vordefiniert.as_deref()),
        ("IFCWALL", Some("PARAPET"))
    );
    let s = ist.element(&wand).unwrap();
    let p = |satz: &str, n: &str| {
        s.eigenschaften
            .iter()
            .find(|p| p.satz == satz && p.name == n)
            .map(|p| p.wert.clone())
    };
    assert_eq!(p("Pset_WallCommon", "IsExternal").as_deref(), Some("Nein"));
    assert_eq!(p("Pset_WallCommon", "FireRating").as_deref(), Some("F90"));
    assert!(p("BauProjektIO", "Dicke").is_some());
    // Die anderen Außenwände behalten ihren Satz.
    let nord = ist
        .elemente
        .iter()
        .find(|e| e.name == "Außenwand Nord")
        .unwrap();
    assert!(
        nord.eigenschaften
            .iter()
            .any(|p| p.name == "IsExternal" && p.wert == "Ja")
    );
    assert_eq!(ist.geschosse[1].name, "1. OG");
    assert_eq!(
        ist.element(&gid("IFCCHIMNEY")).unwrap().geschoss.as_deref(),
        Some(og.as_str())
    );
    assert!(ist.element(&gid("IFCROOF")).is_none());
    let dachplatte = ist
        .elemente
        .iter()
        .find(|e| e.name == "Dachplatte")
        .unwrap();
    assert_eq!(dachplatte.geschoss.as_deref(), Some(og.as_str()));
    let r = ist.element(&neu_raum).unwrap();
    assert_eq!(r.langname.as_deref(), Some("Abstellraum"));
    assert!(nah(r.geometrie.as_ref().unwrap().grundflaeche_m2, 3.));
    assert!(nah(r.geometrie.as_ref().unwrap().oben_m, 2.4));
    assert!(
        r.mengen
            .iter()
            .any(|q| q.name == "NetFloorArea" && nah(q.wert, 3.))
    );
    // Die geschriebene Datei besteht die eingebauten Regeln wie das Soll.
    let pr = pruefen(&ist, BAUANTRAG_IDS).unwrap();
    assert_eq!(pr.fehler, 0, "{:#?}", pr.befunde);
}

#[test]
fn export_ifc2x3_ersetzt_fehlende_klasse() {
    let m = modell(&wand_ifc2x3());
    let w = m.auszug.elemente[0].global_id.clone();
    let liste = vec![Modellaenderung::Klassifizieren {
        global_id: w.clone(),
        klasse: "IFCCHIMNEY".into(),
        vordefiniert: None,
    }];
    let mut soll = m.auszug.clone();
    nacharbeit::anwenden(&mut soll, &liste);
    let (text, b) = nacharbeit_schreiben(&m, &soll, &liste, "w.ifc", "2026-10-28").unwrap();
    assert!(b.abweichungen.is_empty(), "{:?}", b.abweichungen);
    assert!(
        b.hinweise.iter().any(|h| h.contains("IFC2X3")),
        "{:?}",
        b.hinweise
    );
    let ist = modell(&text).auszug;
    assert_eq!(ist.schema, "IFC2X3");
    let e = ist.element(&w).unwrap();
    assert_eq!(e.klasse, "IFCBUILDINGELEMENTPROXY");
    assert_eq!(e.objekttyp.as_deref(), Some("Schornstein"));
}

#[test]
fn entwurf_als_ifc4_mit_stabilen_global_ids() {
    use uuid::Uuid;
    let rechteck = |x0: f64, y0: f64, x1: f64, y1: f64| crate::planaufmass::Planpolygon {
        aussen: vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        innen: vec![],
    };
    let wand = |n: u128, von: [f64; 2], bis: [f64; 2], aussen: bool| Entwurfswand {
        id: Uuid::from_u128(n),
        von,
        bis,
        dicke_m: if aussen { 0.3 } else { 0.1 },
        aussen,
    };
    let geschoss = |n: u128, name: &str, ebene: f64| Entwurfsgeschoss {
        id: Uuid::from_u128(n),
        name: name.into(),
        ebene_m: ebene,
        geschosshoehe_m: 2.8,
        deckendicke_m: 0.2,
        lichte_hoehe_m: Some(2.5),
        waende: vec![
            wand(n * 10 + 1, [0., 0.15], [10., 0.15], true),
            wand(n * 10 + 2, [0., 7.85], [10., 7.85], true),
            wand(n * 10 + 3, [0.15, 0.3], [0.15, 7.7], true),
            wand(n * 10 + 4, [9.85, 0.3], [9.85, 7.7], true),
            wand(n * 10 + 5, [5., 0.3], [5., 7.7], false),
        ],
        raeume: vec![
            Entwurfsraum {
                id: Uuid::from_u128(n * 10 + 6),
                name: format!("{n}.01"),
                langname: Some("Wohnen".into()),
                umriss: vec![rechteck(0.3, 0.3, 4.95, 7.7)],
            },
            Entwurfsraum {
                id: Uuid::from_u128(n * 10 + 7),
                name: format!("{n}.02"),
                langname: Some("Küche".into()),
                umriss: vec![rechteck(5.05, 0.3, 9.7, 7.7)],
            },
        ],
        aussenkante: vec![rechteck(0., 0., 10., 8.)],
    };
    let e = Entwurf {
        id: Uuid::from_u128(1),
        projekt: "EFH Mustermann".into(),
        gebaeude: "Wohnhaus".into(),
        nullpunkt_m_nhn: Some(92.5),
        geschosse: vec![geschoss(1, "EG", 0.), geschoss(2, "OG", 2.8)],
    };
    let text = entwurf_schreiben(&e, "Wohnhaus.ifc", "2026-10-28T12:00:00").unwrap();
    let (n, ab) = entwurf_kontrolle(&text, &e);
    assert!(ab.is_empty(), "{ab:#?}");
    assert_eq!(n, 2 * (5 + 1 + 2));
    assert_eq!(
        text,
        entwurf_schreiben(&e, "Wohnhaus.ifc", "2026-10-28T12:00:00").unwrap()
    );
    let a = modell(&text).auszug;
    assert_eq!(a.schema, "IFC4");
    assert_eq!(a.einheit, "Meter");
    // Auswertung und Prüfung arbeiten auf dem eigenen Modell wie auf fremden.
    let v = auswerten(&a, None);
    assert!(nah(v.grundflaeche_m2, 80.) && nah(v.bgf_m2, 160.), "{v:?}");
    assert_eq!(v.hoehen.first_m, Some(2. * 2.8));
    assert!(v.hoehen.flachdach);
    let r = a.elemente.iter().find(|x| x.name == "1.01").unwrap();
    assert!(nah(
        r.geometrie.as_ref().unwrap().grundflaeche_m2,
        4.65 * 7.4
    ));
    let aussen = a
        .elemente
        .iter()
        .filter(|x| {
            x.eigenschaften
                .iter()
                .any(|p| p.name == "IsExternal" && p.wert == "Ja")
        })
        .count();
    assert_eq!(aussen, 8);
    let pr = pruefen(&a, BAUANTRAG_IDS).unwrap();
    assert_eq!(pr.fehler, 0, "{:#?}", pr.befunde);
    assert!(
        entwurf_schreiben(
            &Entwurf {
                geschosse: vec![],
                ..e
            },
            "x.ifc",
            ""
        )
        .is_err()
    );
}

#[test]
fn geometrie_nacharbeiten_hoehe_umriss_und_deckendicke() {
    let m = modell(&efh_ifc4());
    let a = &m.auszug;
    let finde = |n: &str| {
        a.elemente
            .iter()
            .find(|e| e.name == n)
            .unwrap()
            .global_id
            .clone()
    };
    let (wand, decke) = (finde("Innenwand"), finde("Decke über EG"));
    let raum = a
        .elemente
        .iter()
        .find(|e| e.klasse == "IFCSPACE")
        .unwrap()
        .global_id
        .clone();
    let decke_oben = a
        .element(&decke)
        .unwrap()
        .geometrie
        .as_ref()
        .unwrap()
        .oben_m;
    let liste = vec![
        Modellaenderung::HoeheSetzen {
            global_id: wand.clone(),
            unten_m: None,
            oben_m: Some(2.0),
        },
        Modellaenderung::HoeheSetzen {
            global_id: decke.clone(),
            unten_m: Some(decke_oben - 0.3),
            oben_m: None,
        },
        Modellaenderung::UmrissSetzen {
            global_id: raum.clone(),
            umriss: vec![[1., 1.], [4., 1.], [4., 5.], [1., 5.]],
        },
    ];
    nacharbeit::pruefen(a, &liste).unwrap();
    let mut soll = a.clone();
    assert!(nacharbeit::anwenden(&mut soll, &liste).is_empty());
    let g = |x: &Modellauszug, gid: &str| x.element(gid).unwrap().geometrie.clone().unwrap();
    assert!(nah(g(&soll, &wand).oben_m, 2.0) && nah(g(&soll, &wand).unten_m, 0.));
    assert!(nah(
        g(&soll, &wand).grundflaeche_m2,
        g(a, &wand).grundflaeche_m2
    ));
    assert!(nah(g(&soll, &decke).oben_m - g(&soll, &decke).unten_m, 0.3));
    assert!(nah(g(&soll, &raum).grundflaeche_m2, 12.));
    assert!(nah(g(&soll, &raum).oben_m, efh::LICHTE_HOEHE));
    // Der Export schreibt Ersatzkörper; die Selbstkontrolle vergleicht Höhen und Flächen.
    let (text, b) = nacharbeit_schreiben(&m, &soll, &liste, "x.ifc", "2026-10-28").unwrap();
    assert!(b.abweichungen.is_empty(), "{:#?}", b.abweichungen);
    let ist = modell(&text).auszug;
    assert!(nah(g(&ist, &wand).oben_m, 2.0));
    assert!(nah(g(&ist, &decke).unten_m, decke_oben - 0.3));
    assert!(nah(g(&ist, &raum).grundflaeche_m2, 12.));
    // Abgleich: auf der geschriebenen Datei sind die Änderungen überholt.
    let v = abgleichen(a, &ist, &liste);
    assert_eq!(v.nacharbeit, [Nacharbeitsstand::Ueberholt; 3]);
    // Ungültiges wird abgewiesen.
    let falsch = |c: Modellaenderung| nacharbeit::pruefen(a, &[c]).is_err();
    assert!(falsch(Modellaenderung::HoeheSetzen {
        global_id: wand.clone(),
        unten_m: None,
        oben_m: None,
    }));
    assert!(falsch(Modellaenderung::HoeheSetzen {
        global_id: wand.clone(),
        unten_m: Some(3.),
        oben_m: Some(2.),
    }));
    assert!(falsch(Modellaenderung::UmrissSetzen {
        global_id: raum,
        umriss: vec![[0., 0.], [0.1, 0.], [0.1, 0.1]],
    }));
}
