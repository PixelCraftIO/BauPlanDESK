//! Tauri-Commands: nur Übersetzung zwischen IPC und core-lib.
//! Hier liegt bewusst keine Geschäftslogik.

use std::path::PathBuf;

use bytes::Bytes;
use core_lib::CoreError;
use core_lib::calc::{Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::bootstrap::AppState;
use crate::menu::{MenuState, ZuletztEintrag};

/// Fehler, wie das Frontend sie sieht.
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub art: &'static str,
    pub meldung: String,
}

impl From<CoreError> for CmdError {
    fn from(e: CoreError) -> Self {
        let art = match &e {
            CoreError::NotFound(_) => "nicht_gefunden",
            CoreError::VersionConflict { .. } => "konflikt",
            CoreError::InvalidInput(_) => "ungueltig",
            CoreError::RulePackMissing(_) | CoreError::Script { .. } => "lbo",
            CoreError::License(_) => "lizenz",
            _ => "intern",
        };
        Self {
            art,
            meldung: e.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CmdError>;

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    modus: &'static str,
    storage: String,
    regelpakete: Vec<RulePackInfo>,
    cache: Option<CacheInfo>,
}

#[derive(Serialize)]
pub struct CacheInfo {
    eintraege: usize,
    belegt_bytes: u64,
    treffer: u64,
    downloads: u64,
}

#[tauri::command]
pub async fn app_info(app: tauri::AppHandle, state: State<'_, AppState>) -> CmdResult<AppInfo> {
    let cache = match &state.cache {
        Some(cache) => {
            let s = cache.stats().await;
            Some(CacheInfo {
                eintraege: s.entries,
                belegt_bytes: s.total_bytes,
                treffer: s.hits,
                downloads: s.misses,
            })
        }
        None => None,
    };
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        modus: state.config.name(),
        storage: state.core.storage.backend_name(),
        regelpakete: state.core.lbo.rule_packs(),
        cache,
    })
}

#[tauri::command]
pub async fn projekte_auflisten(state: State<'_, AppState>) -> CmdResult<Vec<Project>> {
    Ok(state.core.list_projects().await?)
}

#[tauri::command]
pub async fn projekt_anlegen(
    state: State<'_, AppState>,
    projekt: NewProject,
) -> CmdResult<Project> {
    Ok(state.core.create_project(projekt).await?)
}

#[tauri::command]
pub async fn projekt_aktualisieren(
    state: State<'_, AppState>,
    aenderung: ProjectUpdate,
) -> CmdResult<Project> {
    Ok(state.core.update_project(aenderung).await?)
}

/// Öffnet ein Projekt: merkt es unter "Zuletzt geöffnet", aktiviert die
/// projektbezogenen Menüpunkte und setzt den Fenstertitel.
#[tauri::command]
pub async fn projekt_oeffnen(
    app: AppHandle,
    state: State<'_, AppState>,
    menu: State<'_, MenuState>,
    id: ProjectId,
) -> CmdResult<Project> {
    let projekt = match state.core.get_project(id).await {
        Ok(p) => p,
        Err(e) => {
            if matches!(e, CoreError::NotFound(_)) {
                menu.vergiss_projekt(&app, &id.to_string());
            }
            return Err(e.into());
        }
    };
    menu.merke_projekt(
        &app,
        ZuletztEintrag {
            id: projekt.id.to_string(),
            name: projekt.name.clone(),
        },
    );
    menu.set_projekt_offen(true);
    set_titel(&app, Some(&projekt.name));
    Ok(projekt)
}

#[tauri::command]
pub fn projekt_schliessen(app: AppHandle, menu: State<'_, MenuState>) {
    menu.set_projekt_offen(false);
    set_titel(&app, None);
}

#[tauri::command]
pub fn zuletzt_geoeffnet(menu: State<'_, MenuState>) -> Vec<ZuletztEintrag> {
    menu.zuletzt.lock().expect("Menü-Lock vergiftet").0.clone()
}

fn set_titel(app: &AppHandle, projekt: Option<&str>) {
    let titel = match projekt {
        Some(name) => format!("{name} – BauProjektIO"),
        None => "BauProjektIO".to_owned(),
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&titel);
    }
}

#[tauri::command]
pub async fn dokumente_auflisten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<DocumentVersion>> {
    Ok(state.core.list_documents(projekt_id).await?)
}

/// Importiert eine lokale Datei (z. B. per Drag & Drop) ins DMS.
#[tauri::command]
pub async fn dokument_importieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument_id: Option<DocumentId>,
    pfad: PathBuf,
) -> CmdResult<DocumentVersion> {
    let data = tokio::fs::read(&pfad).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", pfad.display()),
    })?;
    let name = pfad
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let user = whoami();
    Ok(state
        .core
        .store_document(projekt_id, dokument_id, &name, Bytes::from(data), &user)
        .await?)
}

#[tauri::command]
pub async fn dokument_historie(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<Vec<DocumentVersion>> {
    Ok(state.core.document_history(dokument_id).await?)
}

#[tauri::command]
pub fn abstandsflaeche(
    state: State<'_, AppState>,
    land: Bundesland,
    eingabe: AbstandsflaecheInput,
) -> CmdResult<Berechnung<AbstandsflaecheErgebnis>> {
    Ok(state.core.abstandsflaeche(land, &eingabe)?)
}

#[tauri::command]
pub fn mass_der_nutzung(
    state: State<'_, AppState>,
    eingabe: MassDerNutzung,
    gebiet: Baugebiet,
    festsetzungen: Option<Orientierungswerte>,
) -> CmdResult<Pruefergebnis> {
    Ok(state
        .core
        .mass_der_nutzung(&eingabe, gebiet, festsetzungen)?)
}

/// Platzhalter bis zur Benutzerverwaltung (Rollenmanagement).
fn whoami() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unbekannt".into())
}
