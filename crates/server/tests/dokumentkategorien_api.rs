use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::ContentHash,
};
use std::sync::Arc;
use tower::ServiceExt;
const TOKEN: &str = "kategorien-http-testzugang-12345678901234567890";
async fn call(
    router: &axum::Router,
    method: &str,
    pfad: &str,
    body: serde_json::Value,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut r = Request::builder()
        .method(method)
        .uri(pfad)
        .header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("authorization", format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(r.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1000000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}
#[tokio::test]
async fn kategorien_http_vertrag_konflikte_und_projektzugriff() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(tmp.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("kategorien-http", "Admin").await.unwrap();
    let fremd = core.anmelden("fremd", "Fremd").await.unwrap();
    let fremd_token = "fremder-http-testzugang-12345678901234567890";
    let z = server::api::zugang::Zugaenge {
        schema_version: 1,
        zugaenge: vec![
            server::api::zugang::Zugang {
                benutzer_id: user.id,
                token_sha256: ContentHash::of(TOKEN.as_bytes()),
                gueltig_bis: None,
            },
            server::api::zugang::Zugang {
                benutzer_id: fremd.id,
                token_sha256: ContentHash::of(fremd_token.as_bytes()),
                gueltig_bis: None,
            },
        ],
    };
    let path = tmp.path().join("zugaenge.json");
    std::fs::write(&path, serde_json::to_vec(&z).unwrap()).unwrap();
    let router = server::api::zugang::gesichert(
        core.clone(),
        server::api::zugang::ZugangQuelle::Datei(path),
    );
    let p = core
        .create_project(
            NewProject {
                name: "HTTP-Kategorien".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let doc = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.dwg",
            bytes::Bytes::from_static(b"dwg"),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        call(
            &router,
            "GET",
            "/api/dokumentkategorien",
            serde_json::Value::Null,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, mut katalog) = call(
        &router,
        "GET",
        "/api/dokumentkategorien",
        serde_json::Value::Null,
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    katalog["kategorien"][0]["name"] = "Antrag".into();
    assert_eq!(
        call(
            &router,
            "PUT",
            "/api/dokumentkategorien",
            katalog.clone(),
            Some(fremd_token)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &router,
            "PUT",
            "/api/dokumentkategorien",
            katalog.clone(),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &router,
            "PUT",
            "/api/dokumentkategorien",
            katalog,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let url = format!("/api/dokumente/{}/kategorie", doc.document_id.0);
    let k = serde_json::json!({"revision":0,"kategorie":"lageplan"});
    assert_eq!(
        call(&router, "PUT", &url, k.clone(), Some(fremd_token))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let (status, zuordnung) = call(&router, "PUT", &url, k.clone(), Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(zuordnung["revision"], 1);
    assert_eq!(
        call(&router, "PUT", &url, k, Some(TOKEN)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &router,
            "PUT",
            &url,
            serde_json::json!({"revision":1,"kategorie":"fehlt"}),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, liste) = call(
        &router,
        "GET",
        &format!("/api/projekte/{}/dokumentkategorien", p.id),
        serde_json::Value::Null,
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(liste[0]["kategorie"], "lageplan");
    assert_eq!(
        core.document_history(doc.document_id).await.unwrap().len(),
        1
    );
    let hash = ContentHash::of(b"vorlage");
    let url = format!("/api/vorlagen/{}/kategorie", hash.as_str());
    assert_eq!(
        call(
            &router,
            "PUT",
            &url,
            serde_json::json!({"revision":0,"kategorie":"baubeschreibung"}),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(&router, "GET", &url, serde_json::Value::Null, Some(TOKEN))
            .await
            .1["kategorie"],
        "baubeschreibung"
    );
}
