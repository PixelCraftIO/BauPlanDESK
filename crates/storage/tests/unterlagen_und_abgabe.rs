//! Eine Unterlagenliste: Nachweisstände zählen als Unterlage einer
//! Anforderung, passende Projektdateien werden vorgeschlagen, das
//! Unterlagenregister meldet nichts doppelt und die Planliste kennt die bei
//! der Behörde eingereichten Pläne.
use bytes::Bytes;
use chrono::NaiveDate;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject, Planungsrecht},
    lbo::{AbstandsflaecheInput, LboEngine},
    license::DevelopmentLicense,
    plattform::{ObjektArt, nachweise::NachweisEingabe},
    verfahren::{
        Erforderlichkeit, Verfahrensakte, Verfahrensart,
        externe_einreichung::{Einreichungsweg, ExterneEinreichungswunsch},
    },
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn core() -> (tempfile::TempDir, CoreServices) {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let lbo = Arc::new(LboEngine::new());
    lbo.load_directory(std::path::Path::new("../../lbo-rules"))
        .unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        lbo,
        Arc::new(DevelopmentLicense),
    );
    (dir, c)
}

fn pdf(text: &str) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut d = Document::with_version("1.5");
    let pages = d.new_object_id();
    let font =
        d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content = d.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 12 Tf 50 700 Td ({text}) Tj ET").into_bytes(),
    ));
    let page = d.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],"Contents"=>content,"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}});
    d.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}),
    );
    let catalog = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    d.trailer.set("Root", catalog);
    let mut out = Vec::new();
    d.save_to(&mut out).unwrap();
    out
}

