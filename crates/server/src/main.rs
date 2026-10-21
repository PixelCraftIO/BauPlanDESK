//! Server für die NAS-/Büro- und Cloud-Variante.
//!
//! Konfiguration über Umgebungsvariablen:
//!
//! | Variable                   | Beispiel                                          |
//! |----------------------------|---------------------------------------------------|
//! | `BAUPROJEKT_DATABASE_URL`  | `postgres://user:pw@db/bauprojekt` oder `sqlite:///data/bp.db` |
//! | `BAUPROJEKT_STORAGE`       | `{"type":"nas","root":"/mnt/nas/bauprojekt"}`     |
//! | `BAUPROJEKT_LBO_DIR`       | `/opt/bauprojekt/lbo-rules`                       |
//! | `BAUPROJEKT_FORMULAR_DIR`  | `/opt/bauprojekt/formulare`                       |
//! | `BAUPROJEKT_CODELISTEN_DIR`| `/opt/bauprojekt/codelisten` (XBau, Genericode)   |
//! | `BAUPROJEKT_BIND`          | `0.0.0.0:8080`                                    |

use server::api;
mod zugang_setup;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use core_lib::formular::Formularkatalog;
use core_lib::lbo::LboEngine;
use core_lib::license::{DatenbankLizenz, Lizenzpruefer};
use core_lib::repository::Repositories;
use core_lib::xbau::Codelisten;
use core_lib::{CoreError, CoreResult, CoreServices};
use storage::StorageConfig;
use tracing_subscriber::EnvFilter;

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// Wählt die Datenbank-Implementierung anhand der URL – core-lib bekommt
/// davon nur die Trait-Objekte zu sehen.
async fn connect_database(url: &str) -> CoreResult<Repositories> {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Ok(db_postgres::PostgresDatabase::connect(url, 20)
            .await?
            .repositories())
    } else if let Some(path) = url.strip_prefix("sqlite://") {
        Ok(db_sqlite::SqliteDatabase::open(Path::new(path))
            .await?
            .repositories())
    } else {
        Err(CoreError::InvalidInput(format!(
            "nicht unterstützte Datenbank-URL: {url}"
        )))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let database_url = env_or("BAUPROJEKT_DATABASE_URL", "sqlite://bauprojekt-server.db");
    let storage_config: StorageConfig = serde_json::from_str(&env_or(
        "BAUPROJEKT_STORAGE",
        r#"{"type":"local","root":"bauprojekt-dateien"}"#,
    ))?;
    let lbo_dir = PathBuf::from(env_or("BAUPROJEKT_LBO_DIR", "lbo-rules"));
    let formular_dir = PathBuf::from(env_or("BAUPROJEKT_FORMULAR_DIR", "formulare"));
    let codelisten_dir = PathBuf::from(env_or("BAUPROJEKT_CODELISTEN_DIR", "codelisten"));
    let bind = env_or("BAUPROJEKT_BIND", "127.0.0.1:8080");

    let repos = connect_database(&database_url).await?;
    let storage = storage::build(&storage_config)?;

    let lbo = Arc::new(LboEngine::new());
    let (geladen, fehler) = lbo.load_directory(&lbo_dir)?;
    for e in fehler {
        tracing::error!(error = %e, "LBO-Regelpaket nicht geladen");
    }
    tracing::info!(anzahl = geladen.len(), dir = %lbo_dir.display(), "LBO-Regelpakete geladen");

    let mut formulare = Formularkatalog::default();
    for e in formulare.laden(&formular_dir)? {
        tracing::error!(error = %e, "Formularvorlage nicht geladen");
    }

    let mut codelisten = Codelisten::eingebaut();
    for e in codelisten.laden(&codelisten_dir)? {
        tracing::error!(error = %e, "Codeliste nicht geladen");
    }

    // Lizenz liegt in der Datenbank (gemeinsam mit den Desktop-Arbeitsplätzen).
    // Arbeitsplätze belegt die Desktop-App; der Server selbst zählt nicht mit.
    let lizenz = DatenbankLizenz::new(repos.lizenz.clone(), Lizenzpruefer::standard());
    if !Lizenzpruefer::standard().hat_schluessel() {
        tracing::warn!("Kein Lizenzschlüssel eingebaut: nur die Testversion ist möglich.");
    }
    let core = CoreServices::new(repos, storage, lbo, Arc::new(lizenz))
        .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard())
        .mit_formularen(formulare)
        .mit_codelisten(codelisten);

    if zugang_setup::ausfuehren(&core, &std::env::args().collect::<Vec<_>>()).await? {
        return Ok(());
    }
    // Ohne verifizierte Zugänge wird kein Projektserver geöffnet.
    let zugangspfad=std::env::var("BAUPROJEKT_ZUGANGSDATEI").map(PathBuf::from).map_err(|_|"BAUPROJEKT_ZUGANGSDATEI fehlt. Der Projektserver startet ohne konfigurierte Anmeldung nicht.")?;
    let raw = tokio::fs::read(&zugangspfad).await?;
    let z: api::zugang::Zugaenge = serde_json::from_slice(&raw)?;
    z.pruefen()?;
    if z.zugaenge.is_empty() {
        return Err("Gültige Zugangsliste mit mindestens einem Benutzer erforderlich".into());
    }
    // Technisches Konto nur für interne Migration, niemals als HTTP-Identität.
    let akteur = core.systembenutzer().await?;

    // Bauherren aus Bauanträgen älterer Versionen in die Kundendatenbank.
    match core.bauherren_uebernehmen(&akteur).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(projekte = n, "Bauherren in die Kundendatenbank übernommen"),
        // Speichern des Bauantrags übernimmt je Projekt erneut.
        Err(e) => tracing::error!(fehler = %e, "Bauherren nicht übernommen"),
    }

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "BauPlanIO-Server gestartet");
    axum::serve(
        listener,
        api::zugang::gesichert(core, api::zugang::ZugangQuelle::Datei(zugangspfad)),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
