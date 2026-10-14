//! Kundendatenbank: Kunden pflegen, Projekten als Bauherren zuordnen und
//! Bauherren aus Bauanträgen älterer Versionen übernehmen.

use std::sync::Arc;

use core_lib::benutzer::{NeuerBenutzer, Rolle};
use core_lib::domain::{Aktion, Bundesland, NewProject, Project};
use core_lib::kunden::{Kundendaten, Kundenkategorie, Kundentyp};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::{CoreError, CoreServices};

async fn core(dir: &std::path::Path) -> CoreServices {
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.to_path_buf(),
    })
    .unwrap();
    CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
}

async fn projekt(core: &CoreServices, name: &str) -> Project {
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    core.create_project(
        NewProject {
            name: name.into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        },
        &chefin,
    )
    .await
    .unwrap()
}

fn investor() -> Kundendaten {
    Kundendaten {
        typ: Kundentyp::Organisation,
        kategorie: Kundenkategorie::Investor,
        firma: "  Wohnbau Süd GmbH ".into(),
        // Gehört zu Personen und wird verworfen.
        vorname: "Max".into(),
        ..Kundendaten::default()
    }
}

#[tokio::test]
async fn kunden_pflegen_und_zuordnen() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ben = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "ben".into(),
                anzeigename: "Ben Bearbeiter".into(),
                rolle: Rolle::Bearbeitung,
            },
            &chefin,
        )
        .await
        .unwrap();
    let leser = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "leser".into(),
                anzeigename: "Leo Leser".into(),
                rolle: Rolle::Lesen,
            },
            &chefin,
        )
        .await
        .unwrap();

    assert!(matches!(
        core.kunde_anlegen(investor(), &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    assert!(matches!(
        core.kunde_anlegen(Kundendaten::default(), &ben).await,
        Err(CoreError::InvalidInput(_))
    ));
    let k = core.kunde_anlegen(investor(), &ben).await.unwrap();
    assert_eq!(k.daten.firma, "Wohnbau Süd GmbH");
    assert_eq!(k.daten.vorname, "");
    let erika = core
        .kunde_anlegen(
            Kundendaten {
                vorname: "Erika".into(),
                familienname: "Mustermann".into(),
                ..Kundendaten::default()
            },
            &ben,
        )
        .await
        .unwrap();
    assert_eq!(
        core.kunden_auflisten()
            .await
            .unwrap()
            .iter()
            .map(|k| k.anzeigename())
            .collect::<Vec<_>>(),
        ["Erika Mustermann", "Wohnbau Süd GmbH"]
    );

    let p = projekt(&core, "MFH Lindenweg").await;
    assert!(matches!(
        core.bauherren_setzen(p.id, vec![k.id], p.version, &leser)
            .await,
        Err(CoreError::Forbidden(_))
    ));
    // Doppelt genannte Kunden zählen einmal.
    let p = core
        .bauherren_setzen(p.id, vec![k.id, erika.id, k.id], p.version, &ben)
        .await
        .unwrap();
    assert_eq!(p.version, 2);
    assert_eq!(
        core.bauherren(p.id)
            .await
            .unwrap()
            .iter()
            .map(|k| k.id)
            .collect::<Vec<_>>(),
        [k.id, erika.id]
    );

    // Eine Änderung am Kunden steht im Protokoll des Projekts.
    let mut daten = k.daten.clone();
    daten.anschrift.ort = "München".into();
    core.kunde_aendern(k.id, daten, k.version, &ben)
        .await
        .unwrap();
    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::KundeGeaendert
                && e.eintrag.details == "Bauherr Wohnbau Süd GmbH")
    );
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::BauherrenGeaendert
                && e.eintrag.details == "Wohnbau Süd GmbH, Erika Mustermann")
    );

    // Löschen: nur Projektleitung, und nur ohne Projekt.
    assert!(matches!(
        core.kunde_loeschen(erika.id, 1, &ben).await,
        Err(CoreError::Forbidden(_))
    ));
    let err = core.kunde_loeschen(erika.id, 1, &chefin).await.unwrap_err();
    assert!(err.to_string().contains("„MFH Lindenweg“"), "{err}");
    core.bauherren_setzen(p.id, vec![k.id], p.version, &ben)
        .await
        .unwrap();
    core.kunde_loeschen(erika.id, 1, &chefin).await.unwrap();
    assert!(matches!(
        core.kunde(erika.id).await,
        Err(CoreError::NotFound(_))
    ));
}

