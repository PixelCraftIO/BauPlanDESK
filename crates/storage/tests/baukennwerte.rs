//! Rohbauwert/Gebühr, Baustatistik, Sachstandsbericht und tägliche E-Mail.
use core_lib::{
    CoreServices,
    benutzer::{NeuerBenutzer, Rolle},
    domain::{Bundesland, NewProject},
    kalenderabo::Aboumfang,
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    tagesmail::Tagesmaileinstellung,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn kennwerte_bericht_und_tagesmail() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Anna Planung").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "EFH Muster".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();

    // Ohne Berechnung: BRI offen, keine Gebühr.
    let s = core.gebuehrenschaetzung(p.id, &u).await.unwrap();
    assert!(!s.geprueft);
    assert_eq!(s.gebuehr_euro, None);
    assert!(s.offen.iter().any(|o| o.contains("Brutto-Rauminhalt")));

    // Eingabe überschreibt; Objektkennung muss die Projekt-ID sein.
    let k = serde_json::json!({
        "gebaeudeart": "wohngebaeude", "bri_m3": 800.0, "bautaetigkeit": "neubau",
        "wohnungen": 1, "heizung": "Wärmepumpe", "kosten_bauwerk_euro": 380000.0
    });
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Baukennwerte,
            Uuid::new_v4(),
            0,
            k.clone(),
            &u
        )
        .await
        .is_err()
    );
    core.fachobjekt_speichern(p.id, ObjektArt::Baukennwerte, p.id.0, 0, k, &u)
        .await
        .unwrap();
    let s = core.gebuehrenschaetzung(p.id, &u).await.unwrap();
    assert_eq!(s.bri_m3.quelle, "Eingabe im Projekt");
    assert!(s.rohbauwert_euro.unwrap() > 0.);
    assert!(s.gebuehr_euro.unwrap() > 0.);

    let b = core.baustatistik(p.id, &u).await.unwrap();
    let wert = |n: &str| {
        b.merkmale
            .iter()
            .find(|m| m.bezeichnung == n)
            .unwrap()
            .wert
            .clone()
    };
    assert_eq!(wert("Brutto-Rauminhalt"), "800 m³");
    assert_eq!(wert("Anzahl Wohnungen"), "1");
    assert_eq!(
        wert("Veranschlagte Kosten des Bauwerks (KG 300 + 400)"),
        "380.000 €"
    );
    assert!(b.offen.iter().any(|o| o == "Wohnfläche"));
    for statistik in [false, true] {
        let pdf = core.baukennwerte_pdf(p.id, statistik, &u).await.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    // Sachstandsbericht: Verfahren und offene Nachforderung, keine Aufgaben.
    let heute = core_lib::kalender::heute();
    let frist = heute + chrono::Duration::days(3);
    let vid = Uuid::new_v4();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid,
        0,
        serde_json::json!({
            "bezeichnung": "Bauantrag EFH", "art": "bauantrag", "status": "vorbereitung",
            "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht Musterstadt", "kennung": "", "kontakt": ""},
            "aktenzeichen": "63/2026/14", "verantwortlich": u.id.0, "vorgaenger": null, "profil": null,
            "kommunikation": [], "nachforderungen": [], "anforderungen": [],
            "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
        }),
        &u,
    )
    .await
    .unwrap();
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Aufgaben,
        Uuid::new_v4(),
        0,
        serde_json::json!({"titel": "Interne Notiz Honorar", "beschreibung": "", "faellig": frist,
            "status": "offen", "verantwortlich": u.id.0}),
        &u,
    )
    .await
    .unwrap();
    let brief = core.sachstandsbericht(p.id, &u).await.unwrap();
    let text = serde_json::to_string(&brief).unwrap();
    assert!(text.contains("Bauantrag EFH"));
    assert!(text.contains("63/2026/14"));
    assert!(
        !text.contains("Interne Notiz Honorar"),
        "Aufgaben sind bürointern"
    );
    assert!(
        core.sachstandsbericht_pdf(p.id, &u)
            .await
            .unwrap()
            .starts_with(b"%PDF")
    );

    // Tagesmail: Anna ist abwesend, Bernd vertritt.
    let bernd = core
        .benutzer_anlegen(
            NeuerBenutzer {
                anmeldename: "bernd".into(),
                anzeigename: "Bernd Bau".into(),
                rolle: Rolle::Projektleitung,
            },
            &u,
        )
        .await
        .unwrap();
    assert!(
        core.tagesmail_setzen(
            &u,
            Tagesmaileinstellung {
                aktiv: true,
                adresse: "keine-adresse".into(),
                umfang: Aboumfang::Meine,
                abwesend_bis: None,
                vertretung: None,
            }
        )
        .await
        .is_err()
    );
    let anna = Tagesmaileinstellung {
        aktiv: true,
        adresse: "anna@buero.de".into(),
        umfang: Aboumfang::Meine,
        abwesend_bis: None,
        vertretung: Some(bernd.id),
    };
    core.tagesmail_setzen(&u, anna.clone()).await.unwrap();
    core.tagesmail_setzen(
        &bernd,
        Tagesmaileinstellung {
            adresse: "bernd@buero.de".into(),
            vertretung: None,
            ..anna.clone()
        },
    )
    .await
    .unwrap();
    let m = core.tagesmails(heute).await.unwrap();
    assert_eq!(m.len(), 1, "nur Anna hat fällige Termine");
    assert_eq!(m[0].adresse, "anna@buero.de");
    assert!(m[0].text.contains("Interne Notiz Honorar"));

    core.tagesmail_setzen(
        &u,
        Tagesmaileinstellung {
            abwesend_bis: Some(heute + chrono::Duration::days(5)),
            ..anna
        },
    )
    .await
    .unwrap();
    let m = core.tagesmails(heute).await.unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].adresse, "bernd@buero.de");
    assert!(m[0].text.contains("(Vertretung für Anna Planung)"));

    // Versand je Tag nur einmal.
    assert!(core.tagesmail_beanspruchen(heute).await.unwrap());
    assert!(!core.tagesmail_beanspruchen(heute).await.unwrap());
}
