//! Tauri-Commands: nur Übersetzung zwischen IPC und core-lib.
//! Hier liegt bewusst keine Geschäftslogik.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bytes::Bytes;
use core_lib::CoreError;
use core_lib::benutzer::{Benutzer, BenutzerAenderung, NeuerBenutzer};
use core_lib::cad::{Laengeneinheit, Messung, Punkt};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentLock, DocumentVersion, NewProject, Project, ProjectId,
    ProjectUpdate, Protokolleintrag,
};
use core_lib::formular::Feldwert;
use core_lib::kunden::{Beteiligtenzuordnung, Kunde, KundeId, Kundendaten, Projektbeteiligter};
use core_lib::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, RulePackInfo};
use core_lib::license::Lizenzstatus;
use core_lib::service::{CadPlan, FormularAnalyse};
use core_lib::xbau::{Bauantrag, Buero, Codeliste, GespeicherterBauantrag};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::bootstrap::AppState;
use crate::menu::{MenuState, ZuletztEintrag};

/// Fehler, wie das Frontend sie sieht.
#[derive(Debug, Serialize)]
pub struct CmdError {
    pub art: &'static str,
    pub meldung: String,
}

impl From<CoreError> for CmdError {
    fn from(e: CoreError) -> Self {
        let art = match &e {
            CoreError::NotFound(_) => "nicht_gefunden",
            CoreError::VersionConflict { .. } => "konflikt",
            CoreError::InvalidInput(_) => "ungueltig",
            CoreError::RulePackMissing(_) | CoreError::Script { .. } => "lbo",
            CoreError::License(_) => "lizenz",
            CoreError::Forbidden(_) => "keine_berechtigung",
            CoreError::Gesperrt { .. } => "gesperrt",
            _ => "intern",
        };
        Self {
            art,
            meldung: e.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CmdError>;

#[tauri::command]
pub async fn projektmitglieder(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<core_lib::commit::Fachstand>> {
    Ok(state
        .core
        .projektmitglieder(projekt_id, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn projektmitglied_setzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    mitglied: core_lib::zugriff::ProjektMitglied,
    basis: i64,
) -> CmdResult<core_lib::commit::Fachstand> {
    Ok(state
        .core
        .projektmitglied_setzen(projekt_id, mitglied, basis, &state.benutzer())
        .await?)
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    benutzer: Benutzer,
    modus: &'static str,
    storage: String,
    regelpakete: Vec<RulePackInfo>,
    cache: Option<CacheInfo>,
}

#[derive(Serialize)]
pub struct CacheInfo {
    eintraege: usize,
    belegt_bytes: u64,
    treffer: u64,
    downloads: u64,
}

#[tauri::command]
pub async fn app_info(app: tauri::AppHandle, state: State<'_, AppState>) -> CmdResult<AppInfo> {
    let cache = match &state.cache {
        Some(cache) => {
            let s = cache.stats().await;
            Some(CacheInfo {
                eintraege: s.entries,
                belegt_bytes: s.total_bytes,
                treffer: s.hits,
                downloads: s.misses,
            })
        }
        None => None,
    };
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        benutzer: state.benutzer(),
        modus: state.config.name(),
        storage: state.core.storage.backend_name(),
        regelpakete: state.core.lbo.rule_packs(),
        cache,
    })
}

#[tauri::command]
pub async fn projekte_auflisten(state: State<'_, AppState>) -> CmdResult<Vec<Project>> {
    Ok(state.core.projekte_fuer_benutzer(&state.benutzer()).await?)
}

#[tauri::command]
pub async fn projekt_anlegen(
    state: State<'_, AppState>,
    projekt: NewProject,
) -> CmdResult<Project> {
    Ok(state
        .core
        .create_project(projekt, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn projekt_aktualisieren(
    state: State<'_, AppState>,
    aenderung: ProjectUpdate,
) -> CmdResult<Project> {
    state
        .core
        .projekt_zugriff(
            aenderung.id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .update_project(aenderung, &state.benutzer())
        .await?)
}

/// Liest eine Projektakte ohne den aktiven Fenster-/Menükontext zu ändern.
#[tauri::command]
pub async fn projekt_laden(state: State<'_, AppState>, id: ProjectId) -> CmdResult<Project> {
    state
        .core
        .projekt_zugriff(
            id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.get_project(id).await?)
}

/// Öffnet ein Projekt: merkt es unter "Zuletzt geöffnet", aktiviert die
/// projektbezogenen Menüpunkte und setzt den Fenstertitel.
#[tauri::command]
pub async fn projekt_oeffnen(
    app: AppHandle,
    state: State<'_, AppState>,
    menu: State<'_, MenuState>,
    id: ProjectId,
) -> CmdResult<Project> {
    state
        .core
        .projekt_zugriff(
            id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    let projekt = match state.core.get_project(id).await {
        Ok(p) => p,
        Err(e) => {
            if matches!(e, CoreError::NotFound(_)) {
                menu.vergiss_projekt(&app, &id.to_string());
            }
            return Err(e.into());
        }
    };
    menu.merke_projekt(
        &app,
        ZuletztEintrag {
            id: projekt.id.to_string(),
            name: projekt.name.clone(),
        },
    );
    menu.set_projekt_offen(true);
    set_titel(&app, Some(&projekt.name));
    Ok(projekt)
}

#[tauri::command]
pub fn projekt_schliessen(app: AppHandle, menu: State<'_, MenuState>) {
    menu.set_projekt_offen(false);
    set_titel(&app, None);
}

#[tauri::command]
pub async fn zuletzt_geoeffnet(
    state: State<'_, AppState>,
    menu: State<'_, MenuState>,
) -> CmdResult<Vec<ZuletztEintrag>> {
    let sichtbar = state.core.projekte_fuer_benutzer(&state.benutzer()).await?;
    let zuletzt = menu.zuletzt.lock().expect("Menü-Lock vergiftet").0.clone();
    Ok(zuletzt
        .into_iter()
        .filter(|e| sichtbar.iter().any(|p| p.id.to_string() == e.id))
        .collect())
}

fn set_titel(app: &AppHandle, projekt: Option<&str>) {
    let titel = match projekt {
        Some(name) => format!("{name} – BauProjektIO"),
        None => "BauProjektIO".to_owned(),
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&titel);
    }
}

#[tauri::command]
pub async fn dokumente_auflisten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<DocumentVersion>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.list_documents(projekt_id).await?)
}

/// Importiert eine lokale Datei ins DMS. Für eine neue Version eines
/// bestehenden Dokuments gibt `basis` die Version an, die der Nutzer
/// gesehen hat (Optimistic Locking).
#[tauri::command]
pub async fn dokument_importieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    dokument_id: Option<DocumentId>,
    basis: Option<i64>,
    pfad: PathBuf,
    kategorie: Option<String>,
) -> CmdResult<crate::projektordner::ProjektDokument> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    let data = tokio::fs::read(&pfad).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", pfad.display()),
    })?;
    let name = pfad
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let d = if dokument_id.is_none() && kategorie.is_some() {
        state
            .core
            .store_document_kategorisiert(
                projekt_id,
                &name,
                Bytes::from(data),
                kategorie,
                &state.benutzer(),
            )
            .await?
    } else {
        state
            .core
            .store_document(
                projekt_id,
                dokument_id,
                basis,
                &name,
                Bytes::from(data),
                &state.benutzer(),
            )
            .await?
    };
    Ok(crate::projektordner::ablegen(&state, d).await)
}

#[tauri::command]
pub async fn formular_analysieren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    bezug: Option<core_lib::formular::instanz::FormularBezug>,
) -> CmdResult<FormularAnalyse> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state
        .core
        .formular_analysieren_mit_bezug(dokument_id, bezug)
        .await?)
}

