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
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
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
            darstellung: None,
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
            flurstueck_id: None,
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
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
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
            darstellung: None,
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
                flurstueck_id: None,
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
                flurstueck_id: None,
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
                flurstueck_id: None,
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
        flurstueck_id: None,
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
        flurstueck_id: None,
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
    // Ein von Hand erfasster Raum gleicher Bezeichnung wird nicht verdoppelt.
    let mut hand = alt.clone();
    let markierung = core
        .planaufmass_raumbuch(p.id, aid, 1, &[], alt.clone())
        .await
        .unwrap()
        .raeume[0]
        .bezeichnung
        .clone();
    let hand_id = Uuid::new_v4();
    hand.raeume.push(core_lib::plattform::objekte::Raum {
        id: hand_id,
        nummer: "1".into(),
        bezeichnung: markierung.to_uppercase(),
        nutzung: "Wohnen".into(),
        flaeche_m2: 48.,
        hoehe_m: Some(2.5),
        hoehenzonen: vec![],
    });
    let fragen = core
        .planaufmass_raumbuch_abgleich(p.id, aid, 1, &hand)
        .await
        .unwrap();
    assert_eq!(fragen.len(), 1);
    assert_eq!(fragen[0].vorschlag, Some(hand_id.to_string()));
    assert_eq!(fragen[0].flaeche_m2, Some(50.));
    assert!(
        core.planaufmass_raumbuch(p.id, aid, 1, &[], hand.clone())
            .await
            .is_err()
    );
    let wahl = [Zeilenzuordnung {
        markierung_id: fragen[0].markierung_id,
        zeile: Some(hand_id.to_string()),
    }];
    let verknuepft = core
        .planaufmass_raumbuch(p.id, aid, 1, &wahl, hand.clone())
        .await
        .unwrap();
    assert_eq!(verknuepft.raeume.len(), 1);
    assert_eq!(verknuepft.raeume[0].id, hand_id);
    assert_eq!(verknuepft.raeume[0].flaeche_m2, 50.);
    assert_eq!(
        verknuepft.raeume[0].nutzung, "Wohnen",
        "Nutzung bleibt Eingabe"
    );
    assert!(
        core.planaufmass_raumbuch_abgleich(p.id, aid, 1, &verknuepft)
            .await
            .unwrap()
            .is_empty()
    );
    let getrennt = core
        .planaufmass_raumbuch(
            p.id,
            aid,
            1,
            &[Zeilenzuordnung {
                markierung_id: fragen[0].markierung_id,
                zeile: None,
            }],
            hand,
        )
        .await
        .unwrap();
    assert_eq!(getrennt.raeume.len(), 2);
    let mut r = core
        .planaufmass_raumbuch(p.id, aid, 1, &[], alt)
        .await
        .unwrap();
    assert_eq!(r.raeume.len(), 1);
    assert_eq!(r.raeume[0].flaeche_m2, 50.);
    assert_eq!(r.raeume[0].nutzung, "Aufenthaltsraum");
    r.raeume[0].nummer = "EG-01".into();
    r.raeume[0].hoehe_m = Some(2.8);
    r = core
        .planaufmass_raumbuch(p.id, aid, 1, &[], r)
        .await
        .unwrap();
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
    let r = core
        .planaufmass_raumbuch(p.id, aid, 2, &[], r)
        .await
        .unwrap();
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
            flurstueck_id: None,
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
        dokument: Some(dokument.clone()),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
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
            darstellung: None,
        }],
        markierungen,
        ausschnitte: vec![
            Planausschnitt {
                ebene_id: None,
                id: x_grund,
                bezeichnung: "Grundriss EG".into(),
                art: Ausschnittart::Grundriss,
                von: [0., 0.],
                bis: [200., 200.],
                kalibrierung: None,
                bezugslinie: None,
            },
            Planausschnitt {
                ebene_id: None,
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
            .find(|x| x.objekt == haus.to_string() && x.ziel == format!("geschosse.{sid}.{feld}"))
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
            .filter(|x| x.objekt == haus.to_string())
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
        .planaufmass_raumbuch(p.id, aid, f.revision, &[], leer)
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
        flurstueck_id: None,
        ausschnitt_id: None,
        hoehenzone_von: von_raum,
    };
    // 10 m × 5 m Raum, davon 10 m × 1 m Schräge zwischen 1 m und 2 m.
    let mut a = Planaufmass {
        bezeichnung: "DG".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
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
            darstellung: None,
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
    let mut r = core
        .planaufmass_raumbuch(p.id, aid, 1, &[], leer)
        .await
        .unwrap();
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
    let r = core
        .planaufmass_raumbuch(p.id, aid, 2, &[], r)
        .await
        .unwrap();
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

/// Flurstücksgrenzen aus Flurkarte oder Lageplan: die gezeichnete Fläche wird
/// neben der Katasterfläche gespeichert und ersetzt sie nur ausdrücklich
/// („Fläche aus Plan“, kein amtlicher Wert). Bindungen zeigen auf den
/// gespeicherten Aufmaßstand; eine geänderte Kontur blockiert das stille
/// Speichern, bis sie übernommen wurde.
#[tokio::test]
async fn flurstuecke_werden_aus_dem_pdf_aufmass_uebernommen() {
    use core_lib::{
        domain::{Project, ProjectUpdate},
        grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen, informationen},
        projektbereiche::ProjektBereich,
    };
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
    let flurstueck = |id: &str, nummer: &str, flaeche: f64| FlurstuecksInfo {
        id: id.into(),
        gemarkung: "Linden".into(),
        gemarkungsnummer: String::new(),
        flur: "1".into(),
        nummer: nummer.into(),
        flaeche_m2: flaeche,
        grundbuchbezirk: String::new(),
        grundbuchblatt: String::new(),
        laufende_nummer: None,
        grundbuchauszug: None,
        eigentuemer: String::new(),
        eigentuemer_kunden: vec![],
        bemerkung: String::new(),
        rolle: Default::default(),
        planbindung: None,
        planflaeche_m2: None,
        flaeche_aus_plan: false,
        flaeche_offen: false,
    };
    let mut p = core
        .create_project(
            NewProject {
                name: "Flurkarte".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: core_lib::domain::Planungsrecht {
                    grundstuecksinformationen: Some(Grundstuecksinformationen {
                        flurstuecke: vec![
                            flurstueck("a", "10/1", 1000.),
                            flurstueck("b", "10/2", 1.),
                        ],
                    }),
                    ..Default::default()
                },
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
            "Flurkarte.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let (ebene, m_a, m_b, aufmass) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let markierung = |id, name: &str, flurstueck: Option<&str>, bis: [f64; 2]| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung: Planbedeutung::Flurstueck,
        geometrie: Plangeometrie::Rechteck { von: [0., 0.], bis },
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
        flurstueck_id: flurstueck.map(Into::into),
    };
    let mut a = Planaufmass {
        bezeichnung: "Flurkarte".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Flurstücke".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        // 100 × 100 pt = 10 m × 10 m = 100 m²; 100 × 50 pt = 50 m².
        markierungen: vec![
            markierung(m_a, "Flurstück 10/1", Some("a"), [100., 100.]),
            markierung(m_b, "Flurstück 10/2", Some("b"), [100., 50.]),
        ],
        ausschnitte: vec![],
    };
    // Unbekannte Flurstücks-ID wird abgewiesen.
    let mut falsch = a.clone();
    falsch.markierungen[0].flurstueck_id = Some("fremd".into());
    assert!(core.planaufmass_pruefen(p.id, &falsch).await.is_err());
    // Flurstückszuordnung nur mit Bedeutung „Flurstück“.
    falsch = a.clone();
    falsch.markierungen[0].bedeutung = Planbedeutung::Gebaeudegrundflaeche;
    assert!(core.planaufmass_pruefen(p.id, &falsch).await.is_err());
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, aufmass, 0, save(&a), &user)
        .await
        .unwrap();
    // Aufgaben: je Flurstück eine, mit passender Markierung vorgeschlagen.
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let fa = aufgaben
        .iter()
        .find(|x| x.verbraucher == "flurstueck" && x.objekt == "a")
        .unwrap();
    assert_eq!(fa.ziel, "flurstuecke.a.flaeche_m2");
    assert_eq!(fa.markierung, Some(m_a));
    assert_eq!(fa.flurstueck_id.as_deref(), Some("a"));
    assert!(fa.wert_vorhanden && fa.gebunden.is_none());
    let bindung = |m, ziel: String| Planbindung {
        aufmass_id: aufmass,
        revision: f.revision,
        markierung_id: m,
        groesse: Planmessgroesse::Flaeche,
        ziel,
    };
    let mut infos = informationen(&p.planungsrecht);
    // Falsche Markierung (anderes Flurstück) wird abgewiesen.
    infos.flurstuecke[0].planbindung = Some(bindung(m_b, infos.flurstuecke[0].planziel()));
    assert!(
        core.planaufmass_flurstuecke(p.id, infos.clone())
            .await
            .is_err()
    );
    infos.flurstuecke[0].planbindung = Some(bindung(m_a, infos.flurstuecke[0].planziel()));
    infos.flurstuecke[1].planbindung = Some(bindung(m_b, infos.flurstuecke[1].planziel()));
    infos.flurstuecke[1].flaeche_aus_plan = true;
    let infos = core.planaufmass_flurstuecke(p.id, infos).await.unwrap();
    // Katasterfläche bleibt Rechenwert, die Zeichnung steht daneben.
    assert_eq!(infos.flurstuecke[0].flaeche_m2, 1000.);
    assert_eq!(infos.flurstuecke[0].planflaeche_m2, Some(100.));
    assert_eq!(infos.flurstuecke[0].flaechenabweichung_m2(), Some(-900.));
    // Ohne amtlichen Wert liefert die Zeichnung die Fläche.
    assert_eq!(infos.flurstuecke[1].flaeche_m2, 50.);
    assert_eq!(infos.flurstuecke[1].flaechenabweichung_m2(), None);
    let speichern = |p: &Project, infos: Grundstuecksinformationen| ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(core_lib::domain::Planungsrecht {
            grundstuecksinformationen: Some(infos),
            ..p.planungsrecht.clone()
        }),
    };
    // Eine Oberfläche darf die gezeichnete Fläche nicht selbst eintragen.
    let mut manipuliert = infos.clone();
    manipuliert.flurstuecke[1].flaeche_m2 = 51.;
    assert!(
        core.update_project(speichern(&p, manipuliert), &user)
            .await
            .is_err()
    );
    p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            speichern(&p, infos.clone()),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(p.grundstuecksflaeche_m2, Some(1050.));
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    assert!(
        aufgaben
            .iter()
            .filter(|x| x.verbraucher == "flurstueck")
            .all(|x| x.gebunden.is_some())
    );
    assert!(
        core.flurstueck_planquellen_pruefen(p.id, &informationen(&p.planungsrecht))
            .await
            .is_ok()
    );
    // Geänderte Kontur: neuer Aufmaßstand, Flurstücke bleiben auf dem alten
    // Stand, bis sie ausdrücklich übernommen werden; stilles Speichern scheitert.
    a.markierungen[1].geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [100., 60.],
    };
    let f2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let e = core
        .flurstueck_planquellen_pruefen(p.id, &informationen(&p.planungsrecht))
        .await
        .unwrap_err();
    assert!(e.to_string().contains("Aufmaßstand"), "{e}");
    let mut alt = informationen(&p.planungsrecht);
    alt.flurstuecke[0].bemerkung = "Nur eine Bemerkung".into();
    assert!(
        core.update_project(speichern(&p, alt.clone()), &user)
            .await
            .is_err()
    );
    let uebersicht = core.projektuebersicht(p.id, &user).await.unwrap();
    assert!(
        uebersicht
            .punkte
            .iter()
            .any(|x| x.id == "flurstueck_aufmass"),
        "{:?}",
        uebersicht.punkte
    );
    // Übernahme des neuen Stands: Flurstück „a“ ist unverändert (Bindung bleibt
    // auf alter Revision gültig), „b“ erhält 60 m².
    for f in &mut alt.flurstuecke {
        if let Some(b) = &mut f.planbindung {
            b.revision = f2.revision;
        }
    }
    let neu = core.planaufmass_flurstuecke(p.id, alt).await.unwrap();
    assert_eq!(neu.flurstuecke[1].flaeche_m2, 60.);
    p = core
        .update_project(speichern(&p, neu), &user)
        .await
        .unwrap();
    assert_eq!(p.grundstuecksflaeche_m2, Some(1060.));
    // Verknüpfung lösen: der Wert bleibt als manuelle Angabe stehen.
    let mut geloest = informationen(&p.planungsrecht);
    geloest.flurstuecke[1].planbindung = None;
    let geloest = core.planaufmass_flurstuecke(p.id, geloest).await.unwrap();
    assert_eq!(geloest.flurstuecke[1].flaeche_m2, 60.);
    assert!(
        !geloest.flurstuecke[1].flaeche_aus_plan && geloest.flurstuecke[1].planflaeche_m2.is_none()
    );
    core.update_project(speichern(&p, geloest), &user)
        .await
        .unwrap();
    // Bestandsdaten ohne die neuen Felder bleiben lesbar.
    let alt: FlurstuecksInfo = serde_json::from_str(
        r#"{"id":"x","gemarkung":"L","flur":"1","nummer":"5","flaeche_m2":12.0}"#,
    )
    .unwrap();
    assert!(alt.planbindung.is_none() && !alt.flaeche_aus_plan);
}

