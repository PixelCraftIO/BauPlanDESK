//! Rückweg aus dem früheren Beispielprojekt.
//!
//! Bis 11/2026 stellte „Hilfe → Beispielprojekt öffnen“ ein mitgeliefertes
//! Archiv in ein eigenes Profil unter `<app_data_dir>/Beispielprojekt/` wieder
//! her. Der Menüpunkt ist entfallen; die Abläufe zeigt das Benutzerhandbuch.
//! Wer beim Update noch im Beispielprofil arbeitet, sieht weiter den Hinweis
//! „Zu meinen Projekten zurückkehren“. Das vorher aktive Profil steht in
//! `<app_config_dir>/beispielprojekt-rueckkehr.json`.
use crate::{bootstrap::AppState, commands::CmdError, config::ProfilAuswahl};
use core_lib::CoreError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

const RUECKKEHR: &str = "beispielprojekt-rueckkehr.json";

fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(e.to_string())
}

/// Vor dem Wechsel aktives Profil; `None` ist das Standardprofil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rueckkehr {
    schema_version: u32,
    profil: Option<PathBuf>,
}

/// Stellt die Profilwahl vor dem Beispielprojekt wieder her. Ist das frühere
/// Profil nicht mehr vorhanden, gilt wieder das Standardprofil.
fn rueckkehr_anwenden(global: &Path) -> Result<(), String> {
    let profil = match std::fs::read(global.join(RUECKKEHR)) {
        Ok(raw) => {
            serde_json::from_slice::<Rueckkehr>(&raw)
                .map_err(|e| e.to_string())?
                .profil
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    match profil.filter(|p| p.join("bauprojekt.db").is_file()) {
        Some(pfad) => ProfilAuswahl {
            schema_version: 1,
            pfad,
        }
        .speichern(global)?,
        None => match std::fs::remove_file(global.join("profil.json")) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        },
    }
    match std::fs::remove_file(global.join(RUECKKEHR)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

fn ordner(app: &AppHandle) -> Result<PathBuf, CoreError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(err)?
        .join("Beispielprojekt"))
}

#[derive(Serialize)]
pub struct Beispielstatus {
    /// Die App läuft noch im Profil des früheren Beispielprojekts.
    aktiv: bool,
}

#[tauri::command]
pub fn beispielprojekt_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Beispielstatus, CmdError> {
    Ok(Beispielstatus {
        aktiv: state.data_dir.starts_with(ordner(&app)?),
    })
}

/// Kehrt aus dem Beispielprojekt zum vorherigen Profil zurück und startet neu.
/// Die bearbeitete Kopie bleibt auf der Festplatte liegen.
#[tauri::command]
pub async fn beispielprojekt_verlassen(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CmdError> {
    if !state.data_dir.starts_with(ordner(&app)?) {
        return Err(
            CoreError::InvalidInput("Das Beispielprojekt ist nicht geöffnet".into()).into(),
        );
    }
    let global = app.path().app_config_dir().map_err(err)?;
    rueckkehr_anwenden(&global).map_err(CoreError::Storage)?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profil(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir.join("einstellungen")).unwrap();
        std::fs::write(dir.join("bauprojekt.db"), b"").unwrap();
        std::fs::write(dir.join("einstellungen/config.json"), b"{}").unwrap();
        dir.to_path_buf()
    }

    /// So hat das frühere „Beispielprojekt öffnen“ die Rückkehr gemerkt.
    fn rueckkehr(global: &Path, profil: Option<PathBuf>) {
        let raw = serde_json::to_vec_pretty(&Rueckkehr {
            schema_version: 1,
            profil,
        })
        .unwrap();
        std::fs::write(global.join(RUECKKEHR), raw).unwrap();
    }

    fn beispiel_waehlen(global: &Path, root: &Path) {
        ProfilAuswahl {
            schema_version: 1,
            pfad: root.join("Beispielprojekt/Kopie-1"),
        }
        .speichern(global)
        .unwrap();
    }

    #[test]
    fn rueckkehr_fuehrt_zum_vorherigen_profil() {
        let root = tempfile::tempdir().unwrap();
        let global = root.path().join("global");
        std::fs::create_dir_all(&global).unwrap();
        // Standardprofil: nach der Rückkehr gibt es keine Profilwahl.
        rueckkehr(&global, None);
        beispiel_waehlen(&global, root.path());
        rueckkehr_anwenden(&global).unwrap();
        assert!(!global.join("profil.json").exists());
        assert!(!global.join(RUECKKEHR).exists());

        // Wiederhergestelltes eigenes Profil: genau dorthin zurück.
        let eigenes = profil(&root.path().join("Eigenes"));
        rueckkehr(&global, Some(eigenes.clone()));
        beispiel_waehlen(&global, root.path());
        rueckkehr_anwenden(&global).unwrap();
        let raw = std::fs::read(global.join("profil.json")).unwrap();
        let auswahl: ProfilAuswahl = serde_json::from_slice(&raw).unwrap();
        assert_eq!(auswahl.pfad, eigenes);

        // Verschwundenes Profil: Standardprofil statt Startfehler.
        rueckkehr(&global, Some(eigenes.clone()));
        std::fs::remove_dir_all(&eigenes).unwrap();
        rueckkehr_anwenden(&global).unwrap();
        assert!(!global.join("profil.json").exists());
    }
}
