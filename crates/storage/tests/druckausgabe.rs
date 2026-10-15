//! Echte Originale, archivierte Berechnungen und nicht schreibender Stapeldruck.
use bytes::Bytes;
use core_lib::{
    CoreServices,
    domain::{BebauungsplanArt, Bundesland, NewProject, Planungsrecht, Rechtsgrundlage},
    druckausgabe::{Druckauftrag, Druckquelle},
    flaechen::BauNvo,
    formular::{
        editor::{EigenesPdfFeld, felder_hinzufuegen},
        pdf::{Feldart, Feldwert, befuellen},
    },
    lbo::{AbstandsflaecheInput, LboEngine},
    license::DevelopmentLicense,
    plattform::nachweise::{NachweisEingabe, NutzungsArt},
};
use lopdf::{Document, Stream, dictionary};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;
fn formular(vorname: &str) -> Vec<u8> {
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let font =
        d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let mut kids = Vec::new();
    for (title, w, h) in [("Baubeschreibung", 595, 842), ("Anlage Lageplan", 842, 595)] {
        let c = d.add_object(Stream::new(
            dictionary! {},
            format!("BT /F1 14 Tf 55 {} Td ({title}) Tj ET", h - 50).into_bytes(),
        ));
        let page=d.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),w.into(),h.into()],"Contents"=>c});
        kids.push(page.into());
    }
    d.objects.insert(root,dictionary!{"Type"=>"Pages","Kids"=>kids,"Count"=>2,"Rotate"=>0,"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}}.into());
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    d.trailer.set("Root", cat);
    let mut bytes = Vec::new();
    d.save_to(&mut bytes).unwrap();
    let feld = |name: &str, y: f32, mehrzeilig: bool| EigenesPdfFeld {
        name: name.into(),
        beschriftung: name.into(),
        art: Feldart::Text,
        seite: 1,
        x_mm: 20.,
        y_mm: y,
        breite_mm: 100.,
        hoehe_mm: if mehrzeilig { 35. } else { 10. },
        schriftgroesse_pt: 11.,
        mehrzeilig,
    };
    let mut check = feld("Zustimmung", 125., false);
    check.art = Feldart::Ankreuzfeld;
    check.breite_mm = 5.;
    check.hoehe_mm = 5.;
    let bytes = felder_hinzufuegen(
        &bytes,
        &[
            feld("Vorname", 45., false),
            feld("Anschrift", 65., true),
            check,
        ],
    )
    .unwrap();
    befuellen(
        &bytes,
        &BTreeMap::from([
            ("Vorname".into(), Feldwert::Text(vorname.into())),
            (
                "Anschrift".into(),
                Feldwert::Text("Musterstraße 7\n12345 Musterstadt".into()),
            ),
            ("Zustimmung".into(), Feldwert::Ja(true)),
        ]),
    )
    .unwrap()
}
#[tokio::test]
async fn einzel_und_stapel_mit_unveraenderten_originalen_und_exakten_staenden() {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let lbo = Arc::new(LboEngine::new());
    lbo.load_directory(std::path::Path::new("../../lbo-rules"))
        .unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        lbo,
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("druck", "Daniel").await.unwrap();
    let plan = core_lib::baugrundstuecke::musterplan();
    let p = core
        .create_project(
            NewProject {
                name: "Neubau Musterstraße".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Planungsrecht {
                    rechtsgrundlage: Some(Rechtsgrundlage::Par30),
                    bau_nvo: Some(BauNvo::V2017),
                    bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                    baugrundstuecke: Some(plan.clone()),
                    ..Default::default()
                },
            },
            &user,
        )
        .await
        .unwrap();
    let original = formular("Daniel Müller");
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Baubeschreibung.pdf",
            Bytes::copy_from_slice(&original),
            &user,
        )
        .await
        .unwrap();
    core.store_document(
        p.id,
        Some(d.document_id),
        Some(1),
        "Baubeschreibung.pdf",
        Bytes::from(formular("Neuere Eingabe")),
        &user,
    )
    .await
    .unwrap();
    let dwg = core
        .store_document(
            p.id,
            None,
            None,
            "Lageplan.dwg",
            Bytes::from_static(b"DWG"),
            &user,
        )
        .await
        .unwrap();
    let mut quellen = vec![
        Druckquelle::Dokument {
            dokument_id: d.document_id,
            version: 1,
            seiten: vec![2, 1],
        },
        Druckquelle::Baugrundstueck,
    ];
    for fach in [
        NutzungsArt::Grz,
        NutzungsArt::Gfz,
        NutzungsArt::Geschossigkeit,
        NutzungsArt::Bmz,
    ] {
        let id = Uuid::new_v4();
        core.nachweis_erstellen(
            p.id,
            id,
            format!("{fach:?} Berechnung"),
            NachweisEingabe::Nutzung {
                fach,
                plan: plan.clone(),
            },
            &user,
        )
        .await
        .unwrap();
        quellen.push(Druckquelle::Nachweis { id });
    }
    let id = Uuid::new_v4();
    core.nachweis_erstellen(
        p.id,
        id,
        "Abstandsflächen Nordwand".into(),
        NachweisEingabe::Abstand {
            gegenstand: "Nordwand".into(),
            eingabe: AbstandsflaecheInput {
                wandhoehe_m: 10.,
                gebiet: core_lib::calc::Baugebiet::WA,
            },
        },
        &user,
    )
    .await
    .unwrap();
    quellen.push(Druckquelle::Nachweis { id });
    let a=Druckauftrag{titel:"Bauantragsunterlagen".into(),absender:"Planungsbüro Müller\nMusterstraße 1".into(),empfaenger:"Bauamt Musterstadt\nBauaufsicht".into(),anschreiben:"Sehr geehrte Damen und Herren,\n\nanbei die Unterlagen.\n\nMit freundlichen Grüßen\nDaniel Müller".into(),mit_anschreiben:true,inhaltsverzeichnis:true,projekt_version:p.version,quellen};
    let vorher = core
        .repos
        .protokoll
        .list_for_project(p.id, 1000)
        .await
        .unwrap()
        .len();
    let katalog = core.druckkatalog(p.id).await.unwrap();
    assert!(katalog.eintraege.iter().any(
        |e| matches!(e.quelle,Druckquelle::Dokument{dokument_id,..} if dokument_id==dwg.document_id)
            && !e.verfuegbar
    ));
    let out = core.druckausgabe(p.id, a.clone()).await.unwrap();
    let doc = Document::load_mem(&out.pdf).unwrap();
    assert_eq!(doc.get_pages().len() as u32, out.seiten);
    assert_eq!(out.positionen.len(), a.quellen.len());
    assert_eq!(out.positionen[0].erste_seite, 3);
    assert_eq!(out.positionen[0].seiten, 2);
    assert!(!doc.catalog().unwrap().has(b"AcroForm"));
    let pages = doc.get_pages();
    let first = doc.get_dictionary(pages[&3]).unwrap();
    assert_eq!(
        first.get(b"MediaBox").unwrap().as_array().unwrap()[2]
            .as_i64()
            .unwrap(),
        842
    );
    let second = doc.get_dictionary(pages[&4]).unwrap();
    assert_eq!(
        second.get(b"MediaBox").unwrap().as_array().unwrap()[2]
            .as_i64()
            .unwrap(),
        595
    );
    assert!(!doc.get_page_content(pages[&4]).is_empty());
    assert_eq!(
        core.load_version(d.document_id, 1)
            .await
            .unwrap()
            .1
            .as_ref(),
        original.as_slice()
    );
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 2);
    assert_eq!(
        core.repos
            .protokoll
            .list_for_project(p.id, 1000)
            .await
            .unwrap()
            .len(),
        vorher
    );
    assert!(core.einreichungen(p.id).await.unwrap().is_empty());
    let mut einzel = a.clone();
    einzel.mit_anschreiben = false;
    einzel.inhaltsverzeichnis = false;
    einzel.quellen = vec![Druckquelle::Dokument {
        dokument_id: d.document_id,
        version: 1,
        seiten: vec![1],
    }];
    let single = core.druckausgabe(p.id, einzel.clone()).await.unwrap();
    assert_eq!(single.seiten, 1);
    assert_eq!(single.positionen[0].erste_seite, 1);
    for seiten in [vec![0], vec![3], vec![1, 1]] {
        einzel.quellen = vec![Druckquelle::Dokument {
            dokument_id: d.document_id,
            version: 1,
            seiten,
        }];
        assert!(core.druckausgabe(p.id, einzel.clone()).await.is_err());
    }
    einzel.quellen = vec![Druckquelle::Dokument {
        dokument_id: dwg.document_id,
        version: 1,
        seiten: vec![],
    }];
    assert!(core.druckausgabe(p.id, einzel.clone()).await.is_err());
    einzel.projekt_version += 1;
    assert!(core.druckausgabe(p.id, einzel).await.is_err());
    let fremd = core
        .create_project(
            NewProject {
                name: "Anderes Projekt".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let mut falsch = a.clone();
    falsch.projekt_version = fremd.version;
    assert!(core.druckausgabe(fremd.id, falsch).await.is_err());
    if let Ok(ziel) = std::env::var("BAUPROJEKT_DRUCK_ARTEFAKTE") {
        std::fs::create_dir_all(&ziel).unwrap();
        let ziel = std::path::Path::new(&ziel);
        let mut ui_auftrag = a.clone();
        ui_auftrag.quellen = katalog
            .eintraege
            .iter()
            .filter(|e| e.verfuegbar)
            .map(|e| e.quelle.clone())
            .collect();
        let ui = core.druckausgabe(p.id, ui_auftrag).await.unwrap();
        std::fs::write(ziel.join("ui-stapel.pdf"), &ui.pdf).unwrap();
        let mut ui_einzel = a.clone();
        ui_einzel.mit_anschreiben = false;
        ui_einzel.inhaltsverzeichnis = false;
        ui_einzel.quellen = vec![Druckquelle::Dokument {
            dokument_id: d.document_id,
            version: 2,
            seiten: vec![1],
        }];
        let ui_single = core.druckausgabe(p.id, ui_einzel).await.unwrap();
        std::fs::write(ziel.join("ui-einzel.pdf"), &ui_single.pdf).unwrap();
        std::fs::write(ziel.join("stapel.pdf"), &out.pdf).unwrap();
        std::fs::write(ziel.join("einzel.pdf"), &single.pdf).unwrap();
        std::fs::write(ziel.join("original.pdf"), &original).unwrap();
        std::fs::write(ziel.join("workflow.json"),serde_json::to_vec_pretty(&serde_json::json!({"projekt":p,"katalog":katalog,"auftrag":a,"positionen":out.positionen,"seiten":out.seiten})).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn weitere_fachberichte_beachten_revisionen_und_veraltete_planstaende() {
    use core_lib::{
        cad::{Laengeneinheit, Punkt, baugrenzen::BaugrenzenGeometrie},
        domain::ProjectUpdate,
        plattform::ObjektArt,
    };
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().join("Dateien"), "local").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("fachberichte", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Fachberichte".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.pdf",
            Bytes::from(formular("Daniel")),
            &user,
        )
        .await
        .unwrap();
    let mut recht = p.planungsrecht.clone();
    recht.baugrenzen = Some(BaugrenzenGeometrie {
        dokument_id: d.document_id,
        dokument_version: 1,
        einheit: Laengeneinheit::Meter,
        baufenster: vec![
            Punkt(0., 0.),
            Punkt(10., 0.),
            Punkt(10., 10.),
            Punkt(0., 10.),
        ],
        vorhaben: vec![Punkt(8., 2.), Punkt(12., 2.), Punkt(12., 6.), Punkt(8., 6.)],
    });
    let p = core
        .update_project(
            ProjectUpdate {
                id: p.id,
                expected_version: p.version,
                name: p.name,
                bundesland: p.bundesland,
                grundstuecksflaeche_m2: None,
                planungsrecht: Some(recht),
            },
            &user,
        )
        .await
        .unwrap();
    let gid = Uuid::new_v4();
    core.fachobjekt_speichern(p.id,ObjektArt::Gebaeude,gid,0,serde_json::json!({"bezeichnung":"Haus A","grundstueck_ids":[],"geschosse":[],"quelle":null}),&user).await.unwrap();
    let rid = Uuid::new_v4();
    let raum = serde_json::json!({"bezeichnung":"Erdgeschoss","gebaeude_id":gid,"geschoss_id":null,"raeume":[{"id":Uuid::new_v4(),"nummer":"1","bezeichnung":"Büro","nutzung":"Arbeiten","flaeche_m2":20.0,"hoehe_m":2.5}]});
    let f = core
        .fachobjekt_speichern(p.id, ObjektArt::Raumbuch, rid, 0, raum.clone(), &user)
        .await
        .unwrap();
    let a = Druckauftrag {
        titel: "Fachberichte".into(),
        absender: String::new(),
        empfaenger: String::new(),
        anschreiben: String::new(),
        mit_anschreiben: false,
        inhaltsverzeichnis: true,
        projekt_version: p.version,
        quellen: vec![
            Druckquelle::Baugrenzen,
            Druckquelle::Raumbuch {
                id: rid,
                revision: f.revision,
            },
        ],
    };
    let out = core.druckausgabe(p.id, a.clone()).await.unwrap();
    assert_eq!(out.positionen.len(), 2);
    assert_eq!(out.seiten, 3);
    core.store_document(
        p.id,
        Some(d.document_id),
        Some(1),
        "Plan.pdf",
        Bytes::from(formular("Korrigiert")),
        &user,
    )
    .await
    .unwrap();
    // Historische Geometrie bleibt ausgebbar, wird im Bericht als veraltet markiert.
    let alt = core.druckausgabe(p.id, a.clone()).await.unwrap();
    assert_eq!(alt.seiten, 3);
    let mut neu = raum;
    neu["raeume"][0]["flaeche_m2"] = serde_json::json!(30.0);
    core.fachobjekt_speichern(p.id, ObjektArt::Raumbuch, rid, f.revision, neu, &user)
        .await
        .unwrap();
    assert!(
        core.druckausgabe(p.id, a)
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("Raumbuch geändert")
    );
}
