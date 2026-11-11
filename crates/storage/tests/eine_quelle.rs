//! Eine Quelle je Angabe im Beispielprojekt: Bauantrag, Abstandsplan und
//! Höhennachweis lesen Behörde, Anschrift, Gemeinde, Baugebiet und
//! Höhenfestsetzungen aus dem Projekt und führen keine eigene Fassung.
#[path = "beispiel/basis.rs"]
mod basis;

use core_lib::{
    CoreServices,
    benutzer::Benutzer,
    domain::{Project, ProjectUpdate},
    plattform::ObjektArt,
};
use serde_json::json;

const ARCHIV: &str = "../../beispiele/Beispielprojekt-EFH-Mustermann.zip";

async fn beispiel() -> (
    tempfile::TempDir,
    CoreServices,
    db_sqlite::SqliteDatabase,
    Benutzer,
    Project,
) {
    let archiv = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ARCHIV);
    let dir = tempfile::tempdir().unwrap();
    let profil = dir.path().join("Beispielprojekt");
    storage::sicherung::wiederherstellen(&archiv, &profil).unwrap();
    let (c, db) = basis::profil_oeffnen(&profil).await;
    let ich = core_lib::anmeldung::verwaltung_nach_wiederherstellung(
        &db.repositories(),
        "anwender",
        "Neue Anwenderin",
    )
    .await
    .unwrap();
    let p = c.list_projects().await.unwrap().remove(0);
    (dir, c, db, ich, p)
}

async fn recht_speichern(
    c: &CoreServices,
    p: &Project,
    ich: &Benutzer,
    aendern: impl FnOnce(&mut core_lib::domain::Planungsrecht),
) -> Project {
    let mut recht = p.planungsrecht.clone();
    aendern(&mut recht);
    c.update_project(
        ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
            planungsrecht: Some(recht),
        },
        ich,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn bauantrag_liest_behoerde_und_anschrift_aus_akte_und_stammdaten() {
    let (_dir, c, db, ich, p) = beispiel().await;
    let g = c.bauantrag(p.id).await.unwrap();
    assert!(g.bauaufsicht_zentral && g.anschrift_zentral);
    let akte: core_lib::verfahren::Verfahrensakte = serde_json::from_value(
        c.fachobjekte(p.id, ObjektArt::Verfahren)
            .await
            .unwrap()
            .remove(0)
            .daten,
    )
    .unwrap();
    assert_eq!(g.antrag.bauaufsicht.name, akte.behoerde.name.trim());
    assert!(!g.antrag.bauaufsicht.name.is_empty());
    assert_eq!(g.antrag.grundstueck.anschrift, p.planungsrecht.anschrift);
    assert_eq!(g.antrag.grundstueck.anschrift.strasse, "Lindenweg");
    assert!(
        !g.fehlende_angaben
            .iter()
            .any(|f| f.contains("Bauaufsichtsbehörde: Name")),
        "{:?}",
        g.fehlende_angaben
    );

    // Eine neue Anschrift in den Stammdaten gilt sofort im Bauantrag; eine
    // im Antrag mitgeschickte Fassung wird nicht als Kopie gespeichert.
    let p = recht_speichern(&c, &p, &ich, |r| r.anschrift.hausnummer = "12a".into()).await;
    let mut antrag = g.antrag.clone();
    antrag.grundstueck.anschrift.hausnummer = "99".into();
    let g = c
        .bauantrag_speichern(p.id, antrag, g.version, &ich)
        .await
        .unwrap();
    assert_eq!(g.antrag.grundstueck.anschrift.hausnummer, "12a");
    db.schliessen().await;
}

#[tokio::test]
async fn hoehennachweis_prueft_gegen_die_festsetzungen_des_projekts() {
    let (_dir, c, db, ich, p) = beispiel().await;
    let f = c
        .fachobjekte(p.id, ObjektArt::Hoehen)
        .await
        .unwrap()
        .remove(0);
    let plan: core_lib::berechnungen::FachPlan = serde_json::from_value(f.daten).unwrap();
    let (gleich, r) = c.berechnung_pruefen(p.id, plan.clone()).await.unwrap();
    assert_eq!(
        gleich, plan,
        "gespeicherter Plan entspricht den Festsetzungen"
    );
    assert!(r.vollstaendig, "{:?}", r.offen);
    assert_eq!(plan.eingaben["regeln"][1]["maximum_m"], json!(10.0));

    // Niedrigere Firsthöhe im Bebauungsplan: Der Nachweis übernimmt die Grenze
    // und meldet die Überschreitung, ohne dass jemand sie dort nachträgt.
    recht_speichern(&c, &p, &ich, |r| {
        r.festgesetzte_werte.firsthoehe_max_m = Some(7.0)
    })
    .await;
    let (neu, r) = c.berechnung_pruefen(p.id, plan.clone()).await.unwrap();
    let first = neu.eingaben["regeln"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["art"] == "first")
        .unwrap();
    assert_eq!(first["maximum_m"], json!(7.0));
    assert_ne!(neu.regelkontext.version, plan.regelkontext.version);
    assert!(!neu.regelkontext.anwendbarkeit_bestaetigt);
    assert!(
        r.offen
            .iter()
            .any(|o| o.contains("First") && o.contains("überschritten")),
        "{:?}",
        r.offen
    );
    db.schliessen().await;
}

#[tokio::test]
async fn abstandsplan_fuehrt_gemeinde_und_baugebiet_des_projekts() {
    let (_dir, c, db, ich, p) = beispiel().await;
    let f = c
        .fachobjekte(p.id, ObjektArt::Abstandsflaechen)
        .await
        .unwrap()
        .remove(0);
    let mut daten = f.daten.clone();
    daten["recht"]["gemeinde"] = json!("Andere Gemeinde");
    daten["recht"]["gebiet"] = json!("MI");
    daten["bezeichnung"] = json!("Abstandsflächen, zweite Fassung");
    let g = c
        .fachobjekt_speichern(
            p.id,
            ObjektArt::Abstandsflaechen,
            f.objekt.parse().unwrap(),
            f.revision,
            daten,
            &ich,
        )
        .await
        .unwrap();
    assert_eq!(g.daten["recht"]["gemeinde"], json!("Stadt Musterstadt"));
    assert_eq!(g.daten["recht"]["gebiet"], json!("WA"));
    db.schliessen().await;
}
