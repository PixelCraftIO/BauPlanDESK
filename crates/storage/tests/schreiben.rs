//! Schreiben an die Behörde aus der Akte: echtes PDF im Briefbogen, Ablage
//! als Projektunterlage und Hinweise auf Lücken.
use core_lib::{
    CoreServices,
    domain::{Bundesland, NewProject},
    lbo::LboEngine,
    license::DevelopmentLicense,
    plattform::ObjektArt,
    verfahren::schreiben::{Schreibenart, Schreibenwunsch},
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn antwort_auf_nachforderung_als_pdf_und_projektunterlage() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("dateien"), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let u = core.anmelden("planer", "Daniel").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Neubau Lindenweg".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &u,
        )
        .await
        .unwrap()
        .id;
    let c = Uuid::new_v4();
    let nid = Uuid::new_v4();
    let akte = serde_json::json!({
        "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
        "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht Musterstadt", "kennung": "", "kontakt": ""},
        "aktenzeichen": "BA-42", "verantwortlich": null, "vorgaenger": null, "profil": null,
        "kommunikation": [{"id": c, "art": "brief", "eingehend": true, "datum": "2026-04-01",
            "betreff": "Nachforderung", "beteiligte": "", "inhalt": "", "original": null, "bezuege": []}],
        "nachforderungen": [{"id": nid, "kommunikation": c, "punkte": [
            {"id": Uuid::new_v4(), "titel": "Standsicherheitsnachweis nachreichen", "grundlage": "",
             "verantwortlich": null, "frist": {"datum": "2026-12-01", "ausloeser": "Schreiben", "beleg": null, "erinnerung_tage": 7, "grund": "Frist laut Schreiben"},
             "status": "offen", "antwort": "Der geprüfte Nachweis liegt bei.", "dokumente": [], "abgaben": [], "bestaetigung": null},
            {"id": Uuid::new_v4(), "titel": "Lageplan unterschreiben", "grundlage": "",
             "verantwortlich": null, "frist": null, "status": "offen", "antwort": "", "dokumente": [], "abgaben": [], "bestaetigung": null}
        ]}],
        "anforderungen": [], "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
    });
    let vid = Uuid::new_v4();
    core.fachobjekt_speichern(p, ObjektArt::Verfahren, vid, 0, akte, &u)
        .await
        .unwrap();
    // Die Wiedervorlage nennt die Rücknahmefiktion.
    let (_, a) = core.verfahrensakte(p, vid).await.unwrap();
    let t = core_lib::verfahren::termine::termine(&a, vid, Bundesland::Nw);
    assert!(t[0].hinweis.contains("§ 71 BauO NRW"), "{}", t[0].hinweis);

    let (r, pdf) = core
        .verfahren_schreiben(
            p,
            vid,
            Schreibenwunsch {
                art: Schreibenart::Nachforderungsantwort { nachforderung: nid },
                datum: Some("2026-05-02".parse().unwrap()),
                zusatz: "Für Rückfragen stehen wir gern zur Verfügung.".into(),
                ablegen: true,
            },
            &u,
        )
        .await
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    if let Some(ziel) = std::env::var_os("BAUPROJEKT_SCHREIBEN_ARTEFAKTE") {
        std::fs::create_dir_all(&ziel).unwrap();
        std::fs::write(std::path::Path::new(&ziel).join("antwort.pdf"), &pdf).unwrap();
    }
    let doc = lopdf::Document::load_mem(&pdf).unwrap();
    assert!(!doc.get_pages().is_empty());
    assert_eq!(
        r.dateiname,
        "2026-05-02 Nachreichung zur Nachforderung vom 01.04.2026.pdf"
    );
    assert!(r.hinweise.iter().any(|h| h.contains("Punkt 2")));
    let d = r.dokument.unwrap();
    let kategorien = core.kategoriesnapshots(p).await.unwrap();
    assert_eq!(
        kategorien
            .get(&d.dokument_id.0.to_string())
            .cloned()
            .flatten()
            .unwrap()
            .id,
        "schriftverkehr"
    );
    // Unbekannte Nachforderung: Fehler statt leerem Schreiben.
    assert!(
        core.verfahren_schreiben(
            p,
            vid,
            Schreibenwunsch {
                art: Schreibenart::Nachforderungsantwort {
                    nachforderung: Uuid::new_v4()
                },
                datum: None,
                zusatz: String::new(),
                ablegen: false,
            },
            &u,
        )
        .await
        .is_err()
    );
}
