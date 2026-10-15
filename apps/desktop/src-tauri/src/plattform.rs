//! Dünne Adapter für eigenständig versionierte Fachmodule.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    commit::Fachstand,
    domain::ProjectId,
    plattform::{
        ObjektArt,
        nachweise::{Freigabe, NachweisAnsicht, NachweisEingabe, Nachweisstand},
        objekte::RaumSummen,
    },
};
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub async fn fachobjekte(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
) -> Result<Vec<Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.fachobjekte(projekt_id, art).await?)
}
#[tauri::command]
pub async fn fachobjekt_speichern(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
    id: Uuid,
    basis: i64,
    daten: serde_json::Value,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .fachobjekt_speichern(projekt_id, art, id, basis, daten, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn nachweise(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<NachweisAnsicht>, CmdError> {
    Ok(state.core.nachweise(projekt_id).await?)
}
#[tauri::command]
pub async fn nachweis_erstellen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    bezeichnung: String,
    eingabe: NachweisEingabe,
) -> Result<Nachweisstand, CmdError> {
    Ok(state
        .core
        .nachweis_erstellen(projekt_id, id, bezeichnung, eingabe, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn nachweis_freigeben(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    bemerkung: String,
) -> Result<Freigabe, CmdError> {
    Ok(state
        .core
        .nachweis_freigeben(projekt_id, id, bemerkung, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn nachweis_wiederholen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<serde_json::Value, CmdError> {
    Ok(state.core.nachweis_wiederholen(projekt_id, id).await?)
}
#[tauri::command]
pub async fn raumbuch_summen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<RaumSummen, CmdError> {
    Ok(state.core.raumbuch_summen(projekt_id, id).await?)
}

#[tauri::command]
pub async fn fachverlauf(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
    id: Uuid,
) -> Result<Vec<Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.fachverlauf(projekt_id, art, id).await?)
}

#[tauri::command]
pub async fn projektstatus(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<core_lib::plattform::ablauf::Projektstatus, CmdError> {
    Ok(state.core.projektstatus(projekt_id).await?)
}
#[tauri::command]
pub fn standard_checkliste() -> Result<core_lib::plattform::ablauf::Checkliste, CmdError> {
    Ok(core_lib::plattform::ablauf::Checkliste::standard()?)
}
#[tauri::command]
pub async fn checkliste_anwenden(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    vorlage: core_lib::plattform::ablauf::Checkliste,
) -> Result<usize, CmdError> {
    Ok(state
        .core
        .checkliste_anwenden(projekt_id, vorlage, &state.benutzer())
        .await?)
}
