//! Anmeldedialog und Benutzerwechsel (dünner Adapter zu `core_lib::anmeldung`).
//!
//! Netzwerkbetrieb: Beim Start ist niemand angemeldet
//! ([`Benutzer::nicht_angemeldet`]); die Oberfläche zeigt den Anmeldedialog.
//! Nach Erstkennwort oder Übergang bleibt der Benutzer „ausstehend“, bis er
//! ein eigenes Kennwort gesetzt hat. Einzelplatz: Anmeldung beim Start über
//! das Betriebssystem, ohne Kennwort und ohne Abmelden.

use core_lib::anmeldung::{Anmeldeoptionen, Anmeldung};
use core_lib::benutzer::{Benutzer, BenutzerId};
use serde::Serialize;
use tauri::State;

use crate::bootstrap::AppState;
use crate::commands::CmdError;

type CmdResult<T> = Result<T, CmdError>;

#[derive(Debug, Serialize)]
pub struct Anmeldestatus {
    /// Netzwerkbetrieb: Anmeldung mit Anmeldename und Kennwort.
    pub kennwortanmeldung: bool,
    /// Vollständig angemeldet.
    pub benutzer: Option<Benutzer>,
    /// Angemeldet, muss aber vor jeder Arbeit ein eigenes Kennwort setzen.
    pub kennwort_aendern: Option<Benutzer>,
    /// Für die Änderung ist das bisherige (Erst-)Kennwort anzugeben.
    pub bisheriges_kennwort_noetig: bool,
    /// Nur solange niemand angemeldet ist.
    pub optionen: Option<Anmeldeoptionen>,
    /// Hinweis für Eingabefelder; geprüft wird im Core.
    pub kennwort_mindestlaenge: usize,
}

fn ausstehend(state: &AppState) -> Option<Benutzer> {
    state
        .ausstehend
        .read()
        .expect("Anmelde-Lock vergiftet")
        .clone()
}

fn setzen(state: &AppState, benutzer: Benutzer, ausstehend: Option<Benutzer>) {
    *state.benutzer.write().expect("Benutzer-Lock vergiftet") = benutzer;
    *state.ausstehend.write().expect("Anmelde-Lock vergiftet") = ausstehend;
}

async fn status(state: &AppState) -> CmdResult<Anmeldestatus> {
    let benutzer = state.benutzer();
    let offen = ausstehend(state);
    let optionen = if state.kennwortanmeldung() && !benutzer.ist_angemeldet() && offen.is_none() {
        Some(state.core.anmeldeoptionen(&whoami::username()).await?)
    } else {
        None
    };
    Ok(Anmeldestatus {
        kennwortanmeldung: state.kennwortanmeldung(),
        bisheriges_kennwort_noetig: offen.as_ref().is_some_and(|b| b.kennwort.vorhanden()),
        benutzer: benutzer.ist_angemeldet().then_some(benutzer),
        kennwort_aendern: offen,
        optionen,
        kennwort_mindestlaenge: core_lib::kennwort::MINDESTLAENGE,
    })
}

/// Übernimmt eine geprüfte Anmeldung in den Zustand des Arbeitsplatzes.
async fn uebernehmen(state: &AppState, a: Anmeldung) -> CmdResult<Anmeldestatus> {
    if a.kennwort_aendern {
        setzen(state, Benutzer::nicht_angemeldet(), Some(a.benutzer));
    } else {
        arbeitsplatz_melden(state, &a.benutzer).await;
        setzen(state, a.benutzer, None);
    }
    tracing::info!("Anmeldung übernommen");
    status(state).await
}

/// Lizenz-Arbeitsplatz mit dem Namen des angemeldeten Benutzers melden. Ein
/// Fehler blockiert die Anmeldung nicht; der Lizenzstatus zeigt ihn an.
async fn arbeitsplatz_melden(state: &AppState, b: &Benutzer) {
    if let Err(e) = state
        .core
        .license
        .arbeitsplatz_belegen(&state.arbeitsplatz, &b.anzeigename)
        .await
    {
        tracing::warn!(fehler = %e, "Arbeitsplatz nach der Anmeldung nicht gemeldet");
    }
}

fn nur_netzwerk(state: &AppState) -> CmdResult<()> {
    if state.kennwortanmeldung() {
        Ok(())
    } else {
        Err(CmdError {
            art: "ungueltig",
            meldung:
                "Im Einzelplatzbetrieb erfolgt die Anmeldung über das Benutzerkonto des Rechners."
                    .into(),
        })
    }
}

#[tauri::command]
pub async fn anmeldestatus(state: State<'_, AppState>) -> CmdResult<Anmeldestatus> {
    status(&state).await
}

#[tauri::command]
pub async fn anmelden(
    state: State<'_, AppState>,
    anmeldename: String,
    kennwort: String,
) -> CmdResult<Anmeldestatus> {
    nur_netzwerk(&state)?;
    let a = state
        .core
        .anmelden_mit_kennwort(&anmeldename, &kennwort)
        .await?;
    uebernehmen(&state, a).await
}

/// Einmaliger Übergang über das Benutzerkonto dieses Rechners.
#[tauri::command]
pub async fn anmelden_uebergang(state: State<'_, AppState>) -> CmdResult<Anmeldestatus> {
    nur_netzwerk(&state)?;
    let a = state
        .core
        .anmelden_uebergang(&whoami::username(), &whoami::realname())
        .await?;
    uebernehmen(&state, a).await
}

/// Eigenes Kennwort setzen (nach Erstkennwort/Übergang) oder ändern.
#[tauri::command]
pub async fn kennwort_aendern(
    state: State<'_, AppState>,
    bisher: Option<String>,
    neu: String,
) -> CmdResult<Anmeldestatus> {
    let wer = match ausstehend(&state) {
        Some(b) => b,
        None => state.benutzer(),
    };
    let b = state
        .core
        .kennwort_aendern(&wer, bisher.as_deref(), &neu)
        .await?;
    arbeitsplatz_melden(&state, &b).await;
    setzen(&state, b, None);
    status(&state).await
}

/// Abmelden bzw. Benutzer wechseln (nur Netzwerkbetrieb). Die Oberfläche
/// prüft vorher offene Eingaben und lädt danach neu.
#[tauri::command]
pub async fn abmelden(state: State<'_, AppState>) -> CmdResult<Anmeldestatus> {
    nur_netzwerk(&state)?;
    setzen(&state, Benutzer::nicht_angemeldet(), None);
    arbeitsplatz_melden(&state, &Benutzer::nicht_angemeldet()).await;
    status(&state).await
}

/// Erstkennwort vergeben oder zurücksetzen (Administrator).
#[tauri::command]
pub async fn erstkennwort_vergeben(
    state: State<'_, AppState>,
    benutzer_id: BenutzerId,
    erstkennwort: String,
) -> CmdResult<Benutzer> {
    nur_netzwerk(&state)?;
    Ok(state
        .core
        .erstkennwort_vergeben(benutzer_id, &erstkennwort, &state.benutzer())
        .await?)
}
