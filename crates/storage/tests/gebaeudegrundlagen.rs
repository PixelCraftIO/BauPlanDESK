//! Gebäudegrundlagen → GRZ/GFZ je Grundstück und parametrisierte Vollgeschossprüfung.
use core_lib::{
    CoreServices,
    berechnungen::{BerechnungsArt, FachPlan, vorlagen},
    domain::{Bundesland, NewProject, Planungsrecht},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{
        ObjektArt,
        nachweise::{NachweisEingabe, NutzungsArt},
    },
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
#[tokio::test]
async fn grundlagen_fliessen_getrennt_in_nachweise_und_veralten_bei_aenderung() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("grundlagen", "Prüfbüro").await.unwrap();
    let mut plan = core_lib::baugrundstuecke::musterplan();
    let p = core
        .create_project(
            NewProject {
                name: "Gebäudegrundlagen".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    bebauungsplan_art: core_lib::domain::BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(plan.clone()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    let haus = Uuid::new_v4();
    let garage = Uuid::new_v4();
    let sids = [
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    ];
    let mut daten = json!({"bezeichnung":"Wohnhaus01","grundstueck_ids":[],"flurstueck_ids":grund[0].flurstueck_ids,"quelle":null,"geometrie":{"grundflaeche_m2":100,"typ":"hauptanlage","gelaende_m":-0.8},"geschosse":(["KG","EG","OG","DG"].iter().enumerate().map(|(i,s)|json!({"id":sids[i],"bezeichnung":s,"vollgeschoss":i==1 || i==2,"grundlage":"Fiktiver Kontrollfall","geometrie":{"flaeche_m2":100,"fussboden_m":-2.5+3.*i as f64,"lichte_hoehe_m":2.4,"lichte_hoehe_einheitlich":true,"deckendicke_m":0.3}})).collect::<Vec<_>>())});
    let stand = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, 0, daten.clone(), &user)
        .await
        .unwrap();
    let garagendaten = json!({"bezeichnung":"Garage","grundstueck_ids":[],"flurstueck_ids":grund[0].flurstueck_ids,"quelle":null,"geschosse":[],"geometrie":{"grundflaeche_m2":20,"typ":"garage"}});
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        garage,
        0,
        garagendaten.clone(),
        &user,
    )
    .await
    .unwrap();
    for b in &mut plan.berechnungen {
        let a = b.eingabe.aufstellung.as_mut().unwrap();
        a.bauteile.clear();
        a.geschosse.clear();
    }
    let b = plan
        .berechnungen
        .iter_mut()
        .find(|b| b.schluessel == grund[0].schluessel)
        .unwrap();
    let a = b.eingabe.aufstellung.as_mut().unwrap();
    a.bauteile=serde_json::from_value(json!([{"gebaeude_id":haus,"bezeichnung":"alt","typ":"garage","flaeche_m2":999,"abweichende_anrechnung":null,"begruendung":""},{"gebaeude_id":garage,"bezeichnung":"alt","typ":"hauptanlage","flaeche_m2":999,"abweichende_anrechnung":null,"begruendung":""}])).unwrap();
    a.geschosse=serde_json::from_value(json!(sids.iter().map(|id|json!({"gebaeude_id":haus,"geschoss_id":id,"flaeche_aus_grundlagen":true,"bezeichnung":"alt","vollgeschoss":true,"aussenmass_m2":999,"aufenthalt_m2":0,"waende_m2":0,"erschliessung_m2":0})).chain(std::iter::once(json!({"gebaeude_id":garage,"geschoss_id":null,"flaeche_aus_grundlagen":true,"bezeichnung":"Garage EG","vollgeschoss":true,"aussenmass_m2":999,"aufenthalt_m2":0,"waende_m2":0,"erschliessung_m2":0}))).collect::<Vec<_>>())).unwrap();
    let r = core
        .projekt_baugrundstuecke_pruefen(p.id, &plan)
        .await
        .unwrap();
    let a = r
        .baugrundstuecke
        .iter()
        .find(|b| b.grundstueck.schluessel == grund[0].schluessel)
        .unwrap()
        .pruefung
        .aufstellung
        .as_ref()
        .unwrap();
    assert_eq!(a.mass.grundflaeche_m2, 100.);
    assert_eq!(a.mass.grundflaeche_nebenanlagen_m2, 20.);
    assert_eq!(a.mass.geschossflaeche_m2, 220.);
    assert_eq!(a.vollgeschosse, 2);
    let n = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            "Gebäudeflächen".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: plan.clone(),
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
            .find(|a| a.stand.id == n.id)
            .unwrap()
            .aktuell
    );
    let mut voll = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Vollgeschosse)
        .unwrap()
        .plan;
    voll.gebaeude_id = Some(haus);
    voll.grundstueck_id = Some(grund[0].id);
    voll.gebaeudedaten_uebernehmen = Some(true);
    voll.eingaben["regeln"]["vergleich_unterliegend"] = json!(false);
    voll.eingaben["regeln"]["oberirdisch_pruefen"] = json!(true);
    let (mut voll, r) = core.berechnung_pruefen(p.id, voll).await.unwrap();
    assert_eq!(voll.eingaben["geschosse"].as_array().unwrap().len(), 4);
    assert_eq!(voll.eingaben["teilflaechen"].as_array().unwrap().len(), 4);
    assert_eq!(
        r.kennzahlen
            .iter()
            .find(|k| k.id == format!("geschoss:{}", sids[0]))
            .unwrap()
            .wert,
        Some(0.)
    );
    assert!(!r.vollstaendig);
    assert!(r.offen.iter().any(|o| o.contains("Höhen-/Geländequelle")));
    voll.eingaben["aufmass_geprueft"] = json!(true);
    voll.eingaben["sonderfaelle_geprueft"] = json!(true);
    let (voll, _) = core.berechnung_pruefen(p.id, voll).await.unwrap();
    assert_eq!(voll.eingaben["aufmass_geprueft"], json!(true));
    daten["geometrie"]["grundflaeche_m2"] = json!(110);
    daten["geschosse"][0]["geometrie"]["fussboden_m"] = json!(-2.4);
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        haus,
        stand.revision,
        daten,
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
            .find(|a| a.stand.id == n.id)
            .unwrap()
            .aktuell
    );
    let (voll, _) = core.berechnung_pruefen(p.id, voll).await.unwrap();
    assert_eq!(voll.eingaben["aufmass_geprueft"], json!(false));
    let mut falsch = plan.clone();
    let b = falsch
        .berechnungen
        .iter_mut()
        .find(|b| b.schluessel == grund[0].schluessel)
        .unwrap();
    let a = b.eingabe.aufstellung.as_mut().unwrap();
    a.bauteile.push(a.bauteile[0].clone());
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &falsch)
            .await
            .is_err()
    );
    let mut falsch = plan.clone();
    falsch
        .berechnungen
        .iter_mut()
        .find(|b| b.schluessel == grund[0].schluessel)
        .unwrap()
        .schluessel = grund[1].schluessel.clone();
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &falsch)
            .await
            .is_err()
    );
    let mut mehrdeutig = garagendaten;
    mehrdeutig["flurstueck_ids"] =
        json!([&grund[0].flurstueck_ids[0], &grund[1].flurstueck_ids[0]]);
    core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, garage, 1, mehrdeutig, &user)
        .await
        .unwrap();
    // Garage auf Flurstücken zweier Baugrundstücke: verknüpfte Fläche entfällt
    // mit Hinweis, statt die Prüfung zu blockieren oder still zu verteilen.
    let r = core
        .projekt_baugrundstuecke_pruefen(p.id, &plan)
        .await
        .unwrap();
    for b in &r.baugrundstuecke {
        let a = b.pruefung.aufstellung.as_ref().unwrap();
        assert_eq!(a.mass.grundflaeche_nebenanlagen_m2, 0.);
    }
    let e = core
        .projekt_baugrundstuecke_gebaeude_einbinden(p.id, &plan)
        .await
        .unwrap();
    assert!(
        e.hinweise
            .iter()
            .any(|h| h.text.contains("mehrerer Baugrundstücke"))
    );
    assert!(e.hinweise.iter().any(|h| h.text.contains("entfernt")));
    let mut fremd: FachPlan = voll;
    fremd.gebaeude_id = Some(Uuid::new_v4());
    assert!(core.berechnung_pruefen(p.id, fremd).await.is_err());
}

