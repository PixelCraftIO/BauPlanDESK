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
async fn erstellen_bestaetigen_vergleichen_und_unveraendertes_zip() {
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let e: core_lib::einreichung::Einreichung =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
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
    assert_eq!(response.status(), StatusCode::OK);
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
                    serde_json::json!({"basis":1,"datum":"2026-01-02","notiz":""}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
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
    assert_eq!(
        to_bytes(response.into_body(), 100000).await.unwrap(),
        before
    );
}