#[tokio::test]
async fn nachweis_wird_unterlage_und_listen_melden_einmal() {
    let (_dir, c) = core().await;
    let ich = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Neubau Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht::default(),
            },
            &ich,
        )
        .await
        .unwrap();
    let tag = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
    let stand = c
        .verfahren_vorbereiten(p.id, Verfahrensart::Bauantrag, Some(true), &ich)
        .await
        .unwrap();
    let vid: Uuid = stand.objekt.parse().unwrap();
    let akte: Verfahrensakte = serde_json::from_value(stand.daten).unwrap();
    let abstand = akte
        .anforderungen
        .iter()
        .find(|a| a.nachweise == ["abstand"])
        .expect("Anforderung Abstandsflächen")
        .clone();
    assert_eq!(abstand.erforderlich, Erforderlichkeit::Erforderlich);
    let lageplan = akte
        .anforderungen
        .iter()
        .find(|a| a.titel == "Lageplan")
        .expect("Anforderung Lageplan")
        .clone();
    assert_eq!(lageplan.kategorien, ["lageplan"]);
    let hinweis = |u: &core_lib::projektuebersicht::Projektuebersicht, id: Uuid| {
        u.punkte
            .iter()
            .find(|x| x.id == format!("anforderung:{vid}:{id}"))
            .map(|x| x.hinweis.clone())
            .unwrap_or_default()
    };

    // Ohne Nachweis sagt die Übersicht, was fehlt; übernehmen geht noch nicht.
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    assert!(
        hinweis(&u, abstand.id).contains("noch kein Nachweis erstellt"),
        "{}",
        hinweis(&u, abstand.id)
    );
    let err = c
        .verfahren_nachweis_als_unterlage(p.id, vid, stand.revision, abstand.id, &ich)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("noch kein Nachweis"), "{err}");

    // Mit Nachweis: ein Schritt legt ihn als Unterlage ab und ordnet ihn zu.
    let wand = |gegenstand: &str| NachweisEingabe::Abstand {
        gegenstand: gegenstand.into(),
        eingabe: AbstandsflaecheInput {
            wandhoehe_m: 6.5,
            gebiet: core_lib::calc::Baugebiet::WA,
        },
    };
    c.nachweis_erstellen(
        p.id,
        Uuid::new_v4(),
        "Abstandsflächen Nordwand".into(),
        wand("Nordwand"),
        &ich,
    )
    .await
    .unwrap();
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    assert!(
        hinweis(&u, abstand.id).contains("Der Nachweis liegt vor"),
        "{}",
        hinweis(&u, abstand.id)
    );
    let f = c
        .verfahren_nachweis_als_unterlage(p.id, vid, stand.revision, abstand.id, &ich)
        .await
        .unwrap();
    let akte: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let r = akte
        .anforderungen
        .iter()
        .find(|a| a.id == abstand.id)
        .unwrap();
    let unterlage = r.nachweisunterlage.clone().expect("Nachweisunterlage");
    assert_eq!(r.dokumente, std::slice::from_ref(&unterlage.dokument));
    assert_eq!(unterlage.staende.len(), 1);
    assert!(!r.freigegeben);
    let dateien = c.list_documents(p.id).await.unwrap();
    let datei = dateien
        .iter()
        .find(|d| d.document_id == unterlage.dokument.dokument_id)
        .expect("PDF in den Projektdateien");
    assert_eq!(datei.version, 1);
    assert!(datei.file_name.ends_with(".pdf"), "{}", datei.file_name);
    assert!(
        c.dokumentkategorien(p.id)
            .await
            .unwrap()
            .iter()
            .any(|k| k.dokument_id == datei.document_id
                && k.zuordnung.kategorie.as_deref() == Some("berechnung"))
    );
    // Unverändert wiederholt: kein neuer Stand.
    let gleich = c
        .verfahren_nachweis_als_unterlage(p.id, vid, f.revision, abstand.id, &ich)
        .await
        .unwrap();
    assert_eq!(gleich.revision, f.revision);

    // Neuer Nachweisstand: Die Übersicht meldet es, derselbe Schritt schreibt
    // dasselbe Dokument als Version 2 fort.
    c.nachweis_erstellen(
        p.id,
        Uuid::new_v4(),
        "Abstandsflächen Südwand".into(),
        wand("Südwand"),
        &ich,
    )
    .await
    .unwrap();
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    assert!(
        hinweis(&u, abstand.id).contains("neu erstellt"),
        "{}",
        hinweis(&u, abstand.id)
    );
    let f = c
        .verfahren_nachweis_als_unterlage(p.id, vid, f.revision, abstand.id, &ich)
        .await
        .unwrap();
    let akte: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let r = akte
        .anforderungen
        .iter()
        .find(|a| a.id == abstand.id)
        .unwrap();
    let neu = r.nachweisunterlage.clone().unwrap();
    assert_eq!(neu.dokument.dokument_id, unterlage.dokument.dokument_id);
    assert_eq!(neu.dokument.version, 2);
    assert_eq!(neu.staende.len(), 2);
    assert_eq!(r.dokumente, std::slice::from_ref(&neu.dokument));

    // Dateivorschlag: Eine später abgelegte Datei passender Kategorie nennt
    // die Übersicht bei der Anforderung.
    let plan = c
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(pdf("Lageplan")),
            &ich,
        )
        .await
        .unwrap();
    c.dokumentkategorie_speichern(
        plan.document_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 0,
            kategorie: Some("lageplan".into()),
        },
        &ich,
    )
    .await
    .unwrap();
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    assert!(
        hinweis(&u, lageplan.id).contains("1 Projektdatei passender Kategorie"),
        "{}",
        hinweis(&u, lageplan.id)
    );

    // Das Unterlagenregister meldet nicht, was die Akte schon als Anforderung führt.
    for name in ["Lageplan", "Bodengutachten"] {
        c.fachobjekt_speichern(
            p.id,
            ObjektArt::Unterlagen,
            Uuid::new_v4(),
            0,
            json!({"bezeichnung":name,"erforderlich":true,"dokument":null,"status":"entwurf","bemerkung":"","vorlage":null}),
            &ich,
        )
        .await
        .unwrap();
    }
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    let register: Vec<_> = u
        .punkte
        .iter()
        .filter(|x| x.id.starts_with("unterlage:"))
        .map(|x| x.titel.clone())
        .collect();
    assert_eq!(register, ["Bodengutachten"]);

    // Planliste: Ein extern eingereichter Lageplan trägt einen Behördenindex
    // und erscheint als Plan ohne Eintrag, bis er aufgenommen ist.
    assert!(
        c.planliste(p.id, &ich)
            .await
            .unwrap()
            .ohne_eintrag
            .is_empty()
    );
    let (f, mut akte) = c.verfahrensakte(p.id, vid).await.unwrap();
    akte.behoerde.name = "Bauaufsicht Musterstadt".into();
    let f = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&akte).unwrap(),
            &ich,
        )
        .await
        .unwrap();
    let bezug = core_lib::plattform::objekte::Dokumentbezug {
        dokument_id: plan.document_id,
        version: plan.version,
    };
    c.verfahren_externe_einreichung(
        p.id,
        vid,
        f.revision,
        ExterneEinreichungswunsch {
            id: Uuid::new_v4(),
            datum: core_lib::kalender::heute(),
            weg: Einreichungsweg::Portal,
            beleg: None,
            bemerkung: "Über das Bauportal eingereicht".into(),
            dokumente: vec![bezug],
            berichtigt: None,
        },
        &ich,
    )
    .await
    .unwrap();
    let liste = c.planliste(p.id, &ich).await.unwrap();
    assert_eq!(liste.ohne_eintrag.len(), 1, "{:?}", liste.ohne_eintrag);
    assert_eq!(liste.ohne_eintrag[0].dokument_id, plan.document_id);
    assert_eq!(
        liste.ohne_eintrag[0].behoerde[0].planindex.as_deref(),
        Some("A")
    );
}

