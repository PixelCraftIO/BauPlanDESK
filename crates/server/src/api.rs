//! HTTP-Schicht: nur (De-)Serialisierung und Fehlerabbildung, keine Geschäftslogik.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
#[path = "zugang.rs"]
pub mod zugang;
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

/// Gemeinsamer Servicezustand. Der verifizierte Benutzer kommt pro Anfrage aus der Middleware.
#[derive(Clone)]
pub struct ApiState {
    pub core: CoreServices,
}

fn routen(core: CoreServices) -> Router {
    let api = Router::new()
        .route("/sitzung", get(sitzung))
        .route("/dokumente/{id}/email/{version}", get(email_laden))
        .route(
            "/dokumente/{id}/email/{version}/anhaenge/{index}",
            post(email_anhang_ablegen),
        )
        .route(
            "/dokumentkategorien",
            get(kategorienkatalog).put(kategorienkatalog_speichern),
        )
        .route("/projekte/{id}/dokumentkategorien", get(dokumentkategorien))
        .route(
            "/dokumente/{id}/kategorie",
            axum::routing::put(dokumentkategorie_speichern),
        )
        .route(
            "/vorlagen/{hash}/kategorie",
            get(vorlagenkategorie).put(vorlagenkategorie_speichern),
        )
        .route("/projekte/{id}/status", get(projektstatus))
        .route(
            "/projekte/{id}/paketpruefung",
            axum::routing::post(paket_pruefstand),
        )
        .route(
            "/projekte/{id}/checkliste",
            axum::routing::post(checkliste_anwenden),
        )
        .route(
            "/projekte/{id}/mitglieder",
            get(mitglieder).post(mitglied_setzen),
        )
        .route(
            "/projekte/{id}/grundstueckskontext",
            get(grundstueckskontext),
        )
        .route("/projekte/{id}/bereiche/{bereich}", post(bereich_speichern))
        .route(
            "/projekte/{id}/fachobjekte/{art}",
            get(fachobjekte).post(fachobjekt_speichern),
        )
        .route(
            "/projekte/{id}/nachweise",
            get(nachweise).post(nachweis_erstellen),
        )
        .route(
            "/projekte/{id}/nachweise/{nachweis}/freigabe",
            post(nachweis_freigeben),
        )
        .route(
            "/projekte/{id}/nachweise/{nachweis}/wiederholung",
            get(nachweis_wiederholen),
        )
        .route("/projekte/{id}/verfahrensprofil", get(verfahrensprofil))
        .route(
            "/projekte/{id}/verfahren/{verfahren}/profil",
            post(verfahren_profil),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abgabe",
            post(verfahren_abgabe),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abgaben/{abgabe}/{format}",
            get(verfahren_artefakt),
        )
        .route("/projekte/{id}/erinnerungen", get(verfahren_erinnerungen))
        .route("/projekte/{id}/aktensuche", get(verfahren_suche))
        .route("/projekte/{id}/suchtext", post(suchtext_speichern))
        .route("/projekte/{id}/aktenexport", get(verfahren_aktenexport))
        .route("/projekte", get(list_projects).post(create_project))
        .route("/projekte/{id}", get(get_project).put(update_project))
        .route(
            "/dokumente/{id}/formular",
            get(formular_analysieren).post(formular_ausfuellen),
        )
        .route("/dokumente/{id}/vorlage", post(formular_vorlage_erstellen))
        .route(
            "/projekte/{id}/einreichungen",
            get(einreichungen).post(einreichung_erstellen),
        )
        .route(
            "/projekte/{id}/einreichungsvergleich",
            get(einreichungsvergleich),
        )
        .route(
            "/einreichungen/{id}/bestaetigen",
            post(einreichung_bestaetigen),
        )
        .route("/einreichungen/{id}/zip", get(einreichung_zip))
        .route("/projekte/{id}/druckkatalog", get(druckkatalog))
        .route("/projekte/{id}/druckausgabe", post(druckausgabe))
        .route(
            "/projekte/{id}/druckausgabe/unterlage",
            post(druckausgabe_ablegen),
        )
        .route("/projekte/{id}/projektordner", get(projektordner_plan))
        .route("/projekte/{id}/protokoll", get(protokoll))
        .route(
            "/projekte/{id}/bauherren",
            get(bauherren).put(bauherren_setzen),
        )
        .route("/kunden", get(kunden).post(kunde_anlegen))
        .route(
            "/kunden/{id}",
            get(kunde).put(kunde_aendern).delete(kunde_loeschen),
        )
        .route("/kunden/{id}/projekte", get(kunde_projekte))
        .route("/dokumente/{id}/versionen", get(versionen))
        .route("/dokumente/{id}/sperre", post(sperren).delete(entsperren))
        .route("/dokumente/{id}/plan", get(cad_plan))
        .route("/lizenz", get(lizenz).put(lizenz_einspielen))
        .route(
            "/projekte/{id}/bauantrag",
            get(bauantrag).put(bauantrag_speichern),
        )
        .route("/projekte/{id}/xbau", get(xbau_exportieren))
        .route("/buero", get(buero).put(buero_speichern))
        .route("/xbau/codelisten", get(codelisten))
        .route("/lbo/regelpakete", get(rule_packs))
        .route("/berechnung/abstandsflaeche", post(abstandsflaeche))
        .route("/projekte/{id}/abstandsregeln", get(abstandsregeln))
        .route("/projekte/{id}/abstandsplan", post(abstandsplan))
        .route("/projekte/{id}/abstandscad", post(abstandscad))
        .route(
            "/projekte/{id}/abstandsnachweise/{nachweis}/{format}",
            get(abstandsexport),
        )
        .route("/berechnung/mass-der-nutzung", post(mass_der_nutzung))
        .route("/berechnung/aufmass", post(aufmass))
        .route(
            "/projekte/{id}/nutzung/pruefen",
            post(projekt_nutzung_pruefen),
        )
        .route(
            "/projekte/{id}/baugrundstuecke/pruefen",
            post(projekt_baugrundstuecke_pruefen),
        )
        .route("/berechnung/baugrenzen", post(baugrenzen_pruefen));
    Router::new()
        .route(
            "/health",
            get(bereitschaft).layer(Extension(std::sync::Arc::new(tokio::sync::Mutex::new(
                None::<(std::time::Instant, bool, bool)>,
            )))),
        )
        .nest("/api", api.clone())
        .nest("/api/v1", api)
        .with_state(ApiState { core })
}

