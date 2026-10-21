//! Entwicklungstest für das Abstandsflächenmodul des Beispielprojekts.
#[path = "beispiel/abstand.rs"]
mod abstand;
#[path = "beispiel/basis.rs"]
mod basis;

use core_lib::{
    abstandsflaechen::{AbstandsErgebnis, AbstandsPlan, AbstandsStatus},
    plattform::{ObjektArt, nachweise::NachweisEingabe},
};

#[tokio::test]
async fn beispiel_abstand() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = basis::basis(dir.path().join("profil")).await;
    abstand::abstand(&mut b).await;

    let f =
        b.c.fachobjekte(b.p.id, ObjektArt::Abstandsflaechen)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == abstand::PLAN_ID.to_string())
            .expect("Abstandsplan gespeichert");
    let plan: AbstandsPlan = serde_json::from_value(f.daten).unwrap();
    let n =
        b.c.nachweise(b.p.id)
            .await
            .unwrap()
            .into_iter()
            .find(|n| matches!(n.stand.eingabe, NachweisEingabe::AbstandProjekt { .. }))
            .expect("Abstandsnachweis archiviert");
    assert!(
        n.aktuell,
        "Nachweis nach Wiederherstellung des Gebäudes aktuell"
    );
    let r: AbstandsErgebnis = serde_json::from_value(n.stand.ergebnis.clone()).unwrap();
    // Wohnhaus: Traufwände H = 4,50 m, Giebelwände H = 4,50 + 3,25/3 = 5,583 m;
    // 0,4 H < 3,00 m → überall 3,00 m. Garage: 0 m (Grenzgarage).
    for (seite, h) in [
        (0, 4.5),
        (1, 4.5 + 3.25 / 3.),
        (2, 4.5),
        (3, 4.5 + 3.25 / 3.),
    ] {
        let id = abstand::wand_id(&plan, abstand::UMRISS_HAUS, seite);
        let w = r.waende.iter().find(|w| w.id == id).unwrap();
        assert!(
            (w.h_m.unwrap() - h).abs() < 1e-6,
            "{} H {:?}",
            w.bezeichnung,
            w.h_m
        );
        assert!((w.tiefe_m.unwrap() - 3.).abs() < 1e-9, "{}", w.bezeichnung);
        assert!((w.flaeche_m2.unwrap() - 3. * w.laenge_m).abs() < 1e-6);
        assert_eq!(w.ausserhalb_m2, Some(0.));
        // Die Westwand reicht genau bis an die Grenze: eingehalten.
        assert_eq!(w.status, AbstandsStatus::Eingehalten, "{}", w.bezeichnung);
    }
    for seite in 0..4 {
        let id = abstand::wand_id(&plan, abstand::UMRISS_GARAGE, seite);
        let w = r.waende.iter().find(|w| w.id == id).unwrap();
        assert_eq!(w.tiefe_m, Some(0.));
        assert_eq!(w.status, AbstandsStatus::Eingehalten);
    }
    assert!(r.ueberdeckungen.is_empty());
    assert!(r.konfliktfrei && n.stand.vollstaendig);
    assert!(n.freigabe.is_some(), "Abstandsnachweis freigegeben");
    eprintln!("BEFUNDE ({}):", b.befunde.len());
    for x in &b.befunde {
        eprintln!(" - {x}");
    }
}
