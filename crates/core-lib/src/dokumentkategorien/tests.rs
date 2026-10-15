use super::*;
#[test]
fn katalog_erhaelt_identitaeten_und_verhindert_mehrdeutige_hierarchien() {
    let alt = KategorienKatalog::default();
    assert!(alt.validieren(&alt).is_ok());
    for id in ["grz", "gfz", "baugrundstueck", "abstandsflaechen"] {
        assert_eq!(
            alt.snapshot(id).unwrap().hauptkategorie.as_deref(),
            Some("Berechnung")
        );
    }
    let mut neu = alt.clone();
    neu.kategorien.retain(|k| k.id != "lageplan");
    assert!(neu.validieren(&alt).is_err());
    let mut neu = alt.clone();
    neu.kategorien.push(neu.kategorien[0].clone());
    assert!(neu.validieren(&alt).is_err());
    let mut neu = alt.clone();
    neu.kategorien[0].eltern_id = Some("grz".into());
    assert!(neu.validieren(&alt).is_err());
    let mut neu = alt.clone();
    neu.kategorien[0].eltern_id = Some("bauantrag".into());
    assert!(neu.validieren(&alt).is_err());
    let mut neu = alt.clone();
    neu.kategorien
        .iter_mut()
        .find(|k| k.id == "berechnung")
        .unwrap()
        .aktiv = false;
    assert!(neu.validieren(&alt).is_err());
    let mut neu = alt.clone();
    neu.kategorien
        .iter_mut()
        .find(|k| k.id == "lageplan")
        .unwrap()
        .aktiv = false;
    assert!(neu.validieren(&alt).is_ok());
    assert!(neu.zuweisbar(Some("lageplan")).is_err());
    assert!(neu.snapshot("lageplan").is_ok());
}
