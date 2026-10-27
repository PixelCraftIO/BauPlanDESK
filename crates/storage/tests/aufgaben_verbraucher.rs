//! Aufmaß-Aufgaben für Fachberechnungen und Raumbücher: Positionen einer
//! DIN-277-Berechnung und Räume eines Raumbuchs erscheinen als Aufgaben,
//! passende freie Markierungen werden vorgeschlagen, die Übernahme läuft über
//! `planaufmass_einsetzen` bzw. `planaufmass_raumbuch_einsetzen`, und die
//! Sammelübernahme des Raumbuchs legt keine Dubletten für bereits
//! zugeordnete Räume an. Dazu die Werkzeugkiste als feste Vorlagenliste.
use core_lib::{
    CoreServices,
    berechnungen::{BerechnungsArt, vorlagen},
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planaufmass::*,
    plattform::{
        ObjektArt,
        objekte::{Raum, Raumbuch},
    },
};
use std::sync::Arc;
use uuid::Uuid;

fn markierung(
    name: &str,
    bedeutung: Planbedeutung,
    ebene: Uuid,
    haus: Uuid,
    geschoss: Uuid,
    von: [f64; 2],
    bis: [f64; 2],
) -> Planmarkierung {
    Planmarkierung {
        id: Uuid::new_v4(),
        ebene_id: ebene,
        bezeichnung: name.into(),
        bedeutung,
        geometrie: Plangeometrie::Rechteck { von, bis },
        grundstueck_id: None,
        gebaeude_id: Some(haus),
        geschoss_id: Some(geschoss),
        flurstueck_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
    }
}

