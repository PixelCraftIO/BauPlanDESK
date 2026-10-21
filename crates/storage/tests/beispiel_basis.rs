//! Entwicklungstest für das Basismodul des Beispielprojekts.
#[path = "beispiel/basis.rs"]
mod basis;

#[tokio::test]
async fn beispiel_basis() {
    let dir = tempfile::tempdir().unwrap();
    let b = basis::basis(dir.path().join("profil")).await;
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let r =
        b.c.projekt_baugrundstuecke_pruefen(b.p.id, &plan)
            .await
            .unwrap();
    let a = r.baugrundstuecke[0].pruefung.aufstellung.as_ref().unwrap();
    // Wohnhaus 99 m² + Terrasse 20 m² (Teil der Hauptanlage).
    assert_eq!(a.mass.grundflaeche_m2, 119.);
    assert_eq!(a.mass.grundflaeche_nebenanlagen_m2, 63.5);
    assert_eq!(a.mass.geschossflaeche_m2, 99.);
    assert_eq!(a.vollgeschosse, 1);
    assert!(
        r.baugrundstuecke[0]
            .pruefung
            .nachweise
            .iter()
            .all(|n| n.eingehalten != Some(false))
    );
    assert!(b.befunde.is_empty(), "{:?}", b.befunde);
}
