//! Ende-zu-Ende: Bauantrag erfassen, Büroangaben, XBau-Paket aus dem
//! Abgabestand der Verfahrensakte über `CoreServices` mit SQLite und lokalem
//! Storage.

use std::io::{Cursor, Read};
use std::sync::Arc;

use bytes::Bytes;
use core_lib::benutzer::{NeuerBenutzer, Rolle};
use core_lib::domain::{Aktion, Bundesland, NewProject};
use core_lib::kunden::Kundendaten;
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::xbau::codeliste::{ANLAGEN, MIMETYPES};
use core_lib::xbau::paket::NACHRICHT_DATEI;
use core_lib::xbau::{
    Anlagenwahl, Anschrift, Bauaufsicht, Buero, Codeeintrag, Codeliste, Codelisten,
    Entwurfsverfasser,
};
use core_lib::{CoreError, CoreServices};

/// Codelisten mit Platzhalter-Einträgen (keine echten XRepository-Werte).
fn codelisten() -> Codelisten {
    let mut c = Codelisten::eingebaut();
    for (uri, code) in [(ANLAGEN, "0100"), (MIMETYPES, "application/pdf")] {
        c.hinzufuegen(Codeliste {
            uri: uri.into(),
            version: "test".into(),
            name: uri.into(),
            eintraege: vec![Codeeintrag {
                code: code.into(),
                beschreibung: "Test".into(),
            }],
        });
    }
    c
}

async fn core(dir: &std::path::Path, mit_codelisten: bool) -> CoreServices {
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.to_path_buf(),
    })
    .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard());
    if mit_codelisten {
        core.mit_codelisten(codelisten())
    } else {
        core
    }
}

