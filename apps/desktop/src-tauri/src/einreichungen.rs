//! IPC-Adapter zum unabhängigen Einreichungsmodul.
use crate::{bootstrap::AppState, commands::CmdError};
use chrono::NaiveDate;
use core_lib::{
    CoreError,
    domain::ProjectId,
    einreichung::{DokumentVergleich, Einreichung, NeuesPaket},
};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub async fn einreichungen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<Einreichung>, CmdError> {
    Ok(state.core.einreichungen(projekt_id).await?)
}
#[tauri::command]
pub async fn einreichungs_vergleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    empfaenger: String,
) -> Result<Vec<DokumentVergleich>, CmdError> {
    Ok(state
        .core
        .einreichungs_vergleich(projekt_id, &empfaenger)
        .await?)
}
#[tauri::command]
pub async fn einreichung_erstellen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    paket: NeuesPaket,
    vorgang: Option<Uuid>,
) -> Result<Einreichung, CmdError> {
    Ok(state
        .core
        .einreichung_erstellen_vorgang(projekt_id, paket, vorgang, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn einreichung_bestaetigen(
    state: State<'_, AppState>,
    id: Uuid,
    basis: i64,
    datum: NaiveDate,
    notiz: String,
) -> Result<Einreichung, CmdError> {
    Ok(state
        .core
        .einreichung_bestaetigen(id, basis, datum, notiz, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn einreichung_exportieren(
    state: State<'_, AppState>,
    id: Uuid,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    let (_, zip) = state.core.einreichung_zip(id).await?;
    tokio::fs::write(ziel, zip)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn paket_pruefstand(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokumente: Vec<core_lib::einreichung::DokumentAuswahl>,
) -> Result<core_lib::einreichung::PaketPruefstand, CmdError> {
    Ok(state.core.paket_pruefstand(projekt_id, &dokumente).await?)
}
