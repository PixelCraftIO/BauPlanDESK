//! Durchgehendes Musterverfahren, referenzielle Grenzen und unveränderliche Abgaben.
use bytes::Bytes;
use chrono::Utc;
use core_lib::{
    CoreError, CoreServices,
    benutzer::Benutzer,
    commit::Fachstand,
    domain::{Bundesland, NewProject, ProjectId},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    verfahren::{abgabe::*, profile::*, suche::Textquelle, *},
};
use std::{io::Read, sync::Arc};
use uuid::Uuid;
fn leer() -> Verfahrensakte {
    Verfahrensakte {
        portal: None,
        bezeichnung: "Neubau Musterstraße".into(),
        art: Verfahrensart::Bauantrag,
        antragsangaben: Default::default(),
        status: Verfahrensstatus::Vorbereitung,
        behoerde: Behoerde {
            id: Uuid::new_v4(),
            name: "Bauaufsicht Musterstadt".into(),
            kennung: "bab:test".into(),
            kontakt: "Bauamt".into(),
        },
        aktenzeichen: "BA-2026-42".into(),
        verantwortlich: None,
        vorgaenger: None,
        profil: None,
        kommunikation: vec![],
        nachforderungen: vec![],
        anforderungen: vec![],
        dokumentbeziehungen: vec![],
        abgaben: vec![],
        bescheide: vec![],
        ereignisse: vec![],
    }
}
fn pdf(text: &str) -> Vec<u8> {
    use lopdf::{Document, Stream, dictionary};
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let f = d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let c = d.add_object(Stream::new(
        dictionary! {},
        format!("BT /F 14 Tf 50 750 Td ({text}) Tj ET").into_bytes(),
    ));
    let p=d.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F"=>f}},"Contents"=>c});
    d.objects.insert(
        root,
        dictionary! {"Type"=>"Pages","Kids"=>vec![p.into()],"Count"=>1}.into(),
    );
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    d.trailer.set("Root", cat);
    let mut bytes = vec![];
    d.save_to(&mut bytes).unwrap();
    bytes
}
async fn speichern(
    c: &CoreServices,
    p: ProjectId,
    id: Uuid,
    basis: i64,
    a: &Verfahrensakte,
    u: &Benutzer,
) -> Fachstand {
    c.fachobjekt_speichern(
        p,
        ObjektArt::Verfahren,
        id,
        basis,
        serde_json::to_value(a).unwrap(),
        u,
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn muster_von_nachforderung_bis_genehmigtem_stand_und_neuem_entwurf() {
    let dir = tempfile::tempdir().unwrap();
    let dbpfad = dir.path().join("buero.db");
    let db = db_sqlite::SqliteDatabase::open(&dbpfad).await.unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage.clone(),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Musterverfahren".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let vid = Uuid::new_v4();
    let a = leer();
    let f = speichern(&core, p.id, vid, 0, &a, &user).await;
    assert_eq!(f.revision, 1);
    let k = Anforderungskatalog::standard().unwrap();
    let profil = Vorhabenprofil {
        bundesland: Bundesland::By,
        art: Verfahrensart::Bauantrag,
        gebaeudeklasse: Some(3),
        sonderbau: Some(false),
        nutzung: "Wohnen".into(),
    };
    let f = core
        .verfahren_profil_anwenden(p.id, vid, 1, profil, k, &user)
        .await
        .unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    assert!(
        a.anforderungen
            .iter()
            .all(|r| r.erforderlich == Erforderlichkeit::Ungeklaert)
    );
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(pdf("Lageplan v1")),
            &user,
        )
        .await
        .unwrap();
    let ref1 = Dokumentbezug {
        dokument_id: plan.document_id,
        version: 1,
    };
    let original = core
        .store_document(
            p.id,
            None,
            None,
            "Nachforderung.pdf",
            Bytes::from(pdf("Nachforderung mit drei Punkten")),
            &user,
        )
        .await
        .unwrap();
    let brief = Dokumentbezug {
        dokument_id: original.document_id,
        version: 1,
    };
    let kommunikation = Uuid::new_v4();
    let heute = Utc::now().date_naive();
    a.kommunikation.push(Kommunikation {
        id: kommunikation,
        art: Kommunikationsart::Brief,
        eingehend: true,
        datum: heute,
        betreff: "Nachforderung Lageplan".into(),
        beteiligte: "Bauamt".into(),
        inhalt: "Drei Punkte nachreichen".into(),
        original: Some(brief.clone()),
        bezuege: vec![],
    });
    a.nachforderungen.push(Nachforderung {
        id: Uuid::new_v4(),
        kommunikation,
        punkte: (1..=3)
            .map(|n| Forderungspunkt {
                id: Uuid::new_v4(),
                titel: format!("Punkt {n}"),
                grundlage: "Schreiben Seite 1".into(),
                verantwortlich: Some(user.id.0),
                frist: Some(Frist {
                    datum: heute,
                    ausloeser: "Eingang Schreiben".into(),
                    beleg: Some(brief.clone()),
                    erinnerung_tage: 7,
                    grund: "Behördliche Vorgabe".into(),
                }),
                status: Punktstatus::Offen,
                antwort: String::new(),
                dokumente: vec![],
                abgaben: vec![],
                bestaetigung: None,
            })
            .collect(),
    });
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    assert_eq!(
        core.verfahren_erinnerungen(p.id, heute)
            .await
            .unwrap()
            .len(),
        3
    );
    let plan2 = core
        .store_document(
            p.id,
            Some(plan.document_id),
            Some(1),
            "Lageplan.pdf",
            Bytes::from(pdf("Lageplan v2 korrigiert")),
            &user,
        )
        .await
        .unwrap();
    let ref2 = Dokumentbezug {
        dokument_id: plan.document_id,
        version: 2,
    };
    let w = Abgabewunsch {
        xbau_bezug: None,
        id: Uuid::new_v4(),
        titel: "Nachreichung zwei Punkte".into(),
        dokumente: vec![ref2.clone()],
        xbau: false,
        druck: true,
        anschreiben: "Die korrigierten Unterlagen zum Bauantrag.".into(),
    };
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, w.clone(), &user)
        .await
        .unwrap();
    let id = w.id;
    let zip_vorher = core
        .verfahren_artefakt(p.id, vid, id, "zip")
        .await
        .unwrap()
        .1;
    let druck = core
        .verfahren_artefakt(p.id, vid, id, "pdf")
        .await
        .unwrap()
        .1;
    let retry = core
        .verfahren_abgabe_festhalten(p.id, vid, 0, w.clone(), &user)
        .await
        .unwrap();
    assert_eq!(retry.revision, f.revision);
    let mut anders = w;
    anders.druck = false;
    assert!(
        core.verfahren_abgabe_festhalten(p.id, vid, f.revision, anders, &user)
            .await
            .is_err()
    );
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    a.abgaben[0].versand.push(Versandereignis {
        id: Uuid::new_v4(),
        status: Versandstatus::Zurueckgewiesen,
        datum: heute,
        weg: "Bauportal".into(),
        referenz: "Versuch 1".into(),
        notiz: "Upload abgewiesen".into(),
        beleg: Some(brief.clone()),
        berichtigt: None,
    });
    a.nachforderungen[0].punkte[0].status = Punktstatus::Eingereicht;
    a.nachforderungen[0].punkte[0].abgaben = vec![id];
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&a).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    a.nachforderungen[0].punkte[0].status = Punktstatus::Vorbereitet;
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    a = serde_json::from_value(f.daten).unwrap();
    a.abgaben[0].versand.push(Versandereignis {
        id: Uuid::new_v4(),
        status: Versandstatus::Eingegangen,
        datum: heute,
        weg: "Bauportal".into(),
        referenz: "Erfolgreicher Versuch 2".into(),
        notiz: "Eingang nachgewiesen".into(),
        beleg: Some(brief.clone()),
        berichtigt: None,
    });
    for punkt in &mut a.nachforderungen[0].punkte[..2] {
        punkt.status = Punktstatus::Eingereicht;
        punkt.dokumente = vec![ref2.clone()];
        punkt.abgaben = vec![id];
    }
    a.nachforderungen[0].punkte[2].frist.as_mut().unwrap().datum =
        heute + chrono::Duration::days(14);
    a.nachforderungen[0].punkte[2].frist.as_mut().unwrap().grund =
        "Fristverlängerung mit Schreiben".into();
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    // Korrektur eines Versanddatums ist ein neuer Eintrag, nie Änderung des alten.
    a = serde_json::from_value(f.daten.clone()).unwrap();
    let e = a.abgaben[0].versand[0].id;
    a.abgaben[0].versand.push(Versandereignis {
        id: Uuid::new_v4(),
        status: Versandstatus::Zurueckgewiesen,
        datum: heute,
        weg: "Bauportal".into(),
        referenz: "Versuch 1 berichtigt".into(),
        notiz: "Historischen Eintrag berichtigt".into(),
        beleg: None,
        berichtigt: Some(e),
    });
    // Ein später nacherfasster historischer Fehlschlag darf den neueren Eingang nicht aufheben.
    assert_eq!(
        versandstand(&a.abgaben[0]),
        Some(Versandstatus::Eingegangen)
    );
    let besch = Uuid::new_v4();
    a.bescheide.push(Bescheid {
        id: besch,
        entscheidung: Entscheidung::Genehmigt,
        datum: heute,
        zugestellt: Some(heute),
        original: brief.clone(),
        genehmigte_unterlagen: vec![ref2.clone()],
        auflagen: vec![Auflage {
            id: Uuid::new_v4(),
            titel: "Nachweis vor Baubeginn".into(),
            ausloeser: "Vor Baubeginn".into(),
            verantwortlich: Some(user.id.0),
            frist: Some(Frist {
                datum: heute,
                ausloeser: "Bescheid".into(),
                beleg: Some(brief.clone()),
                erinnerung_tage: 7,
                grund: "Terminüberwachung".into(),
            }),
            erledigt: false,
            nachweise: vec![],
        }],
        ersetzt: None,
    });
    a.status = Verfahrensstatus::Entschieden;
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    let plan3 = core
        .store_document(
            p.id,
            Some(plan.document_id),
            Some(2),
            "Lageplan.pdf",
            Bytes::from(pdf("Lageplan v3 neuer Entwurf")),
            &user,
        )
        .await
        .unwrap();
    assert_eq!(plan3.version, 3);
    assert_eq!(
        core.verfahren_artefakt(p.id, vid, id, "zip")
            .await
            .unwrap()
            .1,
        zip_vorher
    );
    assert_eq!(
        core.verfahren_artefakt(p.id, vid, id, "pdf")
            .await
            .unwrap()
            .1,
        druck
    );
    let (_, a) = core.verfahrensakte(p.id, vid).await.unwrap();
    assert_eq!(a.bescheide[0].genehmigte_unterlagen[0], ref2);
    assert_eq!(a.nachforderungen[0].punkte[2].status, Punktstatus::Offen);
    core.suchtext_speichern(
        p.id,
        ref2.clone(),
        Textquelle::Pdf,
        "Korrigierter Lageplan zur Nachforderung".into(),
        &user,
    )
    .await
    .unwrap();
    let treffer = core.verfahren_suche(p.id, "Nachforderung").await.unwrap();
    assert!(treffer.iter().any(|t| t.dokument == Some(ref2.clone())));
    let export = core.verfahren_aktenexport(p.id).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&export)).unwrap();
    let mut manifest = String::new();
    zip.by_name("Projektakte.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(manifest.contains("BA-2026-42"));
    assert!(
        zip.by_name(&format!("Dokumente/{}/v1-Lageplan.pdf", plan.document_id.0))
            .is_ok()
    );
    assert!(
        zip.by_name(&format!("Dokumente/{}/v3-Lageplan.pdf", plan.document_id.0))
            .is_ok()
    );
    let mut manipuliert = a.clone();
    manipuliert.abgaben[0].titel = "Geändert".into();
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(manipuliert).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    let mut geloescht = a.clone();
    geloescht.nachforderungen.clear();
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(geloescht).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    assert!(matches!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            0,
            serde_json::to_value(&a).unwrap(),
            &user
        )
        .await,
        Err(CoreError::VersionConflict { .. })
    ));
    // Projektgrenzen und Schreibrechte gelten auch bei gültigen Dokumentkennungen.
    let fremd = core
        .create_project(
            NewProject {
                name: "Andere Bauakte".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let fremddokument = core
        .store_document(
            fremd.id,
            None,
            None,
            "FremderPlan.pdf",
            Bytes::from(pdf("Fremd")),
            &user,
        )
        .await
        .unwrap();
    let mut ungueltig = a.clone();
    ungueltig.anforderungen[0].dokumente.push(Dokumentbezug {
        dokument_id: fremddokument.document_id,
        version: 1,
    });
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(ungueltig).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    let leser = core
        .benutzer_anlegen(
            core_lib::benutzer::NeuerBenutzer {
                anmeldename: "leser".into(),
                anzeigename: "Leser".into(),
                rolle: core_lib::benutzer::Rolle::Lesen,
            },
            &user,
        )
        .await
        .unwrap();
    assert!(matches!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&a).unwrap(),
            &leser
        )
        .await,
        Err(CoreError::Forbidden(_))
    ));
    // Zwei Bearbeiter auf demselben Stand: genau ein fachlicher Commit gewinnt.
    let zweites = Uuid::new_v4();
    let mut links = leer();
    speichern(&core, p.id, zweites, 0, &links, &user).await;
    links = core.verfahrensakte(p.id, zweites).await.unwrap().1;
    let mut rechts = links.clone();
    links.aktenzeichen = "A".into();
    rechts.aktenzeichen = "B".into();
    let (l, r) = tokio::join!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            zweites,
            1,
            serde_json::to_value(links).unwrap(),
            &user
        ),
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            zweites,
            1,
            serde_json::to_value(rechts).unwrap(),
            &user
        )
    );
    assert_ne!(l.is_ok(), r.is_ok());
    let (_, mut folge) = core.verfahrensakte(p.id, zweites).await.unwrap();
    folge.vorgaenger = Some(vid);
    let folgestand = speichern(&core, p.id, zweites, 2, &folge, &user).await;
    let mut kreis = a.clone();
    kreis.vorgaenger = Some(zweites);
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(kreis).unwrap(),
            &user
        )
        .await
        .is_err()
    );
    assert_eq!(folgestand.revision, 3);
    let uebersicht = core.projektuebersicht(p.id, &user).await.unwrap();
    let akte = uebersicht.verfahren.iter().find(|v| v.id == vid).unwrap();
    assert_eq!(akte.entscheidung, Some(Entscheidung::Genehmigt));
    assert_eq!(akte.offene_auflagen, 1);
    assert_eq!(akte.offene_punkte, 3); // Zwei Antworten warten auf Bestätigung, ein Punkt ist wieder offen.
    assert!(uebersicht.punkte.iter().any(
        |p| p.ziel.objekt == Some(vid) && p.ziel.register.as_deref() == Some("nachforderungen")
    ));
    assert!(
        uebersicht
            .empfaenger
            .iter()
            .any(|e| e.dokumente.iter().any(|d| d.eingereichte_version == 2))
    );
    for p in a.nachforderungen[0].punkte.iter().take(2) {
        let wartend = uebersicht
            .punkte
            .iter()
            .find(|h| h.ziel.eintrag == Some(p.id) && h.ziel.objekt == Some(vid))
            .unwrap();
        assert_eq!(
            wartend.dringlichkeit,
            core_lib::projektuebersicht::Dringlichkeit::Warten
        );
        assert!(
            wartend.faellig.is_none(),
            "Eingereichte Antwort ist keine unbeantwortete überfällige Nachforderung"
        );
    }
    if let Ok(ziel) = std::env::var("BAUPROJEKT_VERFAHREN_ARTEFAKTE") {
        let ziel = std::path::Path::new(&ziel);
        std::fs::create_dir_all(ziel).unwrap();
        std::fs::write(ziel.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"verfahren":f,"akte":a,"dateien":core.list_documents(p.id).await.unwrap(),"benutzer":user,"erinnerungen":core.verfahren_erinnerungen(p.id,heute).await.unwrap(),"treffer":treffer,"uebersicht":uebersicht})).unwrap()).unwrap();
        std::fs::write(ziel.join("akte.zip"), export).unwrap();
    }
    drop(core);
    drop(db);
    let db = db_sqlite::SqliteDatabase::open(&dbpfad).await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    assert_eq!(
        core.verfahrensakte(p.id, vid).await.unwrap().1.bescheide[0].genehmigte_unterlagen[0],
        ref2
    );
    let _ = ref1;
    let _ = plan2;
}

