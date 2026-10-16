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
    let mut daten = json!({"bezeichnung":"Wohnhaus01","grundstueck_ids":[grund[0].id],"quelle":null,"geometrie":{"grundflaeche_m2":100,"typ":"hauptanlage","gelaende_m":-0.8},"geschosse":(["KG","EG","OG","DG"].iter().enumerate().map(|(i,s)|json!({"id":sids[i],"bezeichnung":s,"vollgeschoss":i==1 || i==2,"grundlage":"Fiktiver Kontrollfall","geometrie":{"flaeche_m2":100,"fussboden_m":-2.5+3.*i as f64,"lichte_hoehe_m":2.4,"lichte_hoehe_einheitlich":true,"deckendicke_m":0.3}})).collect::<Vec<_>>())});
    let stand = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, haus, 0, daten.clone(), &user)
        .await
        .unwrap();
    let garagendaten = json!({"bezeichnung":"Garage","grundstueck_ids":[grund[0].id],"quelle":null,"geschosse":[],"geometrie":{"grundflaeche_m2":20,"typ":"garage"}});
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
    mehrdeutig["grundstueck_ids"] = json!([grund[0].id, grund[1].id]);
    core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, garage, 1, mehrdeutig, &user)
        .await
        .unwrap();
    assert!(
        core.projekt_baugrundstuecke_pruefen(p.id, &plan)
            .await
            .is_err()
    );
    let mut fremd: FachPlan = voll;
    fremd.gebaeude_id = Some(Uuid::new_v4());
    assert!(core.berechnung_pruefen(p.id, fremd).await.is_err());
}
