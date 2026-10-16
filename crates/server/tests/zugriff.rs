use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use core_lib::{
    CoreServices,
    benutzer::BenutzerAenderung,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::ContentHash,
    zugriff::{ProjektMitglied, ProjektRecht},
};
use server::api;
use std::sync::Arc;
use tower::ServiceExt;
const TOKEN: &str = "testzugang-nur-fuer-diese-isolierte-pruefung-123456789";
async fn status(app: &Router, path: &str, token: bool) -> StatusCode {
    let mut r = Request::builder().uri(path);
    if token {
        r = r.header("authorization", format!("Bearer {TOKEN}"));
    }
    app.clone()
        .oneshot(r.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}
#[tokio::test]
async fn anmeldung_projektgrenzen_und_widerruf_werden_pro_anfrage_geprueft() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let leser = core.anmelden("leser", "Leser").await.unwrap();
    let make = || NewProject {
        name: "Privates Projekt".into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let p = core.create_project(make(), &admin).await.unwrap();
    let fremd = core.create_project(make(), &admin).await.unwrap();
    core.projektmitglied_setzen(
        p.id,
        ProjektMitglied {
            benutzer_id: leser.id,
            recht: ProjektRecht::Lesen,
        },
        0,
        &admin,
    )
    .await
    .unwrap();
    let zugang = api::zugang::Zugaenge {
        schema_version: 1,
        zugaenge: vec![api::zugang::Zugang {
            benutzer_id: leser.id,
            token_sha256: ContentHash::of(TOKEN.as_bytes()),
            gueltig_bis: None,
        }],
    };
    let path = dir.path().join("zugaenge.json");
    std::fs::write(&path, serde_json::to_vec(&zugang).unwrap()).unwrap();
    let app = api::zugang::gesichert(core.clone(), api::zugang::ZugangQuelle::Datei(path.clone()));
    assert_eq!(status(&app, "/health", false).await, StatusCode::OK);
    assert_eq!(
        status(&app, "/api/projekte", false).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status(&app, &format!("/api/projekte/{}", p.id), true).await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, &format!("/api/projekte/{}", fremd.id), true).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status(&app, "/api/berechnungsvorlagen", true).await,
        StatusCode::OK
    );
    for vorlage in core_lib::berechnungen::vorlagen().unwrap() {
        for (projekt, expected) in [(p.id, StatusCode::OK), (fremd.id, StatusCode::FORBIDDEN)] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/api/projekte/{projekt}/fachberechnung"))
                        .header("authorization", format!("Bearer {TOKEN}"))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&vorlage.plan).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{:?}", vorlage.plan.modul);
        }
    }
    for (projekt, expected) in [(p.id, StatusCode::OK), (fremd.id, StatusCode::FORBIDDEN)] {
        assert_eq!(
            status(
                &app,
                &format!("/api/projekte/{projekt}/xbau/nachrichten"),
                true
            )
            .await,
            expected
        );
    }
    // Vorschau ist lesend, Aufnahme bleibt schreibgeschützt; fremde Projekte verboten.
    for (projekt, operation, expected) in [
        (p.id, "pruefen", StatusCode::UNPROCESSABLE_ENTITY),
        (p.id, "nachrichten", StatusCode::FORBIDDEN),
        (fremd.id, "pruefen", StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projekte/{projekt}/xbau/{operation}"))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    // Ein fremdes Raumbuch darf auch über den neuen Importweg nicht gelesen werden.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projekte/{}/raumbuchimport", fremd.id))
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"id":uuid::Uuid::new_v4(),"modul":"wohnflaechen"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let mut plan = core_lib::abstandsflaechen::testdaten::plan();
    plan.regeln = core_lib::abstandsflaechen::katalog()
        .unwrap()
        .into_iter()
        .find(|r| r.land == Bundesland::By)
        .unwrap();
    for (projekt, expected) in [(p.id, StatusCode::OK), (fremd.id, StatusCode::FORBIDDEN)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projekte/{projekt}/abstandsplan"))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&plan).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    // Neue Akten-Lesewege sind projektgebunden; Schreibwege sperren Leser
    // bereits vor dem Parsen oder Ausführen eines Auftrags.
    for suffix in [
        "erinnerungen?heute=2026-10-15",
        "aktensuche?suche=Plan",
        "aktenexport",
        "verfahrensprofil",
        "abstandsregeln",
        "uebersicht",
    ] {
        assert_eq!(
            status(&app, &format!("/api/projekte/{}/{suffix}", p.id), true).await,
            StatusCode::OK
        );
        assert_eq!(
            status(&app, &format!("/api/projekte/{}/{suffix}", fremd.id), true).await,
            StatusCode::FORBIDDEN
        );
    }
    for suffix in [
        "suchtext".to_owned(),
        format!("verfahren/{}/profil", uuid::Uuid::new_v4()),
        format!("verfahren/{}/abgabe", uuid::Uuid::new_v4()),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projekte/{}/{suffix}", p.id))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projekte")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let daten: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_eq!(daten.as_array().unwrap().len(), 1);
    assert_eq!(daten[0]["id"], p.id.to_string());
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projekte/{}/fachobjekte/aufgaben", p.id))
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        status(&app, &format!("/api/v1/projekte/{}", p.id), true).await,
        StatusCode::OK
    );
    // Leser dürfen Vorlagen ansehen, aber den gemeinsamen Bürostandard nicht ändern.
    assert_eq!(
        status(&app, "/api/druckgestaltung", true).await,
        StatusCode::OK
    );
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/druckgestaltung")
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&core_lib::druckausgabe::Druckgestaltung::default())
                        .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    // PDF-Ausgabe ist auch als POST eine lesende Operation. Fremde Quellen
    // werden zusätzlich zum Projektpfad im Core überprüft.
    let mail=bytes::Bytes::from_static(b"From: buero@example.org\r\nTo: bauamt@example.org\r\nSubject: Unterlagen\r\n\r\nAnbei die Unterlagen.");
    let d = core
        .store_document(p.id, None, None, "Anschreiben.eml", mail.clone(), &admin)
        .await
        .unwrap();
    let fremdes_dokument = core
        .store_document(fremd.id, None, None, "Fremd.eml", mail, &admin)
        .await
        .unwrap();
    let mut auftrag = core_lib::druckausgabe::Druckauftrag {
        gestaltung: None,
        aktenzeichen: String::new(),
        datum: None,
        titel: "Druckpaket".into(),
        absender: String::new(),
        empfaenger: String::new(),
        anschreiben: String::new(),
        mit_anschreiben: false,
        inhaltsverzeichnis: false,
        projekt_version: p.version,
        quellen: vec![core_lib::druckausgabe::Druckquelle::Dokument {
            dokument_id: d.document_id,
            version: 1,
            seiten: vec![],
        }],
    };
    assert_eq!(
        status(
            &app,
            &format!("/api/v1/projekte/{}/druckkatalog", p.id),
            true
        )
        .await,
        StatusCode::OK
    );
    let request = |a: &core_lib::druckausgabe::Druckauftrag| {
        Request::builder()
            .method("POST")
            .uri(format!("/api/v1/projekte/{}/druckausgabe", p.id))
            .header("authorization", format!("Bearer {TOKEN}"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(a).unwrap()))
            .unwrap()
    };
    let antwort = app.clone().oneshot(request(&auftrag)).await.unwrap();
    assert_eq!(antwort.status(), StatusCode::OK);
    assert_eq!(antwort.headers()["content-type"], "application/pdf");
    assert!(
        to_bytes(antwort.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 1);
    auftrag.quellen = vec![core_lib::druckausgabe::Druckquelle::Dokument {
        dokument_id: fremdes_dokument.document_id,
        version: 1,
        seiten: vec![],
    }];
    assert_eq!(
        app.clone()
            .oneshot(request(&auftrag))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    core.projektmitglied_setzen(
        p.id,
        ProjektMitglied {
            benutzer_id: leser.id,
            recht: ProjektRecht::Kein,
        },
        1,
        &admin,
    )
    .await
    .unwrap();
    assert_eq!(
        status(&app, &format!("/api/v1/projekte/{}", p.id), true).await,
        StatusCode::FORBIDDEN
    );
    core.benutzer_aendern(
        BenutzerAenderung {
            id: leser.id,
            anzeigename: leser.anzeigename.clone(),
            rolle: leser.rolle,
            aktiv: false,
        },
        &admin,
    )
    .await
    .unwrap();
    assert_eq!(
        status(&app, "/api/projekte", true).await,
        StatusCode::UNAUTHORIZED
    );
    std::fs::write(&path, b"{\"schema_version\":1,\"zugaenge\":[]}").unwrap();
    assert_eq!(
        status(&app, "/api/projekte", true).await,
        StatusCode::UNAUTHORIZED
    );
    std::fs::remove_dir_all(dir.path().join("dateien")).unwrap();
    std::fs::write(dir.path().join("dateien"), b"Ausfall").unwrap();
    let unbereit = api::zugang::gesichert(core, api::zugang::ZugangQuelle::Datei(path));
    assert_eq!(
        status(&unbereit, "/health", false).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
