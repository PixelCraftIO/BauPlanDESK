//! Desktop-Adapter für die lesende E-Mail-Ansicht.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{domain::DocumentId, email::ArchivierteEmail};
use tauri::State;
#[tauri::command]
pub async fn email_laden(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> Result<ArchivierteEmail, CmdError> {
    Ok(state.core.archivierte_email(dokument_id, version).await?)
}
