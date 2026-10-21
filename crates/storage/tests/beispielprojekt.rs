//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW) mit allen
//! Arbeitsbereichen. Prüft, dass Eintragungen und Berechnungen ohne Befunde
//! durchlaufen, und erzeugt auf Wunsch ein ladbares Sicherungsarchiv:
//!
//! ```sh
//! BAUPROJEKT_BEISPIEL_ZIEL=beispiele cargo test -p storage --test beispielprojekt
//! ```
//!
//! Laden: Einstellungen → Sicherung und Wiederherstellung → „Wiederherstellen“,
//! dann „Profil aktivieren“ und die App neu starten.
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
    Box::pin(fach::fach(&mut b)).await;
    Box::pin(abstand::abstand(&mut b)).await;
    Box::pin(verfahren::verfahren(&mut b)).await;
    // Am Stichtag ist im Verfahren nur die Nachforderung der Bauaufsicht offen.
    abschluss::uebersicht(
        &mut b,
        &[
            "Nachforderung",
            "Entwässerungsnachweis",
            "Erschließung und Entwässerung",
        ],
    )
    .await;
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
