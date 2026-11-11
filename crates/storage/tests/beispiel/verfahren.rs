//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW), Teil Dokumente
//! und Verfahren: Projektdateien mit Kategorien und Dokumentdaten,
//! Unterlagenregister und Aufgaben, Verfahrensakte (Bauantrag im vereinfachten
//! Verfahren, § 64 BauO NRW 2018), Bauantragsangaben, Formulare aus der
//! Formularbibliothek, XBau-Abgabe, Einreichung, Portal, E-Mail-Archiv und
//! Fristen. Alle Eingaben laufen über die öffentlichen Core-Dienste.
//!
//! Zeitachse (Stichtag 21.10.2026):
//! - 08.09.2026 Planstand der Bauvorlagen, 14.09. Unterschrift Bauherr
//! - 15.09.2026 Bauantrag über Bauportal.NRW eingereicht (externe Einreichung)
//! - 17.09.2026 Eingangsbestätigung (XBau 1120) mit Aktenzeichen
//! - 29.09.2026 Vollständigkeitsbestätigung, Beteiligung Stadtentwässerung
//! - 08.10.2026 Nachforderung Entwässerungsnachweis per E-Mail, Frist 05.11.2026
#![allow(dead_code)]
use super::basis::*;
use bytes::Bytes;
use chrono::NaiveDate;
use core_lib::{
    dokumentkategorien::KategorieZuordnung,
    domain::DocumentVersion,
    formular::{
        editor::{EigenesPdfFeld, felder_hinzufuegen},
        instanz::FormularSpeicheroptionen,
        pdf::{Feldart, Feldwert, felder_lesen},
    },
    plattform::{ObjektArt, objekte::Dokumentbezug},
    projektbereiche::ProjektBereich,
    verfahren::{
        abgabe::Abgabewunsch,
        externe_einreichung::{Einreichungsweg, ExterneEinreichungswunsch},
        profile::{Anforderungskatalog, Vorhabenprofil},
        *,
    },
    xbau::postfach::{Nachrichtenaufnahme, Richtung},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use uuid::Uuid;

pub const BEHOERDE: &str = "Bauaufsicht Stadt Musterstadt";
pub const BEHOERDE_KENNUNG: &str = "bab:059990000";
pub const BUERO_KENNUNG: &str = "buero:planer-architekten-musterstadt";
pub const AKTENZEICHEN: &str = "63-BA-0815/2026";

/// Ergebnisse dieses Moduls für Prüfungen und spätere Module.
pub struct Verfahrensergebnis {
    pub verfahren: Uuid,
    pub abgabe: Uuid,
    pub xbau_xml: String,
    pub xbau_hinweise: Vec<String>,
    pub formular: DocumentVersion,
    pub formularwerte: BTreeMap<String, Feldwert>,
    pub beispielformular: BTreeMap<String, Feldwert>,
    pub paket_zip: Vec<u8>,
    pub portalbereitstellung: Option<Vec<u8>>,
    pub offene_punkte: Vec<String>,
}

fn tag(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

/// PDF-Text: Standardschrift ohne Umlaute, Klammern maskiert.
fn pdf_text(s: &str) -> String {
    s.replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('Ä', "Ae")
        .replace('Ö', "Oe")
        .replace('Ü', "Ue")
        .replace('ß', "ss")
        .replace('§', "Par.")
        .replace('²', "2")
        .replace('–', "-")
        .replace(['„', '“'], "\"")
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .chars()
        .filter(|c| c.is_ascii())
        .collect()
}

/// Kleines, echtes PDF (A4, eine Seite) mit Titel und Textzeilen.
pub fn pdf(titel: &str, zeilen: &[&str]) -> Vec<u8> {
    use lopdf::{Document, Stream, dictionary};
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let f = d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let mut inhalt = format!("BT /F 16 Tf 56 790 Td ({}) Tj ET\n", pdf_text(titel));
    for (i, z) in zeilen.iter().enumerate() {
        inhalt.push_str(&format!(
            "BT /F 10 Tf 56 {} Td ({}) Tj ET\n",
            760 - 16 * i as i32,
            pdf_text(z)
        ));
    }
    inhalt.push_str("BT /F 8 Tf 56 40 Td (Planungsbuero Planer Architekten, Am Markt 5, 59999 Musterstadt) Tj ET\n");
    let c = d.add_object(Stream::new(dictionary! {}, inhalt.into_bytes()));
    let p = d.add_object(dictionary! {"Type"=>"Page","Parent"=>root,
    "MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],
    "Resources"=>dictionary!{"Font"=>dictionary!{"F"=>f}},"Contents"=>c});
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

fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for (i, ch) in data.chunks(3).enumerate() {
        if i > 0 && i % 19 == 0 {
            out.push_str("\r\n");
        }
        let n = (u32::from(ch[0]) << 16)
            | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
            | u32::from(*ch.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= ch.len() {
                out.push(A[((n >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn bezug(v: &DocumentVersion) -> Dokumentbezug {
    Dokumentbezug {
        dokument_id: v.document_id,
        version: v.version,
    }
}

async fn akte(b: &Beispiel, vid: Uuid) -> (i64, Verfahrensakte) {
    let (f, a) = b.c.verfahrensakte(b.p.id, vid).await.unwrap();
    (f.revision, a)
}

/// Speichert die Akte; eine Ablehnung wird als Befund festgehalten.
async fn akte_speichern(b: &mut Beispiel, vid: Uuid, a: &Verfahrensakte, was: &str) -> bool {
    let (rev, _) = akte(b, vid).await;
    match b
        .c
        .fachobjekt_speichern(
            b.p.id,
            ObjektArt::Verfahren,
            vid,
            rev,
            serde_json::to_value(a).unwrap(),
            &b.u,
        )
        .await
    {
        Ok(_) => true,
        Err(e) => {
            b.befund(format!("Verfahrensakte ({was}) abgelehnt: {e}"));
            false
        }
    }
}

/// Legt eine Projektdatei mit fachlicher Kategorie und Dokumentdaten an.
#[allow(clippy::too_many_arguments)]
async fn datei(
    b: &mut Beispiel,
    name: &str,
    kategorie: Option<&str>,
    bytes: Vec<u8>,
    dokumentart: &str,
    plannummer: &str,
    massstab: &str,
    verfasser: &str,
    datum: &str,
) -> DocumentVersion {
    let d =
        b.c.store_document_kategorisiert(
            b.p.id,
            name,
            Bytes::from(bytes),
            kategorie.map(str::to_owned),
            &b.u,
        )
        .await
        .unwrap_or_else(|e| panic!("{name} ablegen: {e}"));
    if d.file_name != name {
        b.befund(format!(
            "Dateiname beim Ablegen verändert: „{name}“ → „{}“",
            d.file_name
        ));
    }
    b.speichern(
        ObjektArt::Dokumentdaten,
        d.document_id.0,
        json!({"dokument": bezug(&d), "dokumentart": dokumentart, "plannummer": plannummer,
            "revisionsindex": if plannummer.is_empty() { "" } else { "a" }, "massstab": massstab,
            "verfasser": verfasser, "datum": tag(datum)}),
    )
    .await;
    d
}

pub struct Dokumente {
    pub lageplan: DocumentVersion,
    pub grundrisse: DocumentVersion,
    pub schnitt: DocumentVersion,
    pub ansichten: DocumentVersion,
    pub baubeschreibung: DocumentVersion,
    pub grz: DocumentVersion,
    pub statik: DocumentVersion,
    pub grundbuch: DocumentVersion,
    pub kammer: DocumentVersion,
    pub statistik: DocumentVersion,
    pub quittung: DocumentVersion,
    pub vollstaendigkeit: DocumentVersion,
}

/// Der Standardkatalog enthält alle im Beispiel verwendeten Kategorien.
async fn kategorien_ergaenzen(b: &mut Beispiel) {
    let k = b.c.kategorienkatalog().await.unwrap();
    for id in [
        "grundbuch",
        "schriftverkehr",
        "bescheinigung",
        "entwaesserung",
    ] {
        if k.kategorie(id).is_err() {
            b.befund(format!("Standardkategorie „{id}“ fehlt"));
        }
    }
}

async fn dokumente_anlegen(b: &mut Beispiel) -> Dokumente {
    kategorien_ergaenzen(b).await;
    let ep = "Erika Planer, Architektin (AKNW A-12345)";
    let lageplan = datei(
        b,
        "Lageplan_1-500.pdf",
        Some("lageplan"),
        pdf(
            "Amtlicher Lageplan 1:500 (§ 3 BauPrüfVO NRW)",
            &[
                "Gemarkung Musterstadt (1234), Flur 3, Flurstück 412, 620 m²",
                "Nachbarn: Flurstück 411 (Nina Nachbar), Flurstück 413 (Bernd Beispiel)",
                "Bebauungsplan Nr. 42 „Am Lindenweg“: WA, GRZ 0,4, GFZ 0,8, II, o",
                "Wohnhaus 99 m², Garage 21 m², Zufahrt 30 m², Stellplatz 12,5 m², Terrasse 20 m²",
                "Abstandsflächen 0,4 H, mind. 3,00 m, auf eigenem Grundstück",
                "Höhenbezug: ±0,00 = 72,64 m ü. NHN, Straßenoberkante 72,40 m ü. NHN",
                "Aufgestellt: Vermessungsbüro Grenzstein (ÖbVI), 02.09.2026",
            ],
        ),
        "Lageplan",
        "EFH-MU-001",
        "1:500",
        "Vermessungsbüro Grenzstein, ÖbVI",
        "2026-09-02",
    )
    .await;
    let grundrisse = datei(
        b,
        "Grundrisse_KG_EG_DG.pdf",
        Some("grundriss"),
        pdf(
            "Grundrisse KG, EG, DG 1:100",
            &[
                "KG: Keller, Technik, Lager; lichte Höhe 2,40 m",
                "EG: Wohnen/Essen/Kochen, Arbeiten, WC, Diele; lichte Höhe 2,60 m",
                "DG: 3 Zimmer, Bad; Kniestock 1,00 m, Dachneigung 38°",
            ],
        ),
        "Grundriss",
        "EFH-MU-101",
        "1:100",
        ep,
        "2026-09-08",
    )
    .await;
    let schnitt = datei(
        b,
        "Schnitt_A-A.pdf",
        Some("schnitt"),
        pdf(
            "Schnitt A-A 1:100",
            &[
                "Traufe +4,20 m, First +7,45 m über ±0,00 (72,64 m ü. NHN)",
                "Gelände -0,30 m, Fundamentsohle -3,10 m",
            ],
        ),
        "Schnitt",
        "EFH-MU-201",
        "1:100",
        ep,
        "2026-09-08",
    )
    .await;
    let ansichten = datei(
        b,
        "Ansichten_Nord_Süd_Ost_West.pdf",
        Some("ansicht"),
        pdf(
            "Ansichten 1:100",
            &["Satteldach 38°, Putzfassade weiß, Dach anthrazit"],
        ),
        "Ansicht",
        "EFH-MU-301",
        "1:100",
        ep,
        "2026-09-08",
    )
    .await;
    // Planliste des Büros: die eingereichten Pläne mit Büroindex a.
    for (d, planart, geschoss, nummer, titel, massstab, datum) in [
        (
            &lageplan,
            "lageplan",
            "",
            1,
            "Amtlicher Lageplan",
            500,
            "2026-09-02",
        ),
        (
            &grundrisse,
            "grundriss",
            "",
            1,
            "Grundrisse KG, EG, DG",
            100,
            "2026-09-08",
        ),
        (&schnitt, "schnitt", "", 1, "Schnitt A-A", 100, "2026-09-08"),
        (
            &ansichten,
            "ansicht",
            "",
            1,
            "Ansichten Nord, Süd, Ost, West",
            100,
            "2026-09-08",
        ),
    ] {
        b.speichern(
            ObjektArt::Planliste,
            d.document_id.0,
            json!({
                "dokument_id": d.document_id, "planart": planart, "geschoss": geschoss,
                "nummer": nummer, "titel": titel, "massstab": massstab,
                "revisionen": [{"index": "a", "datum": datum, "aenderung": "Erstausgabe zum Bauantrag", "version": d.version}],
                "verteiler": []
            }),
        )
        .await;
    }
    let baubeschreibung = datei(
        b,
        "Baubeschreibung.pdf",
        Some("baubeschreibung"),
        pdf(
            "Baubeschreibung (§ 5 BauPrüfVO NRW)",
            &[
                "Neubau eines freistehenden Einfamilienhauses mit Garage, eine Wohneinheit",
                "Gebäudeklasse 1, Massivbau, Keller WU-Beton, Satteldach 38°",
                "Stellplätze: Garage und Stellplatz vor der Garage (2 Stellplätze)",
                "Entwässerung: Anschluss an öffentlichen Mischwasserkanal Lindenweg",
            ],
        ),
        "Baubeschreibung",
        "",
        "",
        ep,
        "2026-09-08",
    )
    .await;
    let grz = datei(
        b,
        "Berechnung_GRZ_GFZ.pdf",
        Some("grz"),
        pdf(
            "Berechnung Maß der baulichen Nutzung (§§ 19, 20 BauNVO 2017)",
            &[
                "Grundstücksfläche 620,00 m²",
                "GR Hauptanlage 119,00 m² (Wohnhaus 99 + Terrasse 20) -> GRZ 0,19 <= 0,4",
                "GR Nebenanlagen 63,50 m² -> GRZ gesamt 0,29 <= 0,6",
                "GF 99,00 m² (nur EG Vollgeschoss) -> GFZ 0,16 <= 0,8",
            ],
        ),
        "Berechnung",
        "",
        "",
        ep,
        "2026-09-08",
    )
    .await;
    let statik = datei(
        b,
        "Statik_Vorabzug_Kriterienkatalog.pdf",
        Some("statik"),
        pdf(
            "Standsicherheit - Vorabzug und Erklärung nach Kriterienkatalog",
            &[
                "Ingenieurbüro Statik Beispiel, Tobias Träger",
                "Erklärung nach Anlage 2 BauPrüfVO NRW: Kriterienkatalog erfüllt, keine Prüfung erforderlich",
                "Vorabzug Positionsplan und Lastannahmen, Stand 05.09.2026",
            ],
        ),
        "Standsicherheitsnachweis",
        "TWP-26-117",
        "",
        "Ingenieurbüro Statik Beispiel",
        "2026-09-05",
    )
    .await;
    let grundbuch = datei(
        b,
        "Grundbuchauszug_Musterstadt_Blatt_1234.pdf",
        Some("grundbuch"),
        pdf(
            "Grundbuch von Musterstadt, Blatt 1234 (Auszug)",
            &[
                "Bestandsverzeichnis lfd. Nr. 1: Gemarkung Musterstadt, Flur 3, Flurstück 412, 620 m²",
                "Abteilung I: Max Mustermann",
                "Abteilung II: keine Eintragungen",
                "Amtsgericht Musterstadt, 20.08.2026",
            ],
        ),
        "Grundbuchauszug",
        "",
        "",
        "Amtsgericht Musterstadt - Grundbuchamt",
        "2026-08-20",
    )
    .await;
    let kammer = datei(
        b,
        "Bescheinigung_Bauvorlageberechtigung_AKNW.pdf",
        Some("bescheinigung"),
        pdf(
            "Architektenkammer Nordrhein-Westfalen - Mitgliedsbescheinigung",
            &["Erika Planer, Architektin, Mitgliedsnummer A-12345, bauvorlageberechtigt"],
        ),
        "Nachweis Bauvorlageberechtigung",
        "",
        "",
        "Architektenkammer NRW",
        "2026-01-10",
    )
    .await;
    let statistik = datei(
        b,
        "Baustatistik_Erhebungsbogen.pdf",
        Some("bauantrag"),
        pdf(
            "Statistik der Baugenehmigungen - Erhebungsbogen",
            &["1 Wohngebäude, 1 Wohnung, Massivbau, Heizung Wärmepumpe"],
        ),
        "Statistischer Erhebungsbogen",
        "",
        "",
        ep,
        "2026-09-08",
    )
    .await;
    let quittung = datei(
        b,
        "Bauportal_NRW_Uebermittlungsbestaetigung.pdf",
        Some("schriftverkehr"),
        pdf(
            "Bauportal.NRW - Übermittlungsbestätigung",
            &[
                "Antrag auf Baugenehmigung (vereinfachtes Verfahren) übermittelt am 15.09.2026 10:42",
                "Empfänger: Bauaufsicht Stadt Musterstadt, Vorgangsnummer BP-NRW-2026-447120",
            ],
        ),
        "Übermittlungsbestätigung",
        "",
        "",
        "Bauportal.NRW",
        "2026-09-15",
    )
    .await;
    let vollstaendigkeit = datei(
        b,
        "Vollstaendigkeitsbestaetigung_2026-09-29.pdf",
        Some("schriftverkehr"),
        pdf(
            "Stadt Musterstadt - Bauaufsicht - Az. 63-BA-0815/2026",
            &[
                "Ihr Bauantrag vom 15.09.2026 ist vollständig (§ 71 BauO NRW 2018).",
                "Vereinfachtes Baugenehmigungsverfahren nach § 64 BauO NRW 2018.",
                "Über den Antrag wird bis zum 10.11.2026 entschieden.",
                "Die Stadtentwässerung Musterstadt wird beteiligt (Stellungnahme bis 27.10.2026).",
            ],
        ),
        "Behördenschreiben",
        "",
        "",
        BEHOERDE,
        "2026-09-29",
    )
    .await;
    Dokumente {
        lageplan,
        grundrisse,
        schnitt,
        ansichten,
        baubeschreibung,
        grz,
        statik,
        grundbuch,
        kammer,
        statistik,
        quittung,
        vollstaendigkeit,
    }
}

/// Grundbuchauszug am Vorhabenflurstück hinterlegen; die GRZ-Ermittlung darf
/// dadurch nicht veralten.
async fn grundbuch_verknuepfen(b: &mut Beispiel, d: &DocumentVersion) {
    let vorher = core_lib::grundstuecksinformationen::quellen_aktuell(&b.p.planungsrecht).unwrap();
    let mut r = b.p.planungsrecht.clone();
    let infos = r.grundstuecksinformationen.as_mut().unwrap();
    let f = infos
        .flurstuecke
        .iter_mut()
        .find(|f| f.id == "fs-412")
        .unwrap();
    f.grundbuchauszug = Some(bezug(d));
    b.bereich(ProjektBereich::Grundstuecksinformationen, r)
        .await;
    let nachher = core_lib::grundstuecksinformationen::quellen_aktuell(&b.p.planungsrecht).unwrap();
    if vorher && !nachher {
        b.befund("Das Verknüpfen des Grundbuchauszugs am Flurstück macht die Baugrundstücksermittlung veraltet, obwohl sich keine Katasterangabe geändert hat.");
    }
    let gespeichert = core_lib::grundstuecksinformationen::informationen(&b.p.planungsrecht);
    assert_eq!(
        gespeichert
            .flurstuecke
            .iter()
            .find(|f| f.id == "fs-412")
            .unwrap()
            .grundbuchauszug,
        Some(bezug(d))
    );
}

/// Feldnamen des Büroformulars = Schlüssel des Projektkontexts.
const FORMULARFELDER: &[(&str, &str)] = &[
    ("bauherr.name", "Bauherr"),
    ("bauherr.strasse_hausnummer", "Straße, Hausnummer"),
    ("bauherr.plz_ort", "PLZ, Ort"),
    ("bauherr.telefon", "Telefon"),
    ("vorhaben.bezeichnung", "Bezeichnung des Vorhabens"),
    ("grundstueck.strasse_hausnummer", "Baugrundstück Straße"),
    ("grundstueck.plz_ort", "Baugrundstück PLZ/Ort"),
    ("grundstueck.gemarkung", "Gemarkung"),
    ("grundstueck.flur", "Flur"),
    ("grundstueck.flurstuecke", "Flurstück(e)"),
    ("grundstueck.flaeche_m2", "Grundstücksgröße m²"),
    (
        "grundstueck.bezugsflaeche_m2",
        "Maßgebende Grundstücksfläche m²",
    ),
    ("grundstueck.grundbuchblatt", "Grundbuchblatt"),
    ("planung.bebauungsplan_nummer", "Bebauungsplan Nr."),
    ("planung.baugebiet", "Baugebiet"),
    ("kennzahlen.grz", "GRZ"),
    ("kennzahlen.grz_gesamt", "GRZ mit Nebenanlagen"),
    ("kennzahlen.gfz", "GFZ"),
    ("flaechen.grundflaeche_m2", "Grundfläche m²"),
    ("flaechen.geschossflaeche_m2", "Geschossfläche m²"),
    ("gebaeude.vollgeschosse", "Vollgeschosse"),
    ("entwurfsverfasser.name", "Entwurfsverfasserin"),
    ("entwurfsverfasser.mitgliedsnummer", "Kammer-Nr."),
    ("entwurfsverfasser.kammerland", "Kammer"),
    ("behoerde.name", "Bauaufsichtsbehörde"),
    ("datum.heute", "Datum"),
];

/// Formulare: Repo-Vorlage (Beispielformular) und eigenes Büroformular mit
/// Grundstücks- und Kennzahlenfeldern, beide aus der Formularbibliothek.
async fn formulare(
    b: &mut Beispiel,
) -> (
    DocumentVersion,
    BTreeMap<String, Feldwert>,
    BTreeMap<String, Feldwert>,
) {
    let root = b.profil.join("Formularbibliothek");
    // 1. Repo-Zuordnung formulare/beispiel-bauantrag.json mit dem Musterformular.
    let muster = core_lib::formular::testdaten::musterformular();
    let v = storage::formularbibliothek::ablegen(&root, "Bauantrag_Beispiel.pdf", &muster)
        .await
        .unwrap();
    let quelle = storage::formularbibliothek::lesen(&root, &v.name, Some(&v.sha256))
        .await
        .unwrap();
    let schema = b.c.formular_schema(b.p.id, &quelle).await.unwrap();
    for h in &schema.quellen.hinweise {
        b.info(format!("Formularquellen: {h}"));
    }
    match &schema.vorlage {
        Some(v) => b.info(format!(
            "Beispielformular erkannt: {} ({})",
            v.titel, v.version
        )),
        None => b.befund("Das Musterformular wird nicht als „beispiel-bauantrag“ erkannt."),
    }
    let beispiel = schema.vorschlag.clone();
    b.c.formular_aus_bibliothek_mit_quellen(
        b.p.id,
        &v.name,
        &quelle,
        &beispiel,
        FormularSpeicheroptionen {
            vorgang: Some(Uuid::new_v4()),
            datenstand: Some(schema.quellen.datenstand.clone()),
        },
        &b.u,
    )
    .await
    .unwrap();
    b.info("Die Repo-Vorlage formulare/beispiel-bauantrag.json ordnet nur Bauherr, Grundstücksgröße und Datum zu; GRZ/GFZ prüft das Büroformular.");

    // 2. Eigenes Büroformular: Felder heißen wie die Kontextschlüssel.
    let mut zeilen = Vec::new();
    for (_, l) in FORMULARFELDER {
        zeilen.push(format!("{l}:"));
    }
    let zeilen: Vec<&str> = zeilen.iter().map(String::as_str).collect();
    let grund = pdf(
        "Antrag auf Baugenehmigung - vereinfachtes Verfahren (Büroformular)",
        &zeilen,
    );
    let felder: Vec<EigenesPdfFeld> = FORMULARFELDER
        .iter()
        .enumerate()
        .map(|(i, (name, l))| EigenesPdfFeld {
            name: (*name).into(),
            beschriftung: (*l).into(),
            art: Feldart::Text,
            seite: 1,
            x_mm: 95.,
            // Zeile i steht 760 - 16 i pt über dem unteren Rand.
            y_mm: (842. - (760. - 16. * i as f32)) * 25.4 / 72. - 3.5,
            breite_mm: 95.,
            hoehe_mm: 5.,
            schriftgroesse_pt: 9.,
            mehrzeilig: false,
        })
        .collect();
    let vorlage = felder_hinzufuegen(&grund, &felder).unwrap();
    let v =
        storage::formularbibliothek::ablegen(&root, "Bauantrag_NRW_Bueroformular.pdf", &vorlage)
            .await
            .unwrap();
    let quelle = storage::formularbibliothek::lesen(&root, &v.name, Some(&v.sha256))
        .await
        .unwrap();
    let schema = b.c.formular_schema(b.p.id, &quelle).await.unwrap();
    let k = &schema.quellen.datenstand.werte;
    // Erwartete Werte aus Basis (620 m², GR 119 m², NA 63,5 m², GF 99 m²).
    for (feld, erwartet) in [
        ("grundstueck.flaeche_m2", "620,00"),
        ("grundstueck.bezugsflaeche_m2", "620,00"),
        ("grundstueck.gemarkung", "Musterstadt"),
        ("grundstueck.flur", "3"),
        ("grundstueck.flurstuecke", "412"),
        ("grundstueck.grundbuchblatt", "1234"),
        ("planung.baugebiet", "WA"),
        ("planung.bebauungsplan_nummer", "42"),
        ("kennzahlen.grz", "0,19"),
        ("kennzahlen.grz_gesamt", "0,29"),
        ("kennzahlen.gfz", "0,16"),
        ("flaechen.grundflaeche_m2", "119,00"),
        ("flaechen.geschossflaeche_m2", "99,00"),
        ("gebaeude.vollgeschosse", "1"),
        ("bauherr.name", "Max Mustermann"),
        ("grundstueck.strasse_hausnummer", "Lindenweg 12"),
        ("entwurfsverfasser.name", "Erika Planer"),
        ("entwurfsverfasser.mitgliedsnummer", "A-12345"),
        ("behoerde.name", BEHOERDE),
    ] {
        let ist = k.wert(feld);
        let vorschlag = schema.vorschlag.get(feld);
        if ist != Some(erwartet) {
            b.befund(format!(
                "Formularquelle {feld}: erwartet „{erwartet}“, geliefert {ist:?}"
            ));
        } else if vorschlag != Some(&Feldwert::Text(erwartet.into())) {
            b.befund(format!(
                "Formularfeld {feld}: Quelle „{erwartet}“ kommt nicht als Vorschlag im Feld an ({vorschlag:?})"
            ));
        }
    }
    for h in &schema.quellen.hinweise {
        b.info(format!("Formularquellen (Büroformular): {h}"));
    }
    let werte = schema.vorschlag.clone();
    let fehlend: Vec<_> = FORMULARFELDER
        .iter()
        .filter(|(n, _)| !werte.contains_key(*n))
        .map(|(n, _)| *n)
        .collect();
    if !fehlend.is_empty() {
        b.info(format!(
            "Büroformular: ohne Projektwert bleiben {fehlend:?}"
        ));
    }
    let d =
        b.c.formular_aus_bibliothek_mit_quellen(
            b.p.id,
            &v.name,
            &quelle,
            &werte,
            FormularSpeicheroptionen {
                vorgang: Some(Uuid::new_v4()),
                datenstand: Some(schema.quellen.datenstand.clone()),
            },
            &b.u,
        )
        .await
        .unwrap();
    b.c.dokumentkategorie_speichern(
        d.document_id,
        KategorieZuordnung {
            revision: 0,
            kategorie: Some("bauantrag".into()),
        },
        &b.u,
    )
    .await
    .unwrap();
    // Gespeichertes PDF wieder lesen: Werte müssen im Dokument stehen.
    let (_, gespeichert) = b.c.load_document(d.document_id).await.unwrap();
    for f in felder_lesen(&gespeichert).unwrap() {
        if let Some(w) = werte.get(&f.name)
            && f.wert.as_ref() != Some(w)
        {
            b.befund(format!(
                "Formular „{}“: Feld {} enthält nach dem Speichern {:?} statt {:?}",
                d.file_name, f.name, f.wert, w
            ));
        }
    }
    let instanz =
        b.c.formular_instanz(d.document_id, d.version)
            .await
            .unwrap();
    assert!(instanz.is_some_and(|i| i.veraltet.is_empty()));
    (d, werte, beispiel)
}

/// Unterlagenregister: Bürocheckliste plus NRW-Bauvorlagen (BauPrüfVO).
async fn unterlagen(b: &mut Beispiel, d: &Dokumente, formular: &DocumentVersion) {
    let neu =
        b.c.checkliste_anwenden(
            b.p.id,
            core_lib::plattform::ablauf::Checkliste::standard().unwrap(),
            &b.u,
        )
        .await
        .unwrap();
    b.info(format!("Bürocheckliste: {neu} Unterlagen angelegt"));
    let vorhanden =
        b.c.fachobjekte(b.p.id, ObjektArt::Unterlagen)
            .await
            .unwrap();
    let punkt = |p: &str| {
        vorhanden
            .iter()
            .find(|f| f.daten["vorlage"]["punkt"] == p)
            .map(|f| (f.objekt.parse::<Uuid>().unwrap(), f.daten.clone()))
            .unwrap()
    };
    let frei = |mut v: Value, dok: &DocumentVersion, erforderlich: bool, bemerkung: &str| {
        v["dokument"] = json!(bezug(dok));
        v["status"] = json!("freigegeben");
        v["erforderlich"] = json!(erforderlich);
        v["bemerkung"] = json!(bemerkung);
        v
    };
    for (p, dok, erf, bem) in [
        (
            "formular",
            formular,
            true,
            "Amtlicher Vordruck Bauantrag, § 1 BauPrüfVO NRW; vom Bauherrn am 14.09.2026 unterschrieben",
        ),
        (
            "lageplan",
            &d.lageplan,
            true,
            "Amtlicher Lageplan nach § 3 BauPrüfVO NRW (ÖbVI)",
        ),
        (
            "zeichnungen",
            &d.grundrisse,
            true,
            "Bauzeichnungen 1:100 nach § 4 BauPrüfVO NRW; Schnitt und Ansichten getrennt geführt",
        ),
        (
            "beschreibung",
            &d.baubeschreibung,
            true,
            "Baubeschreibung nach § 5 BauPrüfVO NRW",
        ),
        (
            "nutzung",
            &d.grz,
            true,
            "Berechnung GRZ/GFZ nach §§ 19, 20 BauNVO 2017 (B-Plan Nr. 42)",
        ),
        (
            "abstand",
            &d.lageplan,
            true,
            "Abstandsflächen im Lageplan dargestellt (§ 6 BauO NRW 2018)",
        ),
    ] {
        let (id, v) = punkt(p);
        b.speichern(ObjektArt::Unterlagen, id, frei(v, dok, erf, bem))
            .await;
    }
    for (bez, dok, erf, status, bem) in [
        (
            "Bauzeichnungen: Schnitt",
            Some(&d.schnitt),
            true,
            "freigegeben",
            "§ 4 BauPrüfVO NRW",
        ),
        (
            "Bauzeichnungen: Ansichten",
            Some(&d.ansichten),
            true,
            "freigegeben",
            "§ 4 BauPrüfVO NRW",
        ),
        (
            "Erklärung zum Standsicherheitsnachweis (Kriterienkatalog)",
            Some(&d.statik),
            true,
            "freigegeben",
            "Anlage 2 BauPrüfVO NRW; Standsicherheitsnachweis wird vor Baubeginn vorgelegt (Vorabzug liegt bei)",
        ),
        (
            "Nachweis der Bauvorlageberechtigung",
            Some(&d.kammer),
            true,
            "freigegeben",
            "§ 67 BauO NRW 2018, Mitglied AKNW",
        ),
        (
            "Statistischer Erhebungsbogen",
            Some(&d.statistik),
            true,
            "freigegeben",
            "Hochbaustatistikgesetz",
        ),
        (
            "Grundbuchauszug",
            Some(&d.grundbuch),
            false,
            "freigegeben",
            "Nicht vorgeschrieben; Eigentum des Bauherrn belegt",
        ),
        (
            "Entwässerungsnachweis (Grundstücksentwässerungsplan)",
            None,
            true,
            "entwurf",
            "Von der Bauaufsicht am 08.10.2026 nachgefordert, Frist 05.11.2026",
        ),
    ] {
        let v = json!({"vorlage": null, "bezeichnung": bez, "erforderlich": erf,
            "dokument": dok.map(bezug), "status": status, "bemerkung": bem});
        b.speichern(ObjektArt::Unterlagen, Uuid::new_v4(), v).await;
    }
}

async fn aufgaben(b: &mut Beispiel, punkt: Uuid) {
    let u = b.u.id;
    for (titel, beschreibung, faellig, status, bezug) in [
        (
            "Bauvorlagen zusammenstellen und freigeben",
            "Lageplan, Bauzeichnungen, Baubeschreibung, GRZ/GFZ-Berechnung prüfen",
            "2026-09-10",
            "erledigt",
            None,
        ),
        (
            "Statik-Vorabzug mit Tragwerksplaner abstimmen",
            "Erklärung nach Kriterienkatalog einholen",
            "2026-09-08",
            "erledigt",
            None,
        ),
        (
            "Bauantrag vom Bauherrn unterschreiben lassen",
            "Termin mit Max Mustermann",
            "2026-09-14",
            "erledigt",
            None,
        ),
        (
            "Entwässerungsnachweis beim Ingenieurbüro beauftragen und prüfen",
            "Grundstücksentwässerungsplan mit Überflutungsnachweis (DIN 1986-100)",
            "2026-10-30",
            "in_arbeit",
            Some(punkt),
        ),
        (
            "Entwässerungsnachweis bei der Bauaufsicht nachreichen",
            "Nachreichung über Bauportal.NRW, Frist 05.11.2026",
            "2026-11-04",
            "offen",
            Some(punkt),
        ),
    ] {
        b.speichern(
            ObjektArt::Aufgaben,
            Uuid::new_v4(),
            json!({"titel": titel, "beschreibung": beschreibung, "faellig": tag(faellig),
                "verantwortlich": u, "status": status, "bezug": bezug}),
        )
        .await;
    }
    // Die Projektorganisation trägt die Identität des Projekts.
    let pid = b.p.id.0;
    b.speichern(
        ObjektArt::Projektorganisation,
        pid,
        json!({"kennzeichen": "2026-017", "phase": "genehmigung", "verantwortlich": u,
            "beschreibung": "EFH Mustermann, Lindenweg 12; Bauantrag vereinfachtes Verfahren"}),
    )
    .await;
}

/// Bauantragsangaben (XBau): Behörde, Entwurfsverfasserin, Anschrift, Anlagen.
async fn bauantrag(b: &mut Beispiel, anlagen: &[(&DocumentVersion, &str, &str)], bewerten: bool) {
    let alt = b.c.bauantrag(b.p.id).await.unwrap();
    let mut antrag = alt.antrag.clone();
    antrag.bauaufsicht = core_lib::xbau::Bauaufsicht {
        name: BEHOERDE.into(),
        kennung: BEHOERDE_KENNUNG.into(),
    };
    antrag.bezeichnung = "Neubau eines freistehenden Einfamilienhauses mit Garage".into();
    antrag.entwurfsverfasser = Some(b.entwurfsverfasser);
    antrag.massnahmen = vec!["11".into()];
    antrag.grundstueck.anschrift = anschrift("Lindenweg", "12", "59999", "Musterstadt");
    // Die Anlagenart folgt der Dokumentkategorie; gewählt wird nur, wo sie
    // davon abweicht (z. B. amtlicher statt einfacher Lageplan).
    let mut aus_kategorie = 0;
    antrag.anlagen = anlagen
        .iter()
        .map(|(d, art, bez)| {
            let vorschlag = alt
                .anlagenart_vorschlaege
                .get(&d.document_id.0.to_string())
                .map(|v| v.anlageart.as_str());
            if vorschlag == Some(*art) {
                aus_kategorie += 1;
            }
            core_lib::xbau::Anlagenwahl {
                dokument: d.document_id,
                anlageart: if vorschlag == Some(*art) {
                    String::new()
                } else {
                    (*art).into()
                },
                bezeichnung: (*bez).into(),
            }
        })
        .collect();
    if bewerten {
        b.info(format!(
            "Bauantrag: {aus_kategorie} von {} Anlagenarten aus der Dokumentkategorie, die übrigen gewählt.",
            anlagen.len()
        ));
        if aus_kategorie < 5 {
            b.befund(format!(
                "Anlagenart-Vorschläge aus Dokumentkategorien: nur {aus_kategorie}"
            ));
        }
    }
    let neu =
        b.c.bauantrag_speichern(b.p.id, antrag, alt.version, &b.u)
            .await
            .unwrap();
    for f in &neu.fehlende_angaben {
        b.info(format!("Bauantrag fehlt: {f}"));
    }
    for h in &neu.hinweise {
        b.info(format!("Bauantrag Hinweis: {h}"));
    }
    if bewerten && neu.verfahrensarten.is_none() {
        b.info("Bauantrag: Für NRW ist keine Landesliste der Verfahrensarten veröffentlicht; die XBau-Nachricht enthält keine Verfahrensart (optional). Eine Liste der Bauaufsicht ließe sich im Bauantrag einspielen.");
    }
    // Ohne geladene NRW-Codeliste nur ein informativer Hinweis, keine
    // unerfüllbare Aufforderung.
    if bewerten && neu.verfahrensarten.is_none() {
        assert!(
            !neu.hinweise
                .iter()
                .any(|h| h.contains("Landes-Verfahrensart im Bauantrag ergänzen")),
            "{:?}",
            neu.hinweise
        );
        assert!(
            neu.hinweise
                .iter()
                .any(|h| h.contains("Zur Information") && h.contains("verfahrensart")),
            "{:?}",
            neu.hinweise
        );
    }
    assert!(neu.flurstuecke_zentral && neu.gebaeude_zentral);
    assert_eq!(neu.antrag.anzahl_gebaeude, 2);
}

pub async fn verfahren(b: &mut Beispiel) -> Verfahrensergebnis {
    // Absenderkennung des Büros fehlt in der Basis; ohne sie gibt es keinen XBau-Export.
    let mut buero = b.c.buero().await.unwrap();
    if buero.kennung.trim().is_empty() {
        b.info("Basis: Büro ohne Absenderkennung (DVDV/Portal) – für XBau hier ergänzt.");
        buero.kennung = BUERO_KENNUNG.into();
        b.c.buero_speichern(buero, &b.system).await.unwrap();
    }

    // 1. Projektdateien.
    let d = dokumente_anlegen(b).await;
    grundbuch_verknuepfen(b, &d.grundbuch).await;

    // 2. Bauantragsangaben ohne Anlagen; Akte und Formulare brauchen Behörde und Anschrift.
    bauantrag(b, &[], false).await;

    // 3. Verfahrensakte in Vorbereitung.
    let vid = Uuid::new_v4();
    let behoerde_id = Uuid::new_v4();
    let a: Verfahrensakte = serde_json::from_value(json!({
        "bezeichnung": "Bauantrag Neubau EFH Mustermann",
        "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": behoerde_id, "name": BEHOERDE, "kennung": BEHOERDE_KENNUNG,
            "kontakt": "Stadt Musterstadt, Fachbereich 63 Bauordnung, Rathausplatz 1, 59999 Musterstadt, bauaufsicht@musterstadt.example"},
        "aktenzeichen": "", "verantwortlich": b.u.id, "vorgaenger": null,
        "pruefverfahren": "vereinfacht", "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    }))
    .unwrap();
    b.speichern(ObjektArt::Verfahren, vid, serde_json::to_value(&a).unwrap())
        .await;
    let (rev, _) = akte(b, vid).await;
    let profil = Vorhabenprofil {
        bundesland: core_lib::domain::Bundesland::Nw,
        art: Verfahrensart::Bauantrag,
        gebaeudeklasse: Some(1),
        sonderbau: Some(false),
        nutzung: "Wohnen".into(),
        wohngebaeude: Some(true),
        rechtsgrundlage: None,
    };
    b.c.verfahren_profil_anwenden(
        b.p.id,
        vid,
        rev,
        profil,
        Anforderungskatalog::mitgeliefert()
            .unwrap()
            .into_iter()
            .find(|k| k.id == "nrw-bauvorlagen")
            .expect("NRW-Bauvorlagenkatalog"),
        &b.u,
    )
    .await
    .unwrap();
    // Profil fachlich bestätigen (Projektleitung).
    let (_, mut a) = akte(b, vid).await;
    a.profil.as_mut().unwrap().bestaetigt_von = Some(b.u.id.0);
    akte_speichern(b, vid, &a, "Profil bestätigen").await;

    // 4. Formulare aus der Formularbibliothek.
    let (formular, formularwerte, beispielformular) = formulare(b).await;

    // 5. Unterlagenregister.
    unterlagen(b, &d, &formular).await;

    // 6. Anforderungen der Akte fachlich einordnen und freigeben.
    let (_, mut a) = akte(b, vid).await;
    let regel = |r: &Anforderung| {
        r.quelle
            .split(" · ")
            .nth(2)
            .and_then(|s| s.split(':').next())
            .unwrap_or("")
            .to_string()
    };
    for r in &mut a.anforderungen {
        let (erf, docs, grund): (Erforderlichkeit, Vec<&DocumentVersion>, &str) = match regel(r)
            .as_str()
        {
            "antrag" => (
                Erforderlichkeit::Erforderlich,
                vec![&formular],
                "§ 70 BauO NRW, amtlicher Vordruck",
            ),
            "liegenschaftskarte" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Vorhaben im Geltungsbereich des B-Plans Nr. 42 (§ 30 BauGB)",
            ),
            "lageplan" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.lageplan],
                "§ 3 BauPrüfVO NRW, amtlicher Lageplan",
            ),
            "bauzeichnungen" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.grundrisse, &d.schnitt, &d.ansichten],
                "§ 4 BauPrüfVO NRW",
            ),
            "baubeschreibung" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.baubeschreibung],
                "§ 5 BauPrüfVO NRW",
            ),
            "betriebsbeschreibung" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Wohngebäude, kein Betrieb",
            ),
            "mass" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.grz],
                "B-Plan Nr. 42, BauNVO 2017",
            ),
            "abstand" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.lageplan],
                "§ 6 BauO NRW 2018; Abstandsflächen im Lageplan dargestellt, Berechnung als Nachweisstand",
            ),
            "standsicherheit" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.statik],
                "Erklärung nach Kriterienkatalog (Anlage 2 BauPrüfVO); Nachweis vor Baubeginn",
            ),
            "standsicherheit_pruefung" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Gebäudeklasse 1, Kriterienkatalog: keine Prüfung",
            ),
            "brandschutzkonzept" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Kein Sonderbau",
            ),
            "geg" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "GEG-Nachweis wird vor Baubeginn als bautechnischer Nachweis vorgelegt",
            ),
            "stellplaetze" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.lageplan, &d.baubeschreibung],
                "§ 48 BauO NRW 2018, Stellplatzsatzung; 2 Stellplätze",
            ),
            "entwaesserung" => (
                Erforderlichkeit::Erforderlich,
                vec![],
                "Entwässerungsnachweis von der Bauaufsicht nachgefordert",
            ),
            "statistik" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.statistik],
                "HBauStatG",
            ),
            "bauvorlageberechtigung" => (
                Erforderlichkeit::Erforderlich,
                vec![&d.kammer],
                "§ 67 BauO NRW, Architektenkammer NRW",
            ),
            "abweichungen" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Festsetzungen des B-Plans Nr. 42 eingehalten; Abweichungsabgleich ohne Bedarf",
            ),
            "zustimmungen" => (
                Erforderlichkeit::NichtErforderlich,
                vec![],
                "Bauherr ist Eigentümer (Grundbuch Blatt 1234), keine Baulasten",
            ),
            andere => {
                b.info(format!(
                    "Anforderung ohne Einordnung: {} ({andere})",
                    r.titel
                ));
                continue;
            }
        };
        r.erforderlich = erf.clone();
        r.grund = grund.into();
        r.dokumente = docs.into_iter().map(bezug).collect();
        r.verantwortlich = Some(b.u.id.0);
        r.freigegeben = !(erf == Erforderlichkeit::Erforderlich && r.dokumente.is_empty());
    }
    akte_speichern(b, vid, &a, "Anforderungen einordnen").await;

    // 7. Bauantrag mit Anlagen (XBau-Anlagenarten der Codeliste 4.0).
    let anlagen: Vec<(&DocumentVersion, &str, &str)> = vec![
        (&formular, "1101", "Bauantrag (Antragsformular)"),
        (&d.lageplan, "1804", "Amtlicher Lageplan 1:500"),
        (&d.grundrisse, "2001", "Grundrisse KG, EG, DG 1:100"),
        (&d.schnitt, "2002", "Schnitt A-A 1:100"),
        (&d.ansichten, "2003", "Ansichten 1:100"),
        (&d.baubeschreibung, "1501", "Baubeschreibung"),
        (&d.grz, "1901", "Berechnung GRZ/GFZ"),
        (
            &d.statik,
            "2810",
            "Erklärung des Tragwerksplaners (Kriterienkatalog)",
        ),
        (&d.kammer, "0401", "Nachweis der Bauvorlageberechtigung"),
        (&d.statistik, "2501", "Statistischer Erhebungsbogen"),
        (&d.grundbuch, "2008", "Grundbuchauszug Blatt 1234"),
    ];
    bauantrag(b, &anlagen, true).await;

    // 7a. Verfahrenswahl zum Antragseingang (15.09.2026, neue Fassung der
    // BauO NRW), Abweichungsabgleich und Unterschriften der Bauvorlagen.
    let wahl =
        b.c.verfahrenswahl(
            b.p.id,
            Some(vid),
            core_lib::verfahren::landesrecht::Verfahrensangaben {
                wohngebaeude: Some(true),
                erschliessung: Some(true),
                stichtag: Some(tag("2026-09-15")),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    b.info(format!(
        "Verfahrenswahl ({}): {}",
        wahl.fassung.clone().unwrap_or_default(),
        wahl.vorschlaege
            .iter()
            .map(|v| format!("{} {:?}", v.titel, v.eignung))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    use core_lib::verfahren::landesrecht::Eignung;
    if !wahl.vorschlaege.iter().any(|v| {
        v.titel == "Vereinfachtes Baugenehmigungsverfahren" && v.eignung == Eignung::Moeglich
    }) {
        b.befund("Verfahrenswahl: vereinfachtes Verfahren nicht als möglich belegt");
    }
    if wahl.fiktion.is_none() {
        b.befund("Verfahrenswahl: Genehmigungsfiktion der Fassung ab 01.09.2026 fehlt");
    }
    let abgleich = b.c.verfahren_abweichungen(b.p.id, vid).await.unwrap();
    if !abgleich.offen.is_empty() {
        b.befund(format!(
            "Abweichungsbedarf ohne Eintrag: {:?}",
            abgleich.bedarf
        ));
    }
    for h in &abgleich.hinweise {
        b.info(format!("Abweichungsabgleich: {h}"));
    }
    let bedarf = b.c.verfahren_unterschriften(b.p.id, vid).await.unwrap();
    let (_, mut a) = akte(b, vid).await;
    use core_lib::verfahren::landesrecht::{Unterschrift, Unterschriftsart, Unterschriftsrolle};
    for u in &bedarf {
        for rolle in &u.fehlend {
            let (name, art) = match rolle {
                Unterschriftsrolle::Bauherrschaft => {
                    ("Max Mustermann", Unterschriftsart::Handschriftlich)
                }
                Unterschriftsrolle::Entwurfsverfasser => {
                    ("Erika Planer", Unterschriftsart::Qualifiziert)
                }
                Unterschriftsrolle::Fachplaner => (
                    "Ingenieurbüro Statik Beispiel",
                    Unterschriftsart::Qualifiziert,
                ),
            };
            a.unterschriften.push(Unterschrift {
                id: Uuid::new_v4(),
                dokument: u.dokument.clone(),
                rolle: *rolle,
                name: name.into(),
                art,
                datum: tag("2026-09-14"),
            });
        }
    }
    b.info(format!(
        "Unterschriften erfasst: {} auf {} Bauvorlagen",
        a.unterschriften.len(),
        bedarf.len()
    ));
    akte_speichern(b, vid, &a, "Unterschriften").await;

    // 8. Abgabestand mit XBau und Druck festhalten.
    let (rev, _) = akte(b, vid).await;
    let abgabe = Uuid::new_v4();
    let dokumente: Vec<Dokumentbezug> = anlagen.iter().map(|(d, _, _)| bezug(d)).collect();
    let f = b
        .c
        .verfahren_abgabe_festhalten(
            b.p.id,
            vid,
            rev,
            Abgabewunsch {
                xbau_bezug: None,
                xbau_folge: None,
                id: abgabe,
                titel: "Bauantrag Neubau EFH Mustermann (Bauvorlagen mit XBau)".into(),
                dokumente: dokumente.clone(),
                xbau: true,
                druck: true,
                anschreiben: "Sehr geehrte Damen und Herren,\nanbei der Antrag auf Baugenehmigung im vereinfachten Verfahren (§ 64 BauO NRW 2018) für den Neubau eines Einfamilienhauses mit Garage, Lindenweg 12.".into(),
            },
            &b.u,
        )
        .await;
    let f = match f {
        Ok(f) => f,
        Err(e) => panic!("Abgabestand mit XBau: {e}"),
    };
    let a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let stand = a.abgaben.iter().find(|x| x.id == abgabe).unwrap().clone();
    for h in &stand.hinweise {
        b.info(format!("Abgabe-Hinweis: {h}"));
    }
    let (_, zip) =
        b.c.verfahren_artefakt(b.p.id, vid, abgabe, "xbau", &b.u)
            .await
            .unwrap();
    let pruefung = core_lib::xbau::austausch::pruefen("XBau.zip", &zip, b.c.xbau_pruefer.as_ref())
        .await
        .expect("XBau-Paket schemagültig");
    b.info(format!(
        "XBau geprüft: {} ({} Anlagen, Nachricht {})",
        pruefung.kopf.element,
        pruefung.anlagen.len(),
        pruefung.kopf.id
    ));
    let mut z = zip::ZipArchive::new(Cursor::new(zip.to_vec())).unwrap();
    let mut xml = String::new();
    let name = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().unwrap().to_string())
        .find(|n| n.ends_with(".xml"))
        .unwrap();
    z.by_name(&name).unwrap().read_to_string(&mut xml).unwrap();
    xbau_plausibel(b, &xml, &d);
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let r =
        b.c.projekt_baugrundstuecke_pruefen(b.p.id, &plan)
            .await
            .unwrap();
    let vg: Vec<_> = r.baugrundstuecke[0]
        .pruefung
        .nachweise
        .iter()
        .filter(|n| n.art == "vollgeschosse")
        .map(|n| (n.gegenstand.clone(), n.ist, n.soll))
        .collect();
    b.info(format!(
        "Nachweise „vollgeschosse“ (Gegenstand, ist, soll): {vg:?}"
    ));
    b.c.verfahren_artefakt(b.p.id, vid, abgabe, "pdf", &b.u)
        .await
        .expect("Druckausgabe");
    let journal =
        b.c.verfahren_xbau_erfassen(b.p.id, vid, abgabe, &b.u)
            .await
            .unwrap();
    assert_eq!(journal.nachricht.richtung, Richtung::Ausgang);

    // 9. Einreichung über Bauportal.NRW am 15.09.2026 (außerhalb der App).
    let (rev, _) = akte(b, vid).await;
    let extern_id = Uuid::new_v4();
    b.c.verfahren_externe_einreichung(
        b.p.id,
        vid,
        rev,
        ExterneEinreichungswunsch {
            id: extern_id,
            datum: tag("2026-09-15"),
            weg: Einreichungsweg::Portal,
            beleg: Some(bezug(&d.quittung)),
            bemerkung: "Bauportal.NRW, Vorgangsnummer BP-NRW-2026-447120; Unterlagen wie Abgabestand „Bauantrag Neubau EFH Mustermann“".into(),
            dokumente: dokumente.clone(),
            berichtigt: None,
        },
        &b.u,
    )
    .await
    .unwrap();
    // Versuch, den Versand des Abgabestands rückwirkend zu erfassen (erwartete Ablehnung).
    let (_, mut versuch) = akte(b, vid).await;
    versuch
        .abgaben
        .iter_mut()
        .find(|x| x.id == abgabe)
        .unwrap()
        .versand
        .push(abgabe::Versandereignis {
            id: Uuid::new_v4(),
            status: abgabe::Versandstatus::Versendet,
            datum: tag("2026-09-15"),
            weg: "Bauportal.NRW".into(),
            referenz: "BP-NRW-2026-447120".into(),
            notiz: "Rückwirkend".into(),
            beleg: Some(bezug(&d.quittung)),
            berichtigt: None,
        });
    let (rev, _) = akte(b, vid).await;
    let e =
        b.c.fachobjekt_speichern(
            b.p.id,
            ObjektArt::Verfahren,
            vid,
            rev,
            serde_json::to_value(&versuch).unwrap(),
            &b.u,
        )
        .await
        .expect_err("Versand vor dem Festhalten");
    b.info(format!("Rückwirkender Versand abgewiesen (erwartet): {e}"));
    let (_, mut a) = akte(b, vid).await;
    let k_einreichung = Uuid::new_v4();
    a.kommunikation.push(Kommunikation {
        id: k_einreichung,
        art: Kommunikationsart::Portal,
        eingehend: false,
        datum: tag("2026-09-15"),
        betreff: "Bauantrag über Bauportal.NRW übermittelt".into(),
        beteiligte: format!("Erika Planer → {BEHOERDE}"),
        inhalt: "Antrag auf Baugenehmigung im vereinfachten Verfahren mit 11 Bauvorlagen".into(),
        original: Some(bezug(&d.quittung)),
        bezuege: vec![Bezug::Abgabe { id: abgabe }],
    });
    a.ereignisse.push(Aktenereignis {
        id: Uuid::new_v4(),
        datum: tag("2026-09-14"),
        erfasst_am: chrono::Utc::now(),
        erfasst_von: b.u.id.0,
        text: "Bauantrag und Bauvorlagen vom Bauherrn Max Mustermann unterschrieben".into(),
        beleg: Some(bezug(&formular)),
        berichtigt: None,
        arten: vec![],
        berichtigung: None,
    });
    a.status = Verfahrensstatus::Eingereicht;
    akte_speichern(b, vid, &a, "eingereicht").await;

    // 10. Eingangsbestätigung als XBau-Nachricht (Kernmodul 1120).
    let vorgang = b.c.bauantrag(b.p.id).await.unwrap().antrag.vorgang;
    let n_id = Uuid::new_v4();
    let xml1120 = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<k:prozessnachrichten.eingangsbestaetigung.1120 xmlns:k="http://www.xleitstelle.de/xbau/kernmodul/1/4/0" produkt="Bauportal" produkthersteller="Land NRW" standard="XBau-Kernmodul" version="1.4.0"><nachrichtenkopf.g2g><identifikation.nachricht><nachrichtenUUID>{n_id}</nachrichtenUUID><nachrichtentyp listURI="urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten" listVersionID="3.0"><code>1120</code></nachrichtentyp><erstellungszeitpunkt>2026-09-17T08:15:00Z</erstellungszeitpunkt></identifikation.nachricht><leser><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>{BUERO_KENNUNG}</kennung><name>Planungsbüro Planer Architekten</name></leser><autor><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>{BEHOERDE_KENNUNG}</kennung><name>{BEHOERDE}</name></autor></nachrichtenkopf.g2g><bezug><referenz>{vorgang}</referenz><vorgang>{AKTENZEICHEN}</vorgang></bezug></k:prozessnachrichten.eingangsbestaetigung.1120>"#
    );
    let dx =
        b.c.store_document_kategorisiert(
            b.p.id,
            "Eingangsbestaetigung_1120.xml",
            Bytes::from(xml1120),
            Some("schriftverkehr".into()),
            &b.u,
        )
        .await
        .unwrap();
    let n =
        b.c.xbau_aufnehmen(
            b.p.id,
            Nachrichtenaufnahme {
                original: bezug(&dx),
                richtung: Richtung::Eingang,
                zuordnung_bestaetigt: true,
            },
            &b.u,
        )
        .await
        .unwrap();
    let (rev, _) = akte(b, vid).await;
    match b
        .c
        .xbau_in_verfahren(b.p.id, n.id, vid, rev, Some(tag("2026-09-17")), &b.u)
        .await
    {
        Ok(_) => {}
        Err(e) => b.befund(format!(
            "Eingangsbestätigung (XBau 1120) nicht in die Akte übernommen: {e}"
        )),
    }
    let (_, a) = akte(b, vid).await;
    if a.aktenzeichen != AKTENZEICHEN {
        b.befund(format!(
            "Aktenzeichen aus der Eingangsbestätigung nicht übernommen: „{}“",
            a.aktenzeichen
        ));
    }
    b.info("Eingangsbestätigung vom 17.09. bezieht sich auf die erst am Stichtag erzeugte XBau-Nachricht; der Core prüft die zeitliche Reihenfolge von Antrag und Bestätigung nicht (Nacherfassung).");

    // 11. Vollständigkeit, Bearbeitungsfrist und Beteiligung der Stadtentwässerung.
    let (_, mut a) = akte(b, vid).await;
    let k_voll = Uuid::new_v4();
    a.kommunikation.push(Kommunikation {
        id: k_voll,
        art: Kommunikationsart::Brief,
        eingehend: true,
        datum: tag("2026-09-29"),
        betreff: "Bestätigung der Vollständigkeit".into(),
        beteiligte: BEHOERDE.into(),
        inhalt: "Bauantrag vollständig; Entscheidung bis 10.11.2026; Beteiligung Stadtentwässerung bis 27.10.2026".into(),
        original: Some(bezug(&d.vollstaendigkeit)),
        bezuege: vec![],
    });
    a.vollstaendigkeit = Some(Vollstaendigkeit {
        datum: tag("2026-09-29"),
        kommunikation: k_voll,
    });
    a.bearbeitungsfrist = Some(Frist {
        datum: tag("2026-11-10"),
        ausloeser: "Vollständigkeitsbestätigung vom 29.09.2026".into(),
        beleg: Some(bezug(&d.vollstaendigkeit)),
        erinnerung_tage: 7,
        grund: "Entscheidungsfrist im vereinfachten Verfahren laut Behörde (§ 64 BauO NRW 2018)"
            .into(),
    });
    a.schritte.push(Verfahrensschritt {
        id: Uuid::new_v4(),
        art: Schrittart::Fachbeteiligung,
        kommunikation: k_voll,
        frist: Some(Frist {
            datum: tag("2026-10-27"),
            ausloeser: "Vollständigkeitsbestätigung vom 29.09.2026".into(),
            beleg: Some(bezug(&d.vollstaendigkeit)),
            erinnerung_tage: 3,
            grund: "Stellungnahme Stadtentwässerung Musterstadt".into(),
        }),
        stellungnahme: None,
        verantwortlich: None,
        folgeschreiben: None,
        gruende: vec![],
    });
    a.status = Verfahrensstatus::Vollstaendig;
    akte_speichern(b, vid, &a, "Vollständigkeit").await;

    // 12. Nachforderung per E-Mail (archivierte EML mit Anhang).
    let anhang = pdf(
        "Stadt Musterstadt - Bauaufsicht - Az. 63-BA-0815/2026 - Nachforderung",
        &[
            "Für die Prüfung im Rahmen der Beteiligung der Stadtentwässerung fehlt:",
            "1. Entwässerungsnachweis (Grundstücksentwässerungsplan 1:100, Überflutungsnachweis DIN 1986-100)",
            "Bitte reichen Sie die Unterlage bis zum 05.11.2026 nach.",
        ],
    );
    let eml = format!(
        "From: =?UTF-8?Q?Bauaufsicht_Stadt_Musterstadt?= <bauaufsicht@musterstadt.example>\r\nTo: Erika Planer <e.planer@planer-architekten.example>\r\nDate: Thu, 08 Oct 2026 09:12:00 +0200\r\nSubject: =?UTF-8?Q?Az._63-BA-0815/2026_=E2=80=93_Nachforderung_Entw=C3=A4sserungsnachweis?=\r\nMessage-ID: <nf-0815-2026@musterstadt.example>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=teil\r\n\r\n--teil\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\nSehr geehrte Frau Planer,\r\nzum Bauantrag Lindenweg 12 fordern wir den Entwässerungsnachweis nach (Frist 05.11.2026). Einzelheiten siehe Anlage.\r\nMit freundlichen Grüßen\r\nBauaufsicht\r\n--teil\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=Nachforderung_2026-10-08.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\n{}\r\n--teil--\r\n",
        base64(&anhang)
    );
    let mail =
        b.c.store_document_kategorisiert(
            b.p.id,
            "Nachforderung_Entwaesserung_2026-10-08.eml",
            Bytes::from(eml),
            Some("email".into()),
            &b.u,
        )
        .await
        .unwrap();
    let gelesen = b.c.archivierte_email(mail.document_id, 1).await.unwrap();
    if gelesen.inhalt.betreff != "Az. 63-BA-0815/2026 – Nachforderung Entwässerungsnachweis" {
        b.befund(format!(
            "E-Mail-Betreff falsch gelesen: {}",
            gelesen.inhalt.betreff
        ));
    }
    assert_eq!(gelesen.inhalt.anhaenge.len(), 1);
    let ah =
        b.c.email_anhang_ablegen(mail.document_id, 1, 0, &b.u)
            .await
            .unwrap();
    let (_, inhalt) = b.c.load_document(ah.document_id).await.unwrap();
    assert_eq!(inhalt.as_ref(), anhang.as_slice(), "Anhang unverändert");
    if b.c
        .dokumentkategorien(b.p.id)
        .await
        .unwrap()
        .iter()
        .any(|k| k.dokument_id == ah.document_id && k.zuordnung.kategorie.is_none())
    {
        b.info(
            "E-Mail-Anhang ohne Kategorie abgelegt (Kategorie der Nachricht wird nicht vererbt).",
        );
    }
    // E-Mail als Kommunikation in die Akte: Datum, Betreff, Absender, Text,
    // Original und der bereits abgelegte Anhang kommen aus dem Archiv.
    let (rev, _) = akte(b, vid).await;
    let email = bezug(&mail);
    if let Err(e) =
        b.c.email_in_verfahren(b.p.id, email.clone(), vid, rev, true, None, &b.u)
            .await
    {
        b.befund(format!("E-Mail in die Verfahrensakte übernehmen: {e}"));
    }
    let k_nf = core_lib::email::verfahren::kommunikation_id(&email);
    let (_, mut a) = akte(b, vid).await;
    match a.kommunikation.iter().find(|k| k.id == k_nf) {
        Some(k) => {
            if k.datum != tag("2026-10-08") || k.betreff != gelesen.inhalt.betreff {
                b.befund(format!(
                    "E-Mail-Übernahme: Datum {} / Betreff „{}“",
                    k.datum, k.betreff
                ));
            }
            if !k
                .bezuege
                .iter()
                .any(|x| matches!(x, Bezug::Dokument { dokument } if dokument.dokument_id == ah.document_id))
            {
                b.befund("E-Mail-Übernahme: abgelegter Anhang nicht verknüpft");
            }
            b.info(format!(
                "E-Mail übernommen: {} · {}",
                k.datum,
                k.beteiligte.replace('\n', " · ")
            ));
        }
        None => b.befund("E-Mail-Übernahme: Kommunikation fehlt in der Akte"),
    }
    let punkt = Uuid::new_v4();
    a.nachforderungen.push(Nachforderung {
        id: Uuid::new_v4(),
        kommunikation: k_nf,
        punkte: vec![Forderungspunkt {
            id: punkt,
            titel: "Entwässerungsnachweis (Grundstücksentwässerungsplan mit Überflutungsnachweis)"
                .into(),
            grundlage:
                "E-Mail der Bauaufsicht vom 08.10.2026, Anlage Nr. 1; Beteiligung Stadtentwässerung"
                    .into(),
            verantwortlich: Some(b.u.id.0),
            frist: Some(Frist {
                datum: tag("2026-11-05"),
                ausloeser: "Nachforderung vom 08.10.2026".into(),
                beleg: Some(bezug(&ah)),
                erinnerung_tage: 7,
                grund: "Frist der Bauaufsicht".into(),
            }),
            status: Punktstatus::InArbeit,
            antwort: String::new(),
            dokumente: vec![],
            abgaben: vec![],
            bestaetigung: None,
            anforderung: None,
        }],
    });
    a.kommunikation.push(Kommunikation {
        id: Uuid::new_v4(),
        art: Kommunikationsart::Telefon,
        eingehend: false,
        datum: tag("2026-10-12"),
        betreff: "Abstimmung Entwässerungsnachweis mit der Stadtentwässerung".into(),
        beteiligte: "Erika Planer, Stadtentwässerung Musterstadt (Hr. Kanal)".into(),
        inhalt:
            "Überflutungsnachweis vereinfacht nach DIN 1986-100 genügt; Einleitbeschränkung 10 l/s"
                .into(),
        original: None,
        bezuege: vec![Bezug::Kommunikation { id: k_nf }],
    });
    a.status = Verfahrensstatus::InPruefung;
    akte_speichern(b, vid, &a, "Nachforderung").await;

    // 12a. Lage am Stichtag: Genehmigungsfiktion ab Vollständigkeit (29.09.)
    // im vereinfachten Verfahren nach sechs Wochen; Entwurf der Antwort auf
    // die Nachforderung; Auswertung der Eingangsbestätigung.
    let lage = b.c.verfahren_optionen(b.p.id, vid).await.unwrap();
    for o in &lage.optionen {
        b.info(format!(
            "Lage „{}“: {}{}",
            lage.lage,
            o.titel,
            o.frist.map(|f| format!(" bis {f}")).unwrap_or_default()
        ));
    }
    // Die Lage rechnet mit dem heutigen Tag: bis zum 10.11.2026 läuft die
    // Frist, danach nennt sie den Ablauf und die Bescheinigung.
    let laeuft = lage
        .optionen
        .iter()
        .any(|o| o.titel == "Genehmigungsfiktion" && o.frist == Some(tag("2026-11-10")));
    let abgelaufen = lage
        .lage
        .contains("Fiktionsfrist seit 10.11.2026 abgelaufen")
        && lage
            .optionen
            .iter()
            .any(|o| o.titel == "Bescheinigung über die Genehmigungsfiktion verlangen");
    if !laeuft && !abgelaufen {
        b.befund(format!(
            "Genehmigungsfiktion nicht zum 10.11.2026 (Entscheidungsfrist der Behörde): {}",
            lage.lage
        ));
    }
    let nf = a
        .nachforderungen
        .iter()
        .find(|n| n.kommunikation == k_nf)
        .unwrap()
        .id;
    let (antwort, antwort_pdf) =
        b.c.verfahren_schreiben(
            b.p.id,
            vid,
            core_lib::verfahren::schreiben::Schreibenwunsch {
                art: core_lib::verfahren::schreiben::Schreibenart::Nachforderungsantwort {
                    nachforderung: nf,
                },
                datum: Some(stichtag()),
                zusatz: String::new(),
                ablegen: false,
            },
            &b.u,
        )
        .await
        .unwrap();
    assert!(antwort_pdf.starts_with(b"%PDF-"));
    for h in &antwort.hinweise {
        b.info(format!("Antwortschreiben (Entwurf): {h}"));
    }
    let auswertung =
        b.c.xbau_auswertung(b.p.id, n.id, Some(vid), &b.u)
            .await
            .unwrap();
    b.info(format!(
        "XBau-Auswertung {}: {}",
        auswertung.element,
        auswertung.wirkung.join(" · ")
    ));

    // 13. Aufgaben und Projektorganisation.
    aufgaben(b, punkt).await;

    // 14. Portal: NRW-Profil, Bereitstellung der eingefrorenen Abgabe.
    let profile = b.c.portalprofile().await.unwrap();
    let nrw = profile
        .iter()
        .find(|p| p.bundesland == Some(core_lib::domain::Bundesland::Nw))
        .cloned();
    let mut portalbereitstellung = None;
    if let Some(nrw) = nrw {
        b.info(format!(
            "Portalprofil NRW: {} (Freigabe {:?})",
            nrw.titel, nrw.freigabe
        ));
        let (rev, _) = akte(b, vid).await;
        let pv = Uuid::new_v4();
        b.c.portal_vorgang_anlegen(
            b.p.id,
            vid,
            rev,
            serde_json::from_value(json!({"id": pv, "profil_id": nrw.id, "profil_version": nrw.version,
                "ziel": {"kommune": "Stadt Musterstadt", "stelle": BEHOERDE, "behoerdenkennung": BEHOERDE_KENNUNG,
                    "url": "https://bauportal.nrw/", "zuordnung_bestaetigt": true}}))
            .unwrap(),
            &b.u,
        )
        .await
        .unwrap();
        let w: core_lib::portale::Portalabgabewunsch = serde_json::from_value(json!({
            "abgabe": abgabe, "art": "erstantrag", "vorgaenger": null, "forderungen": [],
            "dateien": stand.dokumente.iter().map(|x| json!({"dokument": x.dokument, "unterlagenart": null,
                "uploadname": x.name, "ersetzt": null})).collect::<Vec<_>>()
        }))
        .unwrap();
        let v =
            b.c.portal_abgabe_pruefen(b.p.id, vid, pv, &w, &b.u)
                .await
                .unwrap();
        for h in v.pruefung.hinweise.iter().chain(&v.pruefung.fehler) {
            b.info(format!("Portalprüfung: {h}"));
        }
        let (rev, _) = akte(b, vid).await;
        match b
            .c
            .portal_abgabe_festhalten(b.p.id, vid, pv, rev, w, &b.u)
            .await
        {
            Ok(_) => {
                let (_, zip) =
                    b.c.portal_bereitstellung(b.p.id, vid, pv, abgabe, &b.u)
                        .await
                        .unwrap();
                portalbereitstellung = Some(zip);
            }
            Err(e) => b.befund(format!("Portalbereitstellung abgelehnt: {e}")),
        }
        b.info("Portalereignisse dürfen nicht vor dem Festhalten liegen; die nachträglich festgehaltene Portalabgabe gilt durch die externe Einreichung vom 15.09. (Portalquittung, gleiche Dateiversionen) als belegt.");
    } else {
        b.befund("Kein Portalprofil für NRW vorhanden.");
    }

    // 15. Einreichungspaket: Der eingefrorene Abgabestand ist das Paket der
    // eingereichten Unterlagen (Abgabestand.json + Originale). Neue Pakete des
    // früheren Einreichungsmoduls entstehen nur noch in Testdaten
    // (einreichung_erstellen ist #[cfg(feature = "testdaten")]) und würden im
    // Beispiel als unbestätigtes Altpaket erscheinen; geprüft wird hier der
    // produktive Einreichungsvergleich.
    let (_, paket_zip) =
        b.c.verfahren_artefakt(b.p.id, vid, abgabe, "zip", &b.u)
            .await
            .unwrap();
    let mut pz = zip::ZipArchive::new(Cursor::new(paket_zip.to_vec())).unwrap();
    let namen: Vec<String> = (0..pz.len())
        .map(|i| pz.by_index(i).unwrap().name().unwrap().to_string())
        .collect();
    assert!(namen.iter().any(|n| n == "Abgabestand.json"));
    assert_eq!(
        namen
            .iter()
            .filter(|n| n.starts_with("Unterlagen/"))
            .count(),
        11,
        "{namen:?}"
    );
    let pruefstand =
        b.c.paket_pruefstand(
            b.p.id,
            &dokumente
                .iter()
                .map(|x| core_lib::einreichung::DokumentAuswahl {
                    dokument_id: x.dokument_id,
                    version: x.version,
                })
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    b.info(format!(
        "Prüfstand der eingereichten Unterlagen: fehlende Pflichtunterlagen {:?}, ungeprüft {:?}",
        pruefstand.nicht_enthaltene_pflichtunterlagen, pruefstand.ungepruefte_unterlagen
    ));
    let vergleich = b.c.einreichungs_vergleich(b.p.id, BEHOERDE).await.unwrap();
    let neu = vergleich
        .iter()
        .filter(|v| {
            dokumente
                .iter()
                .any(|x| x.dokument_id == v.aktuell.document_id)
        })
        .filter(|v| v.status == core_lib::einreichung::VergleichsStatus::Neu)
        .count();
    assert_eq!(
        neu, 0,
        "Einreichungsvergleich ({BEHOERDE}): extern eingereichte Bauvorlagen gelten als „Neu“"
    );
    assert!(
        vergleich
            .iter()
            .filter(|v| dokumente
                .iter()
                .any(|x| x.dokument_id == v.aktuell.document_id))
            .all(|v| v.verfahren == Some(vid) && v.eingereicht_am == Some(tag("2026-09-15")))
    );

    // 16. Planindex und Fristen/Kalender.
    let plaene = b.c.verfahren_planunterlagen(b.p.id, vid).await.unwrap();
    let entwurf = plaene
        .iter()
        .filter(|u| {
            u.status == planindex::Einreichungsstatus::Entwurf
                && dokumente.iter().any(|x| x.dokument_id == u.dokument_id)
        })
        .count();
    assert_eq!(
        entwurf, 0,
        "Planunterlagen: extern eingereichte Bauvorlagen stehen weiter als „Entwurf“"
    );
    assert!(
        plaene
            .iter()
            .filter(|u| dokumente.iter().any(|x| x.dokument_id == u.dokument_id))
            .all(|u| u.planindex.as_deref() == Some("A") && !u.name.is_empty())
    );
    let termine = b.c.verfahren_termine(b.p.id, stichtag()).await.unwrap();
    for t in &termine {
        b.info(format!("Termin {}: {} ({:?})", t.datum, t.titel, t.art));
    }
    let ics = b.c.verfahren_termine_ics(b.p.id, stichtag()).await.unwrap();
    assert!(
        ics.contains("BEGIN:VCALENDAR") && ics.contains("20261105"),
        "{ics}"
    );
    if !ics.contains("Entwässerungsnachweis bei der Bauaufsicht nachreichen") {
        b.info(
            "Kalender (ICS) enthält nur Verfahrenstermine; fällige Projektaufgaben fehlen dort.",
        );
    }
    let erinnerungen =
        b.c.verfahren_erinnerungen(b.p.id, stichtag())
            .await
            .unwrap();
    for e in &erinnerungen {
        b.info(format!(
            "Wiedervorlage {}: {}{}",
            e.datum,
            e.titel,
            if e.ueberfaellig {
                " (überfällig)"
            } else {
                ""
            }
        ));
    }
    assert_eq!(
        core_lib::kalender::naechster_werktag(core_lib::domain::Bundesland::Nw, tag("2026-11-01")),
        tag("2026-11-02"),
        "Allerheiligen in NRW"
    );

    // 17. Projektübersicht am Stichtag.
    b.neu_laden().await;
    let s =
        b.c.projektuebersicht_am(b.p.id, stichtag(), &b.u)
            .await
            .unwrap();
    let mut offene_punkte = Vec::new();
    for p in &s.punkte {
        let t = format!(
            "{:?}/{:?}: {} – {}",
            p.thema, p.dringlichkeit, p.titel, p.hinweis
        );
        b.info(format!("Offen: {t}"));
        offene_punkte.push(t);
    }
    for v in &s.verfahren {
        b.info(format!(
            "Verfahren: {} · {} · Az. {} · {} offene Punkte · {} Abgaben ({} versendet)",
            v.bezeichnung, v.status, v.aktenzeichen, v.offene_punkte, v.abgaben, v.versendet
        ));
    }
    for a in &s.abgaben {
        b.info(format!(
            "Abgabe: {} · {} · {:?} · {} Dokumente",
            a.titel, a.status, a.datum, a.dokumente
        ));
    }
    // Ein Zustand je Einreichung: Der nachträglich festgehaltene Abgabestand
    // und die externe Einreichung, die ihn belegt, sind eine Zeile mit einer
    // Aussage; dieselbe steht am Abgabestand der Akte.
    let belegt: Vec<_> = s
        .abgaben
        .iter()
        .filter(|a| a.status == "Extern eingereicht (belegt)")
        .collect();
    assert!(!belegt.is_empty(), "belegter Abgabestand in der Übersicht");
    assert!(belegt.iter().all(|a| a.datum == Some(tag("2026-09-15"))));
    assert!(
        !s.abgaben.iter().any(|a| a.status == "Extern eingereicht"),
        "die belegende externe Einreichung ist keine zweite Zeile"
    );
    assert!(s.verfahren.iter().any(|v| v.versendet >= 1));
    let (_, a) = akte(b, vid).await;
    assert!(a.abgaben.iter().any(|x| {
        x.lage.as_ref().is_some_and(|l| {
            l.text == "Extern eingereicht (belegt)"
                && l.eingereicht
                && l.datum == Some(tag("2026-09-15"))
        })
    }));
    let empfang = s
        .empfaenger
        .iter()
        .find(|e| e.empfaenger.contains(BEHOERDE))
        .expect("Empfängerstand der Bauaufsicht aus der externen Einreichung");
    assert_eq!(empfang.dokumente.len(), dokumente.len());
    assert!(
        empfang
            .dokumente
            .iter()
            .all(|d| d.eingereicht_am == tag("2026-09-15"))
    );
    // Nachträglich festgehaltener Abgabestand und Portalabgabe sind durch die
    // externe Einreichung mit Portalquittung belegt.
    for p in &s.punkte {
        assert!(
            !p.hinweis.contains("Versand unbestätigt")
                && !p.titel.contains("Portalabgabe und Dateistand prüfen"),
            "{} – {}",
            p.titel,
            p.hinweis
        );
    }

    Verfahrensergebnis {
        verfahren: vid,
        abgabe,
        xbau_xml: xml,
        xbau_hinweise: stand.hinweise.clone(),
        formular,
        formularwerte,
        beispielformular,
        paket_zip: paket_zip.to_vec(),
        portalbereitstellung,
        offene_punkte,
    }
}

/// Inhalt der Antragsnachricht gegen die Projektdaten prüfen.
fn xbau_plausibel(b: &mut Beispiel, xml: &str, d: &Dokumente) {
    let enthaelt = |s: &str| xml.contains(s);
    for (was, muster) in [
        ("Nachrichtentyp 0200", "baugenehmigung.antrag.0200"),
        ("Behördenkennung", BEHOERDE_KENNUNG),
        ("Absenderkennung Büro", BUERO_KENNUNG),
        ("Bauherr Familienname", "<name>Mustermann</name>"),
        ("Bauherr Vorname", "<name>Max</name>"),
        ("Bauherr E-Mail", "max.mustermann@example.org"),
        ("Entwurfsverfasserin", "<name>Planer</name>"),
        (
            "Kammer-Mitgliedsnummer",
            "<xbau:mitgliedsnummer>A-12345</xbau:mitgliedsnummer>",
        ),
        ("Kammerland NRW (05)", "<code>05</code>"),
        ("Berufsbezeichnung", "Architektin"),
        ("Gemarkungsnummer", "<nummer>1234</nummer>"),
        ("Flur", "<flur>3</flur>"),
        ("Flurstück Zähler", "<zaehler>412</zaehler>"),
        ("Anschrift Baugrundstück", "<strasse>Lindenweg</strasse>"),
        (
            "Anzahl Gebäude",
            "<xbau:anzahlDerGebaeudeImBauvorhaben>2</xbau:anzahlDerGebaeudeImBauvorhaben>",
        ),
        ("Errichtung (11)", "<code>11</code>"),
        ("Gebäudeklasse 1", "<xbau:gebaeudeklasse"),
        (
            "Grundstücksfläche",
            "<xbau:flaecheDesBaugrundstuecks>620.00</xbau:flaecheDesBaugrundstuecks>",
        ),
        (
            "Grundfläche",
            "<xbau:grundflaeche>119.00</xbau:grundflaeche>",
        ),
        (
            "GRZ Soll",
            "<xbau:grundflaechenzahlSoll>0.40</xbau:grundflaechenzahlSoll>",
        ),
        (
            "GRZ geplant",
            "<xbau:grundflaechenzahlGeplant>0.19</xbau:grundflaechenzahlGeplant>",
        ),
        (
            "Geschossfläche",
            "<xbau:geschossflaeche>99.00</xbau:geschossflaeche>",
        ),
        (
            "GFZ Soll",
            "<xbau:geschossflaechenzahlSoll>0.80</xbau:geschossflaechenzahlSoll>",
        ),
        (
            "GFZ geplant",
            "<xbau:geschossflaechenzahlGeplant>0.16</xbau:geschossflaechenzahlGeplant>",
        ),
        ("Fachplaner Tragwerk", "Ingenieurbüro Statik Beispiel"),
        ("Anlage Lageplan 1804", "<code>1804</code>"),
        ("Anlage Grundbuchauszug 2008", "<code>2008</code>"),
        ("Umlaut-Dateiname", "Ansichten_Nord_Süd_Ost_West.pdf"),
    ] {
        if !enthaelt(muster) {
            b.befund(format!(
                "XBau-Antrag: {was} fehlt oder weicht ab (erwartet „{muster}“)"
            ));
        }
    }
    // Wohnhaus (I) + Garage (0): größte Zahl je Gebäude, Festsetzung II.
    assert!(enthaelt(
        "<xbau:anzahlVollgeschosse>1</xbau:anzahlVollgeschosse>"
    ));
    assert!(enthaelt(
        "<xbau:zulaessigeAnzahlVollgeschosse>2</xbau:zulaessigeAnzahlVollgeschosse>"
    ));
    if enthaelt("<code>02</code>\n")
        || xml.contains("beteiligterrolle\" listVersionID=\"2.0\"><code>02</code>")
    {
        b.info("XBau-Antrag: Der Bauherr erscheint zusätzlich als weiterer Beteiligter „Grundstückseigentümer“ (Rolle 02, Fachrichtung 99), weil er am Flurstück als Eigentümer zugeordnet ist.");
    }
    if xml.matches("<anlage>").count() != 11 {
        b.befund(format!(
            "XBau-Antrag: {} statt 11 Anlagen",
            xml.matches("<anlage>").count()
        ));
    }
    if !enthaelt("vereinfacht") && !enthaelt("xbau:verfahrensart") {
        b.info("XBau-Antrag ohne Verfahrensart: Das vereinfachte Verfahren ist in der Nachricht nicht erkennbar.");
    }
    // Tragwerksplaner mit Fachrichtung 1 (Tragwerksplanung/Standsicherheit).
    let kompakt: String = xml.lines().map(str::trim).collect();
    if !kompakt.contains("beteiligterfachrichtung\" listVersionID=\"3.0\"><code>1</code>") {
        b.befund("XBau-Antrag: Fachrichtung des Tragwerksplaners (Code 1) fehlt");
    }
    // Anlagendatum: Plandatum der Dokumentdaten (Grundrisse 08.09., Lageplan 02.09.).
    assert!(enthaelt("<datum>2026-09-08</datum>") && enthaelt("<datum>2026-09-02</datum>"));
    let _ = d;
    eprintln!(
        "--- XBau-Nachricht ---\n{}\n--- Ende XBau ---",
        xml.replace("><", ">\n<")
    );
}
