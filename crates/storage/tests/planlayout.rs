//! Planlayout: eigene Zeichnung maßstäblich mit Plankopf aus Planliste, Ablage
//! als neue Planversion und Plankopf-Stempel auf vorhandenen PDF-Plänen.
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planlayout::{Blattformat, Layoutwunsch, Stempelwunsch},
    plattform::ObjektArt,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn planlayout_mit_plankopf_ablage_und_stempel() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Planung").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "EFH Muster".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let zeichnung = serde_json::json!({
        "bezeichnung": "Grundriss EG", "kalibrierung": {"art": "einheit", "einheit": "meter"},
        "seite": 1, "hoehenbezug": null,
        "ebenen": [{"id": ebene, "bezeichnung": "Wände", "sichtbar": true, "gesperrt": false,
            "darstellung": {"stift_mm": 0.5, "schraffur": "mauerwerk"}}],
        "markierungen": [],
        "elemente": [
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "wand", "von": [10.0, 10.0], "bis": [22.0, 10.0], "dicke_m": 0.365,
             "oeffnungen": [{"art": "fenster", "lage_m": 2.0, "breite_m": 1.51}, {"art": "tuer", "lage_m": 6.0, "breite_m": 1.01, "links": true}]},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "symbol", "punkt": [30.0, 12.0], "symbol": "baum_bestand", "groesse": 5.0, "drehung_grad": 0.0},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "symbol", "punkt": [30.0, 20.0], "symbol": "stellplatz", "groesse": 5.0, "drehung_grad": 90.0},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "raumstempel", "punkt": [12.0, 17.0], "raumbuch_id": Uuid::new_v4(), "raum_id": Uuid::new_v4(), "hoehe": 0.3},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "wand", "von": [22.0, 10.0], "bis": [22.0, 20.0], "dicke_m": 0.365,
             "darstellung": {"farbe": "#aa3300", "linienart": "strichlinie"}},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "masskette", "von": [10.0, 10.0], "bis": [22.0, 10.0], "abstand": -1.0},
            {"id": Uuid::new_v4(), "ebene_id": ebene, "art": "text", "punkt": [12.0, 14.0], "inhalt": "Wohnen", "hoehe": 0.35, "drehung_grad": 0.0}
        ]
    });
    let aid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, aid, 0, zeichnung, &u)
        .await
        .unwrap();
    let mut w = Layoutwunsch {
        aufmass_id: aid,
        revision: f.revision,
        format: Blattformat::A3,
        hochformat: false,
        massstab: 100,
        nordwinkel_grad: Some(0.),
        planinhalt: String::new(),
        dokument_id: None,
        legende: true,
        nur_sichtbare_ebenen: true,
        ablegen: true,
    };
    let (pdf, abgelegt) = core.planlayout(p.id, &w, &u).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    // Sichtprüfung: PLANLAYOUT_AUSGABE=/pfad/blatt.pdf cargo test --test planlayout
    if let Ok(ziel) = std::env::var("PLANLAYOUT_AUSGABE") {
        std::fs::write(ziel, &pdf).unwrap();
    }
    let doc = abgelegt.expect("abgelegt");
    assert_eq!(doc.file_name, "Grundriss EG.pdf");

    // Zu kleiner Maßstab für A4: verständlicher Hinweis.
    let mut eng = w.clone();
    eng.format = Blattformat::A4;
    eng.massstab = 20;
    eng.ablegen = false;
    let e = core
        .planlayout(p.id, &eng, &u)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("passt bei 1 : 20 nicht"), "{e}");

    // In die Planliste aufnehmen: Plankopf trägt Nummer und Index.
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planliste,
        doc.document_id.0,
        0,
        serde_json::json!({
            "dokument_id": doc.document_id, "planart": "grundriss", "geschoss": "EG",
            "nummer": 1, "titel": "Grundriss Erdgeschoss", "massstab": 100,
            "revisionen": [{"index": "a", "datum": "2026-11-01", "aenderung": "Erstausgabe", "version": doc.version}],
            "verteiler": []
        }),
        &u,
    )
    .await
    .unwrap();
    let kopf = core
        .plankopf(p.id, Some(doc.document_id), None, "", None, "A3")
        .await
        .unwrap();
    assert_eq!(kopf.plannummer, "EFH Muster-GR-EG-01");
    assert_eq!(kopf.index, "a");
    assert_eq!(kopf.planinhalt, "Grundriss Erdgeschoss");
    assert_eq!(kopf.vorhaben, "EFH Muster");
    assert_eq!(kopf.massstab, "1 : 100");
    assert_eq!(kopf.datum, "01.11.2026");

    // Neu ausgeben als nächste Version desselben Plans.
    w.dokument_id = Some(doc.document_id);
    let (_, v2) = core.planlayout(p.id, &w, &u).await.unwrap();
    let v2 = v2.unwrap();
    assert_eq!(
        (v2.document_id, v2.version),
        (doc.document_id, doc.version + 1)
    );
    let liste = core.planliste(p.id, &u).await.unwrap();
    assert!(
        liste.zeilen[0].ohne_index,
        "neue Version ohne Index wird gemeldet"
    );

    // Stempel auf Version 1; Index der Version bleibt „a“.
    let gestempelt = core
        .plankopf_stempeln(
            p.id,
            &Stempelwunsch {
                dokument_id: doc.document_id,
                version: doc.version,
                seite: 1,
            },
            &u,
        )
        .await
        .unwrap();
    assert!(gestempelt.len() > 1000);
    // PDF-Unterlage statt Zeichnung: Layout verweist auf den Stempel.
    let auf_pdf = serde_json::json!({
        "bezeichnung": "Aufmaß", "dokument": {"dokument_id": doc.document_id, "version": 1},
        "seite": 1, "kalibrierung": null, "hoehenbezug": null,
        "ebenen": [{"id": ebene, "bezeichnung": "Aufmaß", "sichtbar": true, "gesperrt": false}],
        "markierungen": []
    });
    let bid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, bid, 0, auf_pdf, &u)
        .await
        .unwrap();
    let e = core
        .planlayout(
            p.id,
            &Layoutwunsch {
                aufmass_id: bid,
                revision: f.revision,
                ablegen: false,
                dokument_id: None,
                ..w
            },
            &u,
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Plankopf stempeln"), "{e}");
}