#[tokio::test]
async fn abzuege_und_bauteile_werden_aus_dem_lageplan_uebernommen() {
    use core_lib::{
        baugrundstuecke::{BaugrundstueckBerechnung, einheitsschluessel, musterplan},
        domain::{BebauungsplanArt, Planungsrecht, Project, ProjectUpdate},
        grundstuecksinformationen::{
            BaugrundstuecksErmittlung, FlurstuecksAnsatz, FlurstuecksInfo,
            Grundstuecksinformationen, ermittlung, plan,
        },
        plattform::einbindung::grundstueck_id,
        projektbereiche::ProjektBereich,
    };
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
    let speichern = |p: &Project, recht: Planungsrecht| ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(recht),
    };
    let mut p = core
        .create_project(
            NewProject {
                name: "Lageplan".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    grundstuecksinformationen: Some(Grundstuecksinformationen {
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
                            planbindung: None,
                            planflaeche_m2: None,
                            flaeche_aus_plan: false,
                            flaeche_offen: false,
                        }],
                    }),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let schluessel = einheitsschluessel(&["a".into()]);
    let mut recht = p.planungsrecht.clone();
    recht.baugrundstuecksermittlung = Some(BaugrundstuecksErmittlung {
        ansaetze: vec![FlurstuecksAnsatz {
            flurstueck_id: "a".into(),
            abzuege: vec![],
            abweichende_zuordnung: None,
        }],
        berechnungen: vec![BaugrundstueckBerechnung {
            schluessel: schluessel.clone(),
            eingabe: musterplan().berechnungen[0].eingabe.clone(),
            eigene_festsetzungen: None,
            flurstueck_ids: vec![],
        }],
        quellen_hash: None,
    });
    p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecke,
            speichern(&p, recht),
            &user,
        )
        .await
        .unwrap();
    let grundstueck = grundstueck_id(p.id, &schluessel);
    let doc = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let (ebene, m_gruen, m_zufahrt, aufmass) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let markierung = |id, name: &str, bedeutung, bis: [f64; 2]| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung,
        geometrie: Plangeometrie::Rechteck { von: [0., 0.], bis },
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
        flurstueck_id: None,
    };
    // 100 × 100 pt = 10 m × 10 m = 100 m²; 100 × 50 pt = 50 m².
    let mut gruen = markierung(
        m_gruen,
        "Private Grünfläche",
        Planbedeutung::PrivateGruenflaeche,
        [100., 100.],
    );
    gruen.flurstueck_id = Some("a".into());
    let mut zufahrt = markierung(m_zufahrt, "Zufahrt", Planbedeutung::Zufahrt, [100., 50.]);
    zufahrt.grundstueck_id = Some(grundstueck);
    let mut a = Planaufmass {
        bezeichnung: "Lageplan".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Lageplan".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen: vec![gruen, zufahrt],
        ausschnitte: vec![],
    };
    // Flurstückszuordnung gilt für Abzugsflächen, nicht für Bauteile.
    let mut falsch = a.clone();
    falsch.markierungen[1].flurstueck_id = Some("a".into());
    assert!(core.planaufmass_pruefen(p.id, &falsch).await.is_err());
    core.planaufmass_pruefen(p.id, &a).await.unwrap();
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, aufmass, 0, save(&a), &user)
        .await
        .unwrap();
    let stand = Aufmassstand {
        aufmass_id: aufmass,
        revision: f.revision,
    };
    // Veralteter Aufmaßstand wird nicht übernommen.
    let e = ermittlung(&p.planungsrecht).unwrap().unwrap();
    assert!(
        core.planaufmass_abzuege(
            p.id,
            Some(Aufmassstand {
                revision: f.revision + 1,
                ..stand
            }),
            &[],
            e.clone()
        )
        .await
        .is_err()
    );
    // Abgleich: eine von Hand erfasste private Grünfläche wird nicht verdoppelt.
    let handabzug = |art: &str, flaeche_m2: f64| core_lib::baugrundstuecke::FlaechenAbzug {
        id: None,
        art: art.into(),
        flaeche_m2,
        grundlage: "Festsetzung".into(),
        planbindung: None,
    };
    let mut hand = e.clone();
    hand.ansaetze[0].abzuege = vec![
        handabzug("verkehrsflaeche", 20.),
        handabzug("private_gruenflaeche", 98.),
    ];
    let fragen = core
        .planaufmass_abzuege_abgleich(p.id, stand, &hand)
        .await
        .unwrap();
    assert_eq!(fragen.len(), 1);
    assert_eq!(fragen[0].markierung_id, m_gruen);
    assert_eq!(fragen[0].flaeche_m2, Some(100.));
    assert_eq!(fragen[0].kandidaten.len(), 1, "nur gleiche Abzugsart");
    assert_eq!(fragen[0].vorschlag.as_deref(), Some("#1"));
    // Ohne Entscheidung wird weder verknüpft noch eine zweite Zeile angelegt.
    assert!(
        core.planaufmass_abzuege(p.id, Some(stand), &[], hand.clone())
            .await
            .is_err()
    );
    // Eine Zeile anderer Art ist keine zulässige Wahl.
    let falsch = [Zeilenzuordnung {
        markierung_id: m_gruen,
        zeile: Some("#0".into()),
    }];
    assert!(
        core.planaufmass_abzuege(p.id, Some(stand), &falsch, hand.clone())
            .await
            .is_err()
    );
    // Verknüpfen: die Handzeile übernimmt die gezeichnete Fläche, Grundlage bleibt.
    let wahl = [Zeilenzuordnung {
        markierung_id: m_gruen,
        zeile: fragen[0].vorschlag.clone(),
    }];
    let verknuepft = core
        .planaufmass_abzuege(p.id, Some(stand), &wahl, hand.clone())
        .await
        .unwrap();
    let zeilen = &verknuepft.ansaetze[0].abzuege;
    assert_eq!(zeilen.len(), 2);
    assert_eq!(zeilen[1].flaeche_m2, 100.);
    assert_eq!(zeilen[1].grundlage, "Festsetzung");
    let id = zeilen[1]
        .id
        .clone()
        .expect("Kennung für die verknüpfte Altzeile");
    let bindung = zeilen[1].planbindung.as_ref().unwrap();
    assert_eq!(bindung.markierung_id, m_gruen);
    assert_eq!(bindung.ziel, format!("abzuege.{id}.flaeche_m2"));
    assert!(zeilen[0].planbindung.is_none() && zeilen[0].id.is_none());
    // Erneute Übernahme findet die verknüpfte Zeile ohne Rückfrage wieder.
    assert!(
        core.planaufmass_abzuege_abgleich(p.id, stand, &verknuepft)
            .await
            .unwrap()
            .is_empty()
    );
    let nochmal = core
        .planaufmass_abzuege(p.id, Some(stand), &[], verknuepft.clone())
        .await
        .unwrap();
    assert_eq!(nochmal, verknuepft);
    // Bewusst als eigene Zeile: Handzeile bleibt ungebunden daneben stehen.
    let getrennt = core
        .planaufmass_abzuege(
            p.id,
            Some(stand),
            &[Zeilenzuordnung {
                markierung_id: m_gruen,
                zeile: None,
            }],
            hand,
        )
        .await
        .unwrap();
    assert_eq!(getrennt.ansaetze[0].abzuege.len(), 3);
    // Abzugszeile aus der Markierung: Art folgt der Bedeutung, Fläche aus dem Plan,
    // rechtliche Grundlage bleibt Eingabe.
    let mut neu = core
        .planaufmass_abzuege(p.id, Some(stand), &[], e.clone())
        .await
        .unwrap();
    let abzug = &neu.ansaetze[0].abzuege[0];
    assert_eq!(abzug.id.as_deref(), Some(m_gruen.to_string().as_str()));
    assert_eq!(abzug.art, "private_gruenflaeche");
    assert_eq!(abzug.flaeche_m2, 100.);
    assert!(abzug.grundlage.is_empty() && abzug.planbindung.is_some());
    // Erneute Übernahme legt keine zweite Zeile an.
    let nochmal = core
        .planaufmass_abzuege(p.id, Some(stand), &[], neu.clone())
        .await
        .unwrap();
    assert_eq!(nochmal.ansaetze[0].abzuege.len(), 1);
    // Bindung an ein anderes Ziel oder eine fremde Art wird abgewiesen.
    let mut manipuliert = neu.clone();
    manipuliert.ansaetze[0].abzuege[0].art = "verkehrsflaeche".into();
    assert!(
        core.planaufmass_abzuege(p.id, None, &[], manipuliert)
            .await
            .is_err()
    );
    neu.ansaetze[0].abzuege[0].grundlage = "Planfestsetzung private Grünfläche".into();
    // Eine Oberfläche darf die gezeichnete Fläche nicht selbst eintragen.
    let mut manipuliert = neu.clone();
    manipuliert.ansaetze[0].abzuege[0].flaeche_m2 = 101.;
    let mut recht = p.planungsrecht.clone();
    recht.baugrundstuecksermittlung = Some(manipuliert);
    assert!(
        core.update_project(speichern(&p, recht), &user)
            .await
            .is_err()
    );
    let mut recht = p.planungsrecht.clone();
    recht.baugrundstuecksermittlung = Some(neu);
    p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecke,
            speichern(&p, recht),
            &user,
        )
        .await
        .unwrap();
    let gruppen = plan(&p.planungsrecht)
        .unwrap()
        .unwrap()
        .gruppen(true)
        .unwrap();
    assert_eq!(gruppen[0].abzuege_m2, 100.);
    assert_eq!(gruppen[0].bauland_m2, 900.);
    // Aufgabe je Abzugszeile, bereits verknüpft.
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let ab = aufgaben
        .iter()
        .find(|x| x.verbraucher == "abzug")
        .expect("Abzugsaufgabe");
    assert_eq!(ab.objekt, m_gruen.to_string());
    assert_eq!(ab.gebunden, Some(m_gruen));
    assert_eq!(ab.flurstueck_id.as_deref(), Some("a"));
    // Vor dem Entfernen des Flurstücks nennt die Verweisliste die Markierung.
    let verweise = core
        .flurstuecksverweise(p.id, &["a".to_string()])
        .await
        .unwrap();
    assert!(
        verweise
            .iter()
            .any(|v| v.bereich == "planaufmass" && v.objekt == aufmass),
        "{verweise:?}"
    );
    // GRZ-Bauteil aus der Zufahrt, dem Baugrundstück zugeordnet.
    let e = ermittlung(&p.planungsrecht).unwrap().unwrap();
    let aufstellung = e.berechnungen[0].eingabe.aufstellung.clone().unwrap();
    assert!(
        core.planaufmass_bauteile(p.id, "fremd", Some(stand), &[], aufstellung.clone())
            .await
            .is_err()
    );
    let bauteile = core
        .planaufmass_bauteile(p.id, &schluessel, Some(stand), &[], aufstellung)
        .await
        .unwrap();
    let zufahrt = bauteile
        .bauteile
        .iter()
        .find(|b| b.id.as_deref() == Some(m_zufahrt.to_string().as_str()))
        .expect("Bauteil aus dem Plan");
    assert_eq!(zufahrt.typ, core_lib::flaechen::Bauteiltyp::Zufahrt);
    assert_eq!(zufahrt.flaeche_m2, 50.);
    assert!(zufahrt.gebaeude_id.is_none() && zufahrt.planbindung.is_some());
    assert_eq!(bauteile.bauteile.len(), 2);
    // Abgleich: eine von Hand erfasste Zufahrt übernimmt die gezeichnete Fläche.
    let mut hand = e.berechnungen[0].eingabe.aufstellung.clone().unwrap();
    hand.bauteile.push(core_lib::flaechen::Bauteil {
        id: None,
        planbindung: None,
        gebaeude_id: None,
        bezeichnung: "Zufahrt Hof".into(),
        typ: core_lib::flaechen::Bauteiltyp::Zufahrt,
        flaeche_m2: 48.,
        abweichende_anrechnung: None,
        begruendung: String::new(),
    });
    let fragen = core
        .planaufmass_bauteile_abgleich(p.id, &schluessel, stand, &hand)
        .await
        .unwrap();
    assert_eq!(fragen.len(), 1);
    assert_eq!(fragen[0].markierung_id, m_zufahrt);
    let hand_zeile = format!("#{}", hand.bauteile.len() - 1);
    assert_eq!(fragen[0].vorschlag.as_deref(), Some(hand_zeile.as_str()));
    assert!(
        core.planaufmass_bauteile(p.id, &schluessel, Some(stand), &[], hand.clone())
            .await
            .is_err()
    );
    let verknuepft = core
        .planaufmass_bauteile(
            p.id,
            &schluessel,
            Some(stand),
            &[Zeilenzuordnung {
                markierung_id: m_zufahrt,
                zeile: Some(hand_zeile),
            }],
            hand.clone(),
        )
        .await
        .unwrap();
    let zufahrten: Vec<_> = verknuepft
        .bauteile
        .iter()
        .filter(|b| b.typ == core_lib::flaechen::Bauteiltyp::Zufahrt)
        .collect();
    assert_eq!(zufahrten.len(), 1);
    assert_eq!(zufahrten[0].flaeche_m2, 50.);
    assert_eq!(zufahrten[0].bezeichnung, "Zufahrt Hof");
    assert!(
        core.planaufmass_bauteile_abgleich(p.id, &schluessel, stand, &verknuepft)
            .await
            .unwrap()
            .is_empty()
    );
    let mut b = e.berechnungen[0].clone();
    b.eingabe.aufstellung = Some(bauteile);
    // Einzelspeicherung prüft die Bindung ebenfalls.
    let mut manipuliert = b.clone();
    manipuliert.eingabe.aufstellung.as_mut().unwrap().bauteile[1].flaeche_m2 = 51.;
    assert!(
        core.baugrundstueck_einzeln_speichern(p.id, p.version, manipuliert, &user)
            .await
            .is_err()
    );
    p = core
        .baugrundstueck_einzeln_speichern(p.id, p.version, b, &user)
        .await
        .unwrap();
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let bt = aufgaben
        .iter()
        .find(|x| x.verbraucher == "bauteil")
        .expect("Bauteilaufgabe");
    assert_eq!(bt.gebunden, Some(m_zufahrt));
    assert_eq!(bt.grundstueck_id, Some(grundstueck));
    // Geänderte Kontur: stilles Speichern scheitert, die Übersicht zeigt den Punkt,
    // die ausdrückliche Übernahme des neuen Stands setzt 60 m² ein.
    a.markierungen[1].geometrie = Plangeometrie::Rechteck {
        von: [0., 0.],
        bis: [100., 60.],
    };
    let f2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let e = ermittlung(&p.planungsrecht).unwrap().unwrap();
    assert!(core.abzug_planquellen_pruefen(p.id, &e).await.is_ok());
    let err = core
        .grundstueck_planquellen_pruefen(p.id, &e)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Aufmaßstand"), "{err}");
    let mut recht = p.planungsrecht.clone();
    recht.festgesetzte_werte.grz = Some(0.4);
    assert!(
        core.update_project(speichern(&p, recht), &user)
            .await
            .is_err()
    );
    let uebersicht = core.projektuebersicht(p.id, &user).await.unwrap();
    assert!(
        uebersicht
            .punkte
            .iter()
            .any(|x| x.id.starts_with("bauteil_aufmass:")),
        "{:?}",
        uebersicht.punkte
    );
    let mut aufstellung = e.berechnungen[0].eingabe.aufstellung.clone().unwrap();
    for b in &mut aufstellung.bauteile {
        if let Some(pb) = &mut b.planbindung {
            pb.revision = f2.revision;
        }
    }
    let aufstellung = core
        .planaufmass_bauteile(p.id, &schluessel, None, &[], aufstellung)
        .await
        .unwrap();
    assert_eq!(aufstellung.bauteile[1].flaeche_m2, 60.);
    let mut b = e.berechnungen[0].clone();
    b.eingabe.aufstellung = Some(aufstellung);
    p = core
        .baugrundstueck_einzeln_speichern(p.id, p.version, b, &user)
        .await
        .unwrap();
    // Verknüpfung lösen: der Wert bleibt als manuelle Angabe stehen.
    let e = ermittlung(&p.planungsrecht).unwrap().unwrap();
    let mut aufstellung = e.berechnungen[0].eingabe.aufstellung.clone().unwrap();
    aufstellung.bauteile[1].planbindung = None;
    let aufstellung = core
        .planaufmass_bauteile(p.id, &schluessel, None, &[], aufstellung)
        .await
        .unwrap();
    assert_eq!(aufstellung.bauteile[1].flaeche_m2, 60.);
    let mut b = e.berechnungen[0].clone();
    b.eingabe.aufstellung = Some(aufstellung);
    core.baugrundstueck_einzeln_speichern(p.id, p.version, b, &user)
        .await
        .unwrap();
    // Bestandsdaten ohne Kennung und Bindung bleiben lesbar.
    let alt: core_lib::baugrundstuecke::FlaechenAbzug =
        serde_json::from_str(r#"{"art":"sonstiger_abzug","flaeche_m2":1.0,"grundlage":"x"}"#)
            .unwrap();
    assert!(alt.id.is_none() && alt.planbindung.is_none());
}

