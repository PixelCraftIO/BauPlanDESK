//! Übergaben und unabhängige Fachbereiche mit echter SQLite- und Dateiablage.
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject, Project, ProjectUpdate},
    lbo::LboEngine,
    license::DevelopmentLicense,
    projektbereiche::ProjektBereich,
};
use std::sync::Arc;
fn update(p: &Project) -> ProjectUpdate {
    ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
        planungsrecht: Some(p.planungsrecht.clone()),
    }
}
#[tokio::test]
async fn bereiche_speichern_unabhaengig_aber_gleiche_felder_kollidieren() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Haus".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(640.),
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut a = update(&p);
    a.planungsrecht.as_mut().unwrap().plan_bezeichnung = "Plan A".into();
    let erster = core
        .projekt_bereich_speichern(p.clone(), ProjektBereich::Grundlagen, a.clone(), &user)
        .await
        .unwrap();
    let mut b = update(&p);
    b.planungsrecht.as_mut().unwrap().festgesetzte_werte.grz = Some(0.3);
    let zweiter = core
        .projekt_bereich_speichern(p.clone(), ProjektBereich::Festsetzungen, b, &user)
        .await
        .unwrap();
    assert_eq!(zweiter.planungsrecht.plan_bezeichnung, "Plan A");
    assert_eq!(zweiter.planungsrecht.festgesetzte_werte.grz, Some(0.3));
    assert!(zweiter.version > erster.version);
    let wiederholt = core
        .projekt_bereich_speichern(p.clone(), ProjektBereich::Grundlagen, a, &user)
        .await
        .unwrap();
    assert_eq!(wiederholt.version, zweiter.version);
    let mut c = update(&p);
    c.planungsrecht.as_mut().unwrap().plan_bezeichnung = "Plan B".into();
    assert!(matches!(
        core.projekt_bereich_speichern(p.clone(), ProjektBereich::Grundlagen, c, &user)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));
    let mut d = update(&p);
    d.planungsrecht.as_mut().unwrap().gemeinde = "Musterstadt".into();
    let d = core
        .projekt_bereich_speichern(p, ProjektBereich::Stammdaten, d, &user)
        .await
        .unwrap();
    assert_eq!(d.planungsrecht.gemeinde, "Musterstadt");
    assert_eq!(d.planungsrecht.plan_bezeichnung, "Plan A");
}
#[tokio::test]
async fn rechtswechsel_blockiert_nicht_an_veralteten_abhaengigen_ansatzdaten() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Haus".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: core_lib::domain::Planungsrecht {
                    bebauungsplan_art: core_lib::domain::BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(core_lib::baugrundstuecke::musterplan()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let mut a = update(&p);
    let recht = a.planungsrecht.as_mut().unwrap();
    recht.bebauungsplan_art = core_lib::domain::BebauungsplanArt::Kein;
    recht.rechtsgrundlage = Some(core_lib::domain::Rechtsgrundlage::Par35);
    let neu = core
        .projekt_bereich_speichern(p.clone(), ProjektBereich::Grundlagen, a, &user)
        .await
        .unwrap();
    assert_eq!(
        neu.planungsrecht.rechtsgrundlage,
        Some(core_lib::domain::Rechtsgrundlage::Par35)
    );
    assert!(core.grundstueckskontext(neu.id).await.is_err());
}

