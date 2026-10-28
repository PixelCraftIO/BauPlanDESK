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