#[tokio::test]
async fn projektuebersicht_fuehrt_ohne_flurstuecke_ueber_die_planablage() {
    use core_lib::{
        domain::{Planungsrecht, ProjectUpdate},
        grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen},
        projektbereiche::ProjektBereich,
    };
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
                name: "Start".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let punkte = |u: core_lib::projektuebersicht::Projektuebersicht| u.punkte;
    let u = punkte(core.projektuebersicht(p.id, &user).await.unwrap());
    assert!(
        u.iter()
            .any(|x| x.id == "planablage" && x.ziel.bereich == "dokumente"),
        "{u:?}"
    );
    assert!(!u.iter().any(|x| x.id == "grundstueck"));
    // Ein Lageplan ohne Kategorie zählt nicht; die kategorisierte Flurkarte führt ins Aufmaß.
    core.store_document(
        p.id,
        None,
        None,
        "Irgendwas.pdf",
        Bytes::from(core_lib::formular::testdaten::musterformular()),
        &user,
    )
    .await
    .unwrap();
    let u = punkte(core.projektuebersicht(p.id, &user).await.unwrap());
    assert!(u.iter().any(|x| x.id == "planablage"));
    let doc = core
        .store_document_kategorisiert(
            p.id,
            "Flurkarte.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            Some("flurkarte".into()),
            &user,
        )
        .await
        .unwrap();
    let u = punkte(core.projektuebersicht(p.id, &user).await.unwrap());
    let z = u
        .iter()
        .find(|x| x.id == "flurstueck_zeichnen")
        .unwrap_or_else(|| panic!("{u:?}"));
    assert_eq!(z.ziel.bereich, "planaufmass");
    assert_eq!(z.ziel.register.as_deref(), Some("unterlage"));
    assert_eq!(z.ziel.eintrag, Some(doc.document_id.0));
    assert!(z.hinweis.contains("Flurkarte.pdf"), "{doc:?}");
    assert!(
        !u.iter()
            .any(|x| x.id == "planablage" || x.id == "grundstueck")
    );
    // Mit erfasstem Flurstück tritt der Planweg zurück; es folgt die Baugrundstücksermittlung.
    let mut recht = p.planungsrecht.clone();
    recht.grundstuecksinformationen = Some(Grundstuecksinformationen {
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
            planbindung: None,
            planflaeche_m2: None,
            flaeche_aus_plan: false,
            flaeche_offen: false,
        }],
    });
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name.clone(),
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
                planungsrecht: Some(recht),
            },
            &user,
        )
        .await
        .unwrap();
    let u = punkte(core.projektuebersicht(p.id, &user).await.unwrap());
    assert!(
        !u.iter()
            .any(|x| x.id == "flurstueck_zeichnen" || x.id == "planablage")
    );
}

