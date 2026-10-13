//! Lizenzserver von BauProjektIO (läuft beim Hersteller, z. B. auf Coolify).
//!
//! | Route                                   | Wer              | Zweck |
//! |-----------------------------------------|------------------|-------|
//! | `POST /v1/ticket`                       | Desktop-App      | Rechner aktivieren, Ticket erneuern |
//! | `POST /v1/abmelden`                     | Desktop-App      | Platz eines Rechners freigeben |
//! | `POST /v1/stripe/webhook`               | Stripe           | Zahlungen und Abo-Änderungen |
//! | `GET  /kauf/erfolg?session_id=…`        | Kunde im Browser | Lizenzschlüssel nach dem Bezahlen |
//! | `GET  /updates/…`                       | Desktop-App      | `latest.json` und Installer für den Auto-Updater |
//! | `PUT  /upload/updates/{pfad}`           | Release-Workflow | Update-Dateien hochladen (Upload-Token) |
//! | `/admin/lizenzen…`                      | Hersteller       | Lizenzen ansehen, anlegen, sperren (Admin-Token) |
//! | `GET  /gesund`                          | Coolify          | Healthcheck |

pub mod db;
pub mod schluessel;
pub mod stripe;

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use core_lib::license::ticket::{
    Ablehnung, Ablehnungsgrund, AbmeldeAnfrage, Ticket, TicketAnfrage, TicketAntwort,
    ticket_ausstellen,
};
use core_lib::license::{Edition, Lizenz};
use ed25519_dalek::SigningKey;
use futures::TryStreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::db::{Db, LizenzZeile, Lizenzaenderung, NeueLizenz, edition_aus_text};
use crate::stripe::Folge;

pub struct Einstellungen {
    /// Signiert die Tickets. Der öffentliche Teil steckt als
    /// `BAUPROJEKT_TICKET_SCHLUESSEL` in der App.
    pub ticket_schluessel: SigningKey,
    pub ticket_dauer: Duration,
    pub stripe_webhook_secret: Option<String>,
    pub admin_token: Option<String>,
    pub upload_token: Option<String>,
    pub update_dir: PathBuf,
    /// Edition für Stripe-Käufe ohne Metadatum `edition`.
    pub standard_edition: Edition,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub e: Arc<Einstellungen>,
}

pub fn router(state: AppState) -> Router {
    let updates = ServeDir::new(&state.e.update_dir);
    Router::new()
        .route("/gesund", get(gesund))
        .route("/v1/ticket", post(ticket))
        .route("/v1/abmelden", post(abmelden))
        .route("/v1/stripe/webhook", post(stripe_webhook))
        .route("/kauf/erfolg", get(kauf_erfolg))
        .route("/admin/lizenzen", get(admin_liste).post(admin_anlegen))
        .route(
            "/admin/lizenzen/{id}",
            get(admin_lizenz).patch(admin_aendern),
        )
        .route(
            "/admin/lizenzen/{id}/aktivierungen/{machine_id}",
            delete(admin_aktivierung_loeschen),
        )
        .route(
            "/upload/updates/{*pfad}",
            // Installer sind deutlich größer als die Standardgrenze von 2 MB.
            put(update_hochladen).layer(DefaultBodyLimit::disable()),
        )
        .nest_service("/updates", updates)
        .with_state(state)
}

// --- Fehler -------------------------------------------------------------------

pub enum Fehler {
    Abgelehnt(Ablehnung),
    Anfrage(String),
    NichtGefunden,
    NichtBerechtigt,
    Intern(String),
}

impl From<sqlx::Error> for Fehler {
    fn from(e: sqlx::Error) -> Self {
        Self::Intern(e.to_string())
    }
}

