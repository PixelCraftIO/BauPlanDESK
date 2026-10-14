//! Zusammenstecken der Abhängigkeiten je nach Betriebsart (Dependency Injection).

use std::path::Path;
use std::sync::{Arc, RwLock};

use client_cache::{CacheConfig, CachedStorage};
use core_lib::CoreServices;
use core_lib::benutzer::Benutzer;
use core_lib::formular::Formularkatalog;
use core_lib::lbo::LboEngine;
use core_lib::license::{AboAnbindung, DatenbankLizenz, Lizenzpruefer};
use core_lib::storage::Storage;
use core_lib::xbau::Codelisten;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::config::AppConfig;
use crate::lizenzdienst::{self, HttpLizenzdienst};

pub struct AppState {
    pub core: CoreServices,
    /// Angemeldeter Benutzer (Betriebssystem-Anmeldung). Wird neu gelesen,
    /// wenn ein Administrator ihn ändert.
    pub benutzer: RwLock<Benutzer>,
    /// Name dieses Rechners für die Zählung der Lizenz-Arbeitsplätze.
    pub arbeitsplatz: String,
    pub config: AppConfig,
    /// Nur im Netzwerkmodus vorhanden.
    pub cache: Option<Arc<CachedStorage>>,
    pub lokale_ablage: Option<Arc<storage::WechselbareAblage>>,
}

impl AppState {
    /// Kopie des angemeldeten Benutzers für einen Aufruf in core-lib.
    pub fn benutzer(&self) -> Benutzer {
        self.benutzer
            .read()
            .expect("Benutzer-Lock vergiftet")
            .clone()
    }
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

    let (repos, storage, cache, lokale_ablage): (_, Arc<dyn Storage>, _, _) = match &config {
        AppConfig::Einzelplatz => {
            let db = db_sqlite::SqliteDatabase::open(&data_dir.join("bauprojekt.db")).await?;
            let auswahl =
                crate::config::DateiablageConfig::load(&config_dir, data_dir.join("dateien"))?;
            let ablage = Arc::new(storage::WechselbareAblage::new(auswahl.root)?);
            (
                db.repositories(),
                ablage.clone() as Arc<dyn Storage>,
                None,
                Some(ablage),
            )
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
                None,
            )
        }
    };

    let lbo = Arc::new(init_lbo(app, &data_dir)?);

    // Lizenz und belegte Arbeitsplätze liegen in der (gemeinsamen) Datenbank.
    let pruefer = Lizenzpruefer::standard();
    if !pruefer.hat_schluessel() {
        tracing::warn!("Kein Lizenzschlüssel eingebaut: nur die Testversion ist möglich.");
    }
    let hat_ticket_schluessel = pruefer.hat_ticket_schluessel();
    let mut lizenz = DatenbankLizenz::new(repos.lizenz.clone(), pruefer).mit_arbeitsplatzpflicht();
    let arbeitsplatz = whoami::fallible::hostname().unwrap_or_else(|_| whoami::devicename());
    // Abo-Lizenzen: nur mit eingebautem Ticket-Schlüssel und Lizenzserver.
    match (hat_ticket_schluessel, lizenzdienst::server_url()) {
        (true, Some(url)) => match lizenzdienst::machine_id() {
            Ok(machine_id) => {
                tracing::info!(%url, "Lizenzserver");
                lizenz = lizenz.mit_abo(AboAnbindung {
                    dienst: Arc::new(HttpLizenzdienst::new(url)?),
                    machine_id,
                    rechnername: arbeitsplatz.clone(),
                    app_version: app.package_info().version.to_string(),
                    ticket_datei: data_dir.join("lizenz.ticket"),
                });
            }
            Err(e) => tracing::error!(fehler = %e, "Abo-Lizenzen nicht möglich"),
        },
        _ => tracing::info!("Kein Lizenzserver eingebaut: nur Lizenzdateien möglich."),
    }
    let core = CoreServices::new(repos, storage, lbo, Arc::new(lizenz))
        .mit_formularen(init_formulare(app, &data_dir)?)
        .mit_codelisten(init_codelisten(app, &data_dir)?);
    // Anmeldung über das Betriebssystem. Im Netzwerkmodus teilen sich alle
    // Arbeitsplätze die Benutzertabelle; die erste Anmeldung wird Administrator.
    let benutzer = core
        .anmelden(&whoami::username(), &whoami::realname())
        .await?;
    tracing::info!(benutzer = %benutzer.anmeldename, rolle = ?benutzer.rolle, "angemeldet");
    // Bauherren aus Bauanträgen älterer Versionen in die Kundendatenbank.
    match core.bauherren_uebernehmen(&benutzer).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(projekte = n, "Bauherren in die Kundendatenbank übernommen"),
        // Kein Grund, die App nicht zu starten: Speichern des Bauantrags
        // übernimmt je Projekt erneut.
        Err(e) => tracing::error!(fehler = %e, "Bauherren nicht übernommen"),
    }
    let lizenz = core
        .license
        .arbeitsplatz_belegen(&arbeitsplatz, &benutzer.anzeigename)
        .await?;
    tracing::info!(%arbeitsplatz, zustand = ?lizenz.zustand, "Lizenz geprüft");
    Ok(AppState {
        core,
        benutzer: RwLock::new(benutzer),
        arbeitsplatz,
        config,
        cache,
        lokale_ablage,
    })
}

/// Formularvorlagen: mitgelieferte zuerst, dann Updates aus dem Datenverzeichnis.
fn init_formulare(app: &AppHandle, data_dir: &Path) -> Result<Formularkatalog, BoxError> {
    let mut katalog = Formularkatalog::default();
    let bundled = app.path().resolve("formulare", BaseDirectory::Resource)?;
    let updates = data_dir.join("formulare");
    std::fs::create_dir_all(&updates)?;
    for dir in [bundled, updates] {
        if !dir.is_dir() {
            continue;
        }
        for e in katalog.laden(&dir)? {
            tracing::error!(error = %e, "Formularvorlage übersprungen");
        }
    }
    tracing::info!(
        anzahl = katalog.vorlagen().len(),
        "Formularvorlagen geladen"
    );
    Ok(katalog)
}

/// XBau-Codelisten: eingebaute, mitgelieferte, dann Updates aus dem
/// Datenverzeichnis (gleiche Kennung ersetzt).
fn init_codelisten(app: &AppHandle, data_dir: &Path) -> Result<Codelisten, BoxError> {
    let mut codelisten = Codelisten::eingebaut();
    let bundled = app.path().resolve("codelisten", BaseDirectory::Resource)?;
    let updates = data_dir.join("codelisten");
    std::fs::create_dir_all(&updates)?;
    for dir in [bundled, updates] {
        for e in codelisten.laden(&dir)? {
            tracing::error!(error = %e, "Codeliste übersprungen");
        }
    }
    tracing::info!(
        listen = ?codelisten.alle().map(|l| format!("{} {}", l.uri, l.version)).collect::<Vec<_>>(),
        "XBau-Codelisten geladen"
    );
    Ok(codelisten)
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