#[tokio::test]
async fn nachweisarchiv_bleibt_bei_regelupdate_reproduzierbar() {
    use core_lib::{
        calc::Baugebiet,
        lbo::{AbstandsflaecheInput, RulePackSource},
        plattform::nachweise::NachweisEingabe,
    };
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let lbo = Arc::new(LboEngine::new());
    let script = include_str!("../../../lbo-rules/by.rhai");
    lbo.load_rule_pack(RulePackSource {
        land: Bundesland::By,
        script: script.into(),
        origin: "Testregel".into(),
    })
    .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        lbo.clone(),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Haus".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let eingabe = NachweisEingabe::Abstand {
        gegenstand: "Nordwand".into(),
        eingabe: AbstandsflaecheInput {
            wandhoehe_m: 10.,
            gebiet: Baugebiet::WA,
        },
    };
    let id = uuid::Uuid::new_v4();
    let n = core
        .nachweis_erstellen(p.id, id, "Nordwand".into(), eingabe.clone(), &user)
        .await
        .unwrap();
    assert_eq!(n.ergebnis["ergebnis"]["tiefe_m"], 4.);
    core.nachweis_erstellen(p.id, id, "Nordwand".into(), eingabe, &user)
        .await
        .unwrap();
    assert_eq!(core.nachweise(p.id).await.unwrap().len(), 1);
    core.nachweis_freigeben(p.id, id, "Geprüfter Testfall".into(), &user)
        .await
        .unwrap();
    let mut a = update(&p);
    a.planungsrecht.as_mut().unwrap().gemeinde = "Andere Anschrift".into();
    core.projekt_bereich_speichern(p.clone(), ProjektBereich::Stammdaten, a, &user)
        .await
        .unwrap();
    assert!(core.nachweise(p.id).await.unwrap()[0].aktuell);
    lbo.load_rule_pack(RulePackSource {
        land: Bundesland::By,
        script: script.replace("2026.1", "2026.2").replace("0.4", "0.5"),
        origin: "Neue Regel".into(),
    })
    .unwrap();
    let liste = core.nachweise(p.id).await.unwrap();
    assert!(!liste[0].aktuell);
    assert!(liste[0].freigabe.is_some());
    assert_eq!(
        core.nachweis_wiederholen(p.id, id).await.unwrap(),
        n.ergebnis
    );
    assert!(
        core.nachweis_freigeben(p.id, id, "Erneut prüfen".into(), &user)
            .await
            .is_err()
    );
    assert!(
        core.nachweis_erstellen(
            p.id,
            uuid::Uuid::new_v4(),
            "GRZ ohne Flurstücke".into(),
            core_lib::plattform::nachweise::NachweisEingabe::Nutzung {
                fach: core_lib::plattform::nachweise::NutzungsArt::Grz,
                plan: Default::default()
            },
            &user
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn gebaeude_raumbuch_und_historie_verwenden_dieselbe_identitaet() {
    use core_lib::plattform::ObjektArt;
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Haus".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let id = uuid::Uuid::new_v4();
    let daten = serde_json::json!({"bezeichnung":"Gebäude A","grundstueck_ids":[],"geschosse":[],"quelle":null});
    let erster = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, 0, daten.clone(), &user)
        .await
        .unwrap();
    let gleich = core
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, 0, daten, &user)
        .await
        .unwrap();
    assert_eq!(erster.revision, gleich.revision);
    let mut neu = erster.daten.clone();
    neu["bezeichnung"] = serde_json::json!("Haupthaus");
    core.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, 1, neu, &user)
        .await
        .unwrap();
    let history = core
        .fachverlauf(p.id, ObjektArt::Gebaeude, id)
        .await
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].daten["bezeichnung"], "Gebäude A");
    let raumbuch = uuid::Uuid::new_v4();
    let raum = uuid::Uuid::new_v4();
    let daten = serde_json::json!({"bezeichnung":"EG","gebaeude_id":id,"geschoss_id":null,"raeume":[{"id":raum,"nummer":"1","bezeichnung":"Büro","nutzung":"Arbeiten","flaeche_m2":20,"hoehe_m":2.5}]});
    core.fachobjekt_speichern(p.id, ObjektArt::Raumbuch, raumbuch, 0, daten, &user)
        .await
        .unwrap();
    let summe = core.raumbuch_summen(p.id, raumbuch).await.unwrap();
    assert_eq!(summe.flaeche_m2, 20.);
    assert_eq!(summe.volumen_m3, Some(50.));
}

