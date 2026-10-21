//! Abschluss des Beispielprojekts: Gesamtprüfung über die Projektübersicht,
//! Sicherungsarchiv wie „Einstellungen → Sicherung erstellen“ und Gegenprobe
//! durch Wiederherstellen in ein neues Profil.
#![allow(dead_code)]
use super::basis::*;
use core_lib::benutzer::Rolle;
use std::path::Path;

/// Offene Punkte der Projektübersicht am Stichtag; erwartete Punkte werden
/// als Information, alle übrigen als Befund gemeldet.
pub async fn uebersicht(b: &mut Beispiel, erwartet: &[&str]) {
    let u = b
        .c
        .projektuebersicht_am(b.p.id, stichtag(), &b.u)
        .await
        .unwrap();
    for p in &u.punkte {
        let text = format!("{} – {}", p.titel, p.hinweis);
        if erwartet.iter().any(|e| text.contains(e)) {
            b.info(format!("Projektübersicht (erwartet): {text}"));
        } else {
            b.befund(format!("Projektübersicht: offener Punkt „{text}“"));
        }
    }
    for n in b.c.nachweise(b.p.id).await.unwrap() {
        if !n.aktuell {
            b.befund(format!("Nachweis „{}“ ist nicht aktuell", n.stand.bezeichnung));
        }
    }
}

/// Bauvorlagenpaket wie „Unterlagen → Drucken / PDF-Ausgabe“: empfohlene
/// Berechnungen und alle verfügbaren Unterlagen mit Deckblatt, Anschreiben,
/// Anlagenverzeichnis und Trennblättern; abgelegt im Projekt-DMS.
pub async fn bauvorlagenpaket(b: &mut Beispiel) -> Vec<u8> {
    use core_lib::druckausgabe::{Druckauftrag, Druckquelle};
    b.neu_laden().await;
    let katalog = b.c.druckkatalog(b.p.id).await.unwrap();
    let mut quellen = vec![];
    for e in &katalog.eintraege {
        let berechnung = matches!(e.quelle, Druckquelle::Nachweis { .. } | Druckquelle::Baugrundstueck | Druckquelle::Raumbuch { .. });
        if !e.verfuegbar {
            // Nicht druckbare Dateiarten (z. B. DXF) sind erwartet; veraltete Berechnungen nicht.
            match (&e.hinweis, berechnung) {
                (Some(h), true) => b.befund(format!("Druckkatalog: „{}“ nicht verfügbar: {h}", e.titel)),
                (h, _) => b.info(format!("Druckkatalog: „{}“ nicht verfügbar: {}", e.titel, h.clone().unwrap_or_default())),
            }
            continue;
        }
        if berechnung && !e.empfohlen {
            continue;
        }
        quellen.push(e.quelle.clone());
    }
    let auftrag = Druckauftrag {
        gestaltung: None,
        aktenzeichen: String::new(),
        datum: Some(stichtag()),
        titel: "Bauvorlagen · Neubau Einfamilienhaus Mustermann".into(),
        absender: katalog.absender.clone(),
        empfaenger: "Stadt Musterstadt\nBauaufsicht\nRathausplatz 1\n59999 Musterstadt".into(),
        anschreiben: "Sehr geehrte Damen und Herren,\n\nanbei erhalten Sie die Bauvorlagen und Nachweise zum Neubau des Einfamilienhauses Lindenweg 12.\n\nMit freundlichen Grüßen\nErika Planer".into(),
        mit_anschreiben: true,
        inhaltsverzeichnis: true,
        deckblatt: true,
        trennblaetter: true,
        aufgestellt_von: None,
        projekt_version: b.p.version,
        quellen,
    };
    if katalog.aufgestellt_von.is_empty() {
        b.befund("Druckkatalog schlägt keine aufstellende Person vor (Entwurfsverfasserin im Bauantrag/Büro)");
    }
    let out = b.c.druckausgabe(b.p.id, auftrag).await.unwrap_or_else(|e| panic!("Bauvorlagenpaket: {e}"));
    b.c.druckpdf_ablegen(
        b.p.id,
        "Bauvorlagen Neubau EFH Mustermann.pdf",
        bytes::Bytes::copy_from_slice(&out.pdf),
        uuid::Uuid::new_v4(),
        &b.u,
    )
    .await
    .unwrap();
    b.info(format!("Bauvorlagenpaket: {} Seiten, {} Anlagen", out.seiten, out.positionen.len()));
    out.pdf
}

/// Sicherungsarchiv wie in der Desktop-App (`sicherung_erstellen`).
pub async fn sicherung(b: &Beispiel, ziel: &Path) {
    let scratch = tempfile::tempdir().unwrap();
    let snapshot = scratch.path().join("bauprojekt.db");
    b.db.sichern(&snapshot).await.unwrap();
    let kopie = db_sqlite::SqliteDatabase::open(&snapshot).await.unwrap();
    kopie.integritaet_pruefen().await.unwrap();
    let repos = kopie.repositories();
    let erwartet = storage::sicherung::dms_referenzen(&repos).await.unwrap();
    drop(repos);
    kopie.schliessen().await;
    let mut quellen = vec![storage::sicherung::Sicherungsquelle {
        name: "bauprojekt.db".into(),
        pfad: snapshot,
        erwartet: None,
    }];
    quellen.extend(
        storage::sicherung::dateiablage(&b.profil.join("dateien"), "dateien", erwartet).unwrap(),
    );
    let formulare = b.profil.join("Formularvorlagen");
    if formulare.exists() {
        quellen.extend(storage::sicherung::verzeichnis(&formulare, "Formularvorlagen").unwrap());
    }
    if ziel.exists() {
        std::fs::remove_file(ziel).unwrap();
    }
    if let Some(d) = ziel.parent() {
        std::fs::create_dir_all(d).unwrap();
    }
    storage::sicherung::erstellen(ziel, &quellen).unwrap();
    storage::sicherung::pruefen(ziel).unwrap();
}

/// Gegenprobe: Wiederherstellen in ein neues Profil; ein bisher unbekanntes
/// Rechnerkonto wird Administrator und sieht das Projekt mit aktuellen Nachweisen.
pub async fn gegenprobe(archiv: &Path, projektname: &str) {
    let ziel = tempfile::tempdir().unwrap();
    let profil = ziel.path().join("Wiederhergestellt");
    storage::sicherung::wiederherstellen(archiv, &profil).unwrap();
    db_sqlite::SqliteDatabase::datei_pruefen(&profil.join("bauprojekt.db"))
        .await
        .unwrap();
    let (c, db) = profil_oeffnen(&profil).await;
    let ich = c.anmelden("anwender", "Neue Anwenderin").await.unwrap();
    assert_eq!(ich.rolle, Rolle::Administrator, "Wiederhergestelltes Profil muss bearbeitbar sein");
    let p = c
        .list_projects()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name == projektname)
        .expect("Beispielprojekt im wiederhergestellten Profil");
    assert!(c.nachweise(p.id).await.unwrap().iter().all(|n| n.aktuell));
    for d in c.list_documents(p.id).await.unwrap() {
        c.load_version(d.document_id, d.version).await.unwrap();
    }
    db.schliessen().await;
}
