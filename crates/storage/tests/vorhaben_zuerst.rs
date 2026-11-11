//! Vorhaben zuerst: Mit Verfahren und Festsetzungen weiß die Akte von Anfang
//! an, welche Unterlagen und Nachweise das Vorhaben braucht. Projektübersicht,
//! Projektbaum und Einreichungsprüfung melden dasselbe.
use chrono::NaiveDate;
use core_lib::{
    CoreServices,
    bereichsstatus::Bereichszustand,
    domain::{BebauungsplanArt, Bundesland, NewProject, Planungsrecht, Rechtsgrundlage},
    einreichungspruefung::{Ampel, Pruefbereich},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    verfahren::{Erforderlichkeit, Verfahrensakte, Verfahrensart, Verfahrensstatus},
};
use std::sync::Arc;

async fn core() -> (tempfile::TempDir, CoreServices) {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    (dir, c)
}

fn mit_bebauungsplan() -> Planungsrecht {
    let mut r = Planungsrecht {
        rechtsgrundlage: Some(Rechtsgrundlage::Par30),
        bebauungsplan_art: BebauungsplanArt::Qualifiziert,
        bau_nvo: Some(core_lib::flaechen::BauNvo::V2017),
        plan_bezeichnung: "Am Lindenweg".into(),
        nutzung: "Wohnen".into(),
        ..Default::default()
    };
    r.festgesetzte_werte.grz = Some(0.4);
    r.festgesetzte_werte.gfz = Some(0.8);
    r.festgesetzte_werte.vollgeschosse = Some(2);
    r.festgesetzte_werte.firsthoehe_max_m = Some(10.0);
    r
}