impl IntoResponse for Fehler {
    fn into_response(self) -> Response {
        match self {
            Self::Abgelehnt(a) => {
                let code = match a.grund {
                    Ablehnungsgrund::Unbekannt => StatusCode::NOT_FOUND,
                    _ => StatusCode::FORBIDDEN,
                };
                (code, Json(a)).into_response()
            }
            Self::Anfrage(m) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "fehler": m }))).into_response()
            }
            Self::NichtGefunden => StatusCode::NOT_FOUND.into_response(),
            Self::NichtBerechtigt => StatusCode::UNAUTHORIZED.into_response(),
            Self::Intern(m) => {
                tracing::error!(fehler = %m, "interner Fehler");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

type Ergebnis<T> = Result<T, Fehler>;

fn ablehnung(grund: Ablehnungsgrund) -> Fehler {
    let meldung = match grund {
        Ablehnungsgrund::Unbekannt => {
            "Dieser Lizenzschlüssel ist nicht bekannt. Bitte prüfen Sie die Eingabe."
        }
        Ablehnungsgrund::Inaktiv => {
            "Das Abo zu diesem Lizenzschlüssel ist nicht aktiv. Bitte prüfen Sie Ihre Zahlung oder verlängern Sie das Abo."
        }
        Ablehnungsgrund::Gesperrt => {
            "Dieser Lizenzschlüssel wurde gesperrt. Bitte wenden Sie sich an den Hersteller."
        }
        Ablehnungsgrund::KeinePlaetze => {
            "Alle Arbeitsplätze dieser Lizenz sind von anderen Rechnern belegt. Melden Sie einen Rechner ab oder erweitern Sie die Lizenz."
        }
    };
    Fehler::Abgelehnt(Ablehnung {
        grund,
        meldung: meldung.into(),
    })
}

// --- Desktop-App --------------------------------------------------------------

async fn gesund(State(s): State<AppState>) -> Ergebnis<&'static str> {
    sqlx::query("SELECT 1").execute(&s.db.pool).await?;
    Ok("ok")
}

async fn ticket(
    State(s): State<AppState>,
    Json(mut anfrage): Json<TicketAnfrage>,
) -> Ergebnis<Json<TicketAntwort>> {
    anfrage.lizenzschluessel = schluessel::normalisieren(&anfrage.lizenzschluessel);
    let machine_id = anfrage.machine_id.trim();
    if machine_id.is_empty() || machine_id.len() > 128 {
        return Err(Fehler::Anfrage("ungültige Machine-ID".into()));
    }
    let lizenz =
        s.db.platz_vergeben(&anfrage, s.e.ticket_dauer)
            .await?
            .map_err(ablehnung)?;
    let jetzt = Utc::now();
    let ticket = Ticket {
        lizenz: lizenz_fuer_ticket(&lizenz)?,
        machine_id: anfrage.machine_id.clone(),
        ausgestellt_um: jetzt,
        gueltig_bis: jetzt + s.e.ticket_dauer,
    };
    tracing::info!(lizenz = %lizenz.id, rechner = %anfrage.rechnername, "Ticket ausgestellt");
    Ok(Json(TicketAntwort {
        ticket: ticket_ausstellen(&ticket, &s.e.ticket_schluessel),
    }))
}

fn lizenz_fuer_ticket(l: &LizenzZeile) -> Ergebnis<Lizenz> {
    let lizenznehmer = if !l.lizenznehmer.is_empty() {
        l.lizenznehmer.clone()
    } else {
        l.email.clone().unwrap_or_else(|| "Lizenznehmer".into())
    };
    Ok(Lizenz {
        lizenz_id: l.id,
        lizenznehmer,
        edition: edition_aus_text(&l.edition)
            .ok_or_else(|| Fehler::Intern(format!("unbekannte Edition {}", l.edition)))?,
        arbeitsplaetze: u32::try_from(l.arbeitsplaetze).unwrap_or(1),
        ausgestellt_am: l.erstellt_am.date_naive(),
        gueltig_bis: l.gueltig_bis,
        wartung_bis: l.wartung_bis,
    })
}

