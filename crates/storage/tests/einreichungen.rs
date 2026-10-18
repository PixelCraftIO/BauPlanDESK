//! Reale DMS-, Arbeitskopien-, ZIP- und Einreichungsgrenzen ohne Desktop.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    benutzer::Rolle,
    domain::{Bundesland, NewProject},
    einreichung::{DokumentAuswahl, NeuesPaket, PaketStand, VergleichsStatus},
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use std::{
    io::{Cursor, Read},
    sync::Arc,
};
#[tokio::test]
async fn dateiarbeit_paketstaende_nachreichungen_und_neustart() {
    let temp = tempfile::tempdir().unwrap();
    let dbpath = temp.path().join("projekt.db");
    let db = db_sqlite::SqliteDatabase::open(&dbpath).await.unwrap();
    let ablage = Arc::new(storage::LocalStorage::new(temp.path().join("Ablage"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        ablage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("chronik", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Einreichungsprüfung".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.dwg",
            Bytes::from_static(b"DWG Stand 1"),
            &user,
        )
        .await
        .unwrap();
    let excel = core
        .store_document(
            p.id,
            None,
            None,
            "Berechnung.xlsx",
            Bytes::from_static(b"Excel Stand 1"),
            &user,
        )
        .await
        .unwrap();
    let paket = |title: &str, ds: Vec<&core_lib::domain::DocumentVersion>| NeuesPaket {
        titel: title.into(),
        empfaenger: "Bauamt".into(),
        dokumente: ds
            .into_iter()
            .map(|d| DokumentAuswahl {
                dokument_id: d.document_id,
                version: d.version,
            })
            .collect(),
    };
    assert!(
        core.einreichung_erstellen(p.id, paket("Leer", vec![]), &user)
            .await
            .is_err()
    );
    let foreign = core
        .create_project(
            NewProject {
                name: "Anderes Projekt".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.einreichung_erstellen(foreign.id, paket("Fremde Unterlagen", vec![&plan]), &user)
            .await
            .is_err()
    );
    core.checkliste_anwenden(
        p.id,
        core_lib::plattform::ablauf::Checkliste::standard().unwrap(),
        &user,
    )
    .await
    .unwrap();
    let vorgang = uuid::Uuid::new_v4();
    let (a, b) = tokio::join!(
        core.einreichung_erstellen_vorgang(
            p.id,
            paket("Bauantrag", vec![&plan, &excel]),
            Some(vorgang),
            &user
        ),
        core.einreichung_erstellen_vorgang(
            p.id,
            paket("Bauantrag", vec![&plan, &excel]),
            Some(vorgang),
            &user
        )
    );
    let erster = a.unwrap();
    let parallel = b.unwrap();
    assert_eq!(erster.zip_key, parallel.zip_key);
    let wiederholt = core
        .einreichung_erstellen_vorgang(
            p.id,
            paket("Bauantrag", vec![&plan, &excel]),
            Some(vorgang),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(erster.stand.id, wiederholt.stand.id);
    assert_eq!(core.einreichungen(p.id).await.unwrap().len(), 1);
    assert!(
        core.einreichung_erstellen_vorgang(
            p.id,
            paket("Anderer Titel", vec![&plan]),
            Some(vorgang),
            &user
        )
        .await
        .is_err()
    );
    assert_eq!(
        erster.stand.pruefstand.as_ref().unwrap().unterlagen.len(),
        6
    );
    assert_eq!(
        erster
            .stand
            .pruefstand
            .as_ref()
            .unwrap()
            .nicht_enthaltene_pflichtunterlagen
            .len(),
        4
    );
    let originalzip = core.einreichung_zip(erster.stand.id).await.unwrap().1;
    assert!(
        core.einreichungs_vergleich(p.id, "Bauamt")
            .await
            .unwrap()
            .iter()
            .all(|v| v.status == VergleichsStatus::Neu)
    );
    // Pakete werden nicht mehr bestätigt; „eingereicht“ kommt nur aus der Verfahrensakte.
    let gesperrt = core
        .einreichung_bestaetigen(
            erster.stand.id,
            1,
            "2026-01-01".parse().unwrap(),
            "".into(),
            &user,
        )
        .await
        .unwrap_err();
    assert!(gesperrt.to_string().contains("Externe Einreichung"));
    assert!(
        core.einreichungen(p.id).await.unwrap()[0]
            .einreichung
            .is_none()
    );
    assert!(
        core.einreichung_bestaetigen_altbestand(
            erster.stand.id,
            1,
            "2099-01-01".parse().unwrap(),
            "".into(),
            &user
        )
        .await
        .is_err()
    );
    core.einreichung_bestaetigen_altbestand(
        erster.stand.id,
        1,
        "2026-01-01".parse().unwrap(),
        "Bauportal AZ 123".into(),
        &user,
    )
    .await
    .unwrap();
    assert!(
        core.einreichungs_vergleich(p.id, "Bauamt")
            .await
            .unwrap()
            .iter()
            .all(|v| v.status == VergleichsStatus::BereitsEingereicht)
    );
    assert_eq!(
        core.externe_datei(plan.document_id, 1).await.unwrap().1,
        b"DWG Stand 1".as_slice()
    );
    let root = temp.path().join("Arbeitskopien");
    let copy = storage::arbeitskopie::erstellen(&root, plan.clone(), b"DWG Stand 1")
        .await
        .unwrap();
    assert!(!copy.geaendert);
    std::fs::write(&copy.pfad, b"DWG Korrektur 2").unwrap();
    assert!(storage::arbeitskopie::auflisten(&root, p.id).await.unwrap()[0].geaendert);
    let data = storage::arbeitskopie::inhalt(&copy.pfad).await.unwrap();
    let plan2 = core
        .store_document(
            p.id,
            Some(plan.document_id),
            Some(1),
            &plan.file_name,
            data.into(),
            &user,
        )
        .await
        .unwrap();
    storage::arbeitskopie::basis_aktualisieren(&root, copy.stand.id, plan2.clone())
        .await
        .unwrap();
    let reopened_copy = storage::arbeitskopie::laden(&root, copy.stand.id)
        .await
        .unwrap();
    assert!(!reopened_copy.geaendert);
    assert_eq!(reopened_copy.stand.dokument.version, 2);
    std::fs::write(&copy.pfad, b"Weitere lokale Aenderung").unwrap();
    assert!(matches!(
        core.store_document(
            p.id,
            Some(plan.document_id),
            Some(1),
            &plan.file_name,
            Bytes::from_static(b"veraltete Kopie"),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert_eq!(
        std::fs::read(&copy.pfad).unwrap(),
        b"Weitere lokale Aenderung"
    );
    let foto = core
        .store_document(
            p.id,
            None,
            None,
            "Baustelle.jpg",
            Bytes::from_static(b"Foto"),
            &user,
        )
        .await
        .unwrap();
    let vergleich = core.einreichungs_vergleich(p.id, "Bauamt").await.unwrap();
    assert_eq!(
        vergleich
            .iter()
            .find(|v| v.aktuell.document_id == plan.document_id)
            .unwrap()
            .status,
        VergleichsStatus::Geaendert
    );
    assert_eq!(
        vergleich
            .iter()
            .find(|v| v.aktuell.document_id == foto.document_id)
            .unwrap()
            .status,
        VergleichsStatus::Neu
    );
    let zweiter = core
        .einreichung_erstellen(
            p.id,
            paket("Korrektur und Nachreichung", vec![&plan2, &foto]),
            &user,
        )
        .await
        .unwrap();
    // Ein später exportiertes, unbestätigtes Paket verändert den Amtsstand nicht.
    assert_eq!(
        core.einreichungs_vergleich(p.id, "Bauamt")
            .await
            .unwrap()
            .iter()
            .find(|v| v.aktuell.document_id == plan.document_id)
            .unwrap()
            .zuletzt_eingereicht
            .as_ref()
            .unwrap()
            .version,
        1
    );
    core.einreichung_bestaetigen_altbestand(
        zweiter.stand.id,
        1,
        "2026-01-02".parse().unwrap(),
        "Korrigierter Plan".into(),
        &user,
    )
    .await
    .unwrap();
    let vergleich = core.einreichungs_vergleich(p.id, "Bauamt").await.unwrap();
    assert!(
        vergleich
            .iter()
            .all(|v| v.status == VergleichsStatus::BereitsEingereicht)
    );
    // Teilweise Nachreichung erhält die zuvor eingereichte Excel-Version.
    assert_eq!(
        vergleich
            .iter()
            .find(|v| v.aktuell.document_id == excel.document_id)
            .unwrap()
            .paket_id,
        Some(erster.stand.id)
    );
    assert!(
        core.einreichungs_vergleich(p.id, "Andere Behörde")
            .await
            .unwrap()
            .iter()
            .all(|v| v.status == VergleichsStatus::Neu)
    );
    let plan3 = core
        .store_document(
            p.id,
            Some(plan.document_id),
            Some(2),
            &plan.file_name,
            Bytes::from_static(b"DWG Korrektur 2"),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(plan3.version, 3);
    assert_eq!(
        core.einreichungs_vergleich(p.id, "Bauamt")
            .await
            .unwrap()
            .iter()
            .find(|v| v.aktuell.document_id == plan.document_id)
            .unwrap()
            .status,
        VergleichsStatus::GleicherInhalt
    );
    assert_eq!(
        core_lib::einreichung::vergleichen(
            p.id,
            "Bauamt",
            vec![plan.clone()],
            &core.einreichungen(p.id).await.unwrap()
        )[0]
        .status,
        VergleichsStatus::Aelter
    );
    let historisch = core
        .einreichung_erstellen(
            p.id,
            paket("Älterer Versand nachgetragen", vec![&plan]),
            &user,
        )
        .await
        .unwrap();
    core.einreichung_bestaetigen_altbestand(
        historisch.stand.id,
        1,
        "2025-12-01".parse().unwrap(),
        "Historisch".into(),
        &user,
    )
    .await
    .unwrap();
    assert_eq!(
        core.einreichungs_vergleich(p.id, "Bauamt")
            .await
            .unwrap()
            .iter()
            .find(|v| v.aktuell.document_id == plan.document_id)
            .unwrap()
            .zuletzt_eingereicht
            .as_ref()
            .unwrap()
            .version,
        2
    );
    assert_eq!(
        core.einreichung_zip(erster.stand.id).await.unwrap().1,
        originalzip
    );
    let mut zip = zip::ZipArchive::new(Cursor::new(&originalzip)).unwrap();
    let mut manifest = String::new();
    zip.by_name("Einreichungsstand.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let stand: PaketStand = serde_json::from_str(&manifest).unwrap();
    assert_eq!(stand.dokumente[0].version, 1);
    let mut bytes = Vec::new();
    zip.by_name(&format!("Unterlagen/{}/v1-Plan.dwg", plan.document_id.0))
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"DWG Stand 1");
    assert!(matches!(
        core.einreichung_bestaetigen_altbestand(
            erster.stand.id,
            1,
            "2026-01-03".parse().unwrap(),
            "".into(),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert!(
        core.einreichung_bestaetigen_altbestand(
            erster.stand.id,
            2,
            "2026-01-03".parse().unwrap(),
            "".into(),
            &user
        )
        .await
        .is_err()
    );
    assert!(
        core.einreichung_erstellen(p.id, paket("Doppelt", vec![&plan, &plan]), &user)
            .await
            .is_err()
    );
    let mut reader = user.clone();
    reader.rolle = Rolle::Lesen;
    assert!(matches!(
        core.einreichung_erstellen(p.id, paket("Unzulässig", vec![&plan]), &reader)
            .await,
        Err(CoreError::Forbidden(_))
    ));
    let reopened = db_sqlite::SqliteDatabase::open(&dbpath).await.unwrap();
    let after = CoreServices::new(
        reopened.repositories(),
        ablage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    assert_eq!(after.einreichungen(p.id).await.unwrap().len(), 3);
    assert_eq!(
        after.einreichung_zip(erster.stand.id).await.unwrap().1,
        originalzip
    );
    assert_eq!(
        after.load_version(plan.document_id, 1).await.unwrap().1,
        b"DWG Stand 1".as_slice()
    );
    if let Ok(path) = std::env::var("BAUPROJEKT_EINREICHUNG_ARTEFAKTE") {
        // Ein weiterer Dateistand bleibt gegenüber dem Amtsstand als geändert sichtbar.
        let plan4 = after
            .store_document(
                p.id,
                Some(plan.document_id),
                Some(3),
                &plan.file_name,
                Bytes::from_static(b"DWG neue Korrektur 4"),
                &user,
            )
            .await
            .unwrap();
        let path = std::path::Path::new(&path);
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"plan1":plan,"plan2":plan2,"plan3":plan3,"plan4":plan4,"excel":excel,"foto":foto,"chronik":after.einreichungen(p.id).await.unwrap(),"vergleich":after.einreichungs_vergleich(p.id,"Bauamt").await.unwrap()})).unwrap()).unwrap();
        std::fs::write(path.join("erster-bauantrag.zip"), originalzip).unwrap();
    }
}
