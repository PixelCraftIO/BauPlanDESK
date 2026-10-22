//! Mitgeliefertes Beispielprojekt „Neubau Einfamilienhaus Mustermann“.
//!
//! Das gebündelte Sicherungsarchiv wird nie verändert. „Hilfe → Beispielprojekt
//! öffnen“ stellt es in ein eigenes Profil unter `<app_data_dir>/Beispielprojekt/`
//! wieder her und startet die App darin neu. Dort darf alles bearbeitet werden;
//! die eigenen Projekte bleiben im bisherigen Profil unberührt. Welches Profil
//! vorher aktiv war, steht in `<app_config_dir>/beispielprojekt-rueckkehr.json`,
//! damit „Zu meinen Projekten zurückkehren“ genau dorthin zurückführt.
use crate::{
    bootstrap::AppState,
    commands::CmdError,
    config::ProfilAuswahl,
    menu::{ZuletztEintrag, ZuletztListe},
};
use core_lib::CoreError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State, path::BaseDirectory};

/// Pfad des Archivs unter den Ressourcen (siehe `tauri.conf.json`).
const ARCHIV: &str = "beispiele/Beispielprojekt-EFH-Mustermann.zip";
const RUECKKEHR: &str = "beispielprojekt-rueckkehr.json";
/// Gilt nur für diesen Rechner; liegt neben der Datenbank (siehe bootstrap.rs).
const TICKET: &str = "lizenz.ticket";

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

/// Kopien des Beispielprojekts, älteste zuerst.
fn kopien(ordner: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("Kopie-"))
                && p.join("bauprojekt.db").is_file()
                && p.join("einstellungen/config.json").is_file()
        })
        .collect();
    v.sort();
    v
}

