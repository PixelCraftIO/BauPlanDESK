use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    benutzer::Rolle,
    domain::{Bundesland, NewProject},
    formular::{
        bibliothek::import_pruefen,
        pdf::{Feldwert, felder_lesen},
        testdaten::musterformular,
    },
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use std::{collections::BTreeMap, sync::Arc};
use storage::formularbibliothek as bibliothek;

#[tokio::test]
async fn gemeinsame_vorlage_in_zwei_unabhaengigen_projekten() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Formulare");
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().join("Projekte"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("bibliothek", "Bibliothek").await.unwrap();
    let source = musterformular();
    import_pruefen(&source, &user).unwrap();
    let mut reader = user.clone();
    reader.rolle = Rolle::Lesen;
    assert!(matches!(
        import_pruefen(&source, &reader),
        Err(CoreError::Forbidden(_))
    ));
    assert!(import_pruefen(b"kein PDF", &user).is_err());
    let vorlage = bibliothek::ablegen(&root, "Bauantrag.pdf", &source)
        .await
        .unwrap();
    assert_eq!(vorlage.name, "Bauantrag.pdf");
    let kopie = bibliothek::ablegen(&root, "Bauantrag.pdf", &source)
        .await
        .unwrap();
    assert_ne!(kopie.name, vorlage.name);
    let mut documents = Vec::new();
    for name in ["Projekt A", "Projekt B"] {
        let p = core
            .create_project(
                NewProject {
                    name: name.into(),
                    bundesland: Bundesland::By,
                    grundstuecksflaeche_m2: None,
                    planungsrecht: Default::default(),
                },
                &user,
            )
            .await
            .unwrap();
        let pdf = bibliothek::lesen(&root, &vorlage.name, Some(&vorlage.sha256))
            .await
            .unwrap();
        let d = core
            .store_document(p.id, None, None, &vorlage.name, Bytes::from(pdf), &user)
            .await
            .unwrap();
        documents.push(d);
    }
    assert_ne!(documents[0].document_id, documents[1].document_id);
    let textfeld = felder_lesen(&source)
        .unwrap()
        .into_iter()
        .find(|f| f.art == core_lib::formular::Feldart::Text && !f.schreibgeschuetzt)
        .unwrap()
        .name;
    let filled = core
        .formular_ausfuellen_version(
            documents[0].document_id,
            &BTreeMap::from([(textfeld.clone(), Feldwert::Text("Daniel".into()))]),
            Some(1),
            &user,
        )
        .await
        .unwrap();
    let result = core.load_document(filled.document_id).await.unwrap().1;
    assert_eq!(
        felder_lesen(&result)
            .unwrap()
            .into_iter()
            .find(|f| f.name == textfeld)
            .unwrap()
            .wert,
        Some(Feldwert::Text("Daniel".into()))
    );
    assert_eq!(
        core.load_document(documents[1].document_id)
            .await
            .unwrap()
            .1,
        source
    );
    assert_eq!(
        bibliothek::lesen(&root, &vorlage.name, Some(&vorlage.sha256))
            .await
            .unwrap(),
        source
    );
    // Ein zweiter Arbeitsplatz sieht dieselben Dateien ohne lokale Katalogdaten.
    assert_eq!(bibliothek::auflisten(&root).await.unwrap().len(), 2);
    tokio::fs::write(root.join("Direkt abgelegt.pdf"), &source)
        .await
        .unwrap();
    assert_eq!(bibliothek::auflisten(&root).await.unwrap().len(), 3);
    // Änderung auf dem Server nach Auswahl wird erkannt, statt unbemerkt kopiert.
    tokio::fs::write(root.join(&vorlage.name), &result)
        .await
        .unwrap();
    assert!(matches!(
        bibliothek::lesen(&root, &vorlage.name, Some(&vorlage.sha256)).await,
        Err(CoreError::HashMismatch { .. })
    ));
    assert!(
        bibliothek::lesen(&root, "../Bauantrag.pdf", None)
            .await
            .is_err()
    );
    assert!(
        bibliothek::ordner_pruefen(std::path::Path::new("relativ"))
            .await
            .is_err()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn links_werden_nicht_als_vorlagen_gelesen() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("quelle.pdf");
    std::fs::write(&source, musterformular()).unwrap();
    let root = temp.path().join("Formulare");
    std::fs::create_dir(&root).unwrap();
    std::os::unix::fs::symlink(&source, root.join("Link.pdf")).unwrap();
    assert!(bibliothek::lesen(&root, "Link.pdf", None).await.is_err());
    assert!(bibliothek::auflisten(&root).await.unwrap().is_empty());
}