/// DXF-Unterlagen werden im selben Aufmaß gemessen wie PDF-Blätter: die
/// Zeichnungseinheit kalibriert, Baufenster und Vorhabenumriss gehen als
/// Lageprüfung ans Projekt, und CAD-Aufmaße der früheren DXF-Ansicht werden
/// einmalig als Aufmaß übernommen.
#[tokio::test]
async fn dxf_unterlage_baufenster_und_cad_uebernahme() {
    use core_lib::cad::{
        Laengeneinheit, Punkt,
        projektaufmass::{CadMarkierung, CadProjektaufmass},
    };
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
                name: "DXF-Aufmaß".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let dxf = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.dxf",
            Bytes::from(core_lib::cad::testdaten::musterplan()),
            &user,
        )
        .await
        .unwrap();
    let pdf = core
        .store_document(
            p.id,
            None,
            None,
            "Blatt.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let mark = |id: u128, name: &str, bedeutung: Planbedeutung, von: Planpunkt, bis: Planpunkt| {
        Planmarkierung {
            id: Uuid::from_u128(id),
            ebene_id: ebene,
            bezeichnung: name.into(),
            bedeutung,
            geometrie: Plangeometrie::Rechteck { von, bis },
            grundstueck_id: None,
            gebaeude_id: None,
            geschoss_id: None,
            flurstueck_id: None,
            ausschnitt_id: None,
            hoehenzone_von: None,
        }
    };
    let mut a = Planaufmass {
        bezeichnung: "Lageplan".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: dxf.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Einheit {
            einheit: Laengeneinheit::Meter,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Lageplan".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen: vec![mark(
            1,
            "Grundstück",
            Planbedeutung::Unbestimmt,
            [0., 0.],
            [20., 30.],
        )],
        ausschnitte: vec![],
    };
    let w = core.planaufmass_pruefen(p.id, &a).await.unwrap();
    assert!((w[0].flaeche_m2.unwrap() - 600.).abs() < 1e-9);
    let mut falsch = a.clone();
    falsch.kalibrierung = Some(Plankalibrierung::Massstab { nenner: 100. });
    let e = core.planaufmass_pruefen(p.id, &falsch).await.unwrap_err();
    assert!(e.to_string().contains("Zeichnungseinheit"), "{e}");
    let mut seite2 = a.clone();
    seite2.seite = 2;
    assert!(core.planaufmass_pruefen(p.id, &seite2).await.is_err());
    let mut auf_pdf = a.clone();
    auf_pdf.dokument = Some(Dokumentbezug {
        dokument_id: pdf.document_id,
        version: 1,
    });
    let e = core.planaufmass_pruefen(p.id, &auf_pdf).await.unwrap_err();
    assert!(e.to_string().contains("nur für DXF"), "{e}");
    let aufmass = Uuid::new_v4();
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, aufmass, 0, save(&a), &user)
        .await
        .unwrap();
    let stand = Aufmassstand {
        aufmass_id: aufmass,
        revision: f.revision,
    };
    let e = core
        .planaufmass_baugrenzen(p.id, stand, &user)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("Baufenster"), "{e}");
    a.markierungen.push(mark(
        2,
        "Baufenster",
        Planbedeutung::Baufenster,
        [2., 2.],
        [18., 28.],
    ));
    a.markierungen.push(mark(
        3,
        "Haus",
        Planbedeutung::Vorhabenumriss,
        [4., 4.],
        [14., 12.],
    ));
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.planaufmass_baugrenzen(p.id, stand, &user)
            .await
            .is_err(),
        "alter Aufmaßstand"
    );
    let stand = Aufmassstand {
        aufmass_id: aufmass,
        revision: f.revision,
    };
    let r = core
        .planaufmass_baugrenzen(p.id, stand, &user)
        .await
        .unwrap();
    assert!(r.pruefung.eingehalten);
    assert!((r.pruefung.vorhaben_m2 - 80.).abs() < 1e-6);
    assert!((r.pruefung.baufenster_m2 - 416.).abs() < 1e-6);
    let g = r.projekt.planungsrecht.baugrenzen.as_ref().unwrap();
    assert_eq!(g.dokument_id, dxf.document_id);
    assert_eq!(g.dokument_version, 1);
    assert_eq!(g.einheit, Laengeneinheit::Meter);
    assert_eq!(g.baufenster.len(), 4);
    assert!(g.grundstueck_schluessel.is_none());
    assert_eq!(
        core.get_project(p.id)
            .await
            .unwrap()
            .planungsrecht
            .baugrenzen
            .as_ref(),
        Some(g)
    );
    a.markierungen[2].geometrie = Plangeometrie::Rechteck {
        von: [10., 4.],
        bis: [22., 12.],
    };
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let r = core
        .planaufmass_baugrenzen(
            p.id,
            Aufmassstand {
                aufmass_id: aufmass,
                revision: f.revision,
            },
            &user,
        )
        .await
        .unwrap();
    assert!(!r.pruefung.eingehalten);
    assert!((r.pruefung.ausserhalb_m2 - 32.).abs() < 1e-6);
    a.markierungen.push(mark(
        4,
        "Baufenster 2",
        Planbedeutung::Baufenster,
        [0., 0.],
        [1., 1.],
    ));
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let e = core
        .planaufmass_baugrenzen(
            p.id,
            Aufmassstand {
                aufmass_id: aufmass,
                revision: f.revision,
            },
            &user,
        )
        .await
        .unwrap_err();
    assert!(e.to_string().contains("mehrere"), "{e}");

    // CAD-Aufmaß der früheren DXF-Ansicht (Zeichnungskoordinaten, y nach oben).
    let cad = CadProjektaufmass {
        dokument: Dokumentbezug {
            dokument_id: dxf.document_id,
            version: 1,
        },
        einheit: Laengeneinheit::Meter,
        markierungen: vec![CadMarkierung {
            id: Uuid::from_u128(9),
            bezeichnung: "Umgriff".into(),
            punkte: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            geschlossen: true,
        }],
    };
    core.cadaufmass_speichern(p.id, cad.clone(), 0, &user)
        .await
        .unwrap();
    let r = core.cadaufmass_uebernehmen(p.id, &user).await.unwrap();
    assert_eq!(r.uebernommen.len(), 1, "{r:?}");
    assert!(r.uebersprungen.is_empty(), "{r:?}");
    assert_eq!(r.vorhanden, 0);
    let uebernommen: Vec<Planaufmass> = core
        .fachobjekte(p.id, ObjektArt::Planaufmass)
        .await
        .unwrap()
        .into_iter()
        .filter(|f| f.objekt != aufmass.to_string())
        .map(|f| serde_json::from_value(f.daten).unwrap())
        .collect();
    assert_eq!(uebernommen.len(), 1);
    let u = &uebernommen[0];
    assert!(u.bezeichnung.contains("Lageplan.dxf"), "{}", u.bezeichnung);
    assert_eq!(u.dokument.as_ref().unwrap().version, 1);
    let grenzen = core
        .cad_plan(dxf.document_id)
        .await
        .unwrap()
        .plan
        .grenzen
        .unwrap();
    assert_eq!(
        u.markierungen[0].geometrie,
        Plangeometrie::Polygon {
            punkte: [[0., 0.], [10., 0.], [10., 10.], [0., 10.]]
                .iter()
                .map(|q| seitenpunkt(Punkt(q[0], q[1]), &grenzen))
                .collect()
        }
    );
    assert_eq!(u.markierungen[0].bedeutung, Planbedeutung::Unbestimmt);
    let w = core.planaufmass_pruefen(p.id, u).await.unwrap();
    assert!((w[0].flaeche_m2.unwrap() - 100.).abs() < 1e-9);
    let r = core.cadaufmass_uebernehmen(p.id, &user).await.unwrap();
    assert!(r.uebernommen.is_empty());
    assert_eq!(r.vorhanden, 1);
    assert!(
        core.cadaufmass_laden(p.id, &cad.dokument)
            .await
            .unwrap()
            .is_some(),
        "Altbestand bleibt lesbar"
    );
}

