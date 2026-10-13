//! Konformitätstests für Datenbank-Implementierungen (Feature `testdaten`).
//!
//! `db-sqlite` und `db-postgres` rufen [`repositories`] mit einer leeren
//! Datenbank auf. So verhalten sich beide Backends nachweislich gleich,
//! vor allem beim Optimistic Locking und bei Sperren.

use chrono::Utc;

use crate::CoreError;
use crate::benutzer::{BenutzerAenderung, NeuerBenutzer, Rolle};
use crate::domain::{
    Aktion, Bundesland, DocumentId, DocumentVersion, NeuerProtokolleintrag, NewProject,
    ProjectUpdate,
};
use crate::kunden::{KundeId, Kundendaten, Kundenkategorie, Kundentyp};
use crate::license::Arbeitsplatzbelegung;
use crate::repository::Repositories;
use crate::storage::ContentHash;

/// Eindeutiger Anmeldename, damit die Suite auch auf einer schon benutzten
/// Testdatenbank läuft.
fn eindeutig(name: &str) -> String {
    format!("{name}-{}", &uuid::Uuid::new_v4().simple().to_string()[..8])
}

/// Führt alle Prüfungen nacheinander aus; bricht beim ersten Fehler mit
/// einer Panik ab (für `#[tokio::test]`).
pub async fn repositories(repos: Repositories) {
    projekte(&repos).await;
    dokumente(&repos).await;
    sperren(&repos).await;
    benutzer(&repos).await;
    protokoll(&repos).await;
    lizenz(&repos).await;
    bauantraege(&repos).await;
    kunden(&repos).await;
}

async fn projekt(repos: &Repositories) -> crate::domain::Project {
    repos
        .projects
        .create(NewProject {
            name: "MFH Lindenweg".into(),
            bundesland: Bundesland::Nw,
            grundstuecksflaeche_m2: Some(600.0),
        })
        .await
        .unwrap()
}

fn version(doc: DocumentId, p: &crate::domain::Project, inhalt: &[u8]) -> DocumentVersion {
    DocumentVersion {
        document_id: doc,
        project_id: p.id,
        version: 0,
        file_name: "lageplan.pdf".into(),
        storage_key: format!("k/{}", uuid::Uuid::new_v4()),
        sha256: ContentHash::of(inhalt),
        size_bytes: inhalt.len() as u64,
        created_by: "test".into(),
        created_at: Utc::now(),
    }
}

async fn projekte(repos: &Repositories) {
    let p = projekt(repos).await;
    let update = ProjectUpdate {
        id: p.id,
        expected_version: 1,
        name: "MFH Lindenweg, 2. Entwurf".into(),
        bundesland: Bundesland::Nw,
        grundstuecksflaeche_m2: Some(600.0),
    };
    assert_eq!(
        repos.projects.update(update.clone()).await.unwrap().version,
        2
    );
    // Zweiter Client mit veralteter Version
    let err = repos.projects.update(update).await.unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::VersionConflict {
                expected: 1,
                actual: 2
            }
        ),
        "{err:?}"
    );
}

async fn dokumente(repos: &Repositories) {
    let p = projekt(repos).await;
    let doc = DocumentId::new();
    let d = &repos.documents;
    assert_eq!(
        d.add_version(version(doc, &p, b"v1"), None)
            .await
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        d.add_version(version(doc, &p, b"v2"), Some(1))
            .await
            .unwrap()
            .version,
        2
    );

    // Zwei Arbeitsplätze haben Version 2 geöffnet; der zweite verliert.
    assert_eq!(
        d.add_version(version(doc, &p, b"v3a"), Some(2))
            .await
            .unwrap()
            .version,
        3
    );
    let err = d
        .add_version(version(doc, &p, b"v3b"), Some(2))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::VersionConflict {
                expected: 2,
                actual: 3
            }
        ),
        "{err:?}"
    );

    let history = d.history(doc).await.unwrap();
    assert_eq!(
        history.iter().map(|v| v.version).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(d.latest(doc).await.unwrap().sha256, ContentHash::of(b"v3a"));
    assert_eq!(d.list_for_project(p.id).await.unwrap().len(), 3);
}

