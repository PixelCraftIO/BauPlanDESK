//! HTTP-Schicht: nur (De-)Serialisierung und Fehlerabbildung, keine Geschäftslogik.

use std::collections::BTreeMap;

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
#[path = "zugang.rs"]
pub mod zugang;
use core_lib::benutzer::Benutzer;
use core_lib::cad::{Laengeneinheit, Messung, Punkt};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::domain::{DocumentLock, Protokolleintrag};
use core_lib::formular::Feldwert;
use core_lib::kunden::{Beteiligtenzuordnung, Kunde, KundeId, Kundendaten, Projektbeteiligter};
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use core_lib::license::Lizenzstatus;
use core_lib::service::{CadPlan, FormularAnalyse};
use core_lib::xbau::{Bauantrag, Buero, Codeliste, GespeicherterBauantrag};
use core_lib::{CoreError, CoreServices};
use serde::{Deserialize, Serialize};
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
            // Interna (Pfade, Datenbankmeldungen) nur ins Protokoll; der Client
            // erhält eine Kennung, über die der Betrieb den Eintrag findet.
            let kennung = Uuid::new_v4();
            tracing::error!(%kennung, error = %self.0, "Anfrage fehlgeschlagen");
            return (
                status,
                Json(serde_json::json!({
                    "fehler": format!(
                        "Interner Serverfehler. Bitte die Administration mit der Kennung {kennung} verständigen."
                    ),
                    "kennung": kennung,
                })),
            )
                .into_response();
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

/// Body-Grenze für JSON-Aufträge, deren Inhalt der Core selbst auf 2 MiB begrenzt
/// (Fachobjekte, Suchtexte): Spielraum für Hülle, Einrückung und Escapes, damit
/// nicht axum (Standard 2 MB) vor der fachlichen Prüfung mit 413 abbricht.
const JSON_MIT_KERNGRENZE: usize = 4 * 1024 * 1024;