#[tokio::test]
async fn bauantrag_erfassen_und_exportieren() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &chefin,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from_static(b"%PDF-1.7 Lageplan"),
            &chefin,
        )
        .await
        .unwrap();

    // Neu: Vorgaben, Version 0, fehlende Angaben werden genannt.
    let leer = core.bauantrag(p.id).await.unwrap();
    assert_eq!(leer.version, 0);
    assert!(leer.fehlende_angaben.iter().any(|f| f == "Büro: Name"));
    assert!(
        leer.fehlende_angaben
            .iter()
            .any(|f| f == "Entwurfsverfasser (bauvorlageberechtigte Person)")
    );
    // Einreichen läuft über die Verfahrensakte: Behörde und Aktenzeichen
    // stammen aus der Akte, das XBau-Paket aus dem Abgabestand.
    let vid = uuid::Uuid::new_v4();
    let a = serde_json::json!({"bezeichnung":"Erstantrag","art":"bauantrag","status":"vorbereitung","behoerde":{"id":uuid::Uuid::new_v4(),"name":"Bauamt","kennung":"bab:053150000","kontakt":""},"aktenzeichen":"BA-42","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]});
    core.fachobjekt_speichern(
        p.id,
        core_lib::plattform::ObjektArt::Verfahren,
        vid,
        0,
        a,
        &chefin,
    )
    .await
    .unwrap();
    let abgabe = |xbau_bezug, titel: &str| core_lib::verfahren::abgabe::Abgabewunsch {
        xbau_bezug,
        xbau_folge: None,
        id: uuid::Uuid::new_v4(),
        titel: titel.into(),
        dokumente: vec![core_lib::plattform::objekte::Dokumentbezug {
            dokument_id: plan.document_id,
            version: 1,
        }],
        xbau: true,
        druck: false,
        anschreiben: String::new(),
    };
    let revision = || {
        let core = &core;
        let pid = p.id;
        async move { core.verfahrensakte(pid, vid).await.unwrap().0.revision }
    };
    let err = core
        .verfahren_abgabe_festhalten(p.id, vid, 1, abgabe(None, "Zu früh"), &chefin)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Bauherr"), "{err}");

    core.buero_speichern(buero("Architekturbüro Muster"), &chefin)
        .await
        .unwrap();

    let mut antrag = leer.antrag;
    antrag.bauaufsicht = Bauaufsicht {
        name: "Bauordnungsamt Köln".into(),
        kennung: "bab:053150000".into(),
    };
    let anna = core
        .kunde_anlegen(
            Kundendaten {
                vorname: "Anna".into(),
                familienname: "Müller".into(),
                anschrift: anschrift(),
                ..Kundendaten::default()
            },
            &chefin,
        )
        .await
        .unwrap();
    core.bauherren_setzen(p.id, vec![anna.id], p.version, &chefin)
        .await
        .unwrap();
    antrag.grundstueck.flurstuecke[0].gemarkung = "Lindenthal".into();
    antrag.grundstueck.flurstuecke[0].zaehler = "17".into();
    antrag.anlagen.push(Anlagenwahl {
        dokument: plan.document_id,
        anlageart: "0100".into(),
        bezeichnung: String::new(),
    });
    let gespeichert = core
        .bauantrag_speichern(p.id, antrag.clone(), 0, &chefin)
        .await
        .unwrap();
    assert_eq!(gespeichert.version, 1);
    assert_eq!(gespeichert.fehlende_angaben, Vec::<String>::new());
    assert_eq!(gespeichert.antrag, antrag);

    // Veraltete Version: Konflikt.
    assert!(matches!(
        core.bauantrag_speichern(p.id, antrag.clone(), 0, &chefin)
            .await,
        Err(CoreError::VersionConflict { .. })
    ));

    let erst = abgabe(None, "Antragspaket");
    let aid = erst.id;
    core.verfahren_abgabe_festhalten(p.id, vid, revision().await, erst, &chefin)
        .await
        .unwrap();

    // Den eingefrorenen Stand herunterladen darf auch, wer nur lesen darf.
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
        core.bauantrag_speichern(p.id, antrag, 1, &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    assert!(matches!(
        core.verfahren_artefakt(p.id, vid, aid, "xbau", &leser)
            .await,
        Err(CoreError::Forbidden(_))
    ));
    core.projektmitglied_setzen(
        p.id,
        core_lib::zugriff::ProjektMitglied {
            benutzer_id: leser.id,
            recht: core_lib::zugriff::ProjektRecht::Lesen,
        },
        0,
        &chefin,
    )
    .await
    .unwrap();
    let vorher = core
        .verfahren_artefakt(p.id, vid, aid, "xbau", &leser)
        .await
        .unwrap()
        .1;
    let mut zip = zip::ZipArchive::new(Cursor::new(&vorher)).unwrap();
    let mut xml = String::new();
    zip.by_name(NACHRICHT_DATEI)
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("<code>05</code>"), "NRW");
    assert!(
        xml.contains("<name>Müller</name>"),
        "Bauherr aus der Kundendatenbank"
    );
    assert!(
        xml.contains(&anna.id.to_string()),
        "Kunden-ID als Akteur-ID"
    );
    assert!(
        xml.contains("<xbau:mitgliedsnummer>12345</xbau:mitgliedsnummer>"),
        "Entwurfsverfasser mit Kammereintrag aus den Büroangaben"
    );
    assert!(xml.contains("Architekturbüro Muster"));
    assert!(xml.contains("<dateiname>Lageplan.pdf</dateiname>"));
    assert!(xml.contains(&format!("urn:uuid:{}", plan.document_id.0)));
    let mut pdf = Vec::new();
    zip.by_name("Lageplan.pdf")
        .unwrap()
        .read_to_end(&mut pdf)
        .unwrap();
    assert_eq!(pdf, b"%PDF-1.7 Lageplan");
    let protokoll = core.protokoll(p.id, 50).await.unwrap();
    assert!(
        protokoll
            .iter()
            .any(|e| e.eintrag.aktion == Aktion::BauantragGeaendert)
    );

    // Der Abgabestand wird als Ausgang ins Journal übernommen; die Behörde
    // antwortet darauf mit einer Befundliste.
    let ausgang = core
        .verfahren_xbau_erfassen(p.id, vid, aid, &chefin)
        .await
        .unwrap();
    let eingang_id = uuid::Uuid::new_v4();
    let eingang = format!(
        r#"<k:prozessnachrichten.formellePruefungBefundliste.1140 xmlns:k="http://www.xleitstelle.de/xbau/kernmodul/1/4/0" produkt="Test" produkthersteller="Test" standard="XBau-Kernmodul" version="1.4.0"><nachrichtenkopf.g2g><identifikation.nachricht><nachrichtenUUID>{eingang_id}</nachrichtenUUID><nachrichtentyp listURI="urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten" listVersionID="3.0"><code>1140</code></nachrichtentyp><erstellungszeitpunkt>2026-01-02T12:00:00Z</erstellungszeitpunkt></identifikation.nachricht><leser><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>planer:test</kennung><name>Planer</name></leser><autor><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>{}</kennung><name>Behörde</name></autor></nachrichtenkopf.g2g><bezug><referenz>{}</referenz><vorgang>BA-42</vorgang></bezug><dokumentationVollstaendig>false</dokumentationVollstaendig></k:prozessnachrichten.formellePruefungBefundliste.1140>"#,
        ausgang.nachricht.pruefung.kopf.empfaenger,
        ausgang.nachricht.pruefung.kopf.antragsreferenz.unwrap()
    );
    let eingang_doc = core
        .store_document(
            p.id,
            None,
            None,
            "Befundliste.xml",
            Bytes::from(eingang),
            &chefin,
        )
        .await
        .unwrap();
    core.xbau_aufnehmen(
        p.id,
        core_lib::xbau::postfach::Nachrichtenaufnahme {
            original: core_lib::plattform::objekte::Dokumentbezug {
                dokument_id: eingang_doc.document_id,
                version: 1,
            },
            richtung: core_lib::xbau::postfach::Richtung::Eingang,
            zuordnung_bestaetigt: true,
        },
        &chefin,
    )
    .await
    .unwrap();

    // Nachreichung auf die Befundliste: 0202 mit Bezug und Aktenzeichen.
    let folge = abgabe(Some(eingang_id), "Eingefrorene Nachreichung");
    let folge_id = folge.id;
    core.verfahren_abgabe_festhalten(p.id, vid, revision().await, folge, &chefin)
        .await
        .unwrap();
    let folge_bytes = core
        .verfahren_artefakt(p.id, vid, folge_id, "xbau", &chefin)
        .await
        .unwrap()
        .1;
    let folge_pruefung =
        core_lib::xbau::austausch::pruefen("XBau.zip", &folge_bytes, core.xbau_pruefer.as_ref())
            .await
            .unwrap();
    assert_eq!(
        folge_pruefung.kopf.element,
        "baugenehmigung.antragGeaendert.0202"
    );
    assert_eq!(folge_pruefung.kopf.bezugsnachricht, Some(eingang_id));
    assert_eq!(folge_pruefung.kopf.aktenzeichen.as_deref(), Some("BA-42"));

    // Nachreichung auf den eigenen Antrag behält dessen Antragsreferenz.
    let eigene = abgabe(Some(ausgang.id), "Nachreichung zum Antrag");
    let eigene_id = eigene.id;
    core.verfahren_abgabe_festhalten(p.id, vid, revision().await, eigene, &chefin)
        .await
        .unwrap();
    let eigene_pruefung = core_lib::xbau::austausch::pruefen(
        "XBau.zip",
        &core
            .verfahren_artefakt(p.id, vid, eigene_id, "xbau", &chefin)
            .await
            .unwrap()
            .1,
        core.xbau_pruefer.as_ref(),
    )
    .await
    .unwrap();
    assert_eq!(
        eigene_pruefung.kopf.element,
        "baugenehmigung.antragGeaendert.0202"
    );
    assert_eq!(
        eigene_pruefung.kopf.antragsreferenz,
        ausgang.nachricht.pruefung.kopf.antragsreferenz
    );

    // Folgenachrichten über denselben Abgabeweg: Verlängerung erst nach einer
    // Genehmigung, Rücknahme mit der Erklärung aus dem Anschreiben.
    use core_lib::verfahren::abgabe::Folgewunsch;
    let mut verl = abgabe(None, "Verlängerung");
    verl.xbau_folge = Some(Folgewunsch::Verlaengerung { bis: None });
    let e = core
        .verfahren_abgabe_festhalten(p.id, vid, revision().await, verl, &chefin)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("erteilte Genehmigung"), "{e}");
    let mut ruecknahme = abgabe(None, "Rücknahme");
    ruecknahme.xbau_folge = Some(Folgewunsch::Ruecknahme);
    assert!(
        core.verfahren_abgabe_festhalten(p.id, vid, revision().await, ruecknahme.clone(), &chefin)
            .await
            .unwrap_err()
            .to_string()
            .contains("Erklärung")
    );
    ruecknahme.anschreiben = "Der Bauantrag wird zurückgenommen.".into();
    let ruecknahme_id = ruecknahme.id;
    core.verfahren_abgabe_festhalten(p.id, vid, revision().await, ruecknahme, &chefin)
        .await
        .unwrap();
    let r = core_lib::xbau::austausch::pruefen(
        "XBau.zip",
        &core
            .verfahren_artefakt(p.id, vid, ruecknahme_id, "xbau", &chefin)
            .await
            .unwrap()
            .1,
        core.xbau_pruefer.as_ref(),
    )
    .await
    .unwrap();
    assert_eq!(r.kopf.element, "prozessnachrichten.ruecknahme.1130");
    assert_eq!(r.kopf.aktenzeichen.as_deref(), Some("BA-42"));
    assert_eq!(r.anlagen.len(), 1);

    // Gemeinsamer Abgabestand friert auch XBau ein: spätere Büro- und
    // Planänderungen dürfen weder XML noch Anlagenbytes neu erzeugen.
    core.buero_speichern(buero("Neuer Büroname"), &chefin)
        .await
        .unwrap();
    core.store_document(
        p.id,
        Some(plan.document_id),
        Some(1),
        "Lageplan.pdf",
        Bytes::from_static(b"%PDF-1.7 Neuer Entwurf"),
        &chefin,
    )
    .await
    .unwrap();
    assert_eq!(
        core.verfahren_artefakt(p.id, vid, aid, "xbau", &chefin)
            .await
            .unwrap()
            .1,
        vorher
    );

    // Die Verfahrensart wählt die XBau-Nachricht; Arten ohne Antragsnachricht
    // werden abgewiesen statt als Bauantrag gesendet.
    let abgabe = |xbau_bezug| core_lib::verfahren::abgabe::Abgabewunsch {
        xbau_bezug,
        xbau_folge: None,
        id: uuid::Uuid::new_v4(),
        titel: "Abgabe".into(),
        dokumente: vec![core_lib::plattform::objekte::Dokumentbezug {
            dokument_id: plan.document_id,
            version: 1,
        }],
        xbau: true,
        druck: false,
        anschreiben: String::new(),
    };
    let verfahren = |art: &str, angaben: serde_json::Value| {
        let mut a = serde_json::json!({"bezeichnung":art,"art":art,"status":"vorbereitung","behoerde":{"id":uuid::Uuid::new_v4(),"name":"Bauamt","kennung":"bab:053150000","kontakt":""},"aktenzeichen":"","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]});
        if !angaben.is_null() {
            a["antragsangaben"] = angaben;
        }
        a
    };
    let mut vorbescheid = None;
    for (art, angaben, erwartet) in [
        (
            "vorbescheid",
            serde_json::json!({"vorbescheidfragen":[{"id":uuid::Uuid::new_v4(),"frage":"Ist ein drittes Vollgeschoss zulässig?"}],"als_bauantrag_weiterfuehren":false}),
            Ok("vorbescheid.antrag.0210"),
        ),
        (
            "freistellung",
            serde_json::json!({"vorbescheidfragen":[],"als_bauantrag_weiterfuehren":true}),
            Ok("genehmigungsfreistellung.anzeige.0600"),
        ),
        (
            "aenderungsantrag",
            serde_json::Value::Null,
            Ok("baugenehmigung.antrag.0200"),
        ),
        (
            "vorbescheid",
            serde_json::Value::Null,
            Err("Frage zum Vorbescheid"),
        ),
        ("sonstiges", serde_json::Value::Null, Err("„Sonstiges“")),
        (
            "teilbaugenehmigung",
            serde_json::Value::Null,
            Err("Teilbaugenehmigung"),
        ),
    ] {
        let vid = uuid::Uuid::new_v4();
        core.fachobjekt_speichern(
            p.id,
            core_lib::plattform::ObjektArt::Verfahren,
            vid,
            0,
            verfahren(art, angaben),
            &chefin,
        )
        .await
        .unwrap();
        let w = abgabe(None);
        let aid = w.id;
        let ergebnis = core
            .verfahren_abgabe_festhalten(p.id, vid, 1, w, &chefin)
            .await;
        match erwartet {
            Ok(element) => {
                ergebnis.unwrap();
                let bytes = core
                    .verfahren_artefakt(p.id, vid, aid, "xbau", &chefin)
                    .await
                    .unwrap()
                    .1;
                let pruefung = core_lib::xbau::austausch::pruefen(
                    "XBau.zip",
                    &bytes,
                    core.xbau_pruefer.as_ref(),
                )
                .await
                .unwrap();
                assert_eq!(pruefung.kopf.element, element, "{art}");
                assert!(pruefung.kopf.antragsreferenz.is_some(), "{art}");
                let mut archiv = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
                assert!(archiv.by_name(&format!("{element}.xml")).is_ok(), "{art}");
                if art == "vorbescheid" {
                    vorbescheid = Some((vid, pruefung.kopf.id));
                }
                if art == "freistellung" {
                    let err = core
                        .verfahren_abgabe_festhalten(
                            p.id,
                            vid,
                            core.verfahrensakte(p.id, vid).await.unwrap().0.revision,
                            abgabe(Some(eingang_id)),
                            &chefin,
                        )
                        .await
                        .unwrap_err()
                        .to_string();
                    assert!(err.contains("keine geänderte Anzeige"), "{err}");
                }
            }
            Err(text) => {
                let err = ergebnis.unwrap_err().to_string();
                assert!(err.contains(text), "{art}: {err}");
            }
        }
    }
    // Nachreichung zum Vorbescheid auf eine Befundliste: 0212.
    let (vid, _) = vorbescheid.unwrap();
    let w = abgabe(Some(eingang_id));
    let aid = w.id;
    core.verfahren_abgabe_festhalten(
        p.id,
        vid,
        core.verfahrensakte(p.id, vid).await.unwrap().0.revision,
        w,
        &chefin,
    )
    .await
    .unwrap();
    let bytes = core
        .verfahren_artefakt(p.id, vid, aid, "xbau", &chefin)
        .await
        .unwrap()
        .1;
    let pruefung =
        core_lib::xbau::austausch::pruefen("XBau.zip", &bytes, core.xbau_pruefer.as_ref())
            .await
            .unwrap();
    assert_eq!(pruefung.kopf.element, "vorbescheid.antragGeaendert.0212");
    let akte = core.verfahrensakte(p.id, vid).await.unwrap().1;
    let stand = akte.abgaben.iter().find(|b| b.id == aid).unwrap();
    assert_eq!(
        stand.grundlagen.art,
        Some(core_lib::verfahren::Verfahrensart::Vorbescheid)
    );
    assert_eq!(stand.grundlagen.antragsangaben.vorbescheidfragen.len(), 1);

    // Behörde und Aktenzeichen stammen aus der Verfahrensakte, nicht aus dem
    // Bauantrag.
    let akte = |kennung: &str, aktenzeichen: &str| {
        let mut a = verfahren("bauantrag", serde_json::Value::Null);
        a["behoerde"]["kennung"] = kennung.into();
        a["aktenzeichen"] = aktenzeichen.into();
        a
    };
    let anlegen = |a: serde_json::Value| {
        let vid = uuid::Uuid::new_v4();
        let core = &core;
        let chefin = &chefin;
        async move {
            core.fachobjekt_speichern(
                p.id,
                core_lib::plattform::ObjektArt::Verfahren,
                vid,
                0,
                a,
                chefin,
            )
            .await
            .unwrap();
            vid
        }
    };
    let xml_von = |bytes: &[u8]| {
        let mut archiv = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let name = (0..archiv.len())
            .map(|i| archiv.by_index(i).unwrap().name().unwrap().to_string())
            .find(|n| n.ends_with(".xml"))
            .unwrap();
        let mut xml = String::new();
        archiv
            .by_name(&name)
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        xml
    };
    let vid = anlegen(akte("bab:anders", "BA-42")).await;
    let erst = abgabe(None);
    let erst_id = erst.id;
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, 1, erst, &chefin)
        .await
        .unwrap();
    let a: core_lib::verfahren::Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let stand = a.abgaben.iter().find(|b| b.id == erst_id).unwrap();
    assert!(
        stand.hinweise.iter().any(|h| h.contains(
            "Bauaufsicht im Bauantrag (bab:053150000) weicht von der Behörde der Verfahrensakte (bab:anders) ab"
        )),
        "{:?}",
        stand.hinweise
    );
    assert_eq!(stand.grundlagen.aktenzeichen.as_deref(), Some("BA-42"));
    let xml = xml_von(
        &core
            .verfahren_artefakt(p.id, vid, erst_id, "xbau", &chefin)
            .await
            .unwrap()
            .1,
    );
    assert!(
        xml.contains("<kennung>bab:anders</kennung>"),
        "Leser aus der Akte"
    );
    assert!(!xml.contains("bab:053150000"));

    // Ins Journal übernommen ist die Abgabe genau einmal, auch bei Wiederholung.
    let vorher = core.xbau_nachrichten(p.id, &chefin).await.unwrap().len();
    let n1 = core
        .verfahren_xbau_erfassen(p.id, vid, erst_id, &chefin)
        .await
        .unwrap();
    let n2 = core
        .verfahren_xbau_erfassen(p.id, vid, erst_id, &chefin)
        .await
        .unwrap();
    assert_eq!(n1.id, n2.id);
    assert_eq!(n1.nachricht.pruefung.kopf.id, erst_id);
    assert_eq!(
        core.xbau_nachrichten(p.id, &chefin).await.unwrap().len(),
        vorher + 1
    );

    // Eine Nachreichung auf den eigenen Antrag nennt das Aktenzeichen der Akte.
    let nach = abgabe(Some(n1.id));
    let nach_id = nach.id;
    core.verfahren_abgabe_festhalten(
        p.id,
        vid,
        core.verfahrensakte(p.id, vid).await.unwrap().0.revision,
        nach,
        &chefin,
    )
    .await
    .unwrap();
    let xml = xml_von(
        &core
            .verfahren_artefakt(p.id, vid, nach_id, "xbau", &chefin)
            .await
            .unwrap()
            .1,
    );
    assert!(xml.contains("<vorgang>BA-42</vorgang>"), "{xml}");

    // Ein abweichendes Aktenzeichen der Bezugsnachricht wird abgewiesen.
    let vid = anlegen(akte("bab:053150000", "BA-99")).await;
    let err = core
        .verfahren_abgabe_festhalten(p.id, vid, 1, abgabe(Some(eingang_id)), &chefin)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Aktenzeichen der Bezugsnachricht"), "{err}");

    // Ohne Behördenkennung in der Akte gibt es keine XBau-Ausgabe.
    let vid = anlegen(akte("", "")).await;
    let err = core
        .verfahren_abgabe_festhalten(p.id, vid, 1, abgabe(None), &chefin)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Behördenkennung (DVDV) in der Verfahrensakte"),
        "{err}"
    );
}

