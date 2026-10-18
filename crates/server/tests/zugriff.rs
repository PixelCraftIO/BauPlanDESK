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
/// Kodiert das erste Zeichen einer ID (`3…` → `%33…`), wie es ein Angreifer täte.
fn kodiert(id: impl std::fmt::Display) -> String {
    let id = id.to_string();
    format!("%{:02X}{}", id.as_bytes()[0], &id[1..])
}
async fn anfrage(app: &Router, method: &str, path: &str, body: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
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
    // Prozentkodierte IDs werden wie dekodierte geprüft (axum dekodiert für den Handler).
    for prefix in ["/api", "/api/v1"] {
        assert_eq!(
            status(&app, &format!("{prefix}/projekte/{}", kodiert(p.id)), true).await,
            StatusCode::OK
        );
        assert_eq!(
            status(
                &app,
                &format!("{prefix}/projekte/{}", kodiert(fremd.id)),
                true
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            status(
                &app,
                &format!("{prefix}/projekte/{}/protokoll", kodiert(fremd.id)),
                true
            )
            .await,
            StatusCode::FORBIDDEN
        );
        // Schreibwege: Leser darf weder im eigenen noch im fremden Projekt schreiben.
        for projekt in [p.id, fremd.id] {
            assert_eq!(
                anfrage(
                    &app,
                    "PUT",
                    &format!("{prefix}/projekte/{}", kodiert(projekt)),
                    "{}"
                )
                .await,
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                anfrage(
                    &app,
                    "PUT",
                    &format!("{prefix}/projekte/{}/bauantrag", kodiert(projekt)),
                    "{}"
                )
                .await,
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                anfrage(
                    &app,
                    "POST",
                    &format!(
                        "{prefix}/projekte/{}/fachobjekte/aufgaben",
                        kodiert(projekt)
                    ),
                    "{}"
                )
                .await,
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                anfrage(
                    &app,
                    "POST",
                    &format!(
                        "{prefix}/projekte/{}/verfahren/{}/portale",
                        kodiert(projekt),
                        uuid::Uuid::new_v4()
                    ),
                    "{}"
                )
                .await,
                StatusCode::FORBIDDEN
            );
        }
        // Nicht lesbare IDs unter Projekt-, Dokument- und Einreichungspfaden: 400.
        for pfad in [
            format!("{prefix}/projekte/kein-projekt"),
            format!("{prefix}/projekte/%ZZ{}", &p.id.to_string()[1..]),
            format!("{prefix}/projekte/{}%2Fstatus", p.id),
            format!("{prefix}/dokumente/%C3%28/versionen"),
            format!("{prefix}/einreichungen/x/zip"),
        ] {
            assert_eq!(
                status(&app, &pfad, true).await,
                StatusCode::BAD_REQUEST,
                "{pfad}"
            );
        }
    }
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
    // Portalvorschau ist lesend; Belege, Pakete und Katalogimporte bleiben geschützt.
    let v = uuid::Uuid::new_v4();
    let g = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    for (projekt, suffix, expected) in [
        (
            p.id,
            format!("verfahren/{v}/portale/{g}/pruefen"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            fremd.id,
            format!("verfahren/{v}/portale/{g}/pruefen"),
            StatusCode::FORBIDDEN,
        ),
        (
            p.id,
            format!("verfahren/{v}/portale"),
            StatusCode::FORBIDDEN,
        ),
        (
            p.id,
            format!("verfahren/{v}/portale/{g}/abgaben"),
            StatusCode::FORBIDDEN,
        ),
        (
            p.id,
            format!("verfahren/{v}/portale/{g}/abgaben/{b}/ereignisse"),
            StatusCode::FORBIDDEN,
        ),
        (
            p.id,
            format!("verfahren/{v}/portale/{g}/rueckmeldungen"),
            StatusCode::FORBIDDEN,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projekte/{projekt}/{suffix}"))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{suffix}");
    }
    for suffix in [
        "url".to_owned(),
        format!("abgaben/{b}/paket"),
        format!("abgaben/{b}/dateien/{v}/1"),
    ] {
        assert_eq!(
            status(
                &app,
                &format!(
                    "/api/projekte/{}/verfahren/{v}/portale/{g}/{suffix}",
                    fremd.id
                ),
                true
            )
            .await,
            StatusCode::FORBIDDEN
        );
    }
    // Die Geometrievorschau ist lesend, bleibt aber an die Projektberechtigung gebunden.
    for (projekt, expected) in [(p.id, StatusCode::OK), (fremd.id, StatusCode::FORBIDDEN)] {
        let response = app.clone().oneshot(Request::builder().method("POST")
            .uri(format!("/api/projekte/{}/gebaeudegrundlagen", projekt))
            .header("authorization", format!("Bearer {TOKEN}"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"id":uuid::Uuid::new_v4(),"gebaeude":{"bezeichnung":"Garage","grundstueck_ids":[],"geschosse":[],"quelle":null,"geometrie":{"grundflaeche_m2":20}}}).to_string())).unwrap()).await.unwrap();
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
    // Die Fristberechnung ändert nichts und steht Lesern ihres Projekts offen.
    let auftrag = serde_json::json!({"art":"bauantrag","bescheide":[]});
    for (projekt, expected) in [(p.id, StatusCode::OK), (fremd.id, StatusCode::FORBIDDEN)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/projekte/{projekt}/bescheidfristen"))
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(auftrag.to_string()))
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
    for (dokument, expected) in [
        (d.document_id, StatusCode::OK),
        (fremdes_dokument.document_id, StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            status(
                &app,
                &format!("/api/v1/dokumente/{}/versionen", kodiert(dokument.0)),
                true
            )
            .await,
            expected
        );
    }
    assert_eq!(
        anfrage(
            &app,
            "POST",
            &format!(
                "/api/dokumente/{}/formular",
                kodiert(fremdes_dokument.document_id.0)
            ),
            "{}"
        )
        .await,
        StatusCode::FORBIDDEN
    );
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

/// Eine neuere Programmversion an derselben Datenbank ergänzt Felder bzw. ein
/// neues Recht in einer gespeicherten Mitgliedschaft.
#[tokio::test]
async fn mitgliedschaft_aus_neuerer_version_sperrt_aeltere_arbeitsplaetze_nicht_aus() {
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
    let p = core
        .create_project(
            NewProject {
                name: "Gemeinsames Projekt".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &admin,
        )
        .await
        .unwrap();
    let neuer_stand = |daten: serde_json::Value, revision: i64| core_lib::commit::Fachstand {
        projekt_id: p.id,
        modul: "mitgliedschaften".into(),
        objekt: leser.id.to_string(),
        schema_version: 1,
        revision,
        daten,
    };
    let audit = || core_lib::domain::NeuerProtokolleintrag {
        zeit: chrono::Utc::now(),
        benutzer_id: admin.id,
        benutzer_name: admin.anzeigename.clone(),
        aktion: core_lib::domain::Aktion::BenutzerGeaendert,
        project_id: Some(p.id),
        document_id: None,
        details: "neuere Version".into(),
    };
    core.repos
        .commit
        .fachstand(
            neuer_stand(
                serde_json::json!({"benutzer_id":leser.id,"recht":"lesen","gueltig_bis":"2030-01-01"}),
                0,
            ),
            0,
            audit(),
        )
        .await
        .unwrap();
    // Zusätzliches Feld: Zugriffsprüfung liest weiter, kein Datenbankfehler.
    core.projekt_zugriff(p.id, &leser, ProjektRecht::Lesen)
        .await
        .unwrap();
    assert!(matches!(
        core.projekt_zugriff(p.id, &leser, ProjektRecht::Bearbeiten)
            .await,
        Err(core_lib::CoreError::Forbidden(_))
    ));
    assert_eq!(core.projekte_fuer_benutzer(&leser).await.unwrap().len(), 1);
    // Überschreiben mit dem älteren Format würde das neue Feld verlieren: abgewiesen.
    let e = core
        .projektmitglied_setzen(
            p.id,
            ProjektMitglied {
                benutzer_id: leser.id,
                recht: ProjektRecht::Bearbeiten,
            },
            1,
            &admin,
        )
        .await
        .unwrap_err();
    assert!(
        e.to_string().contains("neueren BauProjektIO-Version"),
        "{e}"
    );
    // Unbekanntes Recht gewährt nichts, die Projektliste bleibt benutzbar.
    core.repos
        .commit
        .fachstand(
            neuer_stand(
                serde_json::json!({"benutzer_id":leser.id,"recht":"pruefen"}),
                1,
            ),
            1,
            audit(),
        )
        .await
        .unwrap();
    let e = core
        .projekt_zugriff(p.id, &leser, ProjektRecht::Lesen)
        .await
        .unwrap_err();
    assert!(matches!(e, core_lib::CoreError::Forbidden(_)), "{e}");
    assert!(
        e.to_string().contains("neueren BauProjektIO-Version"),
        "{e}"
    );
    assert!(
        core.projekte_fuer_benutzer(&leser)
            .await
            .unwrap()
            .is_empty()
    );
    // Die API-Grenze bleibt streng: unbekannte Felder in Eingaben werden abgelehnt.
    assert!(
        serde_json::from_value::<ProjektMitglied>(
            serde_json::json!({"benutzer_id":leser.id,"recht":"lesen","gueltig_bis":"2030-01-01"})
        )
        .is_err()
    );
}
