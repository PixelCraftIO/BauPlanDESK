//! NRW-Bauvorlagenkatalog mit Unterlagenvorschlag, Unterschriftspflichten im
//! Abgabestand und Verfahrenswahl aus den Projektdaten.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    dokumentkategorien::KategorieZuordnung,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    verfahren::{
        Verfahrensakte, Verfahrensart,
        abgabe::Abgabewunsch,
        landesrecht::{
            Eignung, Unterschrift, Unterschriftsart, Unterschriftsrolle, Verfahrensangaben,
        },
        profile::{Anforderungskatalog, Vorhabenprofil},
    },
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn nrw_katalog_unterschriften_und_verfahrenswahl() {
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
                name: "EFH".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap()
        .id;
    let ablegen = |name: &'static str, kategorie: &'static str| {
        let core = &core;
        let u = &u;
        async move {
            let d = core
                .store_document(
                    p,
                    None,
                    None,
                    name,
                    Bytes::from(core_lib::formular::testdaten::musterformular()),
                    u,
                )
                .await
                .unwrap();
            core.dokumentkategorie_speichern(
                d.document_id,
                KategorieZuordnung {
                    revision: 0,
                    kategorie: Some(kategorie.into()),
                },
                u,
            )
            .await
            .unwrap();
            d
        }
    };
    let antrag = ablegen("Bauantrag.pdf", "bauantrag").await;
    let grundriss = ablegen("Grundriss EG.pdf", "grundriss").await;
    let akte: Verfahrensakte = serde_json::from_value(serde_json::json!({
        "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
        "aktenzeichen": "", "verantwortlich": null, "vorgaenger": null, "profil": null,
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

    // Katalog: Unterlagen passender Kategorie werden vorgeschlagen.
    let nrw = Anforderungskatalog::mitgeliefert()
        .unwrap()
        .into_iter()
        .find(|k| k.id == "nrw-bauvorlagen")
        .unwrap();
    let f = core
        .verfahren_profil_anwenden(
            p,
            vid,
            f.revision,
            Vorhabenprofil {
                bundesland: Bundesland::Nw,
                art: Verfahrensart::Bauantrag,
                gebaeudeklasse: Some(1),
                sonderbau: Some(false),
                nutzung: "Wohnen".into(),
            },
            nrw,
            &u,
        )
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    let zeichnungen = a
        .anforderungen
        .iter()
        .find(|r| r.titel.starts_with("Bauzeichnungen"))
        .unwrap();
    assert_eq!(
        zeichnungen.dokumente,
        vec![Dokumentbezug {
            dokument_id: grundriss.document_id,
            version: grundriss.version
        }]
    );
    assert!(!zeichnungen.freigegeben);
    assert!(
        !a.anforderungen
            .iter()
            .any(|r| r.quelle.contains("§ 16 BauPrüfVO")),
        "Vorbescheidsunterlagen gehören nicht zum Bauantrag"
    );

    // Unterschriften: Bedarf je Kategorie; der Abgabestand nennt Fehlendes.
    let bedarf = core.verfahren_unterschriften(p, vid).await.unwrap();
    let b_antrag = bedarf
        .iter()
        .find(|b| b.dokument.dokument_id == antrag.document_id)
        .unwrap();
    assert_eq!(
        b_antrag.fehlend,
        vec![
            Unterschriftsrolle::Bauherrschaft,
            Unterschriftsrolle::Entwurfsverfasser
        ]
    );
    let (_, mut a) = core.verfahrensakte(p, vid).await.unwrap();
    a.unterschriften.push(Unterschrift {
        id: Uuid::new_v4(),
        dokument: b_antrag.dokument.clone(),
        rolle: Unterschriftsrolle::Entwurfsverfasser,
        name: "Anna Architektin".into(),
        art: Unterschriftsart::Qualifiziert,
        datum: core_lib::kalender::heute(),
    });
    let f = core
        .fachobjekt_speichern(
            p,
            ObjektArt::Verfahren,
            vid,
            f.revision,
            serde_json::to_value(&a).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let id = Uuid::new_v4();
    core.verfahren_abgabe_festhalten(
        p,
        vid,
        f.revision,
        Abgabewunsch {
            xbau_bezug: None,
            xbau_folge: None,
            id,
            titel: "Bauantrag".into(),
            dokumente: [&antrag, &grundriss]
                .iter()
                .map(|d| Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                })
                .collect(),
            xbau: false,
            druck: false,
            anschreiben: String::new(),
        },
        &u,
    )
    .await
    .unwrap();
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    let h = &a.abgaben[0].hinweise;
    assert!(
        h.iter()
            .any(|h| h.starts_with("Bauantrag.pdf: Unterschrift Bauherrschaft nicht erfasst")),
        "{h:?}"
    );
    assert!(
        h.iter()
            .any(|h| h.starts_with("Grundriss EG.pdf: Unterschrift Entwurfsverfasser/in")),
        "{h:?}"
    );

    // Verfahrenswahl: ohne Bebauungsplan-Angabe bleibt die Freistellung offen.
    let w = core
        .verfahrenswahl(
            p,
            Some(vid),
            Verfahrensangaben {
                stichtag: Some("2026-08-15".parse().unwrap()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(w.merkmale.gebaeudeklasse, Some(1));
    assert_eq!(w.merkmale.wohngebaeude, Some(true));
    assert_eq!(w.vorschlaege[0].eignung, Eignung::Pruefen);
    assert_eq!(w.vorschlaege[1].eignung, Eignung::Moeglich);
    assert!(w.fiktion.is_none());
    let neu = core
        .verfahrenswahl(p, Some(vid), Verfahrensangaben::default())
        .await
        .unwrap();
    assert!(neu.fiktion.is_some(), "Anträge ab 01.09.2026");
}
