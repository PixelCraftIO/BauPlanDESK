use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planaufmass::*,
    plattform::{ObjektArt, objekte::Dokumentbezug},
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn veraltete_planquelle_blockiert_nur_ihre_grundstueckseinheit() {
    use core_lib::{
        domain::{BebauungsplanArt, Planungsrecht},
        flaechen::BauNvo,
        plattform::objekte::Gebaeude,
    };
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("DMS"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("einheit", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Getrennte Einheiten".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    bau_nvo: Some(BauNvo::V2017),
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(core_lib::baugrundstuecke::musterplan()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let plan = core_lib::grundstuecksinformationen::plan(&p.planungsrecht)
        .unwrap()
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    assert!(grund.len() >= 2);
    let haus = Uuid::new_v4();
    let gd = serde_json::json!({"bezeichnung":"Haus auf Einheit B","grundstueck_ids":[],"flurstueck_ids":grund[1].flurstueck_ids,"quelle":null,"geometrie":{"grundflaeche_m2":100,"typ":"hauptanlage"},"geschosse":[]});
    let gf = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, 0, gd, &user)
        .await
        .unwrap();
    let d = core
        .store_document(p.id, None, None, "Haus B.pdf", Bytes::from(pdf()), &user)
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let mid = Uuid::new_v4();
    let aid = Uuid::new_v4();
    let mut a = Planaufmass {
        bezeichnung: "Haus B".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: d.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Haus".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        ausschnitte: vec![],
        markierungen: vec![Planmarkierung {
            id: mid,
            ebene_id: ebene,
            bezeichnung: "Grundfläche".into(),
            bedeutung: Planbedeutung::Gebaeudegrundflaeche,
            geometrie: Plangeometrie::Rechteck {
                von: [0., 0.],
                bis: [100., 100.],
            },
            grundstueck_id: Some(grund[1].id),
            gebaeude_id: Some(haus),
            geschoss_id: None,
            flurstueck_id: None,
            ausschnitt_id: None,
            hoehenzone_von: None,
            kontur_von: None,
        }],
    };
    let af = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aid,
            0,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let mut g: Gebaeude = serde_json::from_value(gf.daten).unwrap();
    g.planbindungen.push(Planbindung {
        aufmass_id: aid,
        revision: af.revision,
        markierung_id: mid,
        groesse: Planmessgroesse::Flaeche,
        ziel: "geometrie.grundflaeche_m2".into(),
    });
    let g = core.planaufmass_gebaeude(p.id, haus, g).await.unwrap();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        haus,
        gf.revision,
        serde_json::to_value(g).unwrap(),
        &user,
    )
    .await
    .unwrap();
    a.markierungen[0].geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [120., 100.],
    };
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planaufmass,
        aid,
        af.revision,
        serde_json::to_value(a).unwrap(),
        &user,
    )
    .await
    .unwrap();
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &plan)
            .await
            .is_err()
    );
    let r = core
        .baugrundstueck_einzeln_pruefen(p.id, &plan, &grund[0].schluessel)
        .await
        .unwrap();
    assert_eq!(r.baugrundstuecke.len(), 1);
    assert_eq!(
        r.baugrundstuecke[0].grundstueck.schluessel,
        grund[0].schluessel
    );
    assert!(
        core.baugrundstueck_einzeln_pruefen(p.id, &plan, &grund[1].schluessel)
            .await
            .is_err()
    );
    let nachbar = plan
        .berechnungen
        .iter()
        .find(|b| b.schluessel == grund[1].schluessel)
        .unwrap()
        .clone();
    let mut ziel = plan
        .berechnungen
        .iter()
        .find(|b| b.schluessel == grund[0].schluessel)
        .unwrap()
        .clone();
    ziel.eingabe.aufstellung.as_mut().unwrap().bauteile[0].flaeche_m2 = 123.;
    let neu = core
        .baugrundstueck_einzeln_speichern(p.id, p.version, ziel.clone(), &user)
        .await
        .unwrap();
    let nachher = core_lib::grundstuecksinformationen::plan(&neu.planungsrecht)
        .unwrap()
        .unwrap();
    assert_eq!(
        nachher
            .berechnungen
            .iter()
            .find(|b| b.schluessel == grund[1].schluessel)
            .unwrap(),
        &nachbar
    );
    assert_eq!(
        nachher
            .berechnungen
            .iter()
            .find(|b| b.schluessel == grund[0].schluessel)
            .unwrap()
            .eingabe
            .aufstellung
            .as_ref()
            .unwrap()
            .bauteile[0]
            .flaeche_m2,
        123.
    );
    assert!(matches!(
        core.baugrundstueck_einzeln_speichern(p.id, p.version, ziel, &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &nachher)
            .await
            .is_err()
    );
}

