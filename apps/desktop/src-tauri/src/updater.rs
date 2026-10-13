//! Auto-Update über das Tauri-Updater-Plugin.
//!
//! Ablauf: Beim Start wird im Hintergrund geprüft; ist ein Update da, erhält
//! das Frontend das Event `update-verfuegbar` und kann den Nutzer fragen.
//! Installiert wird erst auf ausdrücklichen Wunsch (`update_installieren`),
//! damit niemand mitten in einem Bauantrag neu gestartet wird. Pakete sind
//! mit dem privaten Updater-Schlüssel signiert; der öffentliche Schlüssel
//! steht in `tauri.conf.json`.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

use crate::commands::CmdError;

#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    version: String,
    notizen: Option<String>,
}

fn update_err(e: impl std::fmt::Display) -> CmdError {
    CmdError {
        art: "update",
        meldung: e.to_string(),
    }
}

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, CmdError> {
    let update = app
        .updater()
        .map_err(update_err)?
        .check()
        .await
        .map_err(update_err)?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        notizen: u.body.clone(),
    });
    *app.state::<PendingUpdate>().0.lock().await = update;
    Ok(info)
}

pub fn check_in_background(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        match check(&app).await {
            Ok(Some(info)) => {
                let _ = app.emit("update-verfuegbar", info);
            }
            Ok(None) => tracing::info!("keine Aktualisierung verfügbar"),
            // Offline oder Update-Server nicht erreichbar: kein Fehler für den Nutzer.
            Err(e) => tracing::warn!(fehler = %e.meldung, "Update-Prüfung fehlgeschlagen"),
        }
    });
}

#[tauri::command]
pub async fn update_pruefen(app: AppHandle) -> Result<Option<UpdateInfo>, CmdError> {
    check(&app).await
}

#[tauri::command]
pub async fn update_installieren(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<(), CmdError> {
    let Some(update) = pending.0.lock().await.take() else {
        return Err(update_err("kein Update vorgemerkt"));
    };
    let mut geladen = 0usize;
    update
        .download_and_install(
            |chunk, gesamt| {
                geladen += chunk;
                let _ = app.emit("update-fortschritt", (geladen, gesamt));
            },
            || tracing::info!("Update heruntergeladen, Installation startet"),
        )
        .await
        .map_err(update_err)?;
    app.restart();
}
