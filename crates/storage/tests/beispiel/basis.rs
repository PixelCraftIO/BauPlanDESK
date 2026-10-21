//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW): Profil, Büro,
//! Bauherr, Projekt mit Bebauungsplan, Flurstücke, Baugrundstück und Gebäude.
//!
//! Alle Daten laufen über dieselben Core-Dienste und Bereichsspeicherungen wie
//! in der Oberfläche, damit Validierungen und Berechnungen wie im Betrieb
//! greifen. Befunde (Hinweise des Cores, Auffälligkeiten) werden gesammelt und
//! am Ende ausgegeben.
#![allow(dead_code)]
use chrono::NaiveDate;
use core_lib::{
    CoreServices,
    benutzer::{Benutzer, NeuerBenutzer, Rolle},
    domain::*,
    grundstuecksinformationen::{BaugrundstuecksErmittlung, Grundstuecksinformationen},
    grundstueckskontext::GrundstueckKontext,
    kunden::{Kunde, Kundendaten},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    projektbereiche::ProjektBereich,
    zugriff::{ProjektMitglied, ProjektRecht},
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

/// Fester Kalendertag des Beispiels (Fristen, Aufgaben, Verfahrensdaten).
pub fn stichtag() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()
}
/// Gebäudenullpunkt ±0,00 = OK Fertigfußboden EG.
pub const NULLPUNKT_NHN: f64 = 72.64;
pub const GRUNDSTUECK_M2: f64 = 620.;

pub struct Beispiel {
    pub c: CoreServices,
    pub db: db_sqlite::SqliteDatabase,
    /// Profilverzeichnis wie in der App: `bauprojekt.db`, `dateien/`, …
    pub profil: PathBuf,
    /// Bearbeitende Architektin (Projektleitung, kein Administrator).
    pub u: Benutzer,
    /// Systemkonto für Büroeinstellungen; erzeugt keinen echten Administrator.
    pub system: Benutzer,
    pub p: Project,
    pub bauherr: Kunde,
    pub tragwerksplaner: Kunde,
    pub entwurfsverfasser: Uuid,
    pub grund: Option<GrundstueckKontext>,
    pub haus: Uuid,
    pub garage: Uuid,
    pub kg: Uuid,
    pub eg: Uuid,
    pub dg: Uuid,
    pub befunde: Vec<String>,
    /// Funktionslücken ohne Fehler in Eintragung oder Ergebnis (Ausbauvorschläge).
    pub ausbau: Vec<String>,
}

