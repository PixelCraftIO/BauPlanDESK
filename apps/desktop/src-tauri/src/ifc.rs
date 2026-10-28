//! IFC-Modelle: dünne Adapter zu `core_lib::ifc`; keine lokale Modelllogik.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{domain::ProjectId, plattform::objekte::Dokumentbezug, zugriff::ProjektRecht};
use tauri::State;

#[tauri::command]
pub async fn ifc_auszug(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
) -> Result<core_lib::ifc::Modellauszug, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.ifc_auszug(projekt_id, &dokument).await?)
}

#[tauri::command]
pub async fn ifc_geschosse(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
) -> Result<Vec<core_lib::ifc::Modellgeschoss>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.ifc_geschosse(projekt_id, &dokument).await?)
}

#[tauri::command]
pub async fn ifc_grundriss(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
    geschoss: String,
    schnitthoehe_m: Option<f64>,
) -> Result<core_lib::ifc::Modellgrundriss, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_grundriss(projekt_id, &dokument, &geschoss, schnitthoehe_m)
        .await?)
}

#[tauri::command]
pub async fn ifc_grundriss_plan(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
    geschoss: String,
) -> Result<core_lib::cad::Plan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_grundriss_plan(projekt_id, &dokument, &geschoss)
        .await?)
}

/// Modelldatei für die 3D-Ansicht (Base64; IFCZIP entpackt).
#[tauri::command]
pub async fn ifc_datei(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
) -> Result<String, CmdError> {
    use base64::Engine;
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let daten = state.core.ifc_datei(projekt_id, &dokument).await?;
    Ok(base64::engine::general_purpose::STANDARD.encode(daten))
}