async fn list_projects(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<Vec<Project>> {
    Ok(Json(core.projekte_fuer_benutzer(&akteur).await?))
}

async fn create_project(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(mut body): Json<ProjectUpdate>,
) -> ApiResult<Project> {
    body.id = ProjectId(id);
    let project = core.update_project(body, &akteur).await?;
    Ok(Json(project))
}

#[derive(Deserialize)]
struct ProjektordnerQuery {
    #[serde(default)]
    historie: bool,
}
async fn projektordner_plan(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<ProjektordnerQuery>,
) -> ApiResult<core_lib::projektablage::ProjektordnerPlan> {
    Ok(Json(
        core.projektordner_plan(ProjectId(id), q.historie).await?,
    ))
}

async fn formular_analysieren(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<FormularAnalyse> {
    Ok(Json(core.formular_analysieren(DocumentId(id)).await?))
}

#[derive(Deserialize)]
struct VorlageRequest {
    basis: i64,
    felder: Vec<core_lib::formular::editor::EigenesPdfFeld>,
}
async fn formular_vorlage_erstellen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<VorlageRequest>,
) -> Result<(StatusCode, Json<DocumentVersion>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            core.formular_vorlage_erstellen(DocumentId(id), body.basis, &body.felder, &akteur)
                .await?,
        ),
    ))
}