#[tokio::test]
async fn ungeeignete_anlagen_stehen_bei_den_fehlenden_angaben() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &chefin,
        )
        .await
        .unwrap();
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Grundriss.dxf",
            Bytes::from_static(b"0\nEOF\n"),
            &chefin,
        )
        .await
        .unwrap();
    let mut antrag = core.bauantrag(p.id).await.unwrap().antrag;
    antrag.anlagen.push(Anlagenwahl {
        dokument: plan.document_id,
        anlageart: "9999".into(),
        bezeichnung: String::new(),
    });
    let g = core
        .bauantrag_speichern(p.id, antrag, 0, &chefin)
        .await
        .unwrap();
    assert!(
        g.fehlende_angaben
            .iter()
            .any(|f| f.starts_with("Grundriss.dxf: Der Dateityp image/vnd.dxf")),
        "{:?}",
        g.fehlende_angaben
    );
    assert!(
        g.fehlende_angaben
            .iter()
            .any(|f| f == "Grundriss.dxf: Art der Anlage 9999 steht nicht in der Codeliste"),
        "{:?}",
        g.fehlende_angaben
    );
}

#[tokio::test]
async fn ohne_codelisten_nennt_der_export_die_fehlende_liste() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), false).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &chefin,
        )
        .await
        .unwrap();
    let g = core.bauantrag(p.id).await.unwrap();
    assert!(
        g.fehlende_angaben.iter().any(|f| f.contains(ANLAGEN)),
        "{:?}",
        g.fehlende_angaben
    );
}

