//! Abweichungsbedarf aus den Abstandsflächen, Abgabesperre und eingefrorene
//! Abweichungen im Abgabestand.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    abstandsflaechen::{AbstandsStatus, berechnen, testdaten::plan},
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    verfahren::{
        Verfahrensakte,
        abgabe::Abgabewunsch,
        abweichungen::{Abweichung, Abweichungsart, Nachbarzustimmung},
    },
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn unterschreitung_sperrt_die_abgabe_bis_zum_antrag() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::open(&dir.path().join("buero.db"))
        .await
        .unwrap();
    let storage =
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap());
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Grenzgarage".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    // Wand jenseits der Grenze: die Abstandsfläche reicht über das Grundstück.
    let mut abstand = plan();
    abstand.waende[0].von.y = -1.;
    abstand.waende[0].bis.y = -1.;
    assert_eq!(
        berechnen(&abstand).unwrap().waende[0].status,
        AbstandsStatus::Konflikt
    );
    let plan_id = Uuid::new_v4();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Abstandsflaechen,
        plan_id,
        0,
        serde_json::to_value(&abstand).unwrap(),
        &u,
    )
    .await
    .unwrap();
    let lageplan = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &u,
        )
        .await
        .unwrap();
    let akte: Verfahrensakte = serde_json::from_value(serde_json::json!({
        "bezeichnung": "Bauantrag Grenzgarage", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
        "aktenzeichen": "", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    }))
    .unwrap();
    let vid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let wunsch = |id| Abgabewunsch {
        xbau_bezug: None,
        id,
        titel: "Bauantrag".into(),
        dokumente: vec![Dokumentbezug {
            dokument_id: lageplan.document_id,
            version: lageplan.version,
        }],
        xbau: false,
        druck: false,
        anschreiben: String::new(),
    };

    let g = core.verfahren_abweichungen(p.id, vid).await.unwrap();
    assert_eq!(g.offen.len(), 1, "{g:?}");
    let bedarf = g.bedarf[0].clone();
    assert_eq!(bedarf.art, Abweichungsart::Abweichung);
    assert_eq!(bedarf.rechtsgrundlage, "§ 69 BauO NRW");
    let e = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, wunsch(Uuid::new_v4()), &u)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Abgabe gesperrt"), "{e}");

    // Antrag ohne Begründung genügt nicht.
    let (stand, mut a) = core.verfahrensakte(p.id, vid).await.unwrap();
    let mut abw = Abweichung {
        id: Uuid::new_v4(),
        art: bedarf.art,
        vorschrift: bedarf.vorschrift.clone(),
        rechtsgrundlage: bedarf.rechtsgrundlage.clone(),
        gegenstand: bedarf.gegenstand.clone(),
        soll: bedarf.soll.clone(),
        ist: bedarf.ist.clone(),
        begruendung: String::new(),
        herkunft: Some(bedarf.schluessel.clone()),
        nachbarn: vec![Nachbarzustimmung {
            id: Uuid::new_v4(),
            name: "Familie Nachbar".into(),
            flurstueck: "Flur 1, 12".into(),
            zugestimmt: Some(false),
            datum: Some("2026-01-05".parse().unwrap()),
            beleg: None,
        }],
        dokumente: vec![],
        xbau_code: String::new(),
        entfallen: None,
    };
    a.abweichungen.push(abw.clone());
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            stand.revision,
            serde_json::to_value(&a).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let e = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, wunsch(Uuid::new_v4()), &u)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Begründung"), "{e}");

    abw.begruendung =
        "Garage ohne Aufenthaltsräume; Belichtung des Nachbargrundstücks bleibt gewahrt.".into();
    let (_, mut a) = core.verfahrensakte(p.id, vid).await.unwrap();
    a.abweichungen[0] = abw.clone();
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&a).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let id = Uuid::new_v4();
    let f = core
        .verfahren_abgabe_festhalten(p.id, vid, f.revision, wunsch(id), &u)
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p.id, vid).await.unwrap();
    let stand = a.abgaben.iter().find(|b| b.id == id).unwrap();
    assert_eq!(stand.grundlagen.abweichungen, vec![abw.clone()]);
    assert!(
        stand
            .hinweise
            .iter()
            .any(|h| h.contains("nicht zugestimmt")),
        "{:?}",
        stand.hinweise
    );

    // Einträge werden nicht gelöscht, sondern begründet als entfallen geführt.
    let mut ohne = a.clone();
    ohne.abweichungen.clear();
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&ohne).unwrap(),
            &u,
        )
        .await
        .is_err()
    );
    let mut entfallen = a.clone();
    entfallen.abweichungen[0].entfallen = Some("Garage um 1 m verschoben".into());
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid,
        f.revision,
        serde_json::to_value(&entfallen).unwrap(),
        &u,
    )
    .await
    .unwrap();
}
