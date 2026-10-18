//! Öffentliche Verträge: bestehende Module → Projektübersicht → gezielter Arbeitsbereich.
use chrono::NaiveDate;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, nachweise::NachweisEingabe},
    projektuebersicht::{Dringlichkeit, Themenbereich},
    verfahren::*,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
#[tokio::test]
async fn arbeitsstand_fristen_quellen_und_historische_nachweise() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let leser = c.anmelden("leser", "Lesender Benutzer").await.unwrap();
    let p = c
        .create_project(
            NewProject {
                name: "Neubau Lindenweg · Projektübersicht".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(500.),
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let tag = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
    let vorher = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert!(vorher.punkte.iter().any(|p| p.id == "anforderungen"));
    assert!(vorher.punkte.iter().any(|p| p.id == "recht"));
    assert_eq!(vorher.zahlen.offene_aufgaben, 0);
    assert!(!vorher.hinweise.is_empty());
    c.projektmitglied_setzen(
        p.id,
        core_lib::zugriff::ProjektMitglied {
            benutzer_id: leser.id,
            recht: core_lib::zugriff::ProjektRecht::Lesen,
        },
        0,
        &u,
    )
    .await
    .unwrap();
    assert!(c.projektuebersicht(p.id, &leser).await.is_ok());
    let fremd = c
        .create_project(
            NewProject {
                name: "Fremde Akte".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    assert!(c.projektuebersicht(fremd.id, &leser).await.is_err());
    let mut aufgaben = vec![];
    for (titel, datum, status) in [
        ("Lageplan beim Vermesser anfordern", "2026-10-15", "offen"),
        ("Bauherrnfreigabe einholen", "2026-10-16", "in_arbeit"),
        ("Bereits erledigte Rückfrage", "2026-10-10", "erledigt"),
    ] {
        aufgaben.push(c.fachobjekt_speichern(p.id,ObjektArt::Aufgaben,Uuid::new_v4(),0,json!({"titel":titel,"beschreibung":"Fiktiver Arbeitsschritt für den Bauantrag","faellig":datum,"verantwortlich":u.id,"status":status,"bezug":null}),&u).await.unwrap());
    }
    let uid = Uuid::new_v4();
    let mut unterlage=c.fachobjekt_speichern(p.id,ObjektArt::Unterlagen,uid,0,json!({"bezeichnung":"Lageplan","erforderlich":true,"dokument":null,"status":"entwurf","bemerkung":"Vermessungsgrundlage","vorlage":null}),&u).await.unwrap();
    let a = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert_eq!(a.zahlen.offene_aufgaben, 2);
    assert_eq!(a.zahlen.ueberfaellig, 1);
    assert_eq!(a.zahlen.fehlende_unterlagen, 1);
    assert_eq!(a.punkte[0].dringlichkeit, Dringlichkeit::Ueberfaellig);
    assert_eq!(a.punkte[0].titel, "Lageplan beim Vermesser anfordern");
    assert!(
        a.punkte
            .iter()
            .any(|p| p.dringlichkeit == Dringlichkeit::Heute)
    );
    assert!(
        !a.punkte
            .iter()
            .any(|p| p.titel == "Bereits erledigte Rückfrage")
    );
    let d1 = c
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.pdf",
            bytes::Bytes::from_static(b"Fiktiver Plan 1"),
            &u,
        )
        .await
        .unwrap();
    let mut daten = unterlage.daten.clone();
    daten["dokument"] = json!({"dokument_id":d1.document_id,"version":1});
    daten["status"] = json!("freigegeben");
    unterlage = c
        .fachobjekt_speichern(p.id, ObjektArt::Unterlagen, uid, 1, daten, &u)
        .await
        .unwrap();
    assert_eq!(
        c.projektuebersicht_am(p.id, tag, &u)
            .await
            .unwrap()
            .zahlen
            .fehlende_unterlagen,
        0
    );
    let d2 = c
        .store_document(
            p.id,
            Some(d1.document_id),
            Some(d1.version),
            "Lageplan.pdf",
            bytes::Bytes::from_static(b"Fiktiver Plan 2"),
            &u,
        )
        .await
        .unwrap();
    assert_eq!(
        c.projektuebersicht_am(p.id, tag, &u)
            .await
            .unwrap()
            .zahlen
            .fehlende_unterlagen,
        1
    );
    let vid = Uuid::new_v4();
    let kommunikation = Uuid::new_v4();
    let punkt = Uuid::new_v4();
    let mut akte = Verfahrensakte {
        portal: None,
        bezeichnung: "Bauantrag Wohnhaus".into(),
        art: Verfahrensart::Bauantrag,
        antragsangaben: Default::default(),
        status: Verfahrensstatus::Vorbereitung,
        vollstaendigkeit: None,
        ruecknahme: None,
        schritte: Vec::new(),
        behoerde: Behoerde {
            id: Uuid::new_v4(),
            name: "Bauaufsicht Musterstadt".into(),
            kennung: "musterstadt".into(),
            kontakt: "Bauantragsstelle".into(),
        },
        aktenzeichen: "BA-2026-124".into(),
        verantwortlich: Some(u.id.0),
        vorgaenger: None,
        profil: None,
        kommunikation: vec![Kommunikation {
            id: kommunikation,
            art: Kommunikationsart::Brief,
            eingehend: true,
            datum: tag,
            betreff: "Fehlende Unterlagen zum Bauantrag".into(),
            beteiligte: "Bauaufsicht".into(),
            inhalt: "Bitte Flächenberechnung nachreichen.".into(),
            original: None,
            bezuege: vec![],
        }],
        nachforderungen: vec![Nachforderung {
            id: Uuid::new_v4(),
            kommunikation,
            punkte: vec![Forderungspunkt {
                id: punkt,
                titel: "Wohnflächenberechnung ergänzen".into(),
                grundlage: "Behördliches Schreiben, Punkt 1".into(),
                verantwortlich: Some(u.id.0),
                frist: Some(Frist {
                    datum: tag,
                    ausloeser: "Schreiben".into(),
                    beleg: None,
                    erinnerung_tage: 7,
                    grund: "Fiktiver Kontrollfall".into(),
                }),
                status: Punktstatus::Offen,
                antwort: String::new(),
                dokumente: vec![],
                abgaben: vec![],
                bestaetigung: None,
            }],
        }],
        anforderungen: vec![Anforderung {
            id: Uuid::new_v4(),
            titel: "Stellplatznachweis".into(),
            erforderlich: Erforderlichkeit::Ungeklaert,
            grund: "Kommunale Grundlage prüfen".into(),
            quelle: "Örtliche Satzung".into(),
            dokumente: vec![],
            freigegeben: false,
            verantwortlich: Some(u.id.0),
        }],
        dokumentbeziehungen: vec![],
        abgaben: vec![],
        externe_einreichungen: vec![],
        bescheide: vec![],
        ereignisse: vec![],
    };
    let af = c
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
    let z = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert_eq!(z.zahlen.offene_nachforderungen, 1);
    let n = z
        .punkte
        .iter()
        .find(|p| p.ziel.eintrag == Some(punkt))
        .unwrap();
    assert_eq!(n.ziel.objekt, Some(vid));
    assert_eq!(n.ziel.register.as_deref(), Some("nachforderungen"));
    assert_eq!(n.dringlichkeit, Dringlichkeit::Heute);
    // Zwei archivierte Stände desselben Produzenten: alter Stand darf nicht erneut Arbeit erzeugen.
    let mut wohn = core_lib::berechnungen::vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == core_lib::berechnungen::BerechnungsArt::Wohnflaechen)
        .unwrap()
        .plan;
    wohn.grundstueck_id = Some(core_lib::grundstueckskontext::ermitteln(&p).unwrap()[0].id);
    wohn.regelkontext.quelle = "WoFlV · Fiktiver Kontrollansatz".into();
    wohn.regelkontext.pruefvermerk = "Geprüftes Testaufmaß".into();
    wohn.regelkontext.anwendbarkeit_bestaetigt = true;
    wohn.eingaben["aufmass_vollstaendig"] = json!(true);
    wohn.eingaben["positionen"][0]["aufmass_m2"] = json!(100);
    wohn.eingaben["positionen"][0]["lichte_hoehe_m"] = json!(2.5);
    wohn.eingaben["positionen"][0]["quelle"] = json!("Aufmaß");
    let oid = Uuid::new_v4();
    let mut wf = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Wohnflaechen,
            oid,
            0,
            serde_json::to_value(wohn).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let mut letzte = None;
    for flaeche in [100, 120] {
        if flaeche == 120 {
            let mut d = wf.daten.clone();
            d["eingaben"]["positionen"][0]["aufmass_m2"] = json!(120);
            wf = c
                .fachobjekt_speichern(p.id, ObjektArt::Wohnflaechen, oid, wf.revision, d, &u)
                .await
                .unwrap();
            let mut d = wf.daten.clone();
            d["regelkontext"]["anwendbarkeit_bestaetigt"] = json!(true);
            wf = c
                .fachobjekt_speichern(p.id, ObjektArt::Wohnflaechen, oid, wf.revision, d, &u)
                .await
                .unwrap();
        }
        let plan = serde_json::from_value(wf.daten.clone()).unwrap();
        let n = c
            .nachweis_erstellen(
                p.id,
                Uuid::new_v4(),
                "Wohnfläche Haus A".into(),
                NachweisEingabe::Fach {
                    objekt: oid,
                    plan: Box::new(plan),
                },
                &u,
            )
            .await
            .unwrap();
        letzte = Some(n);
    }
    let nachweis = letzte.unwrap();
    c.nachweis_freigeben(p.id, nachweis.id, "Kontrollfall geprüft".into(), &u)
        .await
        .unwrap();
    let z = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert_eq!(
        z.zahlen.nachweise_pruefen, 0,
        "Historischer Stand darf den aktuellen vollständigen freigegebenen Stand nicht überlagern"
    );
    assert_eq!(c.nachweise(p.id).await.unwrap().len(), 2);
    let paket = c
        .einreichung_erstellen(
            p.id,
            core_lib::einreichung::NeuesPaket {
                titel: "Bauantrag · Erstabgabe".into(),
                empfaenger: "Bauamt".into(),
                dokumente: vec![core_lib::einreichung::DokumentAuswahl {
                    dokument_id: d1.document_id,
                    version: 1,
                }],
            },
            &u,
        )
        .await
        .unwrap();
    let nur_export = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert!(
        nur_export
            .abgaben
            .iter()
            .any(|a| a.id == paket.stand.id && a.status.contains("Nur exportiert"))
    );
    assert!(nur_export.empfaenger.is_empty());
    c.einreichung_bestaetigen_altbestand(
        paket.stand.id,
        paket.version,
        tag,
        "Eingereicht am Kontrolltag".into(),
        &u,
    )
    .await
    .unwrap();
    let eingereicht = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert!(eingereicht.empfaenger.iter().any(|e| {
        e.dokumente
            .iter()
            .any(|d| d.eingereichte_version == 1 && d.aktuelle_version == Some(2))
    }));
    assert!(
        eingereicht
            .punkte
            .iter()
            .any(|p| p.id.starts_with("paketversion:"))
    );

    let uebersicht = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert!(
        uebersicht
            .punkte
            .iter()
            .any(|p| p.thema == Themenbereich::Unterlagen)
    );
    let mut erledigt = aufgaben[0].daten.clone();
    erledigt["status"] = json!("erledigt");
    c.fachobjekt_speichern(
        p.id,
        ObjektArt::Aufgaben,
        Uuid::parse_str(&aufgaben[0].objekt).unwrap(),
        1,
        erledigt,
        &u,
    )
    .await
    .unwrap();
    let nach_erledigung = c.projektuebersicht_am(p.id, tag, &u).await.unwrap();
    assert_eq!(nach_erledigung.zahlen.ueberfaellig, 0);
    if let Ok(out) = std::env::var("BAUPROJEKT_UEBERSICHT_ARTEFAKTE") {
        let out = std::path::Path::new(&out);
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(out.join("workflow.json"),serde_json::to_vec_pretty(&json!({"projekt":p,"benutzer":u,"leser":leser,"uebersicht":uebersicht,"nach_erledigung":nach_erledigung,"aufgaben":aufgaben,"unterlagen":[unterlage],"verfahren":[af],"dateien":[d2],"nachweise":c.nachweise(p.id).await.unwrap()})).unwrap()).unwrap();
    }
    // Eine abschließend entfallene Nachforderung bleibt historisch erhalten, erzeugt aber keine offene Frist.
    akte = serde_json::from_value(af.daten.clone()).unwrap();
    akte.nachforderungen[0].punkte[0].status = Punktstatus::Entfallen;
    akte.nachforderungen[0].punkte[0].antwort = "Behördlich entfallen".into();
    c.fachobjekt_speichern(
        p.id,
        ObjektArt::Verfahren,
        vid,
        1,
        serde_json::to_value(akte).unwrap(),
        &u,
    )
    .await
    .unwrap();
    assert_eq!(
        c.projektuebersicht_am(p.id, tag, &u)
            .await
            .unwrap()
            .zahlen
            .offene_nachforderungen,
        0
    );
}
