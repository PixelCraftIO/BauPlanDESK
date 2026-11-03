//! Eigene Zeichnungen in Metern (ohne Unterlage) neben Aufmaßen auf PDF/DXF:
//! alte Speicherstände bleiben lesbar, Zeichnungen messen in Metern, Hinter-
//! gründe müssen auf vorhandene Dokumentversionen zeigen, Export liefert ein
//! leeres Blatt mit den Konturen.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    berechnungen::{BerechnungsArt, vorlagen},
    domain::{Bundesland, DocumentId, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planaufmass::*,
    plattform::{ObjektArt, objekte::Dokumentbezug},
};
use std::sync::Arc;
use uuid::Uuid;

fn ebene(id: Uuid) -> Planebene {
    Planebene {
        id,
        bezeichnung: "Räume".into(),
        sichtbar: true,
        gesperrt: false,
        darstellung: None,
    }
}
fn markierung(id: Uuid, ebene: Uuid, g: Plangeometrie) -> Planmarkierung {
    Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: "Wohnen".into(),
        bedeutung: Planbedeutung::Nuf,
        geometrie: g,
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        flurstueck_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
    }
}

#[tokio::test]
async fn eigene_zeichnung_in_metern_neben_altem_aufmass() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let project = core
        .create_project(
            NewProject {
                name: "Zeichnung".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let doc = core
        .store_document(
            project.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let e = Uuid::new_v4();

    // 1. Alter Speicherstand ohne die neuen Felder (so wie vor dieser Änderung
    //    gespeichert) wird gelesen, bleibt beim Wiederspeichern dateigleich und
    //    ist kein Zeichnungsobjekt.
    let alt_id = Uuid::new_v4();
    let alt = serde_json::json!({
        "bezeichnung": "EG",
        "dokument": {"dokument_id": doc.document_id, "version": 1},
        "seite": 1,
        "kalibrierung": {"art": "strecke", "von": [0., 0.], "bis": [100., 0.], "meter": 10.},
        "hoehenbezug": null,
        "ebenen": [ebene(e)],
        "markierungen": [markierung(Uuid::new_v4(), e, Plangeometrie::Rechteck { von: [0., 0.], bis: [100., 50.] })]
    });
    let f = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            alt_id,
            0,
            alt.clone(),
            &user,
        )
        .await
        .unwrap();
    let gelesen = core
        .fachobjekt_lesen(project.id, ObjektArt::Planaufmass, alt_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(gelesen.revision, f.revision);
    let a: Planaufmass = serde_json::from_value(gelesen.daten.clone()).unwrap();
    assert!(!a.ist_zeichnung());
    assert_eq!(serde_json::to_value(&a).unwrap(), alt);
    let werte = core.planaufmass_pruefen(project.id, &a).await.unwrap();
    assert!((werte[0].flaeche_m2.unwrap() - 50.).abs() < 1e-9);

    // 2. Eigene Zeichnung ohne Unterlage: Markierungen in Metern, Wand und
    //    Maßkette dazu, Bindung in eine DIN-277-Berechnung liefert Quadratmeter.
    let z_id = Uuid::new_v4();
    let m_id = Uuid::new_v4();
    let mut z = Planaufmass {
        bezeichnung: "Skizze EG".into(),
        dokument: None,
        seite: 1,
        hintergruende: vec![],
        elemente: vec![
            Zeichenelement {
                id: Uuid::new_v4(),
                ebene_id: e,
                darstellung: None,
                form: Zeichenform::Wand {
                    von: [0., 0.],
                    bis: [12.5, 0.],
                    dicke_m: 0.24,
                    oeffnungen: vec![],
                },
            },
            Zeichenelement {
                id: Uuid::new_v4(),
                ebene_id: e,
                darstellung: None,
                form: Zeichenform::Masskette {
                    von: [0., 0.],
                    bis: [12.5, 0.],
                    abstand: 1.,
                },
            },
            Zeichenelement {
                id: Uuid::new_v4(),
                ebene_id: e,
                darstellung: None,
                form: Zeichenform::Text {
                    punkt: [1., 1.],
                    inhalt: "Wohnen".into(),
                    hoehe: 0.25,
                    drehung_grad: 0.,
                },
            },
        ],
        kalibrierung: Some(Plankalibrierung::Einheit {
            einheit: core_lib::cad::Laengeneinheit::Meter,
        }),
        hoehenbezug: None,
        ebenen: vec![ebene(e)],
        markierungen: vec![markierung(
            m_id,
            e,
            Plangeometrie::Rechteck {
                von: [0., 0.],
                bis: [12.5, 8.],
            },
        )],
        ausschnitte: vec![],
    };
    let werte = core.planaufmass_pruefen(project.id, &z).await.unwrap();
    assert!((werte[0].flaeche_m2.unwrap() - 100.).abs() < 1e-9);
    let diagnose = core.planaufmass_diagnose(project.id, &z).await.unwrap();
    assert_eq!(diagnose.markierungen.len(), 1);
    let f = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            z_id,
            0,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let gespeichert = core
        .fachobjekt_lesen(project.id, ObjektArt::Planaufmass, z_id)
        .await
        .unwrap()
        .unwrap();
    assert!(gespeichert.daten.get("dokument").is_none());
    assert_eq!(gespeichert.daten["elemente"].as_array().unwrap().len(), 3);
    let mut plan = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Din277)
        .unwrap()
        .plan;
    plan.planbindungen.push(Planbindung {
        aufmass_id: z_id,
        revision: f.revision,
        markierung_id: m_id,
        groesse: Planmessgroesse::Flaeche,
        ziel: "positionen.raum1.aufmass_m2".into(),
    });
    plan.eingaben["positionen"][0]["aufmass_m2"] = serde_json::json!(999);
    let (aufgeloest, _) = core
        .berechnung_pruefen(project.id, plan.clone())
        .await
        .unwrap();
    assert_eq!(aufgeloest.eingaben["positionen"][0]["aufmass_m2"], 100.);
    assert!(
        aufgeloest.eingaben["positionen"][0]["quelle"]
            .as_str()
            .unwrap()
            .contains("Eigene Zeichnung"),
        "{}",
        aufgeloest.eingaben["positionen"][0]["quelle"]
    );
    let din_id = Uuid::new_v4();
    core.fachobjekt_speichern(
        project.id,
        ObjektArt::Din277,
        din_id,
        0,
        serde_json::to_value(&plan).unwrap(),
        &user,
    )
    .await
    .unwrap();

    // Export ohne Unterlage: leeres Blatt mit Konturen und CSV mit Metern.
    let mut w = Planexportwunsch {
        aufmass_id: z_id,
        revision: f.revision,
        format: Planexportformat::Pdf,
        nur_sichtbare_ebenen: true,
    };
    let pdf = core.planaufmass_export(project.id, &w).await.unwrap();
    let ausgabe = lopdf::Document::load_mem(&pdf).unwrap();
    assert!(!ausgabe.get_pages().is_empty());
    w.format = Planexportformat::Csv;
    let csv = String::from_utf8(core.planaufmass_export(project.id, &w).await.unwrap()).unwrap();
    assert!(csv.contains("100,000000"), "{csv}");

    // 3. Hintergrund auf vorhandener Dokumentversion ist erlaubt, auf fehlender
    //    oder fremder nicht; ein Aufmaß auf Unterlage darf keine Hintergründe tragen.
    let hintergrund = Planhintergrund {
        id: Uuid::new_v4(),
        dokument: Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        },
        seite: 1,
        ursprung: [0., 0.],
        lage: [0., 0.],
        drehung_grad: 0.,
        meter_pro_punkt: 0.01,
        deckkraft: 0.5,
        sichtbar: true,
        geschoss: None,
    };
    z.hintergruende.push(hintergrund.clone());
    let f = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            z_id,
            f.revision,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .unwrap();
    z.hintergruende[0].dokument.version = 7;
    assert!(
        core.fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            z_id,
            f.revision,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .is_err(),
        "fehlende Version"
    );
    z.hintergruende[0].dokument = Dokumentbezug {
        dokument_id: DocumentId(Uuid::new_v4()),
        version: 1,
    };
    assert!(
        core.fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            z_id,
            f.revision,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .is_err(),
        "fremdes Dokument"
    );
    z.hintergruende[0] = hintergrund.clone();
    z.hintergruende[0].seite = 3;
    assert!(
        core.fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            z_id,
            f.revision,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .is_err(),
        "Seite fehlt"
    );
    let mut auf_unterlage = a.clone();
    auf_unterlage.hintergruende.push(hintergrund);
    assert!(
        core.planaufmass_pruefen(project.id, &auf_unterlage)
            .await
            .is_err()
    );

    // Die Projektübersicht meldet die Zeichnung nicht als veraltete Unterlage.
    let uebersicht = core.projektuebersicht(project.id, &user).await.unwrap();
    let text = serde_json::to_string(&uebersicht).unwrap();
    assert!(
        !text.contains("Unterlage ist veraltet oder fehlt"),
        "{text}"
    );
}
