//! Bibliothek → Eingabemaske → ein Projektdokument → erneute Bearbeitung als Version.
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject},
    formular::{
        pdf::{Feldart, Feldwert, felder_lesen},
        testdaten::musterformular,
    },
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use std::{collections::BTreeMap, sync::Arc};
#[tokio::test]
async fn kein_leeres_projektdokument_und_keine_doppelte_ablage() {
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().join("Ablage"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("formular", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Projektformular".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let source = musterformular();
    let root = temp.path().join("Formulare");
    let vorlage = storage::formularbibliothek::ablegen(&root, "Bauantrag.pdf", &source)
        .await
        .unwrap();
    let schema = core.formular_schema(p.id, &source).await.unwrap();
    assert!(core.list_documents(p.id).await.unwrap().is_empty());
    let feld = schema
        .felder
        .iter()
        .find(|f| f.art == Feldart::Text && !f.schreibgeschuetzt)
        .unwrap()
        .name
        .clone();
    let werte = BTreeMap::from([(feld.clone(), Feldwert::Text("Daniel".into()))]);
    let d = core
        .formular_aus_bibliothek(p.id, &vorlage.name, &source, &werte, &user)
        .await
        .unwrap();
    assert_eq!(d.version, 1);
    assert_eq!(core.list_documents(p.id).await.unwrap().len(), 1);
    assert_eq!(
        felder_lesen(&core.load_document(d.document_id).await.unwrap().1)
            .unwrap()
            .into_iter()
            .find(|f| f.name == feld)
            .unwrap()
            .wert,
        Some(Feldwert::Text("Daniel".into()))
    );
    let d2 = core
        .formular_im_projekt_speichern(
            d.document_id,
            1,
            &BTreeMap::from([(feld.clone(), Feldwert::Text("Daniel korrigiert".into()))]),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(d2.document_id, d.document_id);
    assert_eq!(d2.version, 2);
    assert_eq!(
        core_lib::projektablage::versionen(core.list_documents(p.id).await.unwrap(), false).len(),
        1
    );
    assert!(matches!(
        core.formular_im_projekt_speichern(d.document_id, 1, &werte, &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert_eq!(
        storage::formularbibliothek::lesen(&root, &vorlage.name, Some(&vorlage.sha256))
            .await
            .unwrap(),
        source
    );
    let mut reader = user.clone();
    reader.rolle = core_lib::benutzer::Rolle::Lesen;
    assert!(matches!(
        core.formular_aus_bibliothek(p.id, &vorlage.name, &source, &werte, &reader)
            .await,
        Err(CoreError::Forbidden(_))
    ));
}
