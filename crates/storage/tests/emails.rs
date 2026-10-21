//! Originaldateien und konkrete Versionen bleiben beim Lesen unverändert.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    domain::{Bundesland, NewProject},
    email::testdaten::{muster_eml, muster_msg},
    lbo::LboEngine,
    license::DevelopmentLicense,
};
use std::sync::Arc;
#[tokio::test]
async fn archivierte_nachrichten_lesen_ohne_neue_datei_oder_version() {
    let tmp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(tmp.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("mail-archiv", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Mailarchiv".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let eml = muster_eml();
    let msg = muster_msg();
    let d = core
        .store_document_kategorisiert(
            p.id,
            "Bauantrag.eml",
            Bytes::copy_from_slice(&eml),
            Some("email".into()),
            &user,
        )
        .await
        .unwrap();
    let v2 = core
        .store_document(
            p.id,
            Some(d.document_id),
            Some(1),
            "Bauantrag.eml",
            Bytes::from(
                String::from_utf8(eml.clone())
                    .unwrap()
                    .replace("Bauantrag Musterstrasse", "Bauantrag korrigiert"),
            ),
            &user,
        )
        .await
        .unwrap();
    let m = core
        .store_document_kategorisiert(
            p.id,
            "Nachforderung.msg",
            Bytes::copy_from_slice(&msg),
            Some("email".into()),
            &user,
        )
        .await
        .unwrap();
    let vorher = core
        .repos
        .protokoll
        .list_for_project(p.id, 100)
        .await
        .unwrap()
        .len();
    let gelesen1 = core.archivierte_email(d.document_id, 1).await.unwrap();
    let gelesen2 = core.archivierte_email(d.document_id, 2).await.unwrap();
    let gelesen_msg = core.archivierte_email(m.document_id, 1).await.unwrap();
    assert_eq!(gelesen1.inhalt.betreff, "Bauantrag Musterstrasse");
    assert_eq!(gelesen2.inhalt.betreff, "Bauantrag korrigiert");
    assert_eq!(gelesen_msg.inhalt.betreff, "Nachforderung Lageplan");
    assert_eq!(gelesen_msg.inhalt.anhaenge[0].name, "Anmerkungen.pdf");
    assert_eq!(core.load_version(d.document_id, 1).await.unwrap().1, eml);
    assert_eq!(core.load_version(m.document_id, 1).await.unwrap().1, msg);
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 2);
    assert_eq!(core.document_history(m.document_id).await.unwrap().len(), 1);
    assert_eq!(
        core.repos
            .protokoll
            .list_for_project(p.id, 100)
            .await
            .unwrap()
            .len(),
        vorher
    );
    assert!(matches!(
        core.archivierte_email(d.document_id, 3).await,
        Err(CoreError::NotFound(_))
    ));
    let kaputt = core
        .store_document(
            p.id,
            None,
            None,
            "Kaputt.msg",
            Bytes::from_static(b"kein MSG"),
            &user,
        )
        .await
        .unwrap();
    assert!(matches!(
        core.archivierte_email(kaputt.document_id, 1).await,
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(
        core.load_version(kaputt.document_id, 1).await.unwrap().1,
        b"kein MSG".as_slice()
    );
    let mut anhaenge = Vec::new();
    // Anhänge werden bewusst übernommen; Wiederholung erzeugt keine zweite Datei.
    for (quelle, erwartet) in [
        (d.document_id, "Lageplan.pdf"),
        (m.document_id, "Anmerkungen.pdf"),
    ] {
        let anhang = core
            .email_anhang_ablegen(quelle, 1, 0, &user)
            .await
            .unwrap();
        let erneut = core
            .email_anhang_ablegen(quelle, 1, 0, &user)
            .await
            .unwrap();
        assert_eq!(anhang.document_id, erneut.document_id);
        assert_eq!(anhang.file_name, erwartet);
        assert_eq!(
            core.document_history(anhang.document_id)
                .await
                .unwrap()
                .len(),
            1
        );
        let z = core
            .repos
            .commit
            .dokumentzusatz(anhang.document_id, 1, "emailquelle")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(z.daten["dokument_id"], quelle.0.to_string());
        anhaenge.push(anhang);
    }
    // Reale Vergleichsdaten: dieselbe Dateiversion, später eine andere Kategorie.
    core.dokumentkategorie_speichern(
        d.document_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 0,
            kategorie: Some("anschreiben".into()),
        },
        &user,
    )
    .await
    .unwrap();
    let paket = core
        .einreichung_erstellen(
            p.id,
            core_lib::einreichung::NeuesPaket {
                titel: "Bauantrag".into(),
                empfaenger: "Bauamt".into(),
                dokumente: vec![core_lib::einreichung::DokumentAuswahl {
                    dokument_id: d.document_id,
                    version: 2,
                }],
            },
            &user,
        )
        .await
        .unwrap();
    core.einreichung_bestaetigen_altbestand(
        paket.stand.id,
        1,
        v2.created_at.date_naive(),
        "Portal".into(),
        &user,
    )
    .await
    .unwrap();
    core.dokumentkategorie_speichern(
        d.document_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 1,
            kategorie: None,
        },
        &user,
    )
    .await
    .unwrap();
    let vergleich = core.einreichungs_vergleich(p.id, "Bauamt").await.unwrap();
    let row = vergleich
        .iter()
        .find(|v| v.aktuell.document_id == d.document_id)
        .unwrap();
    assert!(row.eingereichte_kategorie_erfasst);
    assert!(row.kategorie.is_none());
    assert_eq!(
        row.eingereichte_kategorie.as_ref().unwrap().name,
        "Anschreiben"
    );
    assert_eq!(
        row.status,
        core_lib::einreichung::VergleichsStatus::BereitsEingereicht
    );
    if let Ok(root) = std::env::var("BAUPROJEKT_EMAIL_ARTEFAKTE") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let wf = serde_json::json!({"projekt":p,"eml1":gelesen1,"eml2":gelesen2,"msg":gelesen_msg,"versionen":[d,v2],"kaputt":kaputt,"anhaenge":anhaenge,"katalog":core.kategorienkatalog().await.unwrap(),"zuordnungen":core.dokumentkategorien(p.id).await.unwrap(),"vergleich":vergleich});
        std::fs::write(
            root.join("workflow.json"),
            serde_json::to_vec_pretty(&wf).unwrap(),
        )
        .unwrap();
    }
}

