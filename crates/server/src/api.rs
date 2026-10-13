//! HTTP-Schicht: nur (De-)Serialisierung und Fehlerabbildung, keine Geschäftslogik.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use core_lib::benutzer::Benutzer;
use core_lib::cad::{Laengeneinheit, Messung, Punkt};
use core_lib::calc::{Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::domain::{DocumentLock, Protokolleintrag};
use core_lib::formular::Feldwert;
use core_lib::kunden::{Kunde, KundeId, Kundendaten};
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use core_lib::license::Lizenzstatus;
use core_lib::service::{CadPlan, FormularAnalyse};
use core_lib::xbau::{Bauantrag, Buero, Codeliste, GespeicherterBauantrag};
use core_lib::{CoreError, CoreServices};
use serde::Deserialize;
use uuid::Uuid;

pub struct ApiError(CoreError);

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            CoreError::NotFound(_) => StatusCode::NOT_FOUND,
            CoreError::VersionConflict { .. } => StatusCode::CONFLICT,
            CoreError::InvalidInput(_) | CoreError::RulePackMissing(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            CoreError::License(_) => StatusCode::PAYMENT_REQUIRED,
            CoreError::Forbidden(_) => StatusCode::FORBIDDEN,
            CoreError::Gesperrt { .. } => StatusCode::LOCKED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if status.is_server_error() {
            tracing::error!(error = %self.0, "Anfrage fehlgeschlagen");
        }
        (
            status,
            Json(serde_json::json!({ "fehler": self.0.to_string() })),
        )
            .into_response()
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

/// Zustand der Handler. `akteur` ist vorerst das technische Konto; mit der
/// Server-Anmeldung kommt der Benutzer aus dem Token der Anfrage.
#[derive(Clone)]
pub struct ApiState {
    pub core: CoreServices,
    pub akteur: Benutzer,
}

pub fn router(core: CoreServices, akteur: Benutzer) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/projekte", get(list_projects).post(create_project))
        .route("/api/projekte/{id}", get(get_project).put(update_project))
        .route(
            "/api/dokumente/{id}/formular",
            get(formular_analysieren).post(formular_ausfuellen),
        )
        .route("/api/projekte/{id}/protokoll", get(protokoll))
        .route(
            "/api/projekte/{id}/bauherren",
            get(bauherren).put(bauherren_setzen),
        )
        .route("/api/kunden", get(kunden).post(kunde_anlegen))
        .route(
            "/api/kunden/{id}",
            get(kunde).put(kunde_aendern).delete(kunde_loeschen),
        )
        .route("/api/kunden/{id}/projekte", get(kunde_projekte))
        .route("/api/dokumente/{id}/versionen", get(versionen))
        .route(
            "/api/dokumente/{id}/sperre",
            post(sperren).delete(entsperren),
        )
        .route("/api/dokumente/{id}/plan", get(cad_plan))
        .route("/api/lizenz", get(lizenz).put(lizenz_einspielen))
        .route(
            "/api/projekte/{id}/bauantrag",
            get(bauantrag).put(bauantrag_speichern),
        )
        .route("/api/projekte/{id}/xbau", get(xbau_exportieren))
        .route("/api/buero", get(buero).put(buero_speichern))
        .route("/api/xbau/codelisten", get(codelisten))
        .route("/api/lbo/regelpakete", get(rule_packs))
        .route("/api/berechnung/abstandsflaeche", post(abstandsflaeche))
        .route("/api/berechnung/mass-der-nutzung", post(mass_der_nutzung))
        .route("/api/berechnung/aufmass", post(aufmass))
        .with_state(ApiState { core, akteur })
}

async fn list_projects(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Vec<Project>> {
    Ok(Json(core.list_projects().await?))
}

async fn create_project(
    State(ApiState { core, akteur }): State<ApiState>,
    Json(body): Json<NewProject>,
) -> Result<(StatusCode, Json<Project>), ApiError> {
    let project = core.create_project(body, &akteur).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn get_project(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Project> {
    let project = core.get_project(ProjectId(id)).await?;
    Ok(Json(project))
}

async fn update_project(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<ProjectUpdate>,
) -> ApiResult<Project> {
    body.id = ProjectId(id);
    let project = core.update_project(body, &akteur).await?;
    Ok(Json(project))
}

async fn formular_analysieren(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<FormularAnalyse> {
    Ok(Json(core.formular_analysieren(DocumentId(id)).await?))
}

async fn formular_ausfuellen(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(werte): Json<BTreeMap<String, Feldwert>>,
) -> Result<(StatusCode, Json<DocumentVersion>), ApiError> {
    let version = core
        .formular_ausfuellen(DocumentId(id), &werte, &akteur)
        .await?;
    Ok((StatusCode::CREATED, Json(version)))
}

#[derive(Deserialize)]
struct ProtokollQuery {
    limit: Option<i64>,
}

async fn protokoll(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<ProtokollQuery>,
) -> ApiResult<Vec<Protokolleintrag>> {
    Ok(Json(
        core.protokoll(ProjectId(id), q.limit.unwrap_or(200))
            .await?,
    ))
}

async fn versionen(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<DocumentVersion>> {
    Ok(Json(core.document_history(DocumentId(id)).await?))
}

async fn sperren(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<DocumentLock> {
    Ok(Json(core.dokument_sperren(DocumentId(id), &akteur).await?))
}

async fn entsperren(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    core.dokument_entsperren(DocumentId(id), &akteur).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn lizenz(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Lizenzstatus> {
    Ok(Json(core.lizenz().await?))
}

/// Body: Inhalt der Lizenzdatei.
async fn lizenz_einspielen(
    State(ApiState { core, akteur }): State<ApiState>,
    datei: axum::body::Bytes,
) -> ApiResult<Lizenzstatus> {
    Ok(Json(core.lizenz_einspielen(&datei, &akteur).await?))
}

async fn cad_plan(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<CadPlan> {
    Ok(Json(core.cad_plan(DocumentId(id)).await?))
}

#[derive(Deserialize)]
struct AufmassRequest {
    punkte: Vec<Punkt>,
    geschlossen: bool,
    einheit: Laengeneinheit,
}

async fn aufmass(
    State(ApiState { core, .. }): State<ApiState>,
    Json(req): Json<AufmassRequest>,
) -> ApiResult<Messung> {
    Ok(Json(core.messen(
        &req.punkte,
        req.geschlossen,
        req.einheit,
    )?))
}

async fn rule_packs(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Vec<RulePackInfo>> {
    Ok(Json(core.lbo.rule_packs()))
}

#[derive(Deserialize)]
struct AbstandsflaecheRequest {
    land: Bundesland,
    #[serde(flatten)]
    input: AbstandsflaecheInput,
}

async fn abstandsflaeche(
    State(ApiState { core, .. }): State<ApiState>,
    Json(req): Json<AbstandsflaecheRequest>,
) -> ApiResult<Berechnung<AbstandsflaecheErgebnis>> {
    let r = core.abstandsflaeche(req.land, &req.input)?;
    Ok(Json(r))
}

#[derive(Deserialize)]
struct MassDerNutzungRequest {
    gebiet: Baugebiet,
    #[serde(flatten)]
    eingabe: MassDerNutzung,
    festsetzungen: Option<Orientierungswerte>,
}

async fn mass_der_nutzung(
    State(ApiState { core, .. }): State<ApiState>,
    Json(req): Json<MassDerNutzungRequest>,
) -> ApiResult<Pruefergebnis> {
    let r = core.mass_der_nutzung(&req.eingabe, req.gebiet, req.festsetzungen)?;
    Ok(Json(r))
}

async fn kunden(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Vec<Kunde>> {
    Ok(Json(core.kunden_auflisten().await?))
}

async fn kunde(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Kunde> {
    Ok(Json(core.kunde(KundeId(id)).await?))
}

async fn kunde_anlegen(
    State(ApiState { core, akteur }): State<ApiState>,
    Json(body): Json<Kundendaten>,
) -> Result<(StatusCode, Json<Kunde>), ApiError> {
    let k = core.kunde_anlegen(body, &akteur).await?;
    Ok((StatusCode::CREATED, Json(k)))
}

#[derive(Deserialize)]
struct KundeAendern {
    daten: Kundendaten,
    /// Zuletzt gelesene Version.
    version: i64,
}

async fn kunde_aendern(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<KundeAendern>,
) -> ApiResult<Kunde> {
    Ok(Json(
        core.kunde_aendern(KundeId(id), body.daten, body.version, &akteur)
            .await?,
    ))
}

#[derive(Deserialize)]
struct VersionQuery {
    version: i64,
}

async fn kunde_loeschen(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<VersionQuery>,
) -> Result<StatusCode, ApiError> {
    core.kunde_loeschen(KundeId(id), q.version, &akteur).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn kunde_projekte(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Project>> {
    Ok(Json(core.kunde_projekte(KundeId(id)).await?))
}

async fn bauherren(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Kunde>> {
    Ok(Json(core.bauherren(ProjectId(id)).await?))
}

#[derive(Deserialize)]
struct BauherrenSetzen {
    kunden: Vec<KundeId>,
    /// Zuletzt gelesene Projektversion.
    version: i64,
}

async fn bauherren_setzen(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<BauherrenSetzen>,
) -> ApiResult<Project> {
    Ok(Json(
        core.bauherren_setzen(ProjectId(id), body.kunden, body.version, &akteur)
            .await?,
    ))
}

async fn bauantrag(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<GespeicherterBauantrag> {
    Ok(Json(core.bauantrag(ProjectId(id)).await?))
}

#[derive(Deserialize)]
struct BauantragSpeichern {
    antrag: Bauantrag,
    /// Zuletzt gelesene Version (0 = neu).
    version: i64,
}

async fn bauantrag_speichern(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<BauantragSpeichern>,
) -> ApiResult<GespeicherterBauantrag> {
    Ok(Json(
        core.bauantrag_speichern(ProjectId(id), body.antrag, body.version, &akteur)
            .await?,
    ))
}

async fn buero(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Buero> {
    Ok(Json(core.buero().await?))
}

async fn buero_speichern(
    State(ApiState { core, akteur }): State<ApiState>,
    Json(body): Json<Buero>,
) -> ApiResult<Buero> {
    Ok(Json(core.buero_speichern(body, &akteur).await?))
}

async fn codelisten(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Vec<Codeliste>> {
    Ok(Json(core.codelisten.alle().cloned().collect()))
}

/// Liefert das ZIP-Paket als Download.
async fn xbau_exportieren(
    State(ApiState { core, akteur }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let export = core.xbau_exportieren(ProjectId(id), &akteur).await?;
    let disposition = format!(
        "attachment; filename=\"bauantrag.zip\"; filename*=UTF-8''{}",
        prozent_kodieren(&export.dateiname)
    );
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/zip".to_string(),
            ),
            (axum::http::header::CONTENT_DISPOSITION, disposition),
        ],
        export.zip,
    )
        .into_response())
}

/// Prozentkodierung für `filename*` (RFC 5987).
fn prozent_kodieren(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
