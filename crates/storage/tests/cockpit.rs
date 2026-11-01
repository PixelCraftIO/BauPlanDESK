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

    // Projekttagebuch: Wiedervorlage erscheint im Cockpit und in der Übersicht.
    let eintrag = |datum: chrono::NaiveDate, wv: chrono::NaiveDate| {
        serde_json::json!({
            "datum": datum, "art": "telefonat", "titel": "Rückruf Statiker",
            "text": "Statik kommt zwei Wochen später.", "beteiligte": "Statikbüro Schmidt",
            "anhaenge": [], "verfahren": null,
            "wiedervorlage": {"datum": wv, "verantwortlich": planer.id.0, "erledigt": false}
        })
    };
    let morgen = heute.succ_opt().unwrap();
    let fehler = core
        .fachobjekt_speichern(
            eigen.id,
            ObjektArt::Projekttagebuch,
            Uuid::new_v4(),
            0,
            eintrag(morgen, morgen),
            &planer,
        )
        .await
        .unwrap_err();
    assert!(fehler.to_string().contains("Zukunft"), "{fehler}");
    let mut fremdanhang = eintrag(heute, morgen);
    fremdanhang["anhaenge"] = serde_json::json!([{"dokument_id": Uuid::new_v4(), "version": 1}]);
    assert!(
        core.fachobjekt_speichern(
            eigen.id,
            ObjektArt::Projekttagebuch,
            Uuid::new_v4(),
            0,
            fremdanhang,
            &planer
        )
        .await
        .is_err()
    );
    let eid = Uuid::new_v4();
    core.fachobjekt_speichern(
        eigen.id,
        ObjektArt::Projekttagebuch,
        eid,
        0,
        eintrag(heute, morgen),
        &planer,
    )
    .await
    .unwrap();
    let c = core.buero_cockpit(&planer, heute).await.unwrap();
    let t = c
        .termine
        .iter()
        .find(|t| t.bezug == "Projekttagebuch")
        .expect("Wiedervorlage im Cockpit");
    assert_eq!(t.titel, "Telefonat: Rückruf Statiker");
    assert_eq!(t.ziel.objekt, Some(eid));
    assert_eq!(t.verantwortlich, Some(planer.id.0));
    let u = core.projektuebersicht(eigen.id, &planer).await.unwrap();
    assert!(
        u.punkte
            .iter()
            .any(|p| p.titel == "Wiedervorlage: Rückruf Statiker"),
        "Wiedervorlage in der Projektübersicht"
    );
}