#[tokio::test]
async fn checkliste_ist_wiederholbar_und_status_erkennt_neue_dokumentversion() {
    use core_lib::plattform::{
        ObjektArt,
        ablauf::Checkliste,
        objekte::{Dokumentbezug, Unterlage, UnterlagenStatus},
    };
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("ablauf", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Checkliste".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let vorlage = Checkliste::standard().unwrap();
    assert_eq!(
        core.checkliste_anwenden(p.id, vorlage.clone(), &user)
            .await
            .unwrap(),
        6
    );
    assert_eq!(
        core.checkliste_anwenden(p.id, vorlage.clone(), &user)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        core.projektstatus(p.id).await.unwrap().fehlende_unterlagen,
        4
    );
    let f = core
        .fachobjekte(p.id, ObjektArt::Unterlagen)
        .await
        .unwrap()
        .into_iter()
        .find(|f| f.daten["vorlage"]["punkt"] == "formular")
        .unwrap();
    let mut u: Unterlage = serde_json::from_value(f.daten).unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Antrag.pdf",
            bytes::Bytes::from_static(b"V1"),
            &user,
        )
        .await
        .unwrap();
    u.dokument = Some(Dokumentbezug {
        dokument_id: d.document_id,
        version: 1,
    });
    u.status = UnterlagenStatus::Freigegeben;
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Unterlagen,
        uuid::Uuid::parse_str(&f.objekt).unwrap(),
        f.revision,
        serde_json::to_value(&u).unwrap(),
        &user,
    )
    .await
    .unwrap();
    let s = core.projektstatus(p.id).await.unwrap();
    assert_eq!(s.fehlende_unterlagen, 3);
    assert_eq!(s.ungepruefte_unterlagen, 0);
    core.store_document(
        p.id,
        Some(d.document_id),
        Some(1),
        "Antrag.pdf",
        bytes::Bytes::from_static(b"V2"),
        &user,
    )
    .await
    .unwrap();
    let s = core.projektstatus(p.id).await.unwrap();
    assert_eq!(s.ungepruefte_unterlagen, 1);
    // Eine neue Vorlage überschreibt keine projektspezifische Prüfung/Zuordnung.
    let mut update = vorlage;
    update.version = "2.0".into();
    update.unterlagen[0].bezeichnung = "Neuer Vorschlag".into();
    assert_eq!(
        core.checkliste_anwenden(p.id, update, &user).await.unwrap(),
        0
    );
    let stand = core
        .fachobjekte(p.id, ObjektArt::Unterlagen)
        .await
        .unwrap()
        .into_iter()
        .find(|g| g.objekt == f.objekt)
        .unwrap();
    assert_eq!(stand.daten["vorlage"]["version"], "1.0");
    assert_eq!(stand.daten["dokument"]["version"], 1);
}

