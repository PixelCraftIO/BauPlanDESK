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