impl Beispiel {
    pub fn grund(&self) -> &GrundstueckKontext {
        self.grund.as_ref().expect("Baugrundstück ermittelt")
    }
    /// Erwartete Information des Cores (kein Problem), nur zur Nachvollziehbarkeit.
    pub fn info(&self, text: impl Into<String>) {
        eprintln!("INFO: {}", text.into());
    }
    /// Fehlende Funktion, die den Ablauf erleichtern würde; das Ergebnis ist korrekt.
    pub fn ausbau(&mut self, text: impl Into<String>) {
        let t = text.into();
        eprintln!("AUSBAU: {t}");
        self.ausbau.push(t);
    }
    /// Problem bei Eintragung oder Berechnung, das behoben oder bewertet werden muss.
    pub fn befund(&mut self, text: impl Into<String>) {
        let t = text.into();
        eprintln!("BEFUND: {t}");
        self.befunde.push(t);
    }
    /// Projekt nach einer Änderung an anderer Stelle neu laden.
    pub async fn neu_laden(&mut self) {
        self.p = self.c.get_project(self.p.id).await.unwrap();
    }
    pub fn aenderung(&self, recht: Planungsrecht) -> ProjectUpdate {
        ProjectUpdate {
            id: self.p.id,
            expected_version: self.p.version,
            name: self.p.name.clone(),
            bundesland: self.p.bundesland,
            grundstuecksflaeche_m2: self.p.grundstuecksflaeche_m2,
            planungsrecht: Some(recht),
        }
    }
    pub async fn bereich(&mut self, bereich: ProjektBereich, recht: Planungsrecht) {
        let neu = self
            .c
            .projekt_bereich_speichern(self.p.clone(), bereich, self.aenderung(recht), &self.u)
            .await
            .unwrap_or_else(|e| panic!("Bereich {bereich:?} speichern: {e}"));
        self.p = neu;
    }
    /// Fachobjekt neu anlegen oder auf dem aktuellen Stand weiterschreiben.
    pub async fn speichern(
        &self,
        art: ObjektArt,
        id: Uuid,
        daten: Value,
    ) -> core_lib::commit::Fachstand {
        let basis = self
            .c
            .fachobjekte(self.p.id, art)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .map(|f| f.revision)
            .unwrap_or(0);
        self.c
            .fachobjekt_speichern(self.p.id, art, id, basis, daten, &self.u)
            .await
            .unwrap_or_else(|e| panic!("{art:?} speichern: {e}"))
    }
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Core wie in der Desktop-App (Einzelplatz): SQLite-Datei, lokale Ablage,
/// mitgelieferte LBO-Regeln, Formularvorlagen, Codelisten und XBau-Prüfer.
pub async fn profil_oeffnen(profil: &std::path::Path) -> (CoreServices, db_sqlite::SqliteDatabase) {
    std::fs::create_dir_all(profil).unwrap();
    let db = db_sqlite::SqliteDatabase::open(&profil.join("bauprojekt.db"))
        .await
        .unwrap();
    let lbo = LboEngine::new();
    let (_, fehler) = lbo.load_directory(&repo().join("lbo-rules")).unwrap();
    assert!(fehler.is_empty(), "LBO-Regeln: {fehler:?}");
    let mut formulare = core_lib::formular::Formularkatalog::default();
    let fehler = formulare.laden(&repo().join("formulare")).unwrap();
    assert!(fehler.is_empty(), "Formulare: {fehler:?}");
    let mut codelisten = core_lib::xbau::codeliste::Codelisten::eingebaut();
    let fehler = codelisten.laden(&repo().join("codelisten")).unwrap();
    assert!(fehler.is_empty(), "Codelisten: {fehler:?}");
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(profil.join("dateien"), "local").unwrap()),
        Arc::new(lbo),
        Arc::new(DevelopmentLicense),
    )
    .mit_xbau_pruefer(storage::xbau::XmlLintPruefer::standard())
    .mit_formularen(formulare)
    .mit_codelisten(codelisten);
    (c, db)
}

pub fn anschrift(strasse: &str, nr: &str, plz: &str, ort: &str) -> core_lib::xbau::Anschrift {
    core_lib::xbau::Anschrift {
        strasse: strasse.into(),
        hausnummer: nr.into(),
        postleitzahl: plz.into(),
        ort: ort.into(),
    }
}

fn person(
    vorname: &str,
    name: &str,
    a: core_lib::xbau::Anschrift,
    mail: &str,
    tel: &str,
) -> Kundendaten {
    serde_json::from_value(json!({
        "typ": "person", "kategorie": "privat", "kundennummer": "", "vorname": vorname,
        "familienname": name, "firma": "", "ansprechpartner": "", "anschrift": a,
        "telefon": tel, "email": mail, "notizen": ""
    }))
    .unwrap()
}