/// Eine Nachforderung ist ein Vorgang: Der Forderungspunkt führt die
/// betroffene Anforderung und seine Aufgaben mit, statt dass alle drei
/// einzeln als offene Punkte und Fristen erscheinen.
#[tokio::test]
async fn nachforderung_ist_ein_vorgang() {
    use core_lib::verfahren::{
        Forderungspunkt, Frist, Kommunikation, Kommunikationsart, Nachforderung, Punktstatus,
    };
    let (_dir, c) = core().await;
    let ich = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Neubau Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht::default(),
            },
            &ich,
        )
        .await
        .unwrap();
    let tag = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
    let frist = NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
    let stand = c
        .verfahren_vorbereiten(p.id, Verfahrensart::Bauantrag, Some(true), &ich)
        .await
        .unwrap();
    let vid: Uuid = stand.objekt.parse().unwrap();
    let mut akte: Verfahrensakte = serde_json::from_value(stand.daten).unwrap();
    let lageplan = akte
        .anforderungen
        .iter()
        .find(|a| a.titel == "Lageplan")
        .unwrap()
        .id;
    let (kommunikation, punkt) = (Uuid::new_v4(), Uuid::new_v4());
    akte.kommunikation.push(Kommunikation {
        id: kommunikation,
        art: Kommunikationsart::Brief,
        eingehend: true,
        datum: tag,
        betreff: "Nachforderung zum Bauantrag".into(),
        beteiligte: "Bauaufsicht".into(),
        inhalt: "Bitte den Lageplan nachreichen.".into(),
        original: None,
        bezuege: vec![],
    });
    akte.nachforderungen.push(Nachforderung {
        id: Uuid::new_v4(),
        kommunikation,
        punkte: vec![Forderungspunkt {
            id: punkt,
            titel: "Lageplan nachreichen".into(),
            grundlage: "Schreiben vom 11.11.2026, Punkt 1".into(),
            verantwortlich: Some(ich.id.0),
            frist: Some(Frist {
                datum: frist,
                ausloeser: "Schreiben".into(),
                beleg: None,
                erinnerung_tage: 7,
                grund: "Fristsetzung der Behörde".into(),
            }),
            status: Punktstatus::Offen,
            antwort: String::new(),
            dokumente: vec![],
            abgaben: vec![],
            bestaetigung: None,
            anforderung: Some(lageplan),
        }],
    });
    let f = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            stand.revision,
            serde_json::to_value(&akte).unwrap(),
            &ich,
        )
        .await
        .unwrap();
    let aufgabe = Uuid::new_v4();
    c.fachobjekt_speichern(
        p.id,
        ObjektArt::Aufgaben,
        aufgabe,
        0,
        json!({"titel":"Vermesser beauftragen","beschreibung":"","faellig":frist,
               "verantwortlich":null,"status":"offen","bezug":punkt}),
        &ich,
    )
    .await
    .unwrap();

    // Übersicht: ein Punkt für den Vorgang, Anforderung und Aufgabe stehen darin.
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    let vorgang = u
        .punkte
        .iter()
        .find(|x| x.ziel.eintrag == Some(punkt))
        .expect("Forderungspunkt in der Übersicht");
    assert!(
        vorgang.hinweis.contains("Vermesser beauftragen"),
        "{}",
        vorgang.hinweis
    );
    assert!(
        !u.punkte
            .iter()
            .any(|x| x.id == format!("anforderung:{vid}:{lageplan}")),
        "nachgeforderte Anforderung darf nicht zusätzlich erscheinen"
    );
    assert!(
        !u.punkte.iter().any(|x| x.titel == "Vermesser beauftragen"),
        "gebundene Aufgabe darf nicht zusätzlich erscheinen"
    );
    // Cockpit: eine Frist für den Vorgang statt zweier.
    let cockpit = c.buero_cockpit(&ich, tag).await.unwrap();
    let am_tag: Vec<_> = cockpit
        .termine
        .iter()
        .filter(|t| t.datum == frist)
        .map(|t| t.titel.clone())
        .collect();
    assert_eq!(am_tag.len(), 1, "{am_tag:?}");

    // Antwortunterlagen des Punkts belegen die Anforderung.
    let plan = c
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(pdf("Lageplan")),
            &ich,
        )
        .await
        .unwrap();
    let bezug = core_lib::plattform::objekte::Dokumentbezug {
        dokument_id: plan.document_id,
        version: plan.version,
    };
    let mut akte: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    akte.nachforderungen[0].punkte[0].dokumente = vec![bezug.clone()];
    akte.nachforderungen[0].punkte[0].status = Punktstatus::InArbeit;
    let f = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&akte).unwrap(),
            &ich,
        )
        .await
        .unwrap();
    let mut akte: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let r = akte
        .anforderungen
        .iter()
        .find(|a| a.id == lageplan)
        .unwrap();
    assert_eq!(r.dokumente, [bezug]);
    assert_eq!(r.erforderlich, Erforderlichkeit::Erforderlich);
    let offen = c
        .fachobjekt_lesen(p.id, ObjektArt::Aufgaben, aufgabe)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(offen.daten["status"], "offen");

    // Ist der Punkt erledigt, sind es seine Aufgaben auch; die Anforderung
    // steht danach wieder für sich (hier: Freigabe offen).
    akte.nachforderungen[0].punkte[0].status = Punktstatus::Entfallen;
    akte.nachforderungen[0].punkte[0].antwort =
        "Behörde verzichtet laut Telefonat auf den Punkt.".into();
    c.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid,
        f.revision,
        serde_json::to_value(&akte).unwrap(),
        &ich,
    )
    .await
    .unwrap();
    let erledigt = c
        .fachobjekt_lesen(p.id, ObjektArt::Aufgaben, aufgabe)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(erledigt.daten["status"], "erledigt");
    let u = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    assert!(!u.punkte.iter().any(|x| x.ziel.eintrag == Some(punkt)));
    assert!(
        u.punkte
            .iter()
            .any(|x| x.id == format!("anforderung:{vid}:{lageplan}"))
    );
}
