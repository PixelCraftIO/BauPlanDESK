//! Ende-zu-Ende über HTTP gegen eine echte PostgreSQL-Datenbank. Läuft nur
//! mit `BAUPROJEKT_TEST_POSTGRES=postgres://…`; jeder Test bekommt ein
//! eigenes Schema.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use core_lib::license::datei::testschluessel;
use core_lib::license::ticket::{Ablehnung, Ablehnungsgrund, TicketAntwort};
use core_lib::license::{Edition, Lizenzpruefer};
use http_body_util::BodyExt;
use lizenz_server::db::Db;
use lizenz_server::stripe::signatur_erzeugen;
use lizenz_server::{AppState, Einstellungen, router};
use serde_json::{Value, json};
use sqlx::postgres::PgConnectOptions;
use tower::ServiceExt;

const ADMIN: &str = "admin-token";
const UPLOAD: &str = "upload-token";
const WHSEC: &str = "whsec_test";

struct Testserver {
    app: Router,
    db: Db,
    _dir: tempfile::TempDir,
}

async fn testserver() -> Option<Testserver> {
    let Ok(url) = std::env::var("BAUPROJEKT_TEST_POSTGRES") else {
        eprintln!("BAUPROJEKT_TEST_POSTGRES nicht gesetzt, Test übersprungen");
        return None;
    };
    let schema = format!("test_{}", uuid::Uuid::new_v4().simple());
    let basis: PgConnectOptions = url.parse().unwrap();
    // Eigenes Schema, damit Tests sich nicht in die Quere kommen.
    let pool = sqlx::PgPool::connect_with(basis.clone()).await.unwrap();
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&pool)
        .await
        .unwrap();
    let db = Db::mit_optionen(basis.options([("search_path", schema.as_str())]))
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let e = Einstellungen {
        ticket_schluessel: testschluessel::ticket_geheim(),
        ticket_dauer: Duration::days(30),
        stripe_webhook_secret: Some(WHSEC.into()),
        admin_token: Some(ADMIN.into()),
        upload_token: Some(UPLOAD.into()),
        update_dir: dir.path().to_path_buf(),
        standard_edition: Edition::Buero,
    };
    Some(Testserver {
        app: router(AppState {
            db: db.clone(),
            e: Arc::new(e),
        }),
        db,
        _dir: dir,
    })
}

