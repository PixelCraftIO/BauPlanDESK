//! Kompatibilität: ältere Fachstände werden beim Lesen in aktueller Form
//! geliefert, und das eigene Projektrecht folgt der Core-Zugriffsregel.
use chrono::Utc;
use core_lib::{
    CoreError, CoreServices,
    commit::Fachstand,
    domain::{Aktion, Bundesland, NeuerProtokolleintrag, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    verfahren::Verfahrensakte,
    zugriff::{ProjektMitglied, ProjektRecht},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

fn core(dir: &std::path::Path, db: &db_sqlite::SqliteDatabase) -> CoreServices {
    CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
}

fn projekt(name: &str) -> NewProject {
    NewProject {
        name: name.into(),
        bundesland: Bundesland::Nw,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    }
}

/// Eine Verfahrensakte, wie sie vor Einführung von `schritte` und
/// `antragsangaben` gespeichert wurde.
fn alte_akte() -> serde_json::Value {
    json!({
        "bezeichnung": "Neubau Altbestand",
        "art": "bauantrag",
        "status": "vorbereitung",
        "behoerde": {
            "id": Uuid::nil(),
            "name": "Bauaufsicht Musterstadt",
            "kennung": "bab:test",
            "kontakt": "Bauamt"
        },
        "aktenzeichen": "BA-2025-1",
        "verantwortlich": null,
        "vorgaenger": null,
        "profil": null,
        "kommunikation": [],
        "nachforderungen": [],
        "anforderungen": [],
        "dokumentbeziehungen": [],
        "abgaben": [],
        "bescheide": [],
        "ereignisse": []
    })
}

#[tokio::test]
async fn alte_verfahrensakte_wird_beim_lesen_vervollstaendigt() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = core(dir.path(), &db);
    let u = c.anmelden("admin", "Planungsbüro").await.unwrap();
    let p = c.create_project(projekt("Altbestand"), &u).await.unwrap();
    let id = Uuid::new_v4();
    let alt = alte_akte();
    // Direkt über die Commit-Schnittstelle, wie ihn eine ältere Anwendung abgelegt hat.
    c.repos
        .commit
        .fachstand(
            Fachstand {
                projekt_id: p.id,
                modul: "verfahren".into(),
                objekt: id.to_string(),
                schema_version: 1,
                revision: 0,
                daten: alt.clone(),
            },
            0,
            NeuerProtokolleintrag {
                zeit: Utc::now(),
                benutzer_id: u.id,
                benutzer_name: u.anzeigename.clone(),
                aktion: Aktion::ProjektGeaendert,
                project_id: Some(p.id),
                document_id: None,
                details: "Altbestand".into(),
            },
        )
        .await
        .unwrap();

    let roh = c.repos.commit.fachstaende(p.id, "verfahren").await.unwrap();
    assert!(
        roh[0].daten.get("schritte").is_none(),
        "Ablage bleibt unverändert"
    );

    let gelesen = c.fachobjekte(p.id, ObjektArt::Verfahren).await.unwrap();
    assert_eq!(gelesen.len(), 1);
    let f = &gelesen[0];
    assert_eq!(f.revision, roh[0].revision, "Lesen ändert keine Revision");
    assert_eq!(f.daten["schritte"], json!([]));
    assert_eq!(
        f.daten["antragsangaben"],
        json!({ "vorbescheidfragen": [], "als_bauantrag_weiterfuehren": false })
    );
    // Gleiche Form wie eine Speicherung über den typisierten Vertrag: Original
    // und Eingabe einer Oberfläche stimmen ohne Bearbeitung überein.
    let typisiert: Verfahrensakte = serde_json::from_value(alt).unwrap();
    assert_eq!(f.daten, serde_json::to_value(&typisiert).unwrap());
    // Ein zweites Lesen liefert dieselbe Form (keine schleichende Änderung).
    let erneut = c.fachobjekte(p.id, ObjektArt::Verfahren).await.unwrap();
    assert_eq!(erneut[0].daten, f.daten);
    // Die Ablage selbst bleibt der Altbestand, bis jemand speichert.
    let roh = c.repos.commit.fachstaende(p.id, "verfahren").await.unwrap();
    assert!(roh[0].daten.get("antragsangaben").is_none());
}

#[tokio::test]
async fn unlesbarer_fachstand_bleibt_unveraendert() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = core(dir.path(), &db);
    let u = c.anmelden("admin", "Planungsbüro").await.unwrap();
    let p = c
        .create_project(projekt("Neuere Fassung"), &u)
        .await
        .unwrap();
    let mut neuer = alte_akte();
    neuer["feld_einer_neueren_anwendung"] = json!(true);
    let audit = || NeuerProtokolleintrag {
        zeit: Utc::now(),
        benutzer_id: u.id,
        benutzer_name: u.anzeigename.clone(),
        aktion: Aktion::ProjektGeaendert,
        project_id: Some(p.id),
        document_id: None,
        details: "Test".into(),
    };
    for (schema_version, daten) in [(1, neuer.clone()), (2, alte_akte())] {
        c.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p.id,
                    modul: "verfahren".into(),
                    objekt: Uuid::new_v4().to_string(),
                    schema_version,
                    revision: 0,
                    daten,
                },
                0,
                audit(),
            )
            .await
            .unwrap();
    }
    let gelesen = c.fachobjekte(p.id, ObjektArt::Verfahren).await.unwrap();
    assert_eq!(gelesen.len(), 2);
    for f in gelesen {
        // Weder Standardwerte ergänzen noch unbekannte Felder verwerfen: Die
        // Speichergrenze meldet diese Datensätze ausdrücklich.
        assert!(f.daten.get("schritte").is_none());
        if f.schema_version == 1 {
            assert_eq!(f.daten, neuer);
        }
    }
}

