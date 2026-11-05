//! Native Dateiausgabe und dünne Adapter zum Core.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{CoreError, abstandsflaechen::*, domain::ProjectId, zugriff::ProjektRecht};
use tauri::State;
#[tauri::command]
pub fn abstandsregeln() -> Result<Vec<AbstandsRegeln>, CmdError> {
    Ok(katalog()?)
}
/// Alle Regeldateien je Land und Fassung mit Wortlaut, Quellen und Prüfstatus.
#[tauri::command]
pub fn abstandsregelwerke() -> Result<Vec<Regelwerk>, CmdError> {
    Ok(regelwerke()?)
}
#[tauri::command]
pub async fn abstandsplan_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: AbstandsPlan,
) -> Result<AbstandsErgebnis, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.abstandsplan_pruefen(projekt_id, &plan).await?)
}
#[tauri::command]
pub async fn abstandsnachweis_export(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: uuid::Uuid,
    format: String,
    ziel: std::path::PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let (_, bytes) = state
        .core
        .abstandsnachweis_export(projekt_id, id, &format)
        .await?;
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn abstandscad_konturen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    einheit: core_lib::cad::Laengeneinheit,
) -> Result<Vec<CadKontur>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .abstandscad_konturen(projekt_id, dokument, einheit)
        .await?)
}
#[tauri::command]
pub fn abstandsumrisse_einbinden(plan: AbstandsPlan) -> Result<UmrissAbgleich, CmdError> {
    Ok(umrisse_abgleichen(plan)?)
}
#[tauri::command]
pub async fn abstandscad_hoehen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    einheit: core_lib::cad::Laengeneinheit,
) -> Result<Vec<CadHoehe>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .abstandscad_hoehen(projekt_id, dokument, einheit)
        .await?)
}
#[tauri::command]
pub fn abstand_gelaende_vorschlag(
    wand: AbstandsWand,
    hoehen: Vec<CadHoehe>,
    max_abstand_m: f64,
) -> Result<Vec<Hoehenpunkt>, CmdError> {
    Ok(gelaende_vorschlag(&wand, &hoehen, max_abstand_m)?)
}