#[derive(Serialize)]
pub struct PdfVorschau {
    dokument: DocumentVersion,
    pdf_base64: String,
    seiten: Vec<core_lib::formular::editor::PdfSeite>,
    felder: Vec<core_lib::formular::Formularfeld>,
}
#[tauri::command]
pub async fn pdf_vorschau(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: Option<i64>,
) -> CmdResult<PdfVorschau> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    use base64::Engine;
    let (dokument, pdf) = match version {
        Some(v) => state.core.load_version(dokument_id, v).await?,
        None => state.core.load_document(dokument_id).await?,
    };
    let seiten = core_lib::formular::editor::seiten(&pdf)?;
    let felder = core_lib::formular::pdf::felder_lesen(&pdf)?;
    Ok(PdfVorschau {
        dokument,
        pdf_base64: base64::engine::general_purpose::STANDARD.encode(&pdf),
        seiten,
        felder,
    })
}
/// Temporäre Füllung derselben PDF-Quelle; kein DMS-, Versions- oder Ordnerschreibvorgang.
#[tauri::command]
pub async fn formular_livevorschau(
    pdf_base64: String,
    werte: BTreeMap<String, core_lib::formular::Feldwert>,
) -> CmdResult<String> {
    use base64::Engine;
    if pdf_base64.len() > 44_739_244
        || werte.len() > 5000
        || werte
            .iter()
            .map(|(k, v)| {
                k.len()
                    + match v {
                        core_lib::formular::Feldwert::Text(t) => t.len(),
                        _ => 1,
                    }
            })
            .sum::<usize>()
            > 1_048_576
    {
        return Err(CoreError::InvalidInput(
            "PDF-Vorschau ist zu groß (maximal 32 MiB PDF und 1 MiB Eingaben)".into(),
        )
        .into());
    }
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(pdf_base64)
        .map_err(|_| CoreError::InvalidInput("Ungültige PDF-Daten für die Vorschau".into()))?;
    let neu = tauri::async_runtime::spawn_blocking(move || {
        core_lib::formular::pdf::befuellen(&pdf, &werte)
    })
    .await
    .map_err(|e| CoreError::InvalidInput(format!("PDF-Vorschau: {e}")))??;
    Ok(base64::engine::general_purpose::STANDARD.encode(neu))
}

