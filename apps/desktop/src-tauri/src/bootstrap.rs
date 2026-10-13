//! Zusammenstecken der Abhängigkeiten je nach Betriebsart (Dependency Injection).

use std::path::Path;
use std::sync::Arc;

use client_cache::{CacheConfig, CachedStorage};
use core_lib::CoreServices;
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::storage::Storage;
use storage::StorageConfig;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::config::AppConfig;

pub struct AppState {
    pub core: CoreServices,
    pub config: AppConfig,
    /// Nur im Netzwerkmodus vorhanden.
    pub cache: Option<Arc<CachedStorage>>,
}

type BoxError = Box<dyn std::error::Error>;

pub async fn init(app: &AppHandle) -> Result<AppState, BoxError> {
    let paths = app.path();
    let config_dir = paths.app_config_dir()?;
    let data_dir = paths.app_data_dir()?;
    let cache_dir = paths.app_cache_dir()?;
    std::fs::create_dir_all(&config_dir)?;
    std::fs::create_dir_all(&data_dir)?;

    let config = AppConfig::load(&config_dir)?;
    tracing::info!(modus = config.name(), "BauProjektIO startet");

    let (repos, storage, cache): (_, Arc<dyn Storage>, _) = match &config {
        AppConfig::Einzelplatz => {
            let db = db_sqlite::SqliteDatabase::open(&data_dir.join("bauprojekt.db")).await?;
            let storage = storage::build(&StorageConfig::Local {
                root: data_dir.join("dateien"),
            })?;
            (db.repositories(), storage, None)
        }
        AppConfig::Netzwerk {
            database_url,
            storage: storage_config,
            cache_max_gb,
        } => {
            let db = db_postgres::PostgresDatabase::connect(database_url, 5).await?;
            let remote = storage::build(storage_config)?;
            let mut cache_config = CacheConfig::new(cache_dir.join("dateien"));
            cache_config.max_bytes = cache_max_gb.saturating_mul(1024 * 1024 * 1024);
            let cache = Arc::new(CachedStorage::open(remote, cache_config).await?);
            (
                db.repositories(),
                cache.clone() as Arc<dyn Storage>,
                Some(cache),
            )
        }
    };

    let lbo = Arc::new(init_lbo(app, &data_dir)?);

    // TODO: Lizenzprüfung (lokale Lizenzdatei / Lizenzserver) injizieren.
    let core = CoreServices::new(repos, storage, lbo, Arc::new(DevelopmentLicense));
    Ok(AppState {
        core,
        config,
        cache,
    })
}

/// LBO-Engine initialisieren: erst die mit dem Installer ausgelieferten
/// Regelpakete, danach nachgeladene Updates aus dem Datenverzeichnis. Ein
/// Update ersetzt das mitgelieferte Paket desselben Landes.
fn init_lbo(app: &AppHandle, data_dir: &Path) -> Result<LboEngine, BoxError> {
    let engine = LboEngine::new();
    let bundled = app.path().resolve("lbo-rules", BaseDirectory::Resource)?;
    let updates = data_dir.join("lbo-rules");
    std::fs::create_dir_all(&updates)?;
    for dir in [bundled, updates] {
        if !dir.is_dir() {
            tracing::warn!(dir = %dir.display(), "LBO-Verzeichnis fehlt");
            continue;
        }
        let (loaded, errors) = engine.load_directory(&dir)?;
        for info in loaded {
            tracing::info!(land = info.land.name(), version = %info.version, "LBO-Regelpaket aktiv");
        }
        for e in errors {
            // Ein defektes Update darf die App nicht blockieren; das zuvor
            // geladene Paket bleibt aktiv.
            tracing::error!(error = %e, "LBO-Regelpaket übersprungen");
        }
    }
    Ok(engine)
}
