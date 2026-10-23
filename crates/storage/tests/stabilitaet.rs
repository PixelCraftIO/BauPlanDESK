//! Begrenzte Listen, Unicode-Suche und Druckabbruch mit echter Persistenz.
use core_lib::{
    CoreServices,
    commit::Fachstand,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
};
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn seiten_historien_suchindex_und_abbruch() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().into(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = c.anmelden("admin", "Planer").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Lastprobe".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let audit = || core_lib::domain::NeuerProtokolleintrag {
        project_id: Some(p.id),
        document_id: None,
        zeit: chrono::Utc::now(),
        benutzer_id: u.id,
        benutzer_name: u.anzeigename.clone(),
        aktion: core_lib::domain::Aktion::ProjektGeaendert,
        details: "Test".into(),
    };
    // Echte Keyset-Abfrage: kein Eintrag fehlt oder erscheint doppelt.
    for i in 0..215 {
        c.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p.id,
                    modul: "aufgaben".into(),
                    objekt: format!("{i:04}"),
                    schema_version: 1,
                    revision: 0,
                    daten: json!({"titel":i}),
                },
                0,
                audit(),
            )
            .await
            .unwrap();
    }
    let mut ids = vec![];
    let mut cursor = None;
    loop {
        let seite = c
            .repos
            .commit
            .fachseite(p.id, "aufgaben", cursor.as_deref(), 100)
            .await
            .unwrap();
        if seite.is_empty() {
            break;
        }
        assert!(seite.len() <= 100);
        cursor = seite.last().map(|f| f.objekt.clone());
        ids.extend(seite.into_iter().map(|f| f.objekt));
    }
    assert_eq!(ids.len(), 215);
    ids.dedup();
    assert_eq!(ids.len(), 215);
    for basis in 1..76 {
        c.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p.id,
                    modul: "aufgaben".into(),
                    objekt: "0000".into(),
                    schema_version: 1,
                    revision: basis,
                    daten: json!({"titel":basis}),
                },
                basis,
                audit(),
            )
            .await
            .unwrap();
    }
    let h = c
        .repos
        .commit
        .fachverlaufsseite(p.id, "aufgaben", "0000", None, 50)
        .await
        .unwrap();
    assert_eq!(h.len(), 50);
    assert_eq!(h[0].revision, 76);
    let h2 = c
        .repos
        .commit
        .fachverlaufsseite(p.id, "aufgaben", "0000", Some(27), 50)
        .await
        .unwrap();
    assert_eq!(h2.len(), 26);
    assert_eq!(h2[0].revision, 26);
    let d = c
        .store_document(
            p.id,
            None,
            None,
            "Beleg.txt",
            bytes::Bytes::from_static(b"Original"),
            &u,
        )
        .await
        .unwrap();
    let bezug = core_lib::plattform::objekte::Dokumentbezug {
        dokument_id: d.document_id,
        version: d.version,
    };
    c.suchtext_speichern(
        p.id,
        bezug.clone(),
        core_lib::verfahren::suche::Textquelle::Manuell,
        "Änderung Straße Grundstück Abstandsfläche 100%".into(),
        &u,
    )
    .await
    .unwrap();
    for q in [
        "änderung",
        "STRASSE",
        "straße",
        "stück",
        "abstands",
        "10",
        "100%",
    ] {
        let treffer = c.verfahren_suche(p.id, q).await.unwrap();
        assert_eq!(
            treffer.iter().any(|t| t.bereich == "Dokumentinhalt"),
            q != "STRASSE",
            "{q}"
        );
    }
    // Aktualisierung ersetzt den Index in derselben Fachtransaktion.
    c.suchtext_speichern(
        p.id,
        bezug,
        core_lib::verfahren::suche::Textquelle::Manuell,
        "Korrigierte Baubeschreibung".into(),
        &u,
    )
    .await
    .unwrap();
    assert!(c.verfahren_suche(p.id, "Straße").await.unwrap().is_empty());
    assert_eq!(
        c.verfahren_suche(p.id, "Baubeschreibung")
            .await
            .unwrap()
            .len(),
        1
    );
    let d2 = c
        .store_document(
            p.id,
            Some(d.document_id),
            Some(1),
            "Beleg neu.txt",
            bytes::Bytes::from_static(b"Neu"),
            &u,
        )
        .await
        .unwrap();
    assert_eq!(d2.version, 2);
    let docs = c.repos.documents.latest_for_project(p.id).await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].version, 2);
    assert_eq!(
        c.repos
            .documents
            .history_page(d.document_id, Some(2), 50)
            .await
            .unwrap()[0]
            .version,
        1
    );
    // Unnormalisierter Altbestand bleibt suchbar.
    c.repos.commit.fachstand(Fachstand{projekt_id:p.id,modul:"suchtexte".into(),objekt:"alt".into(),schema_version:1,revision:0,daten:json!({"dokument":{"dokument_id":d.document_id,"version":1},"sha256":d.sha256,"quelle":"manuell","text":"Altbestand über Straße","indexiert_am":chrono::Utc::now()})},0,audit()).await.unwrap();
    assert_eq!(c.verfahren_suche(p.id, "Straße").await.unwrap().len(), 1);
    let steuerung = core_lib::druckausgabe::Drucksteuerung::default();
    steuerung.abbrechen();
    let auftrag=serde_json::from_value(json!({"titel":"Test","absender":"","empfaenger":"","anschreiben":"","mit_anschreiben":false,"inhaltsverzeichnis":false,"projekt_version":p.version,"quellen":[{"art":"baugrundstueck"}]})).unwrap();
    assert!(
        c.druckausgabe_gesteuert(p.id, auftrag, &Default::default(), steuerung)
            .await
            .err()
            .expect("abgebrochen")
            .to_string()
            .contains("abgebrochen")
    );
    assert!(
        c.fachobjektseite(p.id, ObjektArt::Aufgaben, None, 100)
            .await
            .unwrap()
            .len()
            <= 100
    );
}
