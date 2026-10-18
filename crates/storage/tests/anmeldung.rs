//! Kennwortanmeldung mit echter SQLite-Datenbank: Übergang für Bestandsbüros,
//! Erstkennwort, Fehlversuche mit Sperre und Protokoll.
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_lib::benutzer::{Benutzer, BenutzerAenderung, Kennwortstatus, NeuerBenutzer, Rolle};
use core_lib::domain::{
    Aktion, Bundesland, NeuerProtokolleintrag, NewProject, ProjectId, Protokolleintrag,
};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::repository::ProtokollRepository;
use core_lib::{CoreError, CoreResult, CoreServices};

/// Merkt sich alle Protokolleinträge, auch solche ohne Projekt.
struct Mitschrift {
    innen: Arc<dyn ProtokollRepository>,
    eintraege: Mutex<Vec<NeuerProtokolleintrag>>,
}

#[async_trait]
impl ProtokollRepository for Mitschrift {
    async fn append(&self, eintrag: NeuerProtokolleintrag) -> CoreResult<()> {
        self.eintraege.lock().unwrap().push(eintrag.clone());
        self.innen.append(eintrag).await
    }

    async fn list_for_project(
        &self,
        project: ProjectId,
        limit: i64,
    ) -> CoreResult<Vec<Protokolleintrag>> {
        self.innen.list_for_project(project, limit).await
    }
}

impl Mitschrift {
    fn aktionen(&self, aktion: Aktion) -> Vec<String> {
        self.eintraege
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.aktion == aktion)
            .map(|e| e.details.clone())
            .collect()
    }
}

