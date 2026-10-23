//! Dieselben Fachkerne wie im Projektserver; keine lokale Rechenlogik.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    berechnungen::{BerechnungsVorlage, FachErgebnis, FachPlan},
    domain::ProjectId,
    zugriff::ProjektRecht,
};
use tauri::State;
#[tauri::command]
pub async fn cadaufmass_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    revision: i64,
    ziel: std::path::PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let bytes = state
        .core
        .cadaufmass_csv(projekt_id, &dokument, revision)
        .await?;
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| core_lib::CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub fn cadaufmass_messwerte(
    aufmass: core_lib::cad::projektaufmass::CadProjektaufmass,
) -> Result<Vec<core_lib::cad::Messung>, CmdError> {
    Ok(aufmass.messen()?)
}
#[tauri::command]
pub async fn cadaufmass_laden(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: core_lib::plattform::objekte::Dokumentbezug,
) -> Result<Option<core_lib::commit::Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.cadaufmass_laden(projekt_id, &dokument).await?)
}
#[tauri::command]
pub async fn cadaufmass_speichern(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass: core_lib::cad::projektaufmass::CadProjektaufmass,
    basis: i64,
) -> Result<core_lib::commit::Fachstand, CmdError> {
    Ok(state
        .core
        .cadaufmass_speichern(projekt_id, aufmass, basis, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn planaufmass_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: core_lib::planaufmass::Planexportwunsch,
    ziel: std::path::PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let bytes = state.core.planaufmass_export(projekt_id, &wunsch).await?;
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| core_lib::CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn planaufmass_diagnose(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<core_lib::planaufmass::Planpruefung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_diagnose(projekt_id, &plan).await?)
}
#[tauri::command]
pub fn planaufmass_flaechenoperation(
    a: core_lib::planaufmass::Plangeometrie,
    b: core_lib::planaufmass::Plangeometrie,
    operation: core_lib::planaufmass::Flaechenoperation,
) -> Result<Vec<core_lib::planaufmass::Plangeometrie>, CmdError> {
    Ok(core_lib::planaufmass::flaechenoperation(&a, &b, operation)?)
}
#[tauri::command]
pub async fn planaufmass_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<Vec<core_lib::planaufmass::Planmesswert>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_pruefen(projekt_id, &plan).await?)
}
#[tauri::command]
pub fn planaufmass_ziele(plan: FachPlan) -> Result<Vec<core_lib::planaufmass::Planziel>, CmdError> {
    Ok(core_lib::planaufmass::planziele(&plan)?)
}
#[tauri::command]
pub async fn planaufmass_einsetzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: FachPlan,
) -> Result<FachPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_einsetzen(projekt_id, plan).await?)
}
#[tauri::command]
pub async fn planaufmass_raumbuch(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass_id: uuid::Uuid,
    revision: i64,
    raumbuch: core_lib::plattform::objekte::Raumbuch,
) -> Result<core_lib::plattform::objekte::Raumbuch, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_raumbuch(projekt_id, aufmass_id, revision, raumbuch)
        .await?)
}
#[tauri::command]
pub fn planaufmass_abstandsziele(
    plan: core_lib::abstandsflaechen::AbstandsPlan,
) -> Vec<core_lib::planaufmass::Planziel> {
    core_lib::planaufmass::abstandsziele(&plan)
}
#[tauri::command]
pub async fn planaufmass_abstand(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::abstandsflaechen::AbstandsPlan,
) -> Result<core_lib::abstandsflaechen::AbstandsPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_abstand(projekt_id, plan).await?)
}
#[tauri::command]
pub async fn berechnungsvorlagen() -> Result<Vec<BerechnungsVorlage>, CmdError> {
    Ok(core_lib::berechnungen::vorlagen()?)
}
#[tauri::command]
pub async fn planaufmass_aufgaben(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<Vec<core_lib::planaufmass::Planaufgabe>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_aufgaben(projekt_id, &plan).await?)
}
#[tauri::command]
pub fn planaufmass_gebaeudeziele(
    plan: core_lib::plattform::objekte::Gebaeude,
) -> Vec<core_lib::planaufmass::Planziel> {
    core_lib::planaufmass::gebaeudeziele(&plan)
}
#[tauri::command]
pub async fn planaufmass_gebaeude(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: uuid::Uuid,
    plan: core_lib::plattform::objekte::Gebaeude,
) -> Result<core_lib::plattform::objekte::Gebaeude, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_gebaeude(projekt_id, id, plan)
        .await?)
}
#[derive(serde::Serialize)]
pub struct Vorschau {
    plan: FachPlan,
    ergebnis: FachErgebnis,
}
#[tauri::command]
pub async fn fachberechnung_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: FachPlan,
) -> Result<Vorschau, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let (plan, ergebnis) = state.core.berechnung_pruefen(projekt_id, plan).await?;
    Ok(Vorschau { plan, ergebnis })
}
#[tauri::command]
pub async fn berechnung_aus_raumbuch(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: uuid::Uuid,
    modul: core_lib::berechnungen::BerechnungsArt,
) -> Result<FachPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .berechnung_aus_raumbuch(projekt_id, id, modul)
        .await?)
}
