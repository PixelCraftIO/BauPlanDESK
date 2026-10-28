//! IFC-Modelle im DMS: Auszug lesen, Modellakte speichern; Fremddateien und
//! fremde Projekte werden abgewiesen.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    benutzer::Benutzer,
    domain::{Bundesland, NewProject, Project},
    ifc::{self, testdaten},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::{ObjektArt, objekte::Dokumentbezug},
};
use std::sync::Arc;
use uuid::Uuid;

pub struct Umgebung {
    pub core: CoreServices,
    pub user: Benutzer,
    pub projekt: Project,
    _dir: tempfile::TempDir,
}

pub async fn umgebung() -> Umgebung {
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
    let projekt = neues_projekt(&core, &user, "Modell").await;
    Umgebung {
        core,
        user,
        projekt,
        _dir: dir,
    }
}

async fn neues_projekt(core: &CoreServices, user: &Benutzer, name: &str) -> Project {
    core.create_project(
        NewProject {
            name: name.into(),
            bundesland: Bundesland::Nw,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        },
        user,
    )
    .await
    .unwrap()
}

pub async fn ablegen(u: &Umgebung, name: &str, inhalt: String) -> Dokumentbezug {
    let d = u
        .core
        .store_document(u.projekt.id, None, None, name, Bytes::from(inhalt), &u.user)
        .await
        .unwrap();
    Dokumentbezug {
        dokument_id: d.document_id,
        version: d.version,
    }
}

#[tokio::test]
async fn modell_lesen_und_modellakte_speichern() {
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let a = u.core.ifc_auszug(u.projekt.id, &d).await.unwrap();
    assert_eq!(a.geschosse.len(), 2);
    assert_eq!(a.bericht.raeume, 4);
    let id = Uuid::new_v4();
    let akte = ifc::Modellakte {
        bezeichnung: "Entwurf Architekt".into(),
        dokument: d.clone(),
        uebernahme: None,
        nacharbeit: Vec::new(),
    };
    let f = u
        .core
        .fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Ifcmodell,
            id,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u.user,
        )
        .await
        .unwrap();
    assert_eq!(f.modul, "ifcmodell");
    let gelesen: ifc::Modellakte = serde_json::from_value(
        u.core
            .fachobjekt_lesen(u.projekt.id, ObjektArt::Ifcmodell, id)
            .await
            .unwrap()
            .unwrap()
            .daten,
    )
    .unwrap();
    assert_eq!(gelesen, akte);

    // Keine IFC-Datei, kaputte IFC-Datei, Dokument eines anderen Projekts.
    let pdf = ablegen(&u, "Plan.pdf", "%PDF-1.4".into()).await;
    let e = u
        .core
        .ifc_auszug(u.projekt.id, &pdf)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("keine IFC-Datei"), "{e}");
    let kaputt = ablegen(
        &u,
        "kaputt.ifc",
        "ISO-10303-21;\nDATA;\n#1=IFCWALL(;\n".into(),
    )
    .await;
    let akte = ifc::Modellakte {
        bezeichnung: "Kaputt".into(),
        dokument: kaputt,
        uebernahme: None,
        nacharbeit: Vec::new(),
    };
    assert!(
        u.core
            .fachobjekt_speichern(
                u.projekt.id,
                ObjektArt::Ifcmodell,
                Uuid::new_v4(),
                0,
                serde_json::to_value(&akte).unwrap(),
                &u.user,
            )
            .await
            .is_err()
    );
    let anderes = neues_projekt(&u.core, &u.user, "Anderes").await;
    let e = u
        .core
        .ifc_auszug(anderes.id, &d)
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("nicht zu diesem Projekt"), "{e}");
}

