//! Einreichungsprüfung und Fachplaner-Lieferlisten: Ampel aus Unterlagen,
//! Planliste, Formularen und Lieferungen; Liefertermine im Cockpit.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    einreichungspruefung::{Ampel, Pruefbereich},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn einreichungspruefung_mit_ampel_und_lieferterminen() {
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
    let heute = core_lib::kalender::heute();
    let tag = |n: i64| heute + chrono::Duration::days(n);

    // Leeres Projekt: Planliste leer, keine Formulare, Anforderungen fehlen.
    let e = core.einreichungspruefung(p.id, None, &u).await.unwrap();
    assert_eq!(e.gesamt, Ampel::Rot);
    let bereich = |e: &core_lib::einreichungspruefung::Einreichungspruefung, b| {
        e.bereiche.iter().find(|x| x.bereich == b).unwrap().clone()
    };
    assert_eq!(bereich(&e, Pruefbereich::Plaene).ampel, Ampel::Gelb);
    assert_eq!(bereich(&e, Pruefbereich::Formulare).ampel, Ampel::Gelb);
    assert_eq!(bereich(&e, Pruefbereich::Fachplaner).ampel, Ampel::Gruen);

    // Plan ohne Index ist rot.
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Grundriss EG.pdf",
            Bytes::from_static(b"%PDF-1.4"),
            &u,
        )
        .await
        .unwrap();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planliste,
        plan.document_id.0,
        0,
        serde_json::json!({
            "dokument_id": plan.document_id, "planart": "grundriss", "geschoss": "EG",
            "nummer": 1, "titel": "Grundriss EG", "massstab": 100,
            "revisionen": [], "verteiler": []
        }),
        &u,
    )
    .await
    .unwrap();
    let e = core.einreichungspruefung(p.id, None, &u).await.unwrap();
    let pl = e
        .punkte
        .iter()
        .find(|x| x.bereich == Pruefbereich::Plaene)
        .unwrap();
    assert_eq!(pl.ampel, Ampel::Rot);
    assert!(pl.hinweis.contains("Kein Planindex"), "{}", pl.hinweis);

    // Lieferliste Statik: überfällig → rot, im Cockpit und in der Übersicht.
    let lid = Uuid::new_v4();
    let lieferung = |faellig: chrono::NaiveDate, status: &str| {
        serde_json::json!({
            "disziplin": "tragwerksplanung", "fachplaner": "Statikbüro Schmidt",
            "positionen": [{"bezeichnung": "Standsicherheitsnachweis", "geliefert": false}],
            "angefragt_am": tag(-20), "faellig": faellig, "status": status,
            "verantwortlich": u.id.0, "dokumente": [], "verfahren": null, "bemerkung": ""
        })
    };
    // Liefertermin vor der Anfrage wird abgewiesen.
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Fachplanerlieferung,
            lid,
            0,
            lieferung(tag(-30), "angefragt"),
            &u
        )
        .await
        .is_err()
    );
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Fachplanerlieferung,
            lid,
            0,
            lieferung(tag(-2), "angefragt"),
            &u,
        )
        .await
        .unwrap();
    let e = core.einreichungspruefung(p.id, None, &u).await.unwrap();
    let fp = bereich(&e, Pruefbereich::Fachplaner);
    assert_eq!((fp.ampel, fp.rot), (Ampel::Rot, 1));
    let punkt = e
        .punkte
        .iter()
        .find(|x| x.bereich == Pruefbereich::Fachplaner)
        .unwrap();
    assert!(punkt.hinweis.contains("Standsicherheitsnachweis"));
    assert_eq!(punkt.ziel.bereich, "fachplaner");
    assert_eq!(punkt.ziel.objekt, Some(lid));

    let c = core.buero_cockpit(&u, heute).await.unwrap();
    let t = c
        .termine
        .iter()
        .find(|t| t.bezug == "Fachplaner")
        .expect("Liefertermin im Cockpit");
    assert_eq!(t.titel, "Tragwerksplanung / Statik: Statikbüro Schmidt");
    assert_eq!(t.stufe, core_lib::cockpit::Friststufe::Ueberfaellig);
    let ue = core.projektuebersicht(p.id, &u).await.unwrap();
    assert!(
        ue.punkte
            .iter()
            .any(|x| x.id == format!("fachplaner:{lid}"))
    );

    // Geprüft: verschwindet aus Prüfung und Cockpit.
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Fachplanerlieferung,
        lid,
        f.revision,
        lieferung(tag(-2), "geprueft"),
        &u,
    )
    .await
    .unwrap();
    let e = core.einreichungspruefung(p.id, None, &u).await.unwrap();
    assert_eq!(bereich(&e, Pruefbereich::Fachplaner).ampel, Ampel::Gruen);
    let c = core.buero_cockpit(&u, heute).await.unwrap();
    assert!(!c.termine.iter().any(|t| t.bezug == "Fachplaner"));

    // Fremde Verfahrenskennung wird abgewiesen.
    assert!(
        core.einreichungspruefung(p.id, Some(Uuid::new_v4()), &u)
            .await
            .is_err()
    );
}
