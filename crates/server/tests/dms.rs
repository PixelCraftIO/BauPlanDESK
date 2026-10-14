//! Ende-zu-Ende: Mehrbenutzerbetrieb im DMS über `CoreServices` mit SQLite
//! und lokalem Storage. Rollen, Sperren, Versionskonflikte, Wiederherstellen
//! und Protokoll.

use std::sync::Arc;

use bytes::Bytes;
use core_lib::benutzer::{BenutzerAenderung, NeuerBenutzer, Rolle};
use core_lib::domain::{Aktion, Bundesland, NewProject};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::{CoreError, CoreServices};

async fn core(dir: &std::path::Path) -> CoreServices {
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.to_path_buf(),
    })
    .unwrap();
    CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
}

#[tokio::test]
async fn erste_anmeldung_wird_administrator() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    // Das Systemkonto des Servers zählt nicht als Administrator.
    core.systembenutzer().await.unwrap();

    let chefin = core
        .anmelden("BUERO\\Chefin", "Clara Chefin")
        .await
        .unwrap();
    assert_eq!(
        (chefin.anmeldename.as_str(), chefin.rolle),
        ("chefin", Rolle::Administrator)
    );
    let azubi = core.anmelden("azubi", "").await.unwrap();
    assert_eq!(
        (azubi.anzeigename.as_str(), azubi.rolle),
        ("azubi", Rolle::Lesen)
    );
    // Zweite Anmeldung liefert dasselbe Konto.
    assert_eq!(
        core.anmelden("chefin@buero.local", "x").await.unwrap(),
        chefin
    );

    // Lesen darf nichts ändern.
    let err = core
        .create_project(
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &azubi,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Forbidden(_)), "{err:?}");
    let err = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "neu".into(),
                anzeigename: "Neu".into(),
                rolle: Rolle::Administrator,
            },
            &azubi,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Forbidden(_)), "{err:?}");

    // Der letzte Administrator kann sich nicht selbst herabstufen …
    let err = core
        .benutzer_aendern(
            BenutzerAenderung {
                id: chefin.id,
                anzeigename: chefin.anzeigename.clone(),
                rolle: Rolle::Projektleitung,
                aktiv: true,
            },
            &chefin,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)), "{err:?}");
    // … aber den Azubi befördern und deaktivierte Konten sperren.
    let azubi = core
        .benutzer_aendern(
            BenutzerAenderung {
                id: azubi.id,
                anzeigename: "Alex Azubi".into(),
                rolle: Rolle::Bearbeitung,
                aktiv: false,
            },
            &chefin,
        )
        .await
        .unwrap();
    let err = core.anmelden("azubi", "").await.unwrap_err();
    assert!(matches!(err, CoreError::Forbidden(_)), "{err:?}");
    assert!(matches!(
        azubi.pruefe(core_lib::benutzer::Recht::DokumentAblegen),
        Err(CoreError::Forbidden(_))
    ));
}