#[tokio::test]
async fn verfahren_beim_start_macht_erforderliches_sichtbar() {
    let (_dir, c) = core().await;
    let ich = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Neubau Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: mit_bebauungsplan(),
            },
            &ich,
        )
        .await
        .unwrap();
    let tag = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();

    // Ohne Verfahren steht die Frage nach Vorhaben und Verfahren vorn, und die
    // Festsetzungen verlangen bereits ihre Nachweise.
    let vorher = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    let ids = |u: &core_lib::projektuebersicht::Projektuebersicht| {
        u.punkte.iter().map(|p| p.id.clone()).collect::<Vec<_>>()
    };
    let v = ids(&vorher);
    assert!(v.contains(&"anforderungen".to_string()), "{v:?}");
    for b in ["grz", "gfz", "geschosse", "hoehen"] {
        assert!(v.contains(&format!("erforderlich:{b}")), "{b}: {v:?}");
    }
    assert!(
        !v.contains(&"erforderlich:stellplaetze".to_string()),
        "Stellplätze verlangt erst das Verfahren: {v:?}"
    );
    let pos = |id: &str| v.iter().position(|x| x == id).unwrap();
    assert!(
        pos("anforderungen") < pos("erforderlich:grz"),
        "Verfahren festlegen kommt vor den Nachweisen"
    );

    // Verfahrensakte wie im Dialog „Neues Projekt“: Bauantrag, Wohngebäude.
    let stand = c
        .verfahren_vorbereiten(p.id, Verfahrensart::Bauantrag, Some(true), &ich)
        .await
        .unwrap();
    let akte: Verfahrensakte = serde_json::from_value(stand.daten).unwrap();
    assert_eq!(akte.status, Verfahrensstatus::Vorbereitung);
    assert!(
        akte.behoerde.name.is_empty(),
        "Behörde darf noch offen sein"
    );
    let profil = akte.profil.as_ref().expect("Profil angewendet");
    assert_eq!(profil.katalog.id, "nrw-bauvorlagen");
    assert_eq!(
        profil.vorhaben.rechtsgrundlage,
        Some(Rechtsgrundlage::Par30)
    );
    let anforderung = |teil: &str| {
        akte.anforderungen
            .iter()
            .find(|a| a.titel.contains(teil))
            .unwrap_or_else(|| panic!("Anforderung „{teil}“"))
    };
    // Nach dem Vorhaben nicht einschlägig: als Vorschlag abgewählt.
    for teil in ["Betriebsbeschreibung", "Liegenschaftskarte"] {
        let a = anforderung(teil);
        assert_eq!(
            a.erforderlich,
            Erforderlichkeit::NichtErforderlich,
            "{teil}"
        );
        assert!(a.grund.contains("nicht einschlägig"), "{}", a.grund);
    }
    // Für die Verfahrensart vorgesehen: als Vorschlag erforderlich.
    for teil in ["Lageplan", "Bauzeichnungen", "Stellplätze"] {
        assert_eq!(
            anforderung(teil).erforderlich,
            Erforderlichkeit::Erforderlich,
            "{teil}"
        );
    }
    // Nur im Einzelfall oder mit fehlenden Angaben: bleibt zu klären.
    for teil in [
        "Abweichung",
        "Brandschutzkonzept",
        "Bescheinigung über die Prüfung",
    ] {
        assert_eq!(
            anforderung(teil).erforderlich,
            Erforderlichkeit::Ungeklaert,
            "{teil}"
        );
    }
    // Kein Vorschlag gilt als freigegeben.
    assert!(akte.anforderungen.iter().all(|a| !a.freigegeben));
    assert_eq!(anforderung("Stellplätze").nachweise, ["stellplaetze"]);

    let nachher = c.projektuebersicht_am(p.id, tag, &ich).await.unwrap();
    let n = ids(&nachher);
    assert!(!n.contains(&"anforderungen".to_string()), "{n:?}");
    for b in [
        "grz",
        "gfz",
        "geschosse",
        "hoehen",
        "stellplaetze",
        "abstand",
        "entwaesserung",
    ] {
        assert!(n.contains(&format!("erforderlich:{b}")), "{b}: {n:?}");
    }
    let stellplatz = nachher
        .punkte
        .iter()
        .find(|p| p.id == "erforderlich:stellplaetze")
        .unwrap();
    assert_eq!(stellplatz.ziel.bereich, "stellplaetze");
    assert!(
        stellplatz.hinweis.contains("Stellplätze"),
        "{}",
        stellplatz.hinweis
    );

    // Der Baum zeigt die verlangten Bereiche als zu erledigen statt leer.
    let baum = c.bereichsstatus_am(p.id, tag, &ich).await.unwrap();
    let zustand = |id: &str| {
        baum.bereiche
            .iter()
            .find(|b| b.bereich == id)
            .unwrap_or_else(|| panic!("Bereich {id}"))
    };
    for b in [
        "gfz",
        "hoehen",
        "stellplaetze",
        "abstand",
        "bauantrag",
        "formulare",
    ] {
        assert_eq!(zustand(b).zustand, Bereichszustand::ZuErledigen, "{b}");
        assert!(!zustand(b).gruende.is_empty(), "{b}");
    }
    assert_eq!(zustand("din277").zustand, Bereichszustand::Leer);
    assert!(
        zustand("verfahren")
            .gruende
            .iter()
            .any(|g| g.contains("in Vorbereitung")),
        "{:?}",
        zustand("verfahren").gruende
    );

    // Die Einreichungsprüfung meldet fehlende Nachweise und Antragsangaben rot.
    let pruefung = c
        .einreichungspruefung_am(p.id, None, tag, &ich)
        .await
        .unwrap();
    let bereich = |b: Pruefbereich| pruefung.bereiche.iter().find(|x| x.bereich == b).unwrap();
    assert_eq!(bereich(Pruefbereich::Nachweise).ampel, Ampel::Rot);
    assert!(bereich(Pruefbereich::Nachweise).rot >= 7);
    assert_eq!(bereich(Pruefbereich::Antrag).ampel, Ampel::Rot);
    assert!(
        pruefung
            .punkte
            .iter()
            .any(|p| p.bereich == Pruefbereich::Antrag && p.ziel.bereich == "bauantrag")
    );

    // Bewusst abgewählte Bereiche und abgewählte Anforderungen verlangen nichts.
    c.bereich_benoetigt_setzen(p.id, "entwaesserung", false, &ich)
        .await
        .unwrap();
    let f = c
        .fachobjekte(p.id, ObjektArt::Verfahren)
        .await
        .unwrap()
        .remove(0);
    let mut akte: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    for a in &mut akte.anforderungen {
        if a.nachweise == ["abstand"] {
            a.erforderlich = Erforderlichkeit::NichtErforderlich;
        }
    }
    c.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        f.objekt.parse().unwrap(),
        f.revision,
        serde_json::to_value(&akte).unwrap(),
        &ich,
    )
    .await
    .unwrap();
    let danach = ids(&c.projektuebersicht_am(p.id, tag, &ich).await.unwrap());
    assert!(
        !danach.contains(&"erforderlich:entwaesserung".to_string()),
        "{danach:?}"
    );
    assert!(
        !danach.contains(&"erforderlich:abstand".to_string()),
        "{danach:?}"
    );
    assert!(danach.contains(&"erforderlich:stellplaetze".to_string()));
}

