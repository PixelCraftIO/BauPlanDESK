//! Öffentliche Fachverträge: Grundstück → Fachmodule → Nachweis → Verbraucher → Druck.
use core_lib::{
    CoreServices,
    berechnungen::*,
    domain::{Bundesland, NewProject, Planungsrecht},
    druckausgabe::{Druckauftrag, Druckquelle},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::nachweise::NachweisEingabe,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
fn quelle(p: &mut FachPlan) {
    p.regelkontext.quelle = "Fiktiv überprüfter Kontrollansatz – keine reale Satzung".into();
    p.regelkontext.pruefvermerk = "Unabhängiger arithmetischer Kontrollfall".into();
    p.regelkontext.anwendbarkeit_bestaetigt = true;
}
async fn sichern(
    c: &CoreServices,
    p: core_lib::domain::ProjectId,
    plan: FachPlan,
    u: &core_lib::benutzer::Benutzer,
) -> (
    core_lib::commit::Fachstand,
    FachPlan,
    FachErgebnis,
    core_lib::plattform::nachweise::Nachweisstand,
) {
    let id = Uuid::new_v4();
    let f = c
        .fachobjekt_speichern(
            p,
            plan.modul.objekt(),
            id,
            0,
            serde_json::to_value(&plan).unwrap(),
            u,
        )
        .await
        .unwrap();
    let plan: FachPlan = serde_json::from_value(f.daten.clone()).unwrap();
    let (_, r) = c.berechnung_pruefen(p, plan.clone()).await.unwrap();
    let n = c
        .nachweis_erstellen(
            p,
            Uuid::new_v4(),
            plan.bezeichnung.clone(),
            NachweisEingabe::Fach {
                objekt: id,
                plan: Box::new(plan.clone()),
            },
            u,
        )
        .await
        .unwrap();
    (f, plan, r, n)
}
#[tokio::test]
async fn module_quellen_freigaben_archiv_und_druck() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let lbo = Arc::new(LboEngine::new());
    lbo.load_directory(std::path::Path::new("../../lbo-rules"))
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        lbo,
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("fach", "Prüfbüro Muster").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Neubau – modulare Nachweise".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    rechtsgrundlage: Some(core_lib::domain::Rechtsgrundlage::Par30),
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
    assert_eq!(grund.len(), 2);
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Aufmaß und Satzungsprüfung.txt",
            bytes::Bytes::from_static(b"Fiktiver Kontrollbeleg"),
            &user,
        )
        .await
        .unwrap();
    let vorlagen = vorlagen().unwrap();
    let mut staende = serde_json::Map::new();
    let mut quellen = vec![];
    let mut wp = vorlagen
        .iter()
        .find(|v| v.plan.modul == BerechnungsArt::Wohnflaechen)
        .unwrap()
        .plan
        .clone();
    wp.grundstueck_id = Some(grund[0].id);
    wp.quellen
        .push(core_lib::plattform::objekte::Dokumentbezug {
            dokument_id: d.document_id,
            version: d.version,
        });
    quelle(&mut wp);
    wp.eingaben["positionen"][0]["aufmass_m2"] = json!(100);
    wp.eingaben["positionen"][0]["lichte_hoehe_m"] = json!(2.5);
    wp.eingaben["positionen"][0]["quelle"] = json!("Kontrollaufmaß");
    wp.eingaben["aufmass_vollstaendig"] = json!(true);
    wp.eingaben["wohneinheiten_bestaetigt"] = json!(true);
    let (wf, wp, wr, wn) = sichern(&core, p.id, wp, &user).await;
    assert!(wn.vollstaendig);
    assert!(
        core.nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|n| n.stand.id == wn.id)
            .unwrap()
            .aktuell
    );
    assert_eq!(
        core.nachweis_wiederholen(p.id, wn.id).await.unwrap(),
        serde_json::to_value(&wr).unwrap()
    );
    core.nachweis_freigeben(p.id, wn.id, "Kontrollfall geprüft".into(), &user)
        .await
        .unwrap();
    staende.insert(
        "wohnflaechen".into(),
        json!({"fachstand":wf,"plan":wp,"ergebnis":wr,"nachweis":wn}),
    );
    quellen.push(Druckquelle::Nachweis { id: wn.id });
    let mut sp = vorlagen
        .iter()
        .find(|v| v.plan.modul == BerechnungsArt::Stellplaetze)
        .unwrap()
        .plan
        .clone();
    quelle(&mut sp);
    sp.grundstueck_id = Some(grund[0].id);
    sp.eingaben["regeln"]["richtzahlen"][0]["bezugsart"] = json!("wohnflaeche");
    sp.eingaben["regeln"]["richtzahlen"][0]["faktor"] = json!(1);
    sp.eingaben["regeln"]["richtzahlen"][0]["bezugsmenge"] = json!(50);
    sp.eingaben["positionen"][0]["wohnflaeche_m2"] = json!(9999);
    sp.uebernahmen = vec![
        Uebernahme {
            nachweis_id: wn.id,
            kennzahl: "wohnflaeche".into(),
            ziel: "positionen.nutzung1.wohnflaeche_m2".into(),
        },
        Uebernahme {
            nachweis_id: wn.id,
            kennzahl: "wohneinheiten".into(),
            ziel: "positionen.nutzung1.wohneinheiten".into(),
        },
    ];
    sp.eingaben["angebot"] = json!([{"id":"kfz","bezeichnung":"Kfz laut Lageplan","art":"kfz","anzahl":2,"besucher":0,"barrierefrei":0,"ausserhalb":false,"anerkannt":true,"lage_und_sicherung":"Lageplan"},{"id":"rad","bezeichnung":"Fahrräder","art":"fahrrad","anzahl":2,"besucher":0,"barrierefrei":0,"ausserhalb":false,"anerkannt":true,"lage_und_sicherung":"Lageplan"}]);
    let (sp, _) = core.berechnung_pruefen(p.id, sp).await.unwrap();
    assert_eq!(sp.eingaben["positionen"][0]["wohnflaeche_m2"], json!(100.0));
    assert_eq!(sp.eingaben["positionen"][0]["wohneinheiten"], json!(1.0));
    let mut falsch = sp.clone();
    falsch.uebernahmen[0].ziel = "positionen.nutzung1.nutzungsflaeche_m2".into();
    assert!(core.berechnung_pruefen(p.id, falsch).await.is_err());
    let mut falsch = sp.clone();
    falsch.grundstueck_id = Some(grund[1].id);
    assert!(core.berechnung_pruefen(p.id, falsch).await.is_err());
    let (sf, sp, sr, sn) = sichern(&core, p.id, sp, &user).await;
    assert!(sn.vollstaendig);
    assert_eq!(
        sr.kennzahlen
            .iter()
            .find(|k| k.id == "kfz_bedarf")
            .unwrap()
            .wert,
        Some(2.)
    );
    staende.insert(
        "stellplaetze".into(),
        json!({"fachstand":sf,"plan":sp,"ergebnis":sr,"nachweis":sn}),
    );
    quellen.push(Druckquelle::Nachweis { id: sn.id });
    for art in BerechnungsArt::ALLE {
        if matches!(
            art,
            BerechnungsArt::Wohnflaechen | BerechnungsArt::Stellplaetze
        ) {
            continue;
        }
        let mut plan = vorlagen
            .iter()
            .find(|v| v.plan.modul == art)
            .unwrap()
            .plan
            .clone();
        plan.grundstueck_id = Some(grund[0].id);
        quelle(&mut plan);
        match art {
            BerechnungsArt::Ueberbaubarkeit => {
                plan.eingaben["positionen"][0]["abstand_min_m"] = json!(-0.5);
                plan.eingaben["positionen"][0]["abstand_max_m"] = json!(0);
                plan.eingaben["positionen"][0]["ausserhalb_m2"] = json!(0);
                plan.eingaben["positionen"][0]["quelle"] =
                    json!("Lageplan / vermessener Fassadenabschnitt A");
                plan.eingaben["aufmass_geprueft"] = json!(true);
                plan.eingaben["flaechen_abgegrenzt"] = json!(true);
            }
            BerechnungsArt::Freiflaechen => {
                plan.eingaben["grundstuecksflaeche_m2"] = json!(9999);
                plan.eingaben["positionen"][0]["flaeche_m2"] = json!(grund[0].eigentumsflaeche_m2);
                plan.eingaben["positionen"][0]["quelle"] = json!("Lageplan");
                plan.eingaben["bilanz_geprueft"] = json!(true);
            }
            BerechnungsArt::Din277 => {
                plan.eingaben["positionen"][0]["aufmass_m2"] = json!(100);
                plan.eingaben["positionen"][0]["quelle"] = json!("Aufmaß");
                plan.eingaben["klassifikation_geprueft"] = json!(true);
                plan.eingaben["kgf_vollstaendig"] = json!(true);
            }
            BerechnungsArt::Hoehen => {
                plan.eingaben["punkte"][0]["absolut_m"] = json!(108);
                plan.eingaben["punkte"][0]["bezug_m"] = json!(100);
                plan.eingaben["punkte"][0]["quelle"] = json!("Schnitt");
                plan.eingaben["hoehenbezug"] = json!("Lokales Kontrollsystem in Meter");
                plan.eingaben["bezug_geprueft"] = json!(true);
            }
            BerechnungsArt::Vollgeschosse => {
                plan.eingaben["geschosse"][0]["grundflaeche_m2"] = json!(100);
                plan.eingaben["geschosse"][0]["unterliegend_m2"] = json!(100);
                plan.eingaben["geschosse"][0]["hoehenquelle"] = json!("Schnitt");
                plan.eingaben["teilflaechen"] = json!([{"id":"zone","geschoss_id":"eg","bezeichnung":"Gesamtes EG","flaeche_m2":100,"lichte_hoehe_m":2.5}]);
                plan.eingaben["aufmass_geprueft"] = json!(true);
                plan.eingaben["sonderfaelle_geprueft"] = json!(true);
            }
            BerechnungsArt::Entwaesserung => {
                plan.eingaben["positionen"][0]["flaeche_m2"] = json!(100);
                plan.eingaben["positionen"][0]["quelle"] = json!("Dachplan");
                plan.eingaben["regeln"]["regen"] = json!([{"id":"d10","dauer_min":10,"intensitaet_l_s_ha":100,"quelle":"Fiktiver Kontrollregen"}]);
                plan.eingaben["flaechen_geprueft"] = json!(true);
                plan.eingaben["regendaten_geprueft"] = json!(true);
            }
            _ => unreachable!(),
        }
        let (f, plan, r, n) = sichern(&core, p.id, plan, &user).await;
        assert!(n.vollstaendig, "{:?}: {:?}", art, r.offen);
        if art == BerechnungsArt::Freiflaechen {
            assert_eq!(
                plan.eingaben["grundstuecksflaeche_m2"],
                json!(grund[0].eigentumsflaeche_m2)
            );
        }
        assert_eq!(
            core.nachweis_wiederholen(p.id, n.id).await.unwrap(),
            n.ergebnis
        );
        quellen.push(Druckquelle::Nachweis { id: n.id });
        staende.insert(
            art.objekt().modul().into(),
            json!({"fachstand":f,"plan":plan,"ergebnis":r,"nachweis":n}),
        );
    }
    let katalog = core.druckkatalog(p.id).await.unwrap();
    let auftrag = Druckauftrag {
        gestaltung: None,
        aktenzeichen: "Kontrollfall neue Fachmodule".into(),
        datum: None,
        titel: "Fachnachweise".into(),
        absender: "Prüfbüro Muster".into(),
        empfaenger: "Bauaufsicht Musterstadt".into(),
        anschreiben: "Fiktive Kontrollnachweise zur Prüfung der Anwendung.".into(),
        mit_anschreiben: true,
        inhaltsverzeichnis: true,
        projekt_version: p.version,
        quellen,
    };
    let druck = core.druckausgabe(p.id, auftrag.clone()).await.unwrap();
    let mut einzel = auftrag;
    einzel.quellen = vec![Druckquelle::Nachweis { id: sn.id }];
    einzel.mit_anschreiben = false;
    einzel.inhaltsverzeichnis = false;
    let druck_einzel = core.druckausgabe(p.id, einzel).await.unwrap();
    let mut wohn120 = wp.clone();
    wohn120.eingaben["positionen"][0]["aufmass_m2"] = json!(120);
    wohn120.regelkontext.anwendbarkeit_bestaetigt = false;
    let ergebnis120 = core_lib::berechnungen::berechnen(&wohn120).unwrap();
    assert_eq!(druck.positionen.len(), 8);
    assert!(druck.pdf.starts_with(b"%PDF-"));
    if let Ok(art) = std::env::var("BAUPROJEKT_FACH_ARTEFAKTE") {
        let art = std::path::Path::new(&art);
        std::fs::create_dir_all(art).unwrap();
        std::fs::write(art.join("nachweise.pdf"), &druck.pdf).unwrap();
        std::fs::write(art.join("einzel.pdf"), &druck_einzel.pdf).unwrap();
        std::fs::write(art.join("workflow.json"),serde_json::to_vec_pretty(&json!({"projekt":p,"benutzer":user,"grundstuecke":grund,"dateien":[d],"vorlagen":vorlagen,"staende":staende,"katalog":katalog,"seiten":druck.seiten,"einzel_seiten":druck_einzel.seiten,"ergebnis120":ergebnis120})).unwrap()).unwrap();
    }
    // Vollgeschossprüfung -> Gebäudegeschoss -> bestehende GFZ, ohne Rückkopplung.
    let gid = Uuid::new_v4();
    let sid = Uuid::new_v4();
    let gb=core.fachobjekt_speichern(p.id,core_lib::plattform::ObjektArt::Gebaeude,gid,0,json!({"bezeichnung":"Haus A","grundstueck_ids":[grund[0].id],"geschosse":[{"id":sid,"bezeichnung":"EG","vollgeschoss":null,"grundlage":""}],"quelle":null}),&user).await.unwrap();
    let mut vp = vorlagen
        .iter()
        .find(|v| v.plan.modul == BerechnungsArt::Vollgeschosse)
        .unwrap()
        .plan
        .clone();
    quelle(&mut vp);
    vp.grundstueck_id = Some(grund[0].id);
    vp.gebaeude_id = Some(gid);
    vp.eingaben["geschosse"][0]["gebaeude_geschoss_id"] = json!(sid);
    vp.eingaben["geschosse"][0]["grundflaeche_m2"] = json!(100);
    vp.eingaben["geschosse"][0]["unterliegend_m2"] = json!(100);
    vp.eingaben["geschosse"][0]["hoehenquelle"] = json!("Kontrollschnitt");
    vp.eingaben["teilflaechen"] = json!([{"id":"zone","geschoss_id":"eg","bezeichnung":"EG","flaeche_m2":100,"lichte_hoehe_m":2.5}]);
    vp.eingaben["aufmass_geprueft"] = json!(true);
    vp.eingaben["sonderfaelle_geprueft"] = json!(true);
    let (vf, vp, _, vn) = sichern(&core, p.id, vp, &user).await;
    assert!(vn.vollstaendig);
    let mut daten = gb.daten;
    daten["geschosse"][0]["einstufungsquelle"] =
        json!({"nachweis_id":vn.id,"kennzahl":"geschoss:eg"});
    let geb = core
        .fachobjekt_speichern(
            p.id,
            core_lib::plattform::ObjektArt::Gebaeude,
            gid,
            gb.revision,
            daten,
            &user,
        )
        .await
        .unwrap();
    assert_eq!(geb.daten["geschosse"][0]["vollgeschoss"], json!(true));
    assert!(
        core.nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|n| n.stand.id == vn.id)
            .unwrap()
            .aktuell,
        "Übernahme darf die Quelle nicht selbst veralten lassen"
    );
    let mut nutz = core_lib::baugrundstuecke::musterplan();
    for b in &mut nutz.berechnungen {
        if b.schluessel == grund[0].schluessel {
            for g in &mut b.eingabe.aufstellung.as_mut().unwrap().geschosse {
                g.gebaeude_id = Some(gid.to_string());
                g.geschoss_id = Some(sid.to_string());
            }
        }
    }
    let gn = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            "GFZ mit Vollgeschossquelle".into(),
            NachweisEingabe::Nutzung {
                fach: core_lib::plattform::nachweise::NutzungsArt::Gfz,
                plan: nutz.clone(),
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
            .find(|n| n.stand.id == gn.id)
            .unwrap()
            .aktuell
    );
    let mut neu_v = vp;
    neu_v.eingaben["teilflaechen"][0]["lichte_hoehe_m"] = json!(1.5);
    core.fachobjekt_speichern(
        p.id,
        BerechnungsArt::Vollgeschosse.objekt(),
        Uuid::parse_str(&vf.objekt).unwrap(),
        vf.revision,
        serde_json::to_value(neu_v).unwrap(),
        &user,
    )
    .await
    .unwrap();
    assert!(
        !core
            .nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|n| n.stand.id == gn.id)
            .unwrap()
            .aktuell
    );
    assert!(
        core.nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            "GFZ mit veralteter Quelle".into(),
            NachweisEingabe::Nutzung {
                fach: core_lib::plattform::nachweise::NutzungsArt::Gfz,
                plan: nutz
            },
            &user
        )
        .await
        .is_err()
    );
    assert_eq!(
        core.nachweis_wiederholen(p.id, gn.id).await.unwrap(),
        gn.ergebnis
    );
    // Raumbuch-Fakten bleiben unabhängig von der Geschosseinstufung nutzbar.
    let rid = Uuid::new_v4();
    let raum = Uuid::new_v4();
    let rb=core.fachobjekt_speichern(p.id,core_lib::plattform::ObjektArt::Raumbuch,rid,0,json!({"bezeichnung":"Aufmaß Haus A","gebaeude_id":gid,"geschoss_id":sid,"raeume":[{"id":raum,"nummer":"1","bezeichnung":"Zimmer","nutzung":"Wohnen","flaeche_m2":20,"hoehe_m":2.4}]}),&user).await.unwrap();
    let mut rp = core
        .berechnung_aus_raumbuch(p.id, rid, BerechnungsArt::Wohnflaechen)
        .await
        .unwrap();
    assert_eq!(rp.eingaben["positionen"][0]["typ"], json!(""));
    assert_eq!(rp.eingaben["positionen"][0]["aufmass_m2"], json!(20.0));
    rp.grundstueck_id = Some(grund[0].id);
    rp.eingaben["positionen"][0]["typ"] = json!("wohnraum");
    rp.eingaben["positionen"][0]["wohnung"] = json!("Wohnung B");
    rp.eingaben["aufmass_vollstaendig"] = json!(true);
    quelle(&mut rp);
    let (_, rp, _, rn) = sichern(&core, p.id, rp, &user).await;
    assert!(
        core.nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|n| n.stand.id == rn.id)
            .unwrap()
            .aktuell
    );
    let mut rbd = rb.daten;
    rbd["raeume"][0]["flaeche_m2"] = json!(25);
    core.fachobjekt_speichern(
        p.id,
        core_lib::plattform::ObjektArt::Raumbuch,
        rid,
        rb.revision,
        rbd,
        &user,
    )
    .await
    .unwrap();
    assert!(
        !core
            .nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|n| n.stand.id == rn.id)
            .unwrap()
            .aktuell
    );
    assert!(core.berechnung_pruefen(p.id, rp).await.is_err());
    // Neue Fakten setzen die Bestätigung zurück und machen Verbraucher transitiv veraltet.
    let mut neu = wp.clone();
    neu.eingaben["positionen"][0]["aufmass_m2"] = json!(120);
    let f = core
        .fachobjekt_speichern(
            p.id,
            neu.modul.objekt(),
            Uuid::parse_str(&wf.objekt).unwrap(),
            wf.revision,
            serde_json::to_value(&neu).unwrap(),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(
        f.daten["regelkontext"]["anwendbarkeit_bestaetigt"],
        json!(false)
    );
    let n = core.nachweise(p.id).await.unwrap();
    assert!(!n.iter().find(|n| n.stand.id == wn.id).unwrap().aktuell);
    assert!(!n.iter().find(|n| n.stand.id == sn.id).unwrap().aktuell);
    assert!(core.berechnung_pruefen(p.id, sp.clone()).await.is_err());
    assert_eq!(
        core.nachweis_wiederholen(p.id, wn.id).await.unwrap(),
        wn.ergebnis
    );
    // Regeländerung mit gleicher Version und konkurrierender Revision wird abgelehnt.
    let mut geändert = sp.clone();
    geändert.eingaben["regeln"]["richtzahlen"][0]["faktor"] = json!(2);
    geändert.uebernahmen.clear();
    assert!(
        core.fachobjekt_speichern(
            p.id,
            geändert.modul.objekt(),
            Uuid::parse_str(&sf.objekt).unwrap(),
            sf.revision,
            serde_json::to_value(&geändert).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    let mut leser = user.clone();
    leser.rolle = core_lib::benutzer::Rolle::Lesen;
    assert!(
        core.fachobjekt_speichern(
            p.id,
            wp.modul.objekt(),
            Uuid::new_v4(),
            0,
            serde_json::to_value(wp).unwrap(),
            &leser
        )
        .await
        .is_err()
    );
    let mut q = neu;
    q.regelkontext.anwendbarkeit_bestaetigt = true;
    assert!(
        core.fachobjekt_speichern(
            p.id,
            q.modul.objekt(),
            Uuid::parse_str(&wf.objekt).unwrap(),
            wf.revision,
            serde_json::to_value(q).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    // Aktenexport enthält alle eigenständigen Module.
    let akte = core.verfahren_aktenexport(p.id).await.unwrap();
    assert!(!akte.is_empty());
}