#[tokio::test]
async fn zwei_arbeitsplaetze_bearbeiten_einen_plan() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let leitung = core.anmelden("leitung", "Lea Leitung").await.unwrap();
    let anna = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "anna".into(),
                anzeigename: "Anna".into(),
                rolle: Rolle::Bearbeitung,
            },
            &leitung,
        )
        .await
        .unwrap();
    let ben = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "ben".into(),
                anzeigename: "Ben".into(),
                rolle: Rolle::Bearbeitung,
            },
            &leitung,
        )
        .await
        .unwrap();
    let projekt = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &leitung,
        )
        .await
        .unwrap();
    let v1 = core
        .store_document(
            projekt.id,
            None,
            None,
            "Grundriss.dxf",
            Bytes::from_static(b"v1"),
            &anna,
        )
        .await
        .unwrap();
    let doc = v1.document_id;
    assert_eq!(v1.created_by, "Anna");

    // Anna und Ben haben beide Version 1 geöffnet. Ben speichert zuerst.
    let v2 = core
        .store_document(
            projekt.id,
            Some(doc),
            Some(1),
            "Grundriss.dxf",
            Bytes::from_static(b"ben"),
            &ben,
        )
        .await
        .unwrap();
    assert_eq!(v2.version, 2);
    let err = core
        .store_document(
            projekt.id,
            Some(doc),
            Some(1),
            "Grundriss.dxf",
            Bytes::from_static(b"anna"),
            &anna,
        )
        .await
        .unwrap_err();
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
    // Die verlorene Datei bleibt nicht als Waise im Storage liegen.
    let dateien = walk(dir.path());
    assert_eq!(dateien.len(), 2, "{dateien:?}");

    // Anna sperrt den Plan; Ben kann nichts ablegen und nicht entsperren.
    let sperre = core.dokument_sperren(doc, &anna).await.unwrap();
    assert_eq!(sperre.benutzer_name, "Anna");
    let err = core
        .store_document(
            projekt.id,
            Some(doc),
            Some(2),
            "Grundriss.dxf",
            Bytes::from_static(b"x"),
            &ben,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Gesperrt { ref von, .. } if von == "Anna"),
        "{err:?}"
    );
    assert!(matches!(
        core.dokument_sperren(doc, &ben).await,
        Err(CoreError::Gesperrt { .. })
    ));
    assert!(matches!(
        core.dokument_entsperren(doc, &ben).await,
        Err(CoreError::Forbidden(_))
    ));
    assert_eq!(core.sperren_im_projekt(projekt.id).await.unwrap(), [sperre]);

    // Anna darf trotz Sperre ablegen.
    let v3 = core
        .store_document(
            projekt.id,
            Some(doc),
            Some(2),
            "Grundriss.dxf",
            Bytes::from_static(b"anna v3"),
            &anna,
        )
        .await
        .unwrap();
    assert_eq!(v3.version, 3);

    // Die Projektleitung hebt die Sperre auf (Anna ist im Urlaub).
    core.dokument_entsperren(doc, &leitung).await.unwrap();
    assert!(
        core.sperren_im_projekt(projekt.id)
            .await
            .unwrap()
            .is_empty()
    );

    // Version 1 wiederherstellen ergibt Version 4 mit altem Inhalt.
    let v4 = core.version_wiederherstellen(doc, 1, &ben).await.unwrap();
    assert_eq!(v4.version, 4);
    let (_, inhalt) = core.load_document(doc).await.unwrap();
    assert_eq!(&inhalt[..], b"v1");
    let (alt, inhalt) = core.load_version(doc, 2).await.unwrap();
    assert_eq!(
        (alt.created_by.as_str(), &inhalt[..]),
        ("Ben", b"ben".as_slice())
    );

    // Ein Dokument eines anderen Projekts lässt sich nicht unterschieben.
    let anderes = core
        .create_project(
            NewProject {
                name: "Anderes".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &leitung,
        )
        .await
        .unwrap();
    let err = core
        .store_document(
            anderes.id,
            Some(doc),
            None,
            "x.dxf",
            Bytes::from_static(b"x"),
            &anna,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)), "{err:?}");

    let protokoll = core.protokoll(projekt.id, 100).await.unwrap();
    let aktionen: Vec<_> = protokoll.iter().rev().map(|e| e.eintrag.aktion).collect();
    assert_eq!(
        aktionen,
        [
            Aktion::ProjektAngelegt,
            Aktion::DokumentAbgelegt,
            Aktion::DokumentAbgelegt,
            Aktion::DokumentGesperrt,
            Aktion::DokumentAbgelegt,
            Aktion::SperreAufgehoben,
            Aktion::DokumentAbgelegt,
        ]
    );
    assert_eq!(protokoll[0].eintrag.benutzer_name, "Ben");
    assert_eq!(
        protokoll[0].eintrag.details,
        "Grundriss.dxf, Version 4 (wiederhergestellt aus Version 1)"
    );
    assert_eq!(
        protokoll[1].eintrag.details,
        "Grundriss.dxf (gesperrt von Anna)"
    );
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dateien = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            dateien.extend(walk(&p));
        } else if !p.extension().is_some_and(|e| e == "json") {
            dateien.push(p);
        }
    }
    dateien
}
