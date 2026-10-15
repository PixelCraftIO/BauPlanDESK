use super::*;
#[test]
fn email_ergaenzt_gespeicherte_kataloge_ohne_anpassungen_zu_verlieren() {
    let mut alt = KategorienKatalog {
        revision: 7,
        ..Default::default()
    };
    alt.kategorien.retain(|k| k.id != "email");
    let plan = alt
        .kategorien
        .iter_mut()
        .find(|k| k.id == "lageplan")
        .unwrap();
    plan.name = "Amtlicher Lageplan".into();
    plan.aktiv = false;
    let neu = alt.standard_ergaenzen();
    assert_eq!(neu.revision, 7);
    assert!(neu.kategorie("email").unwrap().aktiv);
    assert_eq!(neu.kategorie("email").unwrap().name, "E-Mail");
    assert_eq!(
        neu.kategorie("lageplan").unwrap().name,
        "Amtlicher Lageplan"
    );
    assert!(!neu.kategorie("lageplan").unwrap().aktiv);
    let mut archiviert = neu.clone();
    archiviert
        .kategorien
        .iter_mut()
        .find(|k| k.id == "email")
        .unwrap()
        .aktiv = false;
    assert!(
        !archiviert
            .standard_ergaenzen()
            .kategorie("email")
            .unwrap()
            .aktiv
    );
    assert!(neu.validieren(&neu).is_ok());
}
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
