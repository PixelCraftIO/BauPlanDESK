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
            hoehenzone_von: None,
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
                hoehenzone_von: None,
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
                hoehenzone_von: None,
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
                hoehenzone_von: None,
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
        hoehenzone_von: None,
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
        hoehenzone_von: None,
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
    let plan: core_lib::abstandsflaechen::AbstandsPlan =
        serde_json::from_value(saved.daten).unwrap();
    // Aufmaß-Aufgaben: gebundene Ziele sind erledigt, passende freie Markierungen
    // werden vorgeschlagen, fehlende Bedeutungen bleiben offen.
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let aufgabe = |verbraucher: &str, ziel: &str| {
        aufgaben
            .iter()
            .find(|x| x.verbraucher == verbraucher && x.ziel == ziel)
            .unwrap_or_else(|| panic!("Aufgabe {verbraucher} {ziel}"))
            .clone()
    };
    assert_eq!(
        aufgabe("gebaeude", "geometrie.grundflaeche_m2").gebunden,
        Some(grundflaeche)
    );
    assert_eq!(
        aufgabe("gebaeude", &format!("geschosse.{sid}.flaeche_m2")).gebunden,
        Some(geschossflaeche)
    );
    assert_eq!(
        aufgabe("gebaeude", "geometrie.traufe_m").gebunden,
        Some(hoehe)
    );
    let gelaende = aufgabe("gebaeude", "geometrie.gelaende_m");
    assert_eq!(gelaende.bedeutung, Planbedeutung::Gelaende);
    assert_eq!(gelaende.geometrie, Aufgabengeometrie::Hoehenpunkt);
    assert!(gelaende.gebunden.is_none() && gelaende.markierung.is_none());
    assert_eq!(
        aufgabe("abstand", &format!("waende.{wid}.abschluss_a_m")).gebunden,
        Some(hoehe)
    );
    assert_eq!(
        aufgabe("abstand", &format!("waende.{wid}.dachneigung_grad")).gebunden,
        Some(dach)
    );
    let dachhoehe = aufgabe("abstand", &format!("waende.{wid}.dachhoehe_m"));
    assert_eq!(
        dachhoehe.markierung,
        Some(dach),
        "freie Dachstrecke wird vorgeschlagen"
    );
    assert!(dachhoehe.gebunden.is_none() && !dachhoehe.wert_vorhanden);
    assert_eq!(
        dachhoehe.gruppe,
        format!("{} · {}", plan.bezeichnung, plan.waende[0].bezeichnung)
    );
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

