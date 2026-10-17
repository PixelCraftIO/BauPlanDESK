//! Fakten sind unabhängig speicherbar; Verbraucher erhalten nur bestätigte Bezugsflächen.
use core_lib::{
    CoreError, CoreServices,
    baugrundstuecke::*,
    domain::*,
    grundstuecksinformationen::*,
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::nachweise::{NachweisEingabe, NutzungsArt},
    projektbereiche::ProjektBereich,
};
use std::sync::Arc;
fn aenderung(p: &Project, recht: Planungsrecht) -> ProjectUpdate {
    ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(recht),
    }
}
fn info() -> Grundstuecksinformationen {
    Grundstuecksinformationen {
        flurstuecke: vec![FlurstuecksInfo {
            id: "a".into(),
            gemarkung: "Linden".into(),
            gemarkungsnummer: String::new(),
            flur: "1".into(),
            nummer: "10/1".into(),
            flaeche_m2: 1000.,
            grundbuchbezirk: String::new(),
            grundbuchblatt: String::new(),
            laufende_nummer: None,
            grundbuchauszug: None,
            eigentuemer: String::new(),
            eigentuemer_kunden: vec![],
            bemerkung: String::new(),
            rolle: Default::default(),
        }],
    }
}
fn ansatz() -> BaugrundstuecksErmittlung {
    let muster = musterplan();
    BaugrundstuecksErmittlung {
        ansaetze: vec![FlurstuecksAnsatz {
            flurstueck_id: "a".into(),
            abzuege: vec![FlaechenAbzug {
                art: "private_gruenflaeche".into(),
                flaeche_m2: 100.,
                grundlage: "Planfestsetzung".into(),
            }],
            abweichende_zuordnung: Some(AbweichendeZuordnung {
                kennung: "A".into(),
                grundlage: "Dokumentierte rechtliche Zuordnung zum Bauvorhaben".into(),
            }),
        }],
        berechnungen: vec![BaugrundstueckBerechnung {
            schluessel: einheitsschluessel(&["a".into()]),
            eingabe: muster.berechnungen[0].eingabe.clone(),
            eigene_festsetzungen: None,
        }],
        quellen_hash: None,
    }
}
#[tokio::test]
async fn fakten_ohne_grundbuch_berechnung_getrennt_und_aenderungen_sperren_alte_flaechen() {
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
                name: "Haus".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(9999.),
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let mut recht = p.planungsrecht.clone();
    recht.grundstuecksinformationen = Some(info());
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert!(p.planungsrecht.baugrundstuecke.is_none());
    assert!(p.planungsrecht.baugrundstuecksermittlung.is_none());
    assert!(core.grundstueckskontext(p.id).await.is_err()); // Keine alte Gesamtfläche als Ersatz.
    assert_eq!(
        plan(&p.planungsrecht)
            .unwrap()
            .unwrap()
            .gruppen(true)
            .unwrap()
            .len(),
        1
    );
    let mut recht = p.planungsrecht.clone();
    let mut ohne_grundbuch = ansatz();
    ohne_grundbuch.ansaetze[0].abweichende_zuordnung = None;
    recht.baugrundstuecksermittlung = Some(ohne_grundbuch);
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecke,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        informationen(&p.planungsrecht).flurstuecke[0].flaeche_m2,
        1000.
    );
    assert!(
        serde_json::to_value(&p.planungsrecht.grundstuecksinformationen).unwrap()["flurstuecke"][0]
            .get("abzuege")
            .is_none()
    );
    assert!(
        serde_json::to_value(&p.planungsrecht.baugrundstuecksermittlung)
            .unwrap()
            .get("flurstuecke")
            .is_none()
    );
    let k = core.grundstueckskontext(p.id).await.unwrap();
    assert_eq!(k.len(), 1);
    assert_eq!(k[0].bezugsflaeche_m2, Some(900.));
    let kontext = core_lib::formular::vorlage::Projektkontext::aus_projekt(&p);
    assert_eq!(kontext.wert("grundstueck.flaeche_m2"), Some("900,00"));
    let eingabe = plan(&p.planungsrecht).unwrap().unwrap();
    let gerechnet = core
        .projekt_baugrundstuecke_pruefen(p.id, &eingabe)
        .await
        .unwrap();
    assert_eq!(
        gerechnet.baugrundstuecke[0]
            .pruefung
            .aufstellung
            .as_ref()
            .unwrap()
            .grundstueck
            .massgebend_m2,
        900.
    );
    let n = core
        .nachweis_erstellen(
            p.id,
            uuid::Uuid::new_v4(),
            "GRZ".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: eingabe.clone(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut anderer_abzug = eingabe.clone();
    anderer_abzug.flurstuecke[0].abzuege[0].flaeche_m2 = 50.;
    let err = core
        .nachweis_erstellen(
            p.id,
            uuid::Uuid::new_v4(),
            "Unbestätigter Ansatz".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: anderer_abzug,
            },
            &user,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("zuerst prüfen und speichern"));
    let mut manipuliert = eingabe;
    manipuliert.flurstuecke[0].flaeche_m2 = 999.;
    assert!(
        core.baugrundstuecke_ermitteln(p.id, &manipuliert)
            .await
            .is_err()
    );
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &manipuliert)
            .await
            .is_err()
    );
    let mut recht = p.planungsrecht.clone();
    recht
        .grundstuecksinformationen
        .as_mut()
        .unwrap()
        .flurstuecke[0]
        .flaeche_m2 = 1200.;
    let neu = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert!(core.grundstueckskontext(p.id).await.is_err());
    let err = core
        .nachweis_erstellen(
            p.id,
            uuid::Uuid::new_v4(),
            "Noch nicht bestätigte Quelle".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: plan(&neu.planungsrecht).unwrap().unwrap(),
            },
            &user,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("zuerst prüfen und speichern"));
    assert!(
        core_lib::formular::vorlage::Projektkontext::aus_projekt(&neu)
            .wert("grundstueck.flaeche_m2")
            .is_none()
    );
    assert!(!core.nachweise(p.id).await.unwrap()[0].aktuell);
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis
    );
    assert!(matches!(
        core.projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecke,
            aenderung(&p, p.planungsrecht.clone()),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    let neu = core
        .projekt_bereich_speichern(
            neu.clone(),
            ProjektBereich::Grundstuecke,
            aenderung(&neu, neu.planungsrecht.clone()),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        core.grundstueckskontext(neu.id).await.unwrap()[0].bezugsflaeche_m2,
        Some(1100.)
    );
    // Ein älterer Faktenentwurf überschreibt keinen inzwischen aktualisierten Berechnungsansatz.
    let basis = neu.clone();
    let mut recht = neu.planungsrecht.clone();
    recht.baugrundstuecksermittlung.as_mut().unwrap().ansaetze[0].abzuege[0].flaeche_m2 = 50.;
    core.projekt_bereich_speichern(
        neu.clone(),
        ProjektBereich::Grundstuecke,
        aenderung(&neu, recht),
        &user,
    )
    .await
    .unwrap();
    let mut recht = basis.planungsrecht.clone();
    recht
        .grundstuecksinformationen
        .as_mut()
        .unwrap()
        .flurstuecke[0]
        .bemerkung = "Muster".into();
    let neu = core
        .projekt_bereich_speichern(
            basis.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&basis, recht),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        neu.planungsrecht
            .baugrundstuecksermittlung
            .unwrap()
            .ansaetze[0]
            .abzuege[0]
            .flaeche_m2,
        50.
    );
}
#[tokio::test]
async fn bestandsprojekt_wird_ohne_verlust_getrennt() {
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
                name: "Bestand".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    baugrundstuecke: Some(musterplan()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let vorher = core.grundstueckskontext(p.id).await.unwrap();
    let n = core
        .nachweis_erstellen(
            p.id,
            uuid::Uuid::new_v4(),
            "Bestands-GRZ".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: musterplan(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut recht = p.planungsrecht.clone();
    recht.gemeinde = "Musterdorf".into();
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Stammdaten,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert!(p.planungsrecht.baugrundstuecke.is_none());
    assert!(core.nachweise(p.id).await.unwrap()[0].aktuell);
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis
    );
    let mut recht = p.planungsrecht.clone();
    let mut infos = informationen(&recht);
    infos.flurstuecke[0].bemerkung = "Auszug noch zu prüfen".into();
    recht.grundstuecksinformationen = Some(infos);
    let neu = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert!(neu.planungsrecht.baugrundstuecke.is_none());
    assert_eq!(informationen(&neu.planungsrecht).flurstuecke.len(), 3);
    assert_eq!(
        neu.planungsrecht
            .baugrundstuecksermittlung
            .as_ref()
            .unwrap()
            .berechnungen
            .len(),
        2
    );
    assert!(core.grundstueckskontext(p.id).await.is_err());
    let neu = core
        .projekt_bereich_speichern(
            neu.clone(),
            ProjektBereich::Grundstuecke,
            aenderung(&neu, neu.planungsrecht.clone()),
            &user,
        )
        .await
        .unwrap();
    let nachher = core.grundstueckskontext(neu.id).await.unwrap();
    assert_eq!(
        vorher
            .iter()
            .map(|g| (&g.id, g.bezugsflaeche_m2))
            .collect::<Vec<_>>(),
        nachher
            .iter()
            .map(|g| (&g.id, g.bezugsflaeche_m2))
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn grundbuchauszug_referenziert_eine_eigene_historische_projektversion() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let projekt = |name: &str| NewProject {
        name: name.into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let p = core
        .create_project(projekt("Eigenes Projekt"), &user)
        .await
        .unwrap();
    let fremd = core
        .create_project(projekt("Anderes Projekt"), &user)
        .await
        .unwrap();
    let pdf = bytes::Bytes::from(core_lib::formular::testdaten::musterformular());
    let v1 = core
        .store_document(p.id, None, None, "Auszug.pdf", pdf.clone(), &user)
        .await
        .unwrap();
    core.store_document(
        p.id,
        Some(v1.document_id),
        Some(1),
        "Auszug.pdf",
        pdf.clone(),
        &user,
    )
    .await
    .unwrap();
    let fremd = core
        .store_document(fremd.id, None, None, "Auszug.pdf", pdf, &user)
        .await
        .unwrap();
    let mut infos = info();
    infos.flurstuecke[0].grundbuchauszug = Some(core_lib::plattform::objekte::Dokumentbezug {
        dokument_id: v1.document_id,
        version: 1,
    });
    let mut recht = p.planungsrecht.clone();
    recht.grundstuecksinformationen = Some(infos);
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&p, recht),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        informationen(&p.planungsrecht).flurstuecke[0]
            .grundbuchauszug
            .as_ref()
            .unwrap()
            .version,
        1
    );
    let mut recht = p.planungsrecht.clone();
    recht
        .grundstuecksinformationen
        .as_mut()
        .unwrap()
        .flurstuecke[0]
        .grundbuchauszug = Some(core_lib::plattform::objekte::Dokumentbezug {
        dokument_id: fremd.document_id,
        version: 1,
    });
    assert!(
        core.update_project(aenderung(&p, recht.clone()), &user)
            .await
            .is_err()
    );
    assert!(
        core.create_project(
            NewProject {
                name: "Neues Projekt mit fremdem Auszug".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: recht.clone()
            },
            &user
        )
        .await
        .is_err()
    );
    assert!(
        core.projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            aenderung(&p, recht),
            &user
        )
        .await
        .is_err()
    );
    assert_eq!(
        core.get_project(p.id)
            .await
            .unwrap()
            .planungsrecht
            .grundstuecksinformationen,
        p.planungsrecht.grundstuecksinformationen
    );
}
