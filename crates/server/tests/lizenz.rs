//! Ende-zu-Ende: Testversion, Ablauf, Lizenz einspielen und Arbeitsplätze
//! mit `DatenbankLizenz` auf SQLite.

use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use core_lib::benutzer::{Benutzer, BenutzerAenderung, NeuerBenutzer, Rolle};
use core_lib::domain::{Bundesland, NewProject};
use core_lib::lbo::LboEngine;
use core_lib::license::datei::{ausstellen, testschluessel};
use core_lib::license::{DatenbankLizenz, Edition, Lizenz, Lizenzpruefer, Lizenzzustand};
use core_lib::{CoreError, CoreServices};

fn pruefer() -> Lizenzpruefer {
    Lizenzpruefer::new(vec![testschluessel::oeffentlich()])
}

fn core(
    db: &db_sqlite::SqliteDatabase,
    dir: &std::path::Path,
    arbeitsplatzpflicht: bool,
) -> CoreServices {
    let repos = db.repositories();
    let mut lizenz = DatenbankLizenz::new(repos.lizenz.clone(), pruefer());
    if arbeitsplatzpflicht {
        lizenz = lizenz.mit_arbeitsplatzpflicht();
    }
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.to_path_buf(),
    })
    .unwrap();
    CoreServices::new(repos, storage, Arc::new(LboEngine::new()), Arc::new(lizenz))
}

fn lizenzdatei(arbeitsplaetze: u32, gueltig_bis: Option<NaiveDate>) -> String {
    ausstellen(
        &Lizenz {
            lizenz_id: uuid::Uuid::new_v4(),
            lizenznehmer: "Architekturbüro Muster".into(),
            edition: Edition::Buero,
            arbeitsplaetze,
            ausgestellt_am: Utc::now().date_naive(),
            gueltig_bis,
            wartung_bis: None,
        },
        &testschluessel::geheim(),
    )
}

async fn projekt_anlegen(core: &CoreServices, wer: &Benutzer) -> Result<(), CoreError> {
    core.create_project(
        NewProject {
            name: "Test".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        },
        wer,
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn testversion_ablauf_und_lizenz() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = core(&db, dir.path(), false);
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();

    // Neue Installation: 30 Tage Testversion, alles erlaubt.
    let s = core.lizenz().await.unwrap();
    assert_eq!(s.zustand, Lizenzzustand::Testversion { tage_uebrig: 30 });
    assert!(s.schreiben_erlaubt);
    projekt_anlegen(&core, &chefin).await.unwrap();

    // 40 Tage später: nur noch lesen.
    let vor_40_tagen = Utc::now().date_naive() - Duration::days(40);
    db.repositories()
        .lizenz
        .einstellung_setzen("testbeginn", &vor_40_tagen.to_string())
        .await
        .unwrap();
    let s = core.lizenz().await.unwrap();
    assert_eq!(s.zustand, Lizenzzustand::TestversionAbgelaufen);
    let err = projekt_anlegen(&core, &chefin).await.unwrap_err();
    assert!(
        matches!(&err, CoreError::License(m) if m.contains("Testversion ist abgelaufen")),
        "{err:?}"
    );
    // Lesen geht weiter.
    assert_eq!(core.list_projects().await.unwrap().len(), 1);

    // Nur Administratoren spielen Lizenzen ein.
    let azubi = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "azubi".into(),
                anzeigename: "Azubi".into(),
                rolle: Rolle::Projektleitung,
            },
            &chefin,
        )
        .await
        .unwrap();
    let datei = lizenzdatei(5, None);
    assert!(matches!(
        core.lizenz_einspielen(datei.as_bytes(), &azubi).await,
        Err(CoreError::Forbidden(_))
    ));
    // Eine manipulierte oder fremde Datei wird abgelehnt.
    let fremd = ausstellen(
        &pruefer_lizenz(),
        &ed25519_dalek::SigningKey::from_bytes(&[9; 32]),
    );
    assert!(matches!(
        core.lizenz_einspielen(fremd.as_bytes(), &chefin).await,
        Err(CoreError::License(_))
    ));
    // Eine abgelaufene Lizenz auch.
    let alt = lizenzdatei(5, Some(vor_40_tagen));
    assert!(matches!(
        core.lizenz_einspielen(alt.as_bytes(), &chefin).await,
        Err(CoreError::License(_))
    ));

    let s = core
        .lizenz_einspielen(datei.as_bytes(), &chefin)
        .await
        .unwrap();
    assert_eq!(s.zustand, Lizenzzustand::Lizenziert);
    assert_eq!(s.arbeitsplaetze_max, 5);
    assert!(
        s.meldung.contains("Architekturbüro Muster"),
        "{}",
        s.meldung
    );
    projekt_anlegen(&core, &chefin).await.unwrap();
}

fn pruefer_lizenz() -> Lizenz {
    Lizenz {
        lizenz_id: uuid::Uuid::new_v4(),
        lizenznehmer: "Raubkopie".into(),
        edition: Edition::Buero,
        arbeitsplaetze: 100,
        ausgestellt_am: Utc::now().date_naive(),
        gueltig_bis: None,
        wartung_bis: None,
    }
}

#[tokio::test]
async fn arbeitsplaetze_werden_gezaehlt() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    // Zwei Rechner an derselben Datenbank, Lizenz für einen Platz.
    let pc1 = core(&db, dir.path(), true);
    let pc2 = core(&db, dir.path(), true);
    let chefin = pc1.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ben = pc2.anmelden("ben", "Ben").await.unwrap();
    pc1.lizenz_einspielen(lizenzdatei(1, None).as_bytes(), &chefin)
        .await
        .unwrap();

    // Ohne belegten Platz keine Änderungen.
    assert_eq!(
        pc1.lizenz().await.unwrap().zustand,
        Lizenzzustand::KeinArbeitsplatz
    );
    let s1 = pc1
        .license
        .arbeitsplatz_belegen("PC-CHEFIN", "Clara Chefin")
        .await
        .unwrap();
    assert!(s1.schreiben_erlaubt);
    assert_eq!(s1.eigener_arbeitsplatz.as_deref(), Some("PC-CHEFIN"));
    projekt_anlegen(&pc1, &chefin).await.unwrap();

    let s2 = pc2
        .license
        .arbeitsplatz_belegen("PC-BEN", "Ben")
        .await
        .unwrap();
    assert_eq!(s2.zustand, Lizenzzustand::KeinArbeitsplatz);
    assert!(
        s2.meldung.contains("PC-CHEFIN (Clara Chefin)"),
        "{}",
        s2.meldung
    );
    let ben_admin = pc1
        .benutzer_aendern(
            BenutzerAenderung {
                id: ben.id,
                anzeigename: ben.anzeigename.clone(),
                rolle: Rolle::Administrator,
                aktiv: true,
            },
            &chefin,
        )
        .await
        .unwrap();
    assert!(matches!(
        projekt_anlegen(&pc2, &ben_admin).await,
        Err(CoreError::License(_))
    ));

    // Die Chefin beendet die App: Ben bekommt den Platz beim nächsten Versuch.
    pc1.license
        .arbeitsplatz_freigeben("PC-CHEFIN")
        .await
        .unwrap();
    let s2 = pc2
        .license
        .arbeitsplatz_belegen("PC-BEN", "Ben")
        .await
        .unwrap();
    assert!(s2.schreiben_erlaubt);
    assert!(!pc1.lizenz().await.unwrap().schreiben_erlaubt);
}
