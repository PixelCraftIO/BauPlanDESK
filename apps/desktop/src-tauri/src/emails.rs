//! Desktop-Adapter für die lesende E-Mail-Ansicht.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{domain::DocumentId, email::ArchivierteEmail};
use tauri::State;
#[tauri::command]
pub async fn email_anhang_ablegen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
    index: usize,
) -> Result<core_lib::domain::DocumentVersion, CmdError> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .email_anhang_ablegen(dokument_id, version, index, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn email_laden(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> Result<ArchivierteEmail, CmdError> {
    let d = state.core.repos.documents.latest(dokument_id).await?;
    state
        .core
        .projekt_zugriff(
            d.project_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.archivierte_email(dokument_id, version).await?)
}
/// Archivierte E-Mail als Kommunikation in eine Verfahrensakte übernehmen.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn email_in_verfahren(
    state: State<'_, AppState>,
    projekt_id: core_lib::domain::ProjectId,
    dokument_id: DocumentId,
    version: i64,
    verfahren: uuid::Uuid,
    basis: i64,
    eingehend: bool,
    datum: Option<chrono::NaiveDate>,
) -> Result<core_lib::commit::Fachstand, CmdError> {
    Ok(state
        .core
        .email_in_verfahren(
            projekt_id,
            core_lib::plattform::objekte::Dokumentbezug {
                dokument_id,
                version,
            },
            verfahren,
            basis,
            eingehend,
            datum,
            &state.benutzer(),
        )
        .await?)
}