/// Platzhalter-Landesliste (keine veröffentlichten Werte), nur für den Test.
const TEST_LANDESLISTE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gc:CodeList xmlns:gc="http://docs.oasis-open.org/codelist/ns/genericode/1.0/">
  <Identification><ShortName>verfahrensart</ShortName><LongName>Test-Verfahrensarten</LongName>
    <Version>0.1</Version><CanonicalUri>urn:xoev-de:xbau:nw:codeliste:verfahrensart</CanonicalUri></Identification>
  <ColumnSet>
    <Column Id="code" Use="required"><ShortName>code</ShortName></Column>
    <Column Id="beschreibung" Use="required"><ShortName>beschreibung</ShortName></Column>
    <Key Id="k"><ShortName>k</ShortName><ColumnRef Ref="code"/></Key>
  </ColumnSet>
  <SimpleCodeList>
    <Row><Value ColumnRef="code"><SimpleValue>T1</SimpleValue></Value><Value ColumnRef="beschreibung"><SimpleValue>Testverfahren</SimpleValue></Value></Row>
  </SimpleCodeList>
</gc:CodeList>"#;

#[tokio::test]
async fn eingespielte_landesliste_gilt_sofort_und_braucht_bueroverwaltung() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ben = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "ben".into(),
                anzeigename: "Ben".into(),
                rolle: Rolle::Bearbeitung,
            },
            &chefin,
        )
        .await
        .unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &chefin,
        )
        .await
        .unwrap();
    assert!(
        core.bauantrag(p.id)
            .await
            .unwrap()
            .verfahrensarten
            .is_none()
    );
    let mut abgelegt = None;
    assert!(matches!(
        core.xbau_codeliste_einspielen(TEST_LANDESLISTE, &ben, |_, _| Ok(()))
            .await,
        Err(CoreError::Forbidden(_))
    ));
    // Schlägt das Ablegen fehl, gilt die Liste nicht.
    assert!(
        core.xbau_codeliste_einspielen(TEST_LANDESLISTE, &chefin, |_, _| Err(
            CoreError::InvalidInput("Ordner schreibgeschützt".into())
        ))
        .await
        .is_err()
    );
    assert!(
        core.bauantrag(p.id)
            .await
            .unwrap()
            .verfahrensarten
            .is_none()
    );
    let liste = core
        .xbau_codeliste_einspielen(TEST_LANDESLISTE, &chefin, |datei, _| {
            abgelegt = Some(datei.to_string());
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        abgelegt.as_deref(),
        Some("urn_xoev-de_xbau_nw_codeliste_verfahrensart.xml")
    );
    let g = core.bauantrag(p.id).await.unwrap();
    assert_eq!(g.verfahrensarten.as_ref(), Some(&liste));
    let mut antrag = g.antrag;
    antrag.verfahrensart = "T1".into();
    let g = core
        .bauantrag_speichern(p.id, antrag, g.version, &chefin)
        .await
        .unwrap();
    assert!(
        !g.fehlende_angaben
            .iter()
            .any(|f| f.contains("Verfahrensart")),
        "{:?}",
        g.fehlende_angaben
    );
}

