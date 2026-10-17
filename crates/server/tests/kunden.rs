//! Kundendatenbank: Kunden pflegen, Projekten als Bauherren zuordnen und
//! Bauherren aus Bauanträgen älterer Versionen übernehmen.

use std::sync::Arc;

use core_lib::benutzer::{NeuerBenutzer, Rolle};
use core_lib::domain::{Aktion, Bundesland, NewProject, Project};
use core_lib::kunden::{
    Beteiligtenrolle, Beteiligtenzuordnung, Kundendaten, Kundenkategorie, Kundentyp,
};
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

fn person(vorname: &str, familienname: &str) -> Kundendaten {
    Kundendaten {
        vorname: vorname.into(),
        familienname: familienname.into(),
        ..Kundendaten::default()
    }
}

#[tokio::test]
async fn beteiligte_in_rollen_und_zustimmung_des_eigentuemers() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = projekt(&core, "Anbau Huber").await;
    let anna = core
        .kunde_anlegen(person("Anna", "Müller"), &chefin)
        .await
        .unwrap();
    let erben = core
        .kunde_anlegen(
            Kundendaten {
                typ: Kundentyp::Organisation,
                firma: "Erbengemeinschaft Huber".into(),
                ..Kundendaten::default()
            },
            &chefin,
        )
        .await
        .unwrap();
    let statik = core
        .kunde_anlegen(person("Tom", "Träger"), &chefin)
        .await
        .unwrap();
    let p = core
        .bauherren_setzen(p.id, vec![anna.id], p.version, &chefin)
        .await
        .unwrap();
    let z = |rolle, kunde| Beteiligtenzuordnung { rolle, kunde };
    let p = core
        .beteiligte_setzen(
            p.id,
            vec![
                z(Beteiligtenrolle::Fachplaner, statik.id),
                z(Beteiligtenrolle::Grundstueckseigentuemer, erben.id),
                z(Beteiligtenrolle::Grundstueckseigentuemer, anna.id),
                z(Beteiligtenrolle::Grundstueckseigentuemer, anna.id),
            ],
            p.version,
            &chefin,
        )
        .await
        .unwrap();
    let b = core.beteiligte(p.id).await.unwrap();
    let liste: Vec<_> = b.iter().map(|b| (b.rolle, b.kunde.anzeigename())).collect();
    assert_eq!(
        liste,
        [
            (
                Beteiligtenrolle::Grundstueckseigentuemer,
                "Erbengemeinschaft Huber".to_string()
            ),
            (
                Beteiligtenrolle::Grundstueckseigentuemer,
                "Anna Müller".to_string()
            ),
            (Beteiligtenrolle::Fachplaner, "Tom Träger".to_string()),
        ],
        "nach Rolle geordnet, ohne Dubletten"
    );

    // Nur der Eigentümer, der nicht Bauherr ist, muss zustimmen.
    let g = core.bauantrag(p.id).await.unwrap();
    assert_eq!(
        g.hinweise,
        ["Zustimmungserklärung von Erbengemeinschaft Huber (Grundstückseigentümer), nicht Bauherr"]
    );
    assert!(!g.fehlende_angaben.iter().any(|f| f.contains("Zustimmung")));

    // Ein Beteiligter kann wie ein Bauherr nicht gelöscht werden.
    let err = core
        .kunde_loeschen(statik.id, statik.version, &chefin)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)), "{err}");
    assert_eq!(core.kunde_projekte(statik.id).await.unwrap().len(), 1);

    // Veraltete Projektversion wird abgewiesen.
    let err = core
        .beteiligte_setzen(p.id, vec![], p.version - 1, &chefin)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::VersionConflict { .. }), "{err}");
    let p = core
        .beteiligte_setzen(p.id, vec![], p.version, &chefin)
        .await
        .unwrap();
    assert!(core.beteiligte(p.id).await.unwrap().is_empty());
    assert!(core.bauantrag(p.id).await.unwrap().hinweise.is_empty());
    core.kunde_loeschen(statik.id, statik.version, &chefin)
        .await
        .unwrap();
    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::BeteiligteGeaendert
                && e.eintrag.details.contains("Tom Träger (Fachplaner)"))
    );
}

