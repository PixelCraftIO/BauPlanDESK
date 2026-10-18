//! Persistierte Festsetzungen, Projektbezug der CAD-Umrisse und quantitative Prüfung.
use bytes::Bytes;
use core_lib::cad::baugrenzen::BaugrenzenGeometrie;
use core_lib::cad::{Laengeneinheit, Punkt, testdaten};
use core_lib::calc::{Baugebiet, MassDerNutzung};
use core_lib::domain::{BebauungsplanArt, Bundesland, NewProject, Planungsrecht, ProjectUpdate};
use core_lib::festsetzungen::{Festsetzungen, NutzungsEingabe};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::{CoreError, CoreServices};
use std::sync::Arc;

fn rechteck(x: f64) -> Vec<Punkt> {
    vec![
        Punkt(x, 0.),
        Punkt(x + 10., 0.),
        Punkt(x + 10., 10.),
        Punkt(x, 10.),
    ]
}

#[tokio::test]
async fn gespeicherte_werte_steuern_nachweise_und_leser_darf_sie_nicht_aendern() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Bebauungsplan".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(800.),
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    festgesetzte_werte: Festsetzungen {
                        grz: Some(0.3),
                        gfz: Some(0.6),
                        vollgeschosse: Some(2),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
            &admin,
        )
        .await
        .unwrap();
    let leser = core.anmelden("leser", "Leser").await.unwrap();
    let eingabe = NutzungsEingabe {
        aufstellung: None,
        mass: MassDerNutzung {
            grundstuecksflaeche_m2: 800.,
            grundflaeche_m2: 320.,
            grundflaeche_nebenanlagen_m2: 100.,
            geschossflaeche_m2: 640.,
            baumasse_m3: None,
        },
        vollgeschosse: Some(3),
        gebiet: Baugebiet::WA,
    };
    let r = core.projekt_nutzung_pruefen(p.id, &eingabe).await.unwrap();
    assert_eq!(r.nachweise[0].soll, Some(0.3));
    assert_eq!(r.nachweise[0].eingehalten, Some(false));
    assert_eq!(r.nachweise[7].ueberschreitung, Some(1.0));
    let mut recht = p.planungsrecht.clone();
    recht.berechnung = Some(eingabe);
    let u = ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(recht),
    };
    assert!(matches!(
        core.update_project(u.clone(), &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    let gespeichert = core.update_project(u, &admin).await.unwrap();
    assert!(
        core.get_project(p.id)
            .await
            .unwrap()
            .planungsrecht
            .berechnung
            .is_some()
    );
    assert_eq!(
        core.projekt_nutzung_pruefen(p.id, gespeichert.planungsrecht.berechnung.as_ref().unwrap())
            .await
            .unwrap()
            .projekt_version,
        gespeichert.version
    );
}

#[tokio::test]
async fn geometrie_ist_an_projekt_und_planversion_gebunden() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let neu = || NewProject {
        name: "Plan".into(),
        bundesland: Bundesland::By,
        grundstuecksflaeche_m2: Some(800.),
        planungsrecht: Default::default(),
    };
    let p = core.create_project(neu(), &admin).await.unwrap();
    let anders = core.create_project(neu(), &admin).await.unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.dxf",
            Bytes::from(testdaten::musterplan()),
            &admin,
        )
        .await
        .unwrap();
    let g = BaugrenzenGeometrie {
        dokument_id: d.document_id,
        dokument_version: d.version,
        einheit: Laengeneinheit::Meter,
        baufenster: rechteck(0.),
        vorhaben: rechteck(2.),
    };
    let r = core.baugrenzen_pruefen(&g).unwrap();
    assert!((r.ausserhalb_m2 - 20.).abs() < 1e-8);
    let mut recht = p.planungsrecht.clone();
    recht.baugrenzen = Some(g.clone());
    let u = ProjectUpdate {
        id: anders.id,
        expected_version: anders.version,
        name: anders.name.clone(),
        bundesland: anders.bundesland,
        grundstuecksflaeche_m2: anders.grundstuecksflaeche_m2,
        planungsrecht: Some(recht.clone()),
    };
    assert!(matches!(
        core.update_project(u, &admin).await,
        Err(CoreError::InvalidInput(_))
    ));
    let update = |version, recht| ProjectUpdate {
        id: p.id,
        expected_version: version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(recht),
    };
    let p2 = core
        .update_project(update(p.version, recht.clone()), &admin)
        .await
        .unwrap();
    assert_eq!(
        core.get_project(p.id)
            .await
            .unwrap()
            .planungsrecht
            .baugrenzen,
        Some(g)
    );
    core.store_document(
        p.id,
        Some(d.document_id),
        Some(d.version),
        "Lageplan.dxf",
        Bytes::from(testdaten::musterplan()),
        &admin,
    )
    .await
    .unwrap();
    // Unveränderten alten Planbezug erhalten, andere Angaben weiterhin bearbeiten.
    recht.gemeinde = "München".into();
    let p3 = core
        .update_project(update(p2.version, recht.clone()), &admin)
        .await
        .unwrap();
    // Eine neue Zuordnung darf nicht auf eine veraltete Planversion zeigen.
    recht.baugrenzen.as_mut().unwrap().vorhaben = rechteck(3.);
    assert!(matches!(
        core.update_project(update(p3.version, recht), &admin).await,
        Err(CoreError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn flaechenaufstellung_wird_gespeichert_und_ersetzt_manuelle_summen() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let e: NutzungsEingabe = serde_json::from_value(serde_json::json!({
        "mass": { "grundstuecksflaeche_m2": 500, "grundflaeche_m2": 999, "grundflaeche_nebenanlagen_m2": 999, "geschossflaeche_m2": 999, "baumasse_m3": null }, "gebiet": "WA", "vollgeschosse": 99,
        "aufstellung": { "bau_nvo": "1977", "bauteile": [{"bezeichnung":"Haus", "typ":"hauptanlage", "flaeche_m2":100,"abweichende_anrechnung":null,"begruendung":""}], "geschosse":[{"bezeichnung":"DG", "vollgeschoss":false,"aussenmass_m2":80,"aufenthalt_m2":30,"waende_m2":10,"erschliessung_m2":5}], "nichtvollgeschoss_faktor":0,"nichtvollgeschoss_festsetzung":"" }
    })).unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Historische BauNVO".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Planungsrecht {
                    berechnung: Some(e.clone()),
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    festgesetzte_werte: Festsetzungen {
                        grz: Some(0.2),
                        gfz: Some(0.1),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
            &admin,
        )
        .await
        .unwrap();
    let gespeichert = core.get_project(p.id).await.unwrap();
    assert_eq!(gespeichert.planungsrecht.berechnung.as_ref(), Some(&e));
    let r = core
        .projekt_nutzung_pruefen(p.id, gespeichert.planungsrecht.berechnung.as_ref().unwrap())
        .await
        .unwrap();
    assert_eq!(r.kennzahlen.grz, 0.2);
    assert_eq!(r.kennzahlen.gfz, 0.09);
    assert_eq!(r.nachweise[1].ist, None);
    assert_eq!(r.nachweise[1].soll, None);
    assert_eq!(r.nachweise[7].ist, Some(0.));
    assert_eq!(r.aufstellung.unwrap().geschosse, vec![45.]);
    let mut falsch = e;
    falsch.aufstellung.as_mut().unwrap().geschosse[0].aufenthalt_m2 = 100.;
    assert!(core.projekt_nutzung_pruefen(p.id, &falsch).await.is_err());
}

#[tokio::test]
async fn gemeinschaftsanteile_und_planregel_werden_zentral_angewendet() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let e: NutzungsEingabe = serde_json::from_value(serde_json::json!({
        "mass": { "grundstuecksflaeche_m2": 500, "grundflaeche_m2": 0, "grundflaeche_nebenanlagen_m2": 0, "geschossflaeche_m2": 0, "baumasse_m3": 1000 }, "gebiet": "WA", "vollgeschosse": 0,
        "aufstellung": { "bau_nvo": "2017", "bauteile": [{"bezeichnung":"Haus", "typ":"hauptanlage", "flaeche_m2":100,"abweichende_anrechnung":null,"begruendung":""}], "geschosse":[{"bezeichnung":"DG", "vollgeschoss":false,"aussenmass_m2":80,"aufenthalt_m2":30,"waende_m2":10,"erschliessung_m2":5}], "nichtvollgeschoss_faktor":0,"nichtvollgeschoss_festsetzung":"", "grundstueck": { "basisflaeche_m2":500, "nicht_massgebend_m2":0, "abzug_begruendung":"", "gemeinschaftsanteile":[{ "bezeichnung":"Spielplatz", "gesamtflaeche_m2":400,"anteil_prozent":25,"festsetzung":"TF 3","im_plangebiet":true,"ausserhalb_baugrundstueck":true }] } }
    })).unwrap();
    let mut recht = Planungsrecht {
        bebauungsplan_art: BebauungsplanArt::Qualifiziert,
        berechnung: Some(e.clone()),
        festgesetzte_werte: Festsetzungen {
            nichtvollgeschoss_faktor: Some(1.0),
            nichtvollgeschoss_festsetzung:
                "TF 4: Aufenthaltsräume im DG einschließlich Wänden und Treppenräumen".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let p = core
        .create_project(
            NewProject {
                name: "Gemeinschaftsanlage".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: recht.clone(),
            },
            &admin,
        )
        .await
        .unwrap();
    let gespeicherter = core
        .get_project(p.id)
        .await
        .unwrap()
        .planungsrecht
        .berechnung
        .unwrap();
    assert_eq!(gespeicherter, e);
    let r = core
        .projekt_nutzung_pruefen(p.id, &gespeicherter)
        .await
        .unwrap();
    assert!((r.kennzahlen.grz - 100. / 600.).abs() < 1e-10);
    assert_eq!(r.kennzahlen.gfz, 45. / 600.);
    assert_eq!(r.kennzahlen.bmz, Some(1000. / 600.));
    assert_eq!(r.aufstellung.unwrap().grundstueck.zuschlag_m2, 100.);
    recht.bebauungsplan_art = BebauungsplanArt::Kein;
    assert!(recht.validate().is_err());
    assert!(
        core.create_project(
            NewProject {
                name: "Ohne Plan".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: recht
            },
            &admin
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn mehrere_baugrundstuecke_bleiben_getrennt_gespeichert_und_berechnet() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let plan = core_lib::baugrundstuecke::musterplan();
    let p = core
        .create_project(
            NewProject {
                name: "Mehrere Grundstücke".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(900.),
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    baugrundstuecke: Some(plan.clone()),
                    festgesetzte_werte: Festsetzungen {
                        grz: Some(0.3),
                        gfz: Some(0.6),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
            &admin,
        )
        .await
        .unwrap();
    let neu = core.get_project(p.id).await.unwrap();
    assert_eq!(neu.planungsrecht.baugrundstuecke, Some(plan.clone()));
    let r = core
        .projekt_baugrundstuecke_pruefen(p.id, &plan)
        .await
        .unwrap();
    assert_eq!(r.baugrundstuecke.len(), 2);
    assert_eq!(r.baugrundstuecke[0].grundstueck.bauland_m2, 430.);
    assert_eq!(r.baugrundstuecke[1].grundstueck.bauland_m2, 400.);
    assert_eq!(
        r.baugrundstuecke[0].pruefung.nachweise[0].eingehalten,
        Some(true)
    );
    assert_eq!(
        r.baugrundstuecke[1].pruefung.nachweise[0].eingehalten,
        Some(false)
    );
    assert!(
        core.projekt_nutzung_pruefen(p.id, &plan.berechnungen[0].eingabe)
            .await
            .is_err()
    );
    let leser = core.anmelden("leser", "Leser").await.unwrap();
    let mut recht = p.planungsrecht.clone();
    recht.baugrundstuecke.as_mut().unwrap().flurstuecke[0].abzuege[0].flaeche_m2 = 60.;
    let u = ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(recht),
    };
    assert!(matches!(
        core.update_project(u.clone(), &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    let p2 = core.update_project(u, &admin).await.unwrap();
    let r = core
        .projekt_baugrundstuecke_pruefen(p.id, p2.planungsrecht.baugrundstuecke.as_ref().unwrap())
        .await
        .unwrap();
    assert_eq!(r.baugrundstuecke[0].grundstueck.bauland_m2, 420.);
    assert_eq!(r.baugrundstuecke[1].grundstueck.bauland_m2, 400.);
    assert_eq!(r.projekt_version, p2.version);
}

#[tokio::test]
async fn innenbereich_speichert_umgebungswerte_und_wertet_sie_nicht_als_verstoss() {
    use core_lib::domain::Rechtsgrundlage;
    use core_lib::festsetzungen::{Quelle, Umgebungsreferenz};
    use core_lib::projektbereiche::ProjektBereich;
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage::build(&storage::StorageConfig::Local {
            root: dir.path().to_owned(),
        })
        .unwrap(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let admin = core.anmelden("admin", "Admin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Baulücke".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Kein,
                    rechtsgrundlage: Some(Rechtsgrundlage::Par34),
                    ..Default::default()
                },
            },
            &admin,
        )
        .await
        .unwrap();
    let eingabe = NutzungsEingabe {
        aufstellung: None,
        mass: MassDerNutzung {
            grundstuecksflaeche_m2: 500.,
            grundflaeche_m2: 225.,
            grundflaeche_nebenanlagen_m2: 0.,
            geschossflaeche_m2: 450.,
            baumasse_m3: None,
        },
        vollgeschosse: Some(2),
        gebiet: Baugebiet::WA,
    };
    // Ohne erfasste Umgebung: GRZ 0,45 ist kein Verstoß gegen § 17 BauNVO.
    let r = core.projekt_nutzung_pruefen(p.id, &eingabe).await.unwrap();
    assert!(r.nachweise.iter().all(|n| n.eingehalten.is_none()));
    assert!(r.nachweise.iter().all(|n| n.soll.is_none()));

    let aenderung = |p: &core_lib::domain::Project, u: Umgebungsreferenz| {
        let mut recht = p.planungsrecht.clone();
        recht.umgebung = Some(u);
        ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
            planungsrecht: Some(recht),
        }
    };
    let ohne_grundlage = Umgebungsreferenz {
        grz: Some(0.35),
        ..Default::default()
    };
    assert!(matches!(
        core.projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundlagen,
            aenderung(&p, ohne_grundlage),
            &admin
        )
        .await,
        Err(CoreError::InvalidInput(_))
    ));
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundlagen,
            aenderung(
                &p,
                Umgebungsreferenz {
                    grz: Some(0.35),
                    gfz: Some(0.7),
                    vollgeschosse: Some(2),
                    grundlage: "Lindenweg 2 bis 10, Ortsbesichtigung".into(),
                },
            ),
            &admin,
        )
        .await
        .unwrap();
    assert_eq!(p.planungsrecht.umgebung.as_ref().unwrap().grz, Some(0.35));
    let r = core.projekt_nutzung_pruefen(p.id, &eingabe).await.unwrap();
    let grz = r.nachweise.iter().find(|n| n.art == "grz").unwrap();
    assert_eq!(
        (grz.soll, grz.quelle, grz.eingehalten),
        (Some(0.35), Quelle::Umgebung, None)
    );
    assert!((grz.abweichung.unwrap() - 0.1).abs() < 1e-9);
}
