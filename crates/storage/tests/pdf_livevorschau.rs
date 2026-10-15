//! Echte PDFs für Live-Vorschau und mehrzeilige Feldwerte; Vorschauen schreiben keine DMS-Version.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    formular::{
        editor::{EigenesPdfFeld, felder_hinzufuegen},
        pdf::{Feldart, Feldwert, befuellen, felder_lesen},
        testdaten::musterformular,
    },
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use lopdf::{Document, Stream, dictionary};
use std::{collections::BTreeMap, sync::Arc};
fn original() -> Vec<u8> {
    let mut d = Document::load_mem(&musterformular()).unwrap();
    d.catalog_mut().unwrap().remove(b"AcroForm");
    let page = *d.get_pages().get(&1).unwrap();
    let parent = d
        .get_dictionary(page)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let font =
        d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let resources = dictionary! {"Font"=>dictionary!{"F1"=>font}};
    let text = |title: &str, label: &str| {
        Stream::new(
            dictionary! {},
            format!(
                "BT /F1 18 Tf 55 790 Td ({title}) Tj ET\nBT /F1 11 Tf 55 735 Td ({label}) Tj ET\n"
            )
            .into_bytes(),
        )
    };
    let c = d.add_object(text("Projektformular - Livevorschau", "Vorname"));
    let p = d.get_dictionary_mut(page).unwrap();
    p.remove(b"Annots");
    p.set("Resources", resources.clone());
    p.set("Contents", c);
    let c = d.add_object(text("Projektformular - Anschrift", "Anschrift"));
    let second=d.add_object(dictionary!{"Type"=>"Page","Parent"=>parent,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],"Resources"=>resources,"Contents"=>c});
    let pages = d.get_dictionary_mut(parent).unwrap();
    let mut kids = pages.get(b"Kids").unwrap().as_array().unwrap().clone();
    kids.push(second.into());
    pages.set("Kids", kids);
    pages.set("Count", 2);
    let mut bytes = Vec::new();
    d.save_to(&mut bytes).unwrap();
    bytes
}
fn feld(name: &str, seite: u32, y: f32, mehrzeilig: bool) -> EigenesPdfFeld {
    EigenesPdfFeld {
        name: name.into(),
        beschriftung: name.into(),
        art: Feldart::Text,
        seite,
        x_mm: 20.,
        y_mm: y,
        breite_mm: 90.,
        hoehe_mm: if mehrzeilig { 40. } else { 9. },
        schriftgroesse_pt: 11.,
        mehrzeilig,
    }
}
#[tokio::test]
async fn vorschau_mehrzeiliger_werte_bleibt_fluechtig_und_entspricht_der_ausgabe() {
    let original = original();
    let mut check = feld("Zustimmung", 2, 95., false);
    check.art = Feldart::Ankreuzfeld;
    check.breite_mm = 5.;
    check.hoehe_mm = 5.;
    let pdf = felder_hinzufuegen(
        &original,
        &[
            feld("Vorname", 1, 45., false),
            feld("Anschrift", 2, 45., true),
            check,
        ],
    )
    .unwrap();
    let felder = felder_lesen(&pdf).unwrap();
    assert!(
        felder
            .iter()
            .find(|f| f.name == "Anschrift")
            .unwrap()
            .mehrzeilig
    );
    assert!(!felder[0].mehrzeilig);
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "PDF-Liveprojekt".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let quelle = core
        .store_document(
            p.id,
            None,
            None,
            "Liveformular.pdf",
            Bytes::from(pdf.clone()),
            &user,
        )
        .await
        .unwrap();
    let a = BTreeMap::from([
        ("Vorname".into(), Feldwert::Text("Daniel".into())),
        (
            "Anschrift".into(),
            Feldwert::Text("Daniel Müller\nMusterstraße 10\n10115 Berlin".into()),
        ),
        ("Zustimmung".into(), Feldwert::Ja(true)),
    ]);
    let b = BTreeMap::from([
        ("Vorname".into(), Feldwert::Text("Anna".into())),
        (
            "Anschrift".into(),
            Feldwert::Text("Anna Beispiel\nPlanerweg 12\n80331 München".into()),
        ),
        ("Zustimmung".into(), Feldwert::Ja(false)),
    ]);
    let daniel = befuellen(&pdf, &a).unwrap();
    let anna = befuellen(&pdf, &b).unwrap();
    for (bytes, werte) in [(&daniel, &a), (&anna, &b)] {
        let gefuellt = felder_lesen(bytes).unwrap();
        for f in gefuellt {
            assert_eq!(f.wert.as_ref(), werte.get(&f.name));
        }
    }
    assert_eq!(
        core.document_history(quelle.document_id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        core.load_version(quelle.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        pdf
    );
    let gespeichert = core
        .formular_im_projekt_speichern(quelle.document_id, 1, &a, &user)
        .await
        .unwrap();
    assert_eq!(gespeichert.version, 2);
    let bytes = core.load_version(quelle.document_id, 2).await.unwrap().1;
    assert_eq!(
        felder_lesen(&bytes).unwrap(),
        felder_lesen(&daniel).unwrap()
    );
    assert_eq!(
        core.load_version(quelle.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        pdf
    );
    if let Ok(ziel) = std::env::var("BAUPROJEKT_PDF_LIVE_ARTEFAKTE") {
        let ziel = std::path::PathBuf::from(ziel);
        std::fs::create_dir_all(&ziel).unwrap();
        for (name, bytes) in [("original", &pdf), ("daniel", &daniel), ("anna", &anna)] {
            std::fs::write(ziel.join(format!("{name}.pdf")), bytes).unwrap();
        }
        std::fs::write(
            ziel.join("workflow.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"dokument":quelle,"felder":felder,"seiten":core_lib::formular::editor::seiten(&pdf).unwrap(),"daniel":a,"anna":b}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