#[tauri::command]
pub async fn formular_vorlage_erstellen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    basis: i64,
    felder: Vec<core_lib::formular::editor::EigenesPdfFeld>,
) -> CmdResult<DocumentVersion> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .formular_vorlage_erstellen(dokument_id, basis, &felder, &state.benutzer())
        .await?)
}

/// Befüllt das Formular und legt es als neues Dokument im Projekt ab.
#[tauri::command]
pub async fn formular_ausfuellen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    werte: BTreeMap<String, Feldwert>,
    basis: Option<i64>,
) -> CmdResult<DocumentVersion> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .formular_ausfuellen_version(dokument_id, &werte, basis, &state.benutzer())
        .await?)
}

/// Schreibt die aktuelle (oder mit `version` eine ältere) Version eines
/// Dokuments an einen vom Nutzer gewählten Ort (z. B. zum Einreichen beim
/// Bauamt).
#[tauri::command]
pub async fn dokument_exportieren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: Option<i64>,
    ziel: PathBuf,
) -> CmdResult<()> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    let (_, data) = match version {
        Some(v) => state.core.load_version(dokument_id, v).await?,
        None => state.core.load_document(dokument_id).await?,
    };
    tokio::fs::write(&ziel, &data).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", ziel.display()),
    })
}

#[tauri::command]
pub async fn dokument_historie(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<Vec<DocumentVersion>> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.document_history(dokument_id).await?)
}

#[tauri::command]
pub fn abstandsflaeche(
    state: State<'_, AppState>,
    land: Bundesland,
    eingabe: AbstandsflaecheInput,
) -> CmdResult<Berechnung<AbstandsflaecheErgebnis>> {
    Ok(state.core.abstandsflaeche(land, &eingabe)?)
}

/// Lädt einen DXF-Plan aus dem DMS für den Plan-Viewer.
#[tauri::command]
pub async fn cad_plan(state: State<'_, AppState>, dokument_id: DocumentId) -> CmdResult<CadPlan> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.cad_plan(dokument_id).await?)
}

#[tauri::command]
pub fn aufmass(
    state: State<'_, AppState>,
    punkte: Vec<Punkt>,
    geschlossen: bool,
    einheit: Laengeneinheit,
) -> CmdResult<Messung> {
    Ok(state.core.messen(&punkte, geschlossen, einheit)?)
}

