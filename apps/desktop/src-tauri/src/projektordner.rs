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
    let root = state.projektordner.read().await;
    let result = async {
        let p = state.core.get_project(d.project_id).await?;
        let plan = core_lib::projektablage::planen(&p, std::slice::from_ref(&d))?;
        let path = format!("{}/{}", plan.ordner, plan.dateien[0].relativer_pfad);
        let (_, data) = state.core.load_version(d.document_id, d.version).await?;
        storage::projekt_export::unveraenderlich_schreiben(&root, &path, &data, &d.sha256).await?;
        Ok::<_, CoreError>(root.join(path).display().to_string())
    }
    .await;
    match result {
        Ok(p) => ProjektDokument {
            dokument: d,
            projektordner_pfad: Some(p),
            ablage_hinweis: None,
        },
        Err(e) => ProjektDokument {
            dokument: d,
            projektordner_pfad: None,
            ablage_hinweis: Some(format!(
                "Im Projekt gespeichert; lesbare Ordnerausgabe fehlgeschlagen: {e}"
            )),
        },
    }
}
#[tauri::command]
pub async fn projektordner_status(state: State<'_, AppState>) -> Result<String, CmdError> {
    Ok(state.projektordner.read().await.display().to_string())
}
#[tauri::command]
pub async fn projektordner_waehlen(
    app: AppHandle,
    state: State<'_, AppState>,
    pfad: PathBuf,
) -> Result<(), CmdError> {
    state
        .benutzer()
        .pruefe(core_lib::benutzer::Recht::BenutzerVerwalten)?;
    if !pfad.is_absolute() {
        return Err(CoreError::InvalidInput("Absoluten Projektordner wählen".into()).into());
    }
    tokio::fs::create_dir_all(&pfad)
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| CoreError::Storage(e.to_string()))?;
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
            benutzer: RwLock::new(user),
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