/// Ein Geschoss des Modells als Hintergrund einer eigenen Zeichnung: nur mit
/// vorhandenem Geschoss; PDF-Hintergründe tragen kein Geschoss.
#[tokio::test]
async fn ifc_geschoss_als_zeichnungshintergrund() {
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let gs = u.core.ifc_geschosse(u.projekt.id, &d).await.unwrap();
    assert_eq!(gs.len(), 2);
    let plan = u
        .core
        .ifc_grundriss_plan(u.projekt.id, &d, &gs[0].global_id)
        .await
        .unwrap();
    assert_eq!(plan.einheit, Some(core_lib::cad::Laengeneinheit::Meter));
    assert!(
        plan.ebenen.iter().any(|e| e.name == "Wand"),
        "{:?}",
        plan.ebenen
    );
    assert!(plan.ebenen.iter().any(|e| e.name == "Räume"));
    // Grenzen sind der Modellumfang: alle Geschosse liegen deckungsgleich.
    let og = u
        .core
        .ifc_grundriss_plan(u.projekt.id, &d, &gs[1].global_id)
        .await
        .unwrap();
    assert_eq!(plan.grenzen, og.grenzen);

    let zeichnung = |geschoss: Option<&str>, dokument: &Dokumentbezug| {
        serde_json::json!({
            "bezeichnung": "Skizze",
            "seite": 1,
            "kalibrierung": {"art": "einheit", "einheit": "meter"},
            "hoehenbezug": null,
            "ebenen": [],
            "markierungen": [],
            "hintergruende": [{
                "id": Uuid::new_v4(),
                "dokument": dokument,
                "seite": 1,
                "ursprung": [0.0, 0.0],
                "lage": [0.0, 0.0],
                "drehung_grad": 0.0,
                "meter_pro_punkt": 1.0,
                "deckkraft": 0.6,
                "sichtbar": true,
                "geschoss": geschoss,
            }],
        })
    };
    let speichern = |v: serde_json::Value| {
        u.core.fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Planaufmass,
            Uuid::new_v4(),
            0,
            v,
            &u.user,
        )
    };
    speichern(zeichnung(Some(&gs[1].global_id), &d))
        .await
        .unwrap();
    let e = speichern(zeichnung(Some("0000000000000000000000"), &d))
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Geschoss"), "{e}");
    let e = speichern(zeichnung(None, &d))
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("Geschoss"), "{e}");
    let pdf = ablegen(&u, "Plan.pdf", "%PDF-1.4".into()).await;
    assert!(
        speichern(zeichnung(Some(&gs[0].global_id), &pdf))
            .await
            .is_err()
    );
}

