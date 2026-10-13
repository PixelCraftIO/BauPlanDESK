//! Abo-Lizenzen: Tickets vom (hier simulierten) Lizenzserver, Bindung an
//! den Rechner, Offline-Zeitraum und Ablehnung, mit `DatenbankLizenz` auf
//! SQLite. Zwei `CoreServices` an derselben Datenbank stehen für zwei
//! Rechner im Büro.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use core_lib::CoreServices;
use core_lib::benutzer::Benutzer;
use core_lib::domain::{Bundesland, NewProject};
use core_lib::lbo::LboEngine;
use core_lib::license::datei::{ausstellen, testschluessel};
use core_lib::license::ticket::{
    AbmeldeAnfrage, DienstFehler, Lizenzdienst, Ticket, TicketAnfrage, ticket_ausstellen,
};
use core_lib::license::{
    AboAnbindung, DatenbankLizenz, Edition, Lizenz, Lizenzpruefer, Lizenzzustand,
};
use uuid::Uuid;

#[derive(Clone)]
enum Verhalten {
    /// Ticket ausstellen, gültig für so viele Tage (negativ: schon abgelaufen).
    Ausstellen(i64),
    Offline,
    Ablehnen(&'static str),
}

/// Simulierter Lizenzserver.
struct Server {
    lizenz_id: Uuid,
    verhalten: Mutex<Verhalten>,
    anfragen: Mutex<Vec<TicketAnfrage>>,
    abmeldungen: Mutex<Vec<String>>,
}

impl Server {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            lizenz_id: Uuid::new_v4(),
            verhalten: Mutex::new(Verhalten::Ausstellen(30)),
            anfragen: Mutex::default(),
            abmeldungen: Mutex::default(),
        })
    }

    fn setze(&self, v: Verhalten) {
        *self.verhalten.lock().unwrap() = v;
    }

    fn anzahl(&self) -> usize {
        self.anfragen.lock().unwrap().len()
    }
}

#[async_trait]
impl Lizenzdienst for Server {
    async fn ticket(&self, anfrage: &TicketAnfrage) -> Result<String, DienstFehler> {
        self.anfragen.lock().unwrap().push(anfrage.clone());
        if anfrage.lizenzschluessel != "BPIO-TEST1-TEST2" {
            return Err(DienstFehler::Abgelehnt("Lizenzschlüssel unbekannt.".into()));
        }
        let verhalten = self.verhalten.lock().unwrap().clone();
        match verhalten {
            Verhalten::Ausstellen(tage) => {
                let jetzt = Utc::now();
                Ok(ticket_ausstellen(
                    &Ticket {
                        lizenz: Lizenz {
                            lizenz_id: self.lizenz_id,
                            lizenznehmer: "Architekturbüro Muster".into(),
                            edition: Edition::Buero,
                            arbeitsplaetze: 5,
                            ausgestellt_am: jetzt.date_naive(),
                            gueltig_bis: None,
                            wartung_bis: None,
                        },
                        machine_id: anfrage.machine_id.clone(),
                        ausgestellt_um: jetzt,
                        gueltig_bis: jetzt + Duration::days(tage),
                    },
                    &testschluessel::ticket_geheim(),
                ))
            }
            Verhalten::Offline => Err(DienstFehler::NichtErreichbar("keine Verbindung".into())),
            Verhalten::Ablehnen(m) => Err(DienstFehler::Abgelehnt(m.into())),
        }
    }

    async fn abmelden(&self, anfrage: &AbmeldeAnfrage) -> Result<(), DienstFehler> {
        self.abmeldungen
            .lock()
            .unwrap()
            .push(anfrage.machine_id.clone());
        Ok(())
    }
}

fn pruefer() -> Lizenzpruefer {
    Lizenzpruefer::new(vec![testschluessel::oeffentlich()])
        .mit_ticket_schluesseln(vec![testschluessel::ticket_oeffentlich()])
}

struct Rechner {
    core: CoreServices,
    name: String,
    ticket: PathBuf,
}

