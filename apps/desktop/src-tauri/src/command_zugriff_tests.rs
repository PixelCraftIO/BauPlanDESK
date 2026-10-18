//! Reale IPC-Funktionen mit SQLite/DMS: Projektmitgliedschaft gilt vor jeder Ausgabe.
use crate::{bootstrap::AppState, commands as c};
use core_lib::{
    CoreServices,
    benutzer::{NeuerBenutzer, Rolle},
    domain::{Bundesland, NewProject, ProjectUpdate},
    lbo::LboEngine,
    license::DevelopmentLicense,
    zugriff::{ProjektMitglied, ProjektRecht},
};
use std::sync::{Arc, RwLock};
use tauri::Manager;

#[tokio::test]
async fn desktop_filtert_projekte_und_schuetzt_stammdaten_dokumente_und_exporte() {
    let root = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(root.path().join("DMS"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Administration").await.unwrap();
    let planer = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "planer".into(),
                anzeigename: "Planung".into(),
                rolle: Rolle::Projektleitung,
            },
            &admin,
        )
        .await
        .unwrap();
    let neu = || NewProject {
        name: "Geschützte Akte".into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let fremd = core.create_project(neu(), &admin).await.unwrap();
    let eigen = core.create_project(neu(), &planer).await.unwrap();
    let dokument = core
        .store_document(
            fremd.id,
            None,
            None,
            "Plan.pdf",
            bytes::Bytes::from_static(b"Original"),
            &admin,
        )
        .await
        .unwrap();
    let app = tauri::test::mock_app();
    app.manage(AppState {
        core,
        config_dir: root.path().join("config"),
        data_dir: root.path().into(),
        benutzer: RwLock::new(planer.clone()),
        arbeitsplatz: "Test".into(),
        config: Default::default(),
        cache: None,
        lokale_ablage: None,
        formularordner: tokio::sync::RwLock::new(root.path().join("Formulare")),
        projektordner: tokio::sync::RwLock::new(root.path().join("Projekte")),
    });
    let liste = c::projekte_auflisten(app.state()).await.unwrap();
    assert_eq!(
        liste.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![eigen.id]
    );
    macro_rules! verboten {
        ($v:expr) => {
            assert_eq!(
                $v.await.err().expect("Zugriff muss abgewiesen werden").art,
                "keine_berechtigung"
            )
        };
    }
    verboten!(c::projekt_laden(app.state(), fremd.id));
    verboten!(c::projekt_recht(app.state(), fremd.id));
    verboten!(c::dokumente_auflisten(app.state(), fremd.id));
    verboten!(c::bauherren(app.state(), fremd.id));
    verboten!(c::beteiligte(app.state(), fremd.id));
    verboten!(c::bauantrag(app.state(), fremd.id));
    verboten!(c::grundstueckskontext(app.state(), fremd.id));
    verboten!(c::pdf_vorschau(app.state(), dokument.document_id, None));
    verboten!(c::dokument_historie(app.state(), dokument.document_id));
    verboten!(c::dokument_sperren(app.state(), dokument.document_id));
    let ziel = root.path().join("export.pdf");
    verboten!(c::dokument_exportieren(
        app.state(),
        dokument.document_id,
        None,
        ziel.clone()
    ));
    assert!(!ziel.exists());
    verboten!(c::projektordner_exportieren(
        app.state(),
        fremd.id,
        root.path().join("Export"),
        true
    ));
    verboten!(crate::plattform::nachweise(app.state(), fremd.id));
    verboten!(crate::plattform::projektstatus(app.state(), fremd.id));
    verboten!(crate::kategorien::dokumentkategorien(app.state(), fremd.id));
    verboten!(crate::einreichungen::einreichungen(app.state(), fremd.id));
    verboten!(c::projektmitglieder(app.state(), fremd.id));
    let aenderung = || ProjectUpdate {
        id: fremd.id,
        expected_version: fremd.version,
        name: "Unerlaubt".into(),
        bundesland: fremd.bundesland,
        grundstuecksflaeche_m2: None,
        planungsrecht: None,
    };
    verboten!(c::projekt_aktualisieren(app.state(), aenderung()));
    assert_eq!(
        app.state::<AppState>()
            .core
            .get_project(fremd.id)
            .await
            .unwrap(),
        fremd
    );

    let core = &app.state::<AppState>().core;
    let m = core
        .projektmitglied_setzen(
            fremd.id,
            ProjektMitglied {
                benutzer_id: planer.id,
                recht: ProjektRecht::Lesen,
            },
            0,
            &admin,
        )
        .await
        .unwrap();
    assert_eq!(
        c::projekt_laden(app.state(), fremd.id).await.unwrap().id,
        fremd.id
    );
    assert_eq!(
        c::projekt_recht(app.state(), fremd.id).await.unwrap(),
        ProjektRecht::Lesen
    );
    assert_eq!(
        c::dokumente_auflisten(app.state(), fremd.id)
            .await
            .unwrap()
            .len(),
        1
    );
    c::dokument_exportieren(app.state(), dokument.document_id, None, ziel.clone())
        .await
        .unwrap();
    assert_eq!(std::fs::read(ziel).unwrap(), b"Original");
    verboten!(c::projekt_aktualisieren(app.state(), aenderung()));
    verboten!(c::projektmitglied_setzen(
        app.state(),
        fremd.id,
        ProjektMitglied {
            benutzer_id: planer.id,
            recht: ProjektRecht::Leitung
        },
        m.revision
    ));
    core.projektmitglied_setzen(
        fremd.id,
        ProjektMitglied {
            benutzer_id: planer.id,
            recht: ProjektRecht::Bearbeiten,
        },
        m.revision,
        &admin,
    )
    .await
    .unwrap();
    assert_eq!(
        c::projekt_aktualisieren(app.state(), aenderung())
            .await
            .unwrap()
            .name,
        "Unerlaubt"
    );
    *app.state::<AppState>().benutzer.write().unwrap() = admin;
    assert_eq!(c::projekte_auflisten(app.state()).await.unwrap().len(), 2);
}