#[tokio::test]
async fn offene_flurstuecksflaeche_wird_aus_dem_aufmass_ermittelt() {
    use core_lib::{
        domain::{Project, ProjectUpdate},
        grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen, informationen},
        projektbereiche::ProjektBereich,
    };
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
    let flurstueck = |id: &str, nummer: &str, flaeche: f64, offen: bool| FlurstuecksInfo {
        id: id.into(),
        gemarkung: "Linden".into(),
        gemarkungsnummer: String::new(),
        flur: "1".into(),
        nummer: nummer.into(),
        flaeche_m2: flaeche,
        grundbuchbezirk: String::new(),
        grundbuchblatt: String::new(),
        laufende_nummer: None,
        grundbuchauszug: None,
        eigentuemer: String::new(),
        eigentuemer_kunden: vec![],
        bemerkung: String::new(),
        rolle: Default::default(),
        planbindung: None,
        planflaeche_m2: None,
        flaeche_aus_plan: false,
        flaeche_offen: offen,
    };
    // Ein Flurstück ohne Katasterfläche lässt sich mit „Fläche wird im Aufmaß ermittelt“ speichern.
    let p = core
        .create_project(
            NewProject {
                name: "Offene Fläche".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(999.),
                planungsrecht: core_lib::domain::Planungsrecht {
                    grundstuecksinformationen: Some(Grundstuecksinformationen {
                        flurstuecke: vec![
                            flurstueck("a", "10/1", 1000., false),
                            flurstueck("b", "10/2", 0., true),
                        ],
                    }),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        p.grundstuecksflaeche_m2, None,
        "keine Teilsumme als Grundstücksfläche"
    );
    let doc = core
        .store_document(
            p.id,
            None,
            None,
            "Flurkarte.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let (ebene, m_b, aufmass) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let a = Planaufmass {
        bezeichnung: "Flurkarte".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Flurstücke".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        // 100 × 50 pt = 10 m × 5 m = 50 m².
        markierungen: vec![Planmarkierung {
            id: m_b,
            ebene_id: ebene,
            bezeichnung: "Flurstück 10/2".into(),
            bedeutung: Planbedeutung::Flurstueck,
            geometrie: Plangeometrie::Rechteck {
                von: [0., 0.],
                bis: [100., 50.],
            },
            grundstueck_id: None,
            gebaeude_id: None,
            geschoss_id: None,
            ausschnitt_id: None,
            hoehenzone_von: None,
            flurstueck_id: Some("b".into()),
        }],
        ausschnitte: vec![],
    };
    // Das Aufmaß arbeitet auch bei offener Fläche.
    assert!(core.planaufmass_pruefen(p.id, &a).await.is_ok());
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            0,
            serde_json::to_value(&a).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let aufgaben = core.planaufmass_aufgaben(p.id, &a).await.unwrap();
    let fb = aufgaben
        .iter()
        .find(|x| x.verbraucher == "flurstueck" && x.objekt == "b")
        .unwrap();
    assert_eq!(fb.markierung, Some(m_b));
    assert!(
        !fb.wert_vorhanden,
        "offene Fläche ist kein vorhandener Wert"
    );
    // Übernahme wie in der Oberfläche: Bindung, ohne amtlichen Wert „aus Plan“.
    let mut infos = informationen(&p.planungsrecht);
    infos.flurstuecke[1].planbindung = Some(Planbindung {
        aufmass_id: aufmass,
        revision: f.revision,
        markierung_id: m_b,
        groesse: Planmessgroesse::Flaeche,
        ziel: infos.flurstuecke[1].planziel(),
    });
    let infos = core.planaufmass_flurstuecke(p.id, infos).await.unwrap();
    let b = &infos.flurstuecke[1];
    assert!(!b.flaeche_offen && b.flaeche_aus_plan);
    assert_eq!((b.flaeche_m2, b.planflaeche_m2), (50., Some(50.)));
    let p: Project = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name.clone(),
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
                planungsrecht: Some(core_lib::domain::Planungsrecht {
                    grundstuecksinformationen: Some(infos),
                    ..p.planungsrecht.clone()
                }),
            },
            &user,
        )
        .await
        .unwrap();
    assert_eq!(p.grundstuecksflaeche_m2, Some(1050.));
}

