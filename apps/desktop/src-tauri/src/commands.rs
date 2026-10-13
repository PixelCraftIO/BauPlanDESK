//! Tauri-Commands: nur Übersetzung zwischen IPC und core-lib.
//! Hier liegt bewusst keine Geschäftslogik.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bytes::Bytes;
use core_lib::CoreError;
use core_lib::benutzer::{Benutzer, BenutzerAenderung, NeuerBenutzer};
use core_lib::cad::{Laengeneinheit, Messung, Punkt};
use core_lib::calc::{Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentLock, DocumentVersion, NewProject, Project, ProjectId,
    ProjectUpdate, Protokolleintrag,
};
use core_lib::formular::Feldwert;
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use core_lib::license::Lizenzstatus;
use core_lib::service::{CadPlan, FormularAnalyse};
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
            CoreError::Forbidden(_) => "keine_berechtigung",
            CoreError::Gesperrt { .. } => "gesperrt",
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
    benutzer: Benutzer,
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
        benutzer: state.benutzer(),
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
    Ok(state
        .core
        .create_project(projekt, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn projekt_aktualisieren(
    state: State<'_, AppState>,
    aenderung: ProjectUpdate,
) -> CmdResult<Project> {
    Ok(state
        .core
        .update_project(aenderung, &state.benutzer())
        .await?)
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

/// Importiert eine lokale Datei ins DMS. Für eine neue Version eines
/// bestehenden Dokuments gibt `basis` die Version an, die der Nutzer
/// gesehen hat (Optimistic Locking).
#[tauri::command]
pub async fn dokument_importieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument_id: Option<DocumentId>,
    basis: Option<i64>,
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
    Ok(state
        .core
        .store_document(
            projekt_id,
            dokument_id,
            basis,
            &name,
            Bytes::from(data),
            &state.benutzer(),
        )
        .await?)
}

#[tauri::command]
pub async fn formular_analysieren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<FormularAnalyse> {
    Ok(state.core.formular_analysieren(dokument_id).await?)
}

/// Befüllt das Formular und legt es als neues Dokument im Projekt ab.
#[tauri::command]
pub async fn formular_ausfuellen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    werte: BTreeMap<String, Feldwert>,
) -> CmdResult<DocumentVersion> {
    Ok(state
        .core
        .formular_ausfuellen(dokument_id, &werte, &state.benutzer())
        .await?)
}

/// Schreibt die aktuelle (oder mit `version` eine ältere) Version eines
/// Dokuments an einen vom Nutzer gewählten Ort (z. B. zum Einreichen beim
/// Bauamt).
#[tauri::command]
pub async fn dokument_exportieren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: Option<i64>,
    ziel: PathBuf,
) -> CmdResult<()> {
    let (_, data) = match version {
        Some(v) => state.core.load_version(dokument_id, v).await?,
        None => state.core.load_document(dokument_id).await?,
    };
    tokio::fs::write(&ziel, &data).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", ziel.display()),
    })
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

/// Lädt einen DXF-Plan aus dem DMS für den Plan-Viewer.
#[tauri::command]
pub async fn cad_plan(state: State<'_, AppState>, dokument_id: DocumentId) -> CmdResult<CadPlan> {
    Ok(state.core.cad_plan(dokument_id).await?)
}

#[tauri::command]
pub fn aufmass(
    state: State<'_, AppState>,
    punkte: Vec<Punkt>,
    geschlossen: bool,
    einheit: Laengeneinheit,
) -> CmdResult<Messung> {
    Ok(state.core.messen(&punkte, geschlossen, einheit)?)
}

#[tauri::command]
pub async fn version_wiederherstellen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> CmdResult<DocumentVersion> {
    Ok(state
        .core
        .version_wiederherstellen(dokument_id, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn dokument_sperren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<DocumentLock> {
    Ok(state
        .core
        .dokument_sperren(dokument_id, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn dokument_entsperren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<()> {
    Ok(state
        .core
        .dokument_entsperren(dokument_id, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn sperren_auflisten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<DocumentLock>> {
    Ok(state.core.sperren_im_projekt(projekt_id).await?)
}

#[tauri::command]
pub async fn protokoll(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<Protokolleintrag>> {
    Ok(state.core.protokoll(projekt_id, 500).await?)
}

#[tauri::command]
pub async fn benutzer_auflisten(state: State<'_, AppState>) -> CmdResult<Vec<Benutzer>> {
    Ok(state.core.benutzer_auflisten().await?)
}

#[tauri::command]
pub async fn benutzer_anlegen(
    state: State<'_, AppState>,
    neu: NeuerBenutzer,
) -> CmdResult<Benutzer> {
    Ok(state.core.benutzer_anlegen(neu, &state.benutzer()).await?)
}

/// Ändert einen Benutzer. Betrifft es den angemeldeten Benutzer selbst,
/// gelten die neuen Rechte sofort.
#[tauri::command]
pub async fn benutzer_aendern(
    state: State<'_, AppState>,
    aenderung: BenutzerAenderung,
) -> CmdResult<Benutzer> {
    let b = state
        .core
        .benutzer_aendern(aenderung, &state.benutzer())
        .await?;
    let mut ich = state.benutzer.write().expect("Benutzer-Lock vergiftet");
    if ich.id == b.id {
        *ich = b.clone();
    }
    Ok(b)
}

#[tauri::command]
pub async fn lizenz_status(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    Ok(state.core.lizenz().await?)
}

/// Lizenzdatei einspielen (Administrator) und danach den Arbeitsplatz mit
/// der neuen Anzahl Plätze belegen.
#[tauri::command]
pub async fn lizenz_einspielen(
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> CmdResult<Lizenzstatus> {
    let datei = tokio::fs::read(&pfad).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", pfad.display()),
    })?;
    let ich = state.benutzer();
    state.core.lizenz_einspielen(&datei, &ich).await?;
    arbeitsplatz_belegen(state).await
}

/// Arbeitsplatz (erneut) belegen, z. B. wenn ein Kollege die App beendet hat.
#[tauri::command]
pub async fn arbeitsplatz_belegen(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    let name = state.benutzer().anzeigename;
    Ok(state
        .core
        .license
        .arbeitsplatz_belegen(&state.arbeitsplatz, &name)
        .await?)
}