#[tauri::command]
pub async fn version_wiederherstellen(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> CmdResult<DocumentVersion> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .version_wiederherstellen(dokument_id, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn dokument_sperren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<DocumentLock> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .dokument_sperren(dokument_id, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn dokument_entsperren(
    state: State<'_, AppState>,
    dokument_id: DocumentId,
) -> CmdResult<()> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .dokument_entsperren(dokument_id, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn sperren_auflisten(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<DocumentLock>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.sperren_im_projekt(projekt_id).await?)
}

#[tauri::command]
pub async fn protokoll(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<Protokolleintrag>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.protokoll(projekt_id, 500).await?)
}

#[tauri::command]
pub async fn benutzer_auflisten(state: State<'_, AppState>) -> CmdResult<Vec<Benutzer>> {
    Ok(state.core.benutzer_auflisten().await?)
}

#[tauri::command]
pub async fn benutzer_anlegen(
    state: State<'_, AppState>,
    neu: NeuerBenutzer,
) -> CmdResult<Benutzer> {
    Ok(state.core.benutzer_anlegen(neu, &state.benutzer()).await?)
}

/// Ändert einen Benutzer. Betrifft es den angemeldeten Benutzer selbst,
/// gelten die neuen Rechte sofort.
#[tauri::command]
pub async fn benutzer_aendern(
    state: State<'_, AppState>,
    aenderung: BenutzerAenderung,
) -> CmdResult<Benutzer> {
    let b = state
        .core
        .benutzer_aendern(aenderung, &state.benutzer())
        .await?;
    let mut ich = state.benutzer.write().expect("Benutzer-Lock vergiftet");
    if ich.id == b.id {
        *ich = b.clone();
    }
    Ok(b)
}

#[tauri::command]
pub async fn lizenz_status(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    Ok(state.core.lizenz().await?)
}

/// Lizenzdatei einspielen (Administrator) und danach den Arbeitsplatz mit
/// der neuen Anzahl Plätze belegen.
#[tauri::command]
pub async fn lizenz_einspielen(
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> CmdResult<Lizenzstatus> {
    let datei = tokio::fs::read(&pfad).await.map_err(|e| CmdError {
        art: "datei",
        meldung: format!("{}: {e}", pfad.display()),
    })?;
    let ich = state.benutzer();
    state.core.lizenz_einspielen(&datei, &ich).await?;
    arbeitsplatz_belegen(state).await
}

/// Abo-Lizenzschlüssel eingeben (Administrator): aktiviert diesen Rechner
/// sofort, die übrigen beim nächsten Kontakt zum Lizenzserver.
#[tauri::command]
pub async fn lizenz_aktivieren(
    state: State<'_, AppState>,
    schluessel: String,
) -> CmdResult<Lizenzstatus> {
    let ich = state.benutzer();
    state.core.abo_aktivieren(&schluessel, &ich).await?;
    arbeitsplatz_belegen(state).await
}

/// Bestätigung jetzt beim Lizenzserver erneuern.
#[tauri::command]
pub async fn lizenz_bestaetigen(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    state.core.abo_bestaetigen().await?;
    arbeitsplatz_belegen(state).await
}

/// Diesen Rechner beim Lizenzserver abmelden (Administrator), damit ein
/// anderer seinen Platz bekommt.
#[tauri::command]
pub async fn lizenz_abmelden(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    let ich = state.benutzer();
    Ok(state.core.abo_abmelden(&ich).await?)
}

/// Arbeitsplatz (erneut) belegen, z. B. wenn ein Kollege die App beendet hat.
#[tauri::command]
pub async fn arbeitsplatz_belegen(state: State<'_, AppState>) -> CmdResult<Lizenzstatus> {
    let name = state.benutzer().anzeigename;
    Ok(state
        .core
        .license
        .arbeitsplatz_belegen(&state.arbeitsplatz, &name)
        .await?)
}

// --- Kunden --------------------------------------------------------------

#[tauri::command]
pub async fn kunden_auflisten(state: State<'_, AppState>) -> CmdResult<Vec<Kunde>> {
    Ok(state.core.kunden_auflisten().await?)
}

#[tauri::command]
pub async fn kunde_anlegen(state: State<'_, AppState>, daten: Kundendaten) -> CmdResult<Kunde> {
    Ok(state.core.kunde_anlegen(daten, &state.benutzer()).await?)
}

#[tauri::command]
pub async fn kunde_aendern(
    state: State<'_, AppState>,
    kunde_id: KundeId,
    daten: Kundendaten,
    version: i64,
) -> CmdResult<Kunde> {
    Ok(state
        .core
        .kunde_aendern(kunde_id, daten, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn kunde_loeschen(
    state: State<'_, AppState>,
    kunde_id: KundeId,
    version: i64,
) -> CmdResult<()> {
    Ok(state
        .core
        .kunde_loeschen(kunde_id, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn kunde_projekte(
    state: State<'_, AppState>,
    kunde_id: KundeId,
) -> CmdResult<Vec<Project>> {
    let sichtbar = state.core.projekte_fuer_benutzer(&state.benutzer()).await?;
    Ok(state
        .core
        .kunde_projekte(kunde_id)
        .await?
        .into_iter()
        .filter(|p| sichtbar.iter().any(|s| s.id == p.id))
        .collect())
}

#[tauri::command]
pub async fn bauherren(state: State<'_, AppState>, projekt_id: ProjectId) -> CmdResult<Vec<Kunde>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.bauherren(projekt_id).await?)
}

#[tauri::command]
pub async fn bauherren_setzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    kunden: Vec<KundeId>,
    version: i64,
) -> CmdResult<Project> {
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
        .bauherren_setzen(projekt_id, kunden, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn beteiligte(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<Projektbeteiligter>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.beteiligte(projekt_id).await?)
}

#[tauri::command]
pub async fn beteiligte_setzen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    beteiligte: Vec<Beteiligtenzuordnung>,
    version: i64,
) -> CmdResult<Project> {
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
        .beteiligte_setzen(projekt_id, beteiligte, version, &state.benutzer())
        .await?)
}

// --- XBau ----------------------------------------------------------------

#[tauri::command]
pub async fn bauantrag(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<GespeicherterBauantrag> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.bauantrag(projekt_id).await?)
}

#[tauri::command]
pub async fn bauantrag_speichern(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    antrag: Bauantrag,
    version: i64,
) -> CmdResult<GespeicherterBauantrag> {
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
        .bauantrag_speichern(projekt_id, antrag, version, &state.benutzer())
        .await?)
}

#[tauri::command]
pub async fn buero(state: State<'_, AppState>) -> CmdResult<Buero> {
    Ok(state.core.buero().await?)
}

#[tauri::command]
pub async fn buero_speichern(state: State<'_, AppState>, buero: Buero) -> CmdResult<Buero> {
    Ok(state.core.buero_speichern(buero, &state.benutzer()).await?)
}

#[tauri::command]
pub fn xbau_codelisten(state: State<'_, AppState>) -> Vec<Codeliste> {
    state.core.codelisten.alle().cloned().collect()
}

#[tauri::command]
pub async fn projekt_nutzung_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    eingabe: core_lib::festsetzungen::NutzungsEingabe,
) -> CmdResult<core_lib::festsetzungen::NutzungsPruefung> {
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
        .projekt_nutzung_pruefen(projekt_id, &eingabe)
        .await?)
}

#[tauri::command]
pub async fn projekt_baugrundstuecke_pruefen(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    eingabe: core_lib::baugrundstuecke::BaugrundstuecksPlan,
) -> CmdResult<core_lib::baugrundstuecke::BaugrundstuecksPruefung> {
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
        .projekt_baugrundstuecke_pruefen(projekt_id, &eingabe)
        .await?)
}

#[tauri::command]
pub fn baugrenzen_pruefen(
    state: State<'_, AppState>,
    geometrie: core_lib::cad::baugrenzen::BaugrenzenGeometrie,
) -> CmdResult<core_lib::cad::baugrenzen::BaugrenzenPruefung> {
    Ok(state.core.baugrenzen_pruefen(&geometrie)?)
}

#[derive(Serialize)]
pub struct AblageStatus {
    modus: &'static str,
    pub root: Option<String>,
    datenbank: String,
    wechsel_erlaubt: bool,
    hinweis: String,
}
#[tauri::command]
pub async fn ablage_status(state: State<'_, AppState>) -> CmdResult<AblageStatus> {
    let root = if let Some(a) = &state.lokale_ablage {
        Some(a.root().await.to_string_lossy().into_owned())
    } else {
        match &state.config {
            crate::config::AppConfig::Netzwerk {
                storage:
                    storage::StorageConfig::Local { root } | storage::StorageConfig::Nas { root },
                ..
            } => Some(root.to_string_lossy().into_owned()),
            _ => None,
        }
    };
    let lokale_db = state.data_dir.join("bauprojekt.db");
    Ok(AblageStatus{modus:state.config.name(),root,datenbank:if state.lokale_ablage.is_some(){lokale_db.display().to_string()}else{"Gemeinsame PostgreSQL-Datenbank (Zugang in der Büro-Konfiguration)".into()},wechsel_erlaubt:state.lokale_ablage.is_some()&&state.benutzer().rolle==core_lib::benutzer::Rolle::Administrator,hinweis:if state.lokale_ablage.is_some(){"Der Wechsel kopiert alle DMS-Dateiversionen mit Hash-Prüfung. Die bisherige Ablage bleibt als Sicherung erhalten. Die Projektdatenbank bleibt an ihrem bisherigen Ort. Eine Freigabe allein macht daraus noch keinen Mehrbenutzerbetrieb."}else{"Gemeinsame Ablage und Datenbank sind zentral in config.json konfiguriert. Ein abweichender Ordner je Arbeitsplatz würde gemeinsame Dokumente unerreichbar machen."}.into()})
}
#[tauri::command]
pub async fn ablage_wechseln(state: State<'_, AppState>, pfad: PathBuf) -> CmdResult<usize> {
    state
        .core
        .berechtigung_pruefen(
            &state.benutzer(),
            core_lib::benutzer::Recht::BenutzerVerwalten,
        )
        .await?;
    let a = state.lokale_ablage.as_ref().ok_or_else(|| {
        CoreError::InvalidInput("Im Netzwerkmodus wird die Ablage zentral konfiguriert".into())
    })?;
    let dir = state.config_dir.clone();
    Ok(a.wechseln(&pfad, |root| {
        crate::config::DateiablageConfig {
            root: root.to_path_buf(),
        }
        .save(&dir)
        .map_err(CoreError::Storage)
    })
    .await?)
}
#[derive(Serialize)]
pub struct ProjektordnerExport {
    ordner: String,
    dateien: usize,
    neu: usize,
}
#[tauri::command]
pub async fn projektordner_exportieren(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    ziel: PathBuf,
    historie: bool,
) -> CmdResult<ProjektordnerExport> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    if !ziel.is_absolute() {
        return Err(CoreError::InvalidInput("Absoluten Zielordner wählen".into()).into());
    }
    let plan = state.core.projektordner_plan(projekt_id, historie).await?;
    let mut neu = 0;
    for f in &plan.dateien {
        let (_, data) = state
            .core
            .load_version(f.dokument.document_id, f.dokument.version)
            .await?;
        let hash = f.dokument.sha256.clone();
        if storage::projekt_export::unveraenderlich_schreiben(
            &ziel,
            &format!("{}/{}", plan.ordner, f.relativer_pfad),
            &data,
            &hash,
        )
        .await?
        {
            neu += 1;
        }
    }
    // Each export manifest has its own name; previous snapshots are retained.
    let manifest = serde_json::to_vec_pretty(&plan).map_err(|e| CmdError {
        art: "datei",
        meldung: e.to_string(),
    })?;
    let path = format!("{}/Projektstand-{}.json", plan.ordner, uuid::Uuid::new_v4());
    storage::projekt_export::unveraenderlich_schreiben(
        &ziel,
        &path,
        &manifest,
        &core_lib::storage::ContentHash::of(&manifest),
    )
    .await?;
    Ok(ProjektordnerExport {
        ordner: ziel.join(&plan.ordner).display().to_string(),
        dateien: plan.dateien.len(),
        neu,
    })
}

#[tauri::command]
pub async fn formular_im_projekt_speichern(
    state: State<'_, AppState>,
    datenstand: Option<core_lib::formular::instanz::FormularDatenstand>,
    dokument_id: DocumentId,
    basis: i64,
    werte: BTreeMap<String, Feldwert>,
) -> CmdResult<crate::projektordner::ProjektDokument> {
    state
        .core
        .dokument_zugriff(
            dokument_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    let d = state
        .core
        .formular_im_projekt_mit_quellen(dokument_id, basis, &werte, datenstand, &state.benutzer())
        .await?;
    Ok(crate::projektordner::ablegen(&state, d).await)
}

#[tauri::command]
pub async fn baugrundstuecke_ermitteln(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
    plan: core_lib::baugrundstuecke::BaugrundstuecksPlan,
) -> CmdResult<Vec<core_lib::baugrundstuecke::BaugrundstueckGruppe>> {
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
        .baugrundstuecke_ermitteln(projekt_id, &plan)
        .await?)
}

#[tauri::command]
pub async fn projekt_bereich_speichern(
    state: State<'_, AppState>,
    basis: Project,
    bereich: core_lib::projektbereiche::ProjektBereich,
    aenderung: ProjectUpdate,
) -> CmdResult<Project> {
    state
        .core
        .projekt_zugriff(
            aenderung.id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    Ok(state
        .core
        .projekt_bereich_speichern(basis, bereich, aenderung, &state.benutzer())
        .await?)
}
#[tauri::command]
pub async fn grundstueckskontext(
    state: State<'_, AppState>,
    projekt_id: ProjectId,
) -> CmdResult<Vec<core_lib::grundstueckskontext::GrundstueckKontext>> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Lesen,
        )
        .await?;
    Ok(state.core.grundstueckskontext(projekt_id).await?)
}
