//! Durchgehendes Musterverfahren, referenzielle Grenzen und unveränderliche Abgaben.
use bytes::Bytes;
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
        vollstaendigkeit: None,
        ruecknahme: None,
        ausfuehrung: None,
        schritte: Vec::new(),
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
        externe_einreichungen: vec![],
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
    let heute = core_lib::kalender::heute();
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
    // Ohne versendeten Abgabestand ist das Verfahren nicht eingereicht.
    let mut versuch = a.clone();
    versuch.status = Verfahrensstatus::Unvollstaendig;
    let err = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&versuch).unwrap(),
            &user,
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("versendeten Abgabestand"), "{err}");
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
        .verfahren_artefakt(p.id, vid, id, "zip", &user)
        .await
        .unwrap()
        .1;
    let druck = core
        .verfahren_artefakt(p.id, vid, id, "pdf", &user)
        .await
        .unwrap()
        .1;
    // Behörde und Aktenzeichen der Akte sind eingefroren und gehen in Druck und XBau.
    let akte: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let g = &akte.abgaben.iter().find(|b| b.id == id).unwrap().grundlagen;
    assert_eq!(g.aktenzeichen.as_deref(), Some("BA-2026-42"));
    assert_eq!(
        g.behoerde.as_ref().map(|b| b.kennung.as_str()),
        Some("bab:test")
    );
    assert_eq!(g.antrag.bauaufsicht.kennung, "bab:test");
    // Ältere Abgabestände ohne diese Angaben bleiben lesbar.
    let mut alt = serde_json::to_value(g).unwrap();
    alt.as_object_mut().unwrap().remove("behoerde");
    alt.as_object_mut().unwrap().remove("aktenzeichen");
    let alt: core_lib::verfahren::abgabe::Abgabegrundlagen = serde_json::from_value(alt).unwrap();
    assert_eq!((alt.behoerde, alt.aktenzeichen), (None, None));
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
    let ablehnen = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        let pid = p.id;
        async move {
            core.fachobjekt_speichern(pid, ObjektArt::Verfahren, vid, revision, a, user)
                .await
                .unwrap_err()
                .to_string()
        }
    };
    // Nach dem Versand: unvollständig, dann vollständig nur mit Bestätigung.
    a.status = Verfahrensstatus::Unvollstaendig;
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    a = serde_json::from_value(f.daten.clone()).unwrap();
    a.status = Verfahrensstatus::Vollstaendig;
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("behördliche Bestätigung"), "{err}");
    a.vollstaendigkeit = Some(Vollstaendigkeit {
        datum: heute,
        kommunikation: Uuid::new_v4(),
    });
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("eingegangenes Schreiben"), "{err}");
    a.vollstaendigkeit = Some(Vollstaendigkeit {
        datum: heute,
        kommunikation,
    });
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    a = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Unvollständig → Vollständig")
    );
    a.status = Verfahrensstatus::Vorbereitung;
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("nicht in die Vorbereitung"), "{err}");
    a.status = Verfahrensstatus::Vollstaendig;
    // Vor der Entscheidung warten eingereichte Antworten auf Bestätigung.
    let vorher = core.projektuebersicht(p.id, &user).await.unwrap();
    assert_eq!(vorher.verfahren[0].offene_punkte, 3);
    for p in a.nachforderungen[0].punkte.iter().take(2) {
        let wartend = vorher
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
    // Genehmigt wird nur, was der Behörde mit einem versendeten Stand vorlag.
    let mut versuch = a.clone();
    versuch.bescheide.push(Bescheid {
        id: Uuid::new_v4(),
        entscheidung: Entscheidung::Genehmigt,
        datum: heute,
        zugestellt: None,
        original: brief.clone(),
        genehmigte_unterlagen: vec![ref1.clone()],
        auflagen: vec![],
        ersetzt: None,
        verlaengerung: None,
    });
    let err = ablehnen(&versuch, f.revision).await;
    assert!(err.contains("aus einem versendeten Abgabestand"), "{err}");
    let besch = Uuid::new_v4();
    a.bescheide.push(Bescheid {
        id: besch,
        entscheidung: Entscheidung::Genehmigt,
        datum: heute,
        zugestellt: None,
        original: brief.clone(),
        genehmigte_unterlagen: vec![ref2.clone()],
        auflagen: vec![Auflage {
            id: Uuid::new_v4(),
            art: Nebenbestimmungsart::Auflage,
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
        verlaengerung: None,
    });
    a.status = Verfahrensstatus::Entschieden;
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    // Das Zustelldatum lässt sich einmal nachtragen, danach nicht mehr ändern.
    a = serde_json::from_value(f.daten.clone()).unwrap();
    a.bescheide[0].zugestellt = Some(heute);
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    a = serde_json::from_value(f.daten.clone()).unwrap();
    assert_eq!(a.bescheide[0].zugestellt, Some(heute));
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Zustellung des Bescheids")
    );
    // Ein erfasstes Zustelldatum ändert sich nur mit begründeter Berichtigung
    // und wird nicht entfernt (siehe berliner_fristen_berichtigung_baubeginn_und_entscheidungsregeln).
    a.bescheide[0].zugestellt = Some(heute + chrono::Duration::days(1));
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("nicht in der Zukunft"), "{err}");
    a.bescheide[0].zugestellt = None;
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("Folgebescheid"), "{err}");
    a.bescheide[0].zugestellt = Some(heute);
    // Verfahrensänderungen erscheinen mit eigener Aktion im Projektprotokoll.
    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    for aktion in [
        core_lib::domain::Aktion::VerfahrenAngelegt,
        core_lib::domain::Aktion::VerfahrenNachforderung,
        core_lib::domain::Aktion::VerfahrenAbgabe,
        core_lib::domain::Aktion::VerfahrensstatusGeaendert,
        core_lib::domain::Aktion::VerfahrenBescheid,
    ] {
        assert!(
            protokoll.iter().any(|e| e.eintrag.aktion == aktion),
            "{aktion:?} fehlt im Protokoll"
        );
    }
    assert!(
        a.ereignisse
            .iter()
            .any(|e| e.arten.contains(&chronik::Ereignisart::Zustellung))
    );
    // Fristen folgen aus Zustellung und Bundesland des Projekts.
    let fristen = core
        .bescheidfristen(
            p.id,
            &fristen::Fristauftrag {
                art: a.art.clone(),
                bescheide: a.bescheide.clone(),
            },
        )
        .await
        .unwrap();
    let rechtsbehelf = fristen[0].rechtsbehelfsfrist.as_ref().unwrap();
    assert_eq!(rechtsbehelf.beginn, heute);
    assert!(rechtsbehelf.ende > heute);
    assert!(fristen[0].geltungsdauer.is_some());
    // Nebenbestimmungen sind typisiert: Ein Hinweis wird nicht erfüllt und
    // nicht wiedervorgelegt, eine Befristung braucht ihr Ablaufdatum.
    let mut hinweis = a.bescheide[0].auflagen[0].clone();
    hinweis.id = Uuid::new_v4();
    hinweis.art = Nebenbestimmungsart::Hinweis;
    hinweis.titel = "Denkmalschutz bei Erdarbeiten beachten".into();
    hinweis.erledigt = true;
    hinweis.nachweise = vec![brief.clone()];
    a.bescheide[0].auflagen.push(hinweis);
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("Hinweis"), "{err}");
    a.bescheide[0].auflagen[1].erledigt = false;
    a.bescheide[0].auflagen[0].art = Nebenbestimmungsart::Befristung;
    a.bescheide[0].auflagen[0].frist = None;
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("Befristung"), "{err}");
    a.bescheide[0].auflagen[0].art = Nebenbestimmungsart::Bedingung;
    a.bescheide[0].auflagen[0].frist = Some(Frist {
        datum: heute,
        ausloeser: "Bescheid".into(),
        beleg: Some(brief.clone()),
        erinnerung_tage: 7,
        grund: "Terminüberwachung".into(),
    });
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    a = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Bedingung Nachweis vor Baubeginn")
    );
    let erinnerungen = core.verfahren_erinnerungen(p.id, heute).await.unwrap();
    assert!(
        erinnerungen
            .iter()
            .any(|e| e.objekt == a.bescheide[0].auflagen[0].id)
    );
    assert!(
        !erinnerungen
            .iter()
            .any(|e| e.objekt == a.bescheide[0].auflagen[1].id)
    );
    a.bescheide[0].zugestellt = Some(heute);
    a.status = Verfahrensstatus::InPruefung;
    let err = ablehnen(&a, f.revision).await;
    assert!(err.contains("nicht wieder geöffnet"), "{err}");
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
        core.verfahren_artefakt(p.id, vid, id, "zip", &user)
            .await
            .unwrap()
            .1,
        zip_vorher
    );
    assert_eq!(
        core.verfahren_artefakt(p.id, vid, id, "pdf", &user)
            .await
            .unwrap()
            .1,
        druck
    );
    let (_, a) = core.verfahrensakte(p.id, vid).await.unwrap();
    assert_eq!(a.bescheide[0].genehmigte_unterlagen[0], ref2);
    // Planindex und Einreichungsstatus aus der gespeicherten Akte.
    let unterlagen = core.verfahren_planunterlagen(p.id, vid).await.unwrap();
    let lage = unterlagen
        .iter()
        .find(|u| u.dokument_id == plan.document_id)
        .unwrap();
    assert_eq!(
        lage.status,
        core_lib::verfahren::planindex::Einreichungsstatus::Genehmigt
    );
    let v2 = lage.staende.iter().find(|s| s.version == 2).unwrap();
    assert_eq!(v2.bescheid, Some(besch));
    assert_eq!(v2.planindex, lage.planindex);
    assert!(lage.hinweise.is_empty(), "{:?}", lage.hinweise);
    assert!(
        a.abgaben
            .iter()
            .flat_map(|b| &b.dokumente)
            .all(|d| d.planindex.is_some())
    );
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
    // Fortlaufender Export in eine Datei liefert dieselben Einträge wie der
    // Export im Arbeitsspeicher.
    let datei = core
        .verfahren_aktenexport_in(p.id, tempfile::tempfile().unwrap())
        .await
        .unwrap();
    let datei_zip = zip::ZipArchive::new(datei).unwrap();
    let mut namen_datei: Vec<_> = datei_zip
        .file_names()
        .map(|n| n.unwrap().into_owned())
        .collect();
    let mut namen_speicher: Vec<_> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
    namen_datei.sort();
    namen_speicher.sort();
    assert_eq!(namen_datei, namen_speicher);
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
    // Mit der Entscheidung enden die Nachforderungen; die Bedingung bleibt offen.
    assert_eq!(akte.offene_punkte, 0);
    assert!(!uebersicht.punkte.iter().any(
        |p| p.ziel.objekt == Some(vid) && p.ziel.register.as_deref() == Some("nachforderungen")
    ));
    assert!(
        !core
            .verfahren_erinnerungen(p.id, heute)
            .await
            .unwrap()
            .iter()
            .any(|e| e.art == termine::Terminart::Forderungspunkt)
    );
    // Berechnete Bescheidfristen sind Termine; sie erscheinen ab ihrem Vorlauf.
    let liste = core.verfahren_termine(p.id, heute).await.unwrap();
    let rb = liste
        .iter()
        .find(|t| t.art == termine::Terminart::Rechtsbehelfsfrist)
        .unwrap();
    assert_eq!(rb.objekt, a.bescheide[0].id);
    assert!(
        liste
            .iter()
            .any(|t| t.art == termine::Terminart::Geltungsdauer)
    );
    let am_ende = core
        .projektuebersicht_am(p.id, rb.datum, &user)
        .await
        .unwrap();
    assert!(am_ende.punkte.iter().any(|h| h.id.starts_with("termin:")
        && h.titel.contains("Rechtsbehelfsfrist")
        && h.dringlichkeit == core_lib::projektuebersicht::Dringlichkeit::Heute));
    let danach = core
        .projektuebersicht_am(p.id, rb.datum + chrono::Duration::days(1), &user)
        .await
        .unwrap();
    assert!(
        !danach
            .punkte
            .iter()
            .any(|h| h.titel.contains("Rechtsbehelfsfrist"))
    );
    assert!(
        uebersicht
            .empfaenger
            .iter()
            .any(|e| e.dokumente.iter().any(|d| d.eingereichte_version == 2))
    );
    if let Ok(ziel) = std::env::var("BAUPROJEKT_VERFAHREN_ARTEFAKTE") {
        let ziel = std::path::Path::new(&ziel);
        std::fs::create_dir_all(ziel).unwrap();
        std::fs::write(ziel.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"verfahren":f,"akte":a,"dateien":core.list_documents(p.id).await.unwrap(),"benutzer":user,"erinnerungen":core.verfahren_erinnerungen(p.id,heute).await.unwrap(),"fristen":core.bescheidfristen(p.id,&fristen::Fristauftrag{art:a.art.clone(),bescheide:a.bescheide.clone()}).await.unwrap(),"treffer":treffer,"uebersicht":uebersicht,"planunterlagen":core.verfahren_planunterlagen(p.id,vid).await.unwrap()})).unwrap()).unwrap();
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
#[tokio::test]
async fn ruecknahme_braucht_datum_und_schreiben_und_bleibt_abgeschlossen() {
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
                name: "Rücknahme".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let vid = Uuid::new_v4();
    let f = speichern(&core, p.id, vid, 0, &leer(), &user).await;
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let versuch = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        let pid = p.id;
        async move {
            core.fachobjekt_speichern(pid, ObjektArt::Verfahren, vid, revision, a, user)
                .await
        }
    };
    a.status = Verfahrensstatus::Zurueckgenommen;
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("Rücknahmeschreiben"), "{err}");
    let schreiben = Uuid::new_v4();
    a.kommunikation.push(Kommunikation {
        id: schreiben,
        art: Kommunikationsart::Brief,
        eingehend: false,
        datum: core_lib::kalender::heute(),
        betreff: "Rücknahme des Bauantrags".into(),
        beteiligte: "Bauamt".into(),
        inhalt: String::new(),
        original: None,
        bezuege: vec![],
    });
    a.ruecknahme = Some(Ruecknahme {
        datum: core_lib::kalender::heute(),
        kommunikation: schreiben,
    });
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("zurückgenommen am")
    );
    a.status = Verfahrensstatus::Vorbereitung;
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("zurückgenommenes Verfahren"), "{err}");
}
#[tokio::test]
async fn anhoerung_und_zurueckstellung_mit_schreiben_frist_und_stellungnahme() {
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
                name: "Anhörung".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let heute = core_lib::kalender::heute();
    let vid = Uuid::new_v4();
    let f = speichern(&core, p.id, vid, 0, &leer(), &user).await;
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let versuch = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        let pid = p.id;
        async move {
            core.fachobjekt_speichern(pid, ObjektArt::Verfahren, vid, revision, a, user)
                .await
        }
    };
    let brief = |eingehend: bool, betreff: &str| Kommunikation {
        id: Uuid::new_v4(),
        art: Kommunikationsart::Brief,
        eingehend,
        datum: heute,
        betreff: betreff.into(),
        beteiligte: "Bauamt".into(),
        inhalt: String::new(),
        original: None,
        bezuege: vec![],
    };
    let anhoerung = brief(true, "Anhörung vor Ablehnung");
    let antwort = brief(false, "Stellungnahme zur Anhörung");
    a.kommunikation = vec![anhoerung.clone(), antwort.clone()];
    let frist = |datum| Frist {
        datum,
        ausloeser: "Anhörungsschreiben".into(),
        beleg: None,
        erinnerung_tage: 14,
        grund: "Frist der Behörde".into(),
    };
    let schritt = Verfahrensschritt {
        id: Uuid::new_v4(),
        art: Schrittart::Anhoerung,
        kommunikation: antwort.id,
        frist: Some(frist(heute + chrono::Duration::days(10))),
        stellungnahme: None,
        folgeschreiben: None,
        verantwortlich: Some(user.id.0),
    };
    a.schritte.push(schritt.clone());
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("eingegangene Schreiben"), "{err}");
    a.schritte[0].kommunikation = anhoerung.id;
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Anhörung erfasst")
    );
    let offen = |e: &[Erinnerung]| e.iter().any(|e| e.objekt == schritt.id);
    assert!(offen(
        &core.verfahren_erinnerungen(p.id, heute).await.unwrap()
    ));
    a.schritte[0].stellungnahme = Some(anhoerung.id);
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("ausgehende"), "{err}");
    a.schritte[0].stellungnahme = Some(antwort.id);
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Stellungnahme zur Anhörung")
    );
    assert!(!offen(
        &core.verfahren_erinnerungen(p.id, heute).await.unwrap()
    ));
    // Art und Schreiben bleiben, der Schritt selbst wird nicht gelöscht.
    a.schritte[0].art = Schrittart::Zurueckstellung;
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("bleiben unverändert"), "{err}");
    a.schritte.clear();
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("nicht gelöscht"), "{err}");
    // Eine Zurückstellung braucht ihr Ende und bleibt bis dahin wiedervorgelegt.
    let (_, mut a) = core.verfahrensakte(p.id, vid).await.unwrap();
    let gemeinde = brief(true, "Zurückstellung nach § 15 BauGB");
    a.kommunikation.push(gemeinde.clone());
    let zurueck = Verfahrensschritt {
        id: Uuid::new_v4(),
        art: Schrittart::Zurueckstellung,
        kommunikation: gemeinde.id,
        frist: None,
        stellungnahme: None,
        folgeschreiben: None,
        verantwortlich: None,
    };
    a.schritte.push(zurueck.clone());
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("Zurückstellung benötigt"), "{err}");
    a.schritte[1].frist = Some(frist(heute + chrono::Duration::days(365)));
    let f = versuch(&a, f.revision).await.unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Zurückstellung erfasst")
    );
    let spaeter = heute + chrono::Duration::days(360);
    assert!(
        core.verfahren_erinnerungen(p.id, spaeter)
            .await
            .unwrap()
            .iter()
            .any(|e| e.objekt == zurueck.id && e.titel.contains("Ende der Zurückstellung"))
    );
    // Nach dem Fristende ist die Zurückstellung nicht überfällig: Die
    // Wiedervorlage endet, die Übersicht wartet auf die Entscheidung.
    let ende = heute + chrono::Duration::days(366);
    assert!(
        !core
            .verfahren_erinnerungen(p.id, ende)
            .await
            .unwrap()
            .iter()
            .any(|e| e.objekt == zurueck.id)
    );
    let ue = core.projektuebersicht_am(p.id, ende, &user).await.unwrap();
    let punkt = ue
        .punkte
        .iter()
        .find(|h| h.ziel.eintrag == Some(zurueck.id))
        .unwrap();
    assert_eq!(
        punkt.dringlichkeit,
        core_lib::projektuebersicht::Dringlichkeit::Warten
    );
    assert!(punkt.titel.contains("Entscheidung erwarten"));
    // Ein Folgeschreiben der Behörde beendet die Zurückstellung vorzeitig.
    let mut a = a;
    let folge = brief(true, "Zurückstellung aufgehoben");
    a.kommunikation.push(folge.clone());
    a.schritte[1].folgeschreiben = Some(a.schritte[1].kommunikation);
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("weiteres eingegangenes"), "{err}");
    a.schritte[0].folgeschreiben = Some(folge.id);
    a.schritte[1].folgeschreiben = None;
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("nur eine Zurückstellung"), "{err}");
    a.schritte[0].folgeschreiben = None;
    a.schritte[1].folgeschreiben = Some(folge.id);
    versuch(&a, f.revision).await.unwrap();
    assert!(
        !core
            .verfahren_erinnerungen(p.id, spaeter)
            .await
            .unwrap()
            .iter()
            .any(|e| e.objekt == zurueck.id)
    );
}

