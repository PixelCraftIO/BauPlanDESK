//! Signierte Updates; Prüfung und Installation werden serialisiert. Der
//! öffentliche Schlüssel und die HTTPS-Quelle werden beim Build festgelegt.
use crate::commands::CmdError;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

const GITHUB: &str =
    "https://github.com/PixelCraftIO/BauPlanDESK/releases/latest/download/latest.json";
#[derive(Default)]
pub struct PendingUpdate {
    update: Mutex<Option<Update>>,
    operation: Mutex<()>,
}
#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    version: String,
    notizen: Option<String>,
}
#[derive(Serialize)]
pub struct UpdateStatus {
    eingerichtet: bool,
    verfuegbar: Option<UpdateInfo>,
    quelle: String,
    hinweis: String,
}
#[derive(Clone, Serialize)]
struct Fortschritt {
    geladen: u64,
    gesamt: Option<u64>,
    phase: &'static str,
}
fn update_err(e: impl std::fmt::Display) -> CmdError {
    CmdError {
        art: "update",
        meldung: e.to_string(),
    }
}
fn schluessel_gueltig(key: &str) -> bool {
    let Ok(bytes) = STANDARD.decode(key.trim()) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    let mut lines = text.lines();
    lines
        .next()
        .is_some_and(|s| s.starts_with("untrusted comment:"))
        && lines
            .next()
            .and_then(|s| STANDARD.decode(s).ok())
            .is_some_and(|b| b.len() == 42 && &b[..2] == b"Ed")
}
fn konfiguration(
    key: &str,
    endpoint: Option<&str>,
    server: Option<&str>,
) -> Result<(String, String), CmdError> {
    if !schluessel_gueltig(key) {
        return Err(update_err(
            "Updates sind für diesen Build noch nicht eingerichtet: gültigen öffentlichen BAUPROJEKT_UPDATER_PUBKEY vor dem Build hinterlegen.",
        ));
    }
    let quelle = endpoint
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_owned())
        .or_else(|| {
            server
                .filter(|s| !s.trim().is_empty())
                .map(|s| format!("{}/updates/latest.json", s.trim().trim_end_matches('/')))
        })
        .unwrap_or_else(|| GITHUB.into());
    let url = reqwest::Url::parse(&quelle).map_err(update_err)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(update_err(
            "Die Updatequelle muss eine HTTPS-Adresse ohne Zugangsdaten oder Fragment sein.",
        ));
    }
    Ok((key.trim().into(), quelle))
}
fn config() -> Result<(String, String), CmdError> {
    konfiguration(
        option_env!("BAUPROJEKT_UPDATER_PUBKEY").unwrap_or(""),
        option_env!("BAUPROJEKT_UPDATE_URL"),
        option_env!("BAUPROJEKT_LIZENZSERVER"),
    )
}
#[tauri::command]
pub async fn update_status(app: AppHandle) -> UpdateStatus {
    let mut status = match config() {
        Ok((_, quelle)) => UpdateStatus { eingerichtet: true,
            verfuegbar: None, quelle, hinweis: "Signierte Updates werden beim Start und auf Anforderung geprüft. Die Installation erfolgt erst auf Ihren Wunsch.".into() },
        Err(e) => UpdateStatus { eingerichtet: false,
            verfuegbar: None, quelle: String::new(), hinweis: e.meldung },
    };
    status.verfuegbar = app
        .state::<PendingUpdate>()
        .update
        .lock()
        .await
        .as_ref()
        .map(|u| UpdateInfo {
            version: u.version.clone(),
            notizen: u.body.clone(),
        });
    status
}
async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, CmdError> {
    let state = app.state::<PendingUpdate>();
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| update_err("Eine Updateaktion läuft bereits."))?;
    let (key, endpoint) = config()?;
    let update = app
        .updater_builder()
        .pubkey(key)
        .endpoints(vec![endpoint.parse().map_err(update_err)?])
        .map_err(update_err)?
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(update_err)?
        .check()
        .await
        .map_err(|e| {
            update_err(format!(
                "Updateprüfung fehlgeschlagen. Netzwerk und veröffentlichtes Release prüfen: {e}"
            ))
        })?;
    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        notizen: u.body.clone(),
    });
    *state.update.lock().await = update;
    Ok(info)
}
pub fn check_in_background(app: AppHandle) {
    if cfg!(debug_assertions) || config().is_err() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        match check(&app).await {
            Ok(Some(info)) => {
                let _ = app.emit("update-verfuegbar", info);
            }
            Ok(None) => tracing::info!("keine Aktualisierung verfügbar"),
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
    version: String,
) -> Result<(), CmdError> {
    let _guard = pending
        .operation
        .try_lock()
        .map_err(|_| update_err("Eine Updateaktion läuft bereits."))?;
    let mut update = pending
        .update
        .lock()
        .await
        .clone()
        .ok_or_else(|| update_err("Kein Update vorgemerkt. Zuerst erneut prüfen."))?;
    if update.version != version {
        return Err(update_err(
            "Der angebotene Updatestand hat sich geändert. Bitte erneut prüfen.",
        ));
    }
    // Große Installer benötigen mehr Zeit als die Manifestabfrage.
    update.timeout = Some(Duration::from_secs(1800));
    let mut geladen = 0u64;
    update
        .download_and_install(
            |chunk, gesamt| {
                geladen = geladen.saturating_add(chunk as u64);
                let _ = app.emit(
                    "update-fortschritt",
                    Fortschritt {
                        geladen,
                        gesamt,
                        phase: "download",
                    },
                );
            },
            || {
                let _ = app.emit(
                    "update-fortschritt",
                    Fortschritt {
                        geladen: 0,
                        gesamt: None,
                        phase: "installation",
                    },
                );
            },
        )
        .await
        .map_err(update_err)?;
    app.restart();
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> String {
        let mut b = vec![0; 42];
        b[..2].copy_from_slice(b"Ed");
        STANDARD.encode(format!(
            "untrusted comment: test public key\n{}\n",
            STANDARD.encode(b)
        ))
    }
    #[test]
    fn quelle_ist_buildgebunden_und_schluessel_erforderlich() {
        assert!(konfiguration("", None, None).is_err());
        assert!(konfiguration("ERSETZEN", None, None).is_err());
        assert_eq!(konfiguration(&key(), None, None).unwrap().1, GITHUB);
        assert_eq!(
            konfiguration(&key(), None, Some("https://lizenz.test/"))
                .unwrap()
                .1,
            "https://lizenz.test/updates/latest.json"
        );
        assert_eq!(
            konfiguration(
                &key(),
                Some("https://update.test/latest.json"),
                Some("https://lizenz.test")
            )
            .unwrap()
            .1,
            "https://update.test/latest.json"
        );
        for url in [
            "http://update.test/latest.json",
            "https://u:p@update.test/latest.json",
            "file:///tmp/latest.json",
        ] {
            assert!(konfiguration(&key(), Some(url), None).is_err());
        }
    }
}