#[tokio::test]
async fn email_wird_einmal_als_kommunikation_mit_anhang_uebernommen() {
    use core_lib::{
        plattform::{ObjektArt, objekte::Dokumentbezug},
        verfahren::{Bezug, Kommunikationsart},
    };
    let tmp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(tmp.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("mail-archiv", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Mailarchiv".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document_kategorisiert(
            p.id,
            "Bauantrag.eml",
            Bytes::from(muster_eml()),
            Some("email".into()),
            &user,
        )
        .await
        .unwrap();
    let anhang = core
        .email_anhang_ablegen(d.document_id, 1, 0, &user)
        .await
        .unwrap();
    let mut akten = Vec::new();
    for name in ["Erstantrag", "Zweite Akte"] {
        let vid = uuid::Uuid::new_v4();
        let a = serde_json::json!({"bezeichnung":name,"art":"bauantrag","status":"vorbereitung","behoerde":{"id":uuid::Uuid::new_v4(),"name":"Bauamt","kennung":"bab:091620000","kontakt":""},"aktenzeichen":"","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]});
        core.fachobjekt_speichern(p.id, ObjektArt::Verfahren, vid, 0, a, &user)
            .await
            .unwrap();
        akten.push(vid);
    }
    let email = Dokumentbezug {
        dokument_id: d.document_id,
        version: 1,
    };
    let (stand, _) = core.verfahrensakte(p.id, akten[0]).await.unwrap();
    // Datum vor dem Sendedatum (15.10.2026) ist unzulässig.
    let tag = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
    assert!(
        core.email_in_verfahren(
            p.id,
            email.clone(),
            akten[0],
            stand.revision,
            false,
            Some(tag("2026-10-14")),
            &user
        )
        .await
        .is_err()
    );
    let f = core
        .email_in_verfahren(
            p.id,
            email.clone(),
            akten[0],
            stand.revision,
            false,
            None,
            &user,
        )
        .await
        .unwrap();
    let (_, a) = core.verfahrensakte(p.id, akten[0]).await.unwrap();
    let k = &a.kommunikation[0];
    assert_eq!(k.art, Kommunikationsart::Email);
    assert!(!k.eingehend);
    assert_eq!(k.datum, tag("2026-10-15"));
    assert_eq!(k.betreff, "Bauantrag Musterstrasse");
    assert_eq!(
        k.beteiligte,
        "Von: Daniel Müller <daniel@example.org>\nAn: Bauamt <amt@example.org>"
    );
    assert!(k.inhalt.starts_with("Bitte pruefen Sie den Bauantrag."));
    assert!(
        k.inhalt
            .contains("- Lageplan.pdf (als Projektunterlage abgelegt)")
    );
    assert_eq!(k.original.as_ref(), Some(&email));
    assert_eq!(
        k.bezuege,
        [Bezug::Dokument {
            dokument: Dokumentbezug {
                dokument_id: anhang.document_id,
                version: anhang.version,
            }
        }]
    );
    // Wiederholt: keine Änderung; in einer anderen Akte: abgewiesen.
    let wieder = core
        .email_in_verfahren(
            p.id,
            email.clone(),
            akten[0],
            f.revision,
            false,
            None,
            &user,
        )
        .await
        .unwrap();
    assert_eq!(wieder.revision, f.revision);
    let (zweite, _) = core.verfahrensakte(p.id, akten[1]).await.unwrap();
    let err = core
        .email_in_verfahren(p.id, email, akten[1], zweite.revision, true, None, &user)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Erstantrag"), "{err}");
}