#[derive(Deserialize)]
struct FormularStandQuery {
    basis: Option<i64>,
}
async fn formular_ausfuellen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(stand): Query<FormularStandQuery>,
    Json(werte): Json<BTreeMap<String, Feldwert>>,
) -> Result<(StatusCode, Json<DocumentVersion>), ApiError> {
    let version = core
        .formular_ausfuellen_version(DocumentId(id), &werte, stand.basis, &akteur)
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<DocumentLock> {
    Ok(Json(core.dokument_sperren(DocumentId(id), &akteur).await?))
}

async fn entsperren(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<VersionQuery>,
) -> Result<StatusCode, ApiError> {
    core.kunde_loeschen(KundeId(id), q.version, &akteur).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn kunde_projekte(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Project>> {
    let mut sichtbar = Vec::new();
    for p in core.kunde_projekte(KundeId(id)).await? {
        if core
            .projekt_zugriff(p.id, &akteur, core_lib::zugriff::ProjektRecht::Lesen)
            .await
            .is_ok()
        {
            sichtbar.push(p);
        }
    }
    Ok(Json(sichtbar))
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Json(body): Json<Buero>,
) -> ApiResult<Buero> {
    Ok(Json(core.buero_speichern(body, &akteur).await?))
}

async fn codelisten(State(ApiState { core, .. }): State<ApiState>) -> ApiResult<Vec<Codeliste>> {
    Ok(Json(core.codelisten.alle().cloned().collect()))
}

/// Liefert das ZIP-Paket als Download.
async fn xbau_exportieren(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
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

async fn projekt_nutzung_pruefen(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(e): Json<core_lib::festsetzungen::NutzungsEingabe>,
) -> ApiResult<core_lib::festsetzungen::NutzungsPruefung> {
    Ok(Json(core.projekt_nutzung_pruefen(ProjectId(id), &e).await?))
}

async fn projekt_baugrundstuecke_pruefen(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(e): Json<core_lib::baugrundstuecke::BaugrundstuecksPlan>,
) -> ApiResult<core_lib::baugrundstuecke::BaugrundstuecksPruefung> {
    Ok(Json(
        core.projekt_baugrundstuecke_pruefen(ProjectId(id), &e)
            .await?,
    ))
}

async fn baugrenzen_pruefen(
    State(ApiState { core, .. }): State<ApiState>,
    Json(g): Json<core_lib::cad::baugrenzen::BaugrenzenGeometrie>,
) -> ApiResult<core_lib::cad::baugrenzen::BaugrenzenPruefung> {
    Ok(Json(core.baugrenzen_pruefen(&g)?))
}

async fn einreichungen(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::einreichung::Einreichung>> {
    Ok(Json(core.einreichungen(ProjectId(id)).await?))
}
#[derive(Deserialize)]
struct EmpfaengerQuery {
    empfaenger: String,
}
async fn einreichungsvergleich(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<EmpfaengerQuery>,
) -> ApiResult<Vec<core_lib::einreichung::DokumentVergleich>> {
    Ok(Json(
        core.einreichungs_vergleich(ProjectId(id), &q.empfaenger)
            .await?,
    ))
}
#[derive(Deserialize)]
struct PaketRequest {
    #[serde(flatten)]
    paket: core_lib::einreichung::NeuesPaket,
    vorgang: Option<Uuid>,
}
async fn einreichung_erstellen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(p): Json<PaketRequest>,
) -> Result<(StatusCode, Json<core_lib::einreichung::Einreichung>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            core.einreichung_erstellen_vorgang(ProjectId(id), p.paket, p.vorgang, &akteur)
                .await?,
        ),
    ))
}
#[derive(Deserialize)]
struct Bestaetigung {
    basis: i64,
    datum: chrono::NaiveDate,
    notiz: String,
}
async fn einreichung_bestaetigen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(b): Json<Bestaetigung>,
) -> ApiResult<core_lib::einreichung::Einreichung> {
    Ok(Json(
        core.einreichung_bestaetigen(id, b.basis, b.datum, b.notiz, &akteur)
            .await?,
    ))
}
async fn einreichung_zip(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let (_, bytes) = core.einreichung_zip(id).await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/zip".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"Einreichung-{id}.zip\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

async fn grundstueckskontext(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::grundstueckskontext::GrundstueckKontext>> {
    Ok(Json(core.grundstueckskontext(ProjectId(id)).await?))
}
#[derive(Deserialize)]
struct BereichRequest {
    basis: Project,
    aenderung: ProjectUpdate,
}
async fn bereich_speichern(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((id, bereich)): Path<(Uuid, core_lib::projektbereiche::ProjektBereich)>,
    Json(body): Json<BereichRequest>,
) -> ApiResult<Project> {
    if body.basis.id != ProjectId(id) || body.aenderung.id != ProjectId(id) {
        return Err(CoreError::InvalidInput("Projektzuordnung stimmt nicht überein".into()).into());
    }
    Ok(Json(
        core.projekt_bereich_speichern(body.basis, bereich, body.aenderung, &akteur)
            .await?,
    ))
}
async fn fachobjekte(
    State(ApiState { core, .. }): State<ApiState>,
    Path((id, art)): Path<(Uuid, core_lib::plattform::ObjektArt)>,
) -> ApiResult<Vec<core_lib::commit::Fachstand>> {
    Ok(Json(core.fachobjekte(ProjectId(id), art).await?))
}
#[derive(Deserialize)]
struct FachRequest {
    id: Uuid,
    basis: i64,
    daten: serde_json::Value,
}
async fn fachobjekt_speichern(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((projekt, art)): Path<(Uuid, core_lib::plattform::ObjektArt)>,
    Json(body): Json<FachRequest>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.fachobjekt_speichern(
            ProjectId(projekt),
            art,
            body.id,
            body.basis,
            body.daten,
            &akteur,
        )
        .await?,
    ))
}
async fn nachweise(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::plattform::nachweise::NachweisAnsicht>> {
    Ok(Json(core.nachweise(ProjectId(id)).await?))
}
#[derive(Deserialize)]
struct NachweisRequest {
    id: Uuid,
    bezeichnung: String,
    eingabe: core_lib::plattform::nachweise::NachweisEingabe,
}
async fn nachweis_erstellen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<NachweisRequest>,
) -> ApiResult<core_lib::plattform::nachweise::Nachweisstand> {
    Ok(Json(
        core.nachweis_erstellen(
            ProjectId(id),
            body.id,
            body.bezeichnung,
            body.eingabe,
            &akteur,
        )
        .await?,
    ))
}
#[derive(Deserialize)]
struct FreigabeRequest {
    bemerkung: String,
}
async fn nachweis_freigeben(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((id, n)): Path<(Uuid, Uuid)>,
    Json(body): Json<FreigabeRequest>,
) -> ApiResult<core_lib::plattform::nachweise::Freigabe> {
    Ok(Json(
        core.nachweis_freigeben(ProjectId(id), n, body.bemerkung, &akteur)
            .await?,
    ))
}
async fn nachweis_wiederholen(
    State(ApiState { core, .. }): State<ApiState>,
    Path((id, n)): Path<(Uuid, Uuid)>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(core.nachweis_wiederholen(ProjectId(id), n).await?))
}