#[tokio::test]
async fn anlagenart_folgt_ohne_eigene_wahl_der_dokumentkategorie() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    // Anlagenliste mit dem Code, den die Kategorie „Lageplan“ vorschlägt.
    core.codelisten.hinzufuegen(Codeliste {
        uri: ANLAGEN.into(),
        version: "test".into(),
        name: "Anlagen".into(),
        eintraege: vec![Codeeintrag {
            code: "1803".into(),
            beschreibung: "Lageplan".into(),
        }],
    });
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &chefin,
        )
        .await
        .unwrap();
    let mut docs = Vec::new();
    for name in ["Lageplan.pdf", "Notiz.pdf"] {
        docs.push(
            core.store_document(
                p.id,
                None,
                None,
                name,
                Bytes::from_static(b"%PDF-1.7"),
                &chefin,
            )
            .await
            .unwrap(),
        );
    }
    core.dokumentkategorie_speichern(
        docs[0].document_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 0,
            kategorie: Some("lageplan".into()),
        },
        &chefin,
    )
    .await
    .unwrap();
    let g = core.bauantrag(p.id).await.unwrap();
    let v = &g.anlagenart_vorschlaege[&docs[0].document_id.0.to_string()];
    assert_eq!(
        (v.anlageart.as_str(), v.kategorie.as_str()),
        ("1803", "Lageplan")
    );
    assert!(
        !g.anlagenart_vorschlaege
            .contains_key(&docs[1].document_id.0.to_string())
    );
    let mut antrag = g.antrag;
    for d in &docs {
        antrag.anlagen.push(Anlagenwahl {
            dokument: d.document_id,
            anlageart: String::new(),
            bezeichnung: String::new(),
        });
    }
    let g = core
        .bauantrag_speichern(p.id, antrag, g.version, &chefin)
        .await
        .unwrap();
    let ohne_art: Vec<_> = g
        .fehlende_angaben
        .iter()
        .filter(|f| f.contains("Art der Anlage wählen"))
        .collect();
    // Nur die Notiz ohne Kategorie braucht noch eine Wahl; gespeichert bleibt leer.
    assert_eq!(
        ohne_art,
        ["2. Anlage: Art der Anlage wählen"],
        "{:?}",
        g.fehlende_angaben
    );
    assert!(g.antrag.anlagen[0].anlageart.is_empty());
}

