//! Ablehnung mit Gründen, Folgeverfahren und Bekanntgabe an Nachbarn.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    verfahren::{
        Ablehnungsgrund, Bescheid, Entscheidung, Nachbarbekanntgabe, Verfahrensakte, Verfahrensart,
        Verfahrensstatus,
        abgabe::Abgabewunsch,
        externe_einreichung::{Einreichungsweg, ExterneEinreichungswunsch},
        optionen::{Folgeverfahrenswunsch, Optionsaktion},
        termine::{Terminart, termine},
    },
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn ablehnung_folgeverfahren_und_nachbarn() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Anbau".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap()
        .id;
    let pdf = core
        .store_document(
            p,
            None,
            None,
            "Plan.pdf",
            Bytes::from(core_lib::formular::testdaten::musterformular()),
            &u,
        )
        .await
        .unwrap();
    let dok = Dokumentbezug {
        dokument_id: pdf.document_id,
        version: pdf.version,
    };
    let akte: Verfahrensakte = serde_json::from_value(serde_json::json!({
        "bezeichnung": "Bauantrag Anbau", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "bab:x", "kontakt": ""},
        "aktenzeichen": "BA-7", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "kommunikation": [], "nachforderungen": [], "anforderungen": [],
        "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    }))
    .unwrap();
    let vid = Uuid::new_v4();
    let f = core
        .fachobjekt_speichern(
            p,
            ObjektArt::Verfahren,
            vid,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let tag = |s: &str| s.parse().unwrap();
    let f = core
        .verfahren_externe_einreichung(
            p,
            vid,
            f.revision,
            ExterneEinreichungswunsch {
                id: Uuid::new_v4(),
                datum: tag("2026-03-01"),
                weg: Einreichungsweg::Post,
                beleg: None,
                bemerkung: String::new(),
                dokumente: vec![dok.clone()],
                berichtigt: None,
            },
            &u,
        )
        .await
        .unwrap();

    // Ablehnung mit Grund.
    let mut a: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let bid = Uuid::new_v4();
    a.status = Verfahrensstatus::Entschieden;
    a.bescheide.push(Bescheid {
        id: bid,
        entscheidung: Entscheidung::Abgelehnt,
        datum: tag("2026-06-01"),
        zugestellt: Some(tag("2026-06-03")),
        original: dok.clone(),
        genehmigte_unterlagen: vec![],
        auflagen: vec![],
        ersetzt: None,
        verlaengerung: None,
        teilbaugenehmigung: false,
        abweichungsentscheidungen: vec![],
    });
    a.ablehnungsgruende.push(Ablehnungsgrund {
        id: Uuid::new_v4(),
        bescheid: bid,
        text: "Überschreitung der Baugrenze um 1,5 m".into(),
        grundlage: "§ 23 BauNVO".into(),
        reaktion: String::new(),
        abweichung: None,
        folgeverfahren: None,
    });
    core.fachobjekt_speichern(
        p,
        ObjektArt::Verfahren,
        vid,
        f.revision,
        serde_json::to_value(&a).unwrap(),
        &u,
    )
    .await
    .unwrap();
    let lage = core.verfahren_optionen(p, vid).await.unwrap();
    assert!(
        lage.lage.contains("Abgelehnt am 01.06.2026"),
        "{}",
        lage.lage
    );
    let rb = lage
        .optionen
        .iter()
        .find(|o| o.titel == "Rechtsbehelf prüfen")
        .unwrap();
    assert_eq!(rb.frist, Some(tag("2026-07-03")));
    assert!(lage.optionen.iter().any(|o| o.aktion
        == Some(Optionsaktion::Folgeverfahren {
            verfahrensart: Verfahrensart::Aenderungsantrag
        })));

    // Folgeverfahren: Bezug, Behörde, Gründe verweisen auf das neue Verfahren.
    let neu = Uuid::new_v4();
    core.verfahren_folgeverfahren(
        p,
        vid,
        Folgeverfahrenswunsch {
            id: neu,
            art: Verfahrensart::Aenderungsantrag,
            bezeichnung: "Geänderter Bauantrag".into(),
        },
        &u,
    )
    .await
    .unwrap();
    let (fneu, b) = core.verfahrensakte(p, neu).await.unwrap();
    assert_eq!(b.vorgaenger, Some(vid));
    assert_eq!(b.behoerde.kennung, "bab:x");
    let (_, alt) = core.verfahrensakte(p, vid).await.unwrap();
    assert_eq!(alt.ablehnungsgruende[0].folgeverfahren, Some(neu));
    core.verfahren_abgabe_festhalten(
        p,
        neu,
        fneu.revision,
        Abgabewunsch {
            xbau_bezug: None,
            xbau_folge: None,
            id: Uuid::new_v4(),
            titel: "Geänderter Antrag".into(),
            dokumente: vec![dok.clone()],
            xbau: false,
            druck: false,
            anschreiben: String::new(),
        },
        &u,
    )
    .await
    .unwrap();
    let (_, b) = core.verfahrensakte(p, neu).await.unwrap();
    assert!(
        b.abgaben[0]
            .hinweise
            .iter()
            .any(|h| h.contains("Überschreitung der Baugrenze")),
        "{:?}",
        b.abgaben[0].hinweise
    );

    // Gespeicherter Wortlaut eines Grundes bleibt unverändert.
    let (s, mut alt) = core.verfahrensakte(p, vid).await.unwrap();
    alt.ablehnungsgruende[0].text = "anders".into();
    assert!(
        core.fachobjekt_speichern(
            p,
            ObjektArt::Verfahren,
            vid,
            s.revision,
            serde_json::to_value(&alt).unwrap(),
            &u,
        )
        .await
        .is_err()
    );

    // Genehmigung im Folgeverfahren: Nachbar erhält eine eigene Frist.
    let (s, _) = core.verfahrensakte(p, neu).await.unwrap();
    let f = core
        .verfahren_externe_einreichung(
            p,
            neu,
            s.revision,
            ExterneEinreichungswunsch {
                id: Uuid::new_v4(),
                datum: tag("2026-07-01"),
                weg: Einreichungsweg::Post,
                beleg: None,
                bemerkung: String::new(),
                dokumente: vec![dok.clone()],
                berichtigt: None,
            },
            &u,
        )
        .await
        .unwrap();
    let mut b: Verfahrensakte = serde_json::from_value(f.daten).unwrap();
    let gid = Uuid::new_v4();
    b.status = Verfahrensstatus::Entschieden;
    b.bescheide.push(Bescheid {
        id: gid,
        entscheidung: Entscheidung::Genehmigt,
        datum: tag("2026-08-10"),
        zugestellt: Some(tag("2026-08-12")),
        original: dok.clone(),
        genehmigte_unterlagen: vec![dok.clone()],
        auflagen: vec![],
        ersetzt: None,
        verlaengerung: None,
        teilbaugenehmigung: false,
        abweichungsentscheidungen: vec![],
    });
    b.nachbarbekanntgaben.push(Nachbarbekanntgabe {
        id: Uuid::new_v4(),
        bescheid: gid,
        name: "Familie Nachbar".into(),
        datum: tag("2026-08-20"),
        beleg: None,
        angefochten: None,
    });
    core.fachobjekt_speichern(
        p,
        ObjektArt::Verfahren,
        neu,
        f.revision,
        serde_json::to_value(&b).unwrap(),
        &u,
    )
    .await
    .unwrap();
    let (_, b) = core.verfahrensakte(p, neu).await.unwrap();
    let t = termine(&b, neu, Bundesland::Nw);
    assert!(
        t.iter().any(|t| t.art == Terminart::Rechtsbehelfsfrist
            && t.titel.contains("Familie Nachbar")
            && t.datum == tag("2026-09-21")),
        "{t:?}"
    );

    // Baubeginn: Checkliste aus der Akte; eine neue Planversion nach der
    // Genehmigung ergibt den Hinweis auf einen Änderungsantrag (Tektur).
    use core_lib::verfahren::ausfuehrung::Phase;
    let c = core
        .verfahren_checkliste(p, neu, Phase::Baubeginn)
        .await
        .unwrap();
    assert!(!c.bereit);
    assert!(
        c.voraussetzungen
            .iter()
            .any(|v| v.titel == "Bauleitung benannt" && v.erfuellt == Some(false))
    );
    core.store_document(
        p,
        Some(pdf.document_id),
        Some(pdf.version),
        "Plan.pdf",
        Bytes::from(core_lib::formular::testdaten::musterformular()),
        &u,
    )
    .await
    .unwrap();
    let c = core
        .verfahren_checkliste(p, neu, Phase::Baubeginn)
        .await
        .unwrap();
    assert!(
        c.hinweise.iter().any(|h| h.contains("Tektur")),
        "{:?}",
        c.hinweise
    );
}
