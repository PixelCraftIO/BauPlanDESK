//! Desktop-Adapter für den projektunabhängigen Formularordner.
use crate::{bootstrap::AppState, commands::CmdError};
use base64::Engine;
use bytes::Bytes;
use core_lib::{
    CoreError,
    benutzer::Recht,
    domain::{DocumentId, DocumentVersion, ProjectId},
    formular::bibliothek::{BibliotheksVorlage, import_pruefen},
};
use serde::Serialize;
use std::path::PathBuf;
use storage::formularbibliothek as ordner;
use tauri::{AppHandle, Manager, State};
type Resultat<T> = Result<T, CmdError>;

#[derive(Serialize)]
pub struct Bibliothek {
    root: PathBuf,
    vorlagen: Vec<BibliotheksVorlage>,
}
#[tauri::command]
pub async fn formularbibliothek(state: State<'_, AppState>) -> Resultat<Bibliothek> {
    let root = state.formularordner.read().await;
    Ok(Bibliothek {
        root: root.clone(),
        vorlagen: ordner::auflisten(&root).await?,
    })
}
/// Ordner verbinden, ohne bestehende Vorlagen zu verschieben oder zu löschen.
#[tauri::command]
pub async fn formularordner_waehlen(
    app: AppHandle,
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Resultat<()> {
    state.benutzer().pruefe(Recht::BenutzerVerwalten)?;
    let mut root = state.formularordner.write().await;
    ordner::ordner_pruefen(&pfad).await?;
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    crate::config::DateiablageConfig { root: pfad.clone() }
        .save_named(&config_dir, "formularbibliothek.json")
        .map_err(CoreError::Storage)?;
    *root = pfad;
    Ok(())
}
#[tauri::command]
pub async fn bibliothek_importieren(
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Resultat<BibliotheksVorlage> {
    state.benutzer().pruefe(Recht::DokumentAblegen)?;
    let name = pfad
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| CoreError::InvalidInput("Ungültiger Dateiname".into()))?;
    let parent = pfad
        .parent()
        .ok_or_else(|| CoreError::InvalidInput("PDF-Datei auswählen".into()))?;
    let pdf = ordner::lesen(parent, name, None).await?;
    import_pruefen(&pdf, &state.benutzer())?;
    let root = state.formularordner.read().await;
    Ok(ordner::ablegen(&root, name, &pdf).await?)
}
#[tauri::command]
pub async fn bibliothek_uebernehmen(
    state: State<'_, AppState>,
    vorlage: BibliotheksVorlage,
    projekt_id: ProjectId,
) -> Resultat<DocumentVersion> {
    state.benutzer().pruefe(Recht::DokumentAblegen)?;
    let root = state.formularordner.read().await;
    let pdf = ordner::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await?;
    import_pruefen(&pdf, &state.benutzer())?;
    Ok(state
        .core
        .store_document(
            projekt_id,
            None,
            None,
            &vorlage.name,
            Bytes::from(pdf),
            &state.benutzer(),
        )
        .await?)
}
#[derive(Serialize)]
pub struct Vorschau {
    pdf_base64: String,
    seiten: Vec<core_lib::formular::editor::PdfSeite>,
    felder: Vec<core_lib::formular::Formularfeld>,
}
#[tauri::command]
pub async fn bibliothek_vorschau(
    state: State<'_, AppState>,
    vorlage: BibliotheksVorlage,
) -> Resultat<Vorschau> {
    let root = state.formularordner.read().await;
    let pdf = ordner::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await?;
    Ok(Vorschau {
        seiten: core_lib::formular::editor::seiten(&pdf)?,
        felder: core_lib::formular::pdf::felder_lesen(&pdf)?,
        pdf_base64: base64::engine::general_purpose::STANDARD.encode(&pdf),
    })
}
#[tauri::command]
pub async fn bibliothek_veroeffentlichen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> Resultat<BibliotheksVorlage> {
    state.benutzer().pruefe(Recht::DokumentAblegen)?;
    let (dokument, pdf) = state.core.load_version(dokument_id, version).await?;
    import_pruefen(&pdf, &state.benutzer())?;
    let root = state.formularordner.read().await;
    Ok(ordner::ablegen(&root, &dokument.file_name, &pdf).await?)
}

#[derive(Serialize)]
pub struct BibliothekMaske {
    schema: core_lib::service::FormularSchema,
    pdf_base64: String,
    seiten: Vec<core_lib::formular::editor::PdfSeite>,
}
#[tauri::command]
pub async fn bibliothek_formular(
    state: State<'_, AppState>,
    bezug: Option<core_lib::formular::instanz::FormularBezug>,
    vorlage: BibliotheksVorlage,
    projekt_id: ProjectId,
) -> Resultat<BibliothekMaske> {
    let root = state.formularordner.read().await;
    let pdf = ordner::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await?;
    Ok(BibliothekMaske {
        schema: state
            .core
            .formular_schema_mit_bezug(projekt_id, &pdf, bezug.unwrap_or_default())
            .await?,
        pdf_base64: base64::engine::general_purpose::STANDARD.encode(&pdf),
        seiten: core_lib::formular::editor::seiten(&pdf)?,
    })
}
#[tauri::command]
pub async fn bibliothek_formular_speichern(
    state: State<'_, AppState>,
    vorlage: BibliotheksVorlage,
    projekt_id: ProjectId,
    vorgang: Option<uuid::Uuid>,
    datenstand: Option<core_lib::formular::instanz::FormularDatenstand>,
    werte: std::collections::BTreeMap<String, core_lib::formular::Feldwert>,
) -> Resultat<crate::projektordner::ProjektDokument> {
    let root = state.formularordner.read().await;
    let pdf = ordner::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await?;
    let d = state
        .core
        .formular_aus_bibliothek_mit_quellen(
            projekt_id,
            &vorlage.name,
            &pdf,
            &werte,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang,
                datenstand,
            },
            &state.benutzer(),
        )
        .await?;
    Ok(crate::projektordner::ablegen(&state, d).await)
}
#[tauri::command]
pub async fn bibliothek_felder_erstellen(
    state: State<'_, AppState>,
    vorlage: BibliotheksVorlage,
    felder: Vec<core_lib::formular::editor::EigenesPdfFeld>,
) -> Resultat<BibliotheksVorlage> {
    state.benutzer().pruefe(Recht::DokumentAblegen)?;
    let root = state.formularordner.read().await;
    let pdf = ordner::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await?;
    let neu = core_lib::formular::editor::felder_hinzufuegen(&pdf, &felder)?;
    let stem = vorlage
        .name
        .get(..vorlage.name.len().saturating_sub(4))
        .unwrap_or("Formular");
    Ok(ordner::ablegen(&root, &format!("{stem} (ausfüllbar).pdf"), &neu).await?)
}
