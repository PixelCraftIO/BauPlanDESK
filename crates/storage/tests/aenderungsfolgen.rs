//! Folgen einer Änderung im Beispielprojekt: Ein Nachweis veraltet nur, wenn
//! sich ändert, was er verwendet; veraltete Nachweise lassen sich gesammelt
//! erneuern; das Gebäude bleibt dabei speicherbar.
#[path = "beispiel/basis.rs"]
mod basis;

use core_lib::{
    CoreServices,
    benutzer::Benutzer,
    domain::{Project, ProjectUpdate},
    plattform::{
        ObjektArt,
        nachweise::{NachweisAnsicht, NachweisEingabe},
    },
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

async fn wohnhaus(c: &CoreServices, p: &Project) -> core_lib::commit::Fachstand {
    c.fachobjekte(p.id, ObjektArt::Gebaeude)
        .await
        .unwrap()
        .into_iter()
        .find(|f| f.daten["bezeichnung"] == "Wohnhaus")
        .expect("Wohnhaus im Beispielprojekt")
}

/// Nur maßgebende Stände: frühere Stände sind Archiv.
fn veraltet(n: &[NachweisAnsicht]) -> Vec<&NachweisAnsicht> {
    n.iter()
        .filter(|n| !n.aktuell && n.ersetzt_durch.is_none())
        .collect()
}

#[tokio::test]
async fn dachneigung_macht_nur_hoehenabhaengige_nachweise_veraltet() {
    let (_dir, c, db, ich, p) = beispiel().await;
    let g = wohnhaus(&c, &p).await;
    let mut daten = g.daten.clone();
    assert_eq!(daten["geometrie"]["dachneigung_grad"], json!(36.0));
    daten["geometrie"]["dachneigung_grad"] = json!(38.0);
    let id = g.objekt.parse().unwrap();
    let g = c
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, g.revision, daten, &ich)
        .await
        .unwrap();

    let nachweise = c.nachweise(p.id).await.unwrap();
    let betroffen = veraltet(&nachweise);
    let namen: Vec<_> = betroffen.iter().map(|n| &n.stand.bezeichnung).collect();
    assert!(
        !betroffen.is_empty()
            && betroffen
                .iter()
                .all(|n| matches!(n.stand.eingabe, NachweisEingabe::AbstandProjekt { .. })),
        "Die Dachneigung betrifft nur den Abstandsflächennachweis, veraltet sind: {namen:?}"
    );

    // Der Antrag ist eingereicht: Die Übersicht meldet die Abweichung.
    let hinweis = |u: core_lib::projektuebersicht::Projektuebersicht| {
        u.punkte
            .into_iter()
            .find(|p| p.id == "einreichungsstand:nachweise")
            .map(|p| p.hinweis)
    };
    let vorher = hinweis(c.projektuebersicht(p.id, &ich).await.unwrap())
        .expect("Hinweis auf geänderte Berechnungen seit der Einreichung");
    assert!(vorher.contains("Abstandsflächennachweis"), "{vorher}");

    // Das Gebäude lässt sich danach beliebig oft weiter ändern.
    let mut daten = g.daten.clone();
    daten["geometrie"]["dachneigung_grad"] = json!(36.0);
    c.fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, g.revision, daten, &ich)
        .await
        .unwrap();
    assert!(
        veraltet(&c.nachweise(p.id).await.unwrap()).is_empty(),
        "Mit der alten Dachneigung ist alles wieder aktuell"
    );
    assert_eq!(
        hinweis(c.projektuebersicht(p.id, &ich).await.unwrap()),
        None
    );
    db.schliessen().await;
}

