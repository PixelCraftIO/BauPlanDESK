//! Sicherung des Einzelplatzprofils; Wiederherstellung erzeugt ein getrenntes Profil.
use crate::{
    bootstrap::AppState,
    commands::CmdError,
    config::{AppConfig, DateiablageConfig, ProfilAuswahl},
};
use core_lib::{CoreError, benutzer::Recht, storage::ContentHash};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
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
            "Im Büro-/Servermodus müssen Datenbank und zentrale Dateiablage gemeinsam durch den Betreiber gesichert werden. Die lokale Cachekopie ist keine vollständige Bürosicherung."
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
    let result=async {
        let db=db_sqlite::SqliteDatabase::open(&state.data_dir.join("bauprojekt.db")).await?;
        let snapshot=scratch.join("bauprojekt.db");db.sichern(&snapshot).await?;
        let kopie=db_sqlite::SqliteDatabase::open(&snapshot).await?;kopie.integritaet_pruefen().await?;
        let repos=kopie.repositories();let mut erwartet=BTreeMap::<String,ContentHash>::new();
        for p in repos.projects.list().await? {
            for d in repos.documents.list_for_project(p.id).await? {
                erwartet.insert(d.storage_key.clone(),d.sha256);
                if let Some(z)=repos.commit.dokumentzusatz(d.document_id,d.version,"formularinstanz").await? {
                    if z.schema_version!=1{return Err(CoreError::InvalidInput("Unbekannte Formularinstanzversion; Sicherung mit neuerer Anwendung erstellen".into()));}
                    let i:core_lib::formular::instanz::FormularInstanz=serde_json::from_value(z.daten).map_err(|e|CoreError::InvalidInput(e.to_string()))?;
                    erwartet.insert(i.vorlagen_storage_key,i.vorlagen_hash);
                }
            }
            for e in repos.einreichungen.auflisten(p.id).await? {erwartet.insert(e.zip_key,e.zip_hash);}
        }
        drop(repos);kopie.schliessen().await;
        let mut quellen=vec![Sicherungsquelle{name:"bauprojekt.db".into(),pfad:snapshot,erwartet:None}];
        let mut dateien=storage::sicherung::verzeichnis(&dmsroot,"dateien")?;
        for q in &mut dateien {if let Some(h)=erwartet.remove(q.name.trim_start_matches("dateien/")){q.erwartet=Some(h);}}
        if !erwartet.is_empty(){return Err(CoreError::Storage(format!("{} Datenbankreferenzen fehlen in der Dateiablage. Die Sicherung wird nicht als vollständig ausgegeben.",erwartet.len())));}
        quellen.extend(dateien);
        let formularroot=state.formularordner.read().await.clone();
        if formularroot.exists(){quellen.extend(storage::sicherung::verzeichnis(&formularroot,"Formularvorlagen")?);}
        if state.config_dir.exists(){quellen.extend(storage::sicherung::verzeichnis(&state.config_dir,"einstellungen")?);}
        for ordner in ["lbo-rules","formulare","codelisten"] {let pfad=state.data_dir.join(ordner);if pfad.exists(){quellen.extend(storage::sicherung::verzeichnis(&pfad,ordner)?);}}
        tauri::async_runtime::spawn_blocking(move||{let m=storage::sicherung::erstellen(&ziel,&quellen)?;storage::sicherung::pruefen(&ziel)?;Ok::<_,CoreError>(m)}).await.map_err(|e|CoreError::Storage(e.to_string()))?
    }.await;
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
    let result = async {
        db_sqlite::SqliteDatabase::datei_pruefen(&ziel.join("bauprojekt.db")).await?;
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
        Ok::<_, CoreError>(())
    }
    .await;
    if let Err(e) = result {
        return Err(CoreError::Storage(format!("Daten wurden in {} entpackt, das Profil wurde wegen eines Prüffehlers nicht aktiviert: {e}",ziel.display())).into());
    }
    Ok(ziel)
}
#[tauri::command]
pub async fn profil_aktivieren(
    app: AppHandle,
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<(), CmdError> {
    verwalten(&state).await?;
    db_sqlite::SqliteDatabase::datei_pruefen(&pfad.join("bauprojekt.db")).await?;
    if !pfad.is_absolute() || !Path::new(&pfad).join("einstellungen/config.json").is_file() {
        return Err(
            CoreError::InvalidInput("Gültiges wiederhergestelltes Profil wählen".into()).into(),
        );
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
