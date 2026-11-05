//! Bereichsstatus der Projektnavigation: Pflichtbereiche, Bebauungsplan →
//! Planfestsetzungen, leere Bereiche ohne Symbol, Grundlagen begonnener Nachweise.
use chrono::NaiveDate;
use core_lib::{
    CoreServices,
    bereichsstatus::{Bereichsstaende, Bereichszustand},
    domain::{BebauungsplanArt, Bundesland, NewProject, Rechtsgrundlage},
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use std::sync::Arc;

fn zustand(s: &Bereichsstaende, id: &str) -> Bereichszustand {
    s.bereiche
        .iter()
        .find(|b| b.bereich == id)
        .unwrap_or_else(|| panic!("Bereich {id} fehlt"))
        .zustand
}
fn gruende(s: &Bereichsstaende, id: &str) -> String {
    s.bereiche
        .iter()
        .find(|b| b.bereich == id)
        .unwrap()
        .gruende
        .join(" | ")
}

#[tokio::test]
async fn neues_projekt_und_bebauungsplan() {
    use Bereichszustand::*;
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Wohnhaus Lindenweg 5".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let tag = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
    let s = c.bereichsstatus_am(p.id, tag, &u).await.unwrap();
    // Pflicht: Stammdaten, Rechtsgrundlagen, Flurstücke.
    for id in ["uebersicht", "rechtsgrundlagen", "flurstuecke"] {
        assert_eq!(zustand(&s, id), ZuErledigen, "{id}");
    }
    assert!(gruende(&s, "uebersicht").contains("Bauherr"));
    assert!(gruende(&s, "flurstuecke").contains("Vorhabenflurstück"));
    // Ohne Bebauungsplan und ohne Daten: kein Symbol.
    for id in [
        "festsetzungen",
        "raumbuch",
        "gebaeude",
        "wohnflaechen",
        "stellplaetze",
        "nachweise",
        "aufgaben",
        "fachplaner",
    ] {
        assert_eq!(zustand(&s, id), Leer, "{id}");
    }
    // Reine Ansichten haben keinen Status.
    for id in ["projektuebersicht", "protokoll", "messwerte", "druck"] {
        assert!(s.bereiche.iter().all(|b| b.bereich != id), "{id}");
    }

    // Bebauungsplan in den Rechtsgrundlagen → Planfestsetzungen werden benötigt.
    let mut neu = p.clone();
    neu.planungsrecht.rechtsgrundlage = Some(Rechtsgrundlage::Par30);
    neu.planungsrecht.bebauungsplan_art = BebauungsplanArt::Qualifiziert;
    neu.planungsrecht.bau_nvo = Some(core_lib::flaechen::BauNvo::V1990);
    let neu = c
        .update_project(
            core_lib::domain::ProjectUpdate {
                id: p.id,
                name: neu.name,
                bundesland: neu.bundesland,
                grundstuecksflaeche_m2: None,
                planungsrecht: Some(neu.planungsrecht),
                expected_version: p.version,
            },
            &u,
        )
        .await
        .unwrap();
    let s = c.bereichsstatus_am(p.id, tag, &u).await.unwrap();
    assert_eq!(zustand(&s, "rechtsgrundlagen"), Geprueft);
    assert_eq!(zustand(&s, "festsetzungen"), ZuErledigen);
    assert!(gruende(&s, "festsetzungen").contains("GRZ"));

    // Mit erfasstem Maß der Nutzung sind die Festsetzungen erledigt.
    let mut recht = neu.planungsrecht.clone();
    recht.festgesetzte_werte.grz = Some(0.4);
    recht.festgesetzte_werte.vollgeschosse = Some(2);
    c.update_project(
        core_lib::domain::ProjectUpdate {
            id: p.id,
            name: neu.name.clone(),
            bundesland: neu.bundesland,
            grundstuecksflaeche_m2: None,
            planungsrecht: Some(recht),
            expected_version: neu.version,
        },
        &u,
    )
    .await
    .unwrap();
    let s = c.bereichsstatus_am(p.id, tag, &u).await.unwrap();
    assert_eq!(zustand(&s, "festsetzungen"), Geprueft);

    // Ein leerer Datenbereich lässt sich als nicht benötigt kennzeichnen …
    assert!(
        s.bereiche
            .iter()
            .find(|b| b.bereich == "stellplaetze")
            .unwrap()
            .kennzeichenbar
    );
    let s = c
        .bereich_benoetigt_setzen(p.id, "stellplaetze", false, &u)
        .await
        .unwrap();
    assert_eq!(zustand(&s, "stellplaetze"), NichtBenoetigt);
    // … Pflichtbereiche und Bereiche mit Daten nicht.
    assert!(
        c.bereich_benoetigt_setzen(p.id, "flurstuecke", false, &u)
            .await
            .is_err()
    );
    assert!(
        c.bereich_benoetigt_setzen(p.id, "festsetzungen", false, &u)
            .await
            .is_err()
    );
    // Die Kennzeichnung gilt projektweit und lässt sich zurücknehmen.
    let s = c.bereichsstatus_am(p.id, tag, &u).await.unwrap();
    assert_eq!(zustand(&s, "stellplaetze"), NichtBenoetigt);
    let s = c
        .bereich_benoetigt_setzen(p.id, "stellplaetze", true, &u)
        .await
        .unwrap();
    assert_eq!(zustand(&s, "stellplaetze"), Leer);
}
