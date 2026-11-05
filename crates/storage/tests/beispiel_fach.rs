//! Entwicklungstest für das Fachmodul des Beispielprojekts (PDF-Aufmaß,
//! Raumbuch, Fachberechnungen, Nachweise, Druckausgabe).
#[path = "beispiel/basis.rs"]
mod basis;
#[path = "beispiel/fach.rs"]
mod fach;

use core_lib::{berechnungen::BerechnungsArt, plattform::nachweise::NachweisEingabe};

fn kennzahl(n: &core_lib::plattform::nachweise::NachweisAnsicht, id: &str) -> Option<f64> {
    n.stand.ergebnis["kennzahlen"]
        .as_array()?
        .iter()
        .find(|k| k["id"] == id)?["wert"]
        .as_f64()
}

/// Der Ablauf des Beispielprojekts ist ein sehr großer async-Zustand; auf dem
/// Standardstapel des Testthreads (2 MiB) läuft er über. Daher ein eigener
/// Thread mit größerem Stapel.
#[test]
fn beispiel_fach() {
    let ergebnis = std::thread::Builder::new()
        .name("beispiel_fach".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(ablauf())
        })
        .unwrap()
        .join();
    if let Err(e) = ergebnis {
        std::panic::resume_unwind(e);
    }
}

async fn ablauf() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = basis::basis(dir.path().join("profil")).await;
    fach::fach(&mut b).await;

    let nachweise = b.c.nachweise(b.p.id).await.unwrap();
    // 8 Fachberechnungen + GRZ, GFZ, Geschossigkeit (+ BMZ)
    assert!(nachweise.len() >= 11, "{}", nachweise.len());
    for n in &nachweise {
        assert!(n.aktuell, "{} veraltet", n.stand.bezeichnung);
        assert!(
            n.freigabe.is_some(),
            "{} nicht freigegeben",
            n.stand.bezeichnung
        );
    }
    let fach = |art: BerechnungsArt| {
        nachweise
            .iter()
            .find(|n| matches!(&n.stand.eingabe, NachweisEingabe::Fach { plan, .. } if plan.modul == art))
            .unwrap_or_else(|| panic!("Nachweis {art:?} fehlt"))
    };
    for art in BerechnungsArt::ALLE {
        assert!(
            fach(art).stand.vollstaendig,
            "{art:?} unvollständig: {}",
            fach(art).stand.ergebnis["offen"]
        );
    }
    let wf = kennzahl(fach(BerechnungsArt::Wohnflaechen), "wohnflaeche").unwrap();
    assert!((130. ..=160.).contains(&wf), "Wohnfläche {wf}");
    assert_eq!(
        kennzahl(fach(BerechnungsArt::Wohnflaechen), "wohneinheiten"),
        Some(1.)
    );
    assert_eq!(
        kennzahl(fach(BerechnungsArt::Stellplaetze), "kfz_bedarf"),
        Some(2.)
    );
    assert_eq!(
        kennzahl(fach(BerechnungsArt::Vollgeschosse), "vollgeschosse"),
        Some(1.)
    );
    let h = fach(BerechnungsArt::Hoehen);
    assert!((kennzahl(h, "hoehe:traufe").unwrap() - 4.44).abs() < 1e-3);
    assert!((kennzahl(h, "hoehe:first").unwrap() - 7.69).abs() < 1e-3);
    let d = fach(BerechnungsArt::Din277);
    assert!(kennzahl(d, "bgf").unwrap() > 290.);
    assert!(kennzahl(d, "bri").unwrap() > 880.);

    // Gebäude bleibt konsistent mit der Basis (99 m², GRZ/GFZ unverändert).
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let r =
        b.c.projekt_baugrundstuecke_pruefen(b.p.id, &plan)
            .await
            .unwrap();
    let a = r.baugrundstuecke[0].pruefung.aufstellung.as_ref().unwrap();
    assert!((a.mass.grundflaeche_m2 - 119.).abs() < 1e-3);
    assert!((a.mass.geschossflaeche_m2 - 99.).abs() < 1e-3);
    assert_eq!(a.vollgeschosse, 1);

    eprintln!("\n=== {} Befunde ===", b.befunde.len());
    for f in &b.befunde {
        eprintln!("- {f}");
    }
}