pub async fn basis(profil: PathBuf) -> Beispiel {
    let (c, db) = profil_oeffnen(&profil).await;
    let system = c.systembenutzer().await.unwrap();
    // Keine echte Administratorin: Wer das Profil wiederherstellt, wird beim
    // ersten Start Administrator (Einzelplatzanmeldung über das Rechnerkonto).
    let u = c
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "e.planer".into(),
                anzeigename: "Erika Planer".into(),
                rolle: Rolle::Projektleitung,
            },
            &system,
        )
        .await
        .unwrap();
    let ev = Uuid::new_v4();
    let buero: core_lib::xbau::Buero = serde_json::from_value(json!({
        "name": "Planungsbüro Planer Architekten",
        "anschrift": anschrift("Am Markt", "5", "59999", "Musterstadt"),
        "kennung": "",
        "entwurfsverfasser": [{"id": ev, "vorname": "Erika", "familienname": "Planer",
            "berufsbezeichnung": "Architektin", "mitgliedsnummer": "A-12345",
            "kammerland": "NW", "nachweis": ""}]
    }))
    .unwrap();
    c.buero_speichern(buero, &system).await.unwrap();

    let bauherr = c
        .kunde_anlegen(
            person(
                "Max",
                "Mustermann",
                anschrift("Musterstraße", "1", "59999", "Musterstadt"),
                "max.mustermann@example.org",
                "02345 678901",
            ),
            &u,
        )
        .await
        .unwrap();
    let tragwerksplaner = c
        .kunde_anlegen(
            serde_json::from_value(json!({
                "typ": "organisation", "kategorie": "planungsbuero", "kundennummer": "",
                "vorname": "", "familienname": "", "firma": "Ingenieurbüro Statik Beispiel",
                "ansprechpartner": "Tobias Träger", "anschrift": anschrift("Brückenweg", "8", "59999", "Musterstadt"),
                "telefon": "02345 111222", "email": "statik@example.org", "notizen": "Tragwerksplanung und Wärmeschutz"
            }))
            .unwrap(),
            &u,
        )
        .await
        .unwrap();

    let p = c
        .create_project(
            NewProject {
                name: "Neubau Einfamilienhaus Mustermann".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht::default(),
            },
            &u,
        )
        .await
        .unwrap();
    // Projektleitung als Mitglied, damit die Akte auch ohne Administratorrolle bearbeitbar ist.
    if c.eigenes_projektrecht(p.id, &u).await.ok() != Some(ProjektRecht::Leitung) {
        c.projektmitglied_setzen(
            p.id,
            ProjektMitglied {
                benutzer_id: u.id,
                recht: ProjektRecht::Leitung,
            },
            0,
            &system,
        )
        .await
        .unwrap();
    }
    let mut b = Beispiel {
        c,
        db,
        profil,
        u,
        system,
        p,
        bauherr,
        tragwerksplaner,
        entwurfsverfasser: ev,
        grund: None,
        haus: Uuid::new_v4(),
        garage: Uuid::new_v4(),
        kg: Uuid::new_v4(),
        eg: Uuid::new_v4(),
        dg: Uuid::new_v4(),
        befunde: vec![],
        ausbau: vec![],
    };
    b.neu_laden().await;
    let p =
        b.c.bauherren_setzen(b.p.id, vec![b.bauherr.id], b.p.version, &b.u)
            .await
            .unwrap();
    b.p = p;
    let p =
        b.c.beteiligte_setzen(
            b.p.id,
            serde_json::from_value(json!([{"kunde": b.tragwerksplaner.id, "rolle": "fachplaner", "fachrichtung": "Tragwerksplanung/Standsicherheit"}]))
                .unwrap(),
            b.p.version,
            &b.u,
        )
        .await
        .unwrap();
    b.p = p;

    // Stammdaten und Rechtsgrundlagen.
    let mut r = b.p.planungsrecht.clone();
    r.gemeinde = "Stadt Musterstadt".into();
    r.standort = "Lindenweg 12, 59999 Musterstadt".into();
    r.vorhaben = "Neubau eines freistehenden Einfamilienhauses mit Garage".into();
    r.nutzung = "Wohnen (eine Wohneinheit)".into();
    r.begruendung = "Vorhaben im Geltungsbereich des qualifizierten Bebauungsplans Nr. 42; Festsetzungen werden eingehalten.".into();
    b.bereich(ProjektBereich::Stammdaten, r).await;
    let mut r = b.p.planungsrecht.clone();
    r.rechtsgrundlage = Some(Rechtsgrundlage::Par30);
    r.bebauungsplan_art = BebauungsplanArt::Qualifiziert;
    r.bau_nvo = Some(core_lib::flaechen::BauNvo::V2017);
    r.plan_bezeichnung = "Am Lindenweg".into();
    r.plan_nummer = "42".into();
    r.plan_rechtskraft = "2019-05-15".into();
    b.bereich(ProjektBereich::Grundlagen, r).await;
    let mut r = b.p.planungsrecht.clone();
    r.festgesetzte_werte = serde_json::from_value(json!({
        "baugebiet": "WA", "grz": 0.4, "gfz": 0.8, "vollgeschosse": 2, "geschossigkeit": "maximal"
    }))
    .unwrap();
    r.festsetzungen = "Allgemeines Wohngebiet (WA), GRZ 0,4, GFZ 0,8, höchstens II Vollgeschosse, offene Bauweise, nur Einzel- und Doppelhäuser, Satteldach 30–45°, Traufhöhe höchstens 6,50 m, Firsthöhe höchstens 10,00 m über Straßenoberkante (72,40 m ü. NHN).".into();
    b.bereich(ProjektBereich::Festsetzungen, r).await;

    // Kataster und Grundbuch: Vorhabenflurstück und zwei Nachbarn.
    let fl = |id: &str,
              nummer: &str,
              flaeche: f64,
              rolle: &str,
              eigentuemer: &str,
              kunden: Vec<Value>| {
        json!({"id": id, "gemarkung": "Musterstadt", "gemarkungsnummer": "1234", "flur": "3",
            "nummer": nummer, "flaeche_m2": flaeche, "grundbuchbezirk": "Musterstadt",
            "grundbuchblatt": if rolle == "vorhaben" { "1234" } else { "" },
            "laufende_nummer": if rolle == "vorhaben" { json!(1) } else { Value::Null },
            "grundbuchauszug": null, "eigentuemer": eigentuemer, "eigentuemer_kunden": kunden,
            "bemerkung": "", "rolle": rolle})
    };
    // Eigentümer werden als Kunden zugeordnet (Freitext ist Altbestand).
    let nachbar1 =
        b.c.kunde_anlegen(
            person(
                "Nina",
                "Nachbar",
                anschrift("Lindenweg", "10", "59999", "Musterstadt"),
                "",
                "",
            ),
            &b.u,
        )
        .await
        .unwrap();
    let nachbar2 =
        b.c.kunde_anlegen(
            person(
                "Bernd",
                "Beispiel",
                anschrift("Lindenweg", "14", "59999", "Musterstadt"),
                "",
                "",
            ),
            &b.u,
        )
        .await
        .unwrap();
    let info: Grundstuecksinformationen = serde_json::from_value(json!({"flurstuecke": [
        fl("fs-412", "412", GRUNDSTUECK_M2, "vorhaben", "", vec![json!(b.bauherr.id)]),
        fl("fs-411", "411", 655., "nachbar", "", vec![json!(nachbar1.id)]),
        fl("fs-413", "413", 598., "nachbar", "", vec![json!(nachbar2.id)]),
    ]}))
    .unwrap();
    let mut r = b.p.planungsrecht.clone();
    r.grundstuecksinformationen = Some(info);
    b.bereich(ProjektBereich::Grundstuecksinformationen, r)
        .await;

    // Gebäude mit Geschossen (Satteldach 36°, Kniestock 1,00 m, Keller).
    let geschoss =
        |id: Uuid, name: &str, voll: bool, grund: &str, ff: f64, lichte: f64, decke: f64| {
            json!({"id": id, "bezeichnung": name, "vollgeschoss": voll, "grundlage": grund,
            "geometrie": {"flaeche_m2": 99, "fussboden_m": ff, "lichte_hoehe_m": lichte,
                "lichte_hoehe_einheitlich": name != "DG", "deckendicke_m": decke}})
        };
    let haus = json!({
        "bezeichnung": "Wohnhaus", "grundstueck_ids": [], "flurstueck_ids": ["fs-412"], "quelle": null,
        "geometrie": {"typ": "hauptanlage", "grundflaeche_m2": 99, "nullpunkt_m_nhn": NULLPUNKT_NHN,
            "gelaende_m": -0.3, "fundamentsohle_m": -3.1, "traufe_m": 4.2, "first_m": 7.45,
            "dachform": "satteldach", "dachneigung_grad": 36,
            "herkunft": "Entwurfsplanung Planungsbüro Planer, Stand 10/2026"},
        "geschosse": [
            geschoss(b.kg, "KG", false, "§ 2 Abs. 6 BauO NRW: Deckenoberkante höchstens 1,60 m über Gelände", -2.8, 2.4, 0.3),
            geschoss(b.eg, "EG", true, "§ 2 Abs. 6 BauO NRW: lichte Höhe 2,60 m über der gesamten Grundfläche", 0., 2.6, 0.25),
            geschoss(b.dg, "DG", false, "§ 2 Abs. 6 BauO NRW: lichte Höhe 2,30 m auf weniger als zwei Dritteln der Grundfläche (Kniestock 1,00 m, 36°)", 2.85, 2.3, 0.2),
        ]
    });
    b.speichern(ObjektArt::Gebaeude, b.haus, haus).await;
    let garage = json!({
        "bezeichnung": "Garage", "grundstueck_ids": [], "flurstueck_ids": ["fs-412"], "quelle": null,
        "geometrie": {"typ": "garage", "grundflaeche_m2": 21, "dachform": "flachdach", "einzelgeschoss_vollgeschoss": false,
            "einzelgeschoss_grundlage": "Garage ohne Aufenthaltsräume, lichte Höhe 2,50 m; kein Vollgeschoss im Sinne von § 20 BauNVO"},
        "geschosse": []
    });
    b.speichern(ObjektArt::Gebaeude, b.garage, garage).await;

    // Baugrundstück ermitteln und GRZ/GFZ-Eingaben speichern (wie „Eingaben speichern“).
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let gruppen = b.c.baugrundstuecke_ermitteln(b.p.id, &plan).await.unwrap();
    assert_eq!(gruppen.len(), 1, "genau ein Baugrundstück erwartet");
    let eingabe = json!({
        "gebiet": "WA", "vollgeschosse": null,
        "mass": {"grundstuecksflaeche_m2": GRUNDSTUECK_M2, "grundflaeche_m2": 0, "grundflaeche_nebenanlagen_m2": 0,
            "geschossflaeche_m2": 0, "baumasse_m3": 282.2},
        "aufstellung": {"bau_nvo": "2017", "nichtvollgeschoss_faktor": 0, "nichtvollgeschoss_festsetzung": "",
            "bauteile": [
                {"gebaeude_id": null, "bezeichnung": "Zufahrt Garage (Pflaster)", "typ": "zufahrt", "flaeche_m2": 30, "abweichende_anrechnung": null, "begruendung": ""},
                {"gebaeude_id": null, "bezeichnung": "Terrasse Süd", "typ": "terrasse_hauptanlage", "flaeche_m2": 20, "abweichende_anrechnung": null, "begruendung": ""},
                {"gebaeude_id": null, "bezeichnung": "Stellplatz vor der Garage", "typ": "stellplatz", "flaeche_m2": 12.5, "abweichende_anrechnung": null, "begruendung": ""}
            ],
            "geschosse": []}
    });
    let ermittlung: BaugrundstuecksErmittlung = serde_json::from_value(json!({
        "ansaetze": [{"flurstueck_id": "fs-412", "abzuege": [], "abweichende_zuordnung": null}],
        "berechnungen": [{"schluessel": gruppen[0].schluessel, "eingabe": eingabe, "eigene_festsetzungen": null}],
        "quellen_hash": null
    }))
    .unwrap();
    let mut r = b.p.planungsrecht.clone();
    r.baugrundstuecksermittlung = Some(ermittlung);
    b.bereich(ProjektBereich::Grundstuecke, r).await;
    // Gebäude in die Aufstellung einbinden (wie beim Öffnen von GRZ/GFZ) und erneut speichern.
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let e =
        b.c.projekt_baugrundstuecke_gebaeude_einbinden(b.p.id, &plan)
            .await
            .unwrap();
    for h in &e.hinweise {
        b.info(format!("Gebäudeeinbindung: {}", h.text));
    }
    let mut r = b.p.planungsrecht.clone();
    let erm = r.baugrundstuecksermittlung.as_mut().unwrap();
    erm.berechnungen = e.plan.berechnungen.clone();
    b.bereich(ProjektBereich::Grundstuecke, r).await;
    b.grund = Some(
        core_lib::grundstueckskontext::ermitteln(&b.p)
            .unwrap()
            .remove(0),
    );
    b
}
