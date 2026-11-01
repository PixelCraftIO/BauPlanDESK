//! Büro-Cockpit: nur lesbare Projekte, Verfahrensstand aus der gespeicherten Akte.
use core_lib::{
    CoreServices,
    benutzer::{NeuerBenutzer, Rolle},
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn cockpit_zeigt_nur_lesbare_projekte_mit_verfahrensstand() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Büroleitung").await.unwrap();
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
    let neu = |name: &str| NewProject {
        name: name.into(),
        bundesland: Bundesland::Nw,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let fremd = core
        .create_project(neu("Fremdes Haus"), &admin)
        .await
        .unwrap();
    let eigen = core
        .create_project(neu("Eigenes Haus"), &planer)
        .await
        .unwrap();
    let akte = serde_json::json!({
        "bezeichnung": "Bauantrag EFH", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht Musterstadt", "kennung": "", "kontakt": ""},
        "aktenzeichen": "63/2026/14", "verantwortlich": planer.id.0, "vorgaenger": null, "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    });
    for (p, wer) in [(fremd.id, &admin), (eigen.id, &planer)] {
        core.fachobjekt_speichern(
            p,
            ObjektArt::Verfahren,
            Uuid::new_v4(),
            0,
            akte.clone(),
            wer,
        )
        .await
        .unwrap();
    }
    let heute = core_lib::kalender::heute();

    let c = core.buero_cockpit(&planer, heute).await.unwrap();
    assert_eq!(c.projekte.len(), 1, "fremdes Projekt darf nicht erscheinen");
    let p = &c.projekte[0];
    assert_eq!(p.name, "Eigenes Haus");
    assert_eq!(p.verfahren.len(), 1);
    assert_eq!(p.verfahren[0].aktenzeichen, "63/2026/14");
    assert_eq!(p.verfahren[0].status_bezeichnung, "In Vorbereitung");
    assert_eq!(p.verfahren[0].verantwortlich, Some(planer.id.0));
    assert!(c.fehler.is_empty());

    let c = core.buero_cockpit(&admin, heute).await.unwrap();
    assert_eq!(c.projekte.len(), 2);
}