async fn sperren(repos: &Repositories) {
    let p = projekt(repos).await;
    let doc = DocumentId::new();
    repos
        .documents
        .add_version(version(doc, &p, b"v1"), None)
        .await
        .unwrap();
    let anna = repos
        .benutzer
        .create(NeuerBenutzer {
            anmeldename: eindeutig("anna"),
            anzeigename: "Anna".into(),
            rolle: Rolle::Bearbeitung,
        })
        .await
        .unwrap();
    let ben = repos
        .benutzer
        .create(NeuerBenutzer {
            anmeldename: eindeutig("ben"),
            anzeigename: "Ben".into(),
            rolle: Rolle::Bearbeitung,
        })
        .await
        .unwrap();

    let d = &repos.documents;
    let s = d.lock(doc, p.id, &anna).await.unwrap();
    assert_eq!((s.benutzer_id, s.project_id), (anna.id, p.id));
    // Erneutes Sperren durch Anna ändert nichts.
    assert_eq!(d.lock(doc, p.id, &anna).await.unwrap(), s);
    let err = d.lock(doc, p.id, &ben).await.unwrap_err();
    assert!(
        matches!(err, CoreError::Gesperrt { ref von, .. } if von == "Anna"),
        "{err:?}"
    );

    // Ben kann Annas Sperre nicht als eigene aufheben.
    assert_eq!(d.unlock(doc, Some(ben.id)).await.unwrap(), None);
    assert_eq!(
        d.locks_for_project(p.id).await.unwrap(),
        std::slice::from_ref(&s)
    );
    assert_eq!(d.unlock(doc, Some(anna.id)).await.unwrap(), Some(s));
    assert_eq!(d.get_lock(doc).await.unwrap(), None);

    // Ohne Inhaberprüfung (Projektleitung hebt fremde Sperre auf)
    d.lock(doc, p.id, &ben).await.unwrap();
    assert!(d.unlock(doc, None).await.unwrap().is_some());
    assert_eq!(d.unlock(doc, None).await.unwrap(), None);
}

async fn benutzer(repos: &Repositories) {
    let b = &repos.benutzer;
    let chef_name = eindeutig("chef");
    let chef = b
        .create(NeuerBenutzer {
            anmeldename: chef_name.clone(),
            anzeigename: "Chefin".into(),
            rolle: Rolle::Administrator,
        })
        .await
        .unwrap();
    assert_eq!(
        b.by_anmeldename(&chef_name).await.unwrap(),
        Some(chef.clone())
    );
    assert_eq!(b.by_anmeldename("niemand").await.unwrap(), None);
    assert_eq!(b.get(chef.id).await.unwrap(), chef);

    let doppelt = b
        .create(NeuerBenutzer {
            anmeldename: chef_name.clone(),
            anzeigename: "Noch ein Chef".into(),
            rolle: Rolle::Lesen,
        })
        .await
        .unwrap_err();
    assert!(matches!(doppelt, CoreError::InvalidInput(_)), "{doppelt:?}");

    let geaendert = b
        .update(BenutzerAenderung {
            id: chef.id,
            anzeigename: "Chefin Muster".into(),
            rolle: Rolle::Projektleitung,
            aktiv: false,
        })
        .await
        .unwrap();
    assert_eq!(
        (
            geaendert.anzeigename.as_str(),
            geaendert.rolle,
            geaendert.aktiv
        ),
        ("Chefin Muster", Rolle::Projektleitung, false)
    );
    assert!(b.list().await.unwrap().iter().any(|x| x.id == chef.id));
}

async fn protokoll(repos: &Repositories) {
    let p = projekt(repos).await;
    let wer = repos
        .benutzer
        .create(NeuerBenutzer {
            anmeldename: eindeutig("protokollant"),
            anzeigename: "Protokollant".into(),
            rolle: Rolle::Bearbeitung,
        })
        .await
        .unwrap();
    for (i, aktion) in [Aktion::ProjektAngelegt, Aktion::DokumentAbgelegt]
        .into_iter()
        .enumerate()
    {
        repos
            .protokoll
            .append(NeuerProtokolleintrag {
                zeit: Utc::now(),
                benutzer_id: wer.id,
                benutzer_name: wer.anzeigename.clone(),
                aktion,
                project_id: Some(p.id),
                document_id: (i == 1).then(DocumentId::new),
                details: format!("Eintrag {i}"),
            })
            .await
            .unwrap();
    }
    let liste = repos.protokoll.list_for_project(p.id, 10).await.unwrap();
    assert_eq!(
        liste.iter().map(|e| e.eintrag.aktion).collect::<Vec<_>>(),
        [Aktion::DokumentAbgelegt, Aktion::ProjektAngelegt],
        "neueste zuerst"
    );
    assert!(liste[0].id > liste[1].id);
    assert_eq!(
        repos
            .protokoll
            .list_for_project(p.id, 1)
            .await
            .unwrap()
            .len(),
        1
    );
}