#[tokio::test]
async fn zugeordnete_gebaeude_gehen_ohne_erneute_eingabe_in_grz_und_gfz_ein() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("einbindung", "Prüfbüro").await.unwrap();
    let mut plan = core_lib::baugrundstuecke::musterplan();
    for b in &mut plan.berechnungen {
        let a = b.eingabe.aufstellung.as_mut().unwrap();
        a.bauteile.clear();
        a.geschosse.clear();
    }
    let p = core
        .create_project(
            NewProject {
                name: "Einbindung".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    bebauungsplan_art: core_lib::domain::BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(plan.clone()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    let haus = Uuid::new_v4();
    let daten = json!({"bezeichnung":"Wohnhaus","grundstueck_ids":[],"flurstueck_ids":grund[0].flurstueck_ids,"quelle":null,"geometrie":{"grundflaeche_m2":100,"typ":"hauptanlage"},"geschosse":(["EG","OG"].iter().map(|s|json!({"id":Uuid::new_v4(),"bezeichnung":s,"vollgeschoss":true,"grundlage":"Kontrollfall","geometrie":{"flaeche_m2":100}})).collect::<Vec<_>>())});
    core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, 0, daten, &user)
        .await
        .unwrap();
    let e = core
        .projekt_baugrundstuecke_gebaeude_einbinden(p.id, &plan)
        .await
        .unwrap();
    assert!(e.hinweise.is_empty(), "{:?}", e.hinweise);
    let r = core
        .projekt_baugrundstuecke_pruefen(p.id, &plan)
        .await
        .unwrap();
    for b in &r.baugrundstuecke {
        let a = b.pruefung.aufstellung.as_ref().unwrap();
        if b.grundstueck.schluessel == grund[0].schluessel {
            assert_eq!(a.mass.grundflaeche_m2, 100.);
            assert_eq!(a.mass.geschossflaeche_m2, 200.);
            assert_eq!(a.vollgeschosse, 2);
        } else {
            assert_eq!(
                a.mass.grundflaeche_m2, 0.,
                "kein Gebäude auf dem zweiten Grundstück"
            );
        }
    }
    // Ein Nachweis aus dem Plan ohne Verknüpfungen enthält das Gebäude ebenfalls.
    let n = core
        .nachweis_erstellen(
            p.id,
            Uuid::new_v4(),
            "GRZ".into(),
            NachweisEingabe::Nutzung {
                fach: NutzungsArt::Grz,
                plan: plan.clone(),
            },
            &user,
        )
        .await
        .unwrap();
    let NachweisEingabe::Nutzung {
        plan: gespeichert, ..
    } = &n.eingabe
    else {
        panic!("Nutzungsnachweis erwartet")
    };
    assert_eq!(gespeichert, &e.plan);
    // Ein neu zugeordnetes Gebäude macht den Nachweis veraltet.
    let garage = json!({"bezeichnung":"Garage","grundstueck_ids":[],"flurstueck_ids":grund[0].flurstueck_ids,"quelle":null,"geschosse":[],"geometrie":{"grundflaeche_m2":20,"typ":"garage"}});
    core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, Uuid::new_v4(), 0, garage, &user)
        .await
        .unwrap();
    assert!(
        !core
            .nachweise(p.id)
            .await
            .unwrap()
            .iter()
            .find(|a| a.stand.id == n.id)
            .unwrap()
            .aktuell
    );
}