/// Akte nach Versand in einem Land, mit eingegangenem Schreiben vom `d0`.
async fn eingereichte_akte(
    land: Bundesland,
    d0: chrono::NaiveDate,
) -> (
    tempfile::TempDir,
    CoreServices,
    Benutzer,
    ProjectId,
    Uuid,
    Fachstand,
    Dokumentbezug,
) {
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
                name: "Fristen".into(),
                bundesland: land,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Bauantrag.pdf",
            Bytes::from(pdf("Bauantrag")),
            &user,
        )
        .await
        .unwrap();
    let dok = Dokumentbezug {
        dokument_id: plan.document_id,
        version: 1,
    };
    let vid = Uuid::new_v4();
    let mut a = leer();
    a.kommunikation.push(Kommunikation {
        id: Uuid::new_v4(),
        art: Kommunikationsart::Brief,
        eingehend: true,
        datum: d0,
        betreff: "Eingangsbestätigung".into(),
        beteiligte: "Bauamt".into(),
        inhalt: String::new(),
        original: None,
        bezuege: vec![],
    });
    let f = speichern(&core, p.id, vid, 0, &a, &user).await;
    let w = Abgabewunsch {
        xbau_bezug: None,
        id: Uuid::new_v4(),
        titel: "Bauantrag".into(),
        dokumente: vec![dok.clone()],
        xbau: false,
        druck: false,
        anschreiben: String::new(),
    };
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, w, &user)
        .await
        .unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    a.abgaben[0].versand.push(Versandereignis {
        id: Uuid::new_v4(),
        status: Versandstatus::Versendet,
        datum: d0,
        weg: "Post".into(),
        referenz: "Einschreiben".into(),
        notiz: "Antrag versendet".into(),
        beleg: None,
        berichtigt: None,
    });
    a.status = Verfahrensstatus::Eingereicht;
    let f = speichern(&core, p.id, vid, f.revision, &a, &user).await;
    (dir, core, user, p.id, vid, f, dok)
}