async fn sitzung(Extension(b): Extension<Benutzer>) -> Json<Benutzer> {
    Json(b)
}
type GesundheitsCache =
    std::sync::Arc<tokio::sync::Mutex<Option<(std::time::Instant, bool, bool)>>>;
async fn bereitschaft(
    State(ApiState { core }): State<ApiState>,
    Extension(cache): Extension<GesundheitsCache>,
) -> Response {
    let mut stand = cache.lock().await;
    if stand
        .as_ref()
        .is_none_or(|(zeit, _, _)| zeit.elapsed() > std::time::Duration::from_secs(30))
    {
        let datenbank = core.repos.lizenz.einstellung("bereitschaft").await.is_ok();
        let speicher = async {
            let key = core_lib::storage::StorageKey::new(format!(
                "system/gesundheit/{}",
                Uuid::new_v4()
            ))?;
            let data = axum::body::Bytes::from_static(b"BauProjektIO Speicherpruefung");
            let hash = core_lib::storage::ContentHash::of(&data);
            core.storage.save(&key, data).await?;
            let pruefung = core.storage.load_verified(&key, &hash).await;
            let loeschen = core.storage.delete(&key).await;
            pruefung?;
            loeschen?;
            Ok::<_, core_lib::CoreError>(())
        }
        .await
        .is_ok();
        *stand = Some((std::time::Instant::now(), datenbank, speicher));
    }
    let (_, db, speicher) = stand.as_ref().expect("Gesundheitsprüfung wurde ausgeführt");
    (if *db&&*speicher {StatusCode::OK}else{StatusCode::SERVICE_UNAVAILABLE},Json(serde_json::json!({"datenbank":if *db{"bereit"}else{"nicht_bereit"},"speicher":if *speicher{"bereit"}else{"nicht_bereit"},"pruefung":"Lesen und Schreiben, Cache höchstens 30 Sekunden"}))).into_response()
}
async fn mitglieder(
    Extension(b): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::commit::Fachstand>> {
    Ok(Json(core.projektmitglieder(ProjectId(id), &b).await?))
}
#[derive(Deserialize)]
struct MitgliedRequest {
    mitglied: core_lib::zugriff::ProjektMitglied,
    basis: i64,
}
async fn mitglied_setzen(
    Extension(b): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<MitgliedRequest>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.projektmitglied_setzen(ProjectId(id), body.mitglied, body.basis, &b)
            .await?,
    ))
}

