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
/// Ablauf der Einzelplatzsicherung: Datenbankreferenzen werden über den
/// Storage-Schlüssel mit `data/` abgeglichen, `tmp/` bleibt außen vor, und ein
/// älterer, nicht portabler Schlüssel blockiert die Sicherung nicht mehr.
#[tokio::test]
async fn einzelplatzsicherung_mit_dokument_formularvorlage_und_altem_schluessel() {
    use core_lib::{
        domain::{DocumentId, DocumentVersion},
        storage::{Storage, StorageKey},
    };
    let temp = tempfile::tempdir().unwrap();
    let dms = temp.path().join("DMS");
    let db = db_sqlite::SqliteDatabase::open(&temp.path().join("live.db"))
        .await
        .unwrap();
    let ablage = Arc::new(storage::LocalStorage::new(dms.clone(), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        ablage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("sicherung", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Einzelplatz".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let formular = core
        .formular_aus_bibliothek(
            p.id,
            "Antrag.pdf",
            &musterformular(),
            &BTreeMap::from([("Bauherr_Name".into(), Feldwert::Text("Daniel".into()))]),
            &user,
        )
        .await
        .unwrap();
    // Neue Namen sind portabel.
    let neu = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan M 1:500.pdf.",
            bytes::Bytes::from_static(b"%PDF-neu"),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(neu.file_name, "Lageplan M 1_500.pdf");
    // Älterer Datenstand: Schlüssel mit Doppelpunkt und Punkt am Ende.
    let alt_key = format!(
        "projekte/{}/dokumente/{}/{}-Ansicht 1:100.",
        p.id,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4()
    );
    let meta = ablage
        .save(
            &StorageKey::new(alt_key.clone()).unwrap(),
            bytes::Bytes::from_static(b"%PDF-alt"),
        )
        .await
        .unwrap();
    let alt = core
        .repos
        .documents
        .add_version(
            DocumentVersion {
                document_id: DocumentId::default(),
                project_id: p.id,
                version: 0,
                file_name: "Ansicht 1:100.".into(),
                storage_key: alt_key.clone(),
                sha256: meta.sha256,
                size_bytes: meta.size_bytes,
                created_by: "Altbestand".into(),
                created_at: chrono::Utc::now(),
            },
            None,
        )
        .await
        .unwrap();
    // Laufender Speichervorgang eines Kollegen.
    std::fs::write(dms.join("tmp/laufend"), b"halb geschrieben").unwrap();

    let snapshot = temp.path().join("bauprojekt.db");
    db.sichern(&snapshot).await.unwrap();
    let kopie = db_sqlite::SqliteDatabase::open(&snapshot).await.unwrap();
    let erwartet = sicherung::dms_referenzen(&kopie.repositories())
        .await
        .unwrap();
    kopie.schliessen().await;
    assert!(erwartet.contains_key(&alt_key));
    assert!(erwartet.keys().any(|k| k.starts_with("formularvorlagen/")));
    let mut quellen = vec![Sicherungsquelle {
        name: "bauprojekt.db".into(),
        pfad: snapshot.clone(),
        erwartet: None,
    }];
    let dateien = sicherung::dateiablage(&dms, "dateien", erwartet.clone()).unwrap();
    assert!(dateien.iter().all(|q| !q.name.starts_with("dateien/tmp/")));
    assert_eq!(
        dateien.iter().filter(|q| q.erwartet.is_some()).count(),
        erwartet.len()
    );
    quellen.extend(dateien);
    let zip = temp.path().join("einzelplatz.zip");
    let manifest = sicherung::erstellen(&zip, &quellen).unwrap();
    assert_eq!(manifest.schema_version, 2);
    assert!(
        manifest
            .dateien
            .iter()
            .any(|f| f.name.ends_with("Ansicht 1%3A100%2E"))
    );
    sicherung::pruefen(&zip).unwrap();

    // Eine fehlende Referenz macht die Sicherung unvollständig.
    let mut fehlend = erwartet.clone();
    fehlend.insert("projekte/fehlt.pdf".into(), ContentHash::of(b"x"));
    assert!(
        sicherung::dateiablage(&dms, "dateien", fehlend)
            .unwrap_err()
            .to_string()
            .contains("Datenbankreferenzen fehlen")
    );

    let ziel = temp.path().join("Profil");
    sicherung::wiederherstellen(&zip, &ziel).unwrap();
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
        core2
            .load_version(alt.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        b"%PDF-alt"
    );
    assert_eq!(
        core2
            .load_version(neu.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        b"%PDF-neu"
    );
    let i = core2
        .formular_instanz(formular.document_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(i.feldwerte["Bauherr_Name"], Feldwert::Text("Daniel".into()));
    assert!(!ziel.join("dateien/tmp/laufend").exists());
}

#[test]
fn archivnamen_sind_portabel_und_umkehrbar() {
    for name in [
        "dateien/data/projekte/a/b-Lageplan 1:500.pdf",
        "dateien/data/x/Was? <neu>|*\".pdf",
        "dateien/data/x/Ende. ",
        "dateien/data/x/100%.pdf",
        "Formularvorlagen/CON.pdf",
        "einstellungen/config.json",
    ] {
        let a = sicherung::archivname(name);
        assert!(!a.contains([':', '?', '<', '>', '|', '*', '"']), "{a}");
        assert!(!a.split('/').any(|s| s.ends_with([' ', '.'])), "{a}");
        assert_eq!(sicherung::originalname(&a).unwrap(), name);
    }
    assert_eq!(
        sicherung::archivname("einstellungen/config.json"),
        "einstellungen/config.json"
    );
    for kaputt in ["a/%2F/b", "a/%2E%2E/b", "a/%zz", "a/%4", "a/%5C/b"] {
        assert!(sicherung::originalname(kaputt).is_err(), "{kaputt}");
    }
}