#[tokio::test]
async fn buero_aendern_braucht_projektleitung() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let chefin = core.anmelden("chefin", "Clara Chefin").await.unwrap();
    let ben = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "ben".into(),
                anzeigename: "Ben".into(),
                rolle: Rolle::Bearbeitung,
            },
            &chefin,
        )
        .await
        .unwrap();
    assert!(matches!(
        core.buero_speichern(Buero::default(), &ben).await,
        Err(CoreError::Forbidden(_))
    ));
    assert_eq!(core.buero().await.unwrap(), Buero::default());
}

#[tokio::test]
async fn zentrale_flurstuecke_gelten_fuer_antrag_und_export() {
    use core_lib::domain::{Planungsrecht, ProjectUpdate};
    use core_lib::grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen};
    use core_lib::projektbereiche::ProjektBereich;
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let u = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Zentrale Flurstücke".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let fl = |id: &str, nummer: &str| FlurstuecksInfo {
        id: id.into(),
        gemarkung: "Lindenthal".into(),
        gemarkungsnummer: String::new(),
        flur: "3".into(),
        nummer: nummer.into(),
        flaeche_m2: 400.,
        grundbuchbezirk: String::new(),
        grundbuchblatt: String::new(),
        laufende_nummer: None,
        grundbuchauszug: None,
        eigentuemer: String::new(),
        eigentuemer_kunden: vec![],
        bemerkung: String::new(),
        rolle: Default::default(),
        planbindung: None,
        planflaeche_m2: None,
        flaeche_aus_plan: false,
        flaeche_offen: false,
    };
    let mut recht = p.planungsrecht.clone();
    recht.grundstuecksinformationen = Some(Grundstuecksinformationen {
        flurstuecke: vec![fl("a", "100/1"), fl("b", "101")],
    });
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name.clone(),
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: None,
                planungsrecht: Some(recht),
            },
            &u,
        )
        .await
        .unwrap();

    // Eine abweichende Eingabe im Antrag wird nicht gespeichert und nicht exportiert.
    let leer = core.bauantrag(p.id).await.unwrap();
    assert!(leer.flurstuecke_zentral);
    assert_eq!(leer.antrag.grundstueck.flurstuecke.len(), 2);
    assert_eq!(leer.antrag.grundstueck.flurstuecke[0].zaehler, "100");
    assert_eq!(leer.antrag.grundstueck.flurstuecke[0].nenner, "1");
    let mut antrag = leer.antrag;
    antrag.grundstueck.flurstuecke[0].gemarkung = "Falschgemarkung".into();
    antrag.grundstueck.flurstuecke[0].zaehler = "999".into();
    antrag.bauaufsicht = Bauaufsicht {
        name: "Bauordnungsamt Köln".into(),
        kennung: "bab:053150000".into(),
    };
    let plan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from_static(b"%PDF-1.7 Lageplan"),
            &u,
        )
        .await
        .unwrap();
    antrag.anlagen.push(Anlagenwahl {
        dokument: plan.document_id,
        anlageart: "0100".into(),
        bezeichnung: String::new(),
    });
    let g = core.bauantrag_speichern(p.id, antrag, 0, &u).await.unwrap();
    assert_eq!(g.antrag.grundstueck.flurstuecke[0].zaehler, "100");
    assert!(!g.fehlende_angaben.iter().any(|f| f.contains("Flurstück")));

    core.buero_speichern(buero("Architekturbüro Muster"), &u)
        .await
        .unwrap();
    let anna = core
        .kunde_anlegen(
            Kundendaten {
                vorname: "Anna".into(),
                familienname: "Müller".into(),
                anschrift: anschrift(),
                ..Kundendaten::default()
            },
            &u,
        )
        .await
        .unwrap();
    core.bauherren_setzen(p.id, vec![anna.id], p.version, &u)
        .await
        .unwrap();
    // Weitere Beteiligte und die Bauherrin als Eigentümerin des Flurstücks a.
    use core_lib::kunden::{Beteiligtenrolle, Beteiligtenzuordnung};
    let mut kunden = Vec::new();
    for (vorname, familienname) in [("Otto", "Vollmacht"), ("Erna", "Eigen")] {
        kunden.push(
            core.kunde_anlegen(
                Kundendaten {
                    vorname: vorname.into(),
                    familienname: familienname.into(),
                    telefon: "0221 4711".into(),
                    ..Kundendaten::default()
                },
                &u,
            )
            .await
            .unwrap(),
        );
    }
    let p = core.get_project(p.id).await.unwrap();
    core.beteiligte_setzen(
        p.id,
        vec![
            Beteiligtenzuordnung {
                rolle: Beteiligtenrolle::Bevollmaechtigter,
                kunde: kunden[0].id,
                fachrichtung: String::new(),
            },
            Beteiligtenzuordnung {
                rolle: Beteiligtenrolle::Grundstueckseigentuemer,
                kunde: kunden[1].id,
                fachrichtung: String::new(),
            },
        ],
        p.version,
        &u,
    )
    .await
    .unwrap();
    let p = core.get_project(p.id).await.unwrap();
    let mut recht = p.planungsrecht.clone();
    recht
        .grundstuecksinformationen
        .as_mut()
        .unwrap()
        .flurstuecke[0]
        .eigentuemer_kunden = vec![anna.id];
    core.projekt_bereich_speichern(
        p.clone(),
        ProjektBereich::Grundstuecksinformationen,
        ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: None,
            planungsrecht: Some(recht),
        },
        &u,
    )
    .await
    .unwrap();
    let bytes = xbau_abgabe(&core, p.id, plan.document_id, &u).await;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut xml = String::new();
    zip.by_name(NACHRICHT_DATEI)
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("<zaehler>100</zaehler>"), "{xml}");
    assert!(xml.contains("<nenner>1</nenner>"));
    assert!(xml.contains("<zaehler>101</zaehler>"));
    assert!(!xml.contains(">999<") && !xml.contains("Falschgemarkung"));
    let k: String = xml.lines().map(str::trim).collect();
    let bevollmaechtigt = &k[k.find("<xbau:bevollmaechtigter>").expect(&xml)..];
    assert!(bevollmaechtigt.contains("<name>Vollmacht</name>"));
    assert!(bevollmaechtigt.contains("<kennung>0221 4711</kennung><zusatz>Telefon</zusatz>"));
    // Eigentümer: die Bauherrin aus dem Flurstück und die Beteiligte Erna.
    assert_eq!(xml.matches("<xbau:weitereBeteiligte>").count(), 2, "{xml}");
    assert_eq!(xml.matches("<code>02</code>").count(), 2);
    assert!(k.contains(&format!(
        "<xbau:weitereBeteiligte><xbau:beteiligter><natuerlichePerson><id>{}</id>",
        anna.id
    )));
}

