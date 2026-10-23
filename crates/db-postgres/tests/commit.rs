//! Same transaction contract as SQLite, against a real PostgreSQL test database.
use chrono::Utc;
use core_lib::{
    CoreError,
    benutzer::BenutzerId,
    commit::{DokumentCommit, DokumentZusatz, Fachstand},
    domain::{Aktion, Bundesland, DocumentId, DocumentVersion, NeuerProtokolleintrag, NewProject},
    storage::ContentHash,
};
#[tokio::test]
async fn fachrevisionen_und_dokumentkontexte_sind_atomare_vertraege() {
    let Ok(url) = std::env::var("BAUPROJEKT_TEST_POSTGRES_URL")
        .or_else(|_| std::env::var("BAUPROJEKT_TEST_POSTGRES"))
    else {
        assert!(
            std::env::var_os("CI").is_none(),
            "PostgreSQL-Konfiguration fehlt"
        );
        return;
    };
    let db = db_postgres::PostgresDatabase::connect(&url, 4)
        .await
        .unwrap();
    let repos = db.repositories();
    let p = repos
        .projects
        .create(NewProject {
            name: "Commitvertrag".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        })
        .await
        .unwrap();
    let audit = || NeuerProtokolleintrag {
        zeit: Utc::now(),
        benutzer_id: BenutzerId::new(),
        benutzer_name: "Test".into(),
        aktion: Aktion::ProjektGeaendert,
        project_id: Some(p.id),
        document_id: None,
        details: "Vertragstest".into(),
    };
    let f = Fachstand {
        projekt_id: p.id,
        modul: "gebaeude".into(),
        objekt: uuid::Uuid::new_v4().to_string(),
        schema_version: 1,
        revision: 0,
        daten: serde_json::json!({"bezeichnung":"Haus A"}),
    };
    repos.commit.fachstand(f.clone(), 0, audit()).await.unwrap();
    let (a, b) = tokio::join!(
        repos.commit.fachstand(f.clone(), 1, audit()),
        repos.commit.fachstand(f.clone(), 1, audit())
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(
        a.err().or(b.err()).unwrap(),
        CoreError::VersionConflict { .. }
    ));
    assert_eq!(
        repos
            .commit
            .fachverlauf(p.id, "gebaeude", &f.objekt)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(repos.projects.get(p.id).await.unwrap().version, 1);
    let d = DocumentVersion {
        document_id: DocumentId::new(),
        project_id: p.id,
        version: 0,
        file_name: "Formular.pdf".into(),
        storage_key: "test/form.pdf".into(),
        sha256: ContentHash::of(b"pdf"),
        size_bytes: 3,
        created_by: "Test".into(),
        created_at: Utc::now(),
    };
    let zusatz = DokumentZusatz {
        modul: "formularinstanz".into(),
        schema_version: 1,
        daten: serde_json::json!({"quelle":"Original"}),
    };
    let auftrag = || DokumentCommit {
        dokument: d.clone(),
        basis: None,
        neuanlage: true,
        zusaetze: vec![zusatz.clone()],
    };
    let (a, b) = tokio::join!(
        repos.commit.dokument(auftrag(), audit()),
        repos.commit.dokument(auftrag(), audit())
    );
    assert_eq!(a.unwrap(), b.unwrap());
    assert_eq!(
        repos.documents.history(d.document_id).await.unwrap().len(),
        1
    );
    assert_eq!(
        repos
            .commit
            .dokumentzusatz(d.document_id, 1, "formularinstanz")
            .await
            .unwrap()
            .unwrap()
            .daten["quelle"],
        "Original"
    );
    // Duplicate metadata keys fail the transaction, including the inserted PDF version.
    let mut kaputt = auftrag();
    kaputt.dokument.document_id = DocumentId::new();
    let id = kaputt.dokument.document_id;
    kaputt.zusaetze.push(zusatz);
    assert!(repos.commit.dokument(kaputt, audit()).await.is_err());
    assert!(repos.documents.history(id).await.unwrap().is_empty());
    // Dieselben begrenzten Leseverträge wie SQLite, tatsächlich gegen PostgreSQL.
    let seite = repos
        .commit
        .fachseite(p.id, "gebaeude", None, 1)
        .await
        .unwrap();
    assert_eq!(seite.len(), 1);
    assert!(
        repos
            .commit
            .fachseite(p.id, "gebaeude", Some(&seite[0].objekt), 1)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        repos
            .commit
            .fachobjekt(p.id, "gebaeude", &f.objekt)
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        repos
            .commit
            .fachverlaufsseite(p.id, "gebaeude", &f.objekt, Some(2), 1)
            .await
            .unwrap()[0]
            .revision,
        1
    );
    assert_eq!(
        repos.commit.fachrevisionen(p.id, "gebaeude").await.unwrap(),
        vec![(f.objekt.clone(), 2)]
    );
    let index = Fachstand {
        projekt_id: p.id,
        modul: "suchtexte".into(),
        objekt: "suche".into(),
        schema_version: 1,
        revision: 0,
        daten: serde_json::json!({"_suchtext_norm":"änderung straße grundstück"}),
    };
    repos.commit.fachstand(index, 0, audit()).await.unwrap();
    assert_eq!(
        repos
            .commit
            .suchtextseite(p.id, None, "straße", 16)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        repos
            .commit
            .suchtextseite(p.id, None, "fremd", 16)
            .await
            .unwrap()
            .is_empty()
    );
    repos
        .documents
        .add_version(d.clone(), Some(1))
        .await
        .unwrap();
    assert_eq!(
        repos.documents.latest_for_project(p.id).await.unwrap()[0].version,
        2
    );
    assert_eq!(
        repos
            .documents
            .history_page(d.document_id, Some(2), 1)
            .await
            .unwrap()[0]
            .version,
        1
    );
}