#[tokio::test]
async fn gebaeude_bleibt_mit_veralteter_vollgeschosspruefung_speicherbar() {
    let (_dir, c, db, ich, p) = beispiel().await;
    let g = wohnhaus(&c, &p).await;
    let id = g.objekt.parse().unwrap();
    let mut daten = g.daten.clone();
    let geschoss = daten["geschosse"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| !s["einstufungsquelle"].is_null())
        .expect("Geschoss mit Einstufung aus der Vollgeschossprüfung");
    let flaeche = daten["geschosse"][geschoss]["geometrie"]["flaeche_m2"]
        .as_f64()
        .unwrap();
    // Gezeichnete Werte folgen dem Aufmaß; hier wird von Hand geändert.
    daten.as_object_mut().unwrap().remove("planbindungen");
    daten["geschosse"][geschoss]["geometrie"]["flaeche_m2"] = json!(flaeche + 1.0);
    let g = c
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, g.revision, daten, &ich)
        .await
        .unwrap();
    let nachweise = c.nachweise(p.id).await.unwrap();
    for n in veraltet(&nachweise) {
        println!("veraltet: {}", n.stand.bezeichnung);
    }
    assert!(
        veraltet(&nachweise)
            .iter()
            .any(|n| matches!(&n.stand.eingabe,
            NachweisEingabe::Fach { plan, .. }
                if plan.modul == core_lib::berechnungen::BerechnungsArt::Vollgeschosse)),
        "Geänderte Geschossfläche macht die Vollgeschossprüfung veraltet"
    );

    // Früher scheiterte jedes weitere Speichern an der veralteten Quelle.
    let mut daten = g.daten.clone();
    daten["geschosse"][geschoss]["geometrie"]["flaeche_m2"] = json!(flaeche + 2.0);
    let g = c
        .fachobjekt_speichern(p.id, ObjektArt::Gebaeude, id, g.revision, daten, &ich)
        .await
        .expect("Gebäude bleibt speicherbar");
    assert_eq!(
        g.daten["geschosse"][geschoss]["vollgeschoss"],
        wohnhaus(&c, &p).await.daten["geschosse"][geschoss]["vollgeschoss"]
    );

    // Geänderte Werte verlangen eine Prüfung von Hand: Die Sammelaktion
    // erneuert solche Nachweise nicht stillschweigend, sondern nennt den Grund.
    let vorher = veraltet(&c.nachweise(p.id).await.unwrap()).len();
    let ergebnis = c.nachweise_erneuern(p.id, &ich).await.unwrap();
    for e in &ergebnis {
        println!("{} → {:?} {:?}", e.bezeichnung, e.neu, e.grund);
    }
    assert_eq!(ergebnis.len(), vorher);
    let vollgeschosse = ergebnis
        .iter()
        .find(|e| e.bezeichnung.contains("Vollgeschoss"))
        .expect("Vollgeschossprüfung in der Rückmeldung");
    assert!(vollgeschosse.neu.is_none() && vollgeschosse.grund.is_some());
    db.schliessen().await;
}

#[tokio::test]
async fn veraltete_nachweise_werden_gesammelt_erneuert() {
    let (_dir, c, db, ich, p) = beispiel().await;
    // Neue Bezeichnung des Bebauungsplans: Alle Nachweise nennen sie, kein
    // Rechenwert ändert sich.
    let mut recht = p.planungsrecht.clone();
    recht.plan_bezeichnung = "Am Lindenweg, 1. Änderung".into();
    c.update_project(
        ProjectUpdate {
            id: p.id,
            expected_version: p.version,
            name: p.name.clone(),
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
            planungsrecht: Some(recht),
        },
        &ich,
    )
    .await
    .unwrap();
    let vorher = c.nachweise(p.id).await.unwrap();
    let faellig = veraltet(&vorher).len();
    assert!(faellig > 5, "nur {faellig} Nachweise veraltet");

    let ergebnis = c.nachweise_erneuern(p.id, &ich).await.unwrap();
    let offen: Vec<_> = ergebnis
        .iter()
        .filter(|e| e.neu.is_none())
        .map(|e| (&e.bezeichnung, &e.grund))
        .collect();
    assert!(offen.is_empty(), "Nicht erneuert: {offen:?}");
    assert_eq!(ergebnis.len(), faellig);

    let nachher = c.nachweise(p.id).await.unwrap();
    assert!(veraltet(&nachher).is_empty());
    assert_eq!(nachher.len(), vorher.len() + faellig);
    // Die früheren Stände bleiben als Archiv erhalten und verweisen auf den neuen.
    for e in &ergebnis {
        let alt = nachher.iter().find(|n| n.stand.id == e.alt).unwrap();
        assert_eq!(alt.ersetzt_durch, e.neu);
    }
    // Die Geschosseinstufung des Gebäudes folgt dem neuen Stand der Prüfung.
    let g = wohnhaus(&c, &p).await;
    for s in g.daten["geschosse"].as_array().unwrap() {
        if let Some(q) = s["einstufungsquelle"]["nachweis_id"].as_str() {
            let n = nachher
                .iter()
                .find(|n| n.stand.id.to_string() == q)
                .unwrap();
            assert!(n.aktuell && n.ersetzt_durch.is_none());
        }
    }
    // Ein zweiter Lauf findet nichts mehr zu tun.
    assert!(c.nachweise_erneuern(p.id, &ich).await.unwrap().is_empty());
    db.schliessen().await;
}
