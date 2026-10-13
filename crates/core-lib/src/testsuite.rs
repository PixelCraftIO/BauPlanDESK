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
