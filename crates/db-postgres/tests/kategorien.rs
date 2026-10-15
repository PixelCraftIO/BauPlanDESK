//! Prüft den gemeinsamen atomaren Einstellungsvertrag auch mit PostgreSQL.
#[tokio::test]
async fn katalog_cas_verhindert_verlorene_aenderungen() {
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
    let key = format!("katalog-test/{}", uuid::Uuid::new_v4());
    let (a, b) = tokio::join!(
        repos.lizenz.einstellung_vergleichen(&key, None, "Stand A"),
        repos.lizenz.einstellung_vergleichen(&key, None, "Stand B")
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let alt = repos.lizenz.einstellung(&key).await.unwrap().unwrap();
    let (a, b) = tokio::join!(
        repos
            .lizenz
            .einstellung_vergleichen(&key, Some(&alt), "Stand C"),
        repos
            .lizenz
            .einstellung_vergleichen(&key, Some(&alt), "Stand D")
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    assert!(
        !repos
            .lizenz
            .einstellung_vergleichen(&key, Some(&alt), "veraltet")
            .await
            .unwrap()
    );
    assert_ne!(
        repos.lizenz.einstellung(&key).await.unwrap().unwrap(),
        "veraltet"
    );
}
