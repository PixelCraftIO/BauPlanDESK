//! HTTP-Schicht: nur (De-)Serialisierung und Fehlerabbildung, keine Geschäftslogik.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use core_lib::calc::{Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::formular::Feldwert;
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use core_lib::service::FormularAnalyse;
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

pub fn router(core: CoreServices) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/projekte", get(list_projects).post(create_project))
        .route("/api/projekte/{id}", get(get_project).put(update_project))
        .route(
            "/api/dokumente/{id}/formular",
            get(formular_analysieren).post(formular_ausfuellen),
        )
        .route("/api/lbo/regelpakete", get(rule_packs))
        .route("/api/berechnung/abstandsflaeche", post(abstandsflaeche))
        .route("/api/berechnung/mass-der-nutzung", post(mass_der_nutzung))
        .with_state(core)
}

async fn list_projects(State(core): State<CoreServices>) -> ApiResult<Vec<Project>> {
    Ok(Json(core.list_projects().await?))
}

async fn create_project(
    State(core): State<CoreServices>,
    Json(body): Json<NewProject>,
) -> Result<(StatusCode, Json<Project>), ApiError> {
    let project = core.create_project(body).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn get_project(State(core): State<CoreServices>, Path(id): Path<Uuid>) -> ApiResult<Project> {
    let project = core.get_project(ProjectId(id)).await?;
    Ok(Json(project))
}

async fn update_project(
    State(core): State<CoreServices>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<ProjectUpdate>,
) -> ApiResult<Project> {
    body.id = ProjectId(id);
    let project = core.update_project(body).await?;
    Ok(Json(project))
}

async fn formular_analysieren(
    State(core): State<CoreServices>,
    Path(id): Path<Uuid>,
) -> ApiResult<FormularAnalyse> {
    Ok(Json(core.formular_analysieren(DocumentId(id)).await?))
}

async fn formular_ausfuellen(
    State(core): State<CoreServices>,
    Path(id): Path<Uuid>,
    Json(werte): Json<BTreeMap<String, Feldwert>>,
) -> Result<(StatusCode, Json<DocumentVersion>), ApiError> {
    // TODO: Benutzer aus der Authentifizierung übernehmen (Rollenmanagement).
    let version = core
        .formular_ausfuellen(DocumentId(id), &werte, "server")
        .await?;
    Ok((StatusCode::CREATED, Json(version)))
}

async fn rule_packs(State(core): State<CoreServices>) -> ApiResult<Vec<RulePackInfo>> {
    Ok(Json(core.lbo.rule_packs()))
}

#[derive(Deserialize)]
struct AbstandsflaecheRequest {
    land: Bundesland,
    #[serde(flatten)]
    input: AbstandsflaecheInput,
}

async fn abstandsflaeche(
    State(core): State<CoreServices>,
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
    State(core): State<CoreServices>,
    Json(req): Json<MassDerNutzungRequest>,
) -> ApiResult<Pruefergebnis> {
    let r = core.mass_der_nutzung(&req.eingabe, req.gebiet, req.festsetzungen)?;
    Ok(Json(r))
}
