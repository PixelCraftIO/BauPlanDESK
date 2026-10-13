//! Ende-zu-Ende: Bauantrag erfassen, Büroangaben, XBau-Paket exportieren
//! über `CoreServices` mit SQLite und lokalem Storage.

use std::io::{Cursor, Read};
use std::sync::Arc;

use bytes::Bytes;
use core_lib::benutzer::{NeuerBenutzer, Rolle};
use core_lib::domain::{Aktion, Bundesland, NewProject};
use core_lib::kunden::Kundendaten;
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::xbau::codeliste::{ANLAGEN, MIMETYPES};
use core_lib::xbau::paket::NACHRICHT_DATEI;
use core_lib::xbau::{
    Anlagenwahl, Anschrift, Bauaufsicht, Buero, Codeeintrag, Codeliste, Codelisten,
};
use core_lib::{CoreError, CoreServices};

/// Codelisten mit Platzhalter-Einträgen (keine echten XRepository-Werte).
fn codelisten() -> Codelisten {
    let mut c = Codelisten::eingebaut();
    for (uri, code) in [(ANLAGEN, "0100"), (MIMETYPES, "application/pdf")] {
        c.hinzufuegen(Codeliste {
            uri: uri.into(),
            version: "test".into(),
            name: uri.into(),
            eintraege: vec![Codeeintrag {
                code: code.into(),
                beschreibung: "Test".into(),
            }],
        });
    }
    c
}

async fn core(dir: &std::path::Path, mit_codelisten: bool) -> CoreServices {
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.to_path_buf(),
    })
    .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    if mit_codelisten {
        core.mit_codelisten(codelisten())
    } else {
        core
    }
}

#[tokio::test]
async fn bauantrag_erfassen_und_exportieren() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
            },
            &chefin,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from_static(b"%PDF-1.7 Lageplan"),
            &chefin,
        )
        .await
        .unwrap();

    // Neu: Vorgaben, Version 0, fehlende Angaben werden genannt.
    let leer = core.bauantrag(p.id).await.unwrap();
    assert_eq!(leer.version, 0);
    assert!(leer.fehlende_angaben.iter().any(|f| f == "Büro: Name"));
    let err = core.xbau_exportieren(p.id, &chefin).await.unwrap_err();
    assert!(err.to_string().contains("Bauherr"), "{err}");

    core.buero_speichern(
        Buero {
            name: "Architekturbüro Muster".into(),
            anschrift: Anschrift::default(),
            kennung: "arc:4711".into(),
        },
        &chefin,
    )
    .await
    .unwrap();

    let mut antrag = leer.antrag;
    antrag.bauaufsicht = Bauaufsicht {
        name: "Bauordnungsamt Köln".into(),
        kennung: "bab:053150000".into(),
    };
    let anna = core
        .kunde_anlegen(
            Kundendaten {
                vorname: "Anna".into(),
                familienname: "Müller".into(),
                ..Kundendaten::default()
            },
            &chefin,
        )
        .await
        .unwrap();
    core.bauherren_setzen(p.id, vec![anna.id], p.version, &chefin)
        .await
        .unwrap();
    antrag.grundstueck.flurstuecke[0].gemarkung = "Lindenthal".into();
    antrag.grundstueck.flurstuecke[0].zaehler = "17".into();
    antrag.anlagen.push(Anlagenwahl {
        dokument: plan.document_id,
        anlageart: "0100".into(),
        bezeichnung: String::new(),
    });
    let gespeichert = core
        .bauantrag_speichern(p.id, antrag.clone(), 0, &chefin)
        .await
        .unwrap();
    assert_eq!(gespeichert.version, 1);
    assert_eq!(gespeichert.fehlende_angaben, Vec::<String>::new());
    assert_eq!(gespeichert.antrag, antrag);

    // Veraltete Version: Konflikt.
    assert!(matches!(
        core.bauantrag_speichern(p.id, antrag.clone(), 0, &chefin)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));

    // Exportieren darf auch, wer nur lesen darf.
    let leser = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "leser".into(),
                anzeigename: "Leo Leser".into(),
                rolle: Rolle::Lesen,
            },
            &chefin,
        )
        .await
        .unwrap();
    assert!(matches!(
        core.bauantrag_speichern(p.id, antrag, 1, &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    let export = core.xbau_exportieren(p.id, &leser).await.unwrap();
    assert!(
        export.dateiname.starts_with("Bauantrag MFH Lindenweg "),
        "{}",
        export.dateiname
    );
    let mut zip = zip::ZipArchive::new(Cursor::new(export.zip)).unwrap();
    let mut xml = String::new();
    zip.by_name(NACHRICHT_DATEI)
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("<code>05</code>"), "NRW");
    assert!(
        xml.contains("<name>Müller</name>"),
        "Bauherr aus der Kundendatenbank"
    );
    assert!(
        xml.contains(&anna.id.to_string()),
        "Kunden-ID als Akteur-ID"
    );
    assert!(xml.contains("<dateiname>Lageplan.pdf</dateiname>"));
    assert!(xml.contains(&format!("urn:uuid:{}", plan.document_id.0)));
    let mut pdf = Vec::new();
    zip.by_name("Lageplan.pdf")
        .unwrap()
        .read_to_end(&mut pdf)
        .unwrap();
    assert_eq!(pdf, b"%PDF-1.7 Lageplan");

    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::XbauExportiert
                && e.eintrag.benutzer_name == "Leo Leser")
    );
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::BauantragGeaendert)
    );
}

#[tokio::test]
async fn ungeeignete_anlagen_stehen_bei_den_fehlenden_angaben() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
            },
            &chefin,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Grundriss.dxf",
            Bytes::from_static(b"0\nEOF\n"),
            &chefin,
        )
        .await
        .unwrap();
    let mut antrag = core.bauantrag(p.id).await.unwrap().antrag;
    antrag.anlagen.push(Anlagenwahl {
        dokument: plan.document_id,
        anlageart: "9999".into(),
        bezeichnung: String::new(),
    });
    let g = core
        .bauantrag_speichern(p.id, antrag, 0, &chefin)
        .await
        .unwrap();
    assert!(
        g.fehlende_angaben
            .iter()
            .any(|f| f.starts_with("Grundriss.dxf: Der Dateityp image/vnd.dxf")),
        "{:?}",
        g.fehlende_angaben
    );
    assert!(
        g.fehlende_angaben
            .iter()
            .any(|f| f == "Grundriss.dxf: Art der Anlage 9999 steht nicht in der Codeliste"),
        "{:?}",
        g.fehlende_angaben
    );
}

#[tokio::test]
async fn ohne_codelisten_nennt_der_export_die_fehlende_liste() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), false).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
            },
            &chefin,
        )
        .await
        .unwrap();
    let g = core.bauantrag(p.id).await.unwrap();
    assert!(
        g.fehlende_angaben.iter().any(|f| f.contains(ANLAGEN)),
        "{:?}",
        g.fehlende_angaben
    );
}

#[tokio::test]
async fn buero_aendern_braucht_projektleitung() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ben = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "ben".into(),
                anzeigename: "Ben".into(),
                rolle: Rolle::Bearbeitung,
            },
            &chefin,
        )
        .await
        .unwrap();
    assert!(matches!(
        core.buero_speichern(Buero::default(), &ben).await,
        Err(CoreError::Forbidden(_))
    ));
    assert_eq!(core.buero().await.unwrap(), Buero::default());
}
