//! Native I/O für die gemeinsame Verfahrenslogik.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    CoreError,
    commit::Fachstand,
    domain::ProjectId,
    plattform::objekte::Dokumentbezug,
    verfahren::{
        Erinnerung,
        abgabe::Abgabewunsch,
        fristen::{Bescheidfristen, Fristauftrag},
        profile::{Anforderungskatalog, Vorhabenprofil},
        suche::{Suchtreffer, Textquelle},
    },
    zugriff::ProjektRecht,
};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub fn verfahrensprofil() -> Result<Anforderungskatalog, CmdError> {
    Ok(Anforderungskatalog::standard()?)
}
/// Mitgelieferte Anforderungskataloge (Büroprofil und Landespakete).
#[tauri::command]
pub fn verfahrenskataloge() -> Result<Vec<Anforderungskatalog>, CmdError> {
    Ok(Anforderungskatalog::mitgeliefert()?)
}
#[tauri::command]
pub async fn verfahren_verfahrenswahl(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Option<Uuid>,
    angaben: core_lib::verfahren::landesrecht::Verfahrensangaben,
) -> Result<core_lib::verfahren::landesrecht::Verfahrenswahl, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahrenswahl(projekt_id, id, angaben).await?)
}
#[tauri::command]
pub async fn verfahren_unterschriften(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<Vec<core_lib::verfahren::landesrecht::Unterschriftsbedarf>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_unterschriften(projekt_id, id).await?)
}
#[tauri::command]
pub async fn verfahren_profil_anwenden(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    basis: i64,
    profil: Vorhabenprofil,
    katalog: Anforderungskatalog,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_profil_anwenden(projekt_id, id, basis, profil, katalog, &state.benutzer())
        .await?)
}
/// Verfahrensakte zum Projektstart: Verfahrensart und Anforderungen aus dem
/// Katalog des Landes.
#[tauri::command]
pub async fn verfahren_vorbereiten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: core_lib::verfahren::Verfahrensart,
    wohngebaeude: Option<bool>,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_vorbereiten(projekt_id, art, wohngebaeude, &state.benutzer())
        .await?)
}
/// Nachweisstände der Anforderung als Unterlage ablegen und zuordnen.
#[tauri::command]
pub async fn verfahren_nachweis_als_unterlage(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    basis: i64,
    anforderung: Uuid,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_nachweis_als_unterlage(projekt_id, id, basis, anforderung, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_abgabe(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    basis: i64,
    wunsch: Abgabewunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_abgabe_festhalten(projekt_id, id, basis, wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_externe_einreichung(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    basis: i64,
    wunsch: core_lib::verfahren::externe_einreichung::ExterneEinreichungswunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_externe_einreichung(projekt_id, id, basis, wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_xbau_erfassen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    abgabe: Uuid,
) -> Result<core_lib::xbau::postfach::Nachrichtenstand, CmdError> {
    Ok(state
        .core
        .verfahren_xbau_erfassen(projekt_id, id, abgabe, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_export(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    abgabe: Uuid,
    format: String,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    let (_, b) = state
        .core
        .verfahren_artefakt(projekt_id, id, abgabe, &format, &state.benutzer())
        .await?;
    tokio::fs::write(ziel, b)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn verfahren_planunterlagen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<Vec<core_lib::verfahren::planindex::Planunterlage>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_planunterlagen(projekt_id, id).await?)
}
#[tauri::command]
pub async fn verfahren_abweichungen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<core_lib::verfahren::abweichungen::Abweichungsabgleich, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_abweichungen(projekt_id, id).await?)
}
/// Erzeugt ein Schreiben an die Behörde; mit Ziel wird das PDF gespeichert.
#[tauri::command]
pub async fn verfahren_schreiben(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    wunsch: core_lib::verfahren::schreiben::Schreibenwunsch,
    ziel: Option<PathBuf>,
) -> Result<core_lib::verfahren::schreiben::Schreibenergebnis, CmdError> {
    let (r, pdf) = state
        .core
        .verfahren_schreiben(projekt_id, id, wunsch, &state.benutzer())
        .await?;
    if let Some(ziel) = ziel {
        tokio::fs::write(ziel, pdf)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?;
    }
    Ok(r)
}
#[tauri::command]
pub async fn verfahren_optionen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<core_lib::verfahren::optionen::Verfahrenslage, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_optionen(projekt_id, id).await?)
}
#[tauri::command]
pub async fn verfahren_folgeverfahren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    wunsch: core_lib::verfahren::optionen::Folgeverfahrenswunsch,
) -> Result<Fachstand, CmdError> {
    Ok(state
        .core
        .verfahren_folgeverfahren(projekt_id, id, wunsch, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_checkliste(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    phase: core_lib::verfahren::ausfuehrung::Phase,
) -> Result<core_lib::verfahren::ausfuehrung::Phasencheckliste, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .verfahren_checkliste(projekt_id, id, phase)
        .await?)
}
#[tauri::command]
pub async fn verfahren_erinnerungen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    heute: chrono::NaiveDate,
) -> Result<Vec<Erinnerung>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_erinnerungen(projekt_id, heute).await?)
}
#[tauri::command]
pub async fn verfahren_bescheidfristen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    auftrag: Fristauftrag,
) -> Result<Vec<Bescheidfristen>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.bescheidfristen(projekt_id, &auftrag).await?)
}
/// Schreibt die laufenden Verfahrenstermine als Kalenderdatei (ICS).
#[tauri::command]
pub async fn verfahren_termine_ics(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let ics = state
        .core
        .verfahren_termine_ics(projekt_id, core_lib::kalender::heute())
        .await?;
    tokio::fs::write(ziel, ics)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn verfahren_suche(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    suche: String,
) -> Result<Vec<Suchtreffer>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.verfahren_suche(projekt_id, &suche).await?)
}
#[tauri::command]
pub async fn suchtext_speichern(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument: Dokumentbezug,
    quelle: Textquelle,
    text: String,
) -> Result<(), CmdError> {
    Ok(state
        .core
        .suchtext_speichern(projekt_id, dokument, quelle, text, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn verfahren_aktenexport(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    ziel: PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    // Fortlaufend in eine Teildatei neben dem Ziel; erst nach Erfolg umbenennen.
    let teil = teildatei(&ziel);
    let datei = std::fs::File::create(&teil).map_err(|e| CoreError::Storage(e.to_string()))?;
    let ergebnis = state
        .core
        .verfahren_aktenexport_in(projekt_id, datei)
        .await
        .and_then(|f| f.sync_all().map_err(|e| CoreError::Storage(e.to_string())));
    teildatei_abschliessen(&teil, &ziel, ergebnis).await
}
/// Pfad der Teildatei, in die ein Export geschrieben wird (`Ziel.zip.teil`).
pub(crate) fn teildatei(ziel: &std::path::Path) -> PathBuf {
    let mut name = ziel
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".teil");
    ziel.with_file_name(name)
}
/// Benennt die Teildatei nach Erfolg in das Ziel um, sonst wird sie entfernt.
pub(crate) async fn teildatei_abschliessen(
    teil: &std::path::Path,
    ziel: &std::path::Path,
    ergebnis: Result<(), CoreError>,
) -> Result<(), CmdError> {
    match ergebnis {
        Ok(()) => Ok(tokio::fs::rename(teil, ziel)
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?),
        Err(e) => {
            let _ = tokio::fs::remove_file(teil).await;
            Err(e.into())
        }
    }
}
#[tauri::command]
pub async fn verfahren_ocr(png_base64: String) -> Result<String, CmdError> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use tokio::io::AsyncWriteExt;
    if png_base64.len() > 24 * 1024 * 1024 {
        return Err(CoreError::InvalidInput("OCR-Bild höchstens 18 MB".into()).into());
    }
    let data = STANDARD
        .decode(png_base64)
        .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
    if data.len() < 24 || !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(CoreError::InvalidInput("OCR benötigt ein PNG-Bild".into()).into());
    }
    let width = u32::from_be_bytes(data[16..20].try_into().expect("Länge geprüft"));
    let height = u32::from_be_bytes(data[20..24].try_into().expect("Länge geprüft"));
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 12_000_000 {
        return Err(
            CoreError::InvalidInput("OCR-Bild benötigt 1 bis 12 Millionen Pixel".into()).into(),
        );
    }
    static OCR: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    let _slot = OCR
        .try_acquire()
        .map_err(|_| CoreError::InvalidInput("Eine OCR-Anfrage läuft bereits".into()))?;
    // stdin/stdout statt temporärer Projektdaten; keinerlei Shell-Auswertung.
    let mut child=tokio::process::Command::new("tesseract").args(["stdin","stdout","-l","deu+eng"]).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).kill_on_drop(true).spawn().map_err(|_|CoreError::InvalidInput("OCR benötigt lokal installiertes Tesseract mit den Sprachpaketen Deutsch und Englisch.".into()))?;
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| CoreError::Storage("OCR-Eingabe fehlt".into()))?;
    let output = tokio::time::timeout(std::time::Duration::from_secs(60), async move {
        input.write_all(&data).await?;
        drop(input);
        child.wait_with_output().await
    })
    .await
    .map_err(|_| CoreError::InvalidInput("OCR-Zeitlimit erreicht".into()))?
    .map_err(|e| CoreError::Storage(e.to_string()))?;
    if !output.status.success() {
        return Err(CoreError::InvalidInput(
            "OCR fehlgeschlagen. Tesseract-Sprachpakete deu/eng und Bildgröße prüfen.".into(),
        )
        .into());
    }
    if output.stdout.len() > 2 * 1024 * 1024 {
        return Err(CoreError::InvalidInput("OCR-Text größer als 2 MB".into()).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    #[tokio::test]
    async fn ocr_prueft_bilder_und_erkennt_echten_beleg() {
        assert!(
            verfahren_ocr(STANDARD.encode(b"Keine Bilddatei"))
                .await
                .is_err()
        );
        let mut uebergross = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        uebergross.extend(100_000u32.to_be_bytes());
        uebergross.extend(100_000u32.to_be_bytes());
        assert!(verfahren_ocr(STANDARD.encode(uebergross)).await.is_err());
        if std::env::var_os("BAUPROJEKT_TEST_OCR").is_none() {
            return;
        }
        let text = verfahren_ocr(STANDARD.encode(include_bytes!("../../tests/ocr-beleg.png")))
            .await
            .unwrap();
        assert!(text.contains("Bauantrag Musterstrasse"), "{text}");
        assert!(text.contains("Lageplan Version 2"), "{text}");
    }
}
