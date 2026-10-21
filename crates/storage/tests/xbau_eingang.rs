//! Behördennachrichten im Journal: Auswertung, Vorschau und wiederholbare
//! fachliche Übernahme in die Verfahrensakte (mit echter Schemaprüfung).
use base64::Engine as _;
use bytes::Bytes;
use core_lib::{
    CoreServices,
    benutzer::Benutzer,
    domain::{Bundesland, NewProject, ProjectId},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    verfahren::{
        Entscheidung, Nebenbestimmungsart, Punktstatus, Schrittart, Verfahrensakte,
        Verfahrensstatus,
        abgabe::{Abgabewunsch, Versandstatus, versandstand},
        abweichungen::{Abweichung, Abweichungsart},
        eingang::Uebernahmewunsch,
        termine::{Terminart, termine},
    },
    xbau::{eingang::Auswertung, postfach::*},
};
use std::{
    io::{Cursor, Write},
    sync::Arc,
};
use uuid::Uuid;

const BEHOERDE: &str = r#"<leser><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>planer:test</kennung><name>Büro</name></leser><autor><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>bab:test</kennung><name>Bauaufsicht</name></autor>"#;
fn kopf(id: Uuid, kern: bool, code: &str) -> String {
    let liste = if kern {
        r#"listURI="urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten" listVersionID="3.0""#
    } else {
        r#"listURI="urn:xoev-de:xbau:codeliste:xbau-nachrichten" listVersionID="5.0""#
    };
    format!(
        r#"<nachrichtenkopf.g2g><identifikation.nachricht><nachrichtenUUID>{id}</nachrichtenUUID><nachrichtentyp {liste}><code>{code}</code></nachrichtentyp><erstellungszeitpunkt>2026-05-04T10:00:00Z</erstellungszeitpunkt></identifikation.nachricht>{BEHOERDE}</nachrichtenkopf.g2g>"#
    )
}
const KERN: &str = r#"xmlns:k="http://www.xleitstelle.de/xbau/kernmodul/1/4/0" produkt="Test" produkthersteller="Muster" standard="XBau-Kernmodul" version="1.4.0""#;
const HOCHBAU: &str = r#"xmlns:xbau="http://www.xleitstelle.de/xbau/2/7" produkt="Test" produkthersteller="Muster" standard="XBau-Hochbau" version="2.7""#;
const REFERENZ: &str = "4a0c8f2e-3c1b-4d5e-8f6a-1b2c3d4e5f60";

