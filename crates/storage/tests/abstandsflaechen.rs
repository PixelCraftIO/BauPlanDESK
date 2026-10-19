//! Durchgehender Projektablauf mit echten Regeln, Originalquellen und PDF.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    abstandsflaechen::{testdaten::*, *},
    domain::{Bundesland, NewProject},
    druckausgabe::{Druckauftrag, Druckquelle},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, nachweise::NachweisEingabe, objekte::Dokumentbezug},
};
use std::sync::Arc;
use uuid::Uuid;
#[tokio::test]
async fn projekt_nachweis_pdf_quellen_und_unveraendertes_archiv() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Abstandsflächen Musterprojekt".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let quelle = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let mut plan = plan();
    plan.quellen.push(Dokumentbezug {
        dokument_id: quelle.document_id,
        version: 1,
    });
    let id = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            id,
            0,
            serde_json::to_value(&plan).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let r = core.abstandsplan_pruefen(p.id, &plan).await.unwrap();
    assert!(r.konfliktfrei);
    let n = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            plan.bezeichnung.clone(),
            NachweisEingabe::AbstandProjekt {
                objekt: id,
                plan: Box::new(plan.clone()),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(n.vollstaendig);
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis
    );
    assert!(core.nachweise(p.id).await.unwrap()[0].aktuell);
    core.nachweis_freigeben(p.id, n.id, "Kontrollfall fachlich geprüft".into(), &user)
        .await
        .unwrap();
    let auftrag = Druckauftrag {
        gestaltung: None,
        aktenzeichen: String::new(),
        datum: None,
        titel: "Abstandsflächennachweis".into(),
        absender: "Architekturbüro".into(),
        empfaenger: "Bauaufsicht".into(),
        anschreiben: String::new(),
        mit_anschreiben: false,
        inhaltsverzeichnis: false,
        deckblatt: false,
        trennblaetter: false,
        aufgestellt_von: None,
        projekt_version: p.version,
        quellen: vec![Druckquelle::Nachweis { id: n.id }],
    };
    let pdf = core.druckausgabe(p.id, auftrag.clone()).await.unwrap();
    let d = lopdf::Document::load_mem(&pdf.pdf).unwrap();
    assert!(d.get_pages().len() >= 3);
    let last = *d.get_pages().last_key_value().unwrap().1;
    // Planblätter sind WinAnsi-kodiert (Umlaute).
    let stream = String::from_utf8_lossy(&d.get_page_content(last)).into_owned();
    assert!(stream.contains("1:100") || stream.contains("1:150"));
    assert!(stream.contains("W1"));
    let json = core
        .abstandsnachweis_export(p.id, n.id, "json")
        .await
        .unwrap()
        .1;
    let original: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(original["eingabe"]["plan"]["quellen"][0]["version"], 1);
    let dxf = core
        .abstandsnachweis_export(p.id, n.id, "dxf")
        .await
        .unwrap()
        .1;
    let cad = core_lib::cad::dxf::lesen(&dxf).unwrap();
    assert_eq!(cad.linien.iter().filter(|l| l.geschlossen).count(), 2);
    let dokument = core
        .druckpdf_ablegen(
            p.id,
            "Abstandsflächen.pdf",
            Bytes::copy_from_slice(&pdf.pdf),
            Uuid::new_v4(),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        core.load_version(dokument.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        pdf.pdf.as_slice()
    );
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(out.join("nachweis.pdf"), &pdf.pdf).unwrap();
        std::fs::write(out.join("plan.dxf"), &dxf).unwrap();
        std::fs::write(out.join("nachweis.json"), &json).unwrap();
        let mut geaendert = plan.clone();
        geaendert.waende[0].abschluss_a_m = Some(110.);
        geaendert.recht.geprueft = false;
        geaendert.umfang_geprueft = false;
        std::fs::write(out.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"plan":plan,"fachstand":f,"nachweis":n,"ergebnis":r,"quelle":quelle,"benutzer":user,"katalog":katalog().unwrap(),"druckseiten":pdf.seiten,"ergebnis_geaendert":berechnen(&geaendert).unwrap()})).unwrap()).unwrap();
    }
    // Eine reine Änderung der Nutzungswerte macht den unabhängigen Abstandsplan nicht veraltet.
    let mut recht = p.planungsrecht.clone();
    recht.festgesetzte_werte.grz = Some(0.5);
    core.update_project(
        core_lib::domain::ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
            planungsrecht: Some(recht),
        },
        &user,
    )
    .await
    .unwrap();
    assert!(core.nachweise(p.id).await.unwrap()[0].aktuell);
    // Änderungen im Entwurf heben Bestätigungen auf und machen den alten Stand
    // veraltet. Bereits erzeugte Dokumentbytes und Wiederholung bleiben unverändert.
    plan.waende[0].abschluss_a_m = Some(110.);
    let f2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            id,
            1,
            serde_json::to_value(&plan).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let neu: AbstandsPlan = serde_json::from_value(f2.daten).unwrap();
    assert!(!neu.recht.geprueft && !neu.umfang_geprueft);
    assert!(!core.nachweise(p.id).await.unwrap()[0].aktuell);
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis
    );
    assert_eq!(
        core.abstandsnachweis_export(p.id, n.id, "json")
            .await
            .unwrap()
            .1,
        json
    );
    assert!(matches!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            id,
            1,
            serde_json::to_value(&neu).unwrap(),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    // Die ursprüngliche Planfassung lässt sich wieder bestätigen; eine neue
    // Version einer verwendeten Quelle macht genau diesen Stand erneut veraltet.
    let NachweisEingabe::AbstandProjekt {
        plan: original_plan,
        ..
    } = &n.eingabe
    else {
        unreachable!()
    };
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Abstandsflaechen,
        id,
        2,
        serde_json::to_value(original_plan).unwrap(),
        &user,
    )
    .await
    .unwrap();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Abstandsflaechen,
        id,
        3,
        serde_json::to_value(original_plan).unwrap(),
        &user,
    )
    .await
    .unwrap();
    assert!(core.nachweise(p.id).await.unwrap()[0].aktuell);
    core.store_document(
        p.id,
        Some(quelle.document_id),
        Some(1),
        "Lageplan.pdf",
        Bytes::from(core_lib::formular::testdaten::musterformular()),
        &user,
    )
    .await
    .unwrap();
    assert!(!core.nachweise(p.id).await.unwrap()[0].aktuell);
    assert_eq!(
        core.abstandsnachweis_export(p.id, n.id, "json")
            .await
            .unwrap()
            .1,
        json
    );
    let cad_datei = core
        .store_document(
            p.id,
            None,
            None,
            "Abstandsplan.dxf",
            Bytes::copy_from_slice(&dxf),
            &user,
        )
        .await
        .unwrap();
    let konturen = core
        .abstandscad_konturen(
            p.id,
            Dokumentbezug {
                dokument_id: cad_datei.document_id,
                version: 1,
            },
            core_lib::cad::Laengeneinheit::Meter,
        )
        .await
        .unwrap();
    assert_eq!(konturen.len(), 2);
    assert!(
        konturen
            .iter()
            .any(|c| c.punkte.iter().any(|p| (p.x - 15.).abs() < 1e-6))
    );
    // Quelldokument eines anderen Projekts darf nicht eingeschleust werden.
    let fremd = core
        .create_project(
            NewProject {
                name: "Andere Akte".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(core.abstandsplan_pruefen(fremd.id, &neu).await.is_err());
    let reader = core
        .benutzer_anlegen(
            core_lib::benutzer::NeuerBenutzer {
                anmeldename: "leser".into(),
                anzeigename: "Leser".into(),
                rolle: core_lib::benutzer::Rolle::Lesen,
            },
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            id,
            2,
            serde_json::to_value(&neu).unwrap(),
            &reader
        )
        .await
        .is_err()
    );
}
fn hp(station_m: f64, hoehe_m: f64) -> Hoehenpunkt {
    Hoehenpunkt {
        station_m,
        hoehe_m,
        quelle: "Lageplan Vorabzug".into(),
    }
}
/// Umriss mit Eckhöhen, Höhenprofil mit Wandteilen, Nachweis, PDF mit
/// Wandabwicklung und Geländevorschlag aus einer DXF.
#[tokio::test]
async fn gebaeudeumriss_hoehenprofil_und_wandabwicklung() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Hanghaus NRW".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut plan = plan();
    plan.waende.clear();
    plan.umrisse = vec![AbstandsUmriss {
        id: Uuid::from_u128(70),
        bezeichnung: "Wohnhaus".into(),
        gebaeude_id: None,
        grundstueck_id: Some(plan.grundstuecke[0].id),
        ecken: [
            (5., 5., 100.),
            (15., 5., 100.6),
            (15., 12., 101.2),
            (5., 12., 100.4),
        ]
        .into_iter()
        .map(|(x, y, g)| UmrissEcke {
            x,
            y,
            gelaende_m: Some(g),
        })
        .collect(),
        oberkante_m: Some(107.),
        quelle: "Lageplan Vorabzug, Höhen m ü. NHN".into(),
    }];
    let abgleich = umrisse_abgleichen(plan.clone()).unwrap();
    assert_eq!(abgleich.plan.waende.len(), 4);
    let mut plan = abgleich.plan.clone();
    for w in &mut plan.waende {
        w.dachart = Dachart::Flach;
    }
    // Südseite: Geländepunkt in der Wand und Höhensprung der Oberkante bei 6 m.
    plan.waende[0].oberkante = vec![hp(0., 107.), hp(6., 107.), hp(6., 104.), hp(10., 104.)];
    plan.waende[0].gelaende.push(hp(5., 100.1));
    let id = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            id,
            0,
            serde_json::to_value(&plan).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let plan: AbstandsPlan = serde_json::from_value(f.daten.clone()).unwrap();
    let r = core.abstandsplan_pruefen(p.id, &plan).await.unwrap();
    assert!(
        r.konfliktfrei,
        "{:?} {:?}",
        r.offen,
        r.waende.iter().map(|w| &w.offen).collect::<Vec<_>>()
    );
    let sued = &r.waende[0];
    assert_eq!(sued.ansatz, Some(Wandhoehenansatz::MittelKanten));
    assert_eq!(sued.wandteile.len(), 2);
    // Teil 1: (107 − 100) und (107 − 100,2) gemittelt = 6,9 m.
    assert!((sued.wandteile[0].wandhoehe_m - 6.9).abs() < 1e-9);
    assert!((sued.wandteile[1].wandhoehe_m - 3.6).abs() < 1e-9);
    let n = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            plan.bezeichnung.clone(),
            NachweisEingabe::AbstandProjekt {
                objekt: id,
                plan: Box::new(plan.clone()),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(n.vollstaendig);
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis
    );
    let pdf = core
        .druckausgabe(
            p.id,
            Druckauftrag {
                gestaltung: None,
                aktenzeichen: String::new(),
                datum: None,
                titel: "Abstandsflächennachweis".into(),
                absender: "Architekturbüro".into(),
                empfaenger: "Bauaufsicht".into(),
                anschreiben: String::new(),
                mit_anschreiben: false,
                inhaltsverzeichnis: false,
                deckblatt: false,
                trennblaetter: false,
                aufgestellt_von: None,
                projekt_version: p.version,
                quellen: vec![Druckquelle::Nachweis { id: n.id }],
            },
        )
        .await
        .unwrap();
    let d = lopdf::Document::load_mem(&pdf.pdf).unwrap();
    let seiten: Vec<String> = d
        .get_pages()
        .values()
        .map(|s| String::from_utf8_lossy(&d.get_page_content(*s)).into_owned())
        .collect();
    let m = seiten.len();
    // Zwei Abwicklungsblätter (4 Wände, 3 je Blatt) vor der Planskizze.
    let abw: Vec<_> = seiten
        .iter()
        .enumerate()
        .filter(|(_, s)| s.contains("Wandabwicklungen"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(abw, vec![m - 3, m - 2]);
    assert!(seiten[m - 3].contains(&format!("Blatt {} von {m}", m - 2)));
    assert!(
        seiten[m - 1].contains("Vektorplanskizze")
            && seiten[m - 1].contains(&format!("Blatt {m} von {m}"))
    );
    // Geländevorschlag aus einer DXF mit Höhenpunkt und Höhenkote.
    let dxf = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nPOINT\n8\nHOEHEN\n10\n5.0\n20\n4.5\n30\n100.05\n0\nTEXT\n8\nKOTEN\n10\n10.0\n20\n4.7\n40\n0.3\n1\n100,45\n0\nENDSEC\n0\nEOF\n";
    let datei = core
        .store_document(p.id, None, None, "Hoehen.dxf", Bytes::from(dxf), &user)
        .await
        .unwrap();
    let hoehen = core
        .abstandscad_hoehen(
            p.id,
            Dokumentbezug {
                dokument_id: datei.document_id,
                version: 1,
            },
            core_lib::cad::Laengeneinheit::Meter,
        )
        .await
        .unwrap();
    assert_eq!(hoehen.len(), 2, "{hoehen:?}");
    let v = gelaende_vorschlag(&plan.waende[0], &hoehen, 1.).unwrap();
    assert_eq!(
        v.iter()
            .map(|h| (h.station_m, h.hoehe_m))
            .collect::<Vec<_>>(),
        vec![(0., 100.05), (5., 100.45)]
    );
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(out.join("profil.pdf"), &pdf.pdf).unwrap();
        std::fs::write(
            out.join("umriss.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "umriss_abgleich": abgleich,
                "ergebnis": r,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
