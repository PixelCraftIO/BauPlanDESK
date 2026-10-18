//! Echte Schemaprüfung, immutable DMS-Versionen und Verfahrensgrenzen.
use bytes::Bytes;
use chrono::Utc;
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    xbau::{austausch::*, postfach::*},
};
use std::{
    io::{Cursor, Write},
    sync::Arc,
};
use uuid::Uuid;
fn nachricht(id: Uuid, referenz: Uuid) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
 <k:prozessnachrichten.eingangsbestaetigung.1120 xmlns:k="http://www.xleitstelle.de/xbau/kernmodul/1/4/0" produkt="Test" produkthersteller="Muster" standard="XBau-Kernmodul" version="1.4.0">
 <nachrichtenkopf.g2g><identifikation.nachricht><nachrichtenUUID>{id}</nachrichtenUUID><nachrichtentyp listURI="urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten" listVersionID="3.0"><code>1120</code></nachrichtentyp><erstellungszeitpunkt>2026-01-02T12:00:00Z</erstellungszeitpunkt></identifikation.nachricht><leser><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>planer:test</kennung><name>Büro &amp; Partner</name></leser><autor><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>bab:test</kennung><name>Bauaufsicht</name></autor></nachrichtenkopf.g2g><bezug><referenz>{referenz}</referenz><vorgang>BA-2026-42</vorgang></bezug></k:prozessnachrichten.eingangsbestaetigung.1120>"#
    )
}
fn paket(xml: &str, extras: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(vec![]));
    let opt = zip::write::SimpleFileOptions::default();
    z.start_file("nachricht.xml", opt).unwrap();
    z.write_all(xml.as_bytes()).unwrap();
    for (n, b) in extras {
        z.start_file(*n, opt).unwrap();
        z.write_all(b).unwrap();
    }
    z.finish().unwrap().into_inner()
}
fn mit_anlage(id: Uuid, referenz: Uuid, inhalt: &[u8]) -> String {
    let hash = core_lib::storage::ContentHash::of(inhalt);
    let anlage = format!(
        r#"<anlagen><anlage><bezeichnung>Prüfplan</bezeichnung><datum>2026-01-02</datum><anlageart listURI="urn:xoev-de:xbau:codeliste:anlagen" listVersionID="test"><code>0100</code></anlageart><mimeType listURI="urn:xoev-de:xbau:codeliste:xbau-mimetypes" listVersionID="test"><code>application/pdf</code></mimeType><anhangOderVerlinkung><anhang hashValue="{hash}" filesize="{}"><dokumentid>urn:uuid:{}</dokumentid><dateiname>Pruefplan.pdf</dateiname></anhang></anhangOderVerlinkung></anlage></anlagen>"#,
        inhalt.len(),
        Uuid::new_v4()
    );
    nachricht(id, referenz)
        .replace("eingangsbestaetigung.1120", "ruecknahme.1130")
        .replace("<code>1120</code>", "<code>1130</code>")
        .replace(
            "</k:prozessnachrichten.ruecknahme.1130>",
            &format!("{anlage}</k:prozessnachrichten.ruecknahme.1130>"),
        )
}
#[tokio::test]
async fn anlagen_integritaet_vollstaendigkeit_und_wiederholbare_uebernahme() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard());
    let u = core.anmelden("admin", "Admin").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Anlagen".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let content = b"%PDF-Anlagentest";
    let xml = mit_anlage(Uuid::new_v4(), Uuid::new_v4(), content);
    let bytes = paket(&xml, &[("Pruefplan.pdf", content)]);
    let result = pruefen("Anlage.zip", &bytes, core.xbau_pruefer.as_ref())
        .await
        .unwrap();
    assert_eq!(result.anlagen.len(), 1);
    assert!(result.anlagen[0].hash_in_nachricht);
    assert_eq!(result.anlagen[0].bytes, content.len() as u64);
    for bad in [
        paket(&xml, &[]),
        paket(&xml, &[("Pruefplan.pdf", b"falscher Inhalt")]),
        paket(
            &xml.replace("filesize=\"17\"", "filesize=\"1\""),
            &[("Pruefplan.pdf", content), ("extra.txt", b"a")],
        ),
    ] {
        assert!(
            pruefen("Falsch.zip", &bad, core.xbau_pruefer.as_ref())
                .await
                .is_err()
        );
    }
    let d = core
        .store_document(p.id, None, None, "Nachricht.zip", Bytes::from(bytes), &u)
        .await
        .unwrap();
    let n = core
        .xbau_aufnehmen(
            p.id,
            Nachrichtenaufnahme {
                original: Dokumentbezug {
                    dokument_id: d.document_id,
                    version: 1,
                },
                richtung: Richtung::Eingang,
                zuordnung_bestaetigt: true,
            },
            &u,
        )
        .await
        .unwrap();
    let a = core
        .xbau_anlage_ablegen(p.id, n.id, "Pruefplan.pdf", &u)
        .await
        .unwrap();
    let wieder = core
        .xbau_anlage_ablegen(p.id, n.id, "Pruefplan.pdf", &u)
        .await
        .unwrap();
    assert_eq!(a.document_id, wieder.document_id);
    assert_eq!(wieder.version, 1);
    assert_eq!(core.document_history(a.document_id).await.unwrap().len(), 1);
    assert!(
        core.repos
            .commit
            .dokumentzusatz(a.document_id, 1, "xbau_anlagenquelle")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        core.xbau_anlage_ablegen(p.id, n.id, "../fremd", &u)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn schemata_pruefen_kerndaten_und_wehren_manipulation_ab() {
    let engine = storage::xbau::XmlLintPruefer::standard();
    let id = Uuid::new_v4();
    let reference = Uuid::new_v4();
    let xml = nachricht(id, reference);
    let b = pruefen("eingang.xml", xml.as_bytes(), engine.as_ref())
        .await
        .unwrap();
    assert_eq!(b.kopf.id, id);
    assert_eq!(b.kopf.antragsreferenz, Some(reference));
    assert_eq!(b.kopf.aktenzeichen.as_deref(), Some("BA-2026-42"));
    assert_eq!(b.kopf.absender, "bab:test");
    assert!(b.anlagen.is_empty());
    for kaputt in [
        xml.replace("<bezug>", "<bezug><unbekannt>1</unbekannt>"),
        xml.replace("<code>1120</code>", "<code>0200</code>"),
        xml.replace("1/4/0", "1/3/0"),
        xml.replace("<referenz>", "<referenz xmlns='urn:falsch'>"),
        xml.replace("</bezug>", "</bezug><bezug/>"),
        format!("{xml}<x/>"),
        format!("<!DOCTYPE x [<!ENTITY geheim SYSTEM 'file:///etc/passwd'>]>{xml}"),
        xml.replace("<name>Bauaufsicht</name>", "<name>&evil;</name>"),
        xml.replace(
            "</bezug>",
            "</bezug><?xml-stylesheet href='https://example.invalid/'?>",
        ),
    ] {
        assert!(
            pruefen("kaputt.xml", kaputt.as_bytes(), engine.as_ref())
                .await
                .is_err()
        );
    }
    assert!(
        pruefen(
            "n.zip",
            &paket(&xml, &[("../fremd.pdf", b"a")]),
            engine.as_ref()
        )
        .await
        .is_err()
    );
    assert!(
        pruefen(
            "n.zip",
            &paket(&xml, &[("unreferenziert.pdf", b"a")]),
            engine.as_ref()
        )
        .await
        .is_err()
    );
    assert!(
        pruefen(
            "n.zip",
            &paket(&xml, &[("zweite.xml", xml.as_bytes())]),
            engine.as_ref()
        )
        .await
        .is_err()
    );
    let fehlt = pruefen("n.xml", xml.as_bytes(), &KeinSchemaPruefer)
        .await
        .unwrap_err();
    assert!(fehlt.to_string().contains("nicht eingerichtet"));
    let offline = storage::xbau::XmlLintPruefer::neu("/gibt-es-nicht/xmllint".into());
    assert!(
        pruefen("n.xml", xml.as_bytes(), &offline)
            .await
            .unwrap_err()
            .to_string()
            .contains("nicht verfügbar")
    );
    let bundle = pruefen("n.zip", &paket(&xml, &[]), engine.as_ref())
        .await
        .unwrap();
    assert_eq!(bundle.kopf, b.kopf);
    let catalog = katalog();
    assert_eq!(catalog.len(), 137);
    let set: std::collections::HashSet<_> = catalog
        .iter()
        .map(|n| (&n.namensraum, &n.element))
        .collect();
    assert_eq!(set.len(), 137);
}
#[tokio::test]
async fn nachrichten_journal_rechte_historie_belege_und_verfahrensakte() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard());
    let admin = core.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let leser = core.anmelden("leser", "Leser").await.unwrap();
    let make = || NewProject {
        name: "XBau Musterprojekt".into(),
        bundesland: Bundesland::Nw,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let p = core.create_project(make(), &admin).await.unwrap();
    let fremd = core.create_project(make(), &admin).await.unwrap();
    let id = Uuid::new_v4();
    let xml = nachricht(id, Uuid::new_v4());
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Eingang.xml",
            Bytes::from(xml.clone()),
            &admin,
        )
        .await
        .unwrap();
    let original = Dokumentbezug {
        dokument_id: d.document_id,
        version: d.version,
    };
    assert!(matches!(
        core.xbau_pruefen(p.id, &original, &leser).await,
        Err(CoreError::Forbidden(_))
    ));
    core.projektmitglied_setzen(
        p.id,
        core_lib::zugriff::ProjektMitglied {
            benutzer_id: leser.id,
            recht: core_lib::zugriff::ProjektRecht::Lesen,
        },
        0,
        &admin,
    )
    .await
    .unwrap();
    core.xbau_pruefen(p.id, &original, &leser).await.unwrap();
    assert!(
        core.xbau_pruefen(fremd.id, &original, &admin)
            .await
            .is_err()
    );
    let w = Nachrichtenaufnahme {
        original: original.clone(),
        richtung: Richtung::Eingang,
        zuordnung_bestaetigt: true,
    };
    assert!(core.xbau_aufnehmen(p.id, w.clone(), &leser).await.is_err());
    let mut unbestaetigt = w.clone();
    unbestaetigt.zuordnung_bestaetigt = false;
    assert!(
        core.xbau_aufnehmen(p.id, unbestaetigt, &admin)
            .await
            .is_err()
    );
    let n = core.xbau_aufnehmen(p.id, w.clone(), &admin).await.unwrap();
    assert_eq!(n.revision, 1);
    assert_eq!(
        core.xbau_aufnehmen(p.id, w.clone(), &admin)
            .await
            .unwrap()
            .revision,
        1
    );
    let mut falsch = w.clone();
    falsch.richtung = Richtung::Ausgang;
    assert!(core.xbau_aufnehmen(p.id, falsch, &admin).await.is_err());
    let d2 = core
        .store_document(
            p.id,
            Some(d.document_id),
            Some(1),
            "Eingang.xml",
            Bytes::from(xml.replace("Bauaufsicht", "Manipuliert")),
            &admin,
        )
        .await
        .unwrap();
    assert!(
        core.xbau_aufnehmen(
            p.id,
            Nachrichtenaufnahme {
                original: Dokumentbezug {
                    dokument_id: d2.document_id,
                    version: 2
                },
                ..w
            },
            &admin
        )
        .await
        .is_err()
    );
    let beleg = core
        .store_document(
            p.id,
            None,
            None,
            "Portalbeleg.txt",
            Bytes::from_static(b"Quittung Test"),
            &admin,
        )
        .await
        .unwrap();
    let mut b = Nachrichtenbeleg {
        id: Uuid::new_v4(),
        art: Belegart::Transportquittung,
        datum: Utc::now(),
        kanal: "Testportal".into(),
        referenz: "test-42".into(),
        dokument: original.clone(),
        notiz: "Nur Test".into(),
        berichtigt: None,
    };
    assert!(
        core.xbau_beleg(p.id, id, 1, b.clone(), &admin)
            .await
            .is_err()
    );
    b.dokument = Dokumentbezug {
        dokument_id: beleg.document_id,
        version: 1,
    };
    let n = core
        .xbau_beleg(p.id, id, 1, b.clone(), &admin)
        .await
        .unwrap();
    assert_eq!(n.revision, 2);
    assert_eq!(
        core.xbau_beleg(p.id, id, 1, b.clone(), &admin)
            .await
            .unwrap()
            .revision,
        2
    );
    let mut annahme = b.clone();
    annahme.id = Uuid::new_v4();
    annahme.art = Belegart::FachlicheAnnahme;
    assert!(matches!(
        core.xbau_beleg(p.id, id, 1, annahme.clone(), &admin).await,
        Err(CoreError::VersionConflict { .. })
    ));
    annahme.berichtigt = Some(b.id);
    let n = core
        .xbau_beleg(p.id, id, 2, annahme.clone(), &admin)
        .await
        .unwrap();
    assert_eq!(n.nachricht.belege.len(), 2);
    let mut doppelt = annahme.clone();
    doppelt.id = Uuid::new_v4();
    assert!(core.xbau_beleg(p.id, id, 3, doppelt, &admin).await.is_err());
    let mut akte:core_lib::verfahren::Verfahrensakte=serde_json::from_value(serde_json::json!({"bezeichnung":"Bauantrag Muster","art":"bauantrag","status":"vorbereitung","behoerde":{"id":Uuid::new_v4(),"name":"Bauaufsicht","kennung":"bab:test","kontakt":""},"aktenzeichen":"BA-2026-42","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]})).unwrap();
    let vid = Uuid::new_v4();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid,
        0,
        serde_json::to_value(&akte).unwrap(),
        &admin,
    )
    .await
    .unwrap();
    let heute = core_lib::kalender::heute();
    // Zugang nie in der Zukunft und nie vor der Erstellung der Nachricht.
    for falsch in [
        heute + chrono::Duration::days(1),
        "2026-01-01".parse().unwrap(),
    ] {
        assert!(
            core.xbau_in_verfahren(p.id, id, vid, 1, Some(falsch), &admin)
                .await
                .is_err()
        );
    }
    let f = core
        .xbau_in_verfahren(p.id, id, vid, 1, None, &admin)
        .await
        .unwrap();
    akte = serde_json::from_value(f.daten.clone()).unwrap();
    assert_eq!(akte.kommunikation.len(), 1);
    // Ohne bestätigten Zugang gilt der Tag der Erfassung, nicht das XML-Erstelldatum.
    assert_eq!(akte.kommunikation[0].datum, heute);
    assert!(akte.kommunikation[0].inhalt.contains("Tag der Erfassung"));
    assert_eq!(akte.kommunikation[0].original, Some(original));
    assert!(akte.abgaben.is_empty());
    assert!(akte.bescheide.is_empty());
    assert!(akte.nachforderungen.is_empty());
    assert_eq!(
        core.xbau_in_verfahren(p.id, id, vid, 1, None, &admin)
            .await
            .unwrap()
            .revision,
        f.revision
    );
    // Eine zweite Akte ohne Aktenzeichen übernimmt das der Behörde aus der
    // Nachricht; dieselbe Nachricht kommt nicht ein zweites Mal in eine Akte.
    let mut zweite = akte.clone();
    zweite.bezeichnung = "Zweites Verfahren".into();
    zweite.aktenzeichen = String::new();
    zweite.kommunikation.clear();
    zweite.ereignisse.clear();
    let vid2 = Uuid::new_v4();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid2,
        0,
        serde_json::to_value(&zweite).unwrap(),
        &admin,
    )
    .await
    .unwrap();
    let doppelt = core
        .xbau_in_verfahren(p.id, id, vid2, 1, None, &admin)
        .await
        .unwrap_err();
    assert!(doppelt.to_string().contains("bereits"), "{doppelt}");
    let id2 = Uuid::new_v4();
    let d3 = core
        .store_document(
            p.id,
            None,
            None,
            "Eingang2.xml",
            Bytes::from(nachricht(id2, Uuid::new_v4())),
            &admin,
        )
        .await
        .unwrap();
    core.xbau_aufnehmen(
        p.id,
        Nachrichtenaufnahme {
            original: Dokumentbezug {
                dokument_id: d3.document_id,
                version: d3.version,
            },
            richtung: Richtung::Eingang,
            zuordnung_bestaetigt: true,
        },
        &admin,
    )
    .await
    .unwrap();
    let f2 = core
        .xbau_in_verfahren(p.id, id2, vid2, 1, Some(heute), &admin)
        .await
        .unwrap();
    let zweite: core_lib::verfahren::Verfahrensakte = serde_json::from_value(f2.daten).unwrap();
    assert_eq!(zweite.aktenzeichen, "BA-2026-42");
    assert_eq!(zweite.kommunikation[0].datum, heute);
    assert!(zweite.kommunikation[0].inhalt.contains("(bestätigt)"));
    assert!(
        zweite
            .ereignisse
            .last()
            .unwrap()
            .text
            .contains("Aktenzeichen: – → BA-2026-42")
    );
    assert_eq!(
        core.repos
            .commit
            .fachverlauf(p.id, "xbau_nachrichten", &id.to_string())
            .await
            .unwrap()
            .len(),
        3
    );
    let export = core.verfahren_aktenexport(p.id).await.unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(export)).unwrap();
    let mut json = String::new();
    use std::io::Read;
    z.by_name("Projektakte.json")
        .unwrap()
        .read_to_string(&mut json)
        .unwrap();
    assert!(json.contains("xbau_nachrichten"));
    // Die Oberflächenprüfung arbeitet mit der ersten Nachricht samt Beleg.
    if let Ok(out) = std::env::var("BAUPROJEKT_XBAU_ARTEFAKTE") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"benutzer":admin,"leser":leser,"nachrichten":core.xbau_nachrichten(p.id,&admin).await.unwrap().into_iter().filter(|m| m.id==id).collect::<Vec<_>>(),"dateien":core.list_documents(p.id).await.unwrap(),"verfahren":[f],"typen":katalog(),"pruefung":n.nachricht.pruefung})).unwrap()).unwrap();
    }
}
