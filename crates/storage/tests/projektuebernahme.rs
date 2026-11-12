//! „Projekte aus Sicherung übernehmen“: Das Beispielprojekt landet in einem
//! bestehenden Profil neben den eigenen Projekten, ohne Profilwechsel und
//! Neustart. Der Test geht dieselben Schritte wie der Befehl
//! `projekte_aus_sicherung_uebernehmen` der Desktop-Anwendung.
#[path = "beispiel/basis.rs"]
mod basis;

use core_lib::{domain::NewProject, formular::instanz::FormularBezug};
use std::path::Path;

const ARCHIV: &str = "../../beispiele/Beispielprojekt-EFH-Mustermann.zip";

/// Entpacken, Quelle migrieren, Dateien kopieren, Zeilen übernehmen.
async fn uebernehmen(
    archiv: &Path,
    arbeit: &Path,
    ziel: &core_lib::CoreServices,
    db: &db_sqlite::SqliteDatabase,
) -> core_lib::CoreResult<Vec<db_sqlite::UebernommenesProjekt>> {
    storage::sicherung::wiederherstellen(archiv, arbeit)?;
    let quelle = db_sqlite::SqliteDatabase::open(&arbeit.join("bauprojekt.db")).await?;
    let repos = quelle.repositories();
    let erwartet = storage::sicherung::dms_referenzen(&repos).await;
    let projekte: Vec<_> = repos
        .projects
        .list()
        .await?
        .into_iter()
        .map(|p| p.id.0)
        .collect();
    drop(repos);
    quelle.schliessen().await;
    for p in &projekte {
        if ziel
            .repos
            .projects
            .get(core_lib::domain::ProjectId(*p))
            .await
            .is_ok()
        {
            return Err(core_lib::CoreError::InvalidInput(
                "Bereits vorhanden".into(),
            ));
        }
    }
    storage::projektuebernahme::dateien_uebernehmen(
        &arbeit.join("dateien"),
        ziel.storage.as_ref(),
        &projekte,
        &erwartet?,
    )
    .await?;
    db.projekte_uebernehmen(&arbeit.join("bauprojekt.db")).await
}

#[tokio::test]
async fn beispielprojekt_in_laufendes_profil_uebernehmen() {
    let archiv = Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIV);
    let dir = tempfile::tempdir().unwrap();
    let (c, db) = basis::profil_oeffnen(&dir.path().join("Eigenes")).await;
    let ich = c.anmelden("daniel", "Daniel").await.unwrap();
    let eigenes = db
        .repositories()
        .projects
        .create(NewProject {
            name: "Eigenes Projekt".into(),
            bundesland: core_lib::domain::Bundesland::Nw,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        })
        .await
        .unwrap();

    let neu = uebernehmen(&archiv, &dir.path().join("arbeit-1"), &c, &db)
        .await
        .unwrap();
    assert_eq!(neu.len(), 1);
    assert_eq!(neu[0].name, "Neubau Einfamilienhaus Mustermann");

    // Sofort sichtbar, über dieselben Services wie in der laufenden App.
    let projekte = c.projekte_fuer_benutzer(&ich).await.unwrap();
    assert_eq!(projekte.len(), 2);
    assert!(projekte.iter().any(|p| p.id == eigenes.id));
    let p = projekte.iter().find(|p| p.id.0 == neu[0].id).unwrap();

    let veraltet: Vec<_> = c
        .nachweise(p.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| !n.aktuell)
        .map(|n| n.stand.bezeichnung)
        .collect();
    assert!(
        veraltet.is_empty(),
        "Nicht aktuelle Nachweise: {veraltet:?}"
    );
    let quellen = c
        .formular_quellen(p.id, FormularBezug::default())
        .await
        .unwrap();
    let werte = &quellen.datenstand.werte;
    assert_eq!(werte.wert("kennzahlen.grz"), Some("0,19"));
    assert_eq!(werte.wert("bauherr.name"), Some("Max Mustermann"));
    // Büroangaben fehlten im Zielprofil und kommen mit dem Projekt.
    assert_eq!(werte.wert("entwurfsverfasser.name"), Some("Erika Planer"));
    let dokumente = c.list_documents(p.id).await.unwrap();
    assert!(!dokumente.is_empty());
    for d in dokumente {
        c.load_version(d.document_id, d.version).await.unwrap();
    }
    let tag = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    c.bereichsstatus_am(p.id, tag, &ich).await.unwrap();

    // Erneut übernehmen: verständlicher Fehler, nichts doppelt.
    let e = db
        .projekte_uebernehmen(&dir.path().join("arbeit-1/bauprojekt.db"))
        .await
        .unwrap_err();
    assert!(e.to_string().contains("Bereits vorhanden"), "{e}");
    assert_eq!(c.list_projects().await.unwrap().len(), 2);
    db.schliessen().await;
}
