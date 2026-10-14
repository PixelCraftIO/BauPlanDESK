//! Schnittstellenprüfung: PDF-Editor → Formularmaske → DMS → Ablagewechsel.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject},
    formular::{
        editor::{EigenesPdfFeld, felder_hinzufuegen},
        pdf::{Feldart, Feldwert, felder_lesen},
        testdaten::musterformular,
    },
    lbo::LboEngine,
    license::DevelopmentLicense,
    storage::Storage,
};
use lopdf::{Document, Stream, dictionary};
use std::{collections::BTreeMap, sync::Arc};
fn original() -> Vec<u8> {
    let mut doc = Document::load_mem(&musterformular()).unwrap();
    doc.catalog_mut().unwrap().remove(b"AcroForm");
    let page = *doc.get_pages().get(&1).unwrap();
    doc.get_dictionary_mut(page).unwrap().remove(b"Annots");
    let font =
        doc.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content=doc.add_object(Stream::new(dictionary!{},b"BT /F1 18 Tf 55 790 Td (Projektformular - eigene Vorlage) Tj ET\nBT /F1 11 Tf 55 750 Td (Vorname:) Tj ET\n55 713 m 380 713 l S\nBT /F1 11 Tf 55 683 Td (Zustimmung:) Tj ET\n55 652 14 14 re S\n".to_vec()));
    doc.get_dictionary_mut(page)
        .unwrap()
        .set("Contents", content);
    doc.get_dictionary_mut(page)
        .unwrap()
        .set("Resources", dictionary! {"Font"=>dictionary!{"F1"=>font}});
    let mut bytes = vec![];
    doc.save_to(&mut bytes).unwrap();
    bytes
}
fn fields() -> Vec<EigenesPdfFeld> {
    vec![
        EigenesPdfFeld {
            name: "<Vorname>".into(),
            beschriftung: "Vorname".into(),
            art: Feldart::Text,
            seite: 1,
            x_mm: 20.0,
            y_mm: 36.0,
            breite_mm: 110.0,
            hoehe_mm: 8.0,
            schriftgroesse_pt: 11.0,
        },
        EigenesPdfFeld {
            name: "Zustimmung".into(),
            beschriftung: "Zustimmung".into(),
            art: Feldart::Ankreuzfeld,
            seite: 1,
            x_mm: 19.4,
            y_mm: 62.0,
            breite_mm: 5.0,
            hoehe_mm: 5.0,
            schriftgroesse_pt: 10.0,
        },
    ]
}
#[tokio::test]
async fn import_vorlage_ausfüllen_export_und_ablagewechsel() {
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let old = temp.path().join("ablage");
    let router = Arc::new(storage::WechselbareAblage::new(old.clone()).unwrap());
    let core = CoreServices::new(
        db.repositories(),
        router.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("pdf-test", "PDF-Test").await.unwrap();
    let project = core
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
    let bytes = original();
    let d = core
        .store_document(
            project.id,
            None,
            None,
            "Eigenes Formular.pdf",
            Bytes::from(bytes.clone()),
            &user,
        )
        .await
        .unwrap();
    let defs = fields();
    assert!(matches!(
        core.formular_vorlage_erstellen(d.document_id, 2, &defs, &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    let mut reader = user.clone();
    reader.rolle = core_lib::benutzer::Rolle::Lesen;
    assert!(matches!(
        core.formular_vorlage_erstellen(d.document_id, 1, &defs, &reader)
            .await,
        Err(CoreError::Forbidden(_))
    ));
    let template = core
        .formular_vorlage_erstellen(d.document_id, 1, &defs, &user)
        .await
        .unwrap();
    let a = core
        .formular_analysieren(template.document_id)
        .await
        .unwrap();
    assert_eq!(a.felder.len(), 2);
    let values = BTreeMap::from([
        (
            "Vorname".into(),
            Feldwert::Text("Daniel Müller Łukasz €".into()),
        ),
        ("Zustimmung".into(), Feldwert::Ja(true)),
    ]);
    assert!(
        core.formular_ausfuellen_version(template.document_id, &values, Some(5), &user)
            .await
            .is_err()
    );
    let filled = core
        .formular_ausfuellen_version(template.document_id, &values, Some(1), &user)
        .await
        .unwrap();
    let (_, filled_bytes) = core.load_document(filled.document_id).await.unwrap();
    let f = felder_lesen(&filled_bytes).unwrap();
    assert_eq!(
        f.iter().find(|f| f.name == "Vorname").unwrap().wert,
        Some(values["Vorname"].clone())
    );
    assert_eq!(core.load_document(d.document_id).await.unwrap().1, bytes);
    let src_doc = Document::load_mem(&bytes).unwrap();
    let out_doc = Document::load_mem(&filled_bytes).unwrap();
    assert_eq!(
        src_doc.get_page_content(*src_doc.get_pages().get(&1).unwrap()),
        out_doc.get_page_content(*out_doc.get_pages().get(&1).unwrap())
    );
    let docs = core.list_documents(project.id).await.unwrap();
    assert_eq!(docs.len(), 3);
    let plan = core_lib::projektablage::planen(&project, &docs).unwrap();
    for entry in &plan.dateien {
        let (_, content) = core
            .load_version(entry.dokument.document_id, entry.dokument.version)
            .await
            .unwrap();
        storage::projekt_export::unveraenderlich_schreiben(
            &temp.path().join("export"),
            &format!("{}/{}", plan.ordner, entry.relativer_pfad),
            &content,
            &entry.dokument.sha256,
        )
        .await
        .unwrap();
    }
    let new = temp.path().join("neue-ablage");
    assert_eq!(router.wechseln(&new, |_| Ok(())).await.unwrap(), 3);
    assert_eq!(
        core.load_document(filled.document_id).await.unwrap().1,
        filled_bytes
    );
    // A fresh adapter simulates reopening the application with the new root.
    let after = storage::LocalStorage::new(new, "local").unwrap();
    assert_eq!(
        after
            .load_verified(
                &core_lib::storage::StorageKey::new(filled.storage_key.clone()).unwrap(),
                &filled.sha256
            )
            .await
            .unwrap(),
        filled_bytes
    );
    if let Ok(dir) = std::env::var("BAUPROJEKT_PDF_ARTEFAKTE") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("original.pdf"), bytes).unwrap();
        std::fs::write(
            dir.join("vorlage.pdf"),
            felder_hinzufuegen(&original(), &defs).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("ausgefuellt.pdf"), filled_bytes).unwrap();
        std::fs::write(dir.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":project,"original":d,"vorlage":template,"ausgefuellt":filled,"felder":defs})).unwrap()).unwrap();
    }
}
