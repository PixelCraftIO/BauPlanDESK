//! Projekt löschen: Rechte, fremde Sperren, Optimistic Locking, Ablage und Protokoll.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    benutzer::{NeuerBenutzer, Rolle},
    domain::{Aktion, Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::{Storage, StorageKey},
};
use std::sync::Arc;

#[tokio::test]
async fn projekt_mit_akte_und_dateien_loeschen() {
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&temp.path().join("projekt.db"))
        .await
        .unwrap();
    let ablage = Arc::new(storage::LocalStorage::new(temp.path().join("Ablage"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        ablage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let neu = |name: &str| NewProject {
        name: name.into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let p = core.create_project(neu("Abbruch"), &admin).await.unwrap();
    let bleibt = core.create_project(neu("Bleibt"), &admin).await.unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.pdf",
            Bytes::from_static(b"%PDF"),
            &admin,
        )
        .await
        .unwrap();
    core.store_document(
        bleibt.id,
        None,
        None,
        "Plan.pdf",
        Bytes::from_static(b"%PDF"),
        &admin,
    )
    .await
    .unwrap();

    let benutzer = |anmeldename: &str, rolle| NeuerBenutzer {
        anmeldename: anmeldename.into(),
        anzeigename: anmeldename.into(),
        rolle,
    };
    let bearbeiter = core
        .benutzer_anlegen(benutzer("bearbeiter", Rolle::Bearbeitung), &admin)
        .await
        .unwrap();
    let leitung = core
        .benutzer_anlegen(benutzer("leitung", Rolle::Projektleitung), &admin)
        .await
        .unwrap();
    assert!(matches!(
        core.delete_project(p.id, p.version, &bearbeiter).await,
        Err(CoreError::Forbidden(_))
    ));
    // Projektleitung ohne Mitgliedschaft in diesem Projekt.
    assert!(matches!(
        core.delete_project(p.id, p.version, &leitung).await,
        Err(CoreError::Forbidden(_))
    ));

    core.dokument_sperren(plan.document_id, &bearbeiter)
        .await
        .unwrap();
    let gesperrt = core.delete_project(p.id, p.version, &admin).await;
    assert!(
        matches!(&gesperrt, Err(CoreError::InvalidInput(m)) if m.contains("bearbeiter")),
        "{gesperrt:?}"
    );
    core.dokument_entsperren(plan.document_id, &bearbeiter)
        .await
        .unwrap();
    assert!(matches!(
        core.delete_project(p.id, p.version - 1, &admin).await,
        Err(CoreError::VersionConflict { .. })
    ));

    let aktuell = core.get_project(p.id).await.unwrap();
    let ergebnis = core
        .delete_project(p.id, aktuell.version, &admin)
        .await
        .unwrap();
    assert!(ergebnis.dateien_geloescht >= 1);
    assert!(ergebnis.dateien_verblieben.is_empty());
    assert!(matches!(
        core.get_project(p.id).await,
        Err(CoreError::NotFound(_))
    ));
    let prefix = StorageKey::new(format!("projekte/{}", p.id)).unwrap();
    assert!(ablage.list(Some(&prefix)).await.unwrap().is_empty());

    // Das andere Projekt und seine Dateien bleiben unberührt.
    let projekte = core.list_projects().await.unwrap();
    assert_eq!(projekte.len(), 1);
    assert_eq!(projekte[0].id, bleibt.id);
    let rest = StorageKey::new(format!("projekte/{}", bleibt.id)).unwrap();
    assert!(!ablage.list(Some(&rest)).await.unwrap().is_empty());

    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    let vermerk = protokoll
        .iter()
        .find(|e| e.eintrag.aktion == Aktion::ProjektGeloescht)
        .expect("Löschvermerk im Protokoll");
    assert!(vermerk.eintrag.details.starts_with("Abbruch"));
    assert_eq!(vermerk.eintrag.benutzer_id, admin.id);
}