#[tokio::test]
async fn abstandsflaechen_linien_aus_dem_lageplan() {
    use core_lib::{
        abstandsflaechen::{APunkt, Aussenrichtung},
        grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen},
    };
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
                name: "Lageplan Abstandsflächen".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: core_lib::domain::Planungsrecht {
                    grundstuecksinformationen: Some(Grundstuecksinformationen {
                        flurstuecke: vec![FlurstuecksInfo {
                            id: "a".into(),
                            gemarkung: "Linden".into(),
                            gemarkungsnummer: String::new(),
                            flur: "1".into(),
                            nummer: "10/1".into(),
                            flaeche_m2: 900.,
                            grundbuchbezirk: String::new(),
                            grundbuchblatt: String::new(),
                            laufende_nummer: None,
                            grundbuchauszug: None,
                            eigentuemer: String::new(),
                            eigentuemer_kunden: vec![],
                            bemerkung: String::new(),
                            rolle: Default::default(),
                            planbindung: None,
                            planflaeche_m2: None,
                            flaeche_aus_plan: false,
                            flaeche_offen: false,
                        }],
                    }),
                    ..Default::default()
                },
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
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let (ebene, aufmass) = (Uuid::new_v4(), Uuid::new_v4());
    let (m_f, m_u, m_a1, m_a2) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let markierung = |id, name: &str, bedeutung, geometrie| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung,
        geometrie,
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
        flurstueck_id: (bedeutung == Planbedeutung::Flurstueck).then(|| "a".into()),
    };
    // 100 pt = 10 m. Flurstück 30 × 30 m, Gebäude 10 × 10 m in der Mitte;
    // A1 an der Nordseite (oben im Blatt), A2 an der Ostseite.
    let mut a = Planaufmass {
        bezeichnung: "Lageplan".into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Abstandsflächen".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen: vec![
            markierung(
                m_f,
                "Flurstück 10/1",
                Planbedeutung::Flurstueck,
                Plangeometrie::Rechteck {
                    von: [0., 0.],
                    bis: [300., 300.],
                },
            ),
            markierung(
                m_u,
                "Wohnhaus",
                Planbedeutung::Vorhabenumriss,
                Plangeometrie::Rechteck {
                    von: [100., 100.],
                    bis: [200., 200.],
                },
            ),
            markierung(
                m_a1,
                "A1",
                Planbedeutung::Abstandsflaeche,
                Plangeometrie::Strecke {
                    von: [100., 100.],
                    bis: [200., 100.],
                },
            ),
            markierung(
                m_a2,
                "A2",
                Planbedeutung::Abstandsflaeche,
                Plangeometrie::Strecke {
                    von: [200., 200.],
                    bis: [200., 100.],
                },
            ),
        ],
        ausschnitte: vec![],
    };
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, aufmass, 0, save(&a), &user)
        .await
        .unwrap();
    // Die Werkzeugkiste bietet die Linie an.
    assert!(
        werkzeugkiste()
            .iter()
            .any(|v| v.bedeutung == Planbedeutung::Abstandsflaeche
                && v.geometrie == Aufgabengeometrie::Strecke)
    );
    let mut plan = core_lib::abstandsflaechen::testdaten::plan();
    plan.waende.clear();
    plan.grundstuecke.clear();
    plan.koordinatenbezug.clear();
    let stand = |revision| Aufmassstand {
        aufmass_id: aufmass,
        revision,
    };
    // Veralteter Stand wird abgewiesen.
    assert!(
        core.abstandslinien_uebernehmen(p.id, plan.clone(), stand(f.revision + 1))
            .await
            .is_err()
    );
    let u = core
        .abstandslinien_uebernehmen(p.id, plan.clone(), stand(f.revision))
        .await
        .unwrap();
    // Echte Übernahme für den Oberflächentest (apps/desktop/tests/abstandsflaechen.py).
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(
            out.join("linien.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "aufmass": {"objekt": aufmass, "revision": f.revision, "daten": a},
                "uebernahme": u,
            }))
            .unwrap(),
        )
        .unwrap();
    }
    let plan = u.plan;
    assert!(u.hinweise.is_empty(), "{:?}", u.hinweise);
    let namen: Vec<_> = plan.waende.iter().map(|w| w.bezeichnung.as_str()).collect();
    assert_eq!(namen, ["A1", "A2"]);
    let (a1, a2) = (&plan.waende[0], &plan.waende[1]);
    let nah = |p: &APunkt, x: f64, y: f64| (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9;
    assert!(nah(&a1.von, 10., -10.) && nah(&a1.bis, 20., -10.));
    assert!(nah(&a2.von, 20., -20.) && nah(&a2.bis, 20., -10.));
    // Außenseite zeigt vom Vorhabenumriss weg: A1 nach Norden, A2 nach Osten.
    assert_eq!(a1.aussen, Aussenrichtung::Links);
    assert_eq!(a2.aussen, Aussenrichtung::Rechts);
    assert_eq!(plan.grundstuecke.len(), 1);
    let g = &plan.grundstuecke[0];
    assert_eq!(g.flurstueck_ids, ["a"]);
    assert_eq!(g.grenze.len(), 4);
    assert!(g.grundlage.contains("Markierung „Flurstück 10/1“"));
    assert!(plan.waende.iter().all(|w| w.grundstueck_id == Some(g.id)));
    assert!(a1.hoehenquelle.contains("Lage: Aufmaß „Lageplan“"));
    assert!(plan.koordinatenbezug.starts_with("Lageplan „Lageplan“"));
    assert_eq!(plan.planbindungen.len(), 3);
    assert!(
        plan.quellen
            .iter()
            .any(|q| q.dokument_id == doc.document_id)
    );
    core.abstands_planquellen_pruefen(p.id, &plan)
        .await
        .unwrap();
    // Ziele für die Anzeige der Verknüpfungen.
    let ziele = abstandsziele(&plan);
    assert!(
        ziele
            .iter()
            .any(|z| z.ziel == format!("waende.{}.lage", a1.id))
    );
    assert!(
        ziele
            .iter()
            .any(|z| z.ziel == format!("grundstuecke.{}.grenze", g.id))
    );
    // Erneute Übernahme ändert nichts.
    let again = core
        .abstandslinien_uebernehmen(p.id, plan.clone(), stand(f.revision))
        .await
        .unwrap();
    assert_eq!(again.plan, plan);

    // Abstandsflächenplan: Ergebnis zurück auf die Lageplanseite.
    let mut mit_hoehen = plan.clone();
    for w in &mut mit_hoehen.waende {
        (w.gelaende_a_m, w.gelaende_b_m) = (Some(100.), Some(100.));
        (w.abschluss_a_m, w.abschluss_b_m) = (Some(106.), Some(106.));
        w.dachart = core_lib::abstandsflaechen::Dachart::Flach;
    }
    let r = core_lib::abstandsflaechen::berechnen(&mit_hoehen).unwrap();
    assert!(r.waende.iter().all(|e| e.polygon.len() >= 3), "{r:?}");
    let pdf = core
        .abstandsflaechenplan(p.id, &mit_hoehen, &r)
        .await
        .unwrap();
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        std::fs::write(
            std::path::Path::new(&out).join("abstandsflaechenplan.pdf"),
            &pdf,
        )
        .unwrap();
    }
    let d = lopdf::Document::load_mem(&pdf).unwrap();
    assert!(d.get_pages().len() >= 2);
    let seite = *d.get_pages().values().next().unwrap();
    let inhalt = String::from_utf8_lossy(&d.get_page_content(seite)).to_string();
    assert!(
        inhalt.contains("/BauDeckkraft20 gs") && inhalt.contains("B*"),
        "{inhalt}"
    );
    // Ohne Linien im Lageplan gibt es keinen Abstandsflächenplan.
    let mut ohne = mit_hoehen.clone();
    ohne.planbindungen.clear();
    assert!(core.abstandsflaechenplan(p.id, &ohne, &r).await.is_err());

    // Linie verschoben: Prüfung meldet die geänderte Grundlage, neue
    // Übernahme zieht die Wand nach.
    a.markierungen[2].geometrie = Plangeometrie::Strecke {
        von: [100., 100.],
        bis: [250., 100.],
    };
    let f2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.abstands_planquellen_pruefen(p.id, &plan)
            .await
            .is_err()
    );
    let plan = core
        .abstandslinien_uebernehmen(p.id, plan, stand(f2.revision))
        .await
        .unwrap()
        .plan;
    assert!(nah(&plan.waende[0].bis, 25., -10.));
    assert!(plan.planbindungen.iter().all(|b| b.revision == f2.revision));

    // Eine von Hand angelegte Wand mit gleichem Namen blockiert neue Linien.
    let mut doppelt = plan.clone();
    doppelt.planbindungen.retain(|b| b.markierung_id != m_a2);
    doppelt.waende.retain(|w| w.bezeichnung != "A2");
    let mut hand = doppelt.waende[0].clone();
    hand.id = Uuid::new_v4();
    hand.bezeichnung = "A2".into();
    doppelt.waende.push(hand);
    assert!(
        core.abstandslinien_uebernehmen(p.id, doppelt, stand(f2.revision))
            .await
            .is_err()
    );

    // Linie gelöscht: Wand bleibt mit ihrer letzten Lage, die Bindung entfällt.
    a.markierungen.retain(|m| m.id != m_a2);
    let f3 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aufmass,
            f2.revision,
            save(&a),
            &user,
        )
        .await
        .unwrap();
    let u = core
        .abstandslinien_uebernehmen(p.id, plan, stand(f3.revision))
        .await
        .unwrap();
    assert_eq!(u.plan.waende.len(), 2);
    assert!(
        u.hinweise
            .iter()
            .any(|h| h.starts_with("A2: Markierung im Lageplan gelöscht"))
    );
    assert!(!u.plan.planbindungen.iter().any(|b| b.markierung_id == m_a2));
}