/// Höhen aus Schnitt-Bezugslinien, getrennte Höhenmaß-Vorschläge, lesbare
/// Quellangaben, Raumbuch-Importe für DIN 277/WoFlV und Gebäudeflächen in
/// grundstücksbezogenen Bilanzen.
#[tokio::test]
async fn ausschnitte_aufgaben_quellen_importe_und_grundstuecksbilanz() {
    use core_lib::plattform::objekte::{Gebaeude, Raumbuch};
    use serde_json::json;
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
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
                name: "Ausschnitte".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: core_lib::domain::Planungsrecht {
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    bebauungsplan_art: core_lib::domain::BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(core_lib::baugrundstuecke::musterplan()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    assert!(grund.len() >= 2);
    let doc = core
        .store_document(
            p.id,
            None,
            None,
            "EP-01_Schnitt.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let dokument = Dokumentbezug {
        dokument_id: doc.document_id,
        version: 1,
    };
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Dokumentdaten,
        doc.document_id.0,
        0,
        json!({"dokument": dokument, "dokumentart": "Bauzeichnung", "plannummer": "EP-01",
            "revisionsindex": "a", "massstab": "1:100", "verfasser": "", "datum": null}),
        &user,
    )
    .await
    .unwrap();
    let (haus, nachbar, sid) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for (id, name, g) in [
        (haus, "Haus", &grund[0]),
        (nachbar, "Nachbarhaus", &grund[1]),
    ] {
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Gebaeude,
            id,
            0,
            json!({"bezeichnung": name, "grundstueck_ids": [], "flurstueck_ids": g.flurstueck_ids,
                "geschosse": [{"id": sid, "bezeichnung": "EG", "vollgeschoss": null, "grundlage": ""}],
                "geometrie": {"typ": "hauptanlage"}, "quelle": null}),
            &user,
        )
        .await
        .unwrap();
    }
    let (ebene, x_grund, x_schnitt) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let mut ids = std::collections::BTreeMap::new();
    let mut m = |name: &str,
                 bedeutung: Planbedeutung,
                 geometrie: Plangeometrie,
                 gebaeude: Uuid,
                 geschoss: Option<Uuid>,
                 aus: Uuid| {
        let id = Uuid::new_v4();
        ids.insert(name.to_string(), id);
        Planmarkierung {
            id,
            ebene_id: ebene,
            bezeichnung: name.into(),
            bedeutung,
            geometrie,
            grundstueck_id: (gebaeude == haus).then_some(grund[0].id),
            gebaeude_id: Some(gebaeude),
            geschoss_id: geschoss,
            ausschnitt_id: Some(aus),
            hoehenzone_von: None,
        }
    };
    let rechteck = |a: [f64; 2], b: [f64; 2]| Plangeometrie::Rechteck { von: a, bis: b };
    let strecke = |a: [f64; 2], b: [f64; 2]| Plangeometrie::Strecke { von: a, bis: b };
    let punkt = |p: [f64; 2]| Plangeometrie::Hoehenpunkt { punkt: p };
    let markierungen = vec![
        m(
            "Gebäudeprojektion",
            Planbedeutung::Gebaeudegrundflaeche,
            rechteck([0., 0.], [100., 80.]),
            haus,
            None,
            x_grund,
        ),
        m(
            "Nachbarhaus",
            Planbedeutung::Gebaeudegrundflaeche,
            rechteck([110., 0.], [190., 50.]),
            nachbar,
            None,
            x_grund,
        ),
        m(
            "Büro",
            Planbedeutung::Nuf,
            rechteck([0., 0.], [50., 40.]),
            haus,
            Some(sid),
            x_grund,
        ),
        m(
            "Wohnen",
            Planbedeutung::Aufenthaltsraum,
            rechteck([50., 0.], [100., 40.]),
            haus,
            Some(sid),
            x_grund,
        ),
        m(
            "±0,00",
            Planbedeutung::Nullpunkt,
            punkt([310., 200.]),
            haus,
            None,
            x_schnitt,
        ),
        m(
            "Traufe",
            Planbedeutung::Traufe,
            punkt([320., 140.]),
            haus,
            None,
            x_schnitt,
        ),
        // Neutral benannt; die Deckendicke steht vor der lichten Höhe.
        m(
            "Maß 1",
            Planbedeutung::Hoehenmass,
            strecke([410., 175.], [410., 172.]),
            haus,
            Some(sid),
            x_schnitt,
        ),
        m(
            "Maß 2",
            Planbedeutung::Hoehenmass,
            strecke([400., 200.], [400., 175.]),
            haus,
            Some(sid),
            x_schnitt,
        ),
        m(
            "Körperhöhe",
            Planbedeutung::Hoehenmass,
            strecke([420., 200.], [420., 140.]),
            haus,
            None,
            x_schnitt,
        ),
    ];
    let a = Planaufmass {
        bezeichnung: "Grundriss und Schnitt".into(),
        dokument: dokument.clone(),
        seite: 1,
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        // Kein blattweiter Höhenbezug: Höhen kommen aus der Bezugslinie des Schnitts.
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Aufmaß".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen,
        ausschnitte: vec![
            Planausschnitt {
                id: x_grund,
                bezeichnung: "Grundriss EG".into(),
                art: Ausschnittart::Grundriss,
                von: [0., 0.],
                bis: [200., 200.],
                kalibrierung: None,
                bezugslinie: None,
            },
            Planausschnitt {
                id: x_schnitt,
                bezeichnung: "Schnitt A-A".into(),
                art: Ausschnittart::Schnitt,
                von: [300., 0.],
                bis: [600., 300.],
                kalibrierung: None,
                bezugslinie: Some(Planbezugslinie {
                    von: [300., 200.],
                    bis: [600., 200.],
                    hoehe_m: 100.,
                    system: "m ü. NHN".into(),
                }),
            },
        ],
    };
    let aid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aid,
            0,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let bindung = |name: &str, groesse, ziel: &str| Planbindung {
        aufmass_id: aid,
        revision: f.revision,
        markierung_id: ids[name],
        groesse,
        ziel: ziel.into(),
    };
    let gebaeude = |id: Uuid| {
        let core = &core;
        async move {
            let f = core.fachobjekte(p.id, ObjektArt::Gebaeude).await.unwrap();
            let f = f.into_iter().find(|f| f.objekt == id.to_string()).unwrap();
            (
                f.revision,
                serde_json::from_value::<Gebaeude>(f.daten).unwrap(),
            )
        }
    };

    // 1. Höhenpunkte im Schnitt mit Bezugslinie „m ü. NHN“ ohne blattweiten Höhenbezug.
    let (rev, mut g) = gebaeude(haus).await;
    g.planbindungen = vec![
        bindung(
            "Gebäudeprojektion",
            Planmessgroesse::Flaeche,
            "geometrie.grundflaeche_m2",
        ),
        bindung("±0,00", Planmessgroesse::Hoehe, "geometrie.nullpunkt_m_nhn"),
        bindung("Traufe", Planmessgroesse::Hoehe, "geometrie.traufe_m"),
    ];
    let g = core.planaufmass_gebaeude(p.id, haus, g).await.unwrap();
    let geo = g.geometrie.clone().unwrap();
    assert_eq!(geo.grundflaeche_m2, Some(80.));
    assert_eq!(geo.nullpunkt_m_nhn, Some(100.));
    assert!((geo.traufe_m.unwrap() - 6.).abs() < 1e-9);
    // Maßgebend ist das System der Bezugslinie, nicht der blattweite Höhenbezug.
    let mut lokal = a.clone();
    lokal.ausschnitte[1].bezugslinie.as_mut().unwrap().system = "lokal".into();
    lokal.hoehenbezug = Some(Planhoehenbezug {
        punkt: [0., 0.],
        hoehe_m: 0.,
        system: "m ü. NHN".into(),
    });
    let f_lokal = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aid,
            f.revision,
            serde_json::to_value(&lokal).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let mut gl = g.clone();
    for b in &mut gl.planbindungen {
        b.revision = f_lokal.revision;
    }
    let e = core.planaufmass_gebaeude(p.id, haus, gl).await.unwrap_err();
    assert!(e.to_string().contains("m ü. NHN"), "{e}");
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aid,
            f_lokal.revision,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let bindung = |name: &str, groesse, ziel: &str| Planbindung {
        aufmass_id: aid,
        revision: f.revision,
        markierung_id: ids[name],
        groesse,
        ziel: ziel.into(),
    };
    let mut g = g;
    for b in &mut g.planbindungen {
        b.revision = f.revision;
    }
    let g = core.planaufmass_gebaeude(p.id, haus, g).await.unwrap();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        haus,
        rev,
        serde_json::to_value(&g).unwrap(),
        &user,
    )
    .await
    .unwrap();

    // 2. Höhenmaße: je Ziel eine plausible, noch nicht vergebene Markierung.
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let vorschlag = |feld: &str| {
        aufgaben
            .iter()
            .find(|x| x.objekt == haus && x.ziel == format!("geschosse.{sid}.{feld}"))
            .unwrap()
            .markierung
    };
    assert_eq!(vorschlag("lichte_hoehe_m"), Some(ids["Maß 2"]));
    assert_eq!(vorschlag("deckendicke_m"), Some(ids["Maß 1"]));
    assert_eq!(
        vorschlag("geschosshoehe_m"),
        None,
        "kein doppelter Vorschlag"
    );
    let gebunden: Vec<_> = g.planbindungen.iter().map(|b| b.markierung_id).collect();
    assert!(
        aufgaben
            .iter()
            .filter(|x| x.objekt == haus)
            .all(|x| x.markierung.is_none_or(|m| !gebunden.contains(&m))),
        "gebundene Markierungen werden nicht erneut vorgeschlagen"
    );

    // 7. Quellangaben: Dateiname/Plannummer statt UUIDs, mehrere Quellen je Zeile.
    let mut dp = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Din277)
        .unwrap()
        .plan;
    dp.grundstueck_id = Some(grund[0].id);
    dp.gebaeude_id = Some(haus);
    dp.eingaben["volumen"] = json!([{"id": "koerper", "bezeichnung": "Baukörper", "form": "aufmass",
        "laenge_m": 0, "breite_m": 0, "breite2_m": 0, "aufmass_m2": 0, "hoehe_m": 0, "quelle": ""}]);
    dp.planbindungen = vec![
        bindung(
            "Gebäudeprojektion",
            Planmessgroesse::Flaeche,
            "volumen.koerper.aufmass_m2",
        ),
        bindung(
            "Körperhöhe",
            Planmessgroesse::Hoehe,
            "volumen.koerper.hoehe_m",
        ),
    ];
    let (dp2, _) = core.berechnung_pruefen(p.id, dp.clone()).await.unwrap();
    let q = dp2.eingaben["volumen"][0]["quelle"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        q.contains("EP-01_Schnitt.pdf") && q.contains("Plan EP-01 Index a"),
        "{q}"
    );
    assert!(
        q.contains("Fläche:") && q.contains("Gebäudeprojektion"),
        "{q}"
    );
    assert!(q.contains("Höhe:") && q.contains("Körperhöhe"), "{q}");
    assert!(
        !q.contains(&aid.to_string()) && !q.contains(&doc.document_id.0.to_string()),
        "{q}"
    );
    let (dp3, _) = core.berechnung_pruefen(p.id, dp2.clone()).await.unwrap();
    assert_eq!(
        dp3, dp2,
        "Quellangabe ist stabil und wächst nicht bei erneuter Prüfung"
    );

    // 8. Grundstücksbezogene Bilanz bindet die Gebäudefläche eines Gebäudes auf dem Grundstück.
    let mut fp = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Freiflaechen)
        .unwrap()
        .plan;
    fp.grundstueck_id = Some(grund[0].id);
    fp.eingaben["positionen"][0]["typ"] = json!("dach");
    fp.planbindungen = vec![bindung(
        "Gebäudeprojektion",
        Planmessgroesse::Flaeche,
        "positionen.flaeche1.flaeche_m2",
    )];
    let (fp2, _) = core.berechnung_pruefen(p.id, fp.clone()).await.unwrap();
    assert_eq!(fp2.eingaben["positionen"][0]["flaeche_m2"], json!(80.));
    fp.planbindungen = vec![bindung(
        "Nachbarhaus",
        Planmessgroesse::Flaeche,
        "positionen.flaeche1.flaeche_m2",
    )];
    let e = core.berechnung_pruefen(p.id, fp.clone()).await.unwrap_err();
    assert!(e.to_string().contains("Nachbarhaus"), "{e}");
    // Gebäudebezogene Berechnungen bleiben streng: fremdes Gebäude wird abgewiesen.
    let mut fremd = dp.clone();
    fremd.planbindungen = vec![bindung(
        "Nachbarhaus",
        Planmessgroesse::Flaeche,
        "volumen.koerper.aufmass_m2",
    )];
    assert!(core.berechnung_pruefen(p.id, fremd).await.is_err());

    // 5./6. Raumbuch-Import: DIN 277 übernimmt Klassifikation und Geschoss,
    // fehlende Angaben werden je Position lesbar gemeldet; WoFlV-Import ist prüfbar.
    let leer = Raumbuch {
        planbindungen: vec![],
        bezeichnung: "Raumbuch EG".into(),
        gebaeude_id: haus,
        geschoss_id: Some(sid),
        raeume: vec![],
    };
    let r = core
        .planaufmass_raumbuch(p.id, aid, f.revision, leer)
        .await
        .unwrap();
    let rid = Uuid::new_v4();
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
    let mut din = core
        .berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Din277)
        .await
        .unwrap();
    din.grundstueck_id = Some(grund[0].id);
    let pos = din.eingaben["positionen"].as_array().unwrap().clone();
    let buero = pos
        .iter()
        .find(|x| x["id"] == json!(ids["Büro"].to_string()))
        .unwrap();
    assert_eq!(
        (buero["art"].as_str(), buero["geschoss"].as_str()),
        (Some("nuf"), Some("EG"))
    );
    let e = core
        .berechnung_pruefen(p.id, din.clone())
        .await
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("Wohnen") && e.contains("Flächenart") && !e.contains("unknown variant"),
        "{e}"
    );
    for x in din.eingaben["positionen"].as_array_mut().unwrap() {
        if x["art"] == json!("") {
            x["art"] = json!("nuf");
        }
    }
    core.berechnung_pruefen(p.id, din).await.unwrap();
    let mut wohn = core
        .berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Wohnflaechen)
        .await
        .unwrap();
    wohn.grundstueck_id = Some(grund[0].id);
    let (_, w) = core.berechnung_pruefen(p.id, wohn).await.unwrap();
    assert!(
        w.offen
            .iter()
            .any(|o| o.contains("Wohnen") && o.contains("Wohnung zuordnen")),
        "{:?}",
        w.offen
    );
    assert!(
        w.offen.iter().any(|o| o.contains("Raumart")),
        "{:?}",
        w.offen
    );
}