fn routen(core: CoreServices) -> Router {
    let api = Router::new()
        .route("/sitzung", get(sitzung))
        .route(
            "/portalprofile",
            get(portalprofile).post(portalprofil_hinzufuegen),
        )
        .route("/projekte/{p}/verfahren/{v}/portale", post(portal_vorgang))
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/pruefen",
            post(portal_pruefen),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/abgaben",
            post(portal_abgabe),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/rueckmeldungen",
            post(portal_rueckmeldung),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/url",
            get(portal_url),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/abgaben/{b}/paket",
            get(portal_paket),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/abgaben/{b}/dateien/{d}/{version}",
            get(portal_datei),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/abgaben/{b}/ereignisse",
            post(portal_ereignis),
        )
        .route(
            "/projekte/{p}/verfahren/{v}/portale/{g}/abgaben/{b}/xbau",
            post(portal_xbau),
        )
        .route("/berechnungsvorlagen", get(berechnungsvorlagen))
        .route(
            "/projekte/{id}/gebaeudegrundlagen",
            post(gebaeude_auswerten),
        )
        .route(
            "/projekte/{id}/raumbuchimport",
            post(berechnung_aus_raumbuch),
        )
        .route(
            "/projekte/{id}/fachberechnung",
            post(fachberechnung_pruefen),
        )
        .route(
            "/druckgestaltung",
            get(druckgestaltung).put(druckgestaltung_speichern),
        )
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
        .route("/projekte/{id}/uebersicht", get(projektuebersicht))
        .route("/projekte/{id}/bereichsstatus", get(bereichsstatus))
        .route(
            "/projekte/{id}/bereichsstatus/{bereich}",
            post(bereich_benoetigt_setzen),
        )
        .route(
            "/projekte/{id}/einreichungspruefung",
            get(einreichungspruefung),
        )
        .route(
            "/projekte/{id}/gebuehrenschaetzung",
            get(gebuehrenschaetzung),
        )
        .route("/projekte/{id}/baustatistik", get(baustatistik))
        .route("/projekte/{id}/sachstandsbericht", get(sachstandsbericht))
        .route(
            "/projekte/{id}/checkliste",
            axum::routing::post(checkliste_anwenden),
        )
        .route(
            "/projekte/{id}/mitglieder",
            get(mitglieder).post(mitglied_setzen),
        )
        .route("/projekte/{id}/recht", get(projektrecht))
        .route(
            "/projekte/{id}/grundstueckskontext",
            get(grundstueckskontext),
        )
        .route("/projekte/{id}/bereiche/{bereich}", post(bereich_speichern))
        .route(
            "/projekte/{id}/gebaeudezuordnung",
            post(gebaeudezuordnung_uebertragen),
        )
        .route(
            "/projekte/{id}/flurstuecksverweise",
            post(flurstuecksverweise),
        )
        .route(
            "/projekte/{id}/flurstuecksverweise/entfernen",
            post(flurstuecksverweise_entfernen),
        )
        .route(
            "/projekte/{id}/fachobjekte/{art}",
            get(fachobjekte)
                .post(fachobjekt_speichern)
                .layer(DefaultBodyLimit::max(JSON_MIT_KERNGRENZE)),
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
        .route("/verfahrenskataloge", get(verfahrenskataloge))
        .route(
            "/projekte/{id}/verfahrenswahl",
            post(verfahren_verfahrenswahl),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/unterschriften",
            get(verfahren_unterschriften),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/profil",
            post(verfahren_profil),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abgabe",
            post(verfahren_abgabe),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/externe-einreichung",
            post(verfahren_externe_einreichung),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abgaben/{abgabe}/xbau-journal",
            post(verfahren_xbau_erfassen),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abgaben/{abgabe}/{format}",
            get(verfahren_artefakt),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/planunterlagen",
            get(verfahren_planunterlagen),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/abweichungen",
            get(verfahren_abweichungen),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/schreiben",
            post(verfahren_schreiben),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/optionen",
            get(verfahren_optionen),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/checkliste",
            get(verfahren_checkliste),
        )
        .route(
            "/projekte/{id}/verfahren/{verfahren}/folgeverfahren",
            post(verfahren_folgeverfahren),
        )
        .route("/projekte/{id}/erinnerungen", get(verfahren_erinnerungen))
        .route("/projekte/{id}/bescheidfristen", post(bescheidfristen))
        .route("/projekte/{id}/termine.ics", get(verfahren_termine_ics))
        .route("/projekte/{id}/aktensuche", get(verfahren_suche))
        .route(
            "/projekte/{id}/suchtext",
            post(suchtext_speichern).layer(DefaultBodyLimit::max(JSON_MIT_KERNGRENZE)),
        )
        .route("/projekte/{id}/aktenexport", get(verfahren_aktenexport))
        .route("/cockpit", get(buero_cockpit))
        .route(
            "/plannummernschema",
            get(plannummernschema).put(plannummernschema_speichern),
        )
        .route("/projekte/{id}/planliste", get(planliste))
        .route(
            "/kalenderabo",
            get(kalenderabo_auskunft)
                .post(kalenderabo_anlegen)
                .delete(kalenderabo_widerrufen),
        )
        .route("/projekte", get(list_projects).post(create_project))
        .route(
            "/projekte/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route(
            "/dokumente/{id}/formular",
            get(formular_analysieren).post(formular_ausfuellen),
        )
        .route("/dokumente/{id}/vorlage", post(formular_vorlage_erstellen))
        .route("/projekte/{id}/einreichungen", get(einreichungen))
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
            // Transportgrenze = Core-Grenze, nur an dieser Route.
            post(druckausgabe_ablegen)
                .layer(DefaultBodyLimit::max(core_lib::druckausgabe::MAX_DRUCKPDF)),
        )
        .route("/projekte/{id}/projektordner", get(projektordner_plan))
        .route("/projekte/{id}/protokoll", get(protokoll))
        .route(
            "/projekte/{id}/bauherren",
            get(bauherren).put(bauherren_setzen),
        )
        .route(
            "/projekte/{id}/beteiligte",
            get(beteiligte).put(beteiligte_setzen),
        )
        .route("/kunden", get(kunden).post(kunde_anlegen))
        .route(
            "/kunden/{id}",
            get(kunde).put(kunde_aendern).delete(kunde_loeschen),
        )
        .route("/kunden/{id}/projekte", get(kunde_projekte))
        .route(
            "/dokumente/{id}/entfernen/{basis}",
            post(dokument_entfernen),
        )
        .route("/dokumente/{id}/versionen", get(versionen))
        .route("/dokumente/{id}/sperre", post(sperren).delete(entsperren))
        .route("/dokumente/{id}/plan", get(cad_plan))
        .route("/lizenz", get(lizenz).put(lizenz_einspielen))
        .route(
            "/projekte/{id}/bauantrag",
            get(bauantrag).put(bauantrag_speichern),
        )
        .route("/buero", get(buero).put(buero_speichern))
        .route("/xbau/codelisten", get(codelisten))
        .route("/xbau/nachrichtentypen", get(xbau_typen))
        .route(
            "/projekte/{p}/xbau/nachrichten/{id}/anlagen",
            post(xbau_anlage),
        )
        .route(
            "/projekte/{id}/xbau/nachrichten",
            get(xbau_nachrichten).post(xbau_aufnehmen),
        )
        .route("/projekte/{id}/xbau/pruefen", post(xbau_pruefen))
        .route(
            "/projekte/{p}/xbau/nachrichten/{id}/belege",
            post(xbau_beleg),
        )
        .route(
            "/projekte/{p}/xbau/nachrichten/{id}/auswertung",
            get(xbau_auswertung).post(xbau_auswertung_uebernehmen),
        )
        .route(
            "/projekte/{p}/xbau/nachrichten/{id}/verfahren",
            post(xbau_verfahren),
        )
        .route(
            "/projekte/{p}/dokumente/{id}/email/{version}/verfahren",
            post(email_verfahren),
        )
        .route("/lbo/regelpakete", get(rule_packs))
        .route("/berechnung/abstandsflaeche", post(abstandsflaeche))
        .route("/projekte/{id}/abstandsregeln", get(abstandsregeln))
        .route("/projekte/{id}/abstandsplan", post(abstandsplan))
        .route("/projekte/{id}/abstandscad", post(abstandscad))
        .route("/projekte/{id}/abstandscadhoehen", post(abstandscad_hoehen))
        .route("/projekte/{id}/abstandsumrisse", post(abstandsumrisse))
        .route("/projekte/{id}/abstandsgelaende", post(abstandsgelaende))
        .route(
            "/projekte/{id}/abstandsnachweise/{nachweis}/{format}",
            get(abstandsexport),
        )
        .route("/berechnung/aufmass", post(aufmass))
        .route("/projekte/{id}/cadaufmass/laden", post(cadaufmass_laden))
        .route(
            "/projekte/{id}/cadaufmass/exportieren",
            post(cadaufmass_exportieren),
        )
        .route(
            "/projekte/{id}/cadaufmass/speichern",
            post(cadaufmass_speichern),
        )
        .route("/projekte/{id}/planaufmass", post(planaufmass_pruefen))
        .route(
            "/projekte/{id}/baugrundstueck/einzeln",
            post(baugrundstueck_einzeln_pruefen),
        )
        .route(
            "/projekte/{id}/baugrundstueck/einbinden",
            post(baugrundstueck_einzeln_einbinden),
        )
        .route(
            "/projekte/{id}/baugrundstueck/speichern",
            post(baugrundstueck_einzeln_speichern),
        )
        .route(
            "/projekte/{id}/planaufmass/diagnose",
            post(planaufmass_diagnose),
        )
        .route(
            "/projekte/{id}/planaufmass/export",
            post(planaufmass_exportieren),
        )
        .route(
            "/berechnung/planaufmass/flaechenoperation",
            post(planaufmass_flaechenoperation),
        )
        .route("/projekte/{id}/planwerte", post(planaufmass_einsetzen))
        .route("/projekte/{id}/planraumbuch", post(planaufmass_raumbuch))
        .route(
            "/projekte/{id}/planraumbuch/abgleich",
            post(planaufmass_raumbuch_abgleich),
        )
        .route("/projekte/{id}/planabstand", post(planaufmass_abstand))
        .route(
            "/berechnung/planaufmass/abstandsziele",
            post(planaufmass_abstandsziele),
        )
        .route("/projekte/{id}/plangebaeude", post(planaufmass_gebaeude))
        .route(
            "/projekte/{id}/planflurstuecke",
            post(planaufmass_flurstuecke),
        )
        .route("/projekte/{id}/planabzuege", post(planaufmass_abzuege))
        .route("/projekte/{id}/planbauteile", post(planaufmass_bauteile))
        .route(
            "/projekte/{id}/planabzuege/abgleich",
            post(planaufmass_abzuege_abgleich),
        )
        .route(
            "/projekte/{id}/planbauteile/abgleich",
            post(planaufmass_bauteile_abgleich),
        )
        .route(
            "/projekte/{id}/planbaugrenzen",
            post(planaufmass_baugrenzen),
        )
        .route(
            "/projekte/{id}/cadaufmass/uebernehmen",
            post(cadaufmass_uebernehmen),
        )
        .route("/projekte/{id}/planaufgaben", post(planaufmass_aufgaben))
        .route("/projekte/{id}/planquellen", get(planquellen_zustand))
        .route(
            "/projekte/{id}/planraumbuch/einsetzen",
            post(planaufmass_raumbuch_einsetzen),
        )
        .route(
            "/berechnung/planaufmass/werkzeugkiste",
            get(planaufmass_werkzeugkiste),
        )
        .route(
            "/berechnung/planaufmass/gebaeudeziele",
            post(planaufmass_gebaeudeziele),
        )
        .route("/berechnung/planaufmass/ziele", post(planaufmass_ziele))
        .route(
            "/projekte/{id}/nutzung/pruefen",
            post(projekt_nutzung_pruefen),
        )
        .route(
            "/projekte/{id}/baugrundstuecke/pruefen",
            post(projekt_baugrundstuecke_pruefen),
        )
        .route(
            "/projekte/{id}/baugrundstuecke/gebaeude",
            post(projekt_baugrundstuecke_gebaeude_einbinden),
        )
        .route("/berechnung/baugrenzen", post(baugrenzen_pruefen));
    Router::new()
        .route(
            "/health",
            get(bereitschaft).layer(Extension(std::sync::Arc::new(tokio::sync::Mutex::new(
                None::<(std::time::Instant, bool, bool)>,
            )))),
        )
        // Kalender-Abo: ohne Sitzung, der Schlüssel im Pfad ist der Zugang.
        .route("/kalender/{benutzer}/{datei}", get(kalenderabo_ics))
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