/// Merkt sich das aktive Profil, bevor das Beispielprofil gewählt wird.
fn rueckkehr_merken(global: &Path) -> Result<(), String> {
    let profil = match std::fs::read(global.join("profil.json")) {
        Ok(raw) => Some(
            serde_json::from_slice::<ProfilAuswahl>(&raw)
                .map_err(|e| e.to_string())?
                .pfad,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let raw = serde_json::to_vec_pretty(&Rueckkehr {
        schema_version: 1,
        profil,
    })
    .map_err(|e| e.to_string())?;
    std::fs::write(global.join(RUECKKEHR), raw).map_err(|e| e.to_string())
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

fn global(app: &AppHandle) -> Result<PathBuf, CoreError> {
    app.path().app_config_dir().map_err(err)
}

fn angemeldet(state: &AppState) -> Result<(), CmdError> {
    if state.benutzer().ist_angemeldet() {
        Ok(())
    } else {
        Err(CoreError::InvalidInput("Bitte zuerst anmelden".into()).into())
    }
}

#[derive(Serialize)]
pub struct Beispielstatus {
    /// Die App läuft gerade im Profil des Beispielprojekts.
    aktiv: bool,
    /// Eine bearbeitete Kopie existiert bereits.
    kopie_vorhanden: bool,
}

#[tauri::command]
pub fn beispielprojekt_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Beispielstatus, CmdError> {
    let ordner = ordner(&app)?;
    Ok(Beispielstatus {
        aktiv: state.data_dir.starts_with(&ordner),
        kopie_vorhanden: !kopien(&ordner).is_empty(),
    })
}

/// Stellt das gebündelte Archiv in eine neue Kopie wieder her. Lizenz und
/// Testbeginn kommen aus dem aktiven Profil; das Projekt steht danach unter
/// „Zuletzt geöffnet“.
async fn kopie_anlegen(
    app: &AppHandle,
    state: &AppState,
    ordner: &Path,
) -> Result<PathBuf, CoreError> {
    let archiv = app
        .path()
        .resolve(ARCHIV, BaseDirectory::Resource)
        .map_err(err)?;
    let ziel = ordner.join(format!(
        "Kopie-{:013}",
        chrono::Utc::now().timestamp_millis()
    ));
    let neu = ziel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        storage::sicherung::wiederherstellen(&archiv, &neu)
    })
    .await
    .map_err(err)??;
    let result = async {
        crate::sicherungen::profil_einrichten(&ziel).await?;
        let db = db_sqlite::SqliteDatabase::open(&ziel.join("bauprojekt.db")).await?;
        let repos = db.repositories();
        let lizenz = core_lib::license::lizenz_uebernehmen(
            state.core.repos.lizenz.as_ref(),
            repos.lizenz.as_ref(),
        )
        .await;
        let projekte = repos.projects.list().await;
        drop(repos);
        db.schliessen().await;
        lizenz?;
        ZuletztListe(
            projekte?
                .into_iter()
                .map(|p| ZuletztEintrag {
                    id: p.id.to_string(),
                    name: p.name,
                })
                .collect(),
        )
        .speichern(&ziel.join("einstellungen/zuletzt.json"));
        let ticket = state.data_dir.join(TICKET);
        if ticket.is_file() {
            tokio::fs::copy(&ticket, ziel.join(TICKET))
                .await
                .map_err(err)?;
        }
        Ok::<_, CoreError>(())
    }
    .await;
    if let Err(e) = result {
        let _ = tokio::fs::remove_dir_all(&ziel).await;
        return Err(e);
    }
    Ok(ziel)
}

/// Öffnet das Beispielprojekt und startet die App neu. `neu` legt eine frische,
/// unveränderte Kopie an; sonst wird die zuletzt bearbeitete Kopie fortgesetzt.
/// Das Frontend prüft vorher, dass keine Eingaben offen sind.
#[tauri::command]
pub async fn beispielprojekt_oeffnen(
    app: AppHandle,
    state: State<'_, AppState>,
    neu: bool,
) -> Result<(), CmdError> {
    angemeldet(&state)?;
    let ordner = ordner(&app)?;
    let ziel = match kopien(&ordner).pop() {
        Some(k) if !neu => k,
        _ => kopie_anlegen(&app, &state, &ordner).await?,
    };
    let global = global(&app)?;
    if !state.data_dir.starts_with(&ordner) {
        rueckkehr_merken(&global).map_err(CoreError::Storage)?;
    }
    ProfilAuswahl {
        schema_version: 1,
        pfad: ziel.clone(),
    }
    .speichern(&global)
    .map_err(CoreError::Storage)?;
    // Ältere Kopien werden nicht mehr gebraucht; die gerade geöffnete
    // Datenbank bleibt bis zum Neustart stehen.
    for k in kopien(&ordner) {
        if k != ziel && !state.data_dir.starts_with(&k) {
            let _ = tokio::fs::remove_dir_all(k).await;
        }
    }
    app.restart();
}

/// Kehrt aus dem Beispielprojekt zum vorherigen Profil zurück und startet neu.
/// Die bearbeitete Kopie bleibt erhalten.
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
    rueckkehr_anwenden(&global(&app)?).map_err(CoreError::Storage)?;
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

    #[test]
    fn rueckkehr_fuehrt_zum_vorherigen_profil() {
        let root = tempfile::tempdir().unwrap();
        let global = root.path().join("global");
        std::fs::create_dir_all(&global).unwrap();
        // Standardprofil: nach der Rückkehr gibt es keine Profilwahl.
        rueckkehr_merken(&global).unwrap();
        ProfilAuswahl {
            schema_version: 1,
            pfad: profil(&root.path().join("Beispielprojekt/Kopie-1")),
        }
        .speichern(&global)
        .unwrap();
        rueckkehr_anwenden(&global).unwrap();
        assert!(!global.join("profil.json").exists());
        assert!(!global.join(RUECKKEHR).exists());

        // Wiederhergestelltes eigenes Profil: genau dorthin zurück.
        let eigenes = profil(&root.path().join("Eigenes"));
        ProfilAuswahl {
            schema_version: 1,
            pfad: eigenes.clone(),
        }
        .speichern(&global)
        .unwrap();
        rueckkehr_merken(&global).unwrap();
        ProfilAuswahl {
            schema_version: 1,
            pfad: root.path().join("Beispielprojekt/Kopie-1"),
        }
        .speichern(&global)
        .unwrap();
        rueckkehr_anwenden(&global).unwrap();
        let raw = std::fs::read(global.join("profil.json")).unwrap();
        let auswahl: ProfilAuswahl = serde_json::from_slice(&raw).unwrap();
        assert_eq!(auswahl.pfad, eigenes);

        // Verschwundenes Profil: Standardprofil statt Startfehler.
        rueckkehr_merken(&global).unwrap();
        std::fs::remove_dir_all(&eigenes).unwrap();
        rueckkehr_anwenden(&global).unwrap();
        assert!(!global.join("profil.json").exists());
    }

    #[test]
    fn nur_vollstaendige_kopien_in_reihenfolge() {
        let root = tempfile::tempdir().unwrap();
        let b = profil(&root.path().join("Kopie-0000000000002"));
        let a = profil(&root.path().join("Kopie-0000000000001"));
        std::fs::create_dir_all(root.path().join("Kopie-0000000000003")).unwrap();
        profil(&root.path().join(".wiederherstellung-x"));
        assert_eq!(kopien(root.path()), vec![a, b]);
        assert!(kopien(&root.path().join("fehlt")).is_empty());
    }
}
