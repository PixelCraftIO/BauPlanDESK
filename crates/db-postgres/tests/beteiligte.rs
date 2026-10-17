//! Beteiligte in Rollen: gleicher Vertrag wie SQLite, gegen PostgreSQL.
use core_lib::{
    CoreError,
    domain::{Bundesland, NewProject},
    kunden::{Beteiligtenrolle, Beteiligtenzuordnung, Kundendaten},
};
#[tokio::test]
async fn beteiligte_rollen_reihenfolge_und_fremdschluessel() {
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
            name: "Beteiligte".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        })
        .await
        .unwrap();
    let kunde = |name: &str| Kundendaten {
        familienname: name.into(),
        ..Kundendaten::default()
    };
    let eigentuemer = repos.kunden.kunde_anlegen(kunde("Eigner")).await.unwrap();
    let planer = repos.kunden.kunde_anlegen(kunde("Planer")).await.unwrap();
    let z = |rolle, kunde| Beteiligtenzuordnung { rolle, kunde };
    let p2 = repos
        .kunden
        .beteiligte_setzen(
            p.id,
            &[
                z(Beteiligtenrolle::Fachplaner, planer.id),
                z(Beteiligtenrolle::Grundstueckseigentuemer, eigentuemer.id),
            ],
            p.version,
        )
        .await
        .unwrap();
    assert_eq!(p2.version, p.version + 1);
    let b = repos.kunden.beteiligte(p.id).await.unwrap();
    assert_eq!(
        b.iter().map(|b| (b.rolle, b.kunde.id)).collect::<Vec<_>>(),
        [
            (Beteiligtenrolle::Grundstueckseigentuemer, eigentuemer.id),
            (Beteiligtenrolle::Fachplaner, planer.id),
        ]
    );
    assert!(matches!(
        repos
            .kunden
            .beteiligte_setzen(p.id, &[], p.version)
            .await
            .unwrap_err(),
        CoreError::VersionConflict { .. }
    ));
    assert_eq!(
        repos.kunden.projekte_des_kunden(planer.id).await.unwrap()[0].id,
        p.id
    );
    assert!(matches!(
        repos
            .kunden
            .kunde_loeschen(planer.id, planer.version)
            .await
            .unwrap_err(),
        CoreError::InvalidInput(_)
    ));
    repos
        .kunden
        .beteiligte_setzen(p.id, &[], p2.version)
        .await
        .unwrap();
    repos
        .kunden
        .kunde_loeschen(planer.id, planer.version)
        .await
        .unwrap();
}