/// Übernahme: Gebäude mit Geschossen, je Geschoss eine Zeichnung mit Räumen,
/// Außenkante und (EG) Gebäudegrundfläche, Raumbücher mit lichter Höhe.
/// Wiederholt: gleiche Markierungs-IDs, eigene Markierungen bleiben.
#[tokio::test]
async fn uebernahme_in_gebaeude_zeichnungen_und_raumbuch() {
    use core_lib::planaufmass::{Planaufmass, Planbedeutung};
    use core_lib::plattform::objekte::{Gebaeude, Raumbuch};
    use testdaten::efh;
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let akte_id = Uuid::new_v4();
    let akte = ifc::Modellakte {
        bezeichnung: "Entwurf".into(),
        dokument: d.clone(),
        uebernahme: None,
        nacharbeit: Vec::new(),
    };
    let f = u
        .core
        .fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Ifcmodell,
            akte_id,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u.user,
        )
        .await
        .unwrap();
    let mut wunsch = ifc::Modelluebernahmewunsch {
        akte_id,
        revision: f.revision,
        gebaeude_id: None,
        raumbuch: true,
    };
    let r = u
        .core
        .ifc_uebernehmen(u.projekt.id, &wunsch, &u.user)
        .await
        .unwrap();
    assert!(r.hinweise.is_empty(), "{:?}", r.hinweise);
    let haus: Gebaeude = serde_json::from_value(r.gebaeude.daten.clone()).unwrap();
    assert_eq!(haus.bezeichnung, "Wohnhaus");
    let namen: Vec<_> = haus
        .geschosse
        .iter()
        .map(|g| g.bezeichnung.as_str())
        .collect();
    assert_eq!(namen, ["EG", "OG"]);
    let eg_hoehe = haus.geschosse[0]
        .geometrie
        .as_ref()
        .unwrap()
        .geschosshoehe_m;
    assert_eq!(eg_hoehe, Some(efh::GESCHOSSHOEHE));
    assert_eq!(r.zeichnungen.len(), 2);

    let eg: Planaufmass = serde_json::from_value(r.zeichnungen[0].daten.clone()).unwrap();
    assert_eq!(eg.hintergruende.len(), 1);
    assert_eq!(eg.hintergruende[0].lage, [ifc::RAND_M, ifc::RAND_M]);
    let arten: Vec<_> = eg.markierungen.iter().map(|m| m.bedeutung).collect();
    assert_eq!(
        arten,
        [
            Planbedeutung::Raum,
            Planbedeutung::Raum,
            Planbedeutung::GeschossAussenkante,
            Planbedeutung::Gebaeudegrundflaeche
        ]
    );
    assert!(
        eg.markierungen
            .iter()
            .all(|m| m.gebaeude_id == Some(haus_id(&r))
                && m.geschoss_id == Some(haus.geschosse[0].id))
    );
    let werte = u.core.planaufmass_pruefen(u.projekt.id, &eg).await.unwrap();
    let flaeche = |i: usize| werte[i].flaeche_m2.unwrap();
    assert!(
        (flaeche(0) - efh::raumflaeche()).abs() < 1e-6,
        "{}",
        flaeche(0)
    );
    assert!(
        (flaeche(2) - efh::LAENGE * efh::BREITE).abs() < 1e-6,
        "{}",
        flaeche(2)
    );
    assert!((flaeche(3) - efh::LAENGE * efh::BREITE).abs() < 1e-6);
    let og: Planaufmass = serde_json::from_value(r.zeichnungen[1].daten.clone()).unwrap();
    assert_eq!(og.markierungen.len(), 3, "OG ohne Gebäudegrundfläche");

    assert_eq!(r.raumbuecher.len(), 2);
    let rb: Raumbuch = serde_json::from_value(r.raumbuecher[0].daten.clone()).unwrap();
    assert_eq!(rb.raeume.len(), 2);
    assert_eq!(rb.raeume[0].bezeichnung, "0.01 Wohnen");
    assert!((rb.raeume[0].flaeche_m2 - efh::raumflaeche()).abs() < 1e-6);
    assert_eq!(rb.raeume[0].hoehe_m, Some(efh::LICHTE_HOEHE));

    // Veraltete Revision der Modellakte wird abgewiesen.
    assert!(
        u.core
            .ifc_uebernehmen(u.projekt.id, &wunsch, &u.user)
            .await
            .is_err()
    );
    // Eigene Markierung ergänzen, dann erneut übernehmen.
    let mut eigen = eg.clone();
    let mut m = eg.markierungen[0].clone();
    m.id = Uuid::new_v4();
    m.bezeichnung = "Terrasse".into();
    m.bedeutung = Planbedeutung::Terrasse;
    eigen.markierungen.push(m.clone());
    u.core
        .fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Planaufmass,
            r.zeichnungen[0].objekt.parse().unwrap(),
            r.zeichnungen[0].revision,
            serde_json::to_value(&eigen).unwrap(),
            &u.user,
        )
        .await
        .unwrap();
    wunsch.revision = r.akte.revision;
    let r2 = u
        .core
        .ifc_uebernehmen(u.projekt.id, &wunsch, &u.user)
        .await
        .unwrap();
    assert_eq!(r2.gebaeude.objekt, r.gebaeude.objekt);
    let haus2: Gebaeude = serde_json::from_value(r2.gebaeude.daten.clone()).unwrap();
    assert_eq!(haus2.geschosse.len(), 2);
    let eg2: Planaufmass = serde_json::from_value(r2.zeichnungen[0].daten.clone()).unwrap();
    assert_eq!(eg2.markierungen.len(), 5);
    assert!(eg2.markierungen.iter().any(|x| x.id == m.id));
    for alt in &eg.markierungen {
        assert!(eg2.markierungen.iter().any(|x| x.id == alt.id));
    }
    assert_eq!(eg2.hintergruende.len(), 1);
    let rb2: Raumbuch = serde_json::from_value(r2.raumbuecher[0].daten.clone()).unwrap();
    assert_eq!(r2.raumbuecher[0].objekt, r.raumbuecher[0].objekt);
    assert_eq!(rb2.raeume.len(), 2);
    let stand = serde_json::from_value::<ifc::Modellakte>(r2.akte.daten)
        .unwrap()
        .uebernahme
        .unwrap();
    assert_eq!(stand.geschosse.len(), 2);
    assert!(stand.geschosse.iter().all(|g| g.raumbuch_id.is_some()));
}

