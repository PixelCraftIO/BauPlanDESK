//! Lesbare Projektordner sind eine automatische Ausgabe des verbindlichen DMS.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{CoreError, domain::DocumentVersion};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
#[derive(Serialize)]
pub struct ProjektDokument {
    #[serde(flatten)]
    pub dokument: DocumentVersion,
    pub projektordner_pfad: Option<String>,
    pub ablage_hinweis: Option<String>,
}
pub async fn ablegen(state: &AppState, d: DocumentVersion) -> ProjektDokument {
    match ausgeben(state, &d, true).await {
        Ok(Some(path)) => ProjektDokument {
            dokument: d,
            projektordner_pfad: Some(path),
            ablage_hinweis: None,
        },
        Ok(None) => ProjektDokument {
            dokument: d,
            projektordner_pfad: None,
            ablage_hinweis: Some(
                "Im Projekt gespeichert; die Ordnerausgabe wird bereits bearbeitet.".into(),
            ),
        },
        Err(e) => ProjektDokument {
            dokument: d,
            projektordner_pfad: None,
            ablage_hinweis: Some(format!(
                "Im Projekt gespeichert; lesbare Ordnerausgabe wird erneut versucht: {e}"
            )),
        },
    }
}
async fn ausgeben(
    state: &AppState,
    d: &DocumentVersion,
    erzwingen: bool,
) -> Result<Option<String>, CoreError> {
    state
        .core
        .projekt_zugriff(
            d.project_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    let root = state.projektordner.read().await;
    let p = state.core.get_project(d.project_id).await?;
    let plan = core_lib::projektablage::planen(&p, std::slice::from_ref(d))?;
    let path = format!("{}/{}", plan.ordner, plan.dateien[0].relativer_pfad);
    let wer = state.benutzer();
    let Some((f, a)) = state
        .core
        .ausgabe_beanspruchen(d, root.display().to_string(), path.clone(), erzwingen, &wer)
        .await?
    else {
        return Ok(None);
    };
    let result = async {
        let (_, data) = state.core.load_version(d.document_id, d.version).await?;
        storage::projekt_export::unveraenderlich_schreiben(&root, &path, &data, &d.sha256).await?;
        Ok::<_, CoreError>(())
    }
    .await;
    state
        .core
        .ausgabe_abschliessen(f, a, result.as_ref().err().map(ToString::to_string), &wer)
        .await?;
    result?;
    Ok(Some(root.join(path).display().to_string()))
}
/// Neue DMS-Versionen besitzen ein atomar gespeichertes Ereignis. Ein abgebrochener
/// Prozess verliert es nicht; abgelaufene Bearbeitungsfristen dürfen neu beansprucht werden.
pub fn hintergrund(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AppState>();
            if state
                .core
                .berechtigung_pruefen(
                    &state.benutzer(),
                    core_lib::benutzer::Recht::DokumentAblegen,
                )
                .await
                .is_ok()
            {
                match state.core.projekte_fuer_benutzer(&state.benutzer()).await {
                    Ok(projekte) => {
                        let mut behandelt = 0;
                        for p in projekte {
                            if behandelt >= 50 {
                                break;
                            }
                            match state
                                .core
                                .projekt_zugriff(
                                    p.id,
                                    &state.benutzer(),
                                    core_lib::zugriff::ProjektRecht::Bearbeiten,
                                )
                                .await
                            {
                                Ok(()) => {}
                                Err(CoreError::Forbidden(_)) => continue,
                                Err(e) => {
                                    tracing::warn!(fehler=%e,"Projektzugriff für Ordnerausgabe nicht prüfbar");
                                    continue;
                                }
                            }
                            let root = state.projektordner.read().await.display().to_string();
                            let auftraege = match state.core.ausgabeauftraege(p.id).await {
                                Ok(a) => a,
                                Err(e) => {
                                    tracing::warn!(fehler=%e,"Ausgabeaufträge nicht lesbar");
                                    continue;
                                }
                            };
                            if let Ok(dokumente) = state.core.list_documents(p.id).await {
                                for d in dokumente {
                                    if behandelt >= 50 {
                                        break;
                                    }
                                    let zugeordnet: Vec<_> = auftraege
                                        .iter()
                                        .filter(|(_, a)| {
                                            a.dokument_id == d.document_id
                                                && a.dokument_version == d.version
                                        })
                                        .collect();
                                    if !zugeordnet.is_empty() {
                                        let aktuell =
                                            zugeordnet.iter().find(|(_, a)| a.ziel_root == root);
                                        if aktuell.is_none_or(|(_,a)|a.status==core_lib::plattform::ausgabe::AusgabeStatus::Fertig || a.status==core_lib::plattform::ausgabe::AusgabeStatus::Laeuft&&a.beansprucht_bis>chrono::Utc::now() || a.status==core_lib::plattform::ausgabe::AusgabeStatus::Fehler&&a.naechster_versuch>chrono::Utc::now()) {continue;}
                                    }
                                    match state
                                        .core
                                        .repos
                                        .commit
                                        .dokumentzusatz(d.document_id, d.version, "ausgabeereignis")
                                        .await
                                    {
                                        Ok(Some(_)) => match ausgeben(&state, &d, false).await {
                                            Ok(Some(_)) => behandelt += 1,
                                            Ok(None) => {}
                                            Err(e) => {
                                                behandelt += 1;
                                                tracing::warn!(fehler=%e,dokument=%d.document_id.0,"Ordnerausgabe bleibt zur Wiederholung vorgemerkt");
                                            }
                                        },
                                        Ok(None) => {}
                                        Err(e) => {
                                            tracing::warn!(fehler=%e,"Ausgabeereignis konnte nicht gelesen werden")
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(fehler=%e,"Ausgabeaufträge konnten nicht geladen werden")
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
}
#[tauri::command]
pub async fn ausgabeauftraege(
    state: State<'_, AppState>,
    projekt_id: core_lib::domain::ProjectId,
) -> Result<Vec<core_lib::plattform::ausgabe::Ausgabeauftrag>, CmdError> {
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
        .ausgabeauftraege(projekt_id)
        .await?
        .into_iter()
        .map(|(_, a)| a)
        .collect())
}
#[tauri::command]
pub async fn ausgabe_wiederholen(
    state: State<'_, AppState>,
    projekt_id: core_lib::domain::ProjectId,
    id: uuid::Uuid,
) -> Result<ProjektDokument, CmdError> {
    state
        .core
        .projekt_zugriff(
            projekt_id,
            &state.benutzer(),
            core_lib::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
    let a = state
        .core
        .ausgabeauftraege(projekt_id)
        .await?
        .into_iter()
        .find(|(_, a)| a.id == id)
        .ok_or_else(|| CoreError::NotFound("Ausgabeauftrag".into()))?
        .1;
    let (d, _) = state
        .core
        .load_version(a.dokument_id, a.dokument_version)
        .await?;
    Ok(ablegen(&state, d).await)
}
#[tauri::command]
pub async fn projektordner_status(state: State<'_, AppState>) -> Result<String, CmdError> {
    Ok(state.projektordner.read().await.display().to_string())
}
#[tauri::command]
pub async fn projektordner_waehlen(
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<(), CmdError> {
    state
        .core
        .berechtigung_pruefen(
            &state.benutzer(),
            core_lib::benutzer::Recht::BenutzerVerwalten,
        )
        .await?;
    if !pfad.is_absolute() {
        return Err(CoreError::InvalidInput("Absoluten Projektordner wählen".into()).into());
    }
    tokio::fs::create_dir_all(&pfad)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    let dir = state.config_dir.clone();
    let mut root = state.projektordner.write().await;
    crate::config::DateiablageConfig { root: pfad.clone() }
        .save_named(&dir, "projektordner.json")
        .map_err(CoreError::Storage)?;
    *root = pfad;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_lib::{
        CoreServices,
        domain::{Bundesland, NewProject},
        lbo::LboEngine,
        license::DevelopmentLicense,
    };
    use std::sync::{Arc, RwLock};
    #[tokio::test]
    async fn automatische_ausgabe_und_fehler_erhalten_das_projektdokument() {
        let root = std::env::temp_dir().join(format!("bauprojekt-ordner-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
        let core = CoreServices::new(
            db.repositories(),
            Arc::new(storage::LocalStorage::new(root.join("DMS"), "local").unwrap()),
            Arc::new(LboEngine::new()),
            Arc::new(DevelopmentLicense),
        );
        let user = core.anmelden("ordner-test", "Test").await.unwrap();
        let p = core
            .create_project(
                NewProject {
                    name: "Wohnhaus".into(),
                    bundesland: Bundesland::By,
                    grundstuecksflaeche_m2: None,
                    planungsrecht: Default::default(),
                },
                &user,
            )
            .await
            .unwrap();
        let d = core
            .store_document(
                p.id,
                None,
                None,
                "Bauantrag.pdf",
                bytes::Bytes::from_static(b"ausgefuellt"),
                &user,
            )
            .await
            .unwrap();
        let state = AppState {
            core,
            config_dir: root.join("config"),
            data_dir: root.clone(),
            benutzer: RwLock::new(user),
            ausstehend: RwLock::new(None),
            arbeitsplatz: "Test".into(),
            config: crate::config::AppConfig::Einzelplatz,
            cache: None,
            lokale_ablage: None,
            formularordner: tokio::sync::RwLock::new(root.join("Formulare")),
            projektordner: tokio::sync::RwLock::new(root.join("Projekte")),
        };
        let output = ablegen(&state, d.clone()).await;
        assert!(output.ablage_hinweis.is_none());
        let path = output.projektordner_pfad.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"ausgefuellt");
        assert!(path.contains("Dokumente"));
        let repeat = ablegen(&state, d.clone()).await;
        assert!(repeat.ablage_hinweis.is_none());
        let admin = state.benutzer();
        let fremder = state
            .core
            .benutzer_anlegen(
                core_lib::benutzer::NeuerBenutzer {
                    anmeldename: "fremder".into(),
                    anzeigename: "Ohne Projektmitgliedschaft".into(),
                    rolle: core_lib::benutzer::Rolle::Bearbeitung,
                },
                &admin,
            )
            .await
            .unwrap();
        *state.benutzer.write().unwrap() = fremder;
        assert!(matches!(
            ausgeben(&state, &d, true).await,
            Err(CoreError::Forbidden(_))
        ));
        *state.benutzer.write().unwrap() = admin;
        let blocked = root.join("keine-map");
        std::fs::write(&blocked, b"datei").unwrap();
        *state.projektordner.write().await = blocked;
        let failed = ablegen(&state, d.clone()).await;
        assert!(failed.ablage_hinweis.is_some());
        assert_eq!(failed.dokument.document_id, d.document_id);
        assert_eq!(state.core.list_documents(p.id).await.unwrap().len(), 1);
        assert_eq!(
            state.core.load_document(d.document_id).await.unwrap().1,
            b"ausgefuellt".as_slice()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
