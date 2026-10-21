//! Portalabgabe → Originaldateien → Beleg → Verfahren, mit echten Repositorys.
use bytes::Bytes;
use core_lib::{
    CoreError, CoreServices,
    benutzer::Benutzer,
    commit::Fachstand,
    domain::{Bundesland, NewProject, ProjectId},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
    portale::*,
    verfahren::{Verfahrensakte, abgabe::*},
};
use serde_json::json;
use std::{
    io::{Cursor, Read},
    sync::Arc,
};
use uuid::Uuid;
fn akte() -> Verfahrensakte {
    serde_json::from_value(json!({"bezeichnung":"Bauantrag Musterstraße","art":"bauantrag","status":"vorbereitung","behoerde":{"id":Uuid::new_v4(),"name":"Bauaufsicht Musterstadt","kennung":"bab:test","kontakt":""},"aktenzeichen":"BA-2026-42","verantwortlich":null,"vorgaenger":null,"profil":null,"kommunikation":[],"nachforderungen":[],"anforderungen":[],"dokumentbeziehungen":[],"abgaben":[],"bescheide":[],"ereignisse":[]})).unwrap()
}
fn daten(f: &Fachstand) -> Verfahrensakte {
    serde_json::from_value(f.daten.clone()).unwrap()
}
fn vorgangswunsch(id: Uuid) -> Portalvorgangswunsch {
    Portalvorgangswunsch {
        id,
        profil_id: "nrw-manuell".into(),
        profil_version: "1".into(),
        ziel: Portalziel {
            kommune: "Musterstadt".into(),
            stelle: "Bauaufsicht Musterstadt".into(),
            behoerdenkennung: "bab:test".into(),
            url: "https://bauportal.nrw/".into(),
            zuordnung_bestaetigt: true,
        },
    }
}
fn abgabewunsch(id: Uuid, docs: Vec<Dokumentbezug>) -> Abgabewunsch {
    Abgabewunsch {
        id,
        xbau_bezug: None,
        xbau_folge: None,
        titel: "Bauantragsunterlagen".into(),
        dokumente: docs,
        xbau: false,
        druck: false,
        anschreiben: String::new(),
    }
}
fn portalwunsch(a: &Verfahrensakte, id: Uuid) -> Portalabgabewunsch {
    let b = a.abgaben.iter().find(|b| b.id == id).unwrap();
    Portalabgabewunsch {
        abgabe: id,
        art: Portalabgabeart::Erstantrag,
        vorgaenger: None,
        dateien: b
            .dokumente
            .iter()
            .enumerate()
            .map(|(i, d)| DateiZuordnung {
                dokument: d.dokument.clone(),
                unterlagenart: None,
                uploadname: format!("{:03}-{}", i + 1, d.name),
                ersetzt: None,
            })
            .collect(),
        forderungen: vec![],
    }
}
async fn speicher(
    c: &CoreServices,
    p: ProjectId,
    name: &str,
    inhalt: &'static [u8],
    u: &Benutzer,
) -> Dokumentbezug {
    let d = c
        .store_document(p, None, None, name, Bytes::from_static(inhalt), u)
        .await
        .unwrap();
    Dokumentbezug {
        dokument_id: d.document_id,
        version: d.version,
    }
}
fn ereignis(id: Uuid, beleg: Dokumentbezug, docs: Vec<Dokumentbezug>) -> Portalereigniswunsch {
    Portalereigniswunsch {
        id,
        art: Portalereignisart::AbgabeBelegt,
        // Ein Versand liegt nicht vor dem Festhalten des Abgabestands.
        datum: core_lib::kalender::heute(),
        referenz: "PORTAL-2026-42".into(),
        notiz: "Originalbeleg und konkrete Dateiliste abgeglichen".into(),
        beleg: Some(beleg),
        dateiabgleich: Dateiabgleich::Vollstaendig,
        bestaetigte_dateien: docs,
        berichtigt: None,
    }
}
#[tokio::test]
async fn durchgaengiger_portalablauf_unveraenderliche_originale_und_atomare_belege() {
    let temp = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(temp.path().join("DMS"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = c.anmelden("admin", "Planungsbüro Muster").await.unwrap();
    let leser = c.anmelden("leser", "Leser").await.unwrap();
    let make = || NewProject {
        name: "Portalprojekt Musterstraße".into(),
        bundesland: Bundesland::Nw,
        grundstuecksflaeche_m2: None,
        planungsrecht: Default::default(),
    };
    let p = c.create_project(make(), &u).await.unwrap();
    let fremd = c.create_project(make(), &u).await.unwrap();
    let v = Uuid::new_v4();
    let g = Uuid::new_v4();
    let b1 = Uuid::new_v4();
    let d1 = speicher(&c, p.id, "Lageplan.pdf", b"Unveraenderter Lageplan V1", &u).await;
    c.dokumentkategorie_speichern(
        d1.dokument_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 0,
            kategorie: Some("lageplan".into()),
        },
        &u,
    )
    .await
    .unwrap();
    let f = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            v,
            0,
            serde_json::to_value(akte()).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let ohne_portal = c
        .verfahren_abgabe_festhalten(p.id, v, f.revision, abgabewunsch(b1, vec![d1.clone()]), &u)
        .await
        .unwrap();
    assert_eq!(
        daten(&ohne_portal).abgaben[0].dokumente[0]
            .kategorie
            .as_ref()
            .unwrap()
            .name,
        "Lageplan"
    );
    let mut falsch = vorgangswunsch(g);
    falsch.ziel.zuordnung_bestaetigt = false;
    assert!(
        c.portal_vorgang_anlegen(p.id, v, ohne_portal.revision, falsch, &u)
            .await
            .is_err()
    );
    let mit_vorgang = c
        .portal_vorgang_anlegen(p.id, v, ohne_portal.revision, vorgangswunsch(g), &u)
        .await
        .unwrap();
    assert_eq!(
        c.portal_vorgang_anlegen(p.id, v, 0, vorgangswunsch(g), &u)
            .await
            .unwrap()
            .revision,
        mit_vorgang.revision
    );
    assert!(matches!(
        c.portal_url(p.id, v, g, &leser).await,
        Err(CoreError::Forbidden(_))
    ));
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
    assert_eq!(
        c.portal_url(p.id, v, g, &leser).await.unwrap(),
        "https://bauportal.nrw/"
    );
    assert!(c.portal_url(fremd.id, v, g, &u).await.is_err());
    let w = portalwunsch(&daten(&mit_vorgang), b1);
    let pruefung = c
        .portal_abgabe_pruefen(p.id, v, g, &w, &leser)
        .await
        .unwrap();
    assert!(!pruefung.pruefung.profil_fachlich_geprueft);
    assert!(pruefung.pruefung.fehler.is_empty());
    assert!(!pruefung.pruefung.hinweise.is_empty());
    assert!(
        c.portal_abgabe_festhalten(p.id, v, g, mit_vorgang.revision, w.clone(), &leser)
            .await
            .is_err()
    );
    let bereit = c
        .portal_abgabe_festhalten(p.id, v, g, mit_vorgang.revision, w.clone(), &u)
        .await
        .unwrap();
    assert_eq!(
        c.portal_abgabe_festhalten(p.id, v, g, 0, w.clone(), &u)
            .await
            .unwrap()
            .revision,
        bereit.revision
    );
    assert!(daten(&bereit).abgaben[0].versand.is_empty());
    let (_, bytes1) = c.portal_bereitstellung(p.id, v, g, b1, &u).await.unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes1)).unwrap();
    let mut plan = vec![];
    zip.by_name("Unterlagen/001-Lageplan.pdf")
        .unwrap()
        .read_to_end(&mut plan)
        .unwrap();
    assert_eq!(plan, b"Unveraenderter Lageplan V1");
    assert!(zip.by_name("Manifest.json").is_ok());
    assert!(zip.by_name("Dateiliste.csv").is_ok());
    let receipt = speicher(
        &c,
        p.id,
        "Portalbeleg.pdf",
        b"Portalbeleg mit Referenz und Dateiliste",
        &u,
    )
    .await;
    let mut ungeprueft = ereignis(Uuid::new_v4(), receipt.clone(), vec![]);
    ungeprueft.dateiabgleich = Dateiabgleich::Ungeklaert;
    let ungeklart = c
        .portal_ereignis_erfassen(p.id, v, g, b1, bereit.revision, ungeprueft.clone(), &u)
        .await
        .unwrap();
    assert_eq!(
        daten(&ungeklart).abgaben[0].versandstatus,
        Some(Versandstatus::Ungeklaert)
    );
    assert!(!dateistand_belegt(
        &daten(&ungeklart).portal.unwrap().vorgaenge[0].abgaben[0]
    ));
    assert_eq!(
        c.portal_ereignis_erfassen(p.id, v, g, b1, 0, ungeprueft.clone(), &u)
            .await
            .unwrap()
            .revision,
        ungeklart.revision
    );
    let mut voll = ereignis(Uuid::new_v4(), receipt.clone(), vec![d1.clone()]);
    voll.berichtigt = Some(ungeprueft.id);
    let belegt = c
        .portal_ereignis_erfassen(p.id, v, g, b1, ungeklart.revision, voll.clone(), &u)
        .await
        .unwrap();
    assert_eq!(
        daten(&belegt).abgaben[0].versandstatus,
        Some(Versandstatus::Versendet)
    );
    assert!(dateistand_belegt(
        &daten(&belegt).portal.unwrap().vorgaenge[0].abgaben[0]
    ));
    let mut erneut = voll.clone();
    erneut.id = Uuid::new_v4();
    assert!(
        c.portal_ereignis_erfassen(p.id, v, g, b1, belegt.revision, erneut, &u)
            .await
            .is_err()
    );
    // Neue Live-Dateien und Kategorien verändern weder das alte Paket noch dessen Originale.
    let d2 = c
        .store_document(
            p.id,
            Some(d1.dokument_id),
            Some(1),
            "Lageplan.pdf",
            Bytes::from_static(b"Lageplan korrigiert V2"),
            &u,
        )
        .await
        .unwrap();
    let d2 = Dokumentbezug {
        dokument_id: d2.document_id,
        version: 2,
    };
    c.dokumentkategorie_speichern(
        d1.dokument_id,
        core_lib::dokumentkategorien::KategorieZuordnung {
            revision: 1,
            kategorie: Some("grundriss".into()),
        },
        &u,
    )
    .await
    .unwrap();
    assert_eq!(
        bytes1,
        c.portal_bereitstellung(p.id, v, g, b1, &u).await.unwrap().1
    );
    assert_eq!(
        c.portal_datei(p.id, v, g, b1, &d1, &u).await.unwrap().1,
        b"Unveraenderter Lageplan V1"
    );
    let extra = speicher(&c, p.id, "Wohnflaeche.pdf", b"Wohnflaechennachweis", &u).await;
    let b2 = Uuid::new_v4();
    let f = c
        .verfahren_abgabe_festhalten(
            p.id,
            v,
            belegt.revision,
            abgabewunsch(b2, vec![d2.clone(), extra.clone()]),
            &u,
        )
        .await
        .unwrap();
    let mut w2 = portalwunsch(&daten(&f), b2);
    w2.art = Portalabgabeart::Nachreichung;
    w2.vorgaenger = Some(b1);
    let vergleich = c.portal_abgabe_pruefen(p.id, v, g, &w2, &u).await.unwrap();
    assert_eq!(vergleich.dateien[0].vergleich, Dateivergleich::Geaendert);
    assert_eq!(vergleich.dateien[1].vergleich, Dateivergleich::Neu);
    let nachreichung = c
        .portal_abgabe_festhalten(p.id, v, g, f.revision, w2.clone(), &u)
        .await
        .unwrap();
    let mut e1 = ereignis(
        Uuid::new_v4(),
        receipt.clone(),
        vec![d2.clone(), extra.clone()],
    );
    e1.art = Portalereignisart::EingangBelegt;
    let mut e2 = e1.clone();
    e2.id = Uuid::new_v4();
    e2.referenz = "zweiter Versuch".into();
    let (r1, r2) = tokio::join!(
        c.portal_ereignis_erfassen(p.id, v, g, b2, nachreichung.revision, e1, &u),
        c.portal_ereignis_erfassen(p.id, v, g, b2, nachreichung.revision, e2, &u)
    );
    assert_ne!(r1.is_ok(), r2.is_ok());
    let f = r1.or(r2).unwrap();
    let a = daten(&f);
    assert_eq!(a.abgaben[1].versand.len(), 1);
    assert_eq!(
        a.portal.as_ref().unwrap().vorgaenge[0].abgaben[1]
            .ereignisse
            .len(),
        1
    );
    // Eine reine Nachreichung darf frühere, nicht erneut beigefügte Pläne nicht vergessen.
    let b3 = Uuid::new_v4();
    let f = c
        .verfahren_abgabe_festhalten(
            p.id,
            v,
            f.revision,
            abgabewunsch(b3, vec![extra.clone()]),
            &u,
        )
        .await
        .unwrap();
    let mut w3 = portalwunsch(&daten(&f), b3);
    w3.art = Portalabgabeart::Nachreichung;
    w3.vorgaenger = Some(b2);
    let f = c
        .portal_abgabe_festhalten(p.id, v, g, f.revision, w3, &u)
        .await
        .unwrap();
    let f = c
        .portal_ereignis_erfassen(
            p.id,
            v,
            g,
            b3,
            f.revision,
            ereignis(Uuid::new_v4(), receipt.clone(), vec![extra.clone()]),
            &u,
        )
        .await
        .unwrap();
    let b4 = Uuid::new_v4();
    let f = c
        .verfahren_abgabe_festhalten(p.id, v, f.revision, abgabewunsch(b4, vec![d2.clone()]), &u)
        .await
        .unwrap();
    let mut w4 = portalwunsch(&daten(&f), b4);
    w4.art = Portalabgabeart::Nachreichung;
    w4.vorgaenger = Some(b3);
    let r = c.portal_abgabe_pruefen(p.id, v, g, &w4, &u).await.unwrap();
    assert_eq!(r.dateien[0].vergleich, Dateivergleich::Identisch);
    assert_eq!(r.referenzdateien.len(), 2);
    let original = speicher(
        &c,
        p.id,
        "Nachforderung.pdf",
        b"Nachforderung Original ohne automatische Frist",
        &u,
    )
    .await;
    let rw = Portalrueckmeldungswunsch {
        id: Uuid::new_v4(),
        betreff: "Nachforderung: Wohnflächen ergänzen".into(),
        datum: "2026-01-08".parse().unwrap(),
        referenz: "BA-2026-42/N1".into(),
        original: original.clone(),
        inhalt: "Die Behörde fordert eine Erläuterung. Frist manuell am Original prüfen.".into(),
        abgabe: Some(b2),
        xbau_nachricht: None,
    };
    let rueckmeldung = c
        .portal_rueckmeldung_erfassen(p.id, v, g, f.revision, rw.clone(), &u)
        .await
        .unwrap();
    assert_eq!(
        c.portal_rueckmeldung_erfassen(p.id, v, g, 0, rw, &u)
            .await
            .unwrap()
            .revision,
        rueckmeldung.revision
    );
    let a = daten(&rueckmeldung);
    assert_eq!(a.kommunikation.len(), 1);
    assert!(a.kommunikation[0].eingehend);
    assert_eq!(a.kommunikation[0].original, Some(original));
    assert!(a.nachforderungen.is_empty());
    assert!(a.bescheide.is_empty());
    // Rohes JSON darf die Portalhistorie oder deren Versandprojektion nicht ändern.
    let mut manipuliert = a.clone();
    manipuliert.portal = None;
    assert!(
        c.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            v,
            rueckmeldung.revision,
            serde_json::to_value(manipuliert).unwrap(),
            &u
        )
        .await
        .is_err()
    );
    let mut manipuliert = a.clone();
    let mut fake = manipuliert.abgaben[0].versand[0].clone();
    fake.id = Uuid::new_v4();
    fake.berichtigt = None;
    manipuliert.abgaben[0].versand.push(fake);
    assert!(
        c.fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            v,
            rueckmeldung.revision,
            serde_json::to_value(manipuliert).unwrap(),
            &u
        )
        .await
        .is_err()
    );
    let mut korrektur = ereignis(Uuid::new_v4(), receipt.clone(), vec![]);
    korrektur.art = Portalereignisart::AusgangUnklar;
    korrektur.dateiabgleich = Dateiabgleich::Ungeklaert;
    korrektur.berichtigt = Some(voll.id);
    let f = c
        .portal_ereignis_erfassen(p.id, v, g, b1, rueckmeldung.revision, korrektur, &u)
        .await
        .unwrap();
    assert_eq!(
        daten(&f).abgaben[0].versandstatus,
        Some(Versandstatus::Ungeklaert)
    );
    assert!(
        c.projektuebersicht(p.id, &u)
            .await
            .unwrap()
            .punkte
            .iter()
            .any(|p| p.ziel.bereich == "portale")
    );
    let archiv = c.verfahren_aktenexport(p.id).await.unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(archiv)).unwrap();
    let mut text = String::new();
    z.by_name("Projektakte.json")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.contains("portalbereitstellung"));
    if let Ok(out) = std::env::var("BAUPROJEKT_PORTAL_ARTEFAKTE") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("workflow.json"),serde_json::to_vec_pretty(&json!({"projekt":p,"benutzer":u,"leser":leser,"verfahren":v,"vorgang":g,"abgabe":b1,"ohne_portal":ohne_portal,"mit_vorgang":mit_vorgang,"bereit":bereit,"ungeklaert":ungeklart,"belegt":belegt,"nachreichung":nachreichung,"rueckmeldung":rueckmeldung,"final":f,"vorbereitung":pruefung,"vergleich":vergleich,"profile":c.portalprofile().await.unwrap(),"dateien":c.list_documents(p.id).await.unwrap(),"beleg":receipt})).unwrap()).unwrap();
        std::fs::write(out.join("Portalunterlagen.zip"), bytes1).unwrap();
    }
}
#[tokio::test]
async fn profile_und_bereitstellung_pruefen_echte_regeln_und_keine_erfundene_nrw_freigabe() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let c = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("DMS"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = c.anmelden("admin", "Admin").await.unwrap();
    let leser = c.anmelden("leser", "Leser").await.unwrap();
    let profile = c.portalprofile().await.unwrap();
    assert!(
        profile
            .iter()
            .all(|p| p.freigabe != Profilfreigabe::FachlichGeprueft)
    );
    let mut profil = profile[0].clone();
    profil.id = "buero-test".into();
    profil.freigabe = Profilfreigabe::FachlichGeprueft;
    profil.quelle = "Fiktive Regeln nur für diesen Test".into();
    profil.erlaubte_endungen = vec!["pdf".into()];
    profil.max_datei_bytes = Some(5);
    profil.unterlagenarten = vec![PortalUnterlagenart {
        code: "LAGE".into(),
        name: "Lageplan".into(),
        erforderlich: true,
        kategorien: vec!["lageplan".into()],
    }];
    c.portalprofil_hinzufuegen(profil.clone(), &u)
        .await
        .unwrap();
    assert!(
        c.portalprofil_hinzufuegen(profil.clone(), &leser)
            .await
            .is_err()
    );
    let mut alt = profil.clone();
    alt.titel = "Geändert".into();
    assert!(c.portalprofil_hinzufuegen(alt, &u).await.is_err());
    let p = c
        .create_project(
            NewProject {
                name: "Profiltest".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap();
    let v = Uuid::new_v4();
    let g = Uuid::new_v4();
    let b = Uuid::new_v4();
    let d = speicher(&c, p.id, "Plan.dwg", b"0123456789", &u).await;
    let f = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Verfahren,
            v,
            0,
            serde_json::to_value(akte()).unwrap(),
            &u,
        )
        .await
        .unwrap();
    let f = c
        .verfahren_abgabe_festhalten(p.id, v, f.revision, abgabewunsch(b, vec![d]), &u)
        .await
        .unwrap();
    let mut vw = vorgangswunsch(g);
    vw.profil_id = profil.id;
    let f = c
        .portal_vorgang_anlegen(p.id, v, f.revision, vw, &u)
        .await
        .unwrap();
    let w = portalwunsch(&daten(&f), b);
    let r = c.portal_abgabe_pruefen(p.id, v, g, &w, &u).await.unwrap();
    assert!(r.pruefung.profil_fachlich_geprueft);
    assert!(r.pruefung.fehler.len() >= 3);
    assert!(
        c.portal_abgabe_festhalten(p.id, v, g, f.revision, w, &u)
            .await
            .is_err()
    );
    for s in [
        "http://bauportal.nrw/",
        "https://user:secret@bauportal.nrw/",
        "https://127.0.0.1/",
        "https://localhost/",
        "https://portal.local/",
        "https://bauportal.nrw:444/",
        "https://bauportal.nrw/?token=123",
        "file:///tmp/portal",
        "javascript:alert(1)",
    ] {
        assert!(browser_url(s).is_err(), "{s}");
    }
    assert_eq!(
        browser_url("https://bauportal.nrw/").unwrap(),
        "https://bauportal.nrw/"
    );
    let palt: Verfahrensakte =
        serde_json::from_value(serde_json::to_value(akte()).unwrap()).unwrap();
    assert!(palt.portal.is_none());
    assert!(serde_json::to_value(palt).unwrap().get("portal").is_none());
}