fn berichtigung(gegenstand: chronik::Berichtigungsgegenstand, grund: &str) -> Aktenereignis {
    Aktenereignis {
        id: Uuid::new_v4(),
        datum: core_lib::kalender::heute(),
        erfasst_am: chrono::Utc::now(),
        erfasst_von: Uuid::nil(),
        text: "Berichtigung".into(),
        beleg: None,
        berichtigt: None,
        arten: vec![],
        berichtigung: Some(chronik::Wertberichtigung {
            gegenstand,
            grund: grund.into(),
            vorher: "vom Client".into(),
            nachher: String::new(),
        }),
    }
}

#[tokio::test]
async fn berliner_fristen_berichtigung_baubeginn_und_entscheidungsregeln() {
    use chronik::Berichtigungsgegenstand as G;
    use chrono::Duration;
    let heute = core_lib::kalender::heute();
    let d0 = heute - Duration::days(400);
    let (_dir, core, user, p, vid, f, dok) = eingereichte_akte(Bundesland::Be, d0).await;
    let versuch = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        async move {
            core.fachobjekt_speichern(p, ObjektArt::Verfahren, vid, revision, a, user)
                .await
        }
    };
    let fehler = |r: Result<Fachstand, CoreError>| r.unwrap_err().to_string();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let schreiben = a.kommunikation[0].id;
    // Vollständigkeit ist nach dem Speichern nur mit Berichtigung änderbar.
    a.status = Verfahrensstatus::Vollstaendig;
    a.vollstaendigkeit = Some(Vollstaendigkeit {
        datum: d0 + Duration::days(5),
        kommunikation: schreiben,
    });
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    a.vollstaendigkeit.as_mut().unwrap().datum = d0 + Duration::days(6);
    let err = fehler(versuch(&a, f.revision).await);
    assert!(err.contains("begründeter Berichtigung"), "{err}");
    let mut ohne = a.clone();
    ohne.vollstaendigkeit = None;
    ohne.ereignisse
        .push(berichtigung(G::Vollstaendigkeit, "Irrtümlich erfasst"));
    let err = fehler(versuch(&ohne, f.revision).await);
    assert!(err.contains("Bearbeitungsstand zusammen"), "{err}");
    a.ereignisse.push(berichtigung(
        G::Vollstaendigkeit,
        "Datum aus dem Schreiben übernommen",
    ));
    let f = versuch(&a, f.revision).await.unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let e = a
        .ereignisse
        .iter()
        .rev()
        .find(|e| e.berichtigung.is_some())
        .unwrap();
    let b = e.berichtigung.as_ref().unwrap();
    assert!(
        b.vorher
            .contains(&(d0 + Duration::days(5)).format("%d.%m.%Y").to_string())
    );
    assert!(
        b.nachher
            .contains(&(d0 + Duration::days(6)).format("%d.%m.%Y").to_string())
    );
    assert!(
        e.text.contains("Grund: Datum aus dem Schreiben"),
        "{}",
        e.text
    );
    assert_eq!(e.arten, vec![chronik::Ereignisart::Berichtigung]);
    // Ein Berichtigungsereignis ohne geänderten Wert wird abgewiesen.
    let mut ohne_aenderung = a.clone();
    ohne_aenderung
        .ereignisse
        .push(berichtigung(G::Ruecknahme, "Test"));
    let err = fehler(versuch(&ohne_aenderung, f.revision).await);
    assert!(err.contains("ohne geänderten"), "{err}");

    // „Entschieden“ verlangt einen echten Entscheidungsbescheid, ein
    // Entscheidungsbescheid verlangt „Entschieden“.
    let d2 = heute - Duration::days(300);
    let bescheid = |entscheidung| Bescheid {
        id: Uuid::new_v4(),
        entscheidung,
        datum: d2,
        zugestellt: Some(d2 + Duration::days(3)),
        original: dok.clone(),
        genehmigte_unterlagen: vec![dok.clone()],
        auflagen: vec![],
        ersetzt: None,
        verlaengerung: None,
    };
    let mut v = a.clone();
    v.bescheide.push(bescheid(Entscheidung::Genehmigt));
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("Entscheidungsbescheid"), "{err}");
    let mut v = a.clone();
    v.bescheide.push(bescheid(Entscheidung::Sonstiges));
    v.status = Verfahrensstatus::Entschieden;
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("Genehmigung oder Ablehnung"), "{err}");
    let mut v = a.clone();
    let mut abgelehnt = bescheid(Entscheidung::Abgelehnt);
    abgelehnt.genehmigte_unterlagen.clear();
    abgelehnt.auflagen.push(Auflage {
        id: Uuid::new_v4(),
        art: Nebenbestimmungsart::Auflage,
        titel: "Auflage".into(),
        ausloeser: String::new(),
        verantwortlich: None,
        frist: None,
        erledigt: false,
        nachweise: vec![],
    });
    v.bescheide.push(abgelehnt);
    v.status = Verfahrensstatus::Entschieden;
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("ablehnender Bescheid"), "{err}");
    let mut a = a;
    let genehmigung = bescheid(Entscheidung::Genehmigt);
    let gid = genehmigung.id;
    a.bescheide.push(genehmigung);
    a.status = Verfahrensstatus::Entschieden;
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();

    // Berlin: zwei Jahre bis Baubeginn, sechs bis Fertigstellung, mit Vorlauf.
    let zugestellt = d2 + Duration::days(3);
    let geltung = zugestellt
        .checked_add_months(chrono::Months::new(24))
        .unwrap();
    let fertig = zugestellt
        .checked_add_months(chrono::Months::new(72))
        .unwrap();
    let liste = core.verfahren_termine(p, heute).await.unwrap();
    let datum = |art| liste.iter().find(|t| t.art == art).map(|t| t.datum);
    assert_eq!(datum(termine::Terminart::Geltungsdauer), Some(geltung));
    assert_eq!(datum(termine::Terminart::Fertigstellung), Some(fertig));
    // Die Rechtsbehelfsfrist ist abgelaufen und endet damit.
    assert_eq!(datum(termine::Terminart::Rechtsbehelfsfrist), None);
    let wv = |tag| {
        let core = &core;
        async move { core.verfahren_erinnerungen(p, tag).await.unwrap() }
    };
    let geltungs_wv = |e: &[Erinnerung]| {
        e.iter()
            .find(|e| e.art == termine::Terminart::Geltungsdauer)
            .map(|e| e.ueberfaellig)
    };
    assert_eq!(geltungs_wv(&wv(geltung - Duration::days(100)).await), None);
    assert_eq!(
        geltungs_wv(&wv(geltung - Duration::days(30)).await),
        Some(false)
    );
    assert_eq!(
        geltungs_wv(&wv(geltung + Duration::days(10)).await),
        Some(true)
    );
    let ics = core.verfahren_termine_ics(p, heute).await.unwrap();
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), liste.len());
    assert!(ics.contains(&format!("DTSTART;VALUE=DATE:{}", geltung.format("%Y%m%d"))));

    // Zustelldatum berichtigen verschiebt die Fristen und hinterlässt eine Spur.
    a.bescheide[0].zugestellt = Some(zugestellt + Duration::days(1));
    a.ereignisse.push(berichtigung(
        G::Zustellung { bescheid: gid },
        "Zustellungsurkunde nennt den Folgetag",
    ));
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    assert!(a.ereignisse.iter().any(|e| {
        e.text.contains("Zustelldatum des Bescheids vom")
            && e.berichtigung
                .as_ref()
                .is_some_and(|b| b.vorher == zugestellt.format("%d.%m.%Y").to_string())
    }));
    assert_eq!(
        core.verfahren_termine(p, heute)
            .await
            .unwrap()
            .iter()
            .find(|t| t.art == termine::Terminart::Geltungsdauer)
            .map(|t| t.datum),
        (zugestellt + Duration::days(1)).checked_add_months(chrono::Months::new(24))
    );
    assert!(
        core.protokoll(p, 20)
            .await
            .unwrap()
            .iter()
            .any(|e| e.eintrag.aktion == core_lib::domain::Aktion::VerfahrenBerichtigt)
    );

    // Befristung erinnert vor dem Ablauf und endet danach; sie ist nicht erfüllbar.
    let ablauf = heute + Duration::days(20);
    let befristung = Auflage {
        id: Uuid::new_v4(),
        art: Nebenbestimmungsart::Befristung,
        titel: "Fliegender Bau".into(),
        ausloeser: "Befristet bis".into(),
        verantwortlich: None,
        frist: Some(Frist {
            datum: ablauf,
            ausloeser: "Bescheid".into(),
            beleg: None,
            erinnerung_tage: 30,
            grund: "Befristung".into(),
        }),
        erledigt: true,
        nachweise: vec![dok.clone()],
    };
    let mut vorbehalt = befristung.clone();
    vorbehalt.id = Uuid::new_v4();
    vorbehalt.art = Nebenbestimmungsart::Widerrufsvorbehalt;
    vorbehalt.frist = None;
    vorbehalt.erledigt = false;
    a.bescheide[0].auflagen.push(befristung);
    let err = fehler(versuch(&a, f.revision).await);
    assert!(err.contains("keine erfüllbare Pflicht"), "{err}");
    a.bescheide[0].auflagen[0].erledigt = false;
    a.bescheide[0].auflagen.push(vorbehalt);
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let ablaeufe = |e: &[Erinnerung]| {
        e.iter()
            .filter(|e| e.art == termine::Terminart::Ablauf)
            .count()
    };
    assert_eq!(ablaeufe(&wv(heute).await), 1);
    assert_eq!(ablaeufe(&wv(ablauf + Duration::days(1)).await), 0);
    let ue = core.projektuebersicht(p, &user).await.unwrap();
    assert_eq!(ue.verfahren[0].offene_auflagen, 0);

    // Ein Verlängerungsbescheid ersetzt die Geltungsdauer-Wiedervorlage.
    let bis = geltung + Duration::days(365);
    let mut verl = bescheid(Entscheidung::Genehmigt);
    verl.datum = heute - Duration::days(1);
    verl.zugestellt = None;
    verl.genehmigte_unterlagen.clear();
    verl.verlaengerung = Some(Verlaengerung { bescheid: gid, bis });
    let vid2 = verl.id;
    let mut zweite = verl.clone();
    zweite.id = Uuid::new_v4();
    let mut v = a.clone();
    v.bescheide.extend([verl.clone(), zweite]);
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("höchstens einmal"), "{err}");
    a.bescheide.push(verl);
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let g: Vec<_> = core
        .verfahren_termine(p, heute)
        .await
        .unwrap()
        .into_iter()
        .filter(|t| t.art == termine::Terminart::Geltungsdauer)
        .collect();
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].objekt, g[0].datum), (vid2, bis));

    // Der Baubeginn beendet die Geltungsdauer, nicht die Fertigstellungsfrist.
    a.ausfuehrung = Some(Bauausfuehrung {
        baubeginn: heute + Duration::days(1),
        fertigstellung: None,
    });
    let err = fehler(versuch(&a, f.revision).await);
    assert!(err.contains("Zukunft"), "{err}");
    a.ausfuehrung = Some(Bauausfuehrung {
        baubeginn: heute - Duration::days(10),
        fertigstellung: None,
    });
    let f = versuch(&a, f.revision).await.unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .arten
            .contains(&chronik::Ereignisart::Ausfuehrung)
    );
    let liste = core.verfahren_termine(p, heute).await.unwrap();
    assert!(
        !liste
            .iter()
            .any(|t| t.art == termine::Terminart::Geltungsdauer)
    );
    assert!(
        liste
            .iter()
            .any(|t| t.art == termine::Terminart::Fertigstellung)
    );
    let mut v = a.clone();
    v.ausfuehrung.as_mut().unwrap().baubeginn = heute - Duration::days(11);
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("Baubeginn ist erfasst"), "{err}");
}