fn haus_id(r: &ifc::Modelluebernahmeergebnis) -> Uuid {
    r.gebaeude.objekt.parse().unwrap()
}

/// Prüfung gegen das mitgelieferte IDS und gegen ein IDS aus den Projektdateien.
#[tokio::test]
async fn pruefung_mit_mitgeliefertem_und_eigenem_ids() {
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let akte_id = Uuid::new_v4();
    u.core
        .fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Ifcmodell,
            akte_id,
            0,
            serde_json::to_value(ifc::Modellakte {
                bezeichnung: "Entwurf".into(),
                dokument: d,
                uebernahme: None,
                nacharbeit: Vec::new(),
            })
            .unwrap(),
            &u.user,
        )
        .await
        .unwrap();
    let p = u
        .core
        .ifc_pruefen(u.projekt.id, akte_id, None)
        .await
        .unwrap();
    assert_eq!(p.regelwerk, "BauPlanIO – Modell für den Bauantrag");
    assert_eq!(p.fehler, 0);
    let ids = ablegen(
        &u,
        "buero.ids",
        r#"<ids xmlns="http://standards.buildingsmart.org/IDS"><info><title>Büro</title></info><specifications>
<specification name="Fenster" identifier="F-FENSTER" ifcVersion="IFC4"><applicability minOccurs="1"><entity><name><simpleValue>IFCWINDOW</simpleValue></name></entity></applicability></specification>
</specifications></ids>"#
            .into(),
    )
    .await;
    let p = u
        .core
        .ifc_pruefen(u.projekt.id, akte_id, Some(&ids))
        .await
        .unwrap();
    assert_eq!(p.regelwerk, "Büro");
    assert_eq!(p.befunde[0].regel, "IDS:F-FENSTER");
    assert_eq!(p.fehler, 1);
    let pdf = ablegen(&u, "Plan.pdf", "%PDF-1.4".into()).await;
    let e = u
        .core
        .ifc_pruefen(u.projekt.id, akte_id, Some(&pdf))
        .await
        .unwrap_err()
        .to_string();
    assert!(e.contains("keine IDS-Datei"), "{e}");
}

