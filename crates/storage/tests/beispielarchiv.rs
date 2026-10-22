//! Das mitgelieferte Beispielprojekt (`beispiele/Beispielprojekt-EFH-Mustermann.zip`)
//! bleibt mit dem aktuellen Stand der Software lesbar und rechnet dieselben
//! Ergebnisse. Die App stellt genau dieses Archiv über „Hilfe → Beispielprojekt
//! öffnen“ in ein eigenes Profil wieder her.
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
    // Wer das Beispiel öffnet, verwaltet die Kopie (wie in der App).
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

/// Die Kopie des Beispielprojekts übernimmt Lizenz und Testbeginn des
/// bisherigen Profils, damit sie weder eine neue Testversion startet noch
/// ohne Lizenz schreibgeschützt ist.
#[tokio::test]
async fn beispielkopie_uebernimmt_lizenz_und_testbeginn() {
    let dir = tempfile::tempdir().unwrap();
    let von = db_sqlite::SqliteDatabase::open(&dir.path().join("von.db"))
        .await
        .unwrap();
    let nach = db_sqlite::SqliteDatabase::open(&dir.path().join("nach.db"))
        .await
        .unwrap();
    let (von, nach) = (von.repositories(), nach.repositories());
    von.lizenz
        .einstellung_setzen("testbeginn", "2026-01-05")
        .await
        .unwrap();
    von.lizenz
        .einstellung_setzen("lizenz", "Lizenzdatei")
        .await
        .unwrap();
    core_lib::license::lizenz_uebernehmen(von.lizenz.as_ref(), nach.lizenz.as_ref())
        .await
        .unwrap();
    assert_eq!(
        nach.lizenz
            .einstellung("testbeginn")
            .await
            .unwrap()
            .as_deref(),
        Some("2026-01-05")
    );
    assert_eq!(
        nach.lizenz.einstellung("lizenz").await.unwrap().as_deref(),
        Some("Lizenzdatei")
    );
    assert_eq!(
        nach.lizenz.einstellung("abo_schluessel").await.unwrap(),
        None
    );
}
