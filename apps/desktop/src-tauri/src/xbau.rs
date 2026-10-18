//! Desktop-Adapter zum geschützten XBau-Journal.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    commit::Fachstand,
    domain::ProjectId,
    plattform::objekte::Dokumentbezug,
    xbau::{austausch::*, postfach::*},
};
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub fn xbau_nachrichtentypen() -> Vec<Nachrichtentyp> {
    katalog()
}
#[tauri::command]
pub async fn xbau_nachrichten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<Nachrichtenstand>, CmdError> {
    Ok(state
        .core
        .xbau_nachrichten(projekt_id, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn xbau_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    original: Dokumentbezug,
) -> Result<Pruefbericht, CmdError> {
    Ok(state
        .core
        .xbau_pruefen(projekt_id, &original, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn xbau_aufnehmen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: Nachrichtenaufnahme,
) -> Result<Nachrichtenstand, CmdError> {
    Ok(state
        .core
        .xbau_aufnehmen(projekt_id, wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn xbau_beleg(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    basis: i64,
    beleg: Nachrichtenbeleg,
) -> Result<Nachrichtenstand, CmdError> {
    Ok(state
        .core
        .xbau_beleg(projekt_id, id, basis, beleg, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn xbau_in_verfahren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    verfahren: Uuid,
    basis: i64,
    zugang: Option<chrono::NaiveDate>,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .xbau_in_verfahren(projekt_id, id, verfahren, basis, zugang, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn xbau_anlage_ablegen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    dateiname: String,
) -> Result<core_lib::domain::DocumentVersion, CmdError> {
    Ok(state
        .core
        .xbau_anlage_ablegen(projekt_id, id, &dateiname, &state.benutzer())
        .await?)
}