async fn projektstatus(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<core_lib::plattform::ablauf::Projektstatus> {
    Ok(Json(core.projektstatus(ProjectId(id)).await?))
}
async fn checkliste_anwenden(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(vorlage): Json<core_lib::plattform::ablauf::Checkliste>,
) -> ApiResult<usize> {
    Ok(Json(
        core.checkliste_anwenden(ProjectId(id), vorlage, &wer)
            .await?,
    ))
}

async fn paket_pruefstand(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(dokumente): Json<Vec<core_lib::einreichung::DokumentAuswahl>>,
) -> ApiResult<core_lib::einreichung::PaketPruefstand> {
    Ok(Json(
        core.paket_pruefstand(ProjectId(id), &dokumente).await?,
    ))
}

// Dokumentkategorien verwenden denselben Core wie der Desktop.
async fn kategorienkatalog(
    State(s): State<ApiState>,
) -> ApiResult<core_lib::dokumentkategorien::KategorienKatalog> {
    Ok(Json(s.core.kategorienkatalog().await?))
}
async fn kategorienkatalog_speichern(
    State(s): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Json(k): Json<core_lib::dokumentkategorien::KategorienKatalog>,
) -> ApiResult<core_lib::dokumentkategorien::KategorienKatalog> {
    Ok(Json(s.core.kategorienkatalog_speichern(k, &wer).await?))
}
async fn dokumentkategorien(
    State(s): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::dokumentkategorien::DokumentKategorie>> {
    Ok(Json(s.core.dokumentkategorien(ProjectId(id)).await?))
}
async fn dokumentkategorie_speichern(
    State(s): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
    Json(k): Json<core_lib::dokumentkategorien::KategorieZuordnung>,
) -> ApiResult<core_lib::dokumentkategorien::DokumentKategorie> {
    Ok(Json(
        s.core
            .dokumentkategorie_speichern(DocumentId(id), k, &wer)
            .await?,
    ))
}
async fn vorlagenkategorie(
    State(s): State<ApiState>,
    Path(hash): Path<core_lib::storage::ContentHash>,
) -> ApiResult<core_lib::dokumentkategorien::KategorieZuordnung> {
    Ok(Json(s.core.vorlagenkategorie(&hash).await?))
}
async fn vorlagenkategorie_speichern(
    State(s): State<ApiState>,
    Path(hash): Path<core_lib::storage::ContentHash>,
    Extension(wer): Extension<Benutzer>,
    Json(k): Json<core_lib::dokumentkategorien::KategorieZuordnung>,
) -> ApiResult<core_lib::dokumentkategorien::KategorieZuordnung> {
    Ok(Json(
        s.core.vorlagenkategorie_speichern(&hash, k, &wer).await?,
    ))
}

async fn email_laden(
    State(s): State<ApiState>,
    Path((id, version)): Path<(Uuid, i64)>,
) -> ApiResult<core_lib::email::ArchivierteEmail> {
    Ok(Json(
        s.core.archivierte_email(DocumentId(id), version).await?,
    ))
}
async fn email_anhang_ablegen(
    State(s): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((id, version, index)): Path<(Uuid, i64, usize)>,
) -> ApiResult<core_lib::domain::DocumentVersion> {
    Ok(Json(
        s.core
            .email_anhang_ablegen(DocumentId(id), version, index, &wer)
            .await?,
    ))
}

async fn druckkatalog(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<core_lib::druckausgabe::Druckkatalog> {
    Ok(Json(core.druckkatalog(ProjectId(id)).await?))
}
#[derive(Deserialize)]
struct Druckablage {
    name: String,
    vorgang: Uuid,
}
async fn druckausgabe_ablegen(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path(id): Path<Uuid>,
    Query(w): Query<Druckablage>,
    pdf: axum::body::Bytes,
) -> ApiResult<core_lib::domain::DocumentVersion> {
    Ok(Json(
        core.druckpdf_ablegen(ProjectId(id), &w.name, pdf, w.vorgang, &wer)
            .await?,
    ))
}
async fn druckausgabe(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(auftrag): Json<core_lib::druckausgabe::Druckauftrag>,
) -> Result<Response, ApiError> {
    let d = core.druckausgabe(ProjectId(id), auftrag).await?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/pdf"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"Druckausgabe.pdf\"",
            ),
        ],
        d.pdf,
    )
        .into_response())
}