async fn umgebung(dir: &std::path::Path) -> (CoreServices, Arc<Mitschrift>) {
    let db = db_sqlite::SqliteDatabase::open(&dir.join("bauprojekt.db"))
        .await
        .unwrap();
    let mut repos = db.repositories();
    let mitschrift = Arc::new(Mitschrift {
        innen: repos.protokoll.clone(),
        eintraege: Mutex::new(Vec::new()),
    });
    repos.protokoll = mitschrift.clone();
    let core = CoreServices::new(
        repos,
        Arc::new(storage::LocalStorage::new(dir.join("dms"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    (core, mitschrift)
}

fn projekt(name: &str) -> NewProject {
    NewProject {
        name: name.into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: Some(500.0),
        planungsrecht: Default::default(),
    }
}

fn verboten(e: CoreResult<impl std::fmt::Debug>) -> String {
    match e {
        Err(CoreError::Forbidden(m)) => m,
        other => panic!("Forbidden erwartet, erhalten: {other:?}"),
    }
}

/// Bestandsbüro: Benutzer aus der bisherigen Betriebssystem-Anmeldung.
async fn bestand(core: &CoreServices) -> (Benutzer, Benutzer) {
    let chefin = core
        .anmelden("BUERO\\Chefin", "Clara Chefin")
        .await
        .unwrap();
    let ben = core.anmelden("ben", "Ben Bauer").await.unwrap();
    let ben = core
        .benutzer_aendern(
            BenutzerAenderung {
                id: ben.id,
                anzeigename: ben.anzeigename,
                rolle: Rolle::Bearbeitung,
                aktiv: true,
            },
            &chefin,
        )
        .await
        .unwrap();
    (chefin, ben)
}

#[tokio::test]
async fn bestandsbuero_admin_meldet_sich_einmalig_ueber_rechnerkonto_an() {
    let dir = tempfile::tempdir().unwrap();
    let (core, protokoll) = umgebung(dir.path()).await;
    let (chefin, ben) = bestand(&core).await;
    let core = core.mit_kennwortpflicht();
    assert_eq!(chefin.kennwort, Kennwortstatus::Keines);

    // Nur der Administrator darf den Übergang nutzen.
    let o = core.anmeldeoptionen("buero\\CHEFIN").await.unwrap();
    assert!(o.uebergang_moeglich && !o.erster_administrator);
    assert_eq!(o.rechnerkonto, "chefin");
    assert!(
        !core
            .anmeldeoptionen("ben")
            .await
            .unwrap()
            .uebergang_moeglich
    );
    let m = verboten(core.anmelden_uebergang("ben", "Ben").await);
    assert!(m.contains("Erstkennwort"), "{m}");
    // Ohne Kennwort auch keine Kennwortanmeldung.
    let m = verboten(core.anmelden_mit_kennwort("ben", "irgendwas-langes").await);
    assert!(m.contains("kein Kennwort"), "{m}");

    let a = core.anmelden_uebergang("chefin", "").await.unwrap();
    assert!(a.kennwort_aendern);
    assert_eq!(a.benutzer.id, chefin.id);
    assert!(a.benutzer.letzte_anmeldung.is_some());
    // Mit Kennwortpflicht keine Arbeit vor dem eigenen Kennwort.
    let m = verboten(core.create_project(projekt("Vorab"), &a.benutzer).await);
    assert!(m.contains("eigenes Kennwort"), "{m}");

    let zu_kurz = core
        .kennwort_aendern(&a.benutzer, None, "kurz")
        .await
        .unwrap_err();
    assert!(matches!(zu_kurz, CoreError::InvalidInput(_)), "{zu_kurz:?}");
    let chefin = core
        .kennwort_aendern(&a.benutzer, None, "Baugrube-Lindenweg")
        .await
        .unwrap();
    assert_eq!(chefin.kennwort, Kennwortstatus::Gesetzt);
    core.create_project(projekt("Nach Kennwort"), &chefin)
        .await
        .unwrap();

    // Übergang ist danach geschlossen, auch für andere Administratoren.
    assert!(
        !core
            .anmeldeoptionen("chefin")
            .await
            .unwrap()
            .uebergang_moeglich
    );
    verboten(core.anmelden_uebergang("chefin", "").await);
    // Ben bleibt ohne Kennwort ausgesperrt, bis er ein Erstkennwort bekommt.
    verboten(core.create_project(projekt("Ben"), &ben).await);

    let a = core
        .anmelden_mit_kennwort("CHEFIN", "Baugrube-Lindenweg")
        .await
        .unwrap();
    assert!(!a.kennwort_aendern);
    assert_eq!(protokoll.aktionen(Aktion::Angemeldet).len(), 2);
    assert_eq!(protokoll.aktionen(Aktion::KennwortGeaendert).len(), 1);
}

#[tokio::test]
async fn erstkennwort_muss_bei_der_ersten_anmeldung_geaendert_werden() {
    let dir = tempfile::tempdir().unwrap();
    let (core, protokoll) = umgebung(dir.path()).await;
    let (_, ben) = bestand(&core).await;
    let core = core.mit_kennwortpflicht();
    let chefin = {
        let a = core.anmelden_uebergang("chefin", "").await.unwrap();
        core.kennwort_aendern(&a.benutzer, None, "Chefin-Kennwort-1")
            .await
            .unwrap()
    };

    // Nur Administratoren vergeben Erstkennwörter.
    let ben_alt = ben.clone();
    verboten(
        core.erstkennwort_vergeben(chefin.id, "Uebernahme-12345", &ben_alt)
            .await,
    );
    let ben = core
        .erstkennwort_vergeben(ben.id, "Erstkennwort-Ben", &chefin)
        .await
        .unwrap();
    assert_eq!(ben.kennwort, Kennwortstatus::Erstkennwort);
    assert!(
        protokoll
            .aktionen(Aktion::KennwortGeaendert)
            .iter()
            .any(|d| d.contains("Erstkennwort für Ben Bauer"))
    );

    // Anmeldename ohne Rücksicht auf Groß-/Kleinschreibung.
    let a = core
        .anmelden_mit_kennwort("Ben", "Erstkennwort-Ben")
        .await
        .unwrap();
    assert!(a.kennwort_aendern);
    verboten(core.create_project(projekt("Ben"), &a.benutzer).await);

    // Das Erstkennwort ist zur Änderung erneut anzugeben.
    let m = verboten(
        core.kennwort_aendern(&a.benutzer, Some("falsch"), "Ben-eigenes-Kennwort")
            .await,
    );
    assert!(m.contains("bisherige"), "{m}");
    verboten(
        core.kennwort_aendern(&a.benutzer, None, "Ben-eigenes-Kennwort")
            .await,
    );
    let ben = core
        .kennwort_aendern(
            &a.benutzer,
            Some("Erstkennwort-Ben"),
            "Ben-eigenes-Kennwort",
        )
        .await
        .unwrap();
    assert_eq!(ben.kennwort, Kennwortstatus::Gesetzt);

    verboten(core.anmelden_mit_kennwort("ben", "Erstkennwort-Ben").await);
    let a = core
        .anmelden_mit_kennwort("ben", "Ben-eigenes-Kennwort")
        .await
        .unwrap();
    assert!(!a.kennwort_aendern);
    // Bearbeitung darf keine Projekte anlegen, aber Kennwortpflicht ist erfüllt.
    let m = verboten(core.create_project(projekt("Ben"), &a.benutzer).await);
    assert!(m.contains("Projektleitung"), "{m}");

    // Eigenes Kennwort ändern verlangt das bisherige.
    verboten(
        core.kennwort_aendern(&a.benutzer, None, "Noch-ein-Kennwort")
            .await,
    );
    core.kennwort_aendern(
        &a.benutzer,
        Some("Ben-eigenes-Kennwort"),
        "Noch-ein-Kennwort",
    )
    .await
    .unwrap();

    // Systemkonto bekommt kein Kennwort.
    let system = core.systembenutzer().await.unwrap();
    assert!(matches!(
        core.erstkennwort_vergeben(system.id, "System-Kennwort", &chefin)
            .await,
        Err(CoreError::InvalidInput(_))
    ));
    verboten(
        core.anmelden_mit_kennwort("system", "System-Kennwort")
            .await,
    );
}

#[tokio::test]
async fn fehlversuche_sperren_vorruebergehend_und_erstkennwort_hebt_auf() {
    let dir = tempfile::tempdir().unwrap();
    let (core, protokoll) = umgebung(dir.path()).await;
    let (_, ben) = bestand(&core).await;
    let core = core.mit_kennwortpflicht();
    let chefin = {
        let a = core.anmelden_uebergang("chefin", "").await.unwrap();
        core.kennwort_aendern(&a.benutzer, None, "Chefin-Kennwort-1")
            .await
            .unwrap()
    };
    core.erstkennwort_vergeben(ben.id, "Erstkennwort-Ben", &chefin)
        .await
        .unwrap();

    for n in 1..=4 {
        let m = verboten(core.anmelden_mit_kennwort("ben", "falsch-falsch").await);
        assert_eq!(m, "Anmeldename oder Kennwort ist falsch.", "Versuch {n}");
    }
    let m = verboten(core.anmelden_mit_kennwort("ben", "falsch-falsch").await);
    assert!(m.contains("Nach 5 Fehlversuchen"), "{m}");
    let gesperrt = core.repos.benutzer.get(ben.id).await.unwrap();
    assert!(gesperrt.anmeldesperre_bis.is_some());

    // Auch das richtige Kennwort hilft während der Sperre nicht.
    let m = verboten(core.anmelden_mit_kennwort("ben", "Erstkennwort-Ben").await);
    assert!(m.contains("gesperrt"), "{m}");
    assert_eq!(protokoll.aktionen(Aktion::AnmeldungFehlgeschlagen).len(), 5);
    assert!(
        protokoll
            .aktionen(Aktion::AnmeldungFehlgeschlagen)
            .last()
            .unwrap()
            .contains("5. Fehlversuch")
    );

    // Ein neues Erstkennwort hebt Sperre und Zähler auf.
    core.erstkennwort_vergeben(ben.id, "Neues-Erstkennwort", &chefin)
        .await
        .unwrap();
    let a = core
        .anmelden_mit_kennwort("ben", "Neues-Erstkennwort")
        .await
        .unwrap();
    assert!(a.kennwort_aendern);
    assert_eq!(a.benutzer.anmeldesperre_bis, None);
    let d = core.repos.benutzer.kennwortdaten(ben.id).await.unwrap();
    assert_eq!(d.fehlversuche, 0);

    // Unbekannte Namen: gleiche Meldung, Eintrag unter dem Systemkonto.
    let m = verboten(core.anmelden_mit_kennwort("niemand", "irgendwas-1").await);
    assert_eq!(m, "Anmeldename oder Kennwort ist falsch.");
    assert!(
        protokoll
            .aktionen(Aktion::AnmeldungFehlgeschlagen)
            .iter()
            .any(|d| d.contains("niemand"))
    );
}

#[tokio::test]
async fn deaktivierte_konten_und_nicht_angemeldet() {
    let dir = tempfile::tempdir().unwrap();
    let (core, _) = umgebung(dir.path()).await;
    let (_, ben) = bestand(&core).await;
    let core = core.mit_kennwortpflicht();
    let chefin = {
        let a = core.anmelden_uebergang("chefin", "").await.unwrap();
        core.kennwort_aendern(&a.benutzer, None, "Chefin-Kennwort-1")
            .await
            .unwrap()
    };
    core.erstkennwort_vergeben(ben.id, "Erstkennwort-Ben", &chefin)
        .await
        .unwrap();
    core.benutzer_aendern(
        BenutzerAenderung {
            id: ben.id,
            anzeigename: ben.anzeigename.clone(),
            rolle: ben.rolle,
            aktiv: false,
        },
        &chefin,
    )
    .await
    .unwrap();
    // Falsches Kennwort verrät nicht, dass das Konto deaktiviert ist.
    assert_eq!(
        verboten(core.anmelden_mit_kennwort("ben", "falsch-falsch").await),
        "Anmeldename oder Kennwort ist falsch."
    );
    let m = verboten(core.anmelden_mit_kennwort("ben", "Erstkennwort-Ben").await);
    assert!(m.contains("deaktiviert"), "{m}");

    let niemand = Benutzer::nicht_angemeldet();
    let m = verboten(core.create_project(projekt("Niemand"), &niemand).await);
    assert!(m.contains("melden Sie sich"), "{m}");
    let p = core
        .create_project(projekt("Sichtbar"), &chefin)
        .await
        .unwrap();
    let m = verboten(
        core.projekt_zugriff(p.id, &niemand, core_lib::zugriff::ProjektRecht::Lesen)
            .await,
    );
    assert!(m.contains("melden Sie sich"), "{m}");
    verboten(
        core.kennwort_aendern(&niemand, None, "Kennwort-12345")
            .await,
    );
}

#[tokio::test]
async fn leere_datenbank_legt_ersten_administrator_ueber_uebergang_an() {
    let dir = tempfile::tempdir().unwrap();
    let (core, _) = umgebung(dir.path()).await;
    let core = core.mit_kennwortpflicht();
    let o = core.anmeldeoptionen("daniel").await.unwrap();
    assert!(o.uebergang_moeglich && o.erster_administrator);
    let a = core
        .anmelden_uebergang("daniel", "Daniel Fiebich")
        .await
        .unwrap();
    assert!(a.kennwort_aendern);
    assert_eq!(a.benutzer.rolle, Rolle::Administrator);
    // Ein zweiter Rechner ist kein Administrator und kommt nicht hinein.
    assert!(
        !core
            .anmeldeoptionen("kollege")
            .await
            .unwrap()
            .uebergang_moeglich
    );
    verboten(core.anmelden_uebergang("kollege", "").await);
    // Vom Administrator angelegte Kollegen bekommen ein Erstkennwort.
    let admin = core
        .kennwort_aendern(&a.benutzer, None, "Daniels-Kennwort")
        .await
        .unwrap();
    let kollege = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "Kollege".into(),
                anzeigename: "Karl Kollege".into(),
                rolle: Rolle::Bearbeitung,
            },
            &admin,
        )
        .await
        .unwrap();
    core.erstkennwort_vergeben(kollege.id, "Karls-Erstkennwort", &admin)
        .await
        .unwrap();
    assert!(
        core.anmelden_mit_kennwort("KOLLEGE", "Karls-Erstkennwort")
            .await
            .unwrap()
            .kennwort_aendern
    );
}

#[tokio::test]
async fn einzelplatz_ohne_kennwortpflicht_arbeitet_wie_bisher() {
    let dir = tempfile::tempdir().unwrap();
    let (core, _) = umgebung(dir.path()).await;
    let u = core.anmelden("planer", "Planer").await.unwrap();
    assert_eq!(u.kennwort, Kennwortstatus::Keines);
    core.create_project(projekt("Einzelplatz"), &u)
        .await
        .unwrap();
}
