//! HTTP adapter uses the same versioned Core contract as Desktop.
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use server::api;
use std::sync::Arc;
use tower::ServiceExt;
#[tokio::test]
async fn bestehende_pakete_bestaetigen_vergleichen_und_unveraendertes_zip() {
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().into(), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("http-test", "HTTP-Test").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "HTTP-Einreichung".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.pdf",
            Bytes::from_static(b"Originalplan"),
            &user,
        )
        .await
        .unwrap();
    let zugang = api::zugang::Zugaenge {
        schema_version: 1,
        zugaenge: vec![api::zugang::Zugang {
            benutzer_id: user.id,
            token_sha256: core_lib::storage::ContentHash::of(
                b"isolierter-http-testzugang-12345678901234567890",
            ),
            gueltig_bis: None,
        }],
    };
    let path = temp.path().join("zugaenge.json");
    std::fs::write(&path, serde_json::to_vec(&zugang).unwrap()).unwrap();
    let router = api::zugang::gesichert(core.clone(), api::zugang::ZugangQuelle::Datei(path));
    let payload = serde_json::json!({"titel":"Bauantrag","empfaenger":"Bauamt","dokumente":[{"dokument_id":d.document_id,"version":1}]});
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .header(
                    "authorization",
                    "Bearer isolierter-http-testzugang-12345678901234567890",
                )
                .method("POST")
                .uri(format!("/api/projekte/{}/einreichungen", p.id))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Neue Pakete entstehen nur noch als Abgabestand der Verfahrensakte.
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let e = core
        .einreichung_erstellen(p.id, serde_json::from_value(payload).unwrap(), &user)
        .await
        .unwrap();
    let zip_uri = format!("/api/einreichungen/{}/zip", e.stand.id);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .header(
                    "authorization",
                    "Bearer isolierter-http-testzugang-12345678901234567890",
                )
                .uri(&zip_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/zip");
    let before = to_bytes(response.into_body(), 100000).await.unwrap();
    let uri = format!("/api/einreichungen/{}/bestaetigen", e.stand.id);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .header(
                    "authorization",
                    "Bearer isolierter-http-testzugang-12345678901234567890",
                )
                .method("POST")
                .uri(&uri)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"basis":1,"datum":"2026-01-01","notiz":"Bauportal"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    // Pakete werden nicht mehr bestätigt; ältere Clients erhalten eine klare Meldung.
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let meldung = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(meldung.contains("Externe Einreichung"), "{meldung}");
    // Bereits früher bestätigte Pakete bleiben lesbar und vergleichbar.
    core.einreichung_bestaetigen_altbestand(
        e.stand.id,
        1,
        "2026-01-01".parse().unwrap(),
        "Bauportal".into(),
        &user,
    )
    .await
    .unwrap();
    core.store_document(
        p.id,
        Some(d.document_id),
        Some(1),
        "Plan.pdf",
        Bytes::from_static(b"Korrigierter Plan"),
        &user,
    )
    .await
    .unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .header(
                    "authorization",
                    "Bearer isolierter-http-testzugang-12345678901234567890",
                )
                .uri(format!(
                    "/api/projekte/{}/einreichungsvergleich?empfaenger=Bauamt",
                    p.id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v: Vec<core_lib::einreichung::DokumentVergleich> =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(
        v[0].status,
        core_lib::einreichung::VergleichsStatus::Geaendert
    );
    assert_eq!(v[0].zuletzt_eingereicht.as_ref().unwrap().version, 1);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .header(
                    "authorization",
                    "Bearer isolierter-http-testzugang-12345678901234567890",
                )
                .uri(zip_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Gestreamte Antwort mit korrekter Länge.
    assert_eq!(
        response.headers()["content-length"],
        before.len().to_string()
    );
    assert_eq!(
        to_bytes(response.into_body(), 100000).await.unwrap(),
        before
    );
    // Druckausgaben über dem axum-Standard (2 MB) erreichen die Core-Prüfung
    // (hier: kein PDF → 422) statt mit 413 abzubrechen; über 128 MB bleibt 413.
    let druck = |groesse: usize| {
        Request::builder()
            .header(
                "authorization",
                "Bearer isolierter-http-testzugang-12345678901234567890",
            )
            .method("POST")
            .uri(format!(
                "/api/projekte/{}/druckausgabe/unterlage?name=Plan&vorgang={}",
                p.id,
                uuid::Uuid::new_v4()
            ))
            .body(Body::from(vec![b'x'; groesse]))
            .unwrap()
    };
    let response = router
        .clone()
        .oneshot(druck(3 * 1024 * 1024))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = router
        .oneshot(druck(core_lib::druckausgabe::MAX_DRUCKPDF + 1))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