/// Nacharbeit in der Modellakte: geprüft beim Speichern, angewendet im Auszug
/// der Akte und in der Übernahme (neuer Raum aus Wänden).
#[tokio::test]
async fn nacharbeit_gespeichert_und_in_uebernahme_wirksam() {
    use core_lib::ifc::{Eigenschaftswert, Modellaenderung};
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let roh = u.core.ifc_auszug(u.projekt.id, &d).await.unwrap();
    let eg = roh.geschosse[0].global_id.clone();
    let akte_id = Uuid::new_v4();
    let mut akte = ifc::Modellakte {
        bezeichnung: "Entwurf".into(),
        dokument: d,
        uebernahme: None,
        nacharbeit: Vec::new(),
    };
    let speichern = |a: &ifc::Modellakte, basis: i64| {
        u.core.fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Ifcmodell,
            akte_id,
            basis,
            serde_json::to_value(a).unwrap(),
            &u.user,
        )
    };
    let f = speichern(&akte, 0).await.unwrap();
    // Räume des EG ausblenden, einen aus den Wänden neu bilden.
    let mut liste: Vec<Modellaenderung> = roh
        .elemente
        .iter()
        .filter(|e| e.klasse == "IFCSPACE" && e.geschoss.as_deref() == Some(eg.as_str()))
        .map(|e| Modellaenderung::Ausblenden {
            global_id: e.global_id.clone(),
        })
        .collect();
    akte.nacharbeit = liste.clone();
    let f = speichern(&akte, f.revision).await.unwrap();
    let v = u
        .core
        .ifc_raumvorschlaege(u.projekt.id, akte_id, &eg)
        .await
        .unwrap();
    assert_eq!(v.len(), 2);
    assert!(v.iter().all(|r| r.vorhanden.is_none()));
    liste.push(Modellaenderung::RaumBilden {
        global_id: v[0].global_id.clone(),
        geschoss: eg.clone(),
        name: "0.01".into(),
        langname: Some("Wohnen und Essen".into()),
        umriss: v[0].umriss.clone(),
        hoehe_m: Some(2.5),
    });
    liste.push(Modellaenderung::EigenschaftSetzen {
        global_id: v[0].global_id.clone(),
        satz: "Pset_SpaceCommon".into(),
        name: "IsExternal".into(),
        wert: Eigenschaftswert::Logisch { wert: false },
    });
    akte.nacharbeit = liste;
    let f = speichern(&akte, f.revision).await.unwrap();
    let a = u.core.ifc_akte_auszug(u.projekt.id, &akte).await.unwrap();
    assert_eq!(a.bericht.raeume, 3);
    assert!(a.element(&v[0].global_id).unwrap().eigenschaften.len() == 1);
    // Ungültige Nacharbeit wird abgewiesen.
    let mut falsch = akte.clone();
    falsch.nacharbeit.push(Modellaenderung::GeschossZuordnen {
        global_id: v[0].global_id.clone(),
        geschoss: "1111111111111111111111".into(),
    });
    assert!(speichern(&falsch, f.revision).await.is_err());

    let r = u
        .core
        .ifc_uebernehmen(
            u.projekt.id,
            &ifc::Modelluebernahmewunsch {
                akte_id,
                revision: f.revision,
                gebaeude_id: None,
                raumbuch: true,
            },
            &u.user,
        )
        .await
        .unwrap();
    let rb: core_lib::plattform::objekte::Raumbuch =
        serde_json::from_value(r.raumbuecher[0].daten.clone()).unwrap();
    assert_eq!(rb.raeume.len(), 1);
    assert_eq!(rb.raeume[0].bezeichnung, "0.01 Wohnen und Essen");
    assert_eq!(rb.raeume[0].hoehe_m, Some(2.5), "Höhe des gebildeten Raums");
}