#[tokio::test]
async fn abstandsflaechen_mit_ansichten_verknuepfen() {
    use core_lib::abstandsflaechen::{AbstandsStatus, berechnen};
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
                name: "Ansichten".into(),
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
            "Plaene.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let ebene = Uuid::new_v4();
    let linie = |id, name: &str, von, bis, ausschnitt| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung: Planbedeutung::Abstandsflaeche,
        geometrie: Plangeometrie::Strecke { von, bis },
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: ausschnitt,
        hoehenzone_von: None,
        flurstueck_id: None,
    };
    let aufmass = |name: &str, markierungen, ausschnitte| Planaufmass {
        bezeichnung: name.into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        // 100 pt = 10 m.
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Abstandsflächen".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen,
        ausschnitte,
    };
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    // Lageplan mit A1 (10 m).
    let (lid, l_a1) = (Uuid::new_v4(), Uuid::new_v4());
    let mut lageplan = aufmass(
        "Lageplan",
        vec![linie(l_a1, "A1", [100., 100.], [200., 100.], None)],
        vec![],
    );
    let fl = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, lid, 0, save(&lageplan), &user)
        .await
        .unwrap();
    // Ansichten: A1 als Wandbezug (10 m), A3 nur in der Ansicht (8 m).
    let (vid, nord, v_a1, v_a3) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let mut ansichten = aufmass(
        "Ansichten",
        vec![
            linie(v_a1, "A1", [600., 500.], [500., 500.], Some(nord)),
            linie(v_a3, "A3", [0., 500.], [80., 500.], Some(nord)),
        ],
        vec![Planausschnitt {
            id: nord,
            bezeichnung: "Ansicht Nord".into(),
            art: Ausschnittart::Ansicht,
            von: [0., 0.],
            bis: [1000., 1000.],
            ebene_id: None,
            kalibrierung: None,
            bezugslinie: None,
        }],
    );
    let fv = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            vid,
            0,
            save(&ansichten),
            &user,
        )
        .await
        .unwrap();
    let mut plan = core_lib::abstandsflaechen::testdaten::plan();
    plan.waende.clear();
    plan.grundstuecke.clear();
    let stand = |aufmass_id, revision| Aufmassstand {
        aufmass_id,
        revision,
    };
    plan = core
        .abstandslinien_uebernehmen(p.id, plan, stand(lid, fl.revision))
        .await
        .unwrap()
        .plan;
    let u = core
        .abstandslinien_uebernehmen(p.id, plan, stand(vid, fv.revision))
        .await
        .unwrap();
    // Echte Übernahme für den Oberflächentest (apps/desktop/tests/abstandsflaechen.py).
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(
            out.join("ansichten.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "aufmass": {"objekt": vid, "revision": fv.revision, "daten": ansichten},
                "uebernahme": u,
            }))
            .unwrap(),
        )
        .unwrap();
    }
    let mut plan = u.plan;
    assert_eq!(
        plan.waende
            .iter()
            .map(|w| w.bezeichnung.as_str())
            .collect::<Vec<_>>(),
        ["A1", "A3"]
    );
    assert!(
        u.hinweise
            .iter()
            .any(|h| h.starts_with("A3: Breite aus „Ansicht Nord“")),
        "{:?}",
        u.hinweise
    );
    let (a1, a3) = (&plan.waende[0], &plan.waende[1]);
    assert!(!a1.breite_aus_ansicht && (a1.ansicht_laenge_m.unwrap() - 10.).abs() < 1e-9);
    assert!(a1.hoehenquelle.contains("Ansicht: Aufmaß „Ansichten“"));
    assert!(a3.breite_aus_ansicht);
    assert_eq!((a3.bis.x, a3.bis.y), (8., 0.));
    assert!(
        plan.planbindungen
            .iter()
            .any(|b| b.ziel == format!("waende.{}.ansicht", a1.id))
    );
    // Die Lage von A1 bleibt aus dem Lageplan; die Ansicht ändert sie nicht.
    assert_eq!((a1.von.x, a1.bis.x), (10., 20.));
    core.abstands_planquellen_pruefen(p.id, &plan)
        .await
        .unwrap();
    let ziele = abstandsziele(&plan);
    assert!(
        ziele
            .iter()
            .any(|z| z.ziel == format!("waende.{}.ansicht", a3.id))
    );
    // Eine Linie gehört genau einer Wand.
    let mut doppelt = plan.clone();
    let a1_ansicht = doppelt
        .planbindungen
        .iter()
        .find(|b| b.markierung_id == v_a1)
        .unwrap()
        .clone();
    doppelt.planbindungen.retain(|b| b.markierung_id != v_a3);
    doppelt.planbindungen.push(Planbindung {
        ziel: format!("waende.{}.ansicht", a3.id),
        ..a1_ansicht
    });
    assert!(core.planaufmass_abstand(p.id, doppelt).await.is_err());

    // A3 ohne Lage: Fläche = Breite × Tiefe, Prüfung über den Grenzabstand.
    let a3 = &mut plan.waende[1];
    a3.gelaende = vec![];
    (a3.gelaende_a_m, a3.gelaende_b_m) = (Some(100.), Some(100.));
    (a3.abschluss_a_m, a3.abschluss_b_m) = (Some(106.), Some(106.));
    a3.dachart = core_lib::abstandsflaechen::Dachart::Flach;
    let r = berechnen(&plan).unwrap();
    let e = r.waende.iter().find(|w| w.bezeichnung == "A3").unwrap();
    assert_eq!(e.status, AbstandsStatus::Offen, "{e:?}");
    assert!(e.offen[0].starts_with("Abstand der Wand zur Grundstücksgrenze angeben"));
    assert!(e.polygon.is_empty());
    assert!((e.flaeche_m2.unwrap() - 24.).abs() < 1e-9, "{e:?}");
    plan.waende[1].grenzabstand_m = Some(4.);
    let e = berechnen(&plan).unwrap().waende.remove(1);
    assert_eq!(e.status, AbstandsStatus::Eingehalten);
    assert!(
        e.herleitung
            .iter()
            .any(|h| h.contains("Angabe des Planers"))
    );
    plan.waende[1].grenzabstand_m = Some(2.5);
    assert_eq!(
        berechnen(&plan).unwrap().waende[1].status,
        AbstandsStatus::Konflikt
    );

    // Breite in der Ansicht weicht vom Lageplan ab: Hinweis im Prüfbericht.
    ansichten.markierungen[0].geometrie = Plangeometrie::Strecke {
        von: [605., 500.],
        bis: [500., 500.],
    };
    let fv2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            vid,
            fv.revision,
            save(&ansichten),
            &user,
        )
        .await
        .unwrap();
    let plan = core
        .abstandslinien_uebernehmen(p.id, plan, stand(vid, fv2.revision))
        .await
        .unwrap()
        .plan;
    assert!((plan.waende[0].ansicht_laenge_m.unwrap() - 10.5).abs() < 1e-9);
    let r = core.abstandsplan_pruefen(p.id, &plan).await.unwrap();
    assert!(
        r.offen
            .iter()
            .any(|o| o.starts_with("A1: Breite in der Ansicht 10,5 m, im Lageplan 10 m")),
        "{:?}",
        r.offen
    );

    // A3 wird nachträglich im Lageplan gezogen: dieselbe Wand erhält Lage und
    // Breite aus dem Lageplan, die Ansicht bleibt Wandbezug.
    let a3_id = plan.waende[1].id;
    lageplan.markierungen.push(linie(
        Uuid::new_v4(),
        "A3",
        [200., 100.],
        [200., 180.],
        None,
    ));
    let fl2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            lid,
            fl.revision,
            save(&lageplan),
            &user,
        )
        .await
        .unwrap();
    let plan = core
        .abstandslinien_uebernehmen(p.id, plan, stand(lid, fl2.revision))
        .await
        .unwrap()
        .plan;
    assert_eq!(plan.waende.len(), 2);
    let a3 = &plan.waende[1];
    assert_eq!(a3.id, a3_id);
    assert!(!a3.breite_aus_ansicht && a3.grenzabstand_m.is_none());
    assert_eq!(
        (a3.von.x, a3.von.y, a3.bis.x, a3.bis.y),
        (20., -10., 20., -18.)
    );
    assert!(
        plan.planbindungen
            .iter()
            .any(|b| b.ziel == format!("waende.{a3_id}.ansicht"))
    );
    assert!(
        plan.planbindungen
            .iter()
            .any(|b| b.ziel == format!("waende.{a3_id}.lage"))
    );
}

