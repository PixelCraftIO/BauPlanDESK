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

#[tokio::test]
async fn formularvorgang_hat_eine_identitaet_und_unveraenderliche_quellen() {
    use core_lib::{formular::instanz::FormularBezug, kunden::Kundendaten};
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().join("Ablage"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("quellen", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Quellenwahl".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut kunden = Vec::new();
    for name in ["Erika", "Daniel"] {
        kunden.push(
            core.kunde_anlegen(
                Kundendaten {
                    vorname: name.into(),
                    familienname: "Muster".into(),
                    ..Default::default()
                },
                &user,
            )
            .await
            .unwrap(),
        );
    }
    core.bauherren_setzen(
        p.id,
        kunden.iter().map(|k| k.id).collect(),
        p.version,
        &user,
    )
    .await
    .unwrap();
    let offen = core
        .formular_quellen(p.id, Default::default())
        .await
        .unwrap();
    assert!(offen.datenstand.werte.wert("bauherr.name").is_none());
    assert_eq!(offen.bauherren.len(), 2);
    let q = core
        .formular_quellen(
            p.id,
            FormularBezug {
                bauherr_id: Some(kunden[1].id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(q.datenstand.werte.wert("bauherr.vorname"), Some("Daniel"));
    let source = musterformular();
    let werte = BTreeMap::from([(
        "Bauherr_Name".into(),
        Feldwert::Text("Daniel Muster".into()),
    )]);
    let vorgang = uuid::Uuid::new_v4();
    let d = core
        .formular_aus_bibliothek_mit_quellen(
            p.id,
            "Antrag.pdf",
            &source,
            &werte,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang: Some(vorgang),
                datenstand: Some(q.datenstand.clone()),
            },
            &user,
        )
        .await
        .unwrap();
    let replay = core
        .formular_aus_bibliothek_mit_quellen(
            p.id,
            "Antrag.pdf",
            &source,
            &werte,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang: Some(vorgang),
                datenstand: Some(q.datenstand.clone()),
            },
            &user,
        )
        .await
        .unwrap();
    assert_eq!(d, replay);
    assert_eq!(
        core.repos
            .protokoll
            .list_for_project(p.id, 1000)
            .await
            .unwrap()
            .iter()
            .filter(|e| e.eintrag.aktion == core_lib::domain::Aktion::FormularAusgefuellt)
            .count(),
        1
    );
    assert_eq!(core.list_documents(p.id).await.unwrap().len(), 1);
    let instanz = core
        .formular_instanz(d.document_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        instanz.quellen.datenstand.bezug.bauherr_id,
        Some(kunden[1].id)
    );
    assert_eq!(instanz.feldwerte, werte);
    let (_, stored) = core.load_document(d.document_id).await.unwrap();
    assert!(stored.len() > 100);
    let changed = BTreeMap::from([("Bauherr_Name".into(), Feldwert::Text("Anderer Text".into()))]);
    assert!(
        core.formular_aus_bibliothek_mit_quellen(
            p.id,
            "Antrag.pdf",
            &source,
            &changed,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang: Some(vorgang),
                datenstand: Some(q.datenstand.clone())
            },
            &user
        )
        .await
        .is_err()
    );
    let mut renamed = kunden[1].daten.clone();
    renamed.vorname = "Geändert".into();
    core.repos
        .kunden
        .kunde_aendern(kunden[1].id, renamed, kunden[1].version)
        .await
        .unwrap();
    assert!(
        core.formular_aus_bibliothek_mit_quellen(
            p.id,
            "Antrag.pdf",
            &source,
            &werte,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang: Some(uuid::Uuid::new_v4()),
                datenstand: Some(q.datenstand)
            },
            &user
        )
        .await
        .is_err()
    );
    // Die alte PDF und ihr protokollierter Datenstand ändern sich nicht mit dem Kundenstamm.
    assert_eq!(
        core.formular_instanz(d.document_id, 1)
            .await
            .unwrap()
            .unwrap()
            .quellen
            .datenstand
            .werte
            .wert("bauherr.vorname"),
        Some("Daniel")
    );
    let d2 = core
        .formular_im_projekt_speichern(d.document_id, 1, &changed, &user)
        .await
        .unwrap();
    assert_eq!(d2.document_id, d.document_id);
    assert_eq!(d2.version, 2);
    let zweite = core
        .formular_instanz(d.document_id, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(zweite.vorlagen_hash, instanz.vorlagen_hash);
    assert_eq!(zweite.vorlagen_storage_key, instanz.vorlagen_storage_key);
    assert_eq!(
        core.storage
            .load_verified(
                &core_lib::storage::StorageKey::new(zweite.vorlagen_storage_key).unwrap(),
                &zweite.vorlagen_hash
            )
            .await
            .unwrap()
            .as_ref(),
        source.as_slice()
    );
}

/// Veraltete Formularwerte werden in Analyse, Formularinstanz und Abgabe
/// gemeldet; Zuordnung nach Bundesland wird gespeichert; ein entfernter
/// Bauherr verhindert das Öffnen nicht.
#[tokio::test]
async fn veraltete_werte_zuordnung_und_entfernter_bauherr() {
    use core_lib::{
        formular::{Formularkatalog, Formularvorlage},
        kunden::Kundendaten,
    };
    let zuordnung = |id: &str, land: Option<Bundesland>, quelle: &str| Formularvorlage {
        id: id.into(),
        titel: id.into(),
        land,
        version: "2026.1".into(),
        erkennung: vec!["Bauherr_Name".into(), "Grundstueck.Flaeche".into()],
        zuordnung: BTreeMap::from([("Bauherr_Name".into(), quelle.into())]),
    };
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().join("Ablage"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_formularen(Formularkatalog::new(vec![
        zuordnung("by-antrag", Some(Bundesland::By), "bauherr.name"),
        zuordnung("bw-antrag", Some(Bundesland::Bw), "projekt.name"),
    ]));
    let user = core.anmelden("aktuell", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Aktualität".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let kunde = core
        .kunde_anlegen(
            Kundendaten {
                vorname: "Erika".into(),
                familienname: "Muster".into(),
                ..Default::default()
            },
            &user,
        )
        .await
        .unwrap();
    core.bauherren_setzen(p.id, vec![kunde.id], p.version, &user)
        .await
        .unwrap();
    let source = musterformular();
    let schema = core.formular_schema(p.id, &source).await.unwrap();
    assert_eq!(schema.vorlage.as_ref().unwrap().id, "by-antrag");
    assert_eq!(
        schema.vorschlag["Bauherr_Name"],
        Feldwert::Text("Erika Muster".into())
    );
    assert!(schema.veraltet.is_empty());
    let d = core
        .formular_aus_bibliothek_mit_quellen(
            p.id,
            "Antrag.pdf",
            &source,
            &schema.vorschlag,
            core_lib::formular::instanz::FormularSpeicheroptionen {
                vorgang: None,
                datenstand: Some(schema.quellen.datenstand.clone()),
            },
            &user,
        )
        .await
        .unwrap();
    let instanz = core
        .formular_instanz(d.document_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(instanz.zuordnung.as_ref().unwrap().id, "by-antrag");
    assert_eq!(instanz.zuordnung.as_ref().unwrap().version, "2026.1");
    assert!(instanz.veraltet.is_empty());

    // Bauherr ändert seinen Namen: das Formular ist veraltet.
    let mut daten = kunde.daten.clone();
    daten.familienname = "Neumann".into();
    core.repos
        .kunden
        .kunde_aendern(kunde.id, daten, kunde.version)
        .await
        .unwrap();
    let analyse = core.formular_analysieren(d.document_id).await.unwrap();
    assert_eq!(analyse.veraltet.len(), 1, "{:?}", analyse.veraltet);
    assert_eq!(analyse.veraltet[0].feld, "Bauherr_Name");
    assert_eq!(
        analyse.veraltet[0].wert,
        Feldwert::Text("Erika Muster".into())
    );
    assert_eq!(
        analyse.veraltet[0].aktuell,
        Feldwert::Text("Erika Neumann".into())
    );
    // Gespeichert ohne Übernahme: die Instanz dokumentiert die Abweichung.
    let d2 = core
        .formular_im_projekt_speichern(
            d.document_id,
            1,
            &BTreeMap::from([(
                "Grundstueck.Flaeche".into(),
                Feldwert::Text("640,00".into()),
            )]),
            &user,
        )
        .await
        .unwrap();
    let zweite = core
        .formular_instanz(d.document_id, d2.version)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(zweite.veraltet.len(), 1);
    assert_eq!(zweite.veraltet[0].feld, "Bauherr_Name");

    // Die Abgabe weist auf das veraltete Formular hin, bricht aber nicht ab.
    let vid = uuid::Uuid::new_v4();
    let akte: core_lib::verfahren::Verfahrensakte = serde_json::from_value(serde_json::json!({
        "portal": null, "bezeichnung": "Neubau", "art": "bauantrag",
        "antragsangaben": serde_json::to_value(core_lib::verfahren::Antragsangaben::default()).unwrap(), "status": "vorbereitung", "vollstaendigkeit": null,
        "ruecknahme": null, "schritte": [],
        "behoerde": {"id": uuid::Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "bab:test", "kontakt": ""},
        "aktenzeichen": "BA-1", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    }))
    .unwrap();
    let f = core
        .fachobjekt_speichern(
            p.id,
            core_lib::plattform::ObjektArt::Verfahren,
            vid,
            0,
            serde_json::to_value(&akte).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let w = core_lib::verfahren::abgabe::Abgabewunsch {
        xbau_bezug: None,
        id: uuid::Uuid::new_v4(),
        titel: "Antrag".into(),
        dokumente: vec![core_lib::plattform::objekte::Dokumentbezug {
            dokument_id: d.document_id,
            version: d2.version,
        }],
        xbau: false,
        druck: false,
        anschreiben: String::new(),
    };
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, w.clone(), &user)
        .await
        .unwrap();
    let akte: core_lib::verfahren::Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let stand = akte.abgaben.iter().find(|a| a.id == w.id).unwrap();
    assert!(
        stand
            .hinweise
            .iter()
            .any(|h| h.contains("Formularwerte weichen") && h.contains("Bauherr_Name")),
        "{:?}",
        stand.hinweise
    );

    // Bauherr aus dem Projekt entfernt: Formular bleibt bearbeitbar.
    let p = core.get_project(p.id).await.unwrap();
    core.bauherren_setzen(p.id, vec![], p.version, &user)
        .await
        .unwrap();
    let analyse = core.formular_analysieren(d.document_id).await.unwrap();
    assert_eq!(analyse.quellen.datenstand.bezug.bauherr_id, None);
    assert!(
        analyse
            .quellen
            .hinweise
            .iter()
            .any(|h| h.contains("nicht mehr zum Projekt")),
        "{:?}",
        analyse.quellen.hinweise
    );
    core.formular_im_projekt_mit_quellen(
        d.document_id,
        d2.version,
        &BTreeMap::from([(
            "Bauherr_Name".into(),
            Feldwert::Text("Erika Neumann".into()),
        )]),
        Some(analyse.quellen.datenstand),
        &user,
    )
    .await
    .unwrap();
}