fn pdf() -> Vec<u8> {
    use lopdf::{Document, Stream, dictionary};
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let c = d.add_object(Stream::new(
        dictionary! {},
        b"0.8 0.85 0.9 RG 0.5 w 70 70 300 200 re S 220 70 m 220 270 l S".to_vec(),
    ));
    let p=d.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),600.into(),400.into()],"Resources"=>dictionary!{},"Contents"=>c});
    d.objects.insert(
        root,
        dictionary! {"Type"=>"Pages","Kids"=>vec![p.into()],"Count"=>1}.into(),
    );
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    d.trailer.set("Root", cat);
    let mut bytes = Vec::new();
    d.save_to(&mut bytes).unwrap();
    bytes
}
#[tokio::test]
async fn einzelpruefung_export_und_cad_aufmasse_erhalten_ihre_quellen() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("DMS"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Planbearbeitung".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let original = pdf();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Grundriss.pdf",
            Bytes::from(original.clone()),
            &user,
        )
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let mut a = Planaufmass {
        bezeichnung: "Grundriss EG".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: d.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [70., 300.],
            bis: [370., 300.],
            meter: 30.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Räume".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        ausschnitte: vec![],
        markierungen: vec![Planmarkierung {
            id: Uuid::new_v4(),
            ebene_id: ebene,
            bezeichnung: "Büro EG".into(),
            bedeutung: Planbedeutung::Raum,
            geometrie: Plangeometrie::Rechteck {
                von: [70., 100.],
                bis: [300., 280.],
            },
            grundstueck_id: None,
            gebaeude_id: None,
            geschoss_id: None,
            flurstueck_id: None,
            ausschnitt_id: None,
            hoehenzone_von: None,
            kontur_von: None,
        }],
    };
    let mut schlecht = a.markierungen[0].clone();
    schlecht.id = Uuid::new_v4();
    schlecht.bezeichnung = "Fehlerhafte Kontur".into();
    schlecht.geometrie = Plangeometrie::Polygon {
        punkte: vec![[0., 0.], [100., 100.], [0., 100.], [100., 0.]],
    };
    a.markierungen.push(schlecht);
    let diagnose = core.planaufmass_diagnose(p.id, &a).await.unwrap();
    assert!(diagnose.markierungen[0].wert.is_some());
    assert!(diagnose.markierungen[1].fehler.is_some());
    assert!(core.planaufmass_pruefen(p.id, &a).await.is_err());
    a.markierungen.pop();
    let id = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            id,
            0,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let mut w = Planexportwunsch {
        aufmass_id: id,
        revision: f.revision,
        format: Planexportformat::Pdf,
        nur_sichtbare_ebenen: true,
    };
    let export = core.planaufmass_export(p.id, &w).await.unwrap();
    let output = lopdf::Document::load_mem(&export).unwrap();
    assert!(output.get_pages().len() >= 2);
    let page = *output.get_pages().values().next().unwrap();
    let content = lopdf::content::Content::decode(&output.get_page_content(page)).unwrap();
    assert!(
        content
            .operations
            .iter()
            .any(|o| o.operator == "Do" && o.operands[0].as_name().ok() == Some(b"BauAufmass0"))
    );
    assert!(
        content.operations.iter().any(|o| o.operator == "m"
            && o.operands.first().and_then(|o| o.as_float().ok()) == Some(70.))
    );
    assert_eq!(
        core.load_version(d.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        original
    );
    if let Ok(path) = std::env::var("BAUPROJEKT_PLANANLAGE_PDF") {
        std::fs::write(path, &export).unwrap();
    }
    w.revision = 0;
    assert!(matches!(
        core.planaufmass_export(p.id, &w).await,
        Err(CoreError::VersionConflict { .. })
    ));
    w.revision = f.revision;
    a.markierungen[0].bezeichnung = "=SUM(1)".into();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            id,
            f.revision,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    w.revision = f.revision;
    w.format = Planexportformat::Csv;
    let csv = String::from_utf8(core.planaufmass_export(p.id, &w).await.unwrap()).unwrap();
    assert!(csv.contains("'=SUM(1)"));
    assert!(csv.contains("414,000000"));
    use core_lib::cad::{
        Laengeneinheit,
        projektaufmass::{CadMarkierung, CadProjektaufmass},
    };
    let cd = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.dxf",
            Bytes::from(core_lib::cad::testdaten::musterplan()),
            &user,
        )
        .await
        .unwrap();
    let cad = CadProjektaufmass {
        dokument: Dokumentbezug {
            dokument_id: cd.document_id,
            version: 1,
        },
        einheit: Laengeneinheit::Meter,
        markierungen: vec![CadMarkierung {
            id: Uuid::new_v4(),
            bezeichnung: "Umgriff".into(),
            punkte: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            geschlossen: true,
        }],
    };
    let f = core
        .cadaufmass_speichern(p.id, cad.clone(), 0, &user)
        .await
        .unwrap();
    assert_eq!(
        core.cadaufmass_laden(p.id, &cad.dokument)
            .await
            .unwrap()
            .unwrap()
            .revision,
        f.revision
    );
    assert_eq!(
        core.cadaufmass_speichern(p.id, cad.clone(), 0, &user)
            .await
            .unwrap()
            .revision,
        f.revision
    );
    let mut konkurrierend = cad.clone();
    let csv = String::from_utf8(
        core.cadaufmass_csv(p.id, &cad.dokument, f.revision)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(csv.contains("Umgriff"));
    assert!(csv.contains("100,000000"));
    assert!(matches!(
        core.cadaufmass_csv(p.id, &cad.dokument, 0).await,
        Err(CoreError::VersionConflict { .. })
    ));
    konkurrierend.markierungen[0].bezeichnung = "Anderer Entwurf".into();
    assert!(matches!(
        core.cadaufmass_speichern(p.id, konkurrierend, 0, &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    let cd2 = core
        .store_document(
            p.id,
            Some(cd.document_id),
            Some(1),
            "Lageplan.dxf",
            Bytes::from_static(b"neue Unterlage"),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.cadaufmass_laden(
            p.id,
            &Dokumentbezug {
                dokument_id: cd2.document_id,
                version: 2
            }
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        core.cadaufmass_laden(p.id, &cad.dokument)
            .await
            .unwrap()
            .is_some()
    );
}
