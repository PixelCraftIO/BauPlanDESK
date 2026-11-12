//! Sicherung des Einzelplatzprofils; Wiederherstellung erzeugt ein getrenntes Profil.
use crate::{
    bootstrap::AppState,
    commands::CmdError,
    config::{AppConfig, DateiablageConfig, ProfilAuswahl},
};
use core_lib::{CoreError, benutzer::Recht};
use serde::Serialize;
use std::path::{Path, PathBuf};
use storage::sicherung::{Sicherungsmanifest, Sicherungsquelle};
use tauri::{AppHandle, Manager, State};
async fn verwalten(state: &AppState) -> Result<(), CmdError> {
    Ok(state
        .core
        .berechtigung_pruefen(&state.benutzer(), Recht::BenutzerVerwalten)
        .await?)
}
#[derive(Serialize)]
pub struct Sicherungsstatus {
    lokal: bool,
    profil: PathBuf,
    hinweis: &'static str,
}
#[tauri::command]
pub fn sicherungsstatus(state: State<'_, AppState>) -> Sicherungsstatus {
    Sicherungsstatus {
        lokal: matches!(state.config, AppConfig::Einzelplatz),
        profil: state.data_dir.clone(),
        hinweis: if matches!(state.config, AppConfig::Einzelplatz) {
            "Sichert Datenbank, sämtliche DMS-Versionen, Formularbibliothek und lokale Regeln/Einstellungen. Projektordner sind wiedererzeugbare Ausgaben. Lokale Arbeitskopien zuerst übernehmen."
        } else {
            "Im Büro-/Servermodus können PostgreSQL und eingebundene NAS-/Dateiablage gemeinsam gesichert und getrennt wiederhergestellt werden. S3 benötigt die zentrale Betreiber-Sicherung. Lokale Arbeitskopien zuvor übernehmen."
        },
    }
}
#[tauri::command]
pub async fn sicherung_erstellen(
    app: AppHandle,
    state: State<'_, AppState>,
    ziel: PathBuf,
) -> Result<Sicherungsmanifest, CmdError> {
    verwalten(&state).await?;
    let ablage = state.lokale_ablage.as_ref().ok_or_else(|| {
        CoreError::InvalidInput("Zentrale Bürosicherung durch den Serverbetreiber verwenden".into())
    })?;
    let dmsroot = ablage.root().await;
    let scratch = app
        .path()
        .app_cache_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?
        .join(format!("sicherung-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&scratch)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    let result = async {
        let db = db_sqlite::SqliteDatabase::open(&state.data_dir.join("bauprojekt.db")).await?;
        let snapshot = scratch.join("bauprojekt.db");
        db.sichern(&snapshot).await?;
        let kopie = db_sqlite::SqliteDatabase::open(&snapshot).await?;
        kopie.integritaet_pruefen().await?;
        let repos = kopie.repositories();
        let erwartet = storage::sicherung::dms_referenzen(&repos).await;
        drop(repos);
        kopie.schliessen().await;
        let mut quellen = vec![Sicherungsquelle {
            name: "bauprojekt.db".into(),
            pfad: snapshot,
            erwartet: None,
        }];
        // Abgleich über den Storage-Schlüssel unter `data/`; `tmp/` bleibt außen vor.
        quellen.extend(storage::sicherung::dateiablage(
            &dmsroot, "dateien", erwartet?,
        )?);
        let formularroot = state.formularordner.read().await.clone();
        if formularroot.exists() {
            quellen.extend(storage::sicherung::verzeichnis(
                &formularroot,
                "Formularvorlagen",
            )?);
        }
        if state.config_dir.exists() {
            quellen.extend(storage::sicherung::verzeichnis(
                &state.config_dir,
                "einstellungen",
            )?);
        }
        for ordner in ["lbo-rules", "formulare", "codelisten"] {
            let pfad = state.data_dir.join(ordner);
            if pfad.exists() {
                quellen.extend(storage::sicherung::verzeichnis(&pfad, ordner)?);
            }
        }
        tauri::async_runtime::spawn_blocking(move || {
            let m = storage::sicherung::erstellen(&ziel, &quellen)?;
            storage::sicherung::pruefen(&ziel)?;
            Ok::<_, CoreError>(m)
        })
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?
    }
    .await;
    let _ = tokio::fs::remove_dir_all(scratch).await;
    Ok(result?)
}
#[tauri::command]
pub async fn sicherung_pruefen(
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<Sicherungsmanifest, CmdError> {
    verwalten(&state).await?;
    Ok(
        tauri::async_runtime::spawn_blocking(move || storage::sicherung::pruefen(&pfad))
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))??,
    )
}
#[tauri::command]
pub async fn sicherung_wiederherstellen(
    state: State<'_, AppState>,
    pfad: PathBuf,
    ziel: PathBuf,
) -> Result<PathBuf, CmdError> {
    verwalten(&state).await?;
    if !ziel.is_absolute() {
        return Err(
            CoreError::InvalidInput("Absolutes neues Profilverzeichnis wählen".into()).into(),
        );
    }
    let neu = ziel.clone();
    tauri::async_runtime::spawn_blocking(move || storage::sicherung::wiederherstellen(&pfad, &neu))
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
    // Erst nach Integritätsprüfung werden absolute Pfade auf das neue Profil gesetzt.
    let result = profil_einrichten(&ziel).await;
    if let Err(e) = result {
        return Err(CoreError::Storage(format!("Daten wurden in {} entpackt, das Profil wurde wegen eines Prüffehlers nicht aktiviert: {e}",ziel.display())).into());
    }
    Ok(ziel)
}
/// Richtet ein frisch entpacktes Sicherungsverzeichnis als Einzelplatzprofil
/// ein: Datenbank prüfen, Rechnerkonto als Verwaltung übernehmen und Ablagen
/// auf das neue Verzeichnis setzen.
pub(crate) async fn profil_einrichten(ziel: &Path) -> Result<(), CoreError> {
    db_sqlite::SqliteDatabase::datei_pruefen(&ziel.join("bauprojekt.db")).await?;
    // Das wiederhergestellte Profil startet als Einzelplatz mit Anmeldung
    // über das Rechnerkonto; wer wiederherstellt, verwaltet es auch dort.
    let db = db_sqlite::SqliteDatabase::open(&ziel.join("bauprojekt.db")).await?;
    let uebernahme = core_lib::anmeldung::verwaltung_nach_wiederherstellung(
        &db.repositories(),
        &whoami::username(),
        &whoami::realname(),
    )
    .await;
    db.schliessen().await;
    uebernahme?;
    let config = ziel.join("einstellungen");
    tokio::fs::create_dir_all(&config)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    for (datei, ordner) in [
        ("dateiablage.json", "dateien"),
        ("formularbibliothek.json", "Formularvorlagen"),
        ("projektordner.json", "Projektordner"),
    ] {
        tokio::fs::create_dir_all(ziel.join(ordner))
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        DateiablageConfig {
            root: ziel.join(ordner),
        }
        .save_named(&config, datei)
        .map_err(CoreError::Storage)?;
    }
    tokio::fs::write(config.join("config.json"), b"{\"modus\":\"einzelplatz\"}")
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[derive(Serialize)]
pub struct UebernommenesProjekt {
    id: uuid::Uuid,
    name: String,
}
/// Übernimmt die Projekte einer Einzelplatzsicherung (z. B. des
/// Beispielprojekts) in das laufende Profil, ohne Profilwechsel und Neustart.
/// Eigene Projekte, Kunden, Benutzer und Büroangaben bleiben unverändert;
/// gibt es ein Projekt schon, wird nichts übernommen.
#[tauri::command]
pub async fn projekte_aus_sicherung_uebernehmen(
    app: AppHandle,
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<Vec<UebernommenesProjekt>, CmdError> {
    verwalten(&state).await?;
    if state.lokale_ablage.is_none() {
        return Err(CoreError::InvalidInput(
            "Projekte aus einer Sicherung lassen sich nur im Einzelplatzbetrieb übernehmen.".into(),
        )
        .into());
    }
    let arbeit = app
        .path()
        .app_cache_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?
        .join(format!("uebernahme-{}", uuid::Uuid::new_v4()));
    let result = async {
        let ziel = arbeit.clone();
        tauri::async_runtime::spawn_blocking(move || {
            storage::sicherung::wiederherstellen(&pfad, &ziel)
        })
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
        // Öffnen bringt eine ältere Sicherung auf den aktuellen Datenbankstand.
        let quelle = db_sqlite::SqliteDatabase::open(&arbeit.join("bauprojekt.db")).await?;
        let repos = quelle.repositories();
        let erwartet = storage::sicherung::dms_referenzen(&repos).await;
        let projekte = repos.projects.list().await;
        drop(repos);
        quelle.schliessen().await;
        let projekte = projekte?;
        let erwartet = erwartet?;
        for p in &projekte {
            if state.core.repos.projects.get(p.id).await.is_ok() {
                return Err(CoreError::InvalidInput(format!(
                    "Bereits vorhanden: „{}“. Zum erneuten Übernehmen das Projekt zuerst löschen.",
                    p.name
                )));
            }
        }
        // Erst die Dateien, dann die Datenbank: Sie verweist nie auf Fehlendes.
        let ids: Vec<_> = projekte.iter().map(|p| p.id.0).collect();
        storage::projektuebernahme::dateien_uebernehmen(
            &arbeit.join("dateien"),
            state.core.storage.as_ref(),
            &ids,
            &erwartet,
        )
        .await?;
        let db = db_sqlite::SqliteDatabase::open(&state.data_dir.join("bauprojekt.db")).await?;
        let uebernommen: Result<_, CoreError> =
            db.projekte_uebernehmen(&arbeit.join("bauprojekt.db")).await;
        db.schliessen().await;
        uebernommen
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&arbeit).await;
    for p in result.as_deref().unwrap_or_default() {
        tracing::info!(projekt = %p.id, name = %p.name, "Projekt aus Sicherung übernommen");
    }
    Ok(result?
        .into_iter()
        .map(|p| UebernommenesProjekt {
            id: p.id,
            name: p.name,
        })
        .collect())
}
#[tauri::command]
pub async fn profil_aktivieren(
    app: AppHandle,
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<(), CmdError> {
    verwalten(&state).await?;
    if !pfad.is_absolute() || !Path::new(&pfad).join("einstellungen/config.json").is_file() {
        return Err(
            CoreError::InvalidInput("Gültiges wiederhergestelltes Profil wählen".into()).into(),
        );
    }
    match AppConfig::load(&pfad.join("einstellungen")).map_err(CoreError::Storage)? {
        AppConfig::Einzelplatz => {
            db_sqlite::SqliteDatabase::datei_pruefen(&pfad.join("bauprojekt.db")).await?
        }
        AppConfig::Netzwerk { database_url, .. } => {
            if !pfad.join("postgres.dump").is_file() {
                return Err(
                    CoreError::InvalidInput("Kein wiederhergestelltes Büroprofil".into()).into(),
                );
            }
            let db = db_postgres::PostgresDatabase::connect(&database_url, 1).await?;
            let _ = db.repositories().projects.list().await?;
        }
    }
    let global = app
        .path()
        .app_config_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    ProfilAuswahl {
        schema_version: 1,
        pfad,
    }
    .speichern(&global)
    .map_err(CoreError::Storage)?;
    Ok(())
}
#[tauri::command]
pub async fn standardprofil_aktivieren(app: AppHandle) -> Result<(), CmdError> {
    let p = app
        .path()
        .app_config_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?
        .join("profil.json");
    match tokio::fs::remove_file(p).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CoreError::Storage(e.to_string()).into()),
    }
}

#[tauri::command]
pub async fn buero_sicherung_erstellen(
    state: State<'_, AppState>,
    ziel: PathBuf,
) -> Result<Sicherungsmanifest, CmdError> {
    verwalten(&state).await?;
    let (url,root)=match &state.config {AppConfig::Netzwerk{database_url,storage:storage::StorageConfig::Nas{root}|storage::StorageConfig::Local{root},..}=>(database_url.clone(),root.clone()),_=>return Err(CoreError::InvalidInput("Bürosicherung benötigt PostgreSQL und eine eingebundene NAS-/Dateiablage. S3-Ablagen über die zentrale Betreiber-Sicherung sichern.".into()).into())};
    let config = state.config_dir.clone();
    let vorlagen = state.formularordner.read().await.clone();
    tauri::async_runtime::spawn_blocking(move || {
        storage::buero_sicherung::erstellen(&url, &root, &config, Some(&vorlagen), &ziel)
    })
    .await
    .map_err(|e| CoreError::Storage(e.to_string()))?
    .map_err(Into::into)
}
#[tauri::command]
pub async fn buero_sicherung_wiederherstellen(
    state: State<'_, AppState>,
    pfad: PathBuf,
    ziel: PathBuf,
    database_url: String,
) -> Result<PathBuf, CmdError> {
    verwalten(&state).await?;
    tauri::async_runtime::spawn_blocking(move || {
        storage::buero_sicherung::wiederherstellen(&pfad, &ziel, &database_url)
    })
    .await
    .map_err(|e| CoreError::Storage(e.to_string()))?
    .map_err(Into::into)
}

async fn ablagewurzel(state: &AppState) -> Result<PathBuf, CmdError> {
    verwalten(state).await?;
    if let Some(a) = &state.lokale_ablage {
        return Ok(a.root().await);
    }
    match &state.config {
        AppConfig::Netzwerk {
            storage: storage::StorageConfig::Nas { root } | storage::StorageConfig::Local { root },
            ..
        } => Ok(root.clone()),
        _ => Err(CoreError::InvalidInput(
            "Diese Prüfung benötigt eine lokale oder eingebundene NAS-Ablage".into(),
        )
        .into()),
    }
}
#[tauri::command]
pub async fn ablage_konsistenz(
    state: State<'_, AppState>,
) -> Result<storage::konsistenz::Bericht, CmdError> {
    let root = ablagewurzel(&state).await?;
    let refs = storage::sicherung::dms_referenzen(&state.core.repos).await?;
    Ok(
        tauri::async_runtime::spawn_blocking(move || storage::konsistenz::pruefen(&root, &refs))
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))??,
    )
}
#[tauri::command]
pub async fn ablage_metadaten_ergaenzen(state: State<'_, AppState>) -> Result<usize, CmdError> {
    let root = ablagewurzel(&state).await?;
    let refs = storage::sicherung::dms_referenzen(&state.core.repos).await?;
    Ok(storage::konsistenz::metadaten_ergaenzen(root, refs).await?)
}