#[tokio::test]
async fn ruecknahme_rueckweisung_und_zurueckgenommene_akte() {
    let heute = core_lib::kalender::heute();
    let d0 = heute - chrono::Duration::days(30);
    let (_dir, core, user, p, vid, f, _dok) = eingereichte_akte(Bundesland::Nw, d0).await;
    let versuch = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        async move {
            core.fachobjekt_speichern(p, ObjektArt::Verfahren, vid, revision, a, user)
                .await
        }
    };
    let fehler = |r: Result<Fachstand, CoreError>| r.unwrap_err().to_string();
    let versand = |status, notiz: &str| Versandereignis {
        id: Uuid::new_v4(),
        status,
        datum: heute,
        weg: "Post".into(),
        referenz: String::new(),
        notiz: notiz.into(),
        beleg: None,
        berichtigt: None,
    };
    let a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    // Wird die einzige Abgabe zurückgewiesen, folgt der Bearbeitungsstand.
    let mut v = a.clone();
    v.abgaben[0].versand.push(versand(
        Versandstatus::Zurueckgewiesen,
        "Unterlagen zurückgesandt",
    ));
    let err = fehler(versuch(&v, f.revision).await);
    assert!(err.contains("Bearbeitungsstand zusammen"), "{err}");
    v.status = Verfahrensstatus::Vorbereitung;
    let f = versuch(&v, f.revision).await.unwrap();
    let mut v: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    v.abgaben[0]
        .versand
        .push(versand(Versandstatus::Versendet, "Erneut versendet"));
    v.status = Verfahrensstatus::Eingereicht;
    let f = versuch(&v, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    // Rücknahme nur zusammen mit dem Status.
    let schreiben = a.kommunikation[0].id;
    a.ruecknahme = Some(Ruecknahme {
        datum: heute,
        kommunikation: schreiben,
    });
    let err = fehler(versuch(&a, f.revision).await);
    assert!(err.contains("nur zusammen"), "{err}");
    a.status = Verfahrensstatus::Zurueckgenommen;
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    a.nachforderungen.push(Nachforderung {
        id: Uuid::new_v4(),
        kommunikation: schreiben,
        punkte: vec![Forderungspunkt {
            id: Uuid::new_v4(),
            titel: "Nach Rücknahme".into(),
            grundlage: String::new(),
            verantwortlich: None,
            frist: None,
            status: Punktstatus::Offen,
            antwort: String::new(),
            dokumente: vec![],
            abgaben: vec![],
            bestaetigung: None,
        }],
    });
    let err = fehler(versuch(&a, f.revision).await);
    assert!(
        err.contains("zurückgenommenes Verfahren erhält keine"),
        "{err}"
    );
    assert!(core.verfahren_termine(p, heute).await.unwrap().is_empty());
}
#[tokio::test]
async fn altakte_mit_externer_einreichung_ohne_zukunftsdaten_und_mit_festem_wortlaut() {
    use core_lib::verfahren::externe_einreichung::{
        Einreichungsweg, ExterneEinreichung, ExterneEinreichungswunsch, gueltige_externe,
    };
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
                name: "Altakte".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let heute = core_lib::kalender::heute();
    let morgen = heute + chrono::Duration::days(1);
    let mut refs = Vec::new();
    for name in [
        "Lageplan.pdf",
        "Entwurf.pdf",
        "Einlieferungsbeleg.pdf",
        "Bescheid.pdf",
    ] {
        let d = core
            .store_document(p.id, None, None, name, Bytes::from(pdf(name)), &user)
            .await
            .unwrap();
        refs.push(Dokumentbezug {
            dokument_id: d.document_id,
            version: d.version,
        });
    }
    let [plan, anderer, quittung, bescheid] = <[Dokumentbezug; 4]>::try_from(refs).unwrap();
    let vid = Uuid::new_v4();
    let f = speichern(&core, p.id, vid, 0, &leer(), &user).await;
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let versuch = |a: &Verfahrensakte, revision: i64| {
        let a = serde_json::to_value(a).unwrap();
        let core = &core;
        let user = &user;
        let pid = p.id;
        async move {
            core.fachobjekt_speichern(pid, ObjektArt::Verfahren, vid, revision, a, user)
                .await
        }
    };
    // Ohne Einreichung kein Status „Eingereicht“.
    a.status = Verfahrensstatus::Eingereicht;
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("externe Einreichung"), "{err}");
    a.status = Verfahrensstatus::Vorbereitung;
    let wunsch = ExterneEinreichungswunsch {
        id: Uuid::new_v4(),
        datum: "2024-03-01".parse().unwrap(),
        weg: Einreichungsweg::Post,
        beleg: Some(quittung.clone()),
        bemerkung: "Per Einschreiben an das Bauamt".into(),
        dokumente: vec![plan.clone()],
        berichtigt: None,
    };
    // Über die allgemeine Speicherung lässt sich keine Einreichung behaupten.
    a.externe_einreichungen.push(ExterneEinreichung {
        id: wunsch.id,
        datum: wunsch.datum,
        weg: wunsch.weg,
        beleg: None,
        bemerkung: String::new(),
        dokumente: vec![],
        berichtigt: None,
        erfasst_am: chrono::Utc::now(),
        erfasst_von: user.id.0,
    });
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("Externe Einreichung nachtragen"), "{err}");
    let mut kuenftig = wunsch.clone();
    kuenftig.datum = morgen;
    assert!(
        core.verfahren_externe_einreichung(p.id, vid, f.revision, kuenftig, &user)
            .await
            .is_err()
    );
    let mut sonstig = wunsch.clone();
    sonstig.weg = Einreichungsweg::Sonstiges;
    sonstig.bemerkung = " ".into();
    assert!(
        core.verfahren_externe_einreichung(p.id, vid, f.revision, sonstig, &user)
            .await
            .is_err()
    );
    let f = core
        .verfahren_externe_einreichung(p.id, vid, f.revision, wunsch.clone(), &user)
        .await
        .unwrap();
    // Wiederholung ist wirkungslos, abweichende Angaben unter gleicher Kennung nicht.
    assert_eq!(
        core.verfahren_externe_einreichung(p.id, vid, f.revision, wunsch.clone(), &user)
            .await
            .unwrap()
            .revision,
        f.revision
    );
    let mut anders = wunsch.clone();
    anders.bemerkung = "Anders".into();
    assert!(
        core.verfahren_externe_einreichung(p.id, vid, f.revision, anders, &user)
            .await
            .is_err()
    );
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    assert_eq!(a.externe_einreichungen.len(), 1);
    assert_eq!(a.externe_einreichungen[0].erfasst_von, user.id.0);
    assert!(a.abgaben.is_empty());
    assert!(
        a.ereignisse
            .last()
            .unwrap()
            .text
            .contains("Externe Einreichung nachgetragen: Post am 2024-03-01")
    );
    a.status = Verfahrensstatus::Eingereicht;
    // Fachliche Daten tatsächlicher Vorgänge liegen nicht in der Zukunft.
    let schreiben = Uuid::new_v4();
    a.kommunikation.push(Kommunikation {
        id: schreiben,
        art: Kommunikationsart::Brief,
        eingehend: true,
        datum: morgen,
        betreff: "Nachforderung".into(),
        beteiligte: "Bauamt".into(),
        inhalt: String::new(),
        original: None,
        bezuege: vec![],
    });
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("nicht in der Zukunft"), "{err}");
    a.kommunikation[0].datum = heute;
    a.nachforderungen.push(Nachforderung {
        id: Uuid::new_v4(),
        kommunikation: schreiben,
        punkte: vec![Forderungspunkt {
            id: Uuid::new_v4(),
            titel: "Brandschutznachweis vorlegen".into(),
            grundlage: "Art. 62 BayBO".into(),
            verantwortlich: None,
            frist: None,
            status: Punktstatus::Offen,
            antwort: String::new(),
            dokumente: vec![],
            abgaben: vec![],
            bestaetigung: None,
        }],
    });
    let neuer_bescheid = |genehmigt: Dokumentbezug, zugestellt| Bescheid {
        id: Uuid::new_v4(),
        entscheidung: Entscheidung::Genehmigt,
        datum: heute,
        zugestellt,
        original: bescheid.clone(),
        genehmigte_unterlagen: vec![genehmigt],
        auflagen: vec![Auflage {
            id: Uuid::new_v4(),
            art: Nebenbestimmungsart::Auflage,
            titel: "Stellplätze vor Nutzungsaufnahme herstellen".into(),
            ausloeser: "Nutzungsaufnahme".into(),
            verantwortlich: None,
            frist: None,
            erledigt: false,
            nachweise: vec![],
        }],
        ersetzt: None,
        verlaengerung: None,
    };
    // Genehmigt werden nur Unterlagen, die der Behörde vorlagen.
    a.bescheide.push(neuer_bescheid(anderer, None));
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("externen Einreichung"), "{err}");
    a.bescheide[0] = neuer_bescheid(plan.clone(), Some(morgen));
    let err = versuch(&a, f.revision).await.unwrap_err().to_string();
    assert!(err.contains("Zustelldatum"), "{err}");
    a.bescheide[0].zugestellt = Some(heute);
    let f = versuch(&a, f.revision).await.unwrap();
    let mut a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    // Wortlaut behördlicher Einträge bleibt fest, Bearbeitungsangaben nicht.
    let mut geaendert = a.clone();
    geaendert.nachforderungen[0].punkte[0].titel = "Brandschutz später".into();
    let err = versuch(&geaendert, f.revision)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Wortlaut"), "{err}");
    let mut geaendert = a.clone();
    geaendert.nachforderungen[0].punkte[0].grundlage = "keine".into();
    assert!(versuch(&geaendert, f.revision).await.is_err());
    let mut geaendert = a.clone();
    geaendert.bescheide[0].auflagen[0].titel = "Stellplätze irgendwann".into();
    let err = versuch(&geaendert, f.revision)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Wortlaut"), "{err}");
    a.nachforderungen[0].punkte[0].status = Punktstatus::InArbeit;
    a.nachforderungen[0].punkte[0].antwort = "Prüfstatiker beauftragt".into();
    a.nachforderungen[0].punkte[0].verantwortlich = Some(user.id.0);
    a.bescheide[0].auflagen[0].verantwortlich = Some(user.id.0);
    let f = versuch(&a, f.revision).await.unwrap();
    // Berichtigung als Folgeeintrag; ein Eintrag wird nur einmal berichtigt.
    let berichtigung = ExterneEinreichungswunsch {
        id: Uuid::new_v4(),
        datum: "2024-03-04".parse().unwrap(),
        bemerkung: "Eingangsstempel der Behörde zeigt den 04.03.2024".into(),
        berichtigt: Some(wunsch.id),
        ..wunsch.clone()
    };
    let f = core
        .verfahren_externe_einreichung(p.id, vid, f.revision, berichtigung, &user)
        .await
        .unwrap();
    let a: Verfahrensakte = serde_json::from_value(f.daten.clone()).unwrap();
    let gueltig: Vec<_> = gueltige_externe(&a, heute).cloned().collect();
    assert_eq!(gueltig.len(), 1);
    assert_eq!(gueltig[0].datum.to_string(), "2024-03-04");
    let zweite = ExterneEinreichungswunsch {
        id: Uuid::new_v4(),
        bemerkung: "Nochmals".into(),
        berichtigt: Some(wunsch.id),
        ..wunsch.clone()
    };
    assert!(
        core.verfahren_externe_einreichung(p.id, vid, f.revision, zweite, &user)
            .await
            .is_err()
    );
    // Die Übersicht führt die externe Einreichung als Abgabe.
    let uebersicht = core.projektuebersicht(p.id, &user).await.unwrap();
    assert!(
        uebersicht
            .abgaben
            .iter()
            .any(|b| b.status == "Extern eingereicht" && b.datum == Some(gueltig[0].datum))
    );
    // Ältere Akten ohne das Feld bleiben lesbar; leer wird es nicht geschrieben.
    let json = serde_json::to_value(leer()).unwrap();
    assert!(json.get("externe_einreichungen").is_none());
    let alt: Verfahrensakte = serde_json::from_value(json).unwrap();
    assert!(alt.externe_einreichungen.is_empty());
}