#[tokio::test]
async fn ausgabeauftrag_ueberlebt_neustart_und_verwirft_veraltete_bestaetigung() {
    use core_lib::{
        domain::{Aktion, NeuerProtokolleintrag},
        plattform::ausgabe::AusgabeStatus,
    };
    let dir = tempfile::tempdir().unwrap();
    let dbpath = dir.path().join("projekt.db");
    let db = db_sqlite::SqliteDatabase::open(&dbpath).await.unwrap();
    let ablage = Arc::new(storage::LocalStorage::new(dir.path().join("DMS"), "test").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        ablage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("ausgabe", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Ausgabe".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Antrag.pdf",
            bytes::Bytes::from_static(b"Original"),
            &user,
        )
        .await
        .unwrap();
    assert!(
        core.repos
            .commit
            .dokumentzusatz(d.document_id, 1, "ausgabeereignis")
            .await
            .unwrap()
            .is_some()
    );
    let (f, a) = core
        .ausgabe_beanspruchen(
            &d,
            "Ziel".into(),
            "Projekt/Dokumente/Antrag.pdf".into(),
            false,
            &user,
        )
        .await
        .unwrap()
        .unwrap();
    assert!(
        core.ausgabe_beanspruchen(&d, "Ziel".into(), a.relativer_pfad.clone(), true, &user)
            .await
            .unwrap()
            .is_none()
    );
    // Simuliert eine abgelaufene Bearbeitungsfrist nach Prozessabbruch, ohne reale Wartezeit.
    let mut abgelaufen = a.clone();
    abgelaufen.beansprucht_bis =
        (std::time::SystemTime::now() - std::time::Duration::from_secs(60)).into();
    let mut neu = f.clone();
    neu.daten = serde_json::to_value(abgelaufen).unwrap();
    core.repos
        .commit
        .fachstand(
            neu,
            f.revision,
            NeuerProtokolleintrag {
                zeit: d.created_at,
                benutzer_id: user.id,
                benutzer_name: user.anzeigename.clone(),
                aktion: Aktion::AusgabeauftragGeaendert,
                project_id: Some(p.id),
                document_id: Some(d.document_id),
                details: "Test: Prozessabbruch".into(),
            },
        )
        .await
        .unwrap();
    drop(core);
    drop(db);
    let db = db_sqlite::SqliteDatabase::open(&dbpath).await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        ablage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let (f2, a2) = core
        .ausgabe_beanspruchen(&d, "Ziel".into(), a.relativer_pfad.clone(), false, &user)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a2.versuche, 2);
    assert!(matches!(
        core.ausgabe_abschliessen(f, a.clone(), None, &user).await,
        Err(CoreError::VersionConflict { .. })
    ));
    core.ausgabe_abschliessen(
        f2,
        a2,
        Some("Freigabe vorübergehend nicht erreichbar".into()),
        &user,
    )
    .await
    .unwrap();
    assert!(
        core.ausgabe_beanspruchen(&d, "Ziel".into(), a.relativer_pfad.clone(), false, &user)
            .await
            .unwrap()
            .is_none()
    );
    let (f3, a3) = core
        .ausgabe_beanspruchen(&d, "Ziel".into(), a.relativer_pfad, true, &user)
        .await
        .unwrap()
        .unwrap();
    core.ausgabe_abschliessen(f3, a3, None, &user)
        .await
        .unwrap();
    let liste = core.ausgabeauftraege(p.id).await.unwrap();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].1.status, AusgabeStatus::Fertig);
    assert_eq!(liste[0].1.versuche, 3);
    assert_eq!(core.list_documents(p.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn geoeffneter_arbeitsplatz_kann_entzogene_adminrechte_nicht_weiterverwenden() {
    use core_lib::benutzer::{BenutzerAenderung, NeuerBenutzer, Rolle};
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let alt = core.anmelden("erster", "Erster Admin").await.unwrap();
    let admin = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "zweiter".into(),
                anzeigename: "Zweiter Admin".into(),
                rolle: Rolle::Administrator,
            },
            &alt,
        )
        .await
        .unwrap();
    core.benutzer_aendern(
        BenutzerAenderung {
            id: alt.id,
            anzeigename: alt.anzeigename.clone(),
            rolle: Rolle::Lesen,
            aktiv: true,
        },
        &admin,
    )
    .await
    .unwrap();
    assert!(matches!(
        core.create_project(
            NewProject {
                name: "Nicht erlaubt".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default()
            },
            &alt
        )
        .await,
        Err(CoreError::Forbidden(_))
    ));
    assert!(matches!(
        core.benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "unberechtigt".into(),
                anzeigename: "Nicht erlaubt".into(),
                rolle: Rolle::Administrator
            },
            &alt
        )
        .await,
        Err(CoreError::Forbidden(_))
    ));
    assert!(
        core.repos
            .benutzer
            .by_anmeldename("unberechtigt")
            .await
            .unwrap()
            .is_none()
    );
}
