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