#[tokio::test]
async fn eigenes_projektrecht_folgt_der_mitgliedschaft() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = core(dir.path(), &db);
    let admin = c.anmelden("admin", "Planungsbüro").await.unwrap();
    let planer = c.anmelden("planer", "Planerin").await.unwrap();
    let p = c
        .create_project(projekt("Mitgliedschaft"), &admin)
        .await
        .unwrap();
    assert_eq!(
        c.eigenes_projektrecht(p.id, &admin).await.unwrap(),
        ProjektRecht::Leitung
    );
    // Ohne Mitgliedschaft wie beim Öffnen abgewiesen.
    assert!(matches!(
        c.eigenes_projektrecht(p.id, &planer).await,
        Err(CoreError::Forbidden(_))
    ));
    let mut revision = 0;
    for recht in [
        ProjektRecht::Lesen,
        ProjektRecht::Bearbeiten,
        ProjektRecht::Leitung,
    ] {
        revision = c
            .projektmitglied_setzen(
                p.id,
                ProjektMitglied {
                    benutzer_id: planer.id,
                    recht,
                },
                revision,
                &admin,
            )
            .await
            .unwrap()
            .revision;
        assert_eq!(c.eigenes_projektrecht(p.id, &planer).await.unwrap(), recht);
        // Dieselbe Regel wie an der Servicegrenze.
        assert_eq!(
            c.projekt_zugriff(p.id, &planer, ProjektRecht::Bearbeiten)
                .await
                .is_ok(),
            recht >= ProjektRecht::Bearbeiten
        );
    }
    c.projektmitglied_setzen(
        p.id,
        ProjektMitglied {
            benutzer_id: planer.id,
            recht: ProjektRecht::Kein,
        },
        revision,
        &admin,
    )
    .await
    .unwrap();
    assert!(matches!(
        c.eigenes_projektrecht(p.id, &planer).await,
        Err(CoreError::Forbidden(_))
    ));
}
