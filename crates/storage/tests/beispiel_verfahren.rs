//! Entwicklungstest für das Verfahrensmodul des Beispielprojekts.
#[path = "beispiel/basis.rs"]
mod basis;
#[path = "beispiel/verfahren.rs"]
mod verfahren;

use core_lib::formular::pdf::Feldwert;

#[tokio::test]
async fn beispiel_verfahren() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = basis::basis(dir.path().join("profil")).await;
    let e = verfahren::verfahren(&mut b).await;

    // XBau: schemagültig (bereits beim Festhalten und erneut im Modul geprüft).
    assert!(e.xbau_xml.contains("baugenehmigung.antrag.0200"));
    assert!(e.xbau_xml.contains("<zaehler>412</zaehler>"));
    assert!(e.xbau_xml.contains(verfahren::BEHOERDE_KENNUNG));

    // Formularwerte aus Projektdaten.
    let t = |k: &str| e.formularwerte.get(k).cloned();
    assert_eq!(
        t("grundstueck.flaeche_m2"),
        Some(Feldwert::Text("620,00".into()))
    );
    assert_eq!(t("kennzahlen.grz"), Some(Feldwert::Text("0,19".into())));
    assert_eq!(t("kennzahlen.gfz"), Some(Feldwert::Text("0,16".into())));
    assert_eq!(
        t("bauherr.name"),
        Some(Feldwert::Text("Max Mustermann".into()))
    );
    assert_eq!(
        e.beispielformular.get("Grundstueck.Flaeche"),
        Some(&Feldwert::Text("620,00".into()))
    );

    // Pakete: Verfahrensabgabe und Unterlagenpaket.
    let (_, a) = b.c.verfahrensakte(b.p.id, e.verfahren).await.unwrap();
    let stand = a.abgaben.iter().find(|x| x.id == e.abgabe).unwrap();
    assert_eq!(stand.dokumente.len(), 11);
    assert!(
        ["zip", "xbau", "pdf"]
            .iter()
            .all(|f| stand.artefakte.iter().any(|a| a.format == *f))
    );
    assert!(
        e.paket_zip.len() > 1000,
        "Unterlagenpaket (Abgabestand-ZIP) erstellt"
    );
    assert!(
        e.portalbereitstellung.is_some(),
        "Portalbereitstellung erstellt"
    );
    assert_eq!(a.aktenzeichen, verfahren::AKTENZEICHEN);
    assert_eq!(a.status, core_lib::verfahren::Verfahrensstatus::InPruefung);
    assert_eq!(
        core_lib::verfahren::termine::offene_forderungspunkte(&a).count(),
        1
    );

    eprintln!("\n=== {} Befunde ===", b.befunde.len());
    for x in &b.befunde {
        eprintln!("- {x}");
    }
}
