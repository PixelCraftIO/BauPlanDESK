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

#[tauri::command]
pub async fn ifc_geschosse(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
) -> Result<Vec<core_lib::ifc::Modellgeschoss>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.ifc_geschosse(projekt_id, &dokument).await?)
}

#[tauri::command]
pub async fn ifc_grundriss(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
    geschoss: String,
    schnitthoehe_m: Option<f64>,
) -> Result<core_lib::ifc::Modellgrundriss, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_grundriss(projekt_id, &dokument, &geschoss, schnitthoehe_m)
        .await?)
}

#[tauri::command]
pub async fn ifc_grundriss_plan(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
    geschoss: String,
) -> Result<core_lib::cad::Plan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_grundriss_plan(projekt_id, &dokument, &geschoss)
        .await?)
}

/// Modelldatei für die 3D-Ansicht (Base64; IFCZIP entpackt).
#[tauri::command]
pub async fn ifc_datei(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
) -> Result<String, CmdError> {
    use base64::Engine;
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let daten = state.core.ifc_datei(projekt_id, &dokument).await?;
    Ok(base64::engine::general_purpose::STANDARD.encode(daten))
}

#[tauri::command]
pub async fn ifc_uebernehmen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: core_lib::ifc::Modelluebernahmewunsch,
) -> Result<core_lib::ifc::Modelluebernahmeergebnis, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Bearbeiten)
        .await?;
    Ok(state
        .core
        .ifc_uebernehmen(projekt_id, &wunsch, &wer)
        .await?)
}

#[tauri::command]
pub async fn ifc_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
    ids: Option<Dokumentbezug>,
) -> Result<core_lib::ifc::Modellpruefung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_pruefen(projekt_id, akte_id, ids.as_ref())
        .await?)
}

#[tauri::command]
pub async fn ifc_raumvorschlaege(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
    geschoss: String,
) -> Result<Vec<core_lib::ifc::Raumvorschlag>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_raumvorschlaege(projekt_id, akte_id, &geschoss)
        .await?)
}

#[tauri::command]
pub async fn ifc_akte_auszug(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
) -> Result<core_lib::ifc::Modellauszug, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let akte = state.core.modellakte(projekt_id, akte_id).await?;
    Ok(state.core.ifc_akte_auszug(projekt_id, &akte).await?)
}

#[tauri::command]
pub async fn ifc_akte_grundriss(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
    geschoss: String,
    schnitthoehe_m: Option<f64>,
) -> Result<core_lib::ifc::Modellgrundriss, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_akte_grundriss(projekt_id, akte_id, &geschoss, schnitthoehe_m)
        .await?)
}

#[tauri::command]
pub async fn ifc_abgleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
    dokument: Dokumentbezug,
) -> Result<core_lib::ifc::Modellabgleich, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_abgleich(projekt_id, akte_id, &dokument)
        .await?)
}

#[tauri::command]
pub async fn ifc_reimport(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: core_lib::ifc::Reimportwunsch,
) -> Result<core_lib::commit::Fachstand, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Bearbeiten)
        .await?;
    Ok(state.core.ifc_reimport(projekt_id, &wunsch, &wer).await?)
}

#[tauri::command]
pub async fn ifc_auswerten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    akte_id: uuid::Uuid,
    grundstueck_id: Option<uuid::Uuid>,
) -> Result<core_lib::ifc::Modellauswertung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .ifc_auswerten(projekt_id, akte_id, grundstueck_id)
        .await?)
}

#[tauri::command]
pub async fn ifc_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: core_lib::ifc::Exportwunsch,
) -> Result<core_lib::ifc::Exportergebnis, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Bearbeiten)
        .await?;
    Ok(state
        .core
        .ifc_exportieren(projekt_id, &wunsch, &wer)
        .await?)
}

#[tauri::command]
pub async fn ifc_dem_bauantrag_beifuegen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument_id: core_lib::domain::DocumentId,
    bezeichnung: String,
) -> Result<core_lib::ifc::Bauantragsanlage, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Bearbeiten)
        .await?;
    Ok(state
        .core
        .ifc_dem_bauantrag_beifuegen(projekt_id, dokument_id, &bezeichnung, &wer)
        .await?)
}