/// Reimport: neue Dokumentversion abgleichen, Akte umstellen; überholte und
/// verwaiste Nacharbeit wahlweise streichen.
#[tokio::test]
async fn reimport_neue_version_mit_nacharbeit() {
    use core_lib::ifc::{Modellaenderung, Nacharbeitsstand, Reimportwunsch};
    let u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let roh = u.core.ifc_auszug(u.projekt.id, &d).await.unwrap();
    let gid = |k: &str| {
        roh.elemente
            .iter()
            .find(|e| e.klasse == k)
            .unwrap()
            .global_id
            .clone()
    };
    let raum = gid("IFCSPACE");
    let akte_id = Uuid::new_v4();
    let akte = ifc::Modellakte {
        bezeichnung: "Entwurf".into(),
        dokument: d.clone(),
        uebernahme: None,
        nacharbeit: vec![
            Modellaenderung::Klassifizieren {
                global_id: gid("IFCBUILDINGELEMENTPROXY"),
                klasse: "IFCWALL".into(),
                vordefiniert: None,
            },
            Modellaenderung::Ausblenden {
                global_id: gid("IFCCHIMNEY"),
            },
            Modellaenderung::Benennen {
                global_id: raum.clone(),
                name: "0.01".into(),
                langname: Some("Wohnzimmer".into()),
            },
        ],
    };
    let f = u
        .core
        .fachobjekt_speichern(
            u.projekt.id,
            ObjektArt::Ifcmodell,
            akte_id,
            0,
            serde_json::to_value(&akte).unwrap(),
            &u.user,
        )
        .await
        .unwrap();
    let v2 = u
        .core
        .store_document(
            u.projekt.id,
            Some(d.dokument_id),
            Some(1),
            "EFH.ifc",
            bytes::Bytes::from(testdaten::efh_ifc4_v2()),
            &u.user,
        )
        .await
        .unwrap();
    let neu = Dokumentbezug {
        dokument_id: v2.document_id,
        version: v2.version,
    };
    assert_eq!(neu.version, 2);
    let a = u
        .core
        .ifc_abgleich(u.projekt.id, akte_id, &neu)
        .await
        .unwrap();
    assert_eq!(a.entfernt.len(), 1);
    assert_eq!(
        a.nacharbeit,
        [
            Nacharbeitsstand::Ueberholt,
            Nacharbeitsstand::Verwaist,
            Nacharbeitsstand::Wirksam
        ]
    );
    let mut w = Reimportwunsch {
        akte_id,
        revision: f.revision,
        dokument: neu.clone(),
        ueberholte_entfernen: true,
        verwaiste_entfernen: true,
    };
    let f2 = u
        .core
        .ifc_reimport(u.projekt.id, &w, &u.user)
        .await
        .unwrap();
    let akte2: ifc::Modellakte = serde_json::from_value(f2.daten).unwrap();
    assert_eq!(akte2.dokument, neu);
    assert_eq!(akte2.nacharbeit.len(), 1);
    let a2 = u.core.ifc_akte_auszug(u.projekt.id, &akte2).await.unwrap();
    assert_eq!(
        a2.element(&raum).unwrap().langname.as_deref(),
        Some("Wohnzimmer")
    );
    // Gleiche Version oder veraltete Revision: abgewiesen.
    w.revision = f2.revision;
    assert!(
        u.core
            .ifc_reimport(u.projekt.id, &w, &u.user)
            .await
            .is_err()
    );
    w.dokument = d;
    w.revision = f.revision;
    assert!(
        u.core
            .ifc_reimport(u.projekt.id, &w, &u.user)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn auswertung_mit_bezugsflaeche_des_projekts() {
    let mut u = umgebung().await;
    let d = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let akte_id = Uuid::new_v4();
    let akte = ifc::Modellakte {
        bezeichnung: "Entwurf".into(),
        dokument: d.clone(),
        uebernahme: None,
        nacharbeit: Vec::new(),
    };
    let speichern = |p: core_lib::domain::ProjectId, a: &ifc::Modellakte| {
        u.core.fachobjekt_speichern(
            p,
            ObjektArt::Ifcmodell,
            akte_id,
            0,
            serde_json::to_value(a).unwrap(),
            &u.user,
        )
    };
    speichern(u.projekt.id, &akte).await.unwrap();
    // Projekt ohne Grundstücksfläche: Auswertung ohne GRZ/GFZ.
    let v = u
        .core
        .ifc_auswerten(u.projekt.id, akte_id, None)
        .await
        .unwrap();
    assert!((v.grundflaeche_m2 - 80.).abs() < 1e-3);
    assert_eq!((v.grz, v.gfz), (None, None));

    // Projekt mit noch nicht aufgeschlüsselter Fläche 400 m² als Bezugsfläche.
    let p = u
        .core
        .create_project(
            core_lib::domain::NewProject {
                name: "Mit Grundstück".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: Some(400.),
                planungsrecht: Default::default(),
            },
            &u.user,
        )
        .await
        .unwrap();
    u.projekt = p.clone();
    let d2 = ablegen(&u, "EFH.ifc", testdaten::efh_ifc4()).await;
    let akte2 = ifc::Modellakte {
        dokument: d2,
        ..akte.clone()
    };
    speichern(p.id, &akte2).await.unwrap();
    let v = u.core.ifc_auswerten(p.id, akte_id, None).await.unwrap();
    let g = v.grundstueck.as_ref().expect("Bezugsfläche");
    assert_eq!(g.flaeche_m2, 400.);
    assert_eq!((v.grz, v.gfz), (Some(0.2), Some(0.4)));
    assert!(
        u.core
            .ifc_auswerten(p.id, akte_id, Some(Uuid::new_v4()))
            .await
            .is_err()
    );
    // Akte eines fremden Projekts ist nicht auswertbar.
    let fremd = neues_projekt(&u.core, &u.user, "Fremd").await;
    assert!(u.core.ifc_auswerten(fremd.id, akte_id, None).await.is_err());
}
