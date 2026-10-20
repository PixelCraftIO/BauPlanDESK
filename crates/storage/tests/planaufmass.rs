use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    berechnungen::{BerechnungsArt, vorlagen},
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planaufmass::*,
    plattform::{ObjektArt, objekte::Dokumentbezug},
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn pdf_aufmass_zu_din277_mit_revisionen_und_originalschutz() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let project = core
        .create_project(
            NewProject {
                name: "PDF-Aufmaß".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let bytes = Bytes::from(core_lib::formular::testdaten::musterformular());
    let doc = core
        .store_document(
            project.id,
            None,
            None,
            "Grundriss.pdf",
            bytes.clone(),
            &user,
        )
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let markierung = Uuid::new_v4();
    let aufmass = Uuid::new_v4();
    let mut a = Planaufmass {
        bezeichnung: "EG".into(),
        dokument: Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        },
        seite: 1,
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "DIN 277".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen: vec![Planmarkierung {
            id: markierung,
            ebene_id: ebene,
            bezeichnung: "Büro".into(),
            bedeutung: Planbedeutung::Nuf,
            geometrie: Plangeometrie::Rechteck {
                von: [0., 0.],
                bis: [100., 50.],
            },
            grundstueck_id: None,
            gebaeude_id: None,
            geschoss_id: None,
            ausschnitt_id: None,
        }],
        ausschnitte: vec![],
    };
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let f = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            aufmass,
            0,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let mut p = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Din277)
        .unwrap()
        .plan;
    p.planbindungen.push(Planbindung {
        aufmass_id: aufmass,
        revision: f.revision,
        markierung_id: markierung,
        groesse: Planmessgroesse::Flaeche,
        ziel: "positionen.raum1.aufmass_m2".into(),
    });
    p.eingaben["positionen"][0]["aufmass_m2"] = serde_json::json!(999);
    let (aufgeloest, r) = core
        .berechnung_pruefen(project.id, p.clone())
        .await
        .unwrap();
    assert_eq!(aufgeloest.eingaben["positionen"][0]["aufmass_m2"], 50.);
    assert_eq!(aufgeloest.eingaben["positionen"][0]["art"], "nuf");
    assert!(
        !r.vollstaendig,
        "Geometrie ersetzt keine fachliche Freigabe"
    );
    assert!(
        aufgeloest.eingaben["positionen"][0]["quelle"]
            .as_str()
            .unwrap()
            .contains("Seite 1")
    );
    let id = Uuid::new_v4();
    let gespeichert = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Din277,
            id,
            0,
            serde_json::to_value(&p).unwrap(),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        gespeichert.daten["eingaben"]["positionen"][0]["aufmass_m2"],
        50.
    );
    // Ziel-IDs sind stabil; kein stilles Ausweichen auf die erste Position.
    let mut falsch = p.clone();
    falsch.planbindungen[0].ziel = "positionen.geloescht.aufmass_m2".into();
    assert!(core.berechnung_pruefen(project.id, falsch).await.is_err());
    let mut falsch = p.clone();
    falsch.planbindungen[0].groesse = Planmessgroesse::Laenge;
    assert!(core.berechnung_pruefen(project.id, falsch).await.is_err());
    let anderer = core
        .create_project(
            NewProject {
                name: "Andere Akte".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(core.planaufmass_pruefen(anderer.id, &a).await.is_err());
    assert!(
        core.berechnung_pruefen(anderer.id, p.clone())
            .await
            .is_err()
    );
    // Nur Maskenrevision ändern, PDF bleibt bytegleich.
    a.kalibrierung = Some(Plankalibrierung::Massstab { nenner: 100. });
    let f2 = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.berechnung_pruefen(project.id, p.clone())
            .await
            .is_err()
    );
    a.bezeichnung = "Veralteter Entwurf".into();
    assert!(matches!(
        core.fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    assert_eq!(
        core.load_version(doc.document_id, 1).await.unwrap().1,
        bytes
    );
    p.planbindungen[0].revision = f2.revision;
    assert!(core.berechnung_pruefen(project.id, p.clone()).await.is_ok());
    // Ebenen und weitere Markierungen ändern die gebundene Messung nicht.
    let mut a: Planaufmass = serde_json::from_value(f2.daten.clone()).unwrap();
    a.ebenen[0].sichtbar = false;
    let mut weitere = a.markierungen[0].clone();
    weitere.id = Uuid::new_v4();
    weitere.bezeichnung = "Weitere Fläche".into();
    a.markierungen.push(weitere);
    let f3 = core
        .fachobjekt_speichern(
            project.id,
            ObjektArt::Planaufmass,
            aufmass,
            f2.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    assert!(f3.revision > f2.revision);
    assert!(
        core.berechnung_pruefen(project.id, p.clone()).await.is_ok(),
        "Gestalterische Änderungen dürfen die Übernahme nicht veralten lassen"
    );
    // Die gebundene Markierung selbst bleibt maßgebend.
    a.markierungen[0].bezeichnung = "Umbenannt".into();
    core.fachobjekt_speichern(
        project.id,
        ObjektArt::Planaufmass,
        aufmass,
        f3.revision,
        save(&a),
        &user,
    )
    .await
    .unwrap();
    assert!(
        core.berechnung_pruefen(project.id, p.clone())
            .await
            .is_err()
    );
    core.store_document(
        project.id,
        Some(doc.document_id),
        Some(1),
        "Grundriss.pdf",
        bytes.clone(),
        &user,
    )
    .await
    .unwrap();
    assert!(core.berechnung_pruefen(project.id, p).await.is_err());
    // Übersicht erfasst Aufmaß und Berechnungsdaten auch ohne archivierten Nachweis.
    let u = core
        .projektuebersicht_am(project.id, chrono::Utc::now().date_naive(), &user)
        .await
        .unwrap();
    assert!(
        u.punkte
            .iter()
            .any(|p| p.ziel.bereich == "planaufmass" && p.hinweis.contains("veraltet"))
    );
    assert!(u.punkte.iter().any(|p| p.ziel.bereich == "din277"));
}

#[tokio::test]
async fn planraeume_und_wandhoehen_bleiben_mit_der_pdf_quelle_verbunden() {
    use core_lib::plattform::{nachweise::NachweisEingabe, objekte::Raumbuch};
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Planräume und Höhen".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let doc = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let gid = Uuid::new_v4();
    let sid = Uuid::new_v4();
    let g=core.fachobjekt_speichern(p.id,ObjektArt::Gebaeude,gid,0,serde_json::json!({"bezeichnung":"Haus A","grundstueck_ids":[],"geschosse":[{"id":sid,"bezeichnung":"EG","vollgeschoss":null,"grundlage":""}],"geometrie":{"typ":"hauptanlage","nullpunkt_m_nhn":100.,"gelaende_m":0.},"quelle":null}),&user).await.unwrap();
    let aid = Uuid::new_v4();
    let rid = Uuid::new_v4();
    let raum = Uuid::new_v4();
    let hoehe = Uuid::new_v4();
    let dach = Uuid::new_v4();
    let ebene = Uuid::new_v4();
    let grundflaeche = Uuid::new_v4();
    let geschossflaeche = Uuid::new_v4();
    let mut a = Planaufmass {
        bezeichnung: "Grundriss und Ansicht".into(),
        dokument: Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        },
        seite: 1,
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: Some(Planhoehenbezug {
            punkt: [0., 100.],
            hoehe_m: 100.,
            system: "NHN".into(),
        }),
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Aufmaß".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen: vec![
            Planmarkierung {
                id: raum,
                ebene_id: ebene,
                bezeichnung: "Wohnen".into(),
                bedeutung: Planbedeutung::Aufenthaltsraum,
                geometrie: Plangeometrie::Rechteck {
                    von: [0., 0.],
                    bis: [100., 50.],
                },
                grundstueck_id: None,
                gebaeude_id: Some(gid),
                geschoss_id: None,
                ausschnitt_id: None,
            },
            Planmarkierung {
                id: hoehe,
                ebene_id: ebene,
                bezeichnung: "Traufe".into(),
                bedeutung: Planbedeutung::Traufe,
                geometrie: Plangeometrie::Hoehenpunkt { punkt: [100., 20.] },
                grundstueck_id: None,
                gebaeude_id: Some(gid),
                geschoss_id: None,
                ausschnitt_id: None,
            },
            Planmarkierung {
                id: dach,
                ebene_id: ebene,
                bezeichnung: "Dach".into(),
                bedeutung: Planbedeutung::Dach,
                geometrie: Plangeometrie::Strecke {
                    von: [100., 20.],
                    bis: [120., 0.],
                },
                grundstueck_id: None,
                gebaeude_id: Some(gid),
                geschoss_id: None,
                ausschnitt_id: None,
            },
        ],
        ausschnitte: vec![],
    };
    a.markierungen.push(Planmarkierung {
        id: grundflaeche,
        ebene_id: ebene,
        bezeichnung: "Gebäudeprojektion".into(),
        bedeutung: Planbedeutung::Gebaeudegrundflaeche,
        geometrie: Plangeometrie::Rechteck {
            von: [0., 0.],
            bis: [100., 80.],
        },
        grundstueck_id: None,
        gebaeude_id: Some(gid),
        geschoss_id: None,
        ausschnitt_id: None,
    });
    a.markierungen.push(Planmarkierung {
        id: geschossflaeche,
        ebene_id: ebene,
        bezeichnung: "Außenkante EG".into(),
        bedeutung: Planbedeutung::GeschossAussenkante,
        geometrie: Plangeometrie::Rechteck {
            von: [0., 0.],
            bis: [100., 60.],
        },
        grundstueck_id: None,
        gebaeude_id: Some(gid),
        geschoss_id: Some(sid),
        ausschnitt_id: None,
    });
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planaufmass,
        aid,
        0,
        serde_json::to_value(&a).unwrap(),
        &user,
    )
    .await
    .unwrap();
    let mut g: core_lib::plattform::objekte::Gebaeude = serde_json::from_value(g.daten).unwrap();
    g.planbindungen = vec![
        Planbindung {
            aufmass_id: aid,
            revision: 1,
            markierung_id: grundflaeche,
            groesse: Planmessgroesse::Flaeche,
            ziel: "geometrie.grundflaeche_m2".into(),
        },
        Planbindung {
            aufmass_id: aid,
            revision: 1,
            markierung_id: geschossflaeche,
            groesse: Planmessgroesse::Flaeche,
            ziel: format!("geschosse.{sid}.flaeche_m2"),
        },
        Planbindung {
            aufmass_id: aid,
            revision: 1,
            markierung_id: hoehe,
            groesse: Planmessgroesse::Hoehe,
            ziel: "geometrie.traufe_m".into(),
        },
    ];
    let mut falsch = g.clone();
    falsch.planbindungen[0].markierung_id = geschossflaeche;
    assert!(
        core.planaufmass_gebaeude(p.id, gid, falsch).await.is_err(),
        "Geschossfläche ist nicht automatisch der Gebäudeumgriff"
    );
    g = core.planaufmass_gebaeude(p.id, gid, g).await.unwrap();
    assert_eq!(g.geometrie.as_ref().unwrap().grundflaeche_m2, Some(80.));
    assert_eq!(g.geometrie.as_ref().unwrap().traufe_m, Some(8.));
    assert_eq!(
        g.geschosse[0].geometrie.as_ref().unwrap().flaeche_m2,
        Some(60.)
    );
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        gid,
        1,
        serde_json::to_value(&g).unwrap(),
        &user,
    )
    .await
    .unwrap();
    let mut lage = core_lib::baugrundstuecke::musterplan();
    lage.berechnungen.truncate(1);
    let eingebunden = core.gebaeude_einbinden(p.id, &lage).await.unwrap();
    assert!(
        eingebunden.plan.berechnungen[0]
            .eingabe
            .aufstellung
            .as_ref()
            .unwrap()
            .bauteile
            .iter()
            .any(|b| b.gebaeude_id == Some(gid) && b.flaeche_m2 == 80.)
    );
    let mut hp = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Hoehen)
        .unwrap()
        .plan;
    hp.gebaeude_id = Some(gid);
    hp.gebaeudedaten_uebernehmen = Some(true);
    core.berechnung_pruefen(p.id, hp.clone())
        .await
        .expect("Höhen aus aktuellen Gebäudeplanquellen");
    // Ohne neues Vertragsfeld lesbar; Räume entstehen direkt aus den Masken.
    let alt: Raumbuch = serde_json::from_value(
        serde_json::json!({"bezeichnung":"Räume","gebaeude_id":gid,"geschoss_id":null,"raeume":[]}),
    )
    .unwrap();
    let mut r = core.planaufmass_raumbuch(p.id, aid, 1, alt).await.unwrap();
    assert_eq!(r.raeume.len(), 1);
    assert_eq!(r.raeume[0].flaeche_m2, 50.);
    assert_eq!(r.raeume[0].nutzung, "Aufenthaltsraum");
    r.raeume[0].nummer = "EG-01".into();
    r.raeume[0].hoehe_m = Some(2.8);
    r = core.planaufmass_raumbuch(p.id, aid, 1, r).await.unwrap();
    assert_eq!(
        r.raeume.len(),
        1,
        "Erneute Übernahme erzeugt keine doppelten Räume"
    );
    assert_eq!(r.raeume[0].nummer, "EG-01");
    assert_eq!(r.raeume[0].hoehe_m, Some(2.8));
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Raumbuch,
        rid,
        0,
        serde_json::to_value(&r).unwrap(),
        &user,
    )
    .await
    .unwrap();
    let wohn = core
        .berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Wohnflaechen)
        .await
        .unwrap();
    assert_eq!(wohn.eingaben["positionen"][0]["aufmass_m2"], 50.);

    let mut plan = core_lib::abstandsflaechen::testdaten::plan();
    plan.hoehenbezug = "NHN".into();
    plan.waende[0].gebaeude_id = Some(gid);
    let wid = plan.waende[0].id;
    plan.planbindungen = vec![
        Planbindung {
            aufmass_id: aid,
            revision: 1,
            markierung_id: hoehe,
            groesse: Planmessgroesse::Hoehe,
            ziel: format!("waende.{wid}.abschluss_a_m"),
        },
        Planbindung {
            aufmass_id: aid,
            revision: 1,
            markierung_id: dach,
            groesse: Planmessgroesse::Neigung,
            ziel: format!("waende.{wid}.dachneigung_grad"),
        },
    ];
    let mut falsch = plan.clone();
    falsch.hoehenbezug = "Lokales System".into();
    assert!(core.planaufmass_abstand(p.id, falsch).await.is_err());
    plan = core.planaufmass_abstand(p.id, plan).await.unwrap();
    assert_eq!(plan.waende[0].abschluss_a_m, Some(108.));
    assert_eq!(plan.waende[0].dachneigung_grad, Some(45.));
    assert!(!plan.umfang_geprueft);
    let mut falsch = plan.clone();
    falsch.waende[0].abschluss_a_m = Some(999.);
    assert!(core.abstandsplan_pruefen(p.id, &falsch).await.is_err());
    plan.umfang_geprueft = true;
    plan.recht.geprueft = true;
    let zid = Uuid::new_v4();
    let saved = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            zid,
            0,
            serde_json::to_value(&plan).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let plan = serde_json::from_value(saved.daten).unwrap();
    let n = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            "Wand".into(),
            NachweisEingabe::AbstandProjekt {
                objekt: zid,
                plan: Box::new(plan),
            },
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|v| v.stand.id == n.id)
            .unwrap()
            .aktuell
    );

    a.markierungen[0].geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [100., 60.],
    };
    // Die Raumkontur betrifft nur das Raumbuch; der geänderte Höhenbezug alle
    // Höhenbindungen und damit Gebäude, Gebäudehöhen und Wände.
    a.hoehenbezug.as_mut().unwrap().system = "m ü. NHN".into();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planaufmass,
        aid,
        1,
        serde_json::to_value(&a).unwrap(),
        &user,
    )
    .await
    .unwrap();
    assert!(core.raumbuch_planquellen_pruefen(p.id, &r).await.is_err());
    assert!(core.gebaeudegrundlagen(p.id, gid).await.is_err());
    assert!(
        core.gebaeude_einbinden(p.id, &lage).await.is_err(),
        "GRZ/GFZ dürfen veraltete Gebäudeaufmaße nicht übernehmen"
    );
    assert!(
        core.berechnung_pruefen(p.id, hp).await.is_err(),
        "Höhenübernahme muss indirekte Planquellen prüfen"
    );
    assert!(
        core.berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Wohnflaechen)
            .await
            .is_err()
    );
    assert!(
        core.berechnung_pruefen(p.id, wohn).await.is_err(),
        "Indirekte Verbraucher dürfen veraltete Planräume nicht weiterverwenden"
    );
    assert!(
        !core
            .nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|v| v.stand.id == n.id)
            .unwrap()
            .aktuell
    );
    assert_eq!(
        core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
        n.ergebnis,
        "Archiv bleibt reproduzierbar"
    );
    let u = core
        .projektuebersicht_am(p.id, chrono::Utc::now().date_naive(), &user)
        .await
        .unwrap();
    assert!(u.punkte.iter().any(|x| x.ziel.bereich == "raumbuch"));
    assert!(u.punkte.iter().any(|x| x.ziel.bereich == "gebaeude"));
    assert!(u.punkte.iter().any(|x| x.ziel.bereich == "abstand"));
    let r = core.planaufmass_raumbuch(p.id, aid, 2, r).await.unwrap();
    assert_eq!(r.raeume[0].flaeche_m2, 60.);
    assert_eq!(r.raeume[0].nummer, "EG-01");
}