#[tokio::test]
async fn nachbarflurstueck_zaehlt_nicht_zum_antrag_und_zur_projektflaeche() {
    use core_lib::domain::{Planungsrecht, ProjectUpdate};
    use core_lib::grundstuecksinformationen::{
        FlurstuecksInfo, FlurstuecksRolle, Grundstuecksinformationen,
    };
    use core_lib::projektbereiche::ProjektBereich;
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path(), true).await;
    let u = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Mit Nachbar".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(9999.),
                planungsrecht: Planungsrecht::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let fl = |id: &str, nummer: &str, rolle| FlurstuecksInfo {
        id: id.into(),
        gemarkung: "Lindenthal".into(),
        gemarkungsnummer: String::new(),
        flur: "3".into(),
        nummer: nummer.into(),
        flaeche_m2: 400.,
        grundbuchbezirk: String::new(),
        grundbuchblatt: String::new(),
        laufende_nummer: None,
        grundbuchauszug: None,
        eigentuemer: String::new(),
        eigentuemer_kunden: vec![],
        bemerkung: String::new(),
        rolle,
        planbindung: None,
        planflaeche_m2: None,
        flaeche_aus_plan: false,
        flaeche_offen: false,
    };
    let mut recht = p.planungsrecht.clone();
    recht.grundstuecksinformationen = Some(Grundstuecksinformationen {
        flurstuecke: vec![
            fl("a", "100/1", FlurstuecksRolle::Vorhaben),
            fl("b", "100/2", FlurstuecksRolle::Vorhaben),
            fl("n", "102", FlurstuecksRolle::Nachbar),
        ],
    });
    let update = |p: &core_lib::domain::Project, flaeche, recht| ProjectUpdate {
        id: p.id,
        expected_version: p.version,
        name: p.name.clone(),
        bundesland: p.bundesland,
        grundstuecksflaeche_m2: flaeche,
        planungsrecht: recht,
    };
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            update(&p, p.grundstuecksflaeche_m2, Some(recht)),
            &u,
        )
        .await
        .unwrap();
    assert_eq!(
        p.grundstuecksflaeche_m2,
        Some(800.),
        "nur Vorhabenflurstücke"
    );
    assert_eq!(
        core.get_project(p.id).await.unwrap().grundstuecksflaeche_m2,
        Some(800.)
    );

    let antrag = core.bauantrag(p.id).await.unwrap().antrag;
    let nummern: Vec<_> = antrag
        .grundstueck
        .flurstuecke
        .iter()
        .map(|f| f.zaehler.as_str())
        .collect();
    assert_eq!(nummern, ["100", "100"]);

    // Die Fläche lässt sich nicht mehr unabhängig von den Flurstücken setzen.
    let err = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Projektdaten,
            update(&p, Some(1234.), None),
            &u,
        )
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("ergibt sich aus den Flurstücken"),
        "{err}"
    );

    // Neue Flurstücksnummern haben die Form Zähler/Nenner; ein Bestandswert
    // in anderer Form bleibt speicherbar und erscheint als Hinweis.
    let mut recht = p.planungsrecht.clone();
    recht
        .grundstuecksinformationen
        .as_mut()
        .unwrap()
        .flurstuecke
        .push(fl("z", "12 a", FlurstuecksRolle::Sonstiges));
    let err = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            update(&p, p.grundstuecksflaeche_m2, Some(recht.clone())),
            &u,
        )
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Lindenthal 12 a"), "{err}");
    let infos = recht.grundstuecksinformationen.as_mut().unwrap();
    infos.flurstuecke[2].nummer = "102".into();
    infos.flurstuecke[3].nummer = "12/3".into();
    infos.flurstuecke[0].gemarkungsnummer = "5318".into();
    let p = core
        .projekt_bereich_speichern(
            p.clone(),
            ProjektBereich::Grundstuecksinformationen,
            update(&p, p.grundstuecksflaeche_m2, Some(recht)),
            &u,
        )
        .await
        .unwrap();
    let g = core.bauantrag(p.id).await.unwrap();
    assert_eq!(g.antrag.grundstueck.flurstuecke[0].gemarkungsnummer, "5318");
    assert_eq!(
        g.hinweise,
        [
            "Flurstück Lindenthal 12/3: Flurstück außerhalb des Vorhabengrundstücks (Zuwegung, Baulastfläche o. Ä.). Klären, ob eine Baulast nötig ist; die Baulasterklärung des Eigentümers ist dann Bauvorlage."
        ]
    );
}

