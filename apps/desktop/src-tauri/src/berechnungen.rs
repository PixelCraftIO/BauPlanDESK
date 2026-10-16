//! Dieselben Fachkerne wie im Projektserver; keine lokale Rechenlogik.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    berechnungen::{BerechnungsVorlage, FachErgebnis, FachPlan},
    domain::ProjectId,
    zugriff::ProjektRecht,
};
use tauri::State;
#[tauri::command]
pub async fn berechnungsvorlagen() -> Result<Vec<BerechnungsVorlage>, CmdError> {
    Ok(core_lib::berechnungen::vorlagen()?)
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