#[tokio::test]
async fn flurstueckseigentuemer_sind_kunden_und_projektbeteiligte() {
    use core_lib::domain::ProjectUpdate;
    use core_lib::grundstuecksinformationen::{
        FlurstuecksInfo, FlurstuecksRolle, Grundstuecksinformationen,
    };
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = projekt(&core, "Anbau Huber").await;
    let anna = core
        .kunde_anlegen(person("Anna", "Müller"), &chefin)
        .await
        .unwrap();
    let emil = core
        .kunde_anlegen(person("Emil", "Huber"), &chefin)
        .await
        .unwrap();
    let nachbarin = core
        .kunde_anlegen(person("Nora", "Nachbar"), &chefin)
        .await
        .unwrap();
    let p = core
        .bauherren_setzen(p.id, vec![anna.id], p.version, &chefin)
        .await
        .unwrap();
    let fl = |id: &str, nummer: &str, rolle, eigentuemer: Vec<_>, text: &str| FlurstuecksInfo {
        id: id.into(),
        gemarkung: "Schwabing".into(),
        flur: String::new(),
        nummer: nummer.into(),
        flaeche_m2: 500.,
        grundbuchbezirk: String::new(),
        grundbuchblatt: String::new(),
        laufende_nummer: None,
        grundbuchauszug: None,
        eigentuemer: text.into(),
        eigentuemer_kunden: eigentuemer,
        bemerkung: String::new(),
        rolle,
    };
    let speichern = |p: &Project, flurstuecke: Vec<FlurstuecksInfo>| {
        let mut recht = p.planungsrecht.clone();
        recht.grundstuecksinformationen = Some(Grundstuecksinformationen { flurstuecke });
        core.update_project(
            ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name.clone(),
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: None,
                planungsrecht: Some(recht),
            },
            &chefin,
        )
    };
    let v = FlurstuecksRolle::Vorhaben;
    let n = FlurstuecksRolle::Nachbar;

    // Freitext ist keine neue Eigentümerangabe mehr.
    let err = speichern(&p, vec![fl("a", "12", v, vec![], "Emil Huber")])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("als Kunden zuordnen"), "{err}");
    // Ein Vorhabeneigentümer muss Bauherr oder verfügungsberechtigter Beteiligter sein.
    let err = speichern(&p, vec![fl("a", "12", v, vec![emil.id], "")])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Flurstück 12"), "{err}");
    let p = core
        .beteiligte_setzen(
            p.id,
            vec![Beteiligtenzuordnung {
                rolle: Beteiligtenrolle::Grundstueckseigentuemer,
                kunde: emil.id,
            }],
            p.version,
            &chefin,
        )
        .await
        .unwrap();
    // Nachbarflurstücke dürfen beliebigen Kunden gehören.
    let p = speichern(
        &p,
        vec![
            fl("a", "12", v, vec![anna.id, emil.id], ""),
            fl("b", "13", n, vec![nachbarin.id], ""),
        ],
    )
    .await
    .unwrap();
    let infos = p.planungsrecht.grundstuecksinformationen.clone().unwrap();
    assert_eq!(infos.flurstuecke[0].eigentuemer_kunden, [anna.id, emil.id]);

    // Wer Eigentümer am Vorhaben ist, bleibt verfügungsberechtigt.
    let err = core
        .beteiligte_setzen(p.id, vec![], p.version, &chefin)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("Emil Huber ist Eigentümer"),
        "{err}"
    );
    let err = core
        .bauherren_setzen(p.id, vec![], p.version, &chefin)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("Anna Müller ist Eigentümer"),
        "{err}"
    );
    // Ein Kunde am Flurstück lässt sich nicht löschen.
    let err = core
        .kunde_loeschen(nachbarin.id, nachbarin.version, &chefin)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Flurstückseigentümer"), "{err}");
    // Die Zustimmungspflicht folgt aus den Projektbeteiligten.
    assert_eq!(
        core.bauantrag(p.id).await.unwrap().hinweise,
        ["Zustimmungserklärung von Emil Huber (Grundstückseigentümer), nicht Bauherr"]
    );
}
