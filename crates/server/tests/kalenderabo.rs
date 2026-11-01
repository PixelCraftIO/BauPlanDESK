//! Kalender-Abo über den Server: Schlüssel statt Sitzung, Projektrechte je Abruf.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    kalenderabo::{Aboschluessel, Aboumfang},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    storage::ContentHash,
    verfahren::externe_einreichung::{Einreichungsweg, ExterneEinreichungswunsch},
    zugriff::{ProjektMitglied, ProjektRecht},
};
use server::api;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "testzugang-kalenderabo-nur-fuer-diese-pruefung-123456789";

async fn abruf(
    app: &Router,
    method: &str,
    path: &str,
    token: bool,
    body: &str,
) -> (StatusCode, String) {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if token {
        r = r.header("authorization", format!("Bearer {TOKEN}"));
    }
    let a = app
        .clone()
        .oneshot(r.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let s = a.status();
    let b = to_bytes(a.into_body(), 1 << 20).await.unwrap();
    (s, String::from_utf8_lossy(&b).into_owned())
}

#[tokio::test]
async fn kalenderabo_liefert_termine_und_ist_widerrufbar() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let planer = core.anmelden("planer", "Planung").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Haus am Hang".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &admin,
        )
        .await
        .unwrap();
    core.projektmitglied_setzen(
        p.id,
        ProjektMitglied {
            benutzer_id: planer.id,
            recht: ProjektRecht::Lesen,
        },
        0,
        &admin,
    )
    .await
    .unwrap();
    // Bearbeitungsfrist der Behörde: ein Termin, der im Abo erscheinen muss.
    let akte = serde_json::json!({
        "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
        "aktenzeichen": "", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "bearbeitungsfrist": {"datum": "2099-03-31", "ausloeser": "Eingangsbestätigung", "beleg": null,
            "erinnerung_tage": 7, "grund": "Laut Eingangsbestätigung"},
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    });
    let vid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Verfahren, vid, 0, akte, &admin)
        .await
        .unwrap();
    core.verfahren_externe_einreichung(
        p.id,
        vid,
        f.revision,
        ExterneEinreichungswunsch {
            id: Uuid::new_v4(),
            datum: "2026-03-01".parse().unwrap(),
            weg: Einreichungsweg::Post,
            beleg: None,
            bemerkung: String::new(),
            dokumente: vec![],
            berichtigt: None,
        },
        &admin,
    )
    .await
    .unwrap();

    let zugang = api::zugang::Zugaenge {
        schema_version: 1,
        zugaenge: vec![api::zugang::Zugang {
            benutzer_id: planer.id,
            token_sha256: ContentHash::of(TOKEN.as_bytes()),
            gueltig_bis: None,
        }],
    };
    let path = dir.path().join("zugaenge.json");
    std::fs::write(&path, serde_json::to_vec(&zugang).unwrap()).unwrap();
    let app = api::zugang::gesichert(core.clone(), api::zugang::ZugangQuelle::Datei(path));

    // Anlegen nur mit Anmeldung.
    assert_eq!(
        abruf(
            &app,
            "POST",
            "/api/kalenderabo",
            false,
            r#"{"umfang":"alle"}"#
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (s, b) = abruf(
        &app,
        "POST",
        "/api/kalenderabo",
        true,
        r#"{"umfang":"alle"}"#,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let abo: Aboschluessel = serde_json::from_str(&b).unwrap();
    assert_eq!(abo.umfang, Aboumfang::Alle);
    assert!(abo.pfad.starts_with(&format!("/kalender/{}/", planer.id.0)));

    // Abruf ohne Sitzung mit Schlüssel.
    let (s, ics) = abruf(&app, "GET", &abo.pfad, false, "").await;
    assert_eq!(s, StatusCode::OK);
    assert!(ics.starts_with("BEGIN:VCALENDAR"));
    assert!(
        ics.replace("\r\n ", "")
            .contains("Haus am Hang · Bauantrag"),
        "{ics}"
    );
    assert!(ics.contains("DTSTART;VALUE=DATE:20990331"));

    // Falscher Schlüssel, fremdes Konto und Unsinn: immer 404.
    let falsch = format!("/kalender/{}/{}.ics", planer.id.0, "0".repeat(64));
    assert_eq!(
        abruf(&app, "GET", &falsch, false, "").await.0,
        StatusCode::NOT_FOUND
    );
    let fremd = abo
        .pfad
        .replace(&planer.id.0.to_string(), &admin.id.0.to_string());
    assert_eq!(
        abruf(&app, "GET", &fremd, false, "").await.0,
        StatusCode::NOT_FOUND
    );
    let kurz = format!("/kalender/{}/abc.ics", planer.id.0);
    assert_eq!(
        abruf(&app, "GET", &kurz, false, "").await.0,
        StatusCode::NOT_FOUND
    );

    // „Nur meine“: der Planer ist nicht zuständig, der Kalender bleibt leer.
    let (_, b) = abruf(
        &app,
        "POST",
        "/api/kalenderabo",
        true,
        r#"{"umfang":"meine"}"#,
    )
    .await;
    let meine: Aboschluessel = serde_json::from_str(&b).unwrap();
    assert_eq!(
        abruf(&app, "GET", &abo.pfad, false, "").await.0,
        StatusCode::NOT_FOUND,
        "alter Schlüssel ungültig"
    );
    let (s, ics) = abruf(&app, "GET", &meine.pfad, false, "").await;
    assert_eq!(s, StatusCode::OK);
    assert!(!ics.contains("BEGIN:VEVENT"));

    // Widerruf.
    assert_eq!(
        abruf(&app, "DELETE", "/api/kalenderabo", true, "").await.0,
        StatusCode::OK
    );
    assert_eq!(
        abruf(&app, "GET", &meine.pfad, false, "").await.0,
        StatusCode::NOT_FOUND
    );
    let (_, b) = abruf(&app, "GET", "/api/kalenderabo", true, "").await;
    assert!(b.contains("\"aktiv\":false"), "{b}");
}