/// Büro-Cockpit über alle Projekte, die der angemeldete Benutzer lesen darf.
async fn buero_cockpit(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<core_lib::cockpit::Buerocockpit> {
    Ok(Json(
        core.buero_cockpit(&akteur, core_lib::kalender::heute())
            .await?,
    ))
}

async fn plannummernschema(
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<core_lib::planliste::Plannummernschema> {
    Ok(Json(core.plannummernschema().await?))
}
async fn plannummernschema_speichern(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
    Json(s): Json<core_lib::planliste::Plannummernschema>,
) -> ApiResult<core_lib::planliste::Plannummernschema> {
    Ok(Json(core.plannummernschema_speichern(s, &akteur).await?))
}
async fn planliste(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
    Path(p): Path<Uuid>,
) -> ApiResult<core_lib::planliste::Planliste> {
    Ok(Json(core.planliste(ProjectId(p), &akteur).await?))
}
async fn kalenderabo_auskunft(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<core_lib::kalenderabo::Aboauskunft> {
    Ok(Json(core.kalenderabo_auskunft(&akteur).await?))
}
#[derive(Deserialize)]
struct Abowunsch {
    umfang: core_lib::kalenderabo::Aboumfang,
}
async fn kalenderabo_anlegen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
    Json(w): Json<Abowunsch>,
) -> ApiResult<core_lib::kalenderabo::Aboschluessel> {
    Ok(Json(core.kalenderabo_anlegen(&akteur, w.umfang).await?))
}
async fn kalenderabo_widerrufen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<()> {
    Ok(Json(core.kalenderabo_widerrufen(&akteur).await?))
}
/// Abonnierter Kalender (RFC 5545). Jeder Fehler beim Schlüssel ergibt 404.
async fn kalenderabo_ics(
    State(ApiState { core }): State<ApiState>,
    Path((benutzer, datei)): Path<(Uuid, String)>,
) -> Result<Response, ApiError> {
    let schluessel = datei.strip_suffix(".ics").unwrap_or("");
    let ics = core
        .kalenderabo_ics(
            core_lib::benutzer::BenutzerId(benutzer),
            schluessel,
            core_lib::kalender::heute(),
        )
        .await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/calendar; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        ics,
    )
        .into_response())
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

async fn delete_project(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<VersionQuery>,
) -> ApiResult<core_lib::service::Projektloeschung> {
    let ergebnis = core
        .delete_project(ProjectId(id), q.version, &akteur)
        .await?;
    Ok(Json(ergebnis))
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

#[derive(Deserialize)]
struct CadPlanQuery {
    #[serde(default)]
    version: Option<i64>,
}

async fn cad_plan(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<CadPlanQuery>,
) -> ApiResult<CadPlan> {
    Ok(Json(match q.version {
        Some(v) => core.cad_plan_version(DocumentId(id), v).await?,
        None => core.cad_plan(DocumentId(id)).await?,
    }))
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

async fn planaufmass_pruefen(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::planaufmass::Planaufmass>,
) -> ApiResult<Vec<core_lib::planaufmass::Planmesswert>> {
    Ok(Json(core.planaufmass_pruefen(ProjectId(id), &plan).await?))
}
async fn planaufmass_ziele(
    Json(plan): Json<core_lib::berechnungen::FachPlan>,
) -> ApiResult<Vec<core_lib::planaufmass::Planziel>> {
    Ok(Json(core_lib::planaufmass::planziele(&plan)?))
}
async fn planaufmass_einsetzen(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::berechnungen::FachPlan>,
) -> ApiResult<core_lib::berechnungen::FachPlan> {
    Ok(Json(core.planaufmass_einsetzen(ProjectId(id), plan).await?))
}
#[derive(Deserialize)]
struct PlanraumRequest {
    aufmass_id: Uuid,
    revision: i64,
    #[serde(default)]
    zuordnung: Vec<core_lib::planaufmass::Zeilenzuordnung>,
    raumbuch: core_lib::plattform::objekte::Raumbuch,
}
async fn planaufmass_raumbuch(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(r): Json<PlanraumRequest>,
) -> ApiResult<core_lib::plattform::objekte::Raumbuch> {
    Ok(Json(
        core.planaufmass_raumbuch(
            ProjectId(id),
            r.aufmass_id,
            r.revision,
            &r.zuordnung,
            r.raumbuch,
        )
        .await?,
    ))
}
async fn planaufmass_raumbuch_abgleich(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(r): Json<PlanraumRequest>,
) -> ApiResult<Vec<core_lib::planaufmass::Abgleichsfrage>> {
    Ok(Json(
        core.planaufmass_raumbuch_abgleich(ProjectId(id), r.aufmass_id, r.revision, &r.raumbuch)
            .await?,
    ))
}
async fn planaufmass_abstandsziele(
    Json(plan): Json<core_lib::abstandsflaechen::AbstandsPlan>,
) -> ApiResult<Vec<core_lib::planaufmass::Planziel>> {
    Ok(Json(core_lib::planaufmass::abstandsziele(&plan)))
}
async fn planaufmass_abstand(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::abstandsflaechen::AbstandsPlan>,
) -> ApiResult<core_lib::abstandsflaechen::AbstandsPlan> {
    Ok(Json(core.planaufmass_abstand(ProjectId(id), plan).await?))
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

async fn kunden(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
) -> ApiResult<Vec<Kunde>> {
    Ok(Json(core.kunden_fuer_benutzer(&wer).await?))
}

async fn kunde(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Kunde> {
    Ok(Json(core.kunde_fuer_benutzer(KundeId(id), &wer).await?))
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

async fn beteiligte(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Projektbeteiligter>> {
    Ok(Json(core.beteiligte(ProjectId(id)).await?))
}

#[derive(Deserialize)]
struct BeteiligteSetzen {
    beteiligte: Vec<Beteiligtenzuordnung>,
    /// Zuletzt gelesene Projektversion.
    version: i64,
}

async fn beteiligte_setzen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<BeteiligteSetzen>,
) -> ApiResult<Project> {
    Ok(Json(
        core.beteiligte_setzen(ProjectId(id), body.beteiligte, body.version, &akteur)
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
    Ok(Json(core.codelisten.aktuell().alle().cloned().collect()))
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

async fn projekt_baugrundstuecke_gebaeude_einbinden(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(e): Json<core_lib::baugrundstuecke::BaugrundstuecksPlan>,
) -> ApiResult<core_lib::plattform::einbindung::GebaeudeEinbindung> {
    Ok(Json(
        core.projekt_baugrundstuecke_gebaeude_einbinden(ProjectId(id), &e)
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
    let (_, datei) = core
        .einreichung_zip_schreiben(id, temporaere_datei()?)
        .await?;
    datei_antwort(
        datei,
        "application/zip",
        format!("attachment; filename=\"Einreichung-{id}.zip\""),
    )
}

/// Anonyme temporäre Datei (Verzeichnis über `TMPDIR`), wird beim Schließen entfernt.
fn temporaere_datei() -> Result<std::fs::File, ApiError> {
    tempfile::tempfile()
        .map_err(|e| CoreError::Storage(format!("Temporäre Exportdatei: {e}")).into())
}

/// Streamt eine fertig geschriebene Exportdatei, ohne sie in den Speicher zu laden.
fn datei_antwort(
    mut datei: std::fs::File,
    typ: &'static str,
    disposition: String,
) -> Result<Response, ApiError> {
    use std::io::Seek;
    let laenge = datei
        .rewind()
        .and_then(|_| datei.metadata())
        .map_err(|e| CoreError::Storage(format!("Temporäre Exportdatei: {e}")))?
        .len();
    let strom = tokio_util::io::ReaderStream::new(tokio::fs::File::from_std(datei));
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, typ.to_string()),
            (axum::http::header::CONTENT_DISPOSITION, disposition),
            (axum::http::header::CONTENT_LENGTH, laenge.to_string()),
        ],
        axum::body::Body::from_stream(strom),
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
#[derive(Deserialize, Default)]
struct FachseitenQuery {
    nach: Option<String>,
    limit: Option<u32>,
    objekt: Option<Uuid>,
    vor: Option<i64>,
}
async fn fachobjekte(
    State(ApiState { core, .. }): State<ApiState>,
    Path((id, art)): Path<(Uuid, core_lib::plattform::ObjektArt)>,
    Query(q): Query<FachseitenQuery>,
) -> ApiResult<Vec<core_lib::commit::Fachstand>> {
    let p = ProjectId(id);
    if let Some(objekt) = q.objekt {
        if q.vor.is_some() || q.limit.is_some() {
            return Ok(Json(
                core.fachverlaufsseite(p, art, objekt, q.vor, q.limit.unwrap_or(50))
                    .await?,
            ));
        }
        return Ok(Json(
            core.fachobjekt_lesen(p, art, objekt)
                .await?
                .into_iter()
                .collect(),
        ));
    }
    if q.limit.is_some() || q.nach.is_some() {
        return Ok(Json(
            core.fachobjektseite(p, art, q.nach.as_deref(), q.limit.unwrap_or(100))
                .await?,
        ));
    }
    Ok(Json(core.fachobjekte(p, art).await?))
}
#[derive(Deserialize)]
struct FlurstueckeRequest {
    ids: Vec<String>,
}
async fn flurstuecksverweise(
    State(ApiState { core, .. }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<FlurstueckeRequest>,
) -> ApiResult<Vec<core_lib::plattform::verweise::Flurstuecksverweis>> {
    Ok(Json(
        core.flurstuecksverweise(ProjectId(id), &body.ids).await?,
    ))
}
async fn flurstuecksverweise_entfernen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<FlurstueckeRequest>,
) -> ApiResult<usize> {
    Ok(Json(
        core.flurstuecksverweise_entfernen(ProjectId(id), &body.ids, &akteur)
            .await?,
    ))
}
#[derive(Deserialize)]
struct ZuordnungRequest {
    alt: String,
    neu: String,
}
async fn gebaeudezuordnung_uebertragen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ZuordnungRequest>,
) -> ApiResult<usize> {
    Ok(Json(
        core.gebaeudezuordnung_uebertragen(ProjectId(id), &body.alt, &body.neu, &akteur)
            .await?,
    ))
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
            let data = axum::body::Bytes::from_static(b"BauPlanIO Speicherpruefung");
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
/// Eigenes wirksames Projektrecht (Core-Regel aus `zugriff.rs`).
async fn projektrecht(
    Extension(b): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<core_lib::zugriff::ProjektRecht> {
    Ok(Json(core.eigenes_projektrecht(ProjectId(id), &b).await?))
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmailVerfahrenswunsch {
    verfahren: Uuid,
    basis: i64,
    eingehend: bool,
    /// Bestätigter Zugang bzw. Versand; fehlt es, gilt das Sendedatum.
    #[serde(default)]
    datum: Option<chrono::NaiveDate>,
}
async fn email_verfahren(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id, version)): Path<(Uuid, Uuid, i64)>,
    Json(w): Json<EmailVerfahrenswunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.email_in_verfahren(
            ProjectId(p),
            core_lib::plattform::objekte::Dokumentbezug {
                dokument_id: DocumentId(id),
                version,
            },
            w.verfahren,
            w.basis,
            w.eingehend,
            w.datum,
            &wer,
        )
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

async fn verfahrenskataloge() -> ApiResult<Vec<core_lib::verfahren::profile::Anforderungskatalog>> {
    Ok(Json(
        core_lib::verfahren::profile::Anforderungskatalog::mitgeliefert()?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verfahrenswahlwunsch {
    #[serde(default)]
    verfahren: Option<Uuid>,
    #[serde(default)]
    angaben: core_lib::verfahren::landesrecht::Verfahrensangaben,
}
async fn verfahren_verfahrenswahl(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path(p): Path<Uuid>,
    Json(w): Json<Verfahrenswahlwunsch>,
) -> ApiResult<core_lib::verfahren::landesrecht::Verfahrenswahl> {
    core.projekt_zugriff(ProjectId(p), &wer, core_lib::zugriff::ProjektRecht::Lesen)
        .await?;
    Ok(Json(
        core.verfahrenswahl(ProjectId(p), w.verfahren, w.angaben)
            .await?,
    ))
}
async fn verfahren_unterschriften(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Vec<core_lib::verfahren::landesrecht::Unterschriftsbedarf>> {
    core.projekt_zugriff(ProjectId(p), &wer, core_lib::zugriff::ProjektRecht::Lesen)
        .await?;
    Ok(Json(core.verfahren_unterschriften(ProjectId(p), id).await?))
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExterneEinreichungAuftrag {
    basis: i64,
    wunsch: core_lib::verfahren::externe_einreichung::ExterneEinreichungswunsch,
}
async fn verfahren_externe_einreichung(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<ExterneEinreichungAuftrag>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.verfahren_externe_einreichung(ProjectId(p), id, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
async fn verfahren_xbau_erfassen(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id, abgabe)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<core_lib::xbau::postfach::Nachrichtenstand> {
    Ok(Json(
        core.verfahren_xbau_erfassen(ProjectId(p), id, abgabe, &wer)
            .await?,
    ))
}
async fn verfahren_planunterlagen(
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Vec<core_lib::verfahren::planindex::Planunterlage>> {
    Ok(Json(core.verfahren_planunterlagen(ProjectId(p), id).await?))
}
async fn verfahren_abweichungen(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<core_lib::verfahren::abweichungen::Abweichungsabgleich> {
    core.projekt_zugriff(ProjectId(p), &wer, core_lib::zugriff::ProjektRecht::Lesen)
        .await?;
    Ok(Json(core.verfahren_abweichungen(ProjectId(p), id).await?))
}
async fn verfahren_optionen(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<core_lib::verfahren::optionen::Verfahrenslage> {
    core.projekt_zugriff(ProjectId(p), &wer, core_lib::zugriff::ProjektRecht::Lesen)
        .await?;
    Ok(Json(core.verfahren_optionen(ProjectId(p), id).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Checklistenfrage {
    phase: core_lib::verfahren::ausfuehrung::Phase,
}
async fn verfahren_checkliste(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Query(q): Query<Checklistenfrage>,
) -> ApiResult<core_lib::verfahren::ausfuehrung::Phasencheckliste> {
    core.projekt_zugriff(ProjectId(p), &wer, core_lib::zugriff::ProjektRecht::Lesen)
        .await?;
    Ok(Json(
        core.verfahren_checkliste(ProjectId(p), id, q.phase).await?,
    ))
}
async fn verfahren_folgeverfahren(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<core_lib::verfahren::optionen::Folgeverfahrenswunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.verfahren_folgeverfahren(ProjectId(p), id, w, &wer)
            .await?,
    ))
}
/// Schreiben mit PDF (Base64), damit der Client es speichern kann.
#[derive(Serialize)]
struct Schreibenantwort {
    #[serde(flatten)]
    ergebnis: core_lib::verfahren::schreiben::Schreibenergebnis,
    pdf_base64: String,
}
async fn verfahren_schreiben(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<core_lib::verfahren::schreiben::Schreibenwunsch>,
) -> ApiResult<Schreibenantwort> {
    use base64::Engine as _;
    let (ergebnis, pdf) = core.verfahren_schreiben(ProjectId(p), id, w, &wer).await?;
    Ok(Json(Schreibenantwort {
        ergebnis,
        pdf_base64: base64::engine::general_purpose::STANDARD.encode(pdf),
    }))
}
async fn verfahren_artefakt(
    State(ApiState { core }): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Path((p, id, abgabe, format)): Path<(Uuid, Uuid, Uuid, String)>,
) -> Result<Response, ApiError> {
    let (_, b) = core
        .verfahren_artefakt(ProjectId(p), id, abgabe, &format, &wer)
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
/// Laufende Verfahrenstermine als Kalenderdatei (RFC 5545), Leserecht.
async fn verfahren_termine_ics(
    State(ApiState { core }): State<ApiState>,
    Path(p): Path<Uuid>,
) -> Result<Response, ApiError> {
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/calendar; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=Verfahrenstermine.ics",
            ),
        ],
        core.verfahren_termine_ics(ProjectId(p), core_lib::kalender::heute())
            .await?,
    )
        .into_response())
}
async fn bescheidfristen(
    State(ApiState { core }): State<ApiState>,
    Path(p): Path<Uuid>,
    Json(auftrag): Json<core_lib::verfahren::fristen::Fristauftrag>,
) -> ApiResult<Vec<core_lib::verfahren::fristen::Bescheidfristen>> {
    Ok(Json(core.bescheidfristen(ProjectId(p), &auftrag).await?))
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
    // Fortlaufend in eine temporäre Datei, danach gestreamt: kein Gesamtarchiv im Speicher.
    let datei = core
        .verfahren_aktenexport_in(ProjectId(p), temporaere_datei()?)
        .await?;
    datei_antwort(
        datei,
        "application/zip",
        "attachment; filename=Projektakte.zip".into(),
    )
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

async fn abstandscad_hoehen(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<AbstandscadWunsch>,
) -> ApiResult<Vec<core_lib::abstandsflaechen::CadHoehe>> {
    Ok(Json(
        core.abstandscad_hoehen(ProjectId(id), w.dokument, w.einheit)
            .await?,
    ))
}
async fn abstandsumrisse(
    Path(_id): Path<Uuid>,
    Json(plan): Json<core_lib::abstandsflaechen::AbstandsPlan>,
) -> ApiResult<core_lib::abstandsflaechen::UmrissAbgleich> {
    Ok(Json(core_lib::abstandsflaechen::umrisse_abgleichen(plan)?))
}
#[derive(Deserialize)]
struct GelaendeWunsch {
    wand: core_lib::abstandsflaechen::AbstandsWand,
    hoehen: Vec<core_lib::abstandsflaechen::CadHoehe>,
    max_abstand_m: f64,
}
async fn abstandsgelaende(
    Path(_id): Path<Uuid>,
    Json(w): Json<GelaendeWunsch>,
) -> ApiResult<Vec<core_lib::abstandsflaechen::Hoehenpunkt>> {
    Ok(Json(core_lib::abstandsflaechen::gelaende_vorschlag(
        &w.wand,
        &w.hoehen,
        w.max_abstand_m,
    )?))
}

async fn druckgestaltung(
    State(s): State<ApiState>,
) -> ApiResult<core_lib::druckausgabe::Druckgestaltung> {
    Ok(Json(s.core.druckgestaltung().await?))
}
async fn druckgestaltung_speichern(
    State(s): State<ApiState>,
    Extension(wer): Extension<Benutzer>,
    Json(g): Json<core_lib::druckausgabe::Druckgestaltung>,
) -> ApiResult<core_lib::druckausgabe::Druckgestaltung> {
    Ok(Json(s.core.druckgestaltung_speichern(g, &wer).await?))
}

async fn berechnungsvorlagen() -> ApiResult<Vec<core_lib::berechnungen::BerechnungsVorlage>> {
    Ok(Json(core_lib::berechnungen::vorlagen()?))
}
async fn fachberechnung_pruefen(
    State(s): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::berechnungen::FachPlan>,
) -> ApiResult<serde_json::Value> {
    let (plan, ergebnis) = s.core.berechnung_pruefen(ProjectId(id), plan).await?;
    Ok(Json(serde_json::json!({"plan":plan,"ergebnis":ergebnis})))
}
#[derive(serde::Deserialize)]
struct RaumbuchImport {
    id: Uuid,
    modul: core_lib::berechnungen::BerechnungsArt,
}
async fn berechnung_aus_raumbuch(
    State(s): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<RaumbuchImport>,
) -> ApiResult<core_lib::berechnungen::FachPlan> {
    Ok(Json(
        s.core
            .berechnung_aus_raumbuch(ProjectId(id), w.id, w.modul)
            .await?,
    ))
}

async fn projektuebersicht(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::projektuebersicht::Projektuebersicht> {
    Ok(Json(core.projektuebersicht(ProjectId(id), &wer).await?))
}

async fn bereichsstatus(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::bereichsstatus::Bereichsstaende> {
    Ok(Json(core.bereichsstatus(ProjectId(id), &wer).await?))
}

#[derive(serde::Deserialize)]
struct BenoetigtEingabe {
    benoetigt: bool,
}
async fn bereich_benoetigt_setzen(
    State(ApiState { core }): State<ApiState>,
    Path((id, bereich)): Path<(Uuid, String)>,
    Extension(wer): Extension<Benutzer>,
    Json(e): Json<BenoetigtEingabe>,
) -> ApiResult<core_lib::bereichsstatus::Bereichsstaende> {
    Ok(Json(
        core.bereich_benoetigt_setzen(ProjectId(id), &bereich, e.benoetigt, &wer)
            .await?,
    ))
}

#[derive(serde::Deserialize)]
struct EinreichungspruefungQuery {
    verfahren: Option<Uuid>,
}
async fn einreichungspruefung(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Query(q): Query<EinreichungspruefungQuery>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::einreichungspruefung::Einreichungspruefung> {
    Ok(Json(
        core.einreichungspruefung(ProjectId(id), q.verfahren, &wer)
            .await?,
    ))
}

async fn gebuehrenschaetzung(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::baukennwerte::Gebuehrenschaetzung> {
    Ok(Json(core.gebuehrenschaetzung(ProjectId(id), &wer).await?))
}
async fn baustatistik(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::baukennwerte::Baustatistikbogen> {
    Ok(Json(core.baustatistik(ProjectId(id), &wer).await?))
}
async fn sachstandsbericht(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Extension(wer): Extension<Benutzer>,
) -> ApiResult<core_lib::druckausgabe::brief::Brief> {
    Ok(Json(core.sachstandsbericht(ProjectId(id), &wer).await?))
}

// XBau-Dateiaustausch. Diese Endpunkte versenden nichts an eine Behörde.
async fn xbau_typen() -> ApiResult<Vec<core_lib::xbau::austausch::Nachrichtentyp>> {
    Ok(Json(core_lib::xbau::austausch::katalog()))
}
async fn xbau_nachrichten(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::xbau::postfach::Nachrichtenstand>> {
    Ok(Json(core.xbau_nachrichten(ProjectId(id), &wer).await?))
}
async fn xbau_pruefen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(original): Json<core_lib::plattform::objekte::Dokumentbezug>,
) -> ApiResult<core_lib::xbau::austausch::Pruefbericht> {
    Ok(Json(
        core.xbau_pruefen(ProjectId(id), &original, &wer).await?,
    ))
}
async fn xbau_aufnehmen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(wunsch): Json<core_lib::xbau::postfach::Nachrichtenaufnahme>,
) -> ApiResult<core_lib::xbau::postfach::Nachrichtenstand> {
    Ok(Json(
        core.xbau_aufnehmen(ProjectId(id), wunsch, &wer).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XbauBelegwunsch {
    basis: i64,
    beleg: core_lib::xbau::postfach::Nachrichtenbeleg,
}
async fn xbau_beleg(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<XbauBelegwunsch>,
) -> ApiResult<core_lib::xbau::postfach::Nachrichtenstand> {
    Ok(Json(
        core.xbau_beleg(ProjectId(p), id, w.basis, w.beleg, &wer)
            .await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XbauVerfahrenswunsch {
    basis: i64,
    verfahren: Uuid,
    /// Bestätigtes Zugangs- bzw. Versanddatum; fehlt es, gilt der Erfassungstag.
    #[serde(default)]
    zugang: Option<chrono::NaiveDate>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XbauAuswertungsfrage {
    #[serde(default)]
    verfahren: Option<Uuid>,
}
async fn xbau_auswertung(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Query(q): Query<XbauAuswertungsfrage>,
) -> ApiResult<core_lib::verfahren::eingang::Nachrichtenauswertung> {
    Ok(Json(
        core.xbau_auswertung(ProjectId(p), id, q.verfahren, &wer)
            .await?,
    ))
}
async fn xbau_auswertung_uebernehmen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<core_lib::verfahren::eingang::Uebernahmewunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.xbau_auswertung_uebernehmen(ProjectId(p), id, w, &wer)
            .await?,
    ))
}
async fn xbau_verfahren(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<XbauVerfahrenswunsch>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.xbau_in_verfahren(ProjectId(p), id, w.verfahren, w.basis, w.zugang, &wer)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct XbauAnlagenwunsch {
    dateiname: String,
}
async fn xbau_anlage(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, id)): Path<(Uuid, Uuid)>,
    Json(w): Json<XbauAnlagenwunsch>,
) -> ApiResult<DocumentVersion> {
    Ok(Json(
        core.xbau_anlage_ablegen(ProjectId(p), id, &w.dateiname, &wer)
            .await?,
    ))
}

// Portalunterstützung: keine automatische Antragstellung.
async fn portalprofile(
    State(ApiState { core }): State<ApiState>,
) -> ApiResult<Vec<core_lib::portale::Portalprofil>> {
    Ok(Json(core.portalprofile().await?))
}
async fn portalprofil_hinzufuegen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Json(profil): Json<core_lib::portale::Portalprofil>,
) -> ApiResult<Vec<core_lib::portale::Portalprofil>> {
    Ok(Json(core.portalprofil_hinzufuegen(profil, &wer).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Portalvorgangsauftrag {
    basis: i64,
    wunsch: core_lib::portale::Portalvorgangswunsch,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Portalabgabeauftrag {
    basis: i64,
    wunsch: core_lib::portale::Portalabgabewunsch,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Portalereignisauftrag {
    basis: i64,
    wunsch: core_lib::portale::Portalereigniswunsch,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Portalrueckmeldungsauftrag {
    basis: i64,
    wunsch: core_lib::portale::Portalrueckmeldungswunsch,
}
async fn portal_vorgang(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v)): Path<(Uuid, Uuid)>,
    Json(w): Json<Portalvorgangsauftrag>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.portal_vorgang_anlegen(ProjectId(p), v, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
async fn portal_pruefen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g)): Path<(Uuid, Uuid, Uuid)>,
    Json(w): Json<core_lib::portale::Portalabgabewunsch>,
) -> ApiResult<core_lib::portale::Portalvorbereitung> {
    Ok(Json(
        core.portal_abgabe_pruefen(ProjectId(p), v, g, &w, &wer)
            .await?,
    ))
}
async fn portal_abgabe(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g)): Path<(Uuid, Uuid, Uuid)>,
    Json(w): Json<Portalabgabeauftrag>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.portal_abgabe_festhalten(ProjectId(p), v, g, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
async fn portal_ereignis(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g, b)): Path<(Uuid, Uuid, Uuid, Uuid)>,
    Json(w): Json<Portalereignisauftrag>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.portal_ereignis_erfassen(ProjectId(p), v, g, b, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
async fn portal_rueckmeldung(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g)): Path<(Uuid, Uuid, Uuid)>,
    Json(w): Json<Portalrueckmeldungsauftrag>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.portal_rueckmeldung_erfassen(ProjectId(p), v, g, w.basis, w.wunsch, &wer)
            .await?,
    ))
}
fn portal_download(name: &str, bytes: Vec<u8>, mime: &str) -> Response {
    (
        [
            (axum::http::header::CONTENT_TYPE, mime.to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename*=UTF-8''{}", prozent_kodieren(name)),
            ),
        ],
        bytes,
    )
        .into_response()
}
async fn portal_paket(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g, b)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> Result<Response, ApiError> {
    let (name, bytes) = core
        .portal_bereitstellung(ProjectId(p), v, g, b, &wer)
        .await?;
    Ok(portal_download(&name, bytes, "application/zip"))
}
async fn portal_datei(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g, b, d, version)): Path<(Uuid, Uuid, Uuid, Uuid, Uuid, i64)>,
) -> Result<Response, ApiError> {
    let (name, bytes) = core
        .portal_datei(
            ProjectId(p),
            v,
            g,
            b,
            &core_lib::plattform::objekte::Dokumentbezug {
                dokument_id: DocumentId(d),
                version,
            },
            &wer,
        )
        .await?;
    Ok(portal_download(&name, bytes, "application/octet-stream"))
}
async fn portal_url(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<String> {
    Ok(Json(core.portal_url(ProjectId(p), v, g, &wer).await?))
}
async fn portal_xbau(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((p, v, g, b)): Path<(Uuid, Uuid, Uuid, Uuid)>,
) -> ApiResult<core_lib::xbau::postfach::Nachrichtenstand> {
    Ok(Json(
        core.portal_xbau_archivieren(ProjectId(p), v, g, b, &wer)
            .await?,
    ))
}

#[derive(serde::Deserialize)]
struct GebaeudeAuswertung {
    id: Uuid,
    gebaeude: core_lib::plattform::objekte::Gebaeude,
}
async fn gebaeude_auswerten(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<GebaeudeAuswertung>,
) -> ApiResult<core_lib::plattform::gebaeude::GebaeudeGrundlagen> {
    Ok(Json(
        core.gebaeude_auswerten(ProjectId(id), w.id, &w.gebaeude)
            .await?,
    ))
}
async fn planaufmass_aufgaben(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::planaufmass::Planaufmass>,
) -> ApiResult<Vec<core_lib::planaufmass::Planaufgabe>> {
    Ok(Json(core.planaufmass_aufgaben(ProjectId(id), &plan).await?))
}
async fn planquellen_zustand(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<core_lib::planaufmass::Planquellenhinweis>> {
    Ok(Json(core.planquellen_zustand(ProjectId(id)).await?))
}
async fn planaufmass_raumbuch_einsetzen(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(r): Json<core_lib::plattform::objekte::Raumbuch>,
) -> ApiResult<core_lib::plattform::objekte::Raumbuch> {
    Ok(Json(
        core.planaufmass_raumbuch_einsetzen(ProjectId(id), r)
            .await?,
    ))
}
async fn planaufmass_werkzeugkiste() -> Json<Vec<core_lib::planaufmass::Werkzeugvorlage>> {
    Json(core_lib::planaufmass::werkzeugkiste())
}
async fn planaufmass_gebaeudeziele(
    Json(plan): Json<core_lib::plattform::objekte::Gebaeude>,
) -> ApiResult<Vec<core_lib::planaufmass::Planziel>> {
    Ok(Json(core_lib::planaufmass::gebaeudeziele(&plan)))
}
async fn planaufmass_gebaeude(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<GebaeudeAuswertung>,
) -> ApiResult<core_lib::plattform::objekte::Gebaeude> {
    Ok(Json(
        core.planaufmass_gebaeude(ProjectId(id), w.id, w.gebaeude)
            .await?,
    ))
}
async fn planaufmass_flurstuecke(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(infos): Json<core_lib::grundstuecksinformationen::Grundstuecksinformationen>,
) -> ApiResult<core_lib::grundstuecksinformationen::Grundstuecksinformationen> {
    Ok(Json(
        core.planaufmass_flurstuecke(ProjectId(id), infos).await?,
    ))
}
#[derive(Deserialize)]
struct AbzugUebernahme {
    #[serde(default)]
    aufmass: Option<core_lib::planaufmass::Aufmassstand>,
    #[serde(default)]
    zuordnung: Vec<core_lib::planaufmass::Zeilenzuordnung>,
    ermittlung: core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung,
}
async fn planaufmass_abzuege(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<AbzugUebernahme>,
) -> ApiResult<core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung> {
    Ok(Json(
        core.planaufmass_abzuege(ProjectId(id), w.aufmass, &w.zuordnung, w.ermittlung)
            .await?,
    ))
}
#[derive(Deserialize)]
struct AbzugAbgleich {
    aufmass: core_lib::planaufmass::Aufmassstand,
    ermittlung: core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung,
}
async fn planaufmass_abzuege_abgleich(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<AbzugAbgleich>,
) -> ApiResult<Vec<core_lib::planaufmass::Abgleichsfrage>> {
    Ok(Json(
        core.planaufmass_abzuege_abgleich(ProjectId(id), w.aufmass, &w.ermittlung)
            .await?,
    ))
}
#[derive(Deserialize)]
struct BauteilUebernahme {
    schluessel: String,
    #[serde(default)]
    aufmass: Option<core_lib::planaufmass::Aufmassstand>,
    #[serde(default)]
    zuordnung: Vec<core_lib::planaufmass::Zeilenzuordnung>,
    aufstellung: core_lib::flaechen::FlaechenAufstellung,
}
async fn planaufmass_bauteile(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<BauteilUebernahme>,
) -> ApiResult<core_lib::flaechen::FlaechenAufstellung> {
    Ok(Json(
        core.planaufmass_bauteile(
            ProjectId(id),
            &w.schluessel,
            w.aufmass,
            &w.zuordnung,
            w.aufstellung,
        )
        .await?,
    ))
}
#[derive(Deserialize)]
struct BauteilAbgleich {
    schluessel: String,
    aufmass: core_lib::planaufmass::Aufmassstand,
    aufstellung: core_lib::flaechen::FlaechenAufstellung,
}
async fn planaufmass_bauteile_abgleich(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<BauteilAbgleich>,
) -> ApiResult<Vec<core_lib::planaufmass::Abgleichsfrage>> {
    Ok(Json(
        core.planaufmass_bauteile_abgleich(ProjectId(id), &w.schluessel, w.aufmass, &w.aufstellung)
            .await?,
    ))
}
/// Ordnet Baufenster und Vorhabenumriss eines gespeicherten Aufmaßstands dem
/// Projekt zu (`planungsrecht.baugrenzen`) und liefert die Lageprüfung.
async fn planaufmass_baugrenzen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(stand): Json<core_lib::planaufmass::Aufmassstand>,
) -> ApiResult<core_lib::planaufmass::BaugrenzenUebernahme> {
    Ok(Json(
        core.planaufmass_baugrenzen(ProjectId(id), stand, &wer)
            .await?,
    ))
}
/// Übernimmt gespeicherte CAD-Aufmaße (Altbestand) als Aufmaße auf der DXF-Unterlage.
async fn cadaufmass_uebernehmen(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
) -> ApiResult<core_lib::planaufmass::CadUebernahme> {
    Ok(Json(
        core.cadaufmass_uebernehmen(ProjectId(id), &wer).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn antwort(e: CoreError) -> (StatusCode, serde_json::Value) {
        let r = ApiError(e).into_response();
        let status = r.status();
        let body = axum::body::to_bytes(r.into_body(), 64 * 1024)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn serverfehler_verraten_keine_interna() {
        let (status, body) = antwort(CoreError::Storage(
            "/srv/bauprojekt/dateien/data/projekte/x: Permission denied".into(),
        ))
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let text = body["fehler"].as_str().unwrap();
        assert!(
            !text.contains("/srv") && !text.contains("Permission"),
            "{text}"
        );
        let kennung = body["kennung"].as_str().unwrap();
        assert!(Uuid::parse_str(kennung).is_ok());
        assert!(text.contains(kennung));
        let (_, body) = antwort(CoreError::Database("relation \"x\" does not exist".into())).await;
        assert!(!body["fehler"].as_str().unwrap().contains("relation"));
        // Fachliche Fehler (4xx) bleiben unverändert aussagekräftig.
        let (status, body) = antwort(CoreError::InvalidInput("Name fehlt".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["fehler"], "Ungültige Eingabe: Name fehlt");
        assert!(body.get("kennung").is_none());
    }
}

async fn dokument_entfernen(
    Extension(akteur): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path((id, basis)): Path<(Uuid, i64)>,
) -> Result<StatusCode, ApiError> {
    core.dokument_entfernen(DocumentId(id), basis, &akteur)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn planaufmass_diagnose(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(plan): Json<core_lib::planaufmass::Planaufmass>,
) -> ApiResult<core_lib::planaufmass::Planpruefung> {
    Ok(Json(core.planaufmass_diagnose(ProjectId(id), &plan).await?))
}
#[derive(Deserialize)]
struct Flaechenwunsch {
    a: core_lib::planaufmass::Plangeometrie,
    b: core_lib::planaufmass::Plangeometrie,
    operation: core_lib::planaufmass::Flaechenoperation,
}
async fn planaufmass_flaechenoperation(
    Json(w): Json<Flaechenwunsch>,
) -> ApiResult<Vec<core_lib::planaufmass::Plangeometrie>> {
    Ok(Json(core_lib::planaufmass::flaechenoperation(
        &w.a,
        &w.b,
        w.operation,
    )?))
}
async fn planaufmass_exportieren(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<core_lib::planaufmass::Planexportwunsch>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let bytes = core.planaufmass_export(ProjectId(id), &w).await?;
    let mime = match w.format {
        core_lib::planaufmass::Planexportformat::Pdf => "application/pdf",
        core_lib::planaufmass::Planexportformat::Csv => "text/csv; charset=utf-8",
    };
    Ok(([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response())
}
async fn cadaufmass_laden(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(d): Json<core_lib::plattform::objekte::Dokumentbezug>,
) -> ApiResult<Option<core_lib::commit::Fachstand>> {
    Ok(Json(core.cadaufmass_laden(ProjectId(id), &d).await?))
}
#[derive(Deserialize)]
struct CadSpeichern {
    aufmass: core_lib::cad::projektaufmass::CadProjektaufmass,
    basis: i64,
}
#[derive(Deserialize)]
struct CadExportieren {
    dokument: core_lib::plattform::objekte::Dokumentbezug,
    revision: i64,
}
async fn cadaufmass_exportieren(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<CadExportieren>,
) -> Result<axum::response::Response, ApiError> {
    use axum::response::IntoResponse;
    let bytes = core
        .cadaufmass_csv(ProjectId(id), &w.dokument, w.revision)
        .await?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")],
        bytes,
    )
        .into_response())
}
#[derive(Deserialize)]
struct EinheitPruefen {
    eingabe: core_lib::baugrundstuecke::BaugrundstuecksPlan,
    schluessel: String,
}
async fn baugrundstueck_einzeln_pruefen(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<EinheitPruefen>,
) -> ApiResult<core_lib::baugrundstuecke::BaugrundstuecksPruefung> {
    Ok(Json(
        core.baugrundstueck_einzeln_pruefen(ProjectId(id), &w.eingabe, &w.schluessel)
            .await?,
    ))
}
async fn baugrundstueck_einzeln_einbinden(
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<EinheitPruefen>,
) -> ApiResult<core_lib::plattform::einbindung::GebaeudeEinbindung> {
    Ok(Json(
        core.baugrundstueck_gebaeude_einbinden(ProjectId(id), &w.eingabe, &w.schluessel)
            .await?,
    ))
}
#[derive(Deserialize)]
struct EinheitSpeichern {
    version: i64,
    berechnung: core_lib::baugrundstuecke::BaugrundstueckBerechnung,
}
async fn baugrundstueck_einzeln_speichern(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<EinheitSpeichern>,
) -> ApiResult<Project> {
    Ok(Json(
        core.baugrundstueck_einzeln_speichern(ProjectId(id), w.version, w.berechnung, &wer)
            .await?,
    ))
}
async fn cadaufmass_speichern(
    Extension(wer): Extension<Benutzer>,
    State(ApiState { core }): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(w): Json<CadSpeichern>,
) -> ApiResult<core_lib::commit::Fachstand> {
    Ok(Json(
        core.cadaufmass_speichern(ProjectId(id), w.aufmass, w.basis, &wer)
            .await?,
    ))
}