impl Testserver {
    async fn anfrage(&self, req: Request<Body>) -> (StatusCode, Vec<u8>) {
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, body)
    }

    async fn json(
        &self,
        methode: &str,
        pfad: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder()
            .method(methode)
            .uri(pfad)
            .header("content-type", "application/json");
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let (status, body) = self
            .anfrage(req.body(Body::from(body.to_string())).unwrap())
            .await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    async fn ticket(&self, schluessel: &str, machine_id: &str) -> (StatusCode, Value) {
        self.json(
            "POST",
            "/v1/ticket",
            None,
            json!({"lizenzschluessel": schluessel, "machine_id": machine_id,
                   "rechnername": format!("PC {machine_id}"), "app_version": "0.1.0"}),
        )
        .await
    }

    async fn stripe(&self, ereignis: Value) -> StatusCode {
        let body = ereignis.to_string();
        let sig = signatur_erzeugen(body.as_bytes(), WHSEC, Utc::now().timestamp());
        self.anfrage(
            Request::post("/v1/stripe/webhook")
                .header("stripe-signature", sig)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .0
    }
}

fn grund(body: &Value) -> Ablehnungsgrund {
    serde_json::from_value::<Ablehnung>(body.clone())
        .unwrap()
        .grund
}

fn pruefer() -> Lizenzpruefer {
    Lizenzpruefer::new(vec![]).mit_ticket_schluesseln(vec![testschluessel::ticket_oeffentlich()])
}

#[tokio::test]
async fn plaetze_aktivieren_abmelden_sperren() {
    let Some(s) = testserver().await else { return };

    // Verwaltung nur mit Token.
    let neu = json!({"lizenznehmer": "Büro Muster", "edition": "buero", "arbeitsplaetze": 2});
    assert_eq!(
        s.json("POST", "/admin/lizenzen", None, neu.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        s.json("POST", "/admin/lizenzen", Some("falsch"), neu.clone())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, lizenz) = s.json("POST", "/admin/lizenzen", Some(ADMIN), neu).await;
    assert_eq!(status, StatusCode::CREATED);
    let schluessel = lizenz["schluessel"].as_str().unwrap().to_string();
    let id = lizenz["id"].as_str().unwrap().to_string();

    // Zwei Plätze: A und B bekommen ein Ticket, C nicht.
    let (status, a) = s.ticket(&schluessel.to_lowercase(), "a").await;
    assert_eq!(status, StatusCode::OK, "{a}");
    let a: TicketAntwort = serde_json::from_value(a).unwrap();
    let t = pruefer().ticket_pruefen(a.ticket.as_bytes(), "a").unwrap();
    assert_eq!(t.lizenz.lizenznehmer, "Büro Muster");
    assert_eq!(t.lizenz.arbeitsplaetze, 2);
    assert!(t.gueltig_bis > Utc::now() + Duration::days(29));
    assert!(pruefer().ticket_pruefen(a.ticket.as_bytes(), "b").is_err());
    assert_eq!(s.ticket(&schluessel, "b").await.0, StatusCode::OK);
    let (status, c) = s.ticket(&schluessel, "c").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(grund(&c), Ablehnungsgrund::KeinePlaetze);
    // A erneuert, ohne einen weiteren Platz zu belegen.
    assert_eq!(s.ticket(&schluessel, "a").await.0, StatusCode::OK);

    // A meldet sich ab, dann ist für C Platz.
    let (status, _) = s
        .json(
            "POST",
            "/v1/abmelden",
            None,
            json!({"lizenzschluessel": schluessel, "machine_id": "a"}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(s.ticket(&schluessel, "c").await.0, StatusCode::OK);

    // B war über die Ticketdauer hinaus nicht da: Platz wird frei.
    sqlx::query(
        "UPDATE aktivierungen SET zuletzt_am = now() - interval '31 days' WHERE machine_id = 'b'",
    )
    .execute(&s.db.pool)
    .await
    .unwrap();
    assert_eq!(s.ticket(&schluessel, "d").await.0, StatusCode::OK);
    let (_, detail) = s
        .json(
            "GET",
            &format!("/admin/lizenzen/{id}"),
            Some(ADMIN),
            Value::Null,
        )
        .await;
    let rechner: Vec<&str> = detail["aktivierungen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["machine_id"].as_str().unwrap())
        .collect();
    assert_eq!(rechner, ["c", "d"]);

    // Sperren.
    let (status, _) = s
        .json(
            "PATCH",
            &format!("/admin/lizenzen/{id}"),
            Some(ADMIN),
            json!({"status": "gesperrt"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = s.ticket(&schluessel, "c").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(grund(&body), Ablehnungsgrund::Gesperrt);

    // Unbekannter Schlüssel.
    let (status, body) = s.ticket("BPIO-AAAAA-BBBBB", "a").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(grund(&body), Ablehnungsgrund::Unbekannt);

    // Befristete Lizenz, abgelaufen.
    let (_, alt) = s
        .json(
            "POST",
            "/admin/lizenzen",
            Some(ADMIN),
            json!({"lizenznehmer": "Alt", "edition": "einzelplatz", "arbeitsplaetze": 1,
                   "gueltig_bis": (Utc::now() - Duration::days(1)).date_naive()}),
        )
        .await;
    let (status, body) = s.ticket(alt["schluessel"].as_str().unwrap(), "a").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(grund(&body), Ablehnungsgrund::Inaktiv);
}

#[tokio::test]
async fn stripe_kauf_und_kuendigung() {
    let Some(s) = testserver().await else { return };
    let jetzt = Utc::now().timestamp();
    let abo = |id: &str, typ: &str, status: &str, menge: u64, created: i64| {
        json!({"id": id, "type": typ, "created": created, "data": {"object": {
            "id": "sub_1", "customer": "cus_1", "status": status, "metadata": {},
            "items": {"data": [{"quantity": menge, "current_period_end": jetzt + 86400 * 30}]}
        }}})
    };

    // Falsche Signatur.
    let res = s
        .anfrage(
            Request::post("/v1/stripe/webhook")
                .header("stripe-signature", format!("t={jetzt},v1=00"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;
    assert_eq!(res.0, StatusCode::BAD_REQUEST);

    // Kunde ist schneller auf der Erfolgsseite als der Webhook.
    let (status, seite) = s
        .anfrage(
            Request::get("/kauf/erfolg?session_id=cs_1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(seite).unwrap().contains("refresh"));

    // Abo-Ereignis kommt vor dem Checkout-Ereignis (Stripe garantiert keine
    // Reihenfolge).
    assert_eq!(
        s.stripe(abo(
            "evt_1",
            "customer.subscription.created",
            "active",
            3,
            jetzt
        ))
        .await,
        StatusCode::OK
    );
    let kauf = json!({"id": "evt_2", "type": "checkout.session.completed", "created": jetzt,
        "data": {"object": {"id": "cs_1", "mode": "subscription", "subscription": "sub_1",
            "customer": "cus_1", "metadata": {"edition": "buero"},
            "customer_details": {"name": "Clara Chefin", "email": "clara@example.com"},
            "custom_fields": [{"key": "lizenznehmer", "text": {"value": "Architekturbüro Muster"}}]}}});
    assert_eq!(s.stripe(kauf.clone()).await, StatusCode::OK);
    // Doppelt zugestellt: kein Fehler, keine zweite Lizenz.
    assert_eq!(s.stripe(kauf).await, StatusCode::OK);
    let (_, liste) = s
        .json("GET", "/admin/lizenzen", Some(ADMIN), Value::Null)
        .await;
    let liste = liste.as_array().unwrap();
    assert_eq!(liste.len(), 1);
    let l = &liste[0];
    assert_eq!(l["lizenznehmer"], "Architekturbüro Muster");
    assert_eq!(l["email"], "clara@example.com");
    assert_eq!(l["arbeitsplaetze"], 3);
    assert_eq!(l["status"], "aktiv");
    let schluessel = l["schluessel"].as_str().unwrap().to_string();

    let (_, seite) = s
        .anfrage(
            Request::get("/kauf/erfolg?session_id=cs_1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let seite = String::from_utf8(seite).unwrap();
    assert!(seite.contains(&schluessel), "{seite}");
    assert!(seite.contains("Architekturbüro Muster"));

    assert_eq!(s.ticket(&schluessel, "a").await.0, StatusCode::OK);

    // Ein verspätetes, älteres Ereignis überschreibt nichts.
    assert_eq!(
        s.stripe(abo(
            "evt_3",
            "customer.subscription.updated",
            "incomplete",
            1,
            jetzt - 100
        ))
        .await,
        StatusCode::OK
    );
    assert_eq!(s.ticket(&schluessel, "a").await.0, StatusCode::OK);

    // Gekündigt: kein Ticket mehr.
    assert_eq!(
        s.stripe(abo(
            "evt_4",
            "customer.subscription.deleted",
            "canceled",
            3,
            jetzt + 10
        ))
        .await,
        StatusCode::OK
    );
    let (status, body) = s.ticket(&schluessel, "a").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(grund(&body), Ablehnungsgrund::Inaktiv);
}

#[tokio::test]
async fn updates_hochladen_und_abrufen() {
    let Some(s) = testserver().await else { return };
    let put = |pfad: &str, token: &str, inhalt: &'static str| {
        Request::put(format!("/upload/updates/{pfad}"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(inhalt))
            .unwrap()
    };
    assert_eq!(
        s.anfrage(put("latest.json", "falsch", "{}")).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        s.anfrage(put("..%2Fx", UPLOAD, "{}")).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        s.anfrage(put("0.2.0/app.exe", UPLOAD, "MZ")).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        s.anfrage(put("latest.json", UPLOAD, r#"{"version":"0.2.0"}"#))
            .await
            .0,
        StatusCode::CREATED
    );

    let (status, body) = s
        .anfrage(
            Request::get("/updates/latest.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, br#"{"version":"0.2.0"}"#);
    let (status, body) = s
        .anfrage(
            Request::get("/updates/0.2.0/app.exe")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!((status, body.as_slice()), (StatusCode::OK, &b"MZ"[..]));
}
