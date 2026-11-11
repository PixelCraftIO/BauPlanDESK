//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW) mit allen
//! Arbeitsbereichen. Prüft, dass Eintragungen und Berechnungen ohne Befunde
//! durchlaufen, und erzeugt auf Wunsch ein ladbares Sicherungsarchiv:
//!
//! ```sh
//! BAUPROJEKT_BEISPIEL_ZIEL=beispiele cargo test -p storage --test beispielprojekt
//! ```
//!
//! Das Archiv ist die Grundlage der Handbuchbilder (`scripts/handbuch-bilder`);
//! `tests/beispielarchiv.rs` prüft das abgelegte Archiv.
mod beispiel;
use beispiel::*;

/// Der Gesamtablauf ist ein sehr großes Future; im Debug-Build reicht der
/// Standard-Stack des Testthreads nicht.
#[test]
fn beispielprojekt_einfamilienhaus_mustermann() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(gesamtablauf())
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn gesamtablauf() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = basis::basis(dir.path().join("profil")).await;
    // Lageplan zuerst: Die Flurstücke sind gezeichnet, bevor Nachweise entstehen.
    Box::pin(abstand::lageplan(&mut b)).await;
    Box::pin(fach::fach(&mut b)).await;
    Box::pin(abstand::abstand(&mut b)).await;
    Box::pin(verfahren::verfahren(&mut b)).await;
    // Am Stichtag ist im Verfahren nur die Nachforderung der Bauaufsicht offen.
    abschluss::uebersicht(
        &mut b,
        &[
            "Nachforderung",
            "Entwässerungsnachweis",
            "Entwässerungsplanung",
        ],
    )
    .await;
    // Die Planliste führt alle eingereichten Pläne.
    let planliste = b.c.planliste(b.p.id, &b.u).await.unwrap();
    assert_eq!(planliste.zeilen.len(), 4);
    assert!(
        planliste.ohne_eintrag.is_empty(),
        "{:?}",
        planliste.ohne_eintrag
    );
    // Stand des Projekts aus der Verfahrensakte.
    let stand = b.c.projektstand(b.p.id).await.unwrap();
    assert_eq!(
        stand.stufe,
        core_lib::projektstand::Projektstufe::Eingereicht
    );
    assert_eq!(stand.seit, "2026-09-15".parse().ok());
    assert_eq!(b.c.projekt_formulare(b.p.id).await.unwrap().len(), 2);
    // Formulare erhalten die Ergebnisse der Fachberechnungen.
    let quellen =
        b.c.formular_quellen(b.p.id, Default::default())
            .await
            .unwrap();
    let wert = |s: &str| quellen.datenstand.werte.wert(s).map(str::to_string);
    assert_eq!(wert("wohnflaeche.gesamt_m2").as_deref(), Some("154,47"));
    assert_eq!(wert("din277.bgf_m2").as_deref(), Some("294,87"));
    assert_eq!(wert("din277.bri_m3").as_deref(), Some("883,57"));
    assert!(wert("stellplaetze.kfz_bedarf").is_some());
    assert!(wert("freiflaechen.versiegelt_m2").is_some());
    assert!(
        quellen
            .datenstand
            .werte
            .datenstand()
            .keys()
            .any(|k| k.starts_with("hoehen.hoehe_")),
        "Höhen aus der Höhenberechnung"
    );
    let katalog = b.c.druckkatalog(b.p.id).await.unwrap();
    assert_eq!(
        katalog.aktenzeichen, "63-BA-0815/2026",
        "Aktenzeichen aus der Verfahrensakte"
    );
    let pdf = abschluss::bauvorlagenpaket(&mut b).await;
    let archiv = dir.path().join("Beispielprojekt-EFH-Mustermann.zip");
    abschluss::sicherung(&b, &archiv).await;
    abschluss::gegenprobe(&archiv, &b.p.name).await;
    if let Ok(ziel) = std::env::var("BAUPROJEKT_BEISPIEL_ZIEL") {
        let ziel = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(ziel);
        std::fs::create_dir_all(&ziel).unwrap();
        std::fs::copy(&archiv, ziel.join("Beispielprojekt-EFH-Mustermann.zip")).unwrap();
        std::fs::write(ziel.join("Bauvorlagen-EFH-Mustermann.pdf"), &pdf).unwrap();
    }
    eprintln!(
        "AUSBAUVORSCHLÄGE ({}):\n- {}",
        b.ausbau.len(),
        b.ausbau.join("\n- ")
    );
    assert!(
        b.befunde.is_empty(),
        "Befunde:\n- {}",
        b.befunde.join("\n- ")
    );
}
