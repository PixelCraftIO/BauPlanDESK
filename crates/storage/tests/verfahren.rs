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
        bezeichnung: "Neubau Musterstraße".into(),
        art: Verfahrensart::Bauantrag,
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
    if let Ok(ziel) = std::env::var("BAUPROJEKT_VERFAHREN_ARTEFAKTE") {
        let ziel = std::path::Path::new(&ziel);
        std::fs::create_dir_all(ziel).unwrap();
        std::fs::write(ziel.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"verfahren":f,"akte":a,"dateien":core.list_documents(p.id).await.unwrap(),"benutzer":user,"erinnerungen":core.verfahren_erinnerungen(p.id,heute).await.unwrap(),"treffer":treffer})).unwrap()).unwrap();
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