fn rechner(
    db: &db_sqlite::SqliteDatabase,
    dir: &Path,
    server: &Arc<Server>,
    machine_id: &str,
) -> Rechner {
    let repos = db.repositories();
    let ticket = dir.join(machine_id).join("lizenz.ticket");
    let mut lizenz = DatenbankLizenz::new(repos.lizenz.clone(), pruefer())
        .mit_arbeitsplatzpflicht()
        .mit_abo(AboAnbindung {
            dienst: server.clone(),
            machine_id: machine_id.into(),
            rechnername: format!("PC-{machine_id}"),
            app_version: "0.1.0".into(),
            ticket_datei: ticket.clone(),
        });
    // Im Test sofort erneut versuchen dürfen.
    lizenz.erneut_versuchen_nach = Duration::zero();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.join("dateien"),
    })
    .unwrap();
    Rechner {
        core: CoreServices::new(repos, storage, Arc::new(LboEngine::new()), Arc::new(lizenz)),
        name: format!("PC-{machine_id}"),
        ticket,
    }
}

impl Rechner {
    async fn starten(&self) -> core_lib::license::Lizenzstatus {
        self.core
            .license
            .arbeitsplatz_belegen(&self.name, "Clara Chefin")
            .await
            .unwrap()
    }
}

async fn projekt_anlegen(core: &CoreServices, wer: &Benutzer) -> Result<(), core_lib::CoreError> {
    core.create_project(
        NewProject {
            name: "Test".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
        },
        wer,
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn abo_aktivieren_und_zweiter_rechner() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let server = Server::new();
    let a = rechner(&db, dir.path(), &server, "a");
    let chefin = a.core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let kollege = a.core.anmelden("kollege", "Karl Kollege").await.unwrap();

    // Falscher Schlüssel: verständliche Meldung, nichts gespeichert.
    let err = a
        .core
        .abo_aktivieren("BPIO-FALSCH", &chefin)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("unbekannt"), "{err}");
    // Nur Administratoren.
    assert!(
        a.core
            .abo_aktivieren("BPIO-TEST1-TEST2", &kollege)
            .await
            .is_err()
    );

    // Kleinbuchstaben und Leerzeichen beim Abtippen sind egal.
    let s = a
        .core
        .abo_aktivieren(" bpio-test1-test2 ", &chefin)
        .await
        .unwrap();
    assert_eq!(s.lizenz.as_ref().unwrap().lizenz_id, server.lizenz_id);
    let abo = s.abo.as_ref().unwrap();
    assert_eq!(abo.lizenzschluessel, "BPIO-•••••-TEST2");
    assert!(abo.offline_gueltig_bis.unwrap() > Utc::now() + Duration::days(29));
    assert!(a.ticket.exists());
    let s = a.starten().await;
    assert_eq!(s.zustand, Lizenzzustand::Lizenziert);
    assert!(
        s.meldung.contains("ohne Internet nutzbar bis"),
        "{}",
        s.meldung
    );
    projekt_anlegen(&a.core, &chefin).await.unwrap();

    // Zweiter Rechner an derselben Datenbank holt sich beim Start selbst ein
    // Ticket mit seiner eigenen Machine-ID.
    let b = rechner(&db, dir.path(), &server, "b");
    let s = b.starten().await;
    assert_eq!(s.zustand, Lizenzzustand::Lizenziert);
    assert_eq!(
        server.anfragen.lock().unwrap().last().unwrap().machine_id,
        "b"
    );
    projekt_anlegen(&b.core, &chefin).await.unwrap();

    // Ein frisches Ticket wird nicht bei jeder Meldung erneuert.
    let vorher = server.anzahl();
    b.starten().await;
    assert_eq!(server.anzahl(), vorher);
    // Auf Wunsch schon.
    b.core.abo_bestaetigen().await.unwrap();
    assert_eq!(server.anzahl(), vorher + 1);
}

#[tokio::test]
async fn kopiertes_ticket_gilt_nicht() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let server = Server::new();
    let a = rechner(&db, dir.path(), &server, "a");
    let chefin = a.core.anmelden("chefin", "Clara Chefin").await.unwrap();
    a.core
        .abo_aktivieren("BPIO-TEST1-TEST2", &chefin)
        .await
        .unwrap();

    // Rechner c ist offline und bekommt das Ticket von a kopiert.
    server.setze(Verhalten::Offline);
    let c = rechner(&db, dir.path(), &server, "c");
    std::fs::create_dir_all(c.ticket.parent().unwrap()).unwrap();
    std::fs::copy(&a.ticket, &c.ticket).unwrap();
    let s = c.starten().await;
    assert!(
        matches!(s.zustand, Lizenzzustand::NichtBestaetigt { .. }),
        "{:?}",
        s.zustand
    );
    assert!(!s.schreiben_erlaubt);
    assert!(projekt_anlegen(&c.core, &chefin).await.is_err());
    // Lesen geht weiter.
    c.core.list_projects().await.unwrap();
}