/// Bauantrag, wie ihn Versionen vor der Kundendatenbank gespeichert haben.
fn alter_bauantrag(familienname: &str) -> String {
    format!(
        r#"{{"bezeichnung":"Neubau","bauherren":[
            {{"art":"person","id":"6f1c1d7e-1111-4222-8333-944455556666","vorname":"Anna",
              "familienname":"{familienname}","anschrift":{{"ort":"Köln"}}}},
            {{"art":"organisation","id":"6f1c1d7e-1111-4222-8333-944455557777","name":"",
              "anschrift":{{}}}}
        ],"massnahmen":["11"],"anzahl_gebaeude":1}}"#
    )
}

#[tokio::test]
async fn bauherren_aus_alten_bauantraegen_uebernehmen() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let a = projekt(&core, "Haus A").await;
    let b = projekt(&core, "Haus B").await;
    let c = projekt(&core, "Haus C").await;
    let r = &core.repos.bauantraege;
    r.bauantrag_speichern(a.id, &alter_bauantrag("Müller"), 0, "alt")
        .await
        .unwrap();
    // Dieselbe Person in anderer Schreibweise: wird nicht doppelt angelegt.
    r.bauantrag_speichern(b.id, &alter_bauantrag("MÜLLER "), 0, "alt")
        .await
        .unwrap();
    r.bauantrag_speichern(c.id, &alter_bauantrag("Schmidt"), 0, "alt")
        .await
        .unwrap();

    // Bis zur Übernahme gelten die alten Bauherren weiter.
    let vorher = core.bauantrag(a.id).await.unwrap();
    assert!(
        !vorher.fehlende_angaben.iter().any(|f| f == "Bauherr"),
        "{:?}",
        vorher.fehlende_angaben
    );

    // Speichern übernimmt vorab, ohne Versionskonflikt.
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ab_hier = core.protokoll(c.id, 50).await.unwrap().len();
    let g = core
        .bauantrag_speichern(c.id, vorher.antrag.clone(), 1, &chefin)
        .await
        .unwrap();
    assert_eq!(g.version, 3);
    let schmidt = core.bauherren(c.id).await.unwrap();
    assert_eq!(schmidt.len(), 1);
    assert_eq!(schmidt[0].anzeigename(), "Anna Schmidt");
    let neu = core.protokoll(c.id, 50).await.unwrap();
    assert!(
        neu[..neu.len() - ab_hier]
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::KundeAngelegt
                && e.eintrag.details == "Anna Schmidt (aus dem Bauantrag übernommen)")
    );

    assert_eq!(core.bauherren_uebernehmen(&chefin).await.unwrap(), 2);
    let kunden = core.kunden_auflisten().await.unwrap();
    assert_eq!(
        kunden
            .iter()
            .map(|k| k.anzeigename().to_lowercase())
            .collect::<Vec<_>>(),
        ["anna müller", "anna schmidt"],
        "leere Bauherren werden nicht übernommen"
    );
    let mueller = &kunden[0];
    assert_eq!(mueller.daten.anschrift.ort, "Köln");
    for p in [&a, &b] {
        let bauherren = core.bauherren(p.id).await.unwrap();
        assert_eq!(
            bauherren.iter().map(|k| k.id).collect::<Vec<_>>(),
            [mueller.id]
        );
        let d = r.bauantrag(p.id).await.unwrap().unwrap();
        assert!(!d.daten.contains("bauherren"), "{}", d.daten);
        assert!(d.daten.contains("Neubau"));
    }
    assert_eq!(core.kunde_projekte(mueller.id).await.unwrap().len(), 2);

    // Wiederholbar.
    assert_eq!(core.bauherren_uebernehmen(&chefin).await.unwrap(), 0);
    assert_eq!(core.kunden_auflisten().await.unwrap().len(), 2);
}