async fn abmelden(
    State(s): State<AppState>,
    Json(anfrage): Json<AbmeldeAnfrage>,
) -> Ergebnis<StatusCode> {
    let schluessel = schluessel::normalisieren(&anfrage.lizenzschluessel);
    // Unbekannter Rechner ist kein Fehler: Der Platz ist dann schon frei.
    s.db.abmelden(&schluessel, anfrage.machine_id.trim())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Stripe -------------------------------------------------------------------

async fn stripe_webhook(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Ergebnis<StatusCode> {
    let Some(secret) = &s.e.stripe_webhook_secret else {
        return Err(Fehler::NichtGefunden);
    };
    let signatur = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if let Err(grund) = stripe::signatur_pruefen(signatur, &body, secret, Utc::now()) {
        tracing::warn!(grund, "Stripe-Webhook abgewiesen");
        return Err(Fehler::Anfrage(grund.into()));
    }
    let ereignis: Value =
        serde_json::from_slice(&body).map_err(|e| Fehler::Anfrage(e.to_string()))?;
    let id = ereignis["id"].as_str().unwrap_or_default();
    let typ = ereignis["type"].as_str().unwrap_or_default();
    let folge = stripe::auswerten(&ereignis);
    if matches!(folge, Folge::Nichts) {
        return Ok(StatusCode::OK);
    }
    let mut tx = s.db.pool.begin().await?;
    if !Db::ereignis_neu(&mut tx, id, typ).await? {
        return Ok(StatusCode::OK);
    }
    match &folge {
        Folge::Kauf(k) => Db::kauf(&mut tx, k, s.e.standard_edition).await?,
        Folge::Abo(a) => Db::abo(&mut tx, a, s.e.standard_edition).await?,
        Folge::Nichts => {}
    }
    tx.commit().await?;
    tracing::info!(id, typ, "Stripe-Ereignis verarbeitet");
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct KaufQuery {
    session_id: String,
}

/// Erfolgsseite nach dem Stripe-Checkout (`success_url` =
/// `https://…/kauf/erfolg?session_id={CHECKOUT_SESSION_ID}`). Kommt der
/// Kunde schneller an als der Webhook, lädt die Seite nach.
async fn kauf_erfolg(
    State(s): State<AppState>,
    Query(q): Query<KaufQuery>,
) -> Ergebnis<Html<String>> {
    let seite = |kopf: &str, inhalt: String| {
        Html(format!(
            "<!doctype html><html lang=\"de\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">{kopf}\
             <title>BauProjektIO – Lizenz</title>\
             <style>body{{font-family:system-ui,sans-serif;max-width:40rem;margin:3rem auto;padding:0 1rem;line-height:1.5}}\
             code{{font-size:1.3rem;padding:.4rem .6rem;background:#eee;border-radius:4px;user-select:all}}</style>\
             </head><body>{inhalt}</body></html>"
        ))
    };
    match s.db.lizenz_zu_checkout(&q.session_id).await? {
        Some(l) => Ok(seite(
            "",
            format!(
                "<h1>Vielen Dank!</h1><p>Ihr Lizenzschlüssel für {}:</p><p><code>{}</code></p>\
                 <p>In BauProjektIO unter <em>Hilfe → Lizenz …</em> eingeben (als Administrator). \
                 Alle Rechner an derselben Datenbank werden danach automatisch freigeschaltet, \
                 bis zu {} Arbeitsplätze.</p><p>Bitte bewahren Sie den Schlüssel auf.</p>",
                html(&l.lizenznehmer),
                html(&l.schluessel),
                l.arbeitsplaetze
            ),
        )),
        None => Ok(seite(
            "<meta http-equiv=\"refresh\" content=\"3\">",
            "<h1>Vielen Dank!</h1><p>Ihre Zahlung wird bestätigt. Diese Seite lädt gleich neu \
             und zeigt dann Ihren Lizenzschlüssel.</p>"
                .into(),
        )),
    }
}

fn html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// --- Verwaltung ---------------------------------------------------------------

/// Vergleicht über SHA-256, damit die Laufzeit nichts über den Token verrät.
fn token_pruefen(headers: &HeaderMap, erwartet: Option<&String>) -> Ergebnis<()> {
    let Some(erwartet) = erwartet.filter(|t| !t.is_empty()) else {
        // Ohne Token ist der Bereich abgeschaltet.
        return Err(Fehler::NichtGefunden);
    };
    let gesendet = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or_default();
    if Sha256::digest(gesendet.as_bytes()) == Sha256::digest(erwartet.as_bytes()) {
        Ok(())
    } else {
        Err(Fehler::NichtBerechtigt)
    }
}

async fn admin_liste(State(s): State<AppState>, headers: HeaderMap) -> Ergebnis<Json<Value>> {
    token_pruefen(&headers, s.e.admin_token.as_ref())?;
    Ok(Json(json!(s.db.lizenzen().await?)))
}

async fn admin_anlegen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(neu): Json<NeueLizenz>,
) -> Ergebnis<(StatusCode, Json<LizenzZeile>)> {
    token_pruefen(&headers, s.e.admin_token.as_ref())?;
    if neu.lizenznehmer.trim().is_empty() || neu.arbeitsplaetze == 0 {
        return Err(Fehler::Anfrage(
            "lizenznehmer und arbeitsplaetze (> 0) angeben".into(),
        ));
    }
    Ok((StatusCode::CREATED, Json(s.db.anlegen(&neu).await?)))
}

async fn admin_lizenz(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Ergebnis<Json<Value>> {
    token_pruefen(&headers, s.e.admin_token.as_ref())?;
    let lizenz = s.db.lizenz(id).await?.ok_or(Fehler::NichtGefunden)?;
    let aktivierungen = s.db.aktivierungen(id).await?;
    Ok(Json(
        json!({ "lizenz": lizenz, "aktivierungen": aktivierungen }),
    ))
}

async fn admin_aendern(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(a): Json<Lizenzaenderung>,
) -> Ergebnis<Json<LizenzZeile>> {
    token_pruefen(&headers, s.e.admin_token.as_ref())?;
    if a.arbeitsplaetze == Some(0) {
        return Err(Fehler::Anfrage(
            "arbeitsplaetze muss größer als 0 sein".into(),
        ));
    }
    Ok(Json(
        s.db.aendern(id, &a).await?.ok_or(Fehler::NichtGefunden)?,
    ))
}

async fn admin_aktivierung_loeschen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((id, machine_id)): Path<(Uuid, String)>,
) -> Ergebnis<StatusCode> {
    token_pruefen(&headers, s.e.admin_token.as_ref())?;
    if s.db.aktivierung_loeschen(id, &machine_id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Fehler::NichtGefunden)
    }
}

// --- Updates ------------------------------------------------------------------

/// Nur einfache relative Pfade: `latest.json`, `0.2.0/BauProjektIO_0.2.0_x64-setup.exe`.
fn update_pfad(basis: &FsPath, pfad: &str) -> Option<PathBuf> {
    let teile: Vec<&str> = pfad.split('/').collect();
    let erlaubt = |t: &&str| {
        !t.is_empty()
            && !t.starts_with('.')
            && t.len() <= 200
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
    };
    (teile.len() <= 3 && teile.iter().all(erlaubt))
        .then(|| teile.iter().fold(basis.to_path_buf(), |p, t| p.join(t)))
}

async fn update_hochladen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(pfad): Path<String>,
    body: Body,
) -> Ergebnis<StatusCode> {
    token_pruefen(&headers, s.e.upload_token.as_ref())?;
    let ziel = update_pfad(&s.e.update_dir, &pfad)
        .ok_or_else(|| Fehler::Anfrage(format!("ungültiger Pfad: {pfad}")))?;
    let intern = |e: std::io::Error| Fehler::Intern(format!("{}: {e}", ziel.display()));
    if let Some(dir) = ziel.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(intern)?;
    }
    // In eine temporäre Datei schreiben und erst am Ende umbenennen: Die App
    // sieht nie eine halb hochgeladene `latest.json` oder einen halben Installer.
    let tmp = ziel.with_file_name(format!(
        ".{}.{}.tmp",
        ziel.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload"),
        Uuid::new_v4()
    ));
    let mut datei = tokio::fs::File::create(&tmp).await.map_err(intern)?;
    let mut strom = body.into_data_stream();
    let ergebnis: Result<(), Fehler> = async {
        while let Some(stueck) = strom
            .try_next()
            .await
            .map_err(|e| Fehler::Anfrage(e.to_string()))?
        {
            datei.write_all(&stueck).await.map_err(intern)?;
        }
        datei.sync_all().await.map_err(intern)?;
        tokio::fs::rename(&tmp, &ziel).await.map_err(intern)
    }
    .await;
    if ergebnis.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    ergebnis?;
    tracing::info!(datei = %pfad, "Update-Datei hochgeladen");
    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_pfade() {
        let b = FsPath::new("/data/updates");
        assert_eq!(
            update_pfad(b, "latest.json").unwrap(),
            b.join("latest.json")
        );
        assert_eq!(
            update_pfad(b, "0.2.0/BauProjektIO_0.2.0_x64-setup.exe").unwrap(),
            b.join("0.2.0/BauProjektIO_0.2.0_x64-setup.exe")
        );
        for boese in [
            "../etc/passwd",
            "a/../../b",
            ".env",
            "a//b",
            "",
            "a b",
            "a/b/c/d",
        ] {
            assert!(update_pfad(b, boese).is_none(), "{boese}");
        }
    }
}
