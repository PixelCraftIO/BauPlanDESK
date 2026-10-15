//! A real WAL database and immutable PDF sources survive archive verification and restore.
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    formular::{Feldwert, testdaten::musterformular},
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::ContentHash,
};
use std::{collections::BTreeMap, io::Write, sync::Arc};
use storage::sicherung::{self, Sicherungsdatei, Sicherungsmanifest, Sicherungsquelle};
#[tokio::test]
async fn konsistenter_wal_stand_mit_formularquellen_wird_in_neues_profil_wiederhergestellt() {
    let temp = tempfile::tempdir().unwrap();
    let dbpath = temp.path().join("live.db");
    let dms = temp.path().join("DMS");
    let db = db_sqlite::SqliteDatabase::open(&dbpath).await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dms.clone(), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("sicherung", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Gesicherter Stand".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .formular_aus_bibliothek(
            p.id,
            "Antrag.pdf",
            &musterformular(),
            &BTreeMap::from([("Bauherr_Name".into(), Feldwert::Text("Daniel".into()))]),
            &user,
        )
        .await
        .unwrap();
    core.checkliste_anwenden(
        p.id,
        core_lib::plattform::ablauf::Checkliste::standard().unwrap(),
        &user,
    )
    .await
    .unwrap();
    let snapshot = temp.path().join("bauprojekt.db");
    db.sichern(&snapshot).await.unwrap();
    // Live changes after the SQLite snapshot must not enter the restored project state.
    core.formular_im_projekt_speichern(
        d.document_id,
        1,
        &BTreeMap::from([(
            "Bauherr_Name".into(),
            Feldwert::Text("Stand nach Sicherung".into()),
        )]),
        &user,
    )
    .await
    .unwrap();
    let mut quellen = vec![Sicherungsquelle {
        name: "bauprojekt.db".into(),
        pfad: snapshot,
        erwartet: None,
    }];
    quellen.extend(sicherung::verzeichnis(&dms, "dateien").unwrap());
    let zip = temp.path().join("sicherung.zip");
    let manifest = sicherung::erstellen(&zip, &quellen).unwrap();
    assert!(
        manifest
            .dateien
            .iter()
            .any(|f| f.name.contains("formularvorlagen/"))
    );
    assert_eq!(
        sicherung::pruefen(&zip).unwrap().dateien.len(),
        manifest.dateien.len()
    );
    let ziel = temp.path().join("Wiederhergestellt");
    sicherung::wiederherstellen(&zip, &ziel).unwrap();
    db_sqlite::SqliteDatabase::datei_pruefen(&ziel.join("bauprojekt.db"))
        .await
        .unwrap();
    let restored = db_sqlite::SqliteDatabase::open(&ziel.join("bauprojekt.db"))
        .await
        .unwrap();
    let core2 = CoreServices::new(
        restored.repositories(),
        Arc::new(storage::LocalStorage::new(ziel.join("dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    assert_eq!(
        core2.get_project(p.id).await.unwrap().name,
        "Gesicherter Stand"
    );
    assert_eq!(core2.list_documents(p.id).await.unwrap().len(), 1);
    assert_eq!(
        core2.load_document(d.document_id).await.unwrap().0.version,
        1
    );
    assert_eq!(
        core2
            .formular_instanz(d.document_id, 1)
            .await
            .unwrap()
            .unwrap()
            .feldwerte["Bauherr_Name"],
        Feldwert::Text("Daniel".into())
    );
    assert_eq!(
        core2.projektstatus(p.id).await.unwrap().fehlende_unterlagen,
        4
    );
    assert_eq!(
        core.load_document(d.document_id).await.unwrap().0.version,
        2
    );
    assert!(sicherung::wiederherstellen(&zip, &ziel).is_err());
    assert!(
        db.sichern(&temp.path().join("bauprojekt.db"))
            .await
            .is_err()
    );
}
#[test]
fn falscher_hash_und_pfadmanipulation_veraendern_keine_bestehenden_daten() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original.txt");
    std::fs::write(&original, b"Erhalten").unwrap();
    for (i, name) in ["dateien/Antrag.pdf", "../original.txt"]
        .into_iter()
        .enumerate()
    {
        let m = Sicherungsmanifest {
            schema_version: 1,
            erstellt_unix: 0,
            dateien: vec![Sicherungsdatei {
                name: name.into(),
                sha256: ContentHash::of(b"anderer Inhalt"),
                bytes: 3,
            }],
        };
        let p = temp.path().join(format!("kaputt-{i}.zip"));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file(name, options).unwrap();
        zip.write_all(b"PDF").unwrap();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(&serde_json::to_vec(&m).unwrap()).unwrap();
        zip.finish().unwrap();
        assert!(sicherung::pruefen(&p).is_err());
        let ziel = temp.path().join(format!("Ziel-{i}"));
        assert!(sicherung::wiederherstellen(&p, &ziel).is_err());
        assert!(!ziel.exists());
        assert_eq!(std::fs::read(&original).unwrap(), b"Erhalten");
    }
}