/// Büroangaben mit einer bauvorlageberechtigten Person.
/// Hält einen XBau-Abgabestand in einer neuen Verfahrensakte fest und liefert
/// dessen eingefrorenes Paket.
async fn xbau_abgabe(
    core: &CoreServices,
    projekt: core_lib::domain::ProjectId,
    dokument: core_lib::domain::DocumentId,
    wer: &core_lib::benutzer::Benutzer,
) -> Vec<u8> {
    let vid = uuid::Uuid::new_v4();
    core.fachobjekt_speichern(
        projekt,
        core_lib::plattform::ObjektArt::Verfahren,
        vid,
        0,
        serde_json::json!({"bezeichnung":"Bauantrag","art":"bauantrag","status":"vorbereitung","behoerde":{"id":uuid::Uuid::new_v4(),"name":"Bauamt","kennung":"bab:053150000","kontakt":""},"aktenzeichen":"","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]}),
        wer,
    )
    .await
    .unwrap();
    let id = uuid::Uuid::new_v4();
    core.verfahren_abgabe_festhalten(
        projekt,
        vid,
        1,
        core_lib::verfahren::abgabe::Abgabewunsch {
            xbau_bezug: None,
            xbau_folge: None,
            id,
            titel: "Bauantrag".into(),
            dokumente: vec![core_lib::plattform::objekte::Dokumentbezug {
                dokument_id: dokument,
                version: 1,
            }],
            xbau: true,
            druck: false,
            anschreiben: String::new(),
        },
        wer,
    )
    .await
    .unwrap();
    core.verfahren_artefakt(projekt, vid, id, "xbau", wer)
        .await
        .unwrap()
        .1
        .to_vec()
}

fn anschrift() -> Anschrift {
    Anschrift {
        strasse: "Aachener Straße".into(),
        hausnummer: "1".into(),
        postleitzahl: "50674".into(),
        ort: "Köln".into(),
    }
}

fn buero(name: &str) -> Buero {
    Buero {
        name: name.into(),
        anschrift: anschrift(),
        kennung: "arc:4711".into(),
        telefon: String::new(),
        email: String::new(),
        entwurfsverfasser: vec![Entwurfsverfasser {
            id: uuid::Uuid::new_v4(),
            vorname: "Eva".into(),
            familienname: "Entwurf".into(),
            berufsbezeichnung: "Architektin".into(),
            mitgliedsnummer: "12345".into(),
            kammerland: Some(Bundesland::By),
            nachweis: String::new(),
            telefon: String::new(),
            mobil: String::new(),
            email: String::new(),
        }],
    }
}
