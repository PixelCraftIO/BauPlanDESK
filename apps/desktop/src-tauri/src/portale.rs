//! Native Datei-/Browseradapter. Fachliche Zuordnungen und Rechte im Core.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    CoreError, commit::Fachstand, domain::ProjectId, plattform::objekte::Dokumentbezug, portale::*,
};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};
use uuid::Uuid;
#[tauri::command]
pub async fn portalprofile(state: State<'_, AppState>) -> Result<Vec<Portalprofil>, CmdError> {
    Ok(state.core.portalprofile().await?)
}
#[tauri::command]
pub async fn portalprofil_hinzufuegen(
    state: State<'_, AppState>,
    profil: Portalprofil,
) -> Result<Vec<Portalprofil>, CmdError> {
    Ok(state
        .core
        .portalprofil_hinzufuegen(profil, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn portal_vorgang_anlegen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    basis: i64,
    wunsch: Portalvorgangswunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .portal_vorgang_anlegen(projekt_id, verfahren, basis, wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn portal_abgabe_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    wunsch: Portalabgabewunsch,
) -> Result<Portalvorbereitung, CmdError> {
    Ok(state
        .core
        .portal_abgabe_pruefen(projekt_id, verfahren, vorgang, &wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn portal_abgabe_festhalten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    basis: i64,
    wunsch: Portalabgabewunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .portal_abgabe_festhalten(
            projekt_id,
            verfahren,
            vorgang,
            basis,
            wunsch,
            &state.benutzer(),
        )
        .await?)
}
#[tauri::command]
pub async fn portal_ereignis_erfassen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    abgabe: Uuid,
    basis: i64,
    wunsch: Portalereigniswunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .portal_ereignis_erfassen(
            projekt_id,
            verfahren,
            vorgang,
            abgabe,
            basis,
            wunsch,
            &state.benutzer(),
        )
        .await?)
}
#[tauri::command]
pub async fn portal_rueckmeldung_erfassen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    basis: i64,
    wunsch: Portalrueckmeldungswunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .portal_rueckmeldung_erfassen(
            projekt_id,
            verfahren,
            vorgang,
            basis,
            wunsch,
            &state.benutzer(),
        )
        .await?)
}
async fn schreiben(ziel: PathBuf, bytes: Vec<u8>) -> Result<(), CmdError> {
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn portal_paket_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    abgabe: Uuid,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    let (_, bytes) = state
        .core
        .portal_bereitstellung(projekt_id, verfahren, vorgang, abgabe, &state.benutzer())
        .await?;
    schreiben(ziel, bytes).await
}
#[tauri::command]
pub async fn portal_datei_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    abgabe: Uuid,
    dokument: Dokumentbezug,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    let (_, bytes) = state
        .core
        .portal_datei(
            projekt_id,
            verfahren,
            vorgang,
            abgabe,
            &dokument,
            &state.benutzer(),
        )
        .await?;
    schreiben(ziel, bytes).await
}
#[tauri::command]
pub async fn portal_oeffnen(
    _app: AppHandle,
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
) -> Result<(), CmdError> {
    let url = state
        .core
        .portal_url(projekt_id, verfahren, vorgang, &state.benutzer())
        .await?;
    tauri::async_runtime::spawn_blocking(move || crate::dateien::starten(Path::new(&url)))
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
    Ok(())
}
#[tauri::command]
pub async fn portal_xbau_archivieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Uuid,
    vorgang: Uuid,
    abgabe: Uuid,
) -> Result<core_lib::xbau::postfach::Nachrichtenstand, CmdError> {
    Ok(state
        .core
        .portal_xbau_archivieren(projekt_id, verfahren, vorgang, abgabe, &state.benutzer())
        .await?)
}