async fn lizenz(repos: &Repositories) {
    let l = &repos.lizenz;
    let k = eindeutig("schluessel");
    assert_eq!(l.einstellung(&k).await.unwrap(), None);
    assert_eq!(l.einstellung_vorbelegen(&k, "erst").await.unwrap(), "erst");
    assert_eq!(l.einstellung_vorbelegen(&k, "zweit").await.unwrap(), "erst");
    l.einstellung_setzen(&k, "neu").await.unwrap();
    assert_eq!(l.einstellung(&k).await.unwrap().as_deref(), Some("neu"));

    // Ab hier gelten alle älteren Belegungen (auch aus früheren Läufen)
    // als abgelaufen.
    let t0 = Utc::now();
    let min = chrono::Duration::minutes(1);
    let platz = |name: &str, t| Arbeitsplatzbelegung {
        arbeitsplatz: name.into(),
        benutzer_name: format!("Benutzer an {name}"),
        belegt_seit: t,
        letzte_meldung: t,
    };
    assert!(
        l.arbeitsplatz_belegen(platz("PC-1", t0), 2, t0)
            .await
            .unwrap()
    );
    assert!(
        l.arbeitsplatz_belegen(platz("PC-2", t0 + min), 2, t0)
            .await
            .unwrap()
    );
    // Lizenz für zwei Plätze: der dritte Rechner bekommt keinen.
    assert!(
        !l.arbeitsplatz_belegen(platz("PC-3", t0 + min), 2, t0)
            .await
            .unwrap()
    );
    // Ein belegter Rechner meldet sich weiter.
    assert!(
        l.arbeitsplatz_belegen(platz("PC-1", t0 + min * 2), 2, t0)
            .await
            .unwrap()
    );
    let aktiv = l.arbeitsplaetze(t0).await.unwrap();
    assert_eq!(
        aktiv
            .iter()
            .map(|b| b.arbeitsplatz.as_str())
            .collect::<Vec<_>>(),
        ["PC-1", "PC-2"]
    );
    assert_eq!(
        aktiv[0].letzte_meldung.timestamp_millis(),
        (t0 + min * 2).timestamp_millis()
    );

    // PC-2 hat sich seit t0 + 1 min nicht gemeldet und läuft ab.
    let spaeter = t0 + chrono::Duration::seconds(90);
    assert!(
        l.arbeitsplatz_belegen(platz("PC-3", t0 + min * 3), 2, spaeter)
            .await
            .unwrap()
    );
    assert_eq!(l.arbeitsplaetze(spaeter).await.unwrap().len(), 2);

    l.arbeitsplatz_freigeben("PC-1").await.unwrap();
    l.arbeitsplatz_freigeben("PC-3").await.unwrap();
    assert!(l.arbeitsplaetze(t0).await.unwrap().is_empty());
}

