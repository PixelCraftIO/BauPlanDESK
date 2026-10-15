//! Kategorien sind fachliche Zuordnungen, keine Dateinamen oder Dateiversionen.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    benutzer::Rolle,
    dokumentkategorien::{Dokumentkategorie, KategorieZuordnung},
    domain::{Bundesland, NewProject},
    einreichung::{DokumentAuswahl, NeuesPaket, PaketStand},
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::ContentHash,
};
use std::{
    io::{Cursor, Read},
    sync::Arc,
};
fn zuordnung(revision: i64, kategorie: Option<&str>) -> KategorieZuordnung {
    KategorieZuordnung {
        revision,
        kategorie: kategorie.map(str::to_owned),
    }
}
#[tokio::test]
async fn kategorien_ueber_versionen_vorlagen_pakete_und_neustart() {
    let temp = tempfile::tempdir().unwrap();
    let dbpfad = temp.path().join("projekt.db");
    let db = db_sqlite::SqliteDatabase::open(&dbpfad).await.unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(temp.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("kategorien-test", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Kategorien".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.dwg",
            Bytes::from_static(b"dwg-v1"),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.dokumentkategorien(p.id).await.unwrap()[0]
            .zuordnung
            .kategorie
            .is_none()
    );
    let k = core
        .dokumentkategorie_speichern(d.document_id, zuordnung(0, Some("lageplan")), &user)
        .await
        .unwrap();
    assert_eq!(k.zuordnung.revision, 1);
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 1);
    assert!(matches!(
        core.dokumentkategorie_speichern(d.document_id, zuordnung(0, Some("ansicht")), &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert!(
        core.dokumentkategorie_speichern(d.document_id, zuordnung(1, Some("unbekannt")), &user)
            .await
            .is_err()
    );
    let v2 = core
        .store_document(
            p.id,
            Some(d.document_id),
            Some(1),
            "Plan_aktuell.dwg",
            Bytes::from_static(b"dwg-v2"),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        core.dokumentkategorien(p.id).await.unwrap()[0]
            .zuordnung
            .kategorie
            .as_deref(),
        Some("lageplan")
    );

    let neu = NeuesPaket {
        titel: "Bauantrag".into(),
        empfaenger: "Bauamt".into(),
        dokumente: vec![DokumentAuswahl {
            dokument_id: d.document_id,
            version: v2.version,
        }],
    };
    let vorgang = uuid::Uuid::new_v4();
    let paket = core
        .einreichung_erstellen_vorgang(p.id, neu.clone(), Some(vorgang), &user)
        .await
        .unwrap();
    let snap = paket
        .stand
        .kategorien
        .get(&d.document_id.0.to_string())
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(snap.name, "Lageplan");
    let (_, zip) = core.einreichung_zip(paket.stand.id).await.unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("Einreichungsstand.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let stand: PaketStand = serde_json::from_str(&manifest).unwrap();
    assert_eq!(
        &stand.kategorien[&d.document_id.0.to_string()],
        &Some(snap.clone())
    );

    let mut katalog = core.kategorienkatalog().await.unwrap();
    let alt = katalog.clone();
    katalog
        .kategorien
        .iter_mut()
        .find(|k| k.id == "lageplan")
        .unwrap()
        .name = "Amtlicher Lageplan".into();
    katalog.kategorien.push(Dokumentkategorie {
        id: "eigen_brandschutz".into(),
        name: "Brandschutz".into(),
        eltern_id: Some("gutachten".into()),
        aktiv: true,
    });
    let katalog = core
        .kategorienkatalog_speichern(katalog, &user)
        .await
        .unwrap();
    assert!(matches!(
        core.kategorienkatalog_speichern(alt, &user).await,
        Err(CoreError::VersionConflict { .. })
    ));
    core.dokumentkategorie_speichern(
        d.document_id,
        zuordnung(1, Some("eigen_brandschutz")),
        &user,
    )
    .await
    .unwrap();
    let wiederholt = core
        .einreichung_erstellen_vorgang(p.id, neu, Some(vorgang), &user)
        .await
        .unwrap();
    assert_eq!(wiederholt.stand.kategorien, paket.stand.kategorien);
    let bestaetigt = core
        .einreichung_bestaetigen(
            paket.stand.id,
            1,
            d.created_at.date_naive(),
            "Portal".into(),
            &user,
        )
        .await
        .unwrap();
    let vergleich = core.einreichungs_vergleich(p.id, "Bauamt").await.unwrap();
    assert_eq!(vergleich[0].kategorie.as_ref().unwrap().name, "Brandschutz");
    assert_eq!(
        vergleich[0].eingereichte_kategorie.as_ref().unwrap().name,
        "Lageplan"
    );
    assert_eq!(
        core.einreichung_zip(paket.stand.id)
            .await
            .unwrap()
            .0
            .stand
            .kategorien,
        bestaetigt.stand.kategorien
    );
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 2);

    let pdf = core_lib::formular::testdaten::musterformular();
    let hash = ContentHash::of(&pdf);
    core.vorlagenkategorie_speichern(&hash, zuordnung(0, Some("baubeschreibung")), &user)
        .await
        .unwrap();
    let formular = core
        .formular_aus_bibliothek(p.id, "Muster.pdf", &pdf, &Default::default(), &user)
        .await
        .unwrap();
    let kategorien = core.dokumentkategorien(p.id).await.unwrap();
    assert_eq!(
        kategorien
            .iter()
            .find(|k| k.dokument_id == formular.document_id)
            .unwrap()
            .zuordnung
            .kategorie
            .as_deref(),
        Some("baubeschreibung")
    );
    core.vorlagenkategorie_speichern(&hash, zuordnung(1, Some("bauantrag")), &user)
        .await
        .unwrap();
    core.formular_im_projekt_speichern(formular.document_id, 1, &Default::default(), &user)
        .await
        .unwrap();
    assert_eq!(
        core.dokumentkategorien(p.id)
            .await
            .unwrap()
            .iter()
            .find(|k| k.dokument_id == formular.document_id)
            .unwrap()
            .zuordnung
            .kategorie
            .as_deref(),
        Some("baubeschreibung")
    );
    // Auch explizit entfernte Kategorien werden nicht aus der Vorlage zurückgeholt.
    core.dokumentkategorie_speichern(formular.document_id, zuordnung(0, None), &user)
        .await
        .unwrap();
    core.formular_im_projekt_speichern(formular.document_id, 2, &Default::default(), &user)
        .await
        .unwrap();
    assert!(
        core.dokumentkategorien(p.id)
            .await
            .unwrap()
            .iter()
            .find(|k| k.dokument_id == formular.document_id)
            .unwrap()
            .zuordnung
            .kategorie
            .is_none()
    );
    let berechnung = core
        .store_document_kategorisiert(
            p.id,
            "Nachweis.xlsx",
            Bytes::from_static(b"nachweis"),
            Some("grz".into()),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        core.kategoriesnapshots(p.id).await.unwrap()[&berechnung.document_id.0.to_string()]
            .as_ref()
            .unwrap()
            .hauptkategorie
            .as_deref(),
        Some("Berechnung")
    );

    let mut archiv = katalog.clone();
    archiv
        .kategorien
        .iter_mut()
        .find(|k| k.id == "eigen_brandschutz")
        .unwrap()
        .aktiv = false;
    core.kategorienkatalog_speichern(archiv, &user)
        .await
        .unwrap();
    assert!(
        core.dokumentkategorie_speichern(
            formular.document_id,
            zuordnung(1, Some("eigen_brandschutz")),
            &user
        )
        .await
        .is_err()
    );
    assert_eq!(
        core.kategoriesnapshots(p.id).await.unwrap()[&d.document_id.0.to_string()]
            .as_ref()
            .unwrap()
            .name,
        "Brandschutz"
    );
    // Leseberechtigung und fremde Sperren bleiben wirksam.
    let mut leser = user.clone();
    leser.rolle = Rolle::Lesen;
    assert!(matches!(
        core.dokumentkategorie_speichern(d.document_id, zuordnung(2, None), &leser)
            .await,
        Err(CoreError::Forbidden(_))
    ));
    let anderer = core.anmelden("anderer", "Kollege").await.unwrap();
    core.repos
        .documents
        .lock(d.document_id, p.id, &anderer)
        .await
        .unwrap();
    assert!(matches!(
        core.dokumentkategorie_speichern(d.document_id, zuordnung(2, None), &user)
            .await,
        Err(CoreError::Gesperrt { .. })
    ));
    core.repos
        .documents
        .unlock(d.document_id, None)
        .await
        .unwrap();
    let db2 = db_sqlite::SqliteDatabase::open(&dbpfad).await.unwrap();
    let core2 = CoreServices::new(
        db2.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    assert_eq!(core2.kategorienkatalog().await.unwrap().revision, 2);
    assert_eq!(
        core2.kategoriesnapshots(p.id).await.unwrap()[&d.document_id.0.to_string()]
            .as_ref()
            .unwrap()
            .id,
        "eigen_brandschutz"
    );
    assert_eq!(
        core2.einreichungen(p.id).await.unwrap()[0].stand.kategorien,
        paket.stand.kategorien
    );
    // Vorlagenkopien erhalten vorhandene Vorgaben; ein schon kategorisiertes Ziel bleibt bestehen.
    let ziel = ContentHash::of(b"bearbeitete Vorlage");
    core2
        .vorlagenkategorie_kopieren(&hash, &ziel, &user)
        .await
        .unwrap();
    assert_eq!(
        core2
            .vorlagenkategorie(&ziel)
            .await
            .unwrap()
            .kategorie
            .as_deref(),
        Some("bauantrag")
    );
    core2
        .vorlagenkategorie_speichern(&ziel, zuordnung(1, Some("lageplan")), &user)
        .await
        .unwrap();
    core2
        .vorlagenkategorie_kopieren(&hash, &ziel, &user)
        .await
        .unwrap();
    assert_eq!(
        core2
            .vorlagenkategorie(&ziel)
            .await
            .unwrap()
            .kategorie
            .as_deref(),
        Some("lageplan")
    );
    let archivierte_vorlage = ContentHash::of(b"archivierte Vorgabe");
    core2
        .dokumentkategorie_als_vorlage(d.document_id, &archivierte_vorlage, &user)
        .await
        .unwrap();
    assert_eq!(
        core2
            .vorlagenkategorie(&archivierte_vorlage)
            .await
            .unwrap()
            .kategorie
            .as_deref(),
        Some("eigen_brandschutz")
    );
    // Kopieren einer bereits zugeordneten archivierten Kategorie erhält die Referenz.
    let kopie = ContentHash::of(b"archivierte Kopie");
    core2
        .vorlagenkategorie_kopieren(&archivierte_vorlage, &kopie, &user)
        .await
        .unwrap();
    assert_eq!(
        core2
            .vorlagenkategorie(&kopie)
            .await
            .unwrap()
            .kategorie
            .as_deref(),
        Some("eigen_brandschutz")
    );
    let mut alt = serde_json::to_value(&paket.stand).unwrap();
    alt.as_object_mut().unwrap().remove("kategorien");
    assert!(
        serde_json::from_value::<PaketStand>(alt)
            .unwrap()
            .kategorien
            .is_empty()
    );
    // Reale Vertragsdaten für den produktiven UI-Test (nur der IPC-Transport wird ersetzt).
    if let Ok(ziel) = std::env::var("BAUPROJEKT_KATEGORIEN_ARTEFAKTE") {
        let root = std::path::PathBuf::from(ziel);
        std::fs::create_dir_all(&root).unwrap();
        let wf = serde_json::json!({"projekt":p,"katalog":core_lib::dokumentkategorien::KategorienKatalog::default(),"dokumente":[v2,berechnung],"paket":paket,"vorlagen":[{"name":"Muster.pdf","sha256":hash,"size":pdf.len(),"felder":3}]});
        std::fs::write(
            root.join("workflow.json"),
            serde_json::to_vec_pretty(&wf).unwrap(),
        )
        .unwrap();
    }
    let log = core2
        .repos
        .protokoll
        .list_for_project(p.id, 100)
        .await
        .unwrap();
    assert!(
        log.iter()
            .any(|e| e.eintrag.aktion == core_lib::domain::Aktion::DokumentKategorieGeaendert)
    );
}

#[tokio::test]
async fn sqlite_einstellungen_sind_auch_bei_gleichzeitiger_bearbeitung_atomisch() {
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let r = db.repositories();
    let (a, b) = tokio::join!(
        r.lizenz.einstellung_vergleichen("katalog", None, "A"),
        r.lizenz.einstellung_vergleichen("katalog", None, "B")
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let alt = r.lizenz.einstellung("katalog").await.unwrap().unwrap();
    let (a, b) = tokio::join!(
        r.lizenz.einstellung_vergleichen("katalog", Some(&alt), "C"),
        r.lizenz.einstellung_vergleichen("katalog", Some(&alt), "D")
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
}