fn eingangsbestaetigung(id: Uuid, abgabe: Uuid) -> String {
    format!(
        r#"<k:prozessnachrichten.eingangsbestaetigung.1120 {KERN} xmlns:bn="http://xoev.de/schemata/basisnachricht/g2g/1_1">{}<bezug><referenz>{REFERENZ}</referenz><vorgang>BA-2026-42</vorgang><bezugNachricht><bn:nachrichtenUUID>{abgabe}</bn:nachrichtenUUID><bn:nachrichtentyp listURI="urn:xoev-de:xbau:codeliste:xbau-nachrichten" listVersionID="5.0"><code>0200</code></bn:nachrichtentyp><bn:erstellungszeitpunkt>2026-05-01T08:00:00Z</bn:erstellungszeitpunkt></bezugNachricht></bezug></k:prozessnachrichten.eingangsbestaetigung.1120>"#,
        kopf(id, true, "1120")
    )
}
fn befundliste(id: Uuid) -> String {
    format!(
        r#"<k:prozessnachrichten.formellePruefungBefundliste.1140 {KERN}>{}<bezug><referenz>{REFERENZ}</referenz><vorgang>BA-2026-42</vorgang></bezug><dokumentationVollstaendig>false</dokumentationVollstaendig><befundFrist><befundliste><befund><artDesBefundes listURI="urn:test:befundart" listVersionID="1"><code>01</code></artDesBefundes><beschreibung><textabsatz>Standsicherheitsnachweis fehlt</textabsatz></beschreibung></befund><befund><artDesBefundes listURI="urn:test:befundart" listVersionID="1"><code>02</code></artDesBefundes><beschreibung><textabsatz>Lageplan ohne Unterschrift</textabsatz></beschreibung></befund></befundliste><frist>2026-12-15</frist></befundFrist></k:prozessnachrichten.formellePruefungBefundliste.1140>"#,
        kopf(id, true, "1140")
    )
}
fn anhoerung(id: Uuid) -> String {
    format!(
        r#"<xbau:baugenehmigung.anhoerung.0203 {HOCHBAU}>{}<xbau:bezug><referenz>{REFERENZ}</referenz><vorgang>BA-2026-42</vorgang></xbau:bezug><xbau:aufforderungZurStellungnahme><textabsatz>Wir beabsichtigen, den Antrag abzulehnen.</textabsatz></xbau:aufforderungZurStellungnahme><xbau:bescheidlage><xbau:befund><xbau:rechtsbereich listURI="urn:test:rechtsbereich" listVersionID="1"><code>21</code></xbau:rechtsbereich><xbau:bezeichnungRegelverstoss>Überschreitung der GRZ</xbau:bezeichnungRegelverstoss><xbau:begruendung>GRZ 0,45 statt 0,4</xbau:begruendung></xbau:befund></xbau:bescheidlage><xbau:frist>2026-12-01</xbau:frist></xbau:baugenehmigung.anhoerung.0203>"#,
        kopf(id, false, "0203")
    )
}
fn bescheid(id: Uuid, abweichung: Uuid) -> String {
    let b64 = |t: &str| base64::engine::general_purpose::STANDARD.encode(t);
    format!(
        r#"<xbau:baugenehmigung.bescheid.0205 {HOCHBAU}>{}<xbau:bezug><referenz>{REFERENZ}</referenz><vorgang>BA-2026-42</vorgang></xbau:bezug><xbau:bescheid><xbau:tenor><xbau:istGenehmigung>true</xbau:istGenehmigung></xbau:tenor><xbau:abweichung><xbau:id>{abweichung}</xbau:id><xbau:istZugelassen>true</xbau:istZugelassen><xbau:art listURI="urn:xoev-de:xbau:codeliste:abweichungart" listVersionID="2.0"><code>1</code></xbau:art><xbau:vorschrift><text>{}</text></xbau:vorschrift><xbau:nebenbestimmung><text>{}</text></xbau:nebenbestimmung></xbau:abweichung><xbau:nebenbestimmung><xbau:art listURI="urn:xoev-de:xbau:codeliste:nebenbestimmungart" listVersionID="3.0"><code>2</code></xbau:art><xbau:rechtsbereich listURI="urn:test:rechtsbereich" listVersionID="1"><code>21</code></xbau:rechtsbereich><xbau:nebenbestimmung><text>{}</text></xbau:nebenbestimmung></xbau:nebenbestimmung><xbau:rechtshelfsbelehrung><textabsatz>Klage binnen eines Monats.</textabsatz></xbau:rechtshelfsbelehrung></xbau:bescheid><xbau:anlagen><anlage><bezeichnung>Bescheid</bezeichnung><versionsnummer>1</versionsnummer><datum>2026-05-04</datum><anlageart listURI="urn:xoev-de:xbau:codeliste:anlagen" listVersionID="4.0"><code>9999</code></anlageart><mimeType listURI="urn:xoev-de:xbau:codeliste:xbau-mimetypes" listVersionID="1"><code>application/pdf</code></mimeType><anhangOderVerlinkung><anhang><dokumentid>urn:uuid:{}</dokumentid><dateiname>Bescheid.pdf</dateiname></anhang></anhangOderVerlinkung></anlage></xbau:anlagen></xbau:baugenehmigung.bescheid.0205>"#,
        kopf(id, false, "0205"),
        b64("<p>§ 6 BauO NRW</p>"),
        b64("<p>Die Garage bleibt ohne Aufenthaltsraum.</p>"),
        b64("<p>Vor Baubeginn ist der geprüfte Standsicherheitsnachweis vorzulegen.</p>"),
        Uuid::new_v4()
    )
}
fn gebuehren(id: Uuid) -> String {
    format!(
        r#"<k:prozessnachrichten.gebuehrenbescheid.1160 {KERN}>{}<bezug><referenz>{REFERENZ}</referenz><vorgang>BA-2026-42</vorgang></bezug><gebuehrenbescheid><istVorauszahlungsbescheid>false</istVorauszahlungsbescheid><begruendung><textabsatz>Gebühren für die Baugenehmigung</textabsatz></begruendung><kostenberechnung><gebuehrenposition><gebuehrenordnung>AVwGebO NRW</gebuehrenordnung><gebuehrentatbestand>Tarifstelle 2.4.1.1</gebuehrentatbestand><position><erlaeuterung>Rohbausumme x Satz</erlaeuterung><betrag>123450</betrag></position></gebuehrenposition><summe>123450</summe></kostenberechnung><zahlungsdaten><iban>DE02120300000000202051</iban><verwendungszweck>BA-2026-42</verwendungszweck></zahlungsdaten><rechtshelfsbelehrung><textabsatz>Klage binnen eines Monats.</textabsatz></rechtshelfsbelehrung></gebuehrenbescheid><frist>2026-12-04</frist></k:prozessnachrichten.gebuehrenbescheid.1160>"#,
        kopf(id, true, "1160")
    )
}
fn paket(xml: &str, anlage: (&str, &[u8])) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(vec![]));
    let opt = zip::write::SimpleFileOptions::default();
    z.start_file("nachricht.xml", opt).unwrap();
    z.write_all(xml.as_bytes()).unwrap();
    z.start_file(anlage.0, opt).unwrap();
    z.write_all(anlage.1).unwrap();
    z.finish().unwrap().into_inner()
}
async fn aufnehmen(
    core: &CoreServices,
    p: ProjectId,
    name: &str,
    daten: Vec<u8>,
    u: &Benutzer,
) -> Uuid {
    let d = core
        .store_document(p, None, None, name, Bytes::from(daten), u)
        .await
        .unwrap();
    core.xbau_aufnehmen(
        p,
        Nachrichtenaufnahme {
            original: Dokumentbezug {
                dokument_id: d.document_id,
                version: d.version,
            },
            richtung: Richtung::Eingang,
            zuordnung_bestaetigt: true,
        },
        u,
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn behoerdennachrichten_werden_fachlich_uebernommen() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    )
    .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard());
    let u = core.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Eingang".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap()
        .id;
    let lageplan = core
        .store_document(
            p,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &u,
        )
        .await
        .unwrap();
    let abw_id = Uuid::new_v4();
    let mut akte: Verfahrensakte = serde_json::from_value(serde_json::json!({
        "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "bab:test", "kontakt": ""},
        "aktenzeichen": "", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    }))
    .unwrap();
    akte.abweichungen.push(Abweichung {
        id: abw_id,
        art: Abweichungsart::Abweichung,
        vorschrift: "§ 6 BauO NRW".into(),
        rechtsgrundlage: "§ 69 BauO NRW".into(),
        gegenstand: "Garage Nordwand".into(),
        soll: "3,00 m".into(),
        ist: "2,50 m".into(),
        begruendung: "Garage ohne Aufenthaltsräume".into(),
        herkunft: None,
        nachbarn: vec![],
        dokumente: vec![],
        xbau_code: String::new(),
        entfallen: None,
    });
    let vid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p,
            ObjektArt::Verfahren,
            vid,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let abgabe = Uuid::new_v4();
    let f = core
        .verfahren_abgabe_festhalten(
            p,
            vid,
            f.revision,
            Abgabewunsch {
                xbau_bezug: None,
                id: abgabe,
                titel: "Bauantrag".into(),
                dokumente: vec![Dokumentbezug {
                    dokument_id: lageplan.document_id,
                    version: lageplan.version,
                }],
                xbau: false,
                druck: false,
                anschreiben: String::new(),
            },
            &u,
        )
        .await
        .unwrap();
    let wunsch = |basis, genehmigte: Vec<Dokumentbezug>| Uebernahmewunsch {
        verfahren: vid,
        basis,
        zugang: None,
        genehmigte_unterlagen: genehmigte,
    };

    // 1120: Eingang des Abgabestands mit der Nachricht als Beleg.
    let n1120 = aufnehmen(
        &core,
        p,
        "Eingang.xml",
        eingangsbestaetigung(Uuid::new_v4(), abgabe).into_bytes(),
        &u,
    )
    .await;
    let vorschau = core.xbau_auswertung(p, n1120, Some(vid), &u).await.unwrap();
    assert!(matches!(
        vorschau.auswertung,
        Auswertung::Eingangsbestaetigung { .. }
    ));
    assert!(
        vorschau.wirkung.iter().any(|w| w.contains("Eingang von")),
        "{:?}",
        vorschau.wirkung
    );
    let f = core
        .xbau_auswertung_uebernehmen(p, n1120, wunsch(f.revision, vec![]), &u)
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    assert_eq!(
        versandstand(&a.abgaben[0]),
        Some(Versandstatus::Eingegangen)
    );
    assert_eq!(a.status, Verfahrensstatus::Eingereicht);
    // Wiederholt: keine weitere Änderung.
    let f2 = core
        .xbau_auswertung_uebernehmen(p, n1120, wunsch(f.revision, vec![]), &u)
        .await
        .unwrap();
    assert_eq!(f2.revision, f.revision);
    assert!(
        core.xbau_auswertung(p, n1120, Some(vid), &u)
            .await
            .unwrap()
            .uebernommen
    );

    // 1140: Nachforderung mit Frist und Hinweis auf die Rücknahmefiktion.
    let n1140 = aufnehmen(
        &core,
        p,
        "Befunde.xml",
        befundliste(Uuid::new_v4()).into_bytes(),
        &u,
    )
    .await;
    let f = core
        .xbau_auswertung_uebernehmen(p, n1140, wunsch(f.revision, vec![]), &u)
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    assert_eq!(a.status, Verfahrensstatus::Unvollstaendig);
    let punkte = &a.nachforderungen[0].punkte;
    assert_eq!(punkte.len(), 2);
    assert_eq!(punkte[0].titel, "Standsicherheitsnachweis fehlt");
    assert_eq!(punkte[0].status, Punktstatus::Offen);
    let frist = punkte[0].frist.as_ref().unwrap();
    assert_eq!(frist.datum, "2026-12-15".parse().unwrap());
    assert!(frist.grund.contains("§ 71 BauO NRW"), "{}", frist.grund);

    // 0203: Anhörung mit Frist und Beanstandung.
    let n0203 = aufnehmen(
        &core,
        p,
        "Anhoerung.xml",
        anhoerung(Uuid::new_v4()).into_bytes(),
        &u,
    )
    .await;
    let f = core
        .xbau_auswertung_uebernehmen(p, n0203, wunsch(f.revision, vec![]), &u)
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    let s = &a.schritte[0];
    assert_eq!(s.art, Schrittart::Anhoerung);
    assert_eq!(
        s.frist.as_ref().unwrap().datum,
        "2026-12-01".parse().unwrap()
    );
    assert_eq!(s.gruende[0].text, "Überschreitung der GRZ");

    // 1160: Gebührenbescheid mit Zahlungsfrist als Wiedervorlage.
    let n1160 = aufnehmen(
        &core,
        p,
        "Gebuehren.xml",
        gebuehren(Uuid::new_v4()).into_bytes(),
        &u,
    )
    .await;
    let f = core
        .xbau_auswertung_uebernehmen(p, n1160, wunsch(f.revision, vec![]), &u)
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    assert_eq!(a.gebuehren[0].betrag_cent, Some(123450));
    assert!(
        termine(&a, vid, Bundesland::Nw)
            .iter()
            .any(|t| t.art == Terminart::Zahlung && t.datum == "2026-12-04".parse().unwrap())
    );

    // 0205: Bescheid; die genehmigten Unterlagen werden ausdrücklich bestätigt.
    let n0205 = aufnehmen(
        &core,
        p,
        "Bescheid.zip",
        paket(
            &bescheid(Uuid::new_v4(), abw_id),
            ("Bescheid.pdf", b"%PDF-1.4 Bescheid"),
        ),
        &u,
    )
    .await;
    let vorschau = core.xbau_auswertung(p, n0205, Some(vid), &u).await.unwrap();
    assert_eq!(
        vorschau.genehmigte_unterlagen,
        vec![Dokumentbezug {
            dokument_id: lageplan.document_id,
            version: lageplan.version
        }]
    );
    assert!(
        core.xbau_auswertung_uebernehmen(p, n0205, wunsch(f.revision, vec![]), &u)
            .await
            .unwrap_err()
            .to_string()
            .contains("genehmigten Unterlagen")
    );
    core.xbau_auswertung_uebernehmen(
        p,
        n0205,
        wunsch(f.revision, vorschau.genehmigte_unterlagen.clone()),
        &u,
    )
    .await
    .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    assert_eq!(a.status, Verfahrensstatus::Entschieden);
    let b = &a.bescheide[0];
    assert_eq!(b.entscheidung, Entscheidung::Genehmigt);
    assert_eq!(b.auflagen[0].art, Nebenbestimmungsart::Auflage);
    assert_eq!(
        b.auflagen[0].titel,
        "Vor Baubeginn ist der geprüfte Standsicherheitsnachweis vorzulegen."
    );
    assert_eq!(b.abweichungsentscheidungen[0].abweichung, abw_id);
    assert!(b.abweichungsentscheidungen[0].zugelassen);
    assert_eq!(
        b.abweichungsentscheidungen[0].nebenbestimmung,
        "Die Garage bleibt ohne Aufenthaltsraum."
    );
}