#[tokio::test]
async fn profil_folgt_dem_vorhaben_und_behaelt_eigene_entscheidungen() {
    let (_dir, c) = core().await;
    let ich = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let mut recht = mit_bebauungsplan();
    recht.rechtsgrundlage = Some(Rechtsgrundlage::Par34);
    recht.bebauungsplan_art = BebauungsplanArt::Kein;
    recht.bau_nvo = None;
    recht.plan_bezeichnung.clear();
    recht.festgesetzte_werte = Default::default();
    let p = c
        .create_project(
            NewProject {
                name: "Werkhalle im Innenbereich".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: recht,
            },
            &ich,
        )
        .await
        .unwrap();
    let stand = c
        .verfahren_vorbereiten(p.id, Verfahrensart::Bauantrag, Some(false), &ich)
        .await
        .unwrap();
    let id: uuid::Uuid = stand.objekt.parse().unwrap();
    let mut akte: Verfahrensakte = serde_json::from_value(stand.daten).unwrap();
    let finde = |a: &Verfahrensakte, teil: &str| {
        a.anforderungen
            .iter()
            .find(|x| x.titel.contains(teil))
            .unwrap()
            .clone()
    };
    // Gewerbe im unbeplanten Innenbereich: beide Unterlagen sind einschlägig.
    assert_eq!(
        finde(&akte, "Betriebsbeschreibung").erforderlich,
        Erforderlichkeit::Erforderlich
    );
    assert_eq!(
        finde(&akte, "Liegenschaftskarte").erforderlich,
        Erforderlichkeit::Erforderlich
    );
    // Ohne Bebauungsplan verlangt die Anforderung „Maß der Nutzung“ GRZ und GFZ.
    let tag = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
    let punkte: Vec<String> = c
        .projektuebersicht_am(p.id, tag, &ich)
        .await
        .unwrap()
        .punkte
        .into_iter()
        .map(|p| p.id)
        .collect();
    assert!(
        punkte.contains(&"erforderlich:grz".to_string()),
        "{punkte:?}"
    );
    assert!(
        !punkte.contains(&"erforderlich:hoehen".to_string()),
        "{punkte:?}"
    );

    // Eigene Entscheidung mit eigener Begründung bleibt beim erneuten Anwenden.
    let eigen = akte
        .anforderungen
        .iter_mut()
        .find(|x| x.titel.contains("Gebäudeenergiegesetz"))
        .unwrap();
    eigen.erforderlich = Erforderlichkeit::NichtErforderlich;
    eigen.grund = "Unbeheizte Halle, mit der Bauaufsicht abgestimmt".into();
    let gespeichert = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            id,
            stand.revision,
            serde_json::to_value(&akte).unwrap(),
            &ich,
        )
        .await
        .unwrap();
    let mut profil = akte.profil.clone().unwrap();
    profil.vorhaben.sonderbau = Some(true);
    let neu = c
        .verfahren_profil_anwenden(
            p.id,
            id,
            gespeichert.revision,
            profil.vorhaben,
            profil.katalog,
            &ich,
        )
        .await
        .unwrap();
    let akte: Verfahrensakte = serde_json::from_value(neu.daten).unwrap();
    let geg = finde(&akte, "Gebäudeenergiegesetz");
    assert_eq!(geg.erforderlich, Erforderlichkeit::NichtErforderlich);
    assert!(geg.grund.contains("Unbeheizte Halle"));
    // Der Vorschlag folgt dem geänderten Profil.
    assert_eq!(
        finde(&akte, "Brandschutzkonzept").erforderlich,
        Erforderlichkeit::Erforderlich
    );
}
