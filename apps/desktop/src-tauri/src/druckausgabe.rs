//! Dünne Adapter; Lesen/Exportieren erzeugt keinen neuen Projektstand.
use crate::{bootstrap::AppState, commands::CmdError};
use base64::{Engine, engine::general_purpose::STANDARD};
use core_lib::{
    CoreError,
    domain::ProjectId,
    druckausgabe::{Druckauftrag, Druckkatalog, Druckposition},
    zugriff::ProjektRecht,
};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
#[derive(Serialize)]
pub struct Druckvorschau {
    pdf_base64: String,
    seiten: u32,
    positionen: Vec<Druckposition>,
}
#[tauri::command]
pub async fn druckkatalog(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Druckkatalog, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.druckkatalog(projekt_id).await?)
}
#[tauri::command]
pub async fn druckausgabe_erstellen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    auftrag: Druckauftrag,
) -> Result<Druckvorschau, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let d = state.core.druckausgabe(projekt_id, auftrag).await?;
    Ok(Druckvorschau {
        pdf_base64: STANDARD.encode(d.pdf),
        seiten: d.seiten,
        positionen: d.positionen,
    })
}
#[tauri::command]
pub async fn druckausgabe_exportieren(pdf_base64: String, ziel: PathBuf) -> Result<(), CmdError> {
    // Exportiert genau die bereits geprüfte Vorschau, ohne erneute Berechnung.
    if pdf_base64.len() > 180 * 1024 * 1024 {
        return Err(CoreError::InvalidInput("Druckausgabe zu groß".into()).into());
    }
    let bytes = STANDARD
        .decode(pdf_base64)
        .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(CoreError::InvalidInput("Keine PDF-Ausgabe".into()).into());
    }
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn druckausgabe_ablegen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    name: String,
    pdf_base64: String,
    vorgang: uuid::Uuid,
) -> Result<core_lib::domain::DocumentVersion, CmdError> {
    if pdf_base64.len() > 180 * 1024 * 1024 {
        return Err(CoreError::InvalidInput("PDF-Ausgabe zu groß".into()).into());
    }
    let b = STANDARD
        .decode(pdf_base64)
        .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
    Ok(state
        .core
        .druckpdf_ablegen(
            projekt_id,
            &name,
            bytes::Bytes::from(b),
            vorgang,
            &state.benutzer(),
        )
        .await?)
}

#[tauri::command]
pub async fn druckgestaltung(
    state: State<'_, AppState>,
) -> Result<core_lib::druckausgabe::Druckgestaltung, CmdError> {
    Ok(state.core.druckgestaltung().await?)
}
#[tauri::command]
pub async fn druckgestaltung_speichern(
    state: State<'_, AppState>,
    gestaltung: core_lib::druckausgabe::Druckgestaltung,
) -> Result<core_lib::druckausgabe::Druckgestaltung, CmdError> {
    Ok(state
        .core
        .druckgestaltung_speichern(gestaltung, &state.benutzer())
        .await?)
}