#[tokio::test]
async fn abstandsflaechen_hoehen_aus_der_ansicht() {
    use core_lib::abstandsflaechen::berechnen;
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
                name: "Höhen".into(),
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
            "Plaene.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &user,
        )
        .await
        .unwrap();
    let mut plan = core_lib::abstandsflaechen::testdaten::plan();
    plan.waende.clear();
    plan.grundstuecke.clear();
    let system = plan.hoehenbezug.clone();
    let ebene = Uuid::new_v4();
    let m = |name: &str, bedeutung, geometrie, ausschnitt| Planmarkierung {
        id: Uuid::new_v4(),
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung,
        geometrie,
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: ausschnitt,
        hoehenzone_von: None,
        flurstueck_id: None,
    };
    let strecke = |von: [f64; 2], bis: [f64; 2]| Plangeometrie::Strecke { von, bis };
    let punkt = |p: [f64; 2]| Plangeometrie::Hoehenpunkt { punkt: p };
    let aufmass = |name: &str, markierungen, ausschnitte| Planaufmass {
        bezeichnung: name.into(),
        dokument: Some(Dokumentbezug {
            dokument_id: doc.document_id,
            version: 1,
        }),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        // 100 pt = 10 m.
        kalibrierung: Some(Plankalibrierung::Strecke {
            von: [0., 0.],
            bis: [100., 0.],
            meter: 10.,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "Abstandsflächen".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen,
        ausschnitte,
    };
    let save = |a: &Planaufmass| serde_json::to_value(a).unwrap();
    let stand = |aufmass_id, revision| Aufmassstand {
        aufmass_id,
        revision,
    };
    // Lageplan: A1 (10 m) und A2.
    let lid = Uuid::new_v4();
    let lageplan = aufmass(
        "Lageplan",
        vec![
            m(
                "A1",
                Planbedeutung::Abstandsflaeche,
                strecke([100., 100.], [200., 100.]),
                None,
            ),
            m(
                "A2",
                Planbedeutung::Abstandsflaeche,
                strecke([200., 100.], [200., 170.]),
                None,
            ),
        ],
        vec![],
    );
    let fl = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, lid, 0, save(&lageplan), &user)
        .await
        .unwrap();
    plan = core
        .abstandslinien_uebernehmen(p.id, plan, stand(lid, fl.revision))
        .await
        .unwrap()
        .plan;
    // Ansicht Süd: ±0,00-Linie bei y = 600 auf 100 m, 10 pt = 1 m. A1 von
    // x = 500 bis 600; Höhe = 100 + (600 − y) / 10.
    let (vid, sued) = (Uuid::new_v4(), Uuid::new_v4());
    let s = Some(sued);
    use Planbedeutung::*;
    let mut ansicht = aufmass(
        "Ansichten",
        vec![
            m(
                "A1",
                Abstandsflaeche,
                strecke([500., 600.], [600., 600.]),
                s,
            ),
            m("G1", Gelaende, strecke([500., 600.], [550., 595.]), s),
            m("G2", Gelaende, strecke([550., 595.], [600., 600.]), s),
            m("G3", Abgrabung, punkt([520., 610.]), s),
            // Außerhalb der Wand: gehört nicht zu A1.
            m("G4", Gelaende, punkt([700., 600.]), s),
            // Traufe mit Dachüberstand, daneben höhere Attika: Höhensprung bei 6 m.
            m("T1", Traufe, strecke([490., 530.], [560., 530.]), s),
            m("AT1", Attika, strecke([560., 520.], [600., 520.]), s),
            m("W1", Wandteilgrenze, strecke([530., 520.], [530., 600.]), s),
            m("F1", First, punkt([530., 480.]), s),
            m("D1", Dach, strecke([500., 530.], [530., 500.]), s),
            m("GA1", Gaube, strecke([510., 505.], [525., 505.]), s),
        ],
        vec![Planausschnitt {
            id: sued,
            bezeichnung: "Ansicht Süd".into(),
            art: Ausschnittart::Ansicht,
            von: [0., 0.],
            bis: [1000., 1000.],
            ebene_id: None,
            kalibrierung: None,
            bezugslinie: Some(Planbezugslinie {
                von: [0., 600.],
                bis: [1000., 600.],
                hoehe_m: 100.,
                system: system.clone(),
            }),
        }],
    );
    let fv = core
        .fachobjekt_speichern(p.id, ObjektArt::Planaufmass, vid, 0, save(&ansicht), &user)
        .await
        .unwrap();
    let u = core
        .abstandslinien_uebernehmen(p.id, plan, stand(vid, fv.revision))
        .await
        .unwrap();
    // Echte Übernahme für den Oberflächentest (apps/desktop/tests/abstandsflaechen.py).
    if let Ok(out) = std::env::var("BAUPROJEKT_ABSTAND_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(
            out.join("hoehen.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "aufmass": {"objekt": vid, "revision": fv.revision, "daten": ansicht},
                "uebernahme": u,
            }))
            .unwrap(),
        )
        .unwrap();
    }
    let mut plan = u.plan;
    let a1 = plan.waende.iter().find(|w| w.bezeichnung == "A1").unwrap();
    let paare = |v: &[core_lib::abstandsflaechen::Hoehenpunkt]| {
        v.iter()
            .map(|p| {
                (
                    (p.station_m * 1000.).round() / 1000.,
                    (p.hoehe_m * 1000.).round() / 1000.,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(paare(&a1.gelaende), [(0., 100.), (5., 100.5), (10., 100.)]);
    assert_eq!(
        paare(&a1.oberkante),
        [(0., 107.), (6., 107.), (6., 108.), (10., 108.)]
    );
    assert_eq!(a1.wandteilgrenzen, [3.]);
    assert!((a1.dachhoehe_m.unwrap() - 4.).abs() < 1e-9);
    assert!((a1.dachneigung_grad.unwrap() - 45.).abs() < 1e-9);
    assert!((a1.gauben_laenge_m.unwrap() - 1.5).abs() < 1e-9);
    assert!(a1.ansicht_hinweise.is_empty(), "{:?}", a1.ansicht_hinweise);
    for f in [
        "gelaende",
        "oberkante",
        "wandteilgrenzen",
        "dachhoehe_m",
        "dachneigung_grad",
        "gauben_laenge_m",
    ] {
        assert!(a1.ansicht_felder.iter().any(|x| x == f), "{f}");
    }
    assert!(a1.gelaende[1].quelle.contains("Ansicht „Ansicht Süd“ · G1"));
    assert!(
        a1.hoehenquelle
            .contains("Abgrabung „G3“ bei 2 m (99 m) nicht maßgebend"),
        "{}",
        a1.hoehenquelle
    );
    core.abstands_planquellen_pruefen(p.id, &plan)
        .await
        .unwrap();
    // Wandteile aus Grenze bei 3 m und Höhensprung bei 6 m.
    let e = berechnen(&plan)
        .unwrap()
        .waende
        .into_iter()
        .find(|e| e.bezeichnung == "A1")
        .unwrap();
    assert_eq!(e.wandteile.len(), 3, "{e:?}");
    assert!(
        !e.offen.iter().any(|o| o.contains("Ansicht")),
        "{:?}",
        e.offen
    );
    // Von Hand geänderte Höhe: Abweichung vom Aufmaß.
    let mut verstellt = plan.clone();
    verstellt.waende[0].gelaende[1].hoehe_m = 101.;
    assert!(
        core.abstands_planquellen_pruefen(p.id, &verstellt)
            .await
            .is_err()
    );

    // Anderes Höhensystem im Plan: Höhen werden nicht übernommen und der
    // alte Abgriff geleert; Neigung und Gauben bleiben.
    plan.hoehenbezug = "DHHN2016".into();
    let plan2 = core.planaufmass_abstand(p.id, plan.clone()).await.unwrap();
    let a1 = &plan2.waende[0];
    assert!(a1.gelaende.is_empty() && a1.oberkante.is_empty() && a1.dachhoehe_m.is_none());
    assert!(a1.dachneigung_grad.is_some());
    assert!(
        a1.ansicht_hinweise[0].contains("misst Höhen im System"),
        "{:?}",
        a1.ansicht_hinweise
    );
    let e = berechnen(&plan2).unwrap().waende.remove(0);
    assert!(e.offen.iter().any(|o| o.contains("misst Höhen im System")));
    plan.hoehenbezug = system;

    // A2 in derselben Ansicht überlappt A1: abgewiesen.
    ansicht.markierungen.push(m(
        "A2",
        Abstandsflaeche,
        strecke([580., 600.], [650., 600.]),
        s,
    ));
    let fv2 = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            vid,
            fv.revision,
            save(&ansicht),
            &user,
        )
        .await
        .unwrap();
    let e = core
        .abstandslinien_uebernehmen(p.id, plan, stand(vid, fv2.revision))
        .await
        .unwrap_err();
    assert!(e.to_string().contains("überlappen"), "{e}");
}
