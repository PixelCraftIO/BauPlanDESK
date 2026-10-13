//! Lizenzserver starten. Konfiguration über Umgebungsvariablen:
//!
//! | Variable                         | Pflicht | Beispiel / Standard |
//! |----------------------------------|---------|---------------------|
//! | `BAUPROJEKT_DATABASE_URL`        | ja      | `postgres://lizenz:pw@db:5432/lizenz` |
//! | `BAUPROJEKT_TICKET_GEHEIM`       | ja      | geheimer Ticket-Schlüssel (Base64), von `lizenz-werkzeug schluessel` |
//! | `BAUPROJEKT_TICKET_TAGE`         | nein    | `30` (so lange läuft die App offline) |
//! | `STRIPE_WEBHOOK_SECRET`          | nein    | `whsec_…`; ohne ist der Webhook abgeschaltet |
//! | `BAUPROJEKT_ADMIN_TOKEN`         | nein    | langer Zufallswert; ohne ist `/admin` abgeschaltet |
//! | `BAUPROJEKT_UPLOAD_TOKEN`        | nein    | langer Zufallswert; ohne ist der Upload abgeschaltet |
//! | `BAUPROJEKT_UPDATE_DIR`          | nein    | `/data/updates` |
//! | `BAUPROJEKT_STANDARD_EDITION`    | nein    | `buero` (für Stripe-Käufe ohne Metadatum `edition`) |
//! | `BAUPROJEKT_BIND`                | nein    | `0.0.0.0:8080` |

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chrono::Duration;
use core_lib::license::Edition;
use core_lib::license::datei::oeffentlich_base64;
use ed25519_dalek::SigningKey;
use lizenz_server::db::{Db, edition_aus_text};
use lizenz_server::{AppState, Einstellungen, router};
use tracing_subscriber::EnvFilter;

type Fehler = Box<dyn std::error::Error>;

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn pflicht(name: &str) -> Result<String, Fehler> {
    env(name).ok_or_else(|| format!("Umgebungsvariable {name} fehlt").into())
}

fn geheimer_schluessel(base64: &str) -> Result<SigningKey, Fehler> {
    let bytes: [u8; 32] = B64
        .decode(base64)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or("BAUPROJEKT_TICKET_GEHEIM ist kein Base64-Schlüssel mit 32 Byte")?;
    Ok(SigningKey::from_bytes(&bytes))
}

#[tokio::main]
async fn main() -> Result<(), Fehler> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let ticket_schluessel = geheimer_schluessel(&pflicht("BAUPROJEKT_TICKET_GEHEIM")?)?;
    let tage: i64 = env("BAUPROJEKT_TICKET_TAGE")
        .map(|t| t.parse())
        .transpose()?
        .unwrap_or(30);
    let standard_edition = match env("BAUPROJEKT_STANDARD_EDITION") {
        Some(e) => edition_aus_text(&e)
            .ok_or("BAUPROJEKT_STANDARD_EDITION: einzelplatz, buero oder cloud")?,
        None => Edition::Buero,
    };
    let e = Einstellungen {
        ticket_schluessel,
        ticket_dauer: Duration::days(tage),
        stripe_webhook_secret: env("STRIPE_WEBHOOK_SECRET"),
        admin_token: env("BAUPROJEKT_ADMIN_TOKEN"),
        upload_token: env("BAUPROJEKT_UPLOAD_TOKEN"),
        update_dir: env("BAUPROJEKT_UPDATE_DIR")
            .unwrap_or_else(|| "/data/updates".into())
            .into(),
        standard_edition,
    };
    tokio::fs::create_dir_all(&e.update_dir).await?;
    tracing::info!(
        ticket_schluessel = %oeffentlich_base64(&e.ticket_schluessel),
        ticket_tage = tage,
        stripe = e.stripe_webhook_secret.is_some(),
        admin = e.admin_token.is_some(),
        upload = e.upload_token.is_some(),
        updates = %e.update_dir.display(),
        "Einstellungen (öffentlicher Ticket-Schlüssel gehört in BAUPROJEKT_TICKET_SCHLUESSEL)"
    );

    let db = Db::verbinden(&pflicht("BAUPROJEKT_DATABASE_URL")?).await?;
    let bind = env("BAUPROJEKT_BIND").unwrap_or_else(|| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "Lizenzserver gestartet");
    axum::serve(listener, router(AppState { db, e: Arc::new(e) }))
        .with_graceful_shutdown(beenden())
        .await?;
    Ok(())
}

/// Strg+C oder SIGTERM (so stoppt Docker bzw. Coolify den Container).
async fn beenden() {
    let strg_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        () = strg_c => {}
        () = term => {}
    }
}