async fn verfahrensprofil() -> ApiResult<core_lib::verfahren::profile::Anforderungskatalog> {
    Ok(Json(
        core_lib::verfahren::profile::Anforderungskatalog::standard()?,
    ))
}
#[derive(Deserialize)]
struct Profilwunsch {
    basis: i64,
    profil: core_lib::verfahren::profile::Vorhabenprofil,
    katalog: core_lib::verfahren::profile::Anforderungskatalog,
}
async fn verfahren_profil(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<Profilwunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.verfahren_profil_anwenden(ProjectId(p), id, w.basis, w.profil, w.katalog, &wer)
            .await?,
    ))
}
#[derive(Deserialize)]
struct Abgabewunsch {
    basis: i64,
    wunsch: core_lib::verfahren::abgabe::Abgabewunsch,
}
async fn verfahren_abgabe(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<Abgabewunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.verfahren_abgabe_festhalten(ProjectId(p), id, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
async fn verfahren_artefakt(
    State(ApiState { core }): State<ApiState>,
    Path((p, id, abgabe, format)): Path<(Uuid, Uuid, Uuid, String)>,
) -> Result<Response, ApiError> {
    let (_, b) = core
        .verfahren_artefakt(ProjectId(p), id, abgabe, &format)
        .await?;
    let typ = if format == "pdf" {
        "application/pdf"
    } else {
        "application/zip"
    };
    Ok(([(axum::http::header::CONTENT_TYPE, typ)], b).into_response())
}
#[derive(Deserialize)]
struct Erinnerungsfrage {
    heute: chrono::NaiveDate,
}
async fn verfahren_erinnerungen(
    State(ApiState { core }): State<ApiState>,
    Path(p): Path<Uuid>,
    Query(q): Query<Erinnerungsfrage>,
) -> ApiResult<Vec<core_lib::verfahren::Erinnerung>> {
    Ok(Json(
        core.verfahren_erinnerungen(ProjectId(p), q.heute).await?,
    ))
}
#[derive(Deserialize)]
struct Aktenfrage {
    suche: String,
}
async fn verfahren_suche(
    State(ApiState { core }): State<ApiState>,
    Path(p): Path<Uuid>,
    Query(q): Query<Aktenfrage>,
) -> ApiResult<Vec<core_lib::verfahren::suche::Suchtreffer>> {
    Ok(Json(core.verfahren_suche(ProjectId(p), &q.suche).await?))
}
#[derive(Deserialize)]
struct Indexwunsch {
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    quelle: core_lib::verfahren::suche::Textquelle,
    text: String,
}
async fn suchtext_speichern(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path(p): Path<Uuid>,
    Json(w): Json<Indexwunsch>,
) -> Result<StatusCode, ApiError> {
    core.suchtext_speichern(ProjectId(p), w.dokument, w.quelle, w.text, &wer)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn verfahren_aktenexport(
    State(ApiState { core }): State<ApiState>,
    Path(p): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/zip"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=Projektakte.zip",
            ),
        ],
        core.verfahren_aktenexport(ProjectId(p)).await?,
    )
        .into_response())
}

async fn abstandsregeln() -> ApiResult<Vec<core_lib::abstandsflaechen::AbstandsRegeln>> {
    Ok(Json(core_lib::abstandsflaechen::katalog()?))
}
async fn abstandsplan(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::abstandsflaechen::AbstandsPlan>,
) -> ApiResult<core_lib::abstandsflaechen::AbstandsErgebnis> {
    Ok(Json(core.abstandsplan_pruefen(ProjectId(id), &plan).await?))
}
async fn abstandsexport(
    State(ApiState { core }): State<ApiState>,
    Path((p, n, format)): Path<(Uuid, Uuid, String)>,
) -> Result<axum::response::Response, ApiError> {
    let (_, b) = core
        .abstandsnachweis_export(ProjectId(p), n, &format)
        .await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                if format == "json" {
                    "application/json"
                } else {
                    "application/dxf"
                },
            ),
            (axum::http::header::CONTENT_DISPOSITION, "attachment"),
        ],
        b,
    )
        .into_response())
}

#[derive(Deserialize)]
struct AbstandscadWunsch {
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    einheit: core_lib::cad::Laengeneinheit,
}
async fn abstandscad(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<AbstandscadWunsch>,
) -> ApiResult<Vec<core_lib::abstandsflaechen::CadKontur>> {
    Ok(Json(
        core.abstandscad_konturen(ProjectId(id), w.dokument, w.einheit)
            .await?,
    ))
}