async fn bauantraege(repos: &Repositories) {
    let p = projekt(repos).await;
    let r = &repos.bauantraege;
    assert_eq!(r.bauantrag(p.id).await.unwrap(), None);

    let daten = r#"{"bezeichnung":"Neubau \"Lindenweg\"","anzahl_gebaeude":2}"#;
    assert_eq!(
        r.bauantrag_speichern(p.id, daten, 0, "anna").await.unwrap(),
        1
    );
    let gelesen = r.bauantrag(p.id).await.unwrap().unwrap();
    assert_eq!(gelesen.version, 1);
    assert_eq!(gelesen.geaendert_von, "anna");
    let json: serde_json::Value = serde_json::from_str(&gelesen.daten).unwrap();
    assert_eq!(json["bezeichnung"], "Neubau \"Lindenweg\"");

    // Zweites Anlegen und veraltete Version: Konflikt.
    assert!(matches!(
        r.bauantrag_speichern(p.id, "{}", 0, "ben").await,
        Err(CoreError::VersionConflict {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(
        r.bauantrag_speichern(p.id, "{}", 1, "ben").await.unwrap(),
        2
    );
    assert!(matches!(
        r.bauantrag_speichern(p.id, "{}", 1, "anna").await,
        Err(CoreError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    // Gelöschtes Projekt nimmt seinen Antrag mit.
    repos.projects.delete(p.id, p.version).await.unwrap();
    assert_eq!(r.bauantrag(p.id).await.unwrap(), None);
}

async fn kunden(repos: &Repositories) {
    let k = &repos.kunden;
    let mut daten = Kundendaten {
        typ: Kundentyp::Organisation,
        kategorie: Kundenkategorie::Investor,
        kundennummer: "K-1001".into(),
        firma: eindeutig("Wohnbau Süd GmbH"),
        ansprechpartner: "Frau Öztürk".into(),
        telefon: "089 123456".into(),
        email: "info@wohnbau-sued.de".into(),
        notizen: "Zahlt pünktlich.\nRückfragen nur per E-Mail.".into(),
        ..Kundendaten::default()
    };
    daten.anschrift.ort = "München".into();
    let investor = k.kunde_anlegen(daten.clone()).await.unwrap();
    assert_eq!(investor.version, 1);
    assert_eq!(investor.daten, daten);
    assert_eq!(k.kunde(investor.id).await.unwrap(), investor);
    assert!(k.kunden().await.unwrap().contains(&investor));
    assert!(matches!(
        k.kunde(KundeId::new()).await,
        Err(CoreError::NotFound(_))
    ));

    // Ändern mit Optimistic Locking
    daten.telefon = "089 654321".into();
    let geaendert = k
        .kunde_aendern(investor.id, daten.clone(), 1)
        .await
        .unwrap();
    assert_eq!(geaendert.version, 2);
    assert_eq!(geaendert.daten.telefon, "089 654321");
    assert_eq!(geaendert.created_at, investor.created_at);
    assert!(matches!(
        k.kunde_aendern(investor.id, daten.clone(), 1).await,
        Err(CoreError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    let person = k
        .kunde_anlegen(Kundendaten {
            vorname: "Erika".into(),
            familienname: eindeutig("Mustermann"),
            ..Kundendaten::default()
        })
        .await
        .unwrap();

    // Zuordnung als Bauherren, Reihenfolge bleibt erhalten.
    let p = projekt(repos).await;
    assert!(k.bauherren(p.id).await.unwrap().is_empty());
    let p2 = k
        .bauherren_setzen(p.id, &[person.id, investor.id], p.version)
        .await
        .unwrap();
    assert_eq!(p2.version, p.version + 1);
    let ids = |v: Vec<crate::kunden::Kunde>| v.into_iter().map(|k| k.id).collect::<Vec<_>>();
    assert_eq!(
        ids(k.bauherren(p.id).await.unwrap()),
        [person.id, investor.id]
    );
    // Veraltete Projektversion: Konflikt, Zuordnung unverändert.
    assert!(matches!(
        k.bauherren_setzen(p.id, &[investor.id], p.version).await,
        Err(CoreError::VersionConflict { .. })
    ));
    let p3 = k
        .bauherren_setzen(p.id, &[investor.id, person.id], p2.version)
        .await
        .unwrap();
    assert_eq!(
        ids(k.bauherren(p.id).await.unwrap()),
        [investor.id, person.id]
    );
    assert_eq!(
        k.projekte_des_kunden(person.id)
            .await
            .unwrap()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        [p.id]
    );
    assert!(matches!(
        k.bauherren_setzen(crate::domain::ProjectId::new(), &[], 1)
            .await,
        Err(CoreError::NotFound(_))
    ));

    // Zugeordnete Kunden lassen sich nicht löschen.
    let err = k.kunde_loeschen(person.id, 1).await.unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)), "{err:?}");
    let p4 = k
        .bauherren_setzen(p.id, &[investor.id], p3.version)
        .await
        .unwrap();
    assert!(matches!(
        k.kunde_loeschen(person.id, 7).await,
        Err(CoreError::VersionConflict {
            expected: 7,
            actual: 1
        })
    ));
    k.kunde_loeschen(person.id, 1).await.unwrap();
    assert!(matches!(
        k.kunde(person.id).await,
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        k.kunde_loeschen(person.id, 1).await,
        Err(CoreError::NotFound(_))
    ));

    // Gelöschtes Projekt gibt seine Bauherren frei.
    repos.projects.delete(p.id, p4.version).await.unwrap();
    assert!(k.projekte_des_kunden(investor.id).await.unwrap().is_empty());
    k.kunde_loeschen(investor.id, 2).await.unwrap();
}