#[tokio::test]
async fn aufgaben_fuer_berechnung_und_raumbuch() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Aufgaben".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let (haus, eg) = (Uuid::new_v4(), Uuid::new_v4());
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Gebaeude,
        haus,
        0,
        serde_json::json!({"bezeichnung":"Haus","grundstueck_ids":[],"geschosse":[{"id":eg,"bezeichnung":"EG","vollgeschoss":null,"grundlage":""}],"quelle":null}),
        &user,
    )
    .await
    .unwrap();

    // Eigene Zeichnung in Metern: NUF „Wohnen“ (24 m²), Räume „Küche“ (12 m²) und „Flur“ (6 m²).
    let e = Uuid::new_v4();
    let wohnen = markierung("Wohnen", Planbedeutung::Nuf, e, haus, eg, [0., 0.], [6., 4.]);
    let kueche = markierung("Küche", Planbedeutung::Raum, e, haus, eg, [6., 0.], [10., 3.]);
    let flur = markierung("Flur", Planbedeutung::Raum, e, haus, eg, [6., 3.], [10., 4.5]);
    let z = Planaufmass {
        bezeichnung: "Skizze EG".into(),
        dokument: None,
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Einheit {
            einheit: core_lib::cad::Laengeneinheit::Meter,
        }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: e,
            bezeichnung: "Räume".into(),
            sichtbar: true,
            gesperrt: false,
        }],
        markierungen: vec![wohnen.clone(), kueche.clone(), flur.clone()],
        ausschnitte: vec![],
    };
    let aid = Uuid::new_v4();
    let gespeichert = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planaufmass,
            aid,
            0,
            serde_json::to_value(&z).unwrap(),
            &user,
        )
        .await
        .unwrap();

    // DIN-277-Berechnung des Gebäudes mit einer NUF-Position; Raumbuch mit manuell erfasster Küche.
    let mut din = vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == BerechnungsArt::Din277)
        .unwrap()
        .plan;
    din.gebaeude_id = Some(haus);
    din.eingaben["positionen"] = serde_json::json!([{"id": "raum1", "geschoss": "EG", "raum": "Wohnen", "art": "nuf", "bereich": "R",
        "form": "aufmass", "laenge_m": 0, "breite_m": 0, "breite2_m": 0, "aufmass_m2": 0, "quelle": ""}]);
    let din_id = Uuid::new_v4();
    let din_stand = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Din277,
            din_id,
            0,
            serde_json::to_value(&din).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let kueche_raum = Uuid::new_v4();
    let raumbuch = Raumbuch {
        planbindungen: vec![],
        bezeichnung: "EG".into(),
        gebaeude_id: haus,
        geschoss_id: Some(eg),
        raeume: vec![Raum {
            id: kueche_raum,
            nummer: "1".into(),
            bezeichnung: "Küche".into(),
            nutzung: "".into(),
            flaeche_m2: 11.,
            hoehe_m: Some(2.6),
            hoehenzonen: vec![],
        }],
    };
    let rb_id = Uuid::new_v4();
    let rb_stand = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Raumbuch,
            rb_id,
            0,
            serde_json::to_value(&raumbuch).unwrap(),
            &user,
        )
        .await
        .unwrap();

    // 1. Aufgaben: NUF-Position mit „Wohnen“, Küche mit „Küche“, weitere Räume mit „Flur“.
    let aufgaben = core.planaufmass_aufgaben(p.id, &z).await.unwrap();
    let finde = |verbraucher: &str, ziel: &str| {
        aufgaben
            .iter()
            .find(|a| a.verbraucher == verbraucher && a.ziel == ziel)
            .unwrap_or_else(|| panic!("Aufgabe {verbraucher} {ziel}"))
            .clone()
    };
    let nuf = finde("berechnung", "positionen.raum1.aufmass_m2");
    assert_eq!(nuf.modul.as_deref(), Some("din277"));
    assert_eq!(nuf.objekt, din_id.to_string());
    assert_eq!(nuf.bedeutung, Planbedeutung::Nuf);
    assert_eq!(nuf.geometrie, Aufgabengeometrie::Flaeche);
    assert_eq!(nuf.gebaeude_id, Some(haus));
    assert_eq!(nuf.markierung, Some(wohnen.id));
    assert!(nuf.gebunden.is_none() && !nuf.wert_vorhanden);
    assert!(nuf.titel.contains("Wohnen"), "{}", nuf.titel);
    assert!(
        nuf.objekt_name.contains("DIN-277"),
        "Modultitel als Objektname: {}",
        nuf.objekt_name
    );
    let k = finde("raumbuch", &format!("raeume.{kueche_raum}.flaeche_m2"));
    assert_eq!(k.markierung, Some(kueche.id), "Raumkontur mit Raumnamen");
    assert!(k.wert_vorhanden && k.gebunden.is_none());
    assert_eq!(k.geschoss_id, Some(eg));
    // NUF-Konturen zählen wie im Raumbuch-Import als Räume; die erste freie ist „Wohnen“.
    let weitere = finde("raumbuch", "raeume");
    assert_eq!(weitere.markierung, Some(wohnen.id));
    assert!(weitere.wert_vorhanden);
    assert!(
        !aufgaben
            .iter()
            .any(|a| a.verbraucher == "berechnung" && a.ziel.starts_with("volumen.")),
        "ohne Volumenkörper keine Volumenaufgaben"
    );

    // 2. Übernahme in die Berechnung: Bindung eintragen, einsetzen, speichern → erledigt.
    let bindung = |m: Uuid, ziel: &str| Planbindung {
        aufmass_id: aid,
        revision: gespeichert.revision,
        markierung_id: m,
        groesse: Planmessgroesse::Flaeche,
        ziel: ziel.into(),
    };
    din.planbindungen = vec![bindung(wohnen.id, "positionen.raum1.aufmass_m2")];
    let eingesetzt = core.planaufmass_einsetzen(p.id, din.clone()).await.unwrap();
    assert_eq!(eingesetzt.eingaben["positionen"][0]["aufmass_m2"], 24.);
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Din277,
        din_id,
        din_stand.revision,
        serde_json::to_value(&eingesetzt).unwrap(),
        &user,
    )
    .await
    .unwrap();

    // 3. Übernahme je Raum: die manuell erfasste Küche erhält die gezeichnete Fläche; Nummer und Höhe bleiben.
    let mut r = raumbuch.clone();
    r.planbindungen = vec![bindung(
        kueche.id,
        &format!("raeume.{kueche_raum}.flaeche_m2"),
    )];
    let r = core.planaufmass_raumbuch_einsetzen(p.id, r).await.unwrap();
    assert!((r.raeume[0].flaeche_m2 - 12.).abs() < 1e-9);
    assert_eq!(r.raeume[0].nummer, "1");
    assert_eq!(r.raeume[0].hoehe_m, Some(2.6));
    let rb_stand = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Raumbuch,
            rb_id,
            rb_stand.revision,
            serde_json::to_value(&r).unwrap(),
            &user,
        )
        .await
        .unwrap();
    let aufgaben = core.planaufmass_aufgaben(p.id, &z).await.unwrap();
    let nuf = aufgaben
        .iter()
        .find(|a| a.verbraucher == "berechnung" && a.ziel == "positionen.raum1.aufmass_m2")
        .unwrap();
    assert_eq!(nuf.gebunden, Some(wohnen.id));
    let k = aufgaben
        .iter()
        .find(|a| a.verbraucher == "raumbuch" && a.ziel == format!("raeume.{kueche_raum}.flaeche_m2"))
        .unwrap();
    assert_eq!(k.gebunden, Some(kueche.id));
    assert_eq!(
        aufgaben
            .iter()
            .find(|a| a.verbraucher == "raumbuch" && a.ziel == "raeume")
            .unwrap()
            .markierung,
        Some(wohnen.id),
        "Wohnen und Flur bleiben als weitere Räume offen"
    );

    // 4. Sammelübernahme: Wohnen und Flur werden neue Räume, die Küche bleibt ein Raum (keine Dublette).
    let r: Raumbuch = serde_json::from_value(rb_stand.daten).unwrap();
    let r = core
        .planaufmass_raumbuch(p.id, aid, gespeichert.revision, r)
        .await
        .unwrap();
    let namen: Vec<_> = r.raeume.iter().map(|x| x.bezeichnung.as_str()).collect();
    assert_eq!(namen, ["Küche", "Wohnen", "Flur"], "{namen:?}");
    assert_eq!(r.raeume[0].id, kueche_raum);
    assert!((r.raeume[0].flaeche_m2 - 12.).abs() < 1e-9);
    assert!((r.raeume[2].flaeche_m2 - 6.).abs() < 1e-9);
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Raumbuch,
        rb_id,
        rb_stand.revision,
        serde_json::to_value(&r).unwrap(),
        &user,
    )
    .await
    .unwrap();
    let aufgaben = core.planaufmass_aufgaben(p.id, &z).await.unwrap();
    assert!(
        !aufgaben
            .iter()
            .any(|a| a.verbraucher == "raumbuch" && a.ziel == "raeume"),
        "ohne freie Raumkonturen keine Sammelaufgabe"
    );
    assert_eq!(
        aufgaben
            .iter()
            .filter(|a| a.verbraucher == "raumbuch" && a.gebunden.is_some())
            .count(),
        3
    );

    // 5. Werkzeugkiste: eindeutige Kennungen, bestimmte Bedeutungen, Höhen als Höhenpunkt oder Strecke.
    let kiste = werkzeugkiste();
    let ids: std::collections::HashSet<_> = kiste.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids.len(), kiste.len());
    assert!(kiste.iter().all(|v| v.bedeutung != Planbedeutung::Unbestimmt));
    assert!(kiste.iter().any(|v| v.titel == "DIN 277 NUF" && v.bedeutung == Planbedeutung::Nuf));
    assert!(kiste.iter().any(|v| v.titel == "Versiegelte Fläche"));
    assert!(
        kiste
            .iter()
            .filter(|v| v.gruppe == "Höhen")
            .all(|v| v.geometrie != Aufgabengeometrie::Flaeche)
    );
}
