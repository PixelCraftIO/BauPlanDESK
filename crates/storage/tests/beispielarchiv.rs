//! Das abgelegte Beispielprojekt (`beispiele/Beispielprojekt-EFH-Mustermann.zip`)
//! bleibt mit dem aktuellen Stand der Software lesbar und rechnet dieselben
//! Ergebnisse. Die Bildautomatik des Benutzerhandbuchs
//! (`scripts/handbuch-bilder`) nimmt alle Bilder aus genau diesem Archiv auf.
//!
//! Schlägt der Test nach einer Regel- oder Berechnungsänderung fehl, das Archiv
//! mit dem Generator neu erzeugen (siehe `beispiele/README.md`) und die
//! geänderten Zahlen dort nachtragen.
#[path = "beispiel/basis.rs"]
mod basis;

use core_lib::{benutzer::Rolle, formular::instanz::FormularBezug};

const ARCHIV: &str = "../../beispiele/Beispielprojekt-EFH-Mustermann.zip";
const PROJEKT: &str = "Neubau Einfamilienhaus Mustermann";

#[tokio::test]
async fn mitgeliefertes_beispielprojekt_rechnet_mit_aktuellem_stand() {
    let archiv = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIV);
    storage::sicherung::pruefen(&archiv).expect("Archiv vollständig und unverändert");
    let dir = tempfile::tempdir().unwrap();
    let profil = dir.path().join("Beispielprojekt");
    storage::sicherung::wiederherstellen(&archiv, &profil).unwrap();
    db_sqlite::SqliteDatabase::datei_pruefen(&profil.join("bauprojekt.db"))
        .await
        .unwrap();

    let (c, db) = basis::profil_oeffnen(&profil).await;
    // Wer das Archiv wiederherstellt, verwaltet das Profil (wie in der App).
    let ich = core_lib::anmeldung::verwaltung_nach_wiederherstellung(
        &db.repositories(),
        "anwender",
        "Neue Anwenderin",
    )
    .await
    .unwrap();
    assert_eq!(ich.rolle, Rolle::Administrator);

    let projekte = c.list_projects().await.unwrap();
    assert_eq!(projekte.len(), 1, "genau ein Beispielprojekt");
    let p = &projekte[0];
    assert_eq!(p.name, PROJEKT);

    // Nachweise sind nur aktuell, wenn die heutige Berechnung aus denselben
    // Eingaben und Regeln zum gespeicherten Ergebnis kommt.
    let nachweise = c.nachweise(p.id).await.unwrap();
    assert!(!nachweise.is_empty());
    let veraltet: Vec<_> = nachweise
        .iter()
        .filter(|n| !n.aktuell)
        .map(|n| n.stand.bezeichnung.clone())
        .collect();
    assert!(
        veraltet.is_empty(),
        "Nicht aktuelle Nachweise: {veraltet:?}"
    );

    // Kennzahlen, wie sie Formulare und Bauantrag aus dem Projekt beziehen;
    // dieselben Zahlen nennt beispiele/README.md.
    let quellen = c
        .formular_quellen(p.id, FormularBezug::default())
        .await
        .unwrap();
    let werte = &quellen.datenstand.werte;
    for (feld, erwartet) in [
        ("grundstueck.flaeche_m2", "620,00"),
        ("grundstueck.bezugsflaeche_m2", "620,00"),
        ("grundstueck.flurstuecke", "412"),
        ("planung.baugebiet", "WA"),
        ("planung.bebauungsplan_nummer", "42"),
        ("kennzahlen.grz", "0,19"),
        ("kennzahlen.grz_gesamt", "0,29"),
        ("kennzahlen.gfz", "0,16"),
        ("flaechen.grundflaeche_m2", "119,00"),
        ("flaechen.geschossflaeche_m2", "99,00"),
        ("gebaeude.vollgeschosse", "1"),
        ("bauherr.name", "Max Mustermann"),
        ("entwurfsverfasser.name", "Erika Planer"),
    ] {
        assert_eq!(werte.wert(feld), Some(erwartet), "Projektwert {feld}");
    }

    // Jede abgelegte Dokumentversion ist vorhanden und unverändert.
    for d in c.list_documents(p.id).await.unwrap() {
        c.load_version(d.document_id, d.version).await.unwrap();
    }
    db.schliessen().await;
}

/// Bereichsstatus der Projektnavigation im Beispielprojekt: Am Stichtag ist die
/// Aufgabe zum nachgeforderten Entwässerungsnachweis überfällig (Problem);
/// Unterlagenregister, Verfahrensakte, Aufmaß (Aufgaben ohne Messwert) und
/// Gebäude (Garage ohne Geschosse) sind in Arbeit. Grundlagen und freigegebene,
/// eingehaltene Nachweise sind geprüft.
#[tokio::test]
async fn beispielprojekt_bereichsstatus() {
    use core_lib::bereichsstatus::Bereichszustand::*;
    let archiv = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIV);
    let dir = tempfile::tempdir().unwrap();
    let profil = dir.path().join("Beispielprojekt");
    storage::sicherung::wiederherstellen(&archiv, &profil).unwrap();
    let (c, db) = basis::profil_oeffnen(&profil).await;
    let ich = core_lib::anmeldung::verwaltung_nach_wiederherstellung(
        &db.repositories(),
        "anwender",
        "Neue Anwenderin",
    )
    .await
    .unwrap();
    let p = &c.list_projects().await.unwrap()[0];
    let tag = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    let s = c.bereichsstatus_am(p.id, tag, &ich).await.unwrap();
    for b in &s.bereiche {
        println!("{:<18} {:?} {:?}", b.bereich, b.zustand, b.gruende);
    }
    let zustand = |id: &str| {
        s.bereiche
            .iter()
            .find(|b| b.bereich == id)
            .unwrap_or_else(|| panic!("Bereich {id} fehlt"))
            .zustand
    };
    for id in [
        "uebersicht",
        "rechtsgrundlagen",
        "festsetzungen",
        "flurstuecke",
        "raumbuch",
        "grundstueck",
        "grz",
        "wohnflaechen",
        "abstand",
        "stellplaetze",
        "entwaesserung",
        "nachweise",
    ] {
        assert_eq!(zustand(id), Geprueft, "{id}");
    }
    for id in ["unterlagen", "planaufmass", "gebaeude"] {
        assert_eq!(zustand(id), InArbeit, "{id}");
    }
    // Die offenen Aufgaben gehören zum nachgeforderten Punkt: Der Vorgang
    // steht einmal beim Verfahren und ist so dringend wie seine überfällige
    // Aufgabe; der Aufgabenbereich meldet ihn nicht ein zweites Mal.
    assert_eq!(zustand("verfahren"), Problem);
    assert_eq!(zustand("aufgaben"), Leer);
    // Listenbereiche werden nie grün.
    for id in [
        "aufgaben",
        "projekttagebuch",
        "fachplaner",
        "dokumente",
        "portale",
    ] {
        assert_ne!(zustand(id), Geprueft, "{id}");
    }
    assert!(
        s.bereiche
            .iter()
            .filter(|b| matches!(b.zustand, Problem | Veraltet | InArbeit | ZuErledigen))
            .all(|b| !b.gruende.is_empty())
    );
    db.schliessen().await;
}
