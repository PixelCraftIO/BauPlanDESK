//! Ende-zu-Ende: DXF-Plan ins DMS laden, öffnen und das Grundstück aufmessen.

use std::sync::Arc;

use bytes::Bytes;
use core_lib::cad::{Laengeneinheit, testdaten};
use core_lib::domain::{Bundesland, NewProject};
use core_lib::lbo::LboEngine;
use core_lib::license::DevelopmentLicense;
use core_lib::{CoreError, CoreServices};

#[tokio::test]
async fn plan_aus_dms_oeffnen_und_messen() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let storage = storage::build(&storage::StorageConfig::Local {
        root: dir.path().to_path_buf(),
    })
    .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        storage,
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let wer = core.anmelden("test", "Test").await.unwrap();
    let projekt = core
        .create_project(
            NewProject {
                name: "MFH Lindenweg".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
            },
            &wer,
        )
        .await
        .unwrap();

    let dxf = core
        .store_document(
            projekt.id,
            None,
            None,
            "Lageplan.dxf",
            Bytes::from(testdaten::musterplan()),
            &wer,
        )
        .await
        .unwrap();
    let geladen = core.cad_plan(dxf.document_id).await.unwrap();
    assert_eq!(geladen.dokument.file_name, "Lageplan.dxf");
    assert_eq!(geladen.plan.einheit, Some(Laengeneinheit::Meter));

    let grundstueck = geladen
        .plan
        .ebenen
        .iter()
        .position(|e| e.name == "Grundstück")
        .unwrap();
    let umriss = geladen
        .plan
        .linien
        .iter()
        .find(|l| l.ebene == grundstueck)
        .unwrap();
    let m = core
        .messen(&umriss.punkte, true, Laengeneinheit::Meter)
        .unwrap();
    assert_eq!(m.flaeche_m2, Some(600.0));

    let dwg = core
        .store_document(
            projekt.id,
            None,
            None,
            "Grundriss.DWG",
            Bytes::from_static(b"AC1032"),
            &wer,
        )
        .await
        .unwrap();
    let fehler = core.cad_plan(dwg.document_id).await.unwrap_err();
    assert!(matches!(fehler, CoreError::InvalidInput(m) if m.contains("DXF")));
}
