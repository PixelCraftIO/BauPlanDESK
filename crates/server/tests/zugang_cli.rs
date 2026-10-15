//! Provisioning uses the production binary and never writes the raw token into the access list.
use std::{
    path::Path,
    process::{Command, Output},
};
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bauprojekt-server"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env(
            "BAUPROJEKT_DATABASE_URL",
            format!("sqlite://{}", root.join("server.db").display()),
        )
        .env(
            "BAUPROJEKT_STORAGE",
            serde_json::json!({"type":"local","root":root.join("dateien")}).to_string(),
        )
        .env("BAUPROJEKT_ZUGANGSDATEI", root.join("zugaenge.json"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn zugang_wird_sicher_bereitgestellt_rotiert_und_system_bleibt_intern() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let token = p.join("admin.token");
    let ziel = token.to_str().unwrap();
    assert!(
        !cli(p, &["zugang-anlegen", "erster", "lesen", ziel])
            .status
            .success()
    );
    assert!(!token.exists());
    assert!(
        !cli(p, &["zugang-anlegen", " system ", "administrator", ziel])
            .status
            .success()
    );
    assert!(
        !cli(
            p,
            &["zugang-anlegen", "domain\\anna", "administrator", ziel]
        )
        .status
        .success()
    );
    assert!(
        cli(p, &["zugang-anlegen", "admin", "administrator", ziel])
            .status
            .success()
    );
    let roh = std::fs::read(&token).unwrap();
    assert!(roh.len() >= 32);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&token).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let liste = std::fs::read(p.join("zugaenge.json")).unwrap();
    assert!(!liste.windows(roh.len()).any(|x| x == roh));
    let z: server::api::zugang::Zugaenge = serde_json::from_slice(&liste).unwrap();
    assert_eq!(z.zugaenge.len(), 1);
    assert_eq!(
        z.zugaenge[0].token_sha256,
        core_lib::storage::ContentHash::of(&roh)
    );
    let neu = p.join("admin-neu.token");
    let neu = neu.to_str().unwrap();
    assert!(
        !cli(p, &["zugang-anlegen", "admin", "lesen", neu])
            .status
            .success()
    );
    assert_eq!(std::fs::read(p.join("zugaenge.json")).unwrap(), liste);
    assert!(
        cli(p, &["zugang-anlegen", "admin", "administrator", neu])
            .status
            .success()
    );
    let danach: server::api::zugang::Zugaenge =
        serde_json::from_slice(&std::fs::read(p.join("zugaenge.json")).unwrap()).unwrap();
    assert_eq!(danach.zugaenge.len(), 1);
    assert_ne!(danach.zugaenge[0].token_sha256, z.zugaenge[0].token_sha256);
    assert!(
        !cli(p, &["zugang-anlegen", "admin", "administrator", neu])
            .status
            .success()
    );
    let mut doppelt = danach.clone();
    doppelt.zugaenge.push(danach.zugaenge[0].clone());
    assert!(doppelt.pruefen().is_err());
}