#[tokio::test]
async fn offline_bis_zum_ablauf_des_tickets() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let server = Server::new();
    let a = rechner(&db, dir.path(), &server, "a");
    let chefin = a.core.anmelden("chefin", "Clara Chefin").await.unwrap();
    a.core
        .abo_aktivieren("BPIO-TEST1-TEST2", &chefin)
        .await
        .unwrap();

    // Server weg: das vorhandene Ticket gilt weiter.
    server.setze(Verhalten::Offline);
    assert_eq!(a.starten().await.zustand, Lizenzzustand::Lizenziert);
    let err = a.core.abo_bestaetigen().await.unwrap_err();
    assert!(err.to_string().contains("nicht erreichbar"), "{err}");
    assert_eq!(a.starten().await.zustand, Lizenzzustand::Lizenziert);

    // Ticket abgelaufen (hier: Server stellt eines aus, das schon abgelaufen
    // ist, wie nach 30 Tagen offline): nur noch lesen.
    server.setze(Verhalten::Ausstellen(-1));
    a.core.abo_bestaetigen().await.unwrap();
    server.setze(Verhalten::Offline);
    let s = a.starten().await;
    assert!(
        matches!(s.zustand, Lizenzzustand::BestaetigungAbgelaufen { .. }),
        "{:?}",
        s.zustand
    );
    assert!(!s.schreiben_erlaubt);
    assert!(s.meldung.contains("mit dem Internet"), "{}", s.meldung);
    assert!(projekt_anlegen(&a.core, &chefin).await.is_err());
    a.core.list_projects().await.unwrap();

    // Wieder online: beim nächsten Kontakt automatisch frei.
    server.setze(Verhalten::Ausstellen(30));
    assert_eq!(a.starten().await.zustand, Lizenzzustand::Lizenziert);
    projekt_anlegen(&a.core, &chefin).await.unwrap();
}

#[tokio::test]
async fn abgelehnt_abgemeldet_und_lizenzdatei() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let server = Server::new();
    let a = rechner(&db, dir.path(), &server, "a");
    let chefin = a.core.anmelden("chefin", "Clara Chefin").await.unwrap();
    a.core
        .abo_aktivieren("BPIO-TEST1-TEST2", &chefin)
        .await
        .unwrap();

    // Abo gekündigt: Server lehnt ab, das Ticket wird sofort verworfen.
    server.setze(Verhalten::Ablehnen("Das Abo ist nicht mehr aktiv."));
    let err = a.core.abo_bestaetigen().await.unwrap_err();
    assert!(err.to_string().contains("nicht mehr aktiv"), "{err}");
    assert!(!a.ticket.exists());
    let s = a.starten().await;
    assert_eq!(
        s.zustand,
        Lizenzzustand::NichtBestaetigt {
            grund: "Das Abo ist nicht mehr aktiv.".into()
        }
    );

    // Abo verlängert, dann Rechner abmelden: Platz frei, keine
    // automatische Neuaktivierung in dieser Sitzung.
    server.setze(Verhalten::Ausstellen(30));
    assert_eq!(a.starten().await.zustand, Lizenzzustand::Lizenziert);
    let s = a.core.abo_abmelden(&chefin).await.unwrap();
    assert_eq!(*server.abmeldungen.lock().unwrap(), ["a"]);
    assert!(!a.ticket.exists());
    assert!(!s.schreiben_erlaubt);
    let vorher = server.anzahl();
    assert!(!a.starten().await.schreiben_erlaubt);
    assert_eq!(server.anzahl(), vorher);

    // Eine Lizenzdatei ersetzt das Abo.
    let datei = ausstellen(
        &Lizenz {
            lizenz_id: Uuid::new_v4(),
            lizenznehmer: "Architekturbüro Muster".into(),
            edition: Edition::Einzelplatz,
            arbeitsplaetze: 1,
            ausgestellt_am: Utc::now().date_naive(),
            gueltig_bis: None,
            wartung_bis: None,
        },
        &testschluessel::geheim(),
    );
    a.core
        .lizenz_einspielen(datei.as_bytes(), &chefin)
        .await
        .unwrap();
    let s = a.starten().await;
    assert_eq!(s.zustand, Lizenzzustand::Lizenziert);
    assert_eq!(s.abo, None);
    assert_eq!(s.lizenz.unwrap().edition, Edition::Einzelplatz);
}