#[tokio::test]
async fn hoehenzonen_aus_pdf_werden_je_zone_als_wohnflaeche_angerechnet() {
    use core_lib::plattform::objekte::Raumbuch;
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
                name: "Dachgeschoss".into(),
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
            "DG.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let gid = Uuid::new_v4();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        gid,
        0,
        serde_json::json!({"bezeichnung":"Haus","grundstueck_ids":[],"geschosse":[],"quelle":null}),
        &user,
    )
    .await
    .unwrap();
    let (ebene, raum, zone, aid, rid) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let markierung = |id, name: &str, bis: [f64; 2], von_raum: Option<Uuid>| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung: Planbedeutung::Aufenthaltsraum,
        geometrie: Plangeometrie::Rechteck { von: [0., 0.], bis },
        grundstueck_id: None,
        gebaeude_id: Some(gid),
        geschoss_id: None,
        ausschnitt_id: None,
        hoehenzone_von: von_raum,
    };
    // 10 m × 5 m Raum, davon 10 m × 1 m Schräge zwischen 1 m und 2 m.
    let mut a = Planaufmass {
        bezeichnung: "DG".into(),
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
            bezeichnung: "Räume".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen: vec![
            markierung(raum, "Schlafen", [100., 50.], None),
            markierung(zone, "Schräge 1–2 m", [100., 10.], None),
        ],
        ausschnitte: vec![],
    };
    // Eine Zone muss auf einen Raum zeigen, der selbst keine Zone ist.
    let mut falsch = a.clone();
    falsch.markierungen[1].hoehenzone_von = Some(zone);
    assert!(falsch.messen().is_err());
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
    // Zunächst als eigener Raum übernommen (wie bisher).
    let leer: Raumbuch = serde_json::from_value(
        serde_json::json!({"bezeichnung":"DG","gebaeude_id":gid,"geschoss_id":null,"raeume":[]}),
    )
    .unwrap();
    let mut r = core.planaufmass_raumbuch(p.id, aid, 1, leer).await.unwrap();
    assert_eq!(r.raeume.len(), 2);
    r.raeume.iter_mut().find(|x| x.id == zone).unwrap().hoehe_m = Some(1.0);
    r.raeume.iter_mut().find(|x| x.id == raum).unwrap().hoehe_m = Some(2.5);
    // Als Höhenzone gekennzeichnet: wird Zone des Raums, Höhe bleibt erhalten.
    a.markierungen[1].hoehenzone_von = Some(raum);
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
    let r = core.planaufmass_raumbuch(p.id, aid, 2, r).await.unwrap();
    assert_eq!(r.raeume.len(), 1);
    let z = &r.raeume[0].hoehenzonen[0];
    assert_eq!(
        (z.id, z.flaeche_m2, z.lichte_hoehe_min_m),
        (zone, 10., Some(1.0))
    );
    assert!(
        r.planbindungen
            .iter()
            .any(|b| b.ziel == format!("raeume.{raum}.hoehenzonen.{zone}.flaeche_m2"))
    );
    assert!(
        !r.planbindungen
            .iter()
            .any(|b| b.ziel.starts_with(&format!("raeume.{zone}.")))
    );
    let summen = r.summen().unwrap();
    assert_eq!(summen.flaeche_m2, 50.);
    assert_eq!(
        summen.volumen_m3, None,
        "Zone ohne größte Höhe: Rauminhalt offen"
    );
    let mut mit_max = r.clone();
    mit_max.raeume[0].hoehenzonen[0].lichte_hoehe_max_m = Some(2.0);
    // 40 m² × 2,5 m + 10 m² × (1 m + 2 m) / 2
    assert_eq!(mit_max.summen().unwrap().volumen_m3, Some(115.));
    mit_max.raeume[0].hoehenzonen[0].flaeche_m2 = 51.;
    assert!(mit_max.summen().is_err(), "Zonen größer als der Raum");
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

    let mut wohn = core
        .berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Wohnflaechen)
        .await
        .unwrap();
    let pos = wohn.eingaben["positionen"].as_array().unwrap().clone();
    assert_eq!(pos.len(), 2);
    assert_eq!(
        (
            pos[0]["aufmass_m2"].as_f64(),
            pos[0]["lichte_hoehe_m"].as_f64()
        ),
        (Some(40.), Some(2.5))
    );
    assert_eq!(pos[1]["id"], format!("{raum}_{zone}"));
    assert_eq!(
        (
            pos[1]["aufmass_m2"].as_f64(),
            pos[1]["lichte_hoehe_m"].as_f64()
        ),
        (Some(10.), Some(1.0))
    );
    // Mit Raumart und Wohnung: 40 m² voll + 10 m² zur Hälfte.
    let vorlage = &vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Wohnflaechen)
        .unwrap()
        .plan
        .eingaben["positionen"][0];
    for p in wohn.eingaben["positionen"].as_array_mut().unwrap() {
        p["typ"] = vorlage["typ"].clone();
        p["wohnung"] = "WE 1".into();
    }
    let e: core_lib::berechnungen::wohnflaechen::Eingabe =
        serde_json::from_value(wohn.eingaben.clone()).unwrap();
    let ergebnis = core_lib::berechnungen::wohnflaechen::berechnen(&e).unwrap();
    let wf = ergebnis
        .kennzahlen
        .iter()
        .find(|k| k.id == "wohnflaeche")
        .and_then(|k| k.wert)
        .unwrap();
    assert!((wf - 45.).abs() < 1e-9, "{wf}");
}