#[tokio::test]
async fn flurstueckszuordnung_uebersteht_umgruppierung_und_speist_formular_und_bauantrag() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("formulare", "Prüfbüro").await.unwrap();
    // Projekt zunächst nur mit vorläufiger Projektfläche.
    let p = core
        .create_project(
            NewProject {
                name: "Umgruppierung".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Planungsrecht {
                    bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
                    bebauungsplan_art: core_lib::domain::BebauungsplanArt::Qualifiziert,
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let vorlaeufig = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    assert_eq!(vorlaeufig[0].schluessel, "projektflaeche");
    // Gebäude zunächst ohne Flurstücke erfasst (noch keine bekannt).
    let haus = Uuid::new_v4();
    let daten = json!({"bezeichnung":"Wohnhaus","grundstueck_ids":[],"quelle":null,"geometrie":{"grundflaeche_m2":100,"typ":"hauptanlage"},"geschosse":[]});
    let stand = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, 0, daten.clone(), &user)
        .await
        .unwrap();
    // Neue Gebäude werden nicht mehr Baugrundstücken zugeordnet.
    let mut alt = daten.clone();
    alt["grundstueck_ids"] = json!([vorlaeufig[0].id]);
    assert!(
        core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, stand.revision, alt, &user)
            .await
            .is_err()
    );
    // Flurstücke erfassen: a+b und c bilden zwei Baugrundstücke.
    let projekt_mit = |p: &core_lib::domain::Project,
                       plan: core_lib::baugrundstuecke::BaugrundstuecksPlan| {
        core_lib::domain::ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: None,
            planungsrecht: Some(Planungsrecht {
                baugrundstuecke: Some(plan),
                grundstuecksinformationen: None,
                baugrundstuecksermittlung: None,
                ..p.planungsrecht.clone()
            }),
        }
    };
    let mut plan = core_lib::baugrundstuecke::musterplan();
    for b in &mut plan.berechnungen {
        let a = b.eingabe.aufstellung.as_mut().unwrap();
        a.bauteile.clear();
        a.geschosse.clear();
    }
    let p = core
        .update_project(projekt_mit(&p, plan.clone()), &user)
        .await
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    assert_eq!(grund.len(), 2);
    let gespeichert = |p: &core_lib::domain::Project| {
        core_lib::grundstuecksinformationen::plan(&p.planungsrecht)
            .unwrap()
            .unwrap()
    };
    let e = core
        .projekt_baugrundstuecke_gebaeude_einbinden(p.id, &gespeichert(&p))
        .await
        .unwrap();
    assert!(
        e.hinweise
            .iter()
            .any(|h| h.text.contains("Keinem Flurstück"))
    );
    // Unbekanntes Flurstück wird abgewiesen, Flurstück c angenommen.
    let mut fremd = daten.clone();
    fremd["flurstueck_ids"] = json!(["unbekannt"]);
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Gebaeude,
            haus,
            stand.revision,
            fremd,
            &user
        )
        .await
        .is_err()
    );
    let mut auf_c = daten.clone();
    auf_c["flurstueck_ids"] = json!(["c"]);
    auf_c["geometrie"]["grundflaeche_m2"] = json!(110);
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        haus,
        stand.revision,
        auf_c,
        &user,
    )
    .await
    .unwrap();
    // Umgruppierung: c erhält dieselbe Grundbuchstelle wie a und b. Die
    // Gebäudezuordnung bleibt ohne Übertragung gültig.
    let mut neu = plan.clone();
    neu.flurstuecke
        .iter_mut()
        .find(|f| f.id == "c")
        .unwrap()
        .laufende_nummer = 1;
    let einheit = neu.gruppen(true).unwrap().remove(0);
    neu.berechnungen = vec![core_lib::baugrundstuecke::BaugrundstueckBerechnung {
        schluessel: einheit.schluessel.clone(),
        eingabe: plan.berechnungen[0].eingabe.clone(),
        eigene_festsetzungen: None,
        flurstueck_ids: vec![],
    }];
    let p = core
        .update_project(projekt_mit(&p, neu), &user)
        .await
        .unwrap();
    let grund = core_lib::grundstueckskontext::ermitteln(&p).unwrap();
    assert_eq!(grund.len(), 1);
    let e = core
        .projekt_baugrundstuecke_gebaeude_einbinden(p.id, &gespeichert(&p))
        .await
        .unwrap();
    // Nur der offene Hinweis zur Einstufung des Einzelgeschosses, keiner zur Lage.
    assert!(
        e.hinweise
            .iter()
            .all(|h| h.text.contains("Vollgeschosseinstufung")),
        "{:?}",
        e.hinweise
    );
    // Formular erhält die GRZ-Fläche aus dem Gebäude, ohne Neueingabe.
    let q = core
        .formular_quellen(
            p.id,
            core_lib::formular::instanz::FormularBezug {
                grundstueck_id: Some(grund[0].id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        q.datenstand.werte.wert("flaechen.grundflaeche_m2"),
        Some("110,00"),
        "{:?}",
        q.hinweise
    );
    // Bauantrag übernimmt die Gebäudeanzahl.
    let antrag = core.bauantrag(p.id).await.unwrap();
    assert!(antrag.gebaeude_zentral);
    assert_eq!(antrag.antrag.anzahl_gebaeude, 1);
    // Verweise auf ein Flurstück werden gefunden; bereinigt wird erst, wenn es
    // tatsächlich nicht mehr erfasst ist.
    let ids = vec!["c".to_string()];
    let v = core.flurstuecksverweise(p.id, &ids).await.unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].bereich, "gebaeude");
    assert!(
        core.flurstuecksverweise_entfernen(p.id, &ids, &user)
            .await
            .is_err()
    );
    let mut ohne_c = plan.clone();
    ohne_c.flurstuecke.retain(|f| f.id != "c");
    let ab = ohne_c.gruppen(true).unwrap().remove(0).schluessel;
    ohne_c.berechnungen.retain(|b| b.schluessel == ab);
    let p = core
        .update_project(projekt_mit(&p, ohne_c), &user)
        .await
        .unwrap();
    assert_eq!(
        core.flurstuecksverweise_entfernen(p.id, &ids, &user)
            .await
            .unwrap(),
        1
    );
    assert!(
        core.flurstuecksverweise(p.id, &ids)
            .await
            .unwrap()
            .is_empty()
    );
}
