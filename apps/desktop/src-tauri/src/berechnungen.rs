//! Dieselben Fachkerne wie im Projektserver; keine lokale Rechenlogik.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    berechnungen::{BerechnungsVorlage, FachErgebnis, FachPlan},
    domain::ProjectId,
    zugriff::ProjektRecht,
};
use tauri::State;
#[tauri::command]
pub async fn planaufmass_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    wunsch: core_lib::planaufmass::Planexportwunsch,
    ziel: std::path::PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let bytes = state.core.planaufmass_export(projekt_id, &wunsch).await?;
    tokio::fs::write(ziel, bytes)
        .await
        .map_err(|e| core_lib::CoreError::Storage(e.to_string()))?;
    Ok(())
}
#[tauri::command]
pub async fn planaufmass_diagnose(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<core_lib::planaufmass::Planpruefung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_diagnose(projekt_id, &plan).await?)
}
#[tauri::command]
pub fn planaufmass_flaechenoperation(
    a: core_lib::planaufmass::Plangeometrie,
    b: core_lib::planaufmass::Plangeometrie,
    operation: core_lib::planaufmass::Flaechenoperation,
) -> Result<Vec<core_lib::planaufmass::Plangeometrie>, CmdError> {
    Ok(core_lib::planaufmass::flaechenoperation(&a, &b, operation)?)
}
#[tauri::command]
pub async fn planaufmass_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<Vec<core_lib::planaufmass::Planmesswert>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_pruefen(projekt_id, &plan).await?)
}
#[tauri::command]
pub fn planaufmass_ziele(plan: FachPlan) -> Result<Vec<core_lib::planaufmass::Planziel>, CmdError> {
    Ok(core_lib::planaufmass::planziele(&plan)?)
}
#[tauri::command]
pub async fn planaufmass_einsetzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: FachPlan,
) -> Result<FachPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_einsetzen(projekt_id, plan).await?)
}
#[tauri::command]
pub async fn planaufmass_raumbuch(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass_id: uuid::Uuid,
    revision: i64,
    zuordnung: Option<Vec<core_lib::planaufmass::Zeilenzuordnung>>,
    raumbuch: core_lib::plattform::objekte::Raumbuch,
) -> Result<core_lib::plattform::objekte::Raumbuch, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_raumbuch(
            projekt_id,
            aufmass_id,
            revision,
            &zuordnung.unwrap_or_default(),
            raumbuch,
        )
        .await?)
}
#[tauri::command]
pub async fn planaufmass_raumbuch_abgleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass_id: uuid::Uuid,
    revision: i64,
    raumbuch: core_lib::plattform::objekte::Raumbuch,
) -> Result<Vec<core_lib::planaufmass::Abgleichsfrage>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_raumbuch_abgleich(projekt_id, aufmass_id, revision, &raumbuch)
        .await?)
}
#[tauri::command]
pub fn planaufmass_abstandsziele(
    plan: core_lib::abstandsflaechen::AbstandsPlan,
) -> Vec<core_lib::planaufmass::Planziel> {
    core_lib::planaufmass::abstandsziele(&plan)
}
#[tauri::command]
pub async fn planaufmass_abstand(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::abstandsflaechen::AbstandsPlan,
) -> Result<core_lib::abstandsflaechen::AbstandsPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_abstand(projekt_id, plan).await?)
}
/// Linien „Abstandsfläche“ und Flurstücksgrenzen eines Lageplans übernehmen.
#[tauri::command]
pub async fn abstandslinien_uebernehmen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::abstandsflaechen::AbstandsPlan,
    aufmass: core_lib::planaufmass::Aufmassstand,
) -> Result<core_lib::planaufmass::AbstandslinienUebernahme, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .abstandslinien_uebernehmen(projekt_id, plan, aufmass)
        .await?)
}
#[tauri::command]
pub async fn berechnungsvorlagen() -> Result<Vec<BerechnungsVorlage>, CmdError> {
    Ok(core_lib::berechnungen::vorlagen()?)
}
#[tauri::command]
pub async fn planaufmass_aufgaben(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::planaufmass::Planaufmass,
) -> Result<Vec<core_lib::planaufmass::Planaufgabe>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planaufmass_aufgaben(projekt_id, &plan).await?)
}
#[tauri::command]
pub async fn planquellen_zustand(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<Vec<core_lib::planaufmass::Planquellenhinweis>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state.core.planquellen_zustand(projekt_id).await?)
}
#[tauri::command]
pub async fn planaufmass_raumbuch_einsetzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    raumbuch: core_lib::plattform::objekte::Raumbuch,
) -> Result<core_lib::plattform::objekte::Raumbuch, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_raumbuch_einsetzen(projekt_id, raumbuch)
        .await?)
}
#[tauri::command]
pub fn planaufmass_werkzeugkiste() -> Vec<core_lib::planaufmass::Werkzeugvorlage> {
    core_lib::planaufmass::werkzeugkiste()
}
#[tauri::command]
pub async fn planaufmass_flurstuecke(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    infos: core_lib::grundstuecksinformationen::Grundstuecksinformationen,
) -> Result<core_lib::grundstuecksinformationen::Grundstuecksinformationen, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_flurstuecke(projekt_id, infos)
        .await?)
}
#[tauri::command]
pub async fn planaufmass_abzuege(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass: Option<core_lib::planaufmass::Aufmassstand>,
    zuordnung: Option<Vec<core_lib::planaufmass::Zeilenzuordnung>>,
    ermittlung: core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung,
) -> Result<core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_abzuege(
            projekt_id,
            aufmass,
            &zuordnung.unwrap_or_default(),
            ermittlung,
        )
        .await?)
}
#[tauri::command]
pub async fn planaufmass_abzuege_abgleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass: core_lib::planaufmass::Aufmassstand,
    ermittlung: core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung,
) -> Result<Vec<core_lib::planaufmass::Abgleichsfrage>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_abzuege_abgleich(projekt_id, aufmass, &ermittlung)
        .await?)
}
#[tauri::command]
pub async fn planaufmass_bauteile(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    schluessel: String,
    aufmass: Option<core_lib::planaufmass::Aufmassstand>,
    zuordnung: Option<Vec<core_lib::planaufmass::Zeilenzuordnung>>,
    aufstellung: core_lib::flaechen::FlaechenAufstellung,
) -> Result<core_lib::flaechen::FlaechenAufstellung, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_bauteile(
            projekt_id,
            &schluessel,
            aufmass,
            &zuordnung.unwrap_or_default(),
            aufstellung,
        )
        .await?)
}
#[tauri::command]
pub async fn planaufmass_bauteile_abgleich(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    schluessel: String,
    aufmass: core_lib::planaufmass::Aufmassstand,
    aufstellung: core_lib::flaechen::FlaechenAufstellung,
) -> Result<Vec<core_lib::planaufmass::Abgleichsfrage>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_bauteile_abgleich(projekt_id, &schluessel, aufmass, &aufstellung)
        .await?)
}
#[tauri::command]
pub async fn planaufmass_baugrenzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    aufmass: core_lib::planaufmass::Aufmassstand,
) -> Result<core_lib::planaufmass::BaugrenzenUebernahme, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Bearbeiten)
        .await?;
    Ok(state
        .core
        .planaufmass_baugrenzen(projekt_id, aufmass, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn cadaufmass_uebernehmen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> Result<core_lib::planaufmass::CadUebernahme, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Bearbeiten)
        .await?;
    Ok(state
        .core
        .cadaufmass_uebernehmen(projekt_id, &state.benutzer())
        .await?)
}
#[tauri::command]
pub fn planaufmass_gebaeudeziele(
    plan: core_lib::plattform::objekte::Gebaeude,
) -> Vec<core_lib::planaufmass::Planziel> {
    core_lib::planaufmass::gebaeudeziele(&plan)
}
#[tauri::command]
pub async fn planaufmass_gebaeude(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: uuid::Uuid,
    plan: core_lib::plattform::objekte::Gebaeude,
) -> Result<core_lib::plattform::objekte::Gebaeude, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .planaufmass_gebaeude(projekt_id, id, plan)
        .await?)
}
#[derive(serde::Serialize)]
pub struct Vorschau {
    plan: FachPlan,
    ergebnis: FachErgebnis,
}
#[tauri::command]
pub async fn fachberechnung_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: FachPlan,
) -> Result<Vorschau, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    let (plan, ergebnis) = state.core.berechnung_pruefen(projekt_id, plan).await?;
    Ok(Vorschau { plan, ergebnis })
}
#[tauri::command]
pub async fn berechnung_aus_raumbuch(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    id: uuid::Uuid,
    modul: core_lib::berechnungen::BerechnungsArt,
) -> Result<FachPlan, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .berechnung_aus_raumbuch(projekt_id, id, modul)
        .await?)
}
/// Führt die Räume einer Berechnung dem aktuellen Raumbuch nach; `None`, wenn
/// die Berechnung bereits auf dem aktuellen Stand ist.
#[tauri::command]
pub async fn berechnung_raumbuch_nachfuehren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    mut plan: FachPlan,
) -> Result<Option<FachPlan>, CmdError> {
    state
        .core
        .projekt_zugriff(projekt_id, &state.benutzer(), ProjektRecht::Lesen)
        .await?;
    Ok(state
        .core
        .berechnung_raumbuch_nachfuehren(projekt_id, &mut plan)
        .await?
        .then_some(plan))
}
