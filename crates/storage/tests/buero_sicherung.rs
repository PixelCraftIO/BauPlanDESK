//! Tatsächlicher PostgreSQL-Dump/Restore; ausschließlich kurzlebige eigene Datenbanken.
use std::process::Command;
use storage::buero_sicherung;
fn sql(url: &url::Url, db: &str, query: &str) -> String {
    let o = Command::new("psql")
        .env("PGHOST", url.host_str().unwrap())
        .env("PGPORT", url.port().unwrap_or(5432).to_string())
        .env("PGUSER", url.username())
        .env("PGPASSWORD", url.password().unwrap_or(""))
        .env("PGDATABASE", db)
        .args(["-X", "-A", "-t", "-v", "ON_ERROR_STOP=1", "-c", query])
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "SQL-Testvorbereitung fehlgeschlagen: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap().trim().into()
}
#[test]
fn postgres_und_dateien_in_neues_profil_wiederherstellen() {
    let Ok(base) = std::env::var("BAUPROJEKT_TEST_BUERO_SICHERUNG") else {
        eprintln!("Bürosicherungs-Integration: BAUPROJEKT_TEST_BUERO_SICHERUNG nicht gesetzt");
        return;
    };
    let url = url::Url::parse(&base).unwrap();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let quelle = format!("sicherung_quelle_{suffix}");
    let ziel = format!("sicherung_ziel_{suffix}");
    let admin = url.path().trim_start_matches('/');
    sql(&url, admin, &format!("CREATE DATABASE {quelle}"));
    sql(&url, admin, &format!("CREATE DATABASE {ziel}"));
    let result = std::panic::catch_unwind(|| {
        sql(
            &url,
            &quelle,
            "CREATE TABLE beleg (id integer PRIMARY KEY, text text); INSERT INTO beleg VALUES (1, 'Genehmigter Plan Version 2')",
        );
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Dateien");
        let cfg = dir.path().join("Einstellungen");
        let forms = dir.path().join("Formulare");
        for p in [&root, &cfg, &forms] {
            std::fs::create_dir(p).unwrap();
        }
        std::fs::write(root.join("plan.pdf"), b"Originaler Planstand").unwrap();
        std::fs::write(forms.join("bauantrag.pdf"), b"Originale Vorlage").unwrap();
        let archiv = dir.path().join("Buero.zip");
        let mut source = url.clone();
        source.set_path(&quelle);
        let mut target = url.clone();
        target.set_path(&ziel);
        buero_sicherung::erstellen(source.as_str(), &root, &cfg, Some(&forms), &archiv).unwrap();
        // Nachträgliche Änderungen gehören nicht zum eingefrorenen Backup.
        sql(&url, &quelle, "UPDATE beleg SET text='Spaeterer Entwurf'");
        let restored = dir.path().join("Wiederhergestellt");
        buero_sicherung::wiederherstellen(&archiv, &restored, target.as_str()).unwrap();
        assert_eq!(
            sql(&url, &ziel, "SELECT text FROM beleg"),
            "Genehmigter Plan Version 2"
        );
        assert_eq!(
            std::fs::read(restored.join("dateien/plan.pdf")).unwrap(),
            b"Originaler Planstand"
        );
        assert!(restored.join("Formularvorlagen/bauantrag.pdf").is_file());
        let config: serde_json::Value = serde_json::from_slice(
            &std::fs::read(restored.join("einstellungen/config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(config["database_url"], target.as_str());
        assert!(
            buero_sicherung::wiederherstellen(
                &archiv,
                &dir.path().join("ZweiterVersuch"),
                target.as_str()
            )
            .is_err()
        );
        assert_eq!(sql(&url, &ziel, "SELECT count(*) FROM beleg"), "1");
        assert!(buero_sicherung::wiederherstellen(&archiv, &restored, source.as_str()).is_err());
    });
    sql(&url, admin, &format!("DROP DATABASE {quelle}"));
    sql(&url, admin, &format!("DROP DATABASE {ziel}"));
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}
