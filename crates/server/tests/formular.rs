//! Ende-zu-Ende: Formular ins DMS laden, analysieren, befüllen, Ergebnis prüfen.
//! Läuft gegen SQLite und lokales Storage, also dieselbe Verdrahtung wie die
//! Einzelplatz-App.

use std::collections::BTreeMap;
use std::sync::Arc;

use bytes::Bytes;
use core_lib::CoreServices;
use core_lib::domain::{Bundesland, NewProject};
use core_lib::formular::{Feldwert, Formularkatalog, Formularvorlage, pdf, testdaten};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;

#[tokio::test]
async fn formular_aus_dms_befuellen() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.path().to_path_buf(),
    })
    .unwrap();
    let vorlage: Formularvorlage = serde_json::from_str(
        r#"{ "id": "t", "titel": "Testantrag", "version": "1",
             "erkennung": ["Bauherr_Name"],
             "zuordnung": { "Bauherr_Name": "projekt.name",
                            "Grundstueck.Flaeche": "grundstueck.flaeche_m2" } }"#,
    )
    .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_formularen(Formularkatalog::new(vec![vorlage]));

    let wer = core.anmelden("test", "Test").await.unwrap();
    let projekt = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(1250.5),
                planungsrecht: Default::default(),
            },
            &wer,
        )
        .await
        .unwrap();
    let leer = core
        .store_document(
            projekt.id,
            None,
            None,
            "Bauantrag.pdf",
            Bytes::from(testdaten::musterformular()),
            &wer,
        )
        .await
        .unwrap();

    let analyse = core.formular_analysieren(leer.document_id).await.unwrap();
    assert_eq!(analyse.vorlage.unwrap().titel, "Testantrag");
    assert_eq!(
        analyse.vorschlag["Grundstueck.Flaeche"],
        Feldwert::Text("1250,50".into())
    );

    let mut werte = analyse.vorschlag.clone();
    werte.insert("Sonderbau".into(), Feldwert::Ja(true));
    let ausgefuellt = core
        .formular_ausfuellen(leer.document_id, &werte, &wer)
        .await
        .unwrap();
    assert_eq!(ausgefuellt.file_name, "Bauantrag (ausgefüllt).pdf");
    assert_ne!(ausgefuellt.document_id, leer.document_id);

    let (_, inhalt) = core.load_document(ausgefuellt.document_id).await.unwrap();
    let felder: BTreeMap<_, _> = pdf::felder_lesen(&inhalt)
        .unwrap()
        .into_iter()
        .map(|f| (f.name, f.wert))
        .collect();
    assert_eq!(
        felder["Bauherr_Name"],
        Some(Feldwert::Text("MFH Lindenweg".into()))
    );
    assert_eq!(felder["Sonderbau"], Some(Feldwert::Ja(true)));

    // Original bleibt leer.
    let (_, original) = core.load_document(leer.document_id).await.unwrap();
    let original = pdf::felder_lesen(&original).unwrap();
    assert!(
        original
            .iter()
            .all(|f| f.name != "Bauherr_Name" || f.wert.is_none())
    );
}
