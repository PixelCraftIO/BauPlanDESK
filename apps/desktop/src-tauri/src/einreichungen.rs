//! IPC-Adapter zum unabhängigen Einreichungsmodul.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    CoreError,
    domain::ProjectId,
    einreichung::{DokumentVergleich, Einreichung},
};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub async fn einreichungen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<Einreichung>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.einreichungen(projekt_id).await?)
}
#[tauri::command]
pub async fn einreichungs_vergleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    empfaenger: String,
) -> Result<Vec<DokumentVergleich>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .einreichungs_vergleich(projekt_id, &empfaenger)
        .await?)
}
#[tauri::command]
pub async fn einreichung_exportieren(
    state: State<'_, AppState>,
    id: Uuid,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .einreichung_zugriff(
            id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    let (_, zip) = state.core.einreichung_zip(id).await?;
    tokio::fs::write(ziel, zip)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
