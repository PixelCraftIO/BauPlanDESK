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
        pdf_base64: core_lib::rechenjob::ausfuehren("druck-base64", move || {
            Ok(STANDARD.encode(d.pdf))
        })
        .await?,
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
    let bytes = core_lib::rechenjob::ausfuehren("druck-dekodieren", move || {
        STANDARD
            .decode(pdf_base64)
            .map_err(|e| CoreError::InvalidInput(e.to_string()))
    })
    .await?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(CoreError::InvalidInput("Keine PDF-Ausgabe".into()).into());
    }
    storage::atomare_datei::ersetzen(ziel, bytes.into()).await?;
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
    let b = core_lib::rechenjob::ausfuehren("druck-dekodieren", move || {
        STANDARD
            .decode(pdf_base64)
            .map_err(|e| CoreError::InvalidInput(e.to_string()))
    })
    .await?;
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

/// Binärer Vorschaupfad: Browser erhält eine kurzlebige Kennung, keinen
/// Base64-Text. Export und DMS-Ablage verwenden dieselbe geprüfte Datei.
#[derive(Serialize)]
pub struct Artefaktvorschau {
    artefakt: uuid::Uuid,
    seiten: u32,
    positionen: Vec<Druckposition>,
}
#[tauri::command]
pub async fn druckartefakt_erstellen(
    state: State<'_, AppState>,
    artefakte: State<'_, storage::druckartefakte::Druckartefakte>,
    projekt_id: ProjectId,
    auftrag: Druckauftrag,
    vorgang: uuid::Uuid,
    jobs: State<'_, crate::druckjobs::Druckjobs>,
    fenster: tauri::WebviewWindow,
) -> Result<Artefaktvorschau, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Lesen)
        .await?;
    use tauri::Emitter;
    let mut steuerung = core_lib::druckausgabe::Drucksteuerung::default();
    steuerung.fortschritt = Some(std::sync::Arc::new(move |fertig, gesamt| {
        let _ = fenster.emit(
            "druckfortschritt",
            serde_json::json!({"vorgang": vorgang, "fertig": fertig, "gesamt": gesamt}),
        );
    }));
    let job = jobs.starten(vorgang, projekt_id, wer.id.0, steuerung)?;
    let d = state
        .core
        .druckausgabe_gesteuert(
            projekt_id,
            auftrag,
            &Default::default(),
            job.steuerung.clone(),
        )
        .await?;
    job.steuerung.pruefen()?;
    let artefakt = artefakte.speichern(projekt_id, wer.id.0, d.pdf).await?;
    if state.benutzer().id != wer.id || job.steuerung.pruefen().is_err() {
        artefakte.entfernen(artefakt, projekt_id, wer.id.0)?;
        return Err(CoreError::InvalidInput(
            "Druckauftrag abgebrochen oder Benutzer abgemeldet".into(),
        )
        .into());
    }
    Ok(Artefaktvorschau {
        artefakt,
        seiten: d.seiten,
        positionen: d.positionen,
    })
}
#[tauri::command]
pub async fn druckartefakt_lesen(
    state: State<'_, AppState>,
    artefakte: State<'_, storage::druckartefakte::Druckartefakte>,
    projekt_id: ProjectId,
    artefakt: uuid::Uuid,
) -> Result<tauri::ipc::Response, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Lesen)
        .await?;
    let daten: Vec<u8> = artefakte
        .lesen(artefakt, projekt_id, wer.id.0)
        .await?
        .into();
    Ok(tauri::ipc::Response::new(daten))
}
#[tauri::command]
pub async fn druckartefakt_entfernen(
    state: State<'_, AppState>,
    artefakte: State<'_, storage::druckartefakte::Druckartefakte>,
    projekt_id: ProjectId,
    artefakt: uuid::Uuid,
) -> Result<(), CmdError> {
    Ok(artefakte.entfernen(artefakt, projekt_id, state.benutzer().id.0)?)
}
#[tauri::command]
pub async fn druckartefakt_exportieren(
    state: State<'_, AppState>,
    artefakte: State<'_, storage::druckartefakte::Druckartefakte>,
    projekt_id: ProjectId,
    artefakt: uuid::Uuid,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Lesen)
        .await?;
    storage::atomare_datei::ersetzen(ziel, artefakte.lesen(artefakt, projekt_id, wer.id.0).await?)
        .await?;
    Ok(())
}
#[tauri::command]
pub async fn druckartefakt_ablegen(
    state: State<'_, AppState>,
    artefakte: State<'_, storage::druckartefakte::Druckartefakte>,
    projekt_id: ProjectId,
    artefakt: uuid::Uuid,
    name: String,
    vorgang: uuid::Uuid,
) -> Result<core_lib::domain::DocumentVersion, CmdError> {
    let wer = state.benutzer();
    state
        .core
        .projekt_zugriff(projekt_id, &wer, ProjektRecht::Bearbeiten)
        .await?;
    let daten = artefakte.lesen(artefakt, projekt_id, wer.id.0).await?;
    Ok(state
        .core
        .druckpdf_ablegen(projekt_id, &name, daten, vorgang, &wer)
        .await?)
}

#[tauri::command]
pub async fn druckartefakt_abbrechen(
    state: State<'_, AppState>,
    jobs: State<'_, crate::druckjobs::Druckjobs>,
    projekt_id: ProjectId,
    vorgang: uuid::Uuid,
) -> Result<(), CmdError> {
    Ok(jobs.abbrechen(vorgang, projekt_id, state.benutzer().id.0)?)
}