#[tokio::test]
async fn profil_verlangt_zustimmung_des_eigentuemers_der_nicht_bauherr_ist() {
    use core_lib::kunden::{Beteiligtenrolle, Beteiligtenzuordnung, Kundendaten};
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Eigentümerzustimmung".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let kunde = |vorname: &str| Kundendaten {
        vorname: vorname.into(),
        familienname: "Huber".into(),
        ..Kundendaten::default()
    };
    let bauherr = core.kunde_anlegen(kunde("Berta"), &user).await.unwrap();
    let eigentuemer = core.kunde_anlegen(kunde("Emil"), &user).await.unwrap();
    let p = core
        .bauherren_setzen(p.id, vec![bauherr.id], p.version, &user)
        .await
        .unwrap();
    core.beteiligte_setzen(
        p.id,
        vec![Beteiligtenzuordnung {
            rolle: Beteiligtenrolle::Grundstueckseigentuemer,
            kunde: eigentuemer.id,
        }],
        p.version,
        &user,
    )
    .await
    .unwrap();
    let vid = Uuid::new_v4();
    speichern(&core, p.id, vid, 0, &leer(), &user).await;
    let profil = Vorhabenprofil {
        bundesland: Bundesland::By,
        art: Verfahrensart::Bauantrag,
        gebaeudeklasse: Some(1),
        sonderbau: Some(false),
        nutzung: "Wohnen".into(),
    };
    let f = core
        .verfahren_profil_anwenden(
            p.id,
            vid,
            1,
            profil.clone(),
            Anforderungskatalog::standard().unwrap(),
            &user,
        )
        .await
        .unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let z: Vec<_> = a
        .anforderungen
        .iter()
        .filter(|r| r.titel.starts_with("Zustimmungserklärung"))
        .collect();
    assert_eq!(z.len(), 1);
    assert_eq!(z[0].titel, "Zustimmungserklärung von Emil Huber");
    assert_eq!(z[0].erforderlich, Erforderlichkeit::Ungeklaert);

    // Erneutes Anwenden legt die Anforderung nicht doppelt an.
    let f = core
        .verfahren_profil_anwenden(
            p.id,
            vid,
            f.revision,
            profil,
            Anforderungskatalog::standard().unwrap(),
            &user,
        )
        .await
        .unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    assert_eq!(
        a.anforderungen
            .iter()
            .filter(|r| r.titel.starts_with("Zustimmungserklärung"))
            .count(),
        1
    );

    // Ändern sich die Beteiligten, hält jede Speicherung der Akte die
    // Zustimmungspflicht aktuell, auch ohne erneutes Profil.
    let frieda = core.kunde_anlegen(kunde("Frieda"), &user).await.unwrap();
    let p = core.get_project(p.id).await.unwrap();
    let p = core
        .beteiligte_setzen(
            p.id,
            vec![
                Beteiligtenzuordnung {
                    rolle: Beteiligtenrolle::Grundstueckseigentuemer,
                    kunde: eigentuemer.id,
                },
                Beteiligtenzuordnung {
                    rolle: Beteiligtenrolle::Erbbauberechtigter,
                    kunde: frieda.id,
                },
            ],
            p.version,
            &user,
        )
        .await
        .unwrap();
    let mut a = a;
    a.bezeichnung = "Bauantrag mit Erbbaurecht".into();
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let titel: Vec<_> = a
        .anforderungen
        .iter()
        .filter(|r| r.titel.starts_with("Zustimmungserklärung"))
        .map(|r| r.titel.as_str())
        .collect();
    assert_eq!(
        titel,
        [
            "Zustimmungserklärung von Emil Huber",
            "Zustimmungserklärung von Frieda Huber"
        ]
    );

    // Ein Flurstück mit Rolle „Sonstiges“ verlangt eine Baulastklärung.
    let flurstuecke = |rolle| {
        let fl =
            |id: &str, nummer: &str, rolle| core_lib::grundstuecksinformationen::FlurstuecksInfo {
                id: id.into(),
                gemarkung: "Schwabing".into(),
                gemarkungsnummer: String::new(),
                flur: String::new(),
                nummer: nummer.into(),
                flaeche_m2: 500.,
                grundbuchbezirk: String::new(),
                grundbuchblatt: String::new(),
                laufende_nummer: None,
                grundbuchauszug: None,
                eigentuemer: String::new(),
                eigentuemer_kunden: vec![],
                bemerkung: String::new(),
                rolle,
            };
        core_lib::domain::Planungsrecht {
            grundstuecksinformationen: Some(
                core_lib::grundstuecksinformationen::Grundstuecksinformationen {
                    flurstuecke: vec![
                        fl(
                            "a",
                            "12",
                            core_lib::grundstuecksinformationen::FlurstuecksRolle::Vorhaben,
                        ),
                        fl("w", "13/1", rolle),
                    ],
                },
            ),
            ..Default::default()
        }
    };
    let projekt_setzen = |p: core_lib::domain::Project, recht| {
        core.update_project(
            core_lib::domain::ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name.clone(),
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: None,
                planungsrecht: Some(recht),
            },
            &user,
        )
    };
    let p = projekt_setzen(
        p,
        flurstuecke(core_lib::grundstuecksinformationen::FlurstuecksRolle::Sonstiges),
    )
    .await
    .unwrap();
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let baulast: Vec<_> = a
        .anforderungen
        .iter()
        .filter(|r| r.quelle == "Flurstücke")
        .collect();
    assert_eq!(baulast.len(), 1);
    assert_eq!(
        baulast[0].titel,
        "Baulast für Flurstück Schwabing 13/1 klären"
    );
    assert_eq!(baulast[0].erforderlich, Erforderlichkeit::Ungeklaert);
    let p = projekt_setzen(
        p,
        flurstuecke(core_lib::grundstuecksinformationen::FlurstuecksRolle::Nachbar),
    )
    .await
    .unwrap();

    // Wird der Eigentümer Bauherr, bleibt der Akteneintrag; die Abgabe weist darauf
    // hin und friert die Beteiligten ein.
    core.bauherren_setzen(p.id, vec![bauherr.id, eigentuemer.id], p.version, &user)
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from_static(b"%PDF-1.7 Lageplan"),
            &user,
        )
        .await
        .unwrap();
    let w = core_lib::verfahren::abgabe::Abgabewunsch {
        xbau_bezug: None,
        id: Uuid::new_v4(),
        titel: "Antragspaket".into(),
        dokumente: vec![core_lib::plattform::objekte::Dokumentbezug {
            dokument_id: plan.document_id,
            version: 1,
        }],
        xbau: false,
        druck: false,
        anschreiben: String::new(),
    };
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, w.clone(), &user)
        .await
        .unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let stand = a.abgaben.iter().find(|b| b.id == w.id).unwrap();
    assert!(
        stand.hinweise.iter().any(|h| h
            == "Zustimmungserklärung von Emil Huber: nach den aktuellen Projektbeteiligten nicht mehr erforderlich. Anforderung prüfen."),
        "{:?}",
        stand.hinweise
    );
    assert!(
        stand.hinweise.iter().any(|h| h
            == "Baulast für Flurstück Schwabing 13/1 klären: nach den aktuellen Flurstücken nicht mehr erforderlich. Anforderung prüfen."),
        "{:?}",
        stand.hinweise
    );
    let beteiligte: Vec<_> = stand
        .grundlagen
        .beteiligte
        .iter()
        .map(|b| (b.rolle, b.kunde.anzeigename()))
        .collect();
    assert_eq!(
        beteiligte,
        [
            (
                Beteiligtenrolle::Grundstueckseigentuemer,
                "Emil Huber".to_string()
            ),
            (
                Beteiligtenrolle::Erbbauberechtigter,
                "Frieda Huber".to_string()
            )
        ]
    );
}
