//! Planliste: Plannummer aus Büroschema und Projektkennzeichen, Büroindex,
//! neuere Dateiversion ohne Index.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    planliste::Plannummernschema,
    plattform::ObjektArt,
};
use std::sync::Arc;

#[tokio::test]
async fn planliste_nummern_index_und_dubletten() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Planung").await.unwrap();
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
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Projektorganisation,
        p.id.0,
        0,
        serde_json::json!({"kennzeichen": "2026-014", "phase": "entwurf", "verantwortlich": null, "beschreibung": ""}),
        &u,
    )
    .await
    .unwrap();
    let ablegen = |name: &'static str, id| {
        let core = core.clone();
        let u = u.clone();
        async move {
            core.store_document(p.id, id, None, name, Bytes::from_static(b"%PDF-1.4"), &u)
                .await
                .unwrap()
        }
    };
    let eg = ablegen("Grundriss EG.pdf", None).await;
    let og = ablegen("Grundriss OG.pdf", None).await;
    let heute = core_lib::kalender::heute();
    let eintrag = |d: &core_lib::domain::DocumentVersion, geschoss: &str, nr: u32| {
        serde_json::json!({
            "dokument_id": d.document_id, "planart": "grundriss", "geschoss": geschoss,
            "nummer": nr, "titel": format!("Grundriss {geschoss}"), "massstab": 100,
            "revisionen": [{"index": "a", "datum": heute, "aenderung": "Erstausgabe", "version": d.version}],
            "verteiler": []
        })
    };
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planliste,
            eg.document_id.0,
            0,
            eintrag(&eg, "EG", 1),
            &u,
        )
        .await
        .unwrap();
    assert_eq!(f.daten["plannummer"], "2026-014-GR-EG-01");
    // Gleiche Nummer an anderem Plan wird abgewiesen.
    let mut doppelt = eintrag(&og, "EG", 1);
    doppelt["titel"] = "Grundriss OG".into();
    let e = core
        .fachobjekt_speichern(p.id, ObjektArt::Planliste, og.document_id.0, 0, doppelt, &u)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("schon an"), "{e}");
    core.fachobjekt_speichern(
        p.id,
        ObjektArt::Planliste,
        og.document_id.0,
        0,
        eintrag(&og, "OG", 2),
        &u,
    )
    .await
    .unwrap();
    // Falsche Objektkennung: Eintrag gehört zum Dokument.
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Planliste,
            og.document_id.0,
            1,
            eintrag(&eg, "EG", 1),
            &u
        )
        .await
        .is_err()
    );

    // Neue Dateiversion ohne Index wird gemeldet.
    ablegen("Grundriss EG.pdf", Some(eg.document_id)).await;
    let l = core.planliste(p.id, &u).await.unwrap();
    assert_eq!(l.kennzeichen, "2026-014");
    let nummern: Vec<_> = l
        .zeilen
        .iter()
        .map(|z| z.eintrag.plannummer.as_str())
        .collect();
    assert_eq!(nummern, ["2026-014-GR-EG-01", "2026-014-GR-OG-02"]);
    let z = &l.zeilen[0];
    assert!(z.ohne_index);
    assert_eq!(z.aktuelle_version, Some(2));
    assert_eq!(z.index.as_ref().unwrap().index, "a");
    assert!(!l.zeilen[1].ohne_index);

    // Büroschema ändern: Projektleitung nötig, Muster wird geprüft.
    assert!(
        core.plannummernschema_speichern(
            Plannummernschema {
                muster: "{planart}".into()
            },
            &u
        )
        .await
        .is_err()
    );
    core.plannummernschema_speichern(
        Plannummernschema {
            muster: "{planart}{nr:3}".into(),
        },
        &u,
    )
    .await
    .unwrap();
    let og_rev = l.zeilen[1].revision;
    let f = core
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Planliste,
            og.document_id.0,
            og_rev,
            eintrag(&og, "OG", 2),
            &u,
        )
        .await
        .unwrap();
    assert_eq!(f.daten["plannummer"], "GR002");
    // Veraltete Basis: Konflikt statt stillem Überschreiben.
    assert!(
        core.fachobjekt_speichern(
            p.id,
            ObjektArt::Planliste,
            og.document_id.0,
            og_rev,
            eintrag(&og, "OG", 3),
            &u
        )
        .await
        .is_err()
    );
}
