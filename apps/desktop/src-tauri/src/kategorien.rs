//! Adapter zum gemeinsamen Kategorienvertrag; keine eigenen Katalogregeln.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    dokumentkategorien::*,
    domain::{DocumentId, ProjectId},
    storage::ContentHash,
};
use tauri::State;
#[tauri::command]
pub async fn kategorienkatalog(state: State<'_, AppState>) -> Result<KategorienKatalog, CmdError> {
    Ok(state.core.kategorienkatalog().await?)
}
#[tauri::command]
pub async fn kategorienkatalog_speichern(
    state: State<'_, AppState>,
    katalog: KategorienKatalog,
) -> Result<KategorienKatalog, CmdError> {
    Ok(state
        .core
        .kategorienkatalog_speichern(katalog, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn dokumentkategorien(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<DokumentKategorie>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.dokumentkategorien(projekt_id).await?)
}
#[tauri::command]
pub async fn dokumentkategorie_speichern(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    zuordnung: KategorieZuordnung,
) -> Result<DokumentKategorie, CmdError> {
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
        .dokumentkategorie_speichern(dokument_id, zuordnung, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn vorlagenkategorie(
    state: State<'_, AppState>,
    hash: ContentHash,
) -> Result<KategorieZuordnung, CmdError> {
    Ok(state.core.vorlagenkategorie(&hash).await?)
}
#[tauri::command]
pub async fn vorlagenkategorie_speichern(
    state: State<'_, AppState>,
    hash: ContentHash,
    zuordnung: KategorieZuordnung,
) -> Result<KategorieZuordnung, CmdError> {
    Ok(state
        .core
        .vorlagenkategorie_speichern(&hash, zuordnung, &state.benutzer())
        .await?)
}
