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
pub async fn einreichung_erstellen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    paket: NeuesPaket,
    vorgang: Option<Uuid>,
) -> Result<Einreichung, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
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
    state
        .core
        .einreichung_zugriff(
            id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
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

#[tauri::command]
pub async fn paket_pruefstand(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokumente: Vec<core_lib::einreichung::DokumentAuswahl>,
) -> Result<core_lib::einreichung::PaketPruefstand, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.paket_pruefstand(projekt_id, &dokumente).await?)
}
