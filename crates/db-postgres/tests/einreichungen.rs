//! Opt-in against a dedicated test database: BAUPROJEKT_TEST_POSTGRES_URL.
use chrono::Utc;
use core_lib::{
    CoreError,
    domain::{Bundesland, NewProject},
    einreichung::{Einreichung, EinreichungsBestaetigung, PaketStand},
    storage::ContentHash,
};
use uuid::Uuid;
#[tokio::test]
async fn paket_persistenz_und_atomare_bestaetigung() {
    let Ok(url) = std::env::var("BAUPROJEKT_TEST_POSTGRES_URL")
        .or_else(|_| std::env::var("BAUPROJEKT_TEST_POSTGRES"))
    else {
        assert!(
            std::env::var_os("CI").is_none(),
            "PostgreSQL-Testkonfiguration fehlt in CI"
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
            name: "Chronik PostgreSQL-Test".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        })
        .await
        .unwrap();
    let e = Einreichung {
        stand: PaketStand {
            id: Uuid::new_v4(),
            projekt_id: p.id,
            titel: "Bauantrag".into(),
            empfaenger: "Bauamt".into(),
            erstellt_am: Utc::now(),
            erstellt_von: "Test".into(),
            dokumente: vec![],
        },
        zip_key: "test/paket.zip".into(),
        zip_hash: ContentHash::of(b"zip"),
        size: 3,
        version: 1,
        einreichung: None,
    };
    repos.einreichungen.anlegen(&e).await.unwrap();
    assert_eq!(repos.einreichungen.auflisten(p.id).await.unwrap().len(), 1);
    let b = EinreichungsBestaetigung {
        datum: Utc::now().date_naive(),
        notiz: "Bauportal".into(),
        bestaetigt_am: Utc::now(),
        bestaetigt_von: "Test".into(),
    };
    let (a, c) = tokio::join!(
        repos.einreichungen.bestaetigen(e.stand.id, 1, b.clone()),
        repos.einreichungen.bestaetigen(e.stand.id, 1, b)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(c.is_ok()), 1);
    assert!(matches!(
        a.err().or(c.err()).unwrap(),
        CoreError::VersionConflict { .. }
    ));
    let actual = repos.einreichungen.laden(e.stand.id).await.unwrap();
    assert_eq!(actual.version, 2);
    assert!(actual.einreichung.is_some());
    assert_eq!(actual.zip_hash, e.zip_hash);
    repos.projects.delete(p.id, p.version).await.unwrap();
}
