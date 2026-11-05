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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.nachweis_wiederholen(projekt_id, id).await?)
}
#[tauri::command]
pub async fn raumbuch_summen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
) -> Result<RaumSummen, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
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
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .checkliste_anwenden(projekt_id, vorlage, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn projektuebersicht(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<core_lib::projektuebersicht::Projektuebersicht, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .projektuebersicht(projekt_id, &state.benutzer())
        .await?)
}

/// Status je Navigationsbereich (offen, erledigt, leer) für die Projektnavigation.
#[tauri::command]
pub async fn bereichsstatus(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<core_lib::bereichsstatus::Bereichsstaende, CmdError> {
    Ok(state
        .core
        .bereichsstatus(projekt_id, &state.benutzer())
        .await?)
}

/// Leeren Bereich als „nicht benötigt“ kennzeichnen oder die Kennzeichnung zurücknehmen.
#[tauri::command]
pub async fn bereich_benoetigt_setzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    bereich: String,
    benoetigt: bool,
) -> Result<core_lib::bereichsstatus::Bereichsstaende, CmdError> {
    Ok(state
        .core
        .bereich_benoetigt_setzen(projekt_id, &bereich, benoetigt, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn einreichungspruefung(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    verfahren: Option<Uuid>,
) -> Result<core_lib::einreichungspruefung::Einreichungspruefung, CmdError> {
    Ok(state
        .core
        .einreichungspruefung(projekt_id, verfahren, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn gebaeude_auswerten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: Uuid,
    gebaeude: core_lib::plattform::objekte::Gebaeude,
) -> Result<core_lib::plattform::gebaeude::GebaeudeGrundlagen, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .gebaeude_auswerten(projekt_id, id, &gebaeude)
        .await?)
}
#[tauri::command]
pub async fn gebaeudezuordnung_uebertragen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    alt: String,
    neu: String,
) -> Result<usize, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .gebaeudezuordnung_uebertragen(projekt_id, &alt, &neu, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn flurstuecksverweise(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    ids: Vec<String>,
) -> Result<Vec<core_lib::plattform::verweise::Flurstuecksverweis>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.flurstuecksverweise(projekt_id, &ids).await?)
}
#[tauri::command]
pub async fn flurstuecksverweise_entfernen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    ids: Vec<String>,
) -> Result<usize, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .flurstuecksverweise_entfernen(projekt_id, &ids, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn fachobjektseite(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
    nach: Option<String>,
    limit: u32,
) -> Result<Vec<Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .fachobjektseite(projekt_id, art, nach.as_deref(), limit)
        .await?)
}
#[tauri::command]
pub async fn fachobjekt_lesen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
    id: Uuid,
) -> Result<Option<Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.fachobjekt_lesen(projekt_id, art, id).await?)
}
#[tauri::command]
pub async fn fachverlaufsseite(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    art: ObjektArt,
    id: Uuid,
    vor: Option<i64>,
    limit: u32,
) -> Result<Vec<Fachstand>, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .fachverlaufsseite(projekt_id, art, id, vor, limit)
        .await?)
}
