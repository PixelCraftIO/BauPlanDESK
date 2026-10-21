//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“: Abstandsflächen nach
//! § 6 BauO NRW 2018 (Paket `abstand-nw`) mit Lageplan-DXF, Grundstücks-
//! grenzen je Flurstück, Gebäudeumrissen aus Gebäude/Geschosse, Satteldach mit
//! Giebelanrechnung, Grenzgarage, Rechts-/Umfangsprüfung, Nachweis, Freigabe,
//! PDF sowie JSON-/DXF-Export.
//!
//! Lage (lokales System in Metern, x nach Osten, y nach Norden, Ursprung
//! Südwestecke Flst. 412): Flurstück 412 = 20,00 × 31,00 m, Straße Lindenweg
//! an der Nordseite (y = 31). Wohnhaus 11,00 × 9,00 m, 5,00 m hinter der
//! Straßengrenze und 3,00 m von der Westgrenze: x 3–14, y 17–26, Traufseiten
//! Nord/Süd. Grenzgarage 3,50 × 6,00 m an der Ostgrenze: x 16,5–20, y 20–26.
#![allow(dead_code)]
use super::basis::*;
use bytes::Bytes;
use core_lib::{
    abstandsflaechen::*,
    cad::Laengeneinheit,
    druckausgabe::{Druckauftrag, Druckquelle},
    plattform::{ObjektArt, nachweise::NachweisEingabe, objekte::Dokumentbezug},
};
use uuid::Uuid;

/// Fachobjekt des Abstandsplans (fest, damit spätere Module darauf verweisen können).
pub const PLAN_ID: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0000_0412);
pub const G412: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0412);
pub const G411: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0411);
pub const G413: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0413);
pub const UMRISS_HAUS: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0002_0001);
pub const UMRISS_GARAGE: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0002_0002);
pub const DACHNEIGUNG: f64 = 38.;

fn lw(ebene: &str, p: &[(f64, f64)]) -> String {
    let mut s = format!("0\nLWPOLYLINE\n8\n{ebene}\n90\n{}\n70\n1\n", p.len());
    for (x, y) in p {
        s += &format!("10\n{x}\n20\n{y}\n");
    }
    s
}
fn rechteck(x: f64, y: f64, b: f64, t: f64) -> Vec<(f64, f64)> {
    vec![(x, y), (x + b, y), (x + b, y + t), (x, y + t)]
}
/// Lageplan des Vermessers als DXF in Metern (Umlaufsinn gegen den Uhrzeigersinn).
pub fn lageplan_dxf() -> String {
    let mut s = String::from(
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n",
    );
    s += &lw("FLST_411", &rechteck(-21.13, 0., 21.13, 31.));
    s += &lw("FLST_412", &rechteck(0., 0., 20., 31.));
    s += &lw("FLST_413", &rechteck(20., 0., 19.29, 31.));
    s += &lw("GEB_WOHNHAUS", &rechteck(3., 17., 11., 9.));
    s += &lw("GEB_GARAGE", &rechteck(16.5, 20., 3.5, 6.));
    s += "0\nENDSEC\n0\nEOF\n";
    s
}
fn nah(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}
fn wand_mut<'a>(p: &'a mut AbstandsPlan, umriss: Uuid, seite: usize) -> &'a mut AbstandsWand {
    p.waende
        .iter_mut()
        .find(|w| {
            w.umriss
                .as_ref()
                .is_some_and(|s| s.umriss_id == umriss && s.seite == seite)
        })
        .expect("Umrisswand")
}
pub fn wand_id(p: &AbstandsPlan, umriss: Uuid, seite: usize) -> Uuid {
    p.waende
        .iter()
        .find(|w| {
            w.umriss
                .as_ref()
                .is_some_and(|s| s.umriss_id == umriss && s.seite == seite)
        })
        .expect("Umrisswand")
        .id
}
fn ergebnis<'a>(r: &'a AbstandsErgebnis, id: Uuid) -> &'a WandErgebnis {
    r.waende.iter().find(|w| w.id == id).expect("Wandergebnis")
}

pub async fn abstand(b: &mut super::basis::Beispiel) {
    let pid = b.p.id;
    // --- 1. Höhen aus Gebäude/Geschosse (gleicher Weg wie „Traufe übernehmen“) ---
    let gh =
        b.c.gebaeudegrundlagen(pid, b.haus)
            .await
            .unwrap()
            .hoehen_nhn;
    let (gelaende, traufe, first) = (
        gh.gelaende_m.expect("Gelände Wohnhaus"),
        gh.traufe_m.expect("Traufe Wohnhaus"),
        gh.first_m.expect("First Wohnhaus"),
    );
    assert!(
        nah(gelaende, 72.34) && nah(traufe, 76.84) && nah(first, 80.09),
        "{gh:?}"
    );
    let giebelhoehe = first - traufe; // 3,25 m
    // Plausibilität Satteldach: halbe Gebäudetiefe 4,50 m × tan 38° = 3,52 m.
    let erwartet = 4.5 * DACHNEIGUNG.to_radians().tan();
    if (erwartet - giebelhoehe).abs() > 0.1 {
        b.befund(format!(
            "Abstandsflächen/Gebäude: Giebelhöhe First − Traufe = {giebelhoehe:.2} m passt nicht zur Dachneigung {DACHNEIGUNG}° bei 9,00 m Giebelbreite \
             (erwartet 4,50 × tan 38° = {erwartet:.2} m, entspricht {:.1}°). Der Core prüft Giebelhöhe, beide Dachneigungen und Wandlänge nicht \
             gegeneinander; die Basisdaten (Traufe +4,20 / First +7,45) sind in sich nicht stimmig. Gerechnet wird mit der Giebelhöhe aus den Gebäudehöhen; mit 3,52 m ergäbe sich H = 5,67 m, die Tiefe bliebe 3,00 m.",
            (giebelhoehe / 4.5).atan().to_degrees()
        ));
    }
    let gg =
        b.c.gebaeudegrundlagen(pid, b.garage)
            .await
            .unwrap()
            .hoehen_nhn;
    if gg.traufe_m.is_none() {
        b.info("Garage hat in Gebäude/Geschosse keinen Nullpunkt/keine Wandhöhe (Basis); Oberkante 2,80 m über Gelände wird im Umriss manuell erfasst.");
    }

    // --- 2. Lageplan-DXF archivieren und Konturen übernehmen ---
    let dxf =
        b.c.store_document(
            pid,
            None,
            None,
            "Lageplan_Vermessung_Flst412.dxf",
            Bytes::from(lageplan_dxf()),
            &b.u,
        )
        .await
        .unwrap();
    let quelle = Dokumentbezug {
        dokument_id: dxf.document_id,
        version: dxf.version,
    };
    let konturen =
        b.c.abstandscad_konturen(pid, quelle.clone(), Laengeneinheit::Meter)
            .await
            .unwrap();
    assert_eq!(konturen.len(), 5, "{konturen:?}");
    let kontur = |ebene: &str| {
        konturen
            .iter()
            .find(|k| k.ebene == ebene)
            .unwrap_or_else(|| panic!("Kontur {ebene}"))
            .punkte
            .clone()
    };
    let flaeche = |p: &[APunkt]| {
        (0..p.len())
            .map(|i| {
                let (a, c) = (&p[i], &p[(i + 1) % p.len()]);
                a.x * c.y - c.x * a.y
            })
            .sum::<f64>()
            .abs()
            / 2.
    };
    let grenze412 = kontur("FLST_412");
    assert!(nah(flaeche(&grenze412), GRUNDSTUECK_M2));

    // --- 3. Abstandsplan erfassen ---
    let regeln = katalog()
        .unwrap()
        .into_iter()
        .find(|r| r.land == core_lib::domain::Bundesland::Nw)
        .expect("NRW-Paket");
    assert_eq!(
        (regeln.id.as_str(), regeln.version.as_str()),
        ("abstand-nw", "2026-09-01")
    );
    let r = &b.p.planungsrecht;
    let gebiet = serde_json::to_value(r.festgesetzte_werte.baugebiet)
        .unwrap()
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert_eq!(gebiet, "WA");
    let grundstueck = |id, name: &str, fs: &str, grenze: Vec<APunkt>| AbstandsGrundstueck {
        id,
        bezeichnung: name.into(),
        flurstueck_ids: vec![fs.into()],
        grenze,
        innenringe: vec![],
        grundlage: format!(
            "Lageplan zum Bauantrag, ÖbVI-Vermessung 09/2026 (Lageplan_Vermessung_Flst412.dxf, Ebene {}); Gemarkung Musterstadt, Flur 3",
            fs.replace("fs-", "FLST_")
        ),
    };
    let haus_quelle = format!(
        "Gebäude/Geschosse „Wohnhaus“: Gelände {gelaende:.2}, Traufe {traufe:.2} m ü. NHN; Grundriss aus Lageplan-DXF (Ebene GEB_WOHNHAUS)"
    );
    let ecken = |p: Vec<APunkt>, g: f64| {
        p.into_iter()
            .map(|p| UmrissEcke {
                x: p.x,
                y: p.y,
                gelaende_m: Some(g),
            })
            .collect::<Vec<_>>()
    };
    let mut plan = AbstandsPlan {
        planbindungen: vec![],
        bezeichnung: "Abstandsflächen Neubau EFH Mustermann, Lindenweg 12".into(),
        regeln,
        recht: AbstandsRecht {
            gemeinde: r.gemeinde.clone(),
            fassung: String::new(), // nach dem Paket gesetzt
            anwendung: "Neubau; Bauantrag 10/2026 nach Inkrafttreten der Fassung vom 01.09.2026, keine Übergangsregelung einschlägig".into(),
            gebiet: gebiet.clone(),
            satzung: "Keine örtliche Bauvorschrift über abweichende Abstandsflächentiefen (§ 89 BauO NRW); Auskunft Stadt Musterstadt".into(),
            planfestsetzung: format!(
                "Bebauungsplan Nr. {} „{}“: offene Bauweise, keine Festsetzung abweichender Abstandsflächen oder Grenzbebauung (§ 6 Abs. 1 Satz 3, Abs. 5 Satz 6 BauO NRW)",
                r.plan_nummer, r.plan_bezeichnung
            ),
            geprueft: true,
            pruefvermerk: "Geprüft E. Planer 21.10.2026: Traufwände H = 76,84 − 72,34 = 4,50 m (Dach 38° ≤ 45° nicht angerechnet), 0,4 H = 1,80 m < 3,00 m → 3,00 m; \
                Giebelwände H = 4,50 + 3,25/3 = 5,58 m, 0,4 H = 2,23 m → 3,00 m; Grenzgarage § 6 Abs. 8 Satz 1 Nr. 1: Wandhöhe 2,80 m ≤ 3 m, Grenzlänge 6,00 m ≤ 9 m"
                .into(),
        },
        hoehenbezug: format!(
            "m ü. NHN (DHHN2016); ±0,00 = OK FFB EG = {NULLPUNKT_NHN:.2} m ü. NHN; maßgebliche Geländeoberfläche eben {gelaende:.2} m ü. NHN (natürliches Gelände, keine Abgrabungen)"
        ),
        koordinatenbezug: "Lokales System des Lageplans in Metern, x nach Osten, y nach Norden, Ursprung Südwestecke Flst. 412; Straße Lindenweg im Norden".into(),
        grundstuecke: vec![
            grundstueck(G412, "Baugrundstück Flst. 412 (Bauherr Mustermann)", "fs-412", grenze412),
            grundstueck(G411, "Nachbar Flst. 411 (Nachbar, West)", "fs-411", kontur("FLST_411")),
            grundstueck(G413, "Nachbar Flst. 413 (Beispiel, Ost)", "fs-413", kontur("FLST_413")),
        ],
        waende: vec![],
        flaechen: vec![],
        quellen: vec![quelle.clone()],
        umfang_geprueft: true,
        umfang_vermerk: "Alle Außenwände von Wohnhaus (4) und Garage (4) aus den Gebäudeumrissen; Gelände eben, keine Vorbauten > 1,50 m, \
            keine Gauben, Dachüberstand 0,50 m unter § 6 Abs. 6; Garage ohne Aufenthaltsraum geprüft"
            .into(),
        umrisse: vec![
            AbstandsUmriss {
                id: UMRISS_HAUS,
                bezeichnung: "Wohnhaus".into(),
                gebaeude_id: Some(b.haus),
                grundstueck_id: Some(G412),
                ecken: ecken(kontur("GEB_WOHNHAUS"), gelaende),
                oberkante_m: Some(traufe),
                quelle: haus_quelle.clone(),
            },
            AbstandsUmriss {
                id: UMRISS_GARAGE,
                bezeichnung: "Garage".into(),
                gebaeude_id: Some(b.garage),
                grundstueck_id: Some(G412),
                ecken: ecken(kontur("GEB_GARAGE"), gelaende),
                oberkante_m: Some(gelaende + 2.8),
                quelle: "Garage: Flachdach OK Attika 2,80 m über Gelände (Ansicht Garage, Entwurf 10/2026); Grundriss aus Lageplan-DXF (Ebene GEB_GARAGE)".into(),
            },
        ],
    };
    plan.recht.fassung = plan.regeln.rechtsstand.clone();
    // Seiten als Wände übernehmen (Core-Abgleich wie in der Oberfläche).
    let abgleich = umrisse_abgleichen(plan).unwrap();
    for h in &abgleich.hinweise {
        b.info(format!("Umrissabgleich: {h}"));
    }
    let mut plan = abgleich.plan;
    assert_eq!(plan.waende.len(), 8);
    // Seiten im Gegenuhrzeigersinn ab Südwestecke: 0 Süd, 1 Ost, 2 Nord, 3 West.
    let namen = ["Süd", "Ost", "Nord", "West"];
    for (seite, name) in namen.iter().enumerate() {
        let traufseite = seite % 2 == 0;
        let w = wand_mut(&mut plan, UMRISS_HAUS, seite);
        assert_eq!(w.aussen, Aussenrichtung::Rechts);
        w.bezeichnung = format!(
            "{name}wand ({})",
            if traufseite {
                "Traufseite"
            } else {
                "Giebelseite"
            }
        );
        w.dachhoehe_m = Some(giebelhoehe);
        w.dachneigung_grad = Some(DACHNEIGUNG);
        if traufseite {
            w.dachart = Dachart::Dachflaeche;
            w.dachgrundlage =
                "Satteldach 38°, Dachfläche über der Traufwand (≤ 45°, keine Gauben)".into();
        } else {
            w.dachart = Dachart::Giebel;
            w.dachneigung_b_grad = Some(DACHNEIGUNG);
            w.dachgrundlage = format!(
                "Giebeldreieck First − Traufe = {} m, beide Dachseiten 38°",
                format!("{giebelhoehe:.2}").replace('.', ",")
            );
        }
    }
    for (seite, name) in namen.iter().enumerate() {
        let w = wand_mut(&mut plan, UMRISS_GARAGE, seite);
        w.bezeichnung = format!("Garage {name}");
        w.dachart = Dachart::Flach;
        w.dachgrundlage = "Flachdach mit Attika".into();
        w.fall = Wandfall::Garage;
        w.nutzung_geprueft = true;
        w.grenzkennung = "Ostgrenze zu Flst. 413".into();
    }

    // --- 4. Grenzgarage: nur die Grenzwand trägt die Grenzkennung; die übrigen
    // Garagenwände sind Teil der privilegierten Anlage (§ 6 Abs. 8 Satz 1 BauO NRW,
    // „ohne eigene Abstandsflächen“) und zählen nicht zur Grenzlänge.
    for seite in [0, 2, 3] {
        wand_mut(&mut plan, UMRISS_GARAGE, seite)
            .grenzkennung
            .clear();
    }
    let garage_ost = wand_id(&plan, UMRISS_GARAGE, 1);
    let r = b.c.abstandsplan_pruefen(pid, &plan).await.unwrap();
    for seite in 0..4 {
        let e = ergebnis(&r, wand_id(&plan, UMRISS_GARAGE, seite));
        if e.tiefe_m != Some(0.) {
            b.befund(format!(
                "Abstandsflächen/Grenzgarage: {} erhält Tiefe {:?} statt 0 (Grenzlänge 6,00 m ≤ 9 m; offen: {:?}).",
                e.bezeichnung, e.tiefe_m, e.offen
            ));
        }
    }
    assert!(
        ergebnis(&r, garage_ost)
            .herleitung
            .iter()
            .any(|h| h.starts_with("Grenzanlage"))
    );

    // --- 5. Speichern (Bereichsspeicherung des Fachobjekts) ---
    let f = b
        .speichern(
            ObjektArt::Abstandsflaechen,
            PLAN_ID,
            serde_json::to_value(&plan).unwrap(),
        )
        .await;
    let plan: AbstandsPlan = serde_json::from_value(f.daten.clone()).unwrap();
    assert!(
        plan.recht.geprueft && plan.umfang_geprueft,
        "Bestätigungen beim ersten Speichern erhalten"
    );
    assert!(
        plan.waende
            .iter()
            .filter(|w| w.gebaeude_id == Some(b.haus))
            .all(|w| w.gebaeude == "Wohnhaus")
    );

    // --- 6. Prüfen und von Hand nachrechnen ---
    let r = b.c.abstandsplan_pruefen(pid, &plan).await.unwrap();
    for w in &r.waende {
        eprintln!(
            "  {:<28} L={:>5.2} Wandh={:?} Dach={:?} H={:?} T={:?} {:?} offen={:?}",
            w.bezeichnung,
            w.laenge_m,
            w.wandhoehe_m,
            w.dachanteil_m,
            w.h_m,
            w.tiefe_m,
            w.status,
            w.offen
        );
        for h in &w.herleitung {
            eprintln!("      {h}");
            if h.contains("0000000") || h.contains("9999999") {
                b.befund(format!("Abstandsflächen/Herleitung: ungerundete Gleitkommazahl im Text „{h}“ (mod.rs:868–872 verwendet `{{h}}` statt `hoehen::zahl`)."));
            }
        }
    }
    let h_trauf = traufe - gelaende;
    let h_giebel = h_trauf + giebelhoehe / 3.;
    for seite in 0..4 {
        let e = ergebnis(&r, wand_id(&plan, UMRISS_HAUS, seite));
        let h = if seite % 2 == 0 { h_trauf } else { h_giebel };
        assert!(
            nah(e.h_m.unwrap(), h),
            "{}: H {:?} ≠ {h}",
            e.bezeichnung,
            e.h_m
        );
        let t = (0.4 * h).max(3.);
        assert!(
            nah(e.tiefe_m.unwrap(), t),
            "{}: T {:?} ≠ {t}",
            e.bezeichnung,
            e.tiefe_m
        );
        assert!(nah(t, 3.));
    }
    for seite in 0..4 {
        let e = ergebnis(&r, wand_id(&plan, UMRISS_GARAGE, seite));
        assert_eq!(e.tiefe_m, Some(0.), "{}", e.bezeichnung);
        assert_eq!(
            e.status,
            AbstandsStatus::Eingehalten,
            "{}: {:?}",
            e.bezeichnung,
            e.offen
        );
    }
    assert!(r.ueberdeckungen.is_empty(), "{:?}", r.ueberdeckungen);
    // Westgiebel: 3,00 m Grenzabstand bei 3,00 m Tiefe – die Abstandsfläche reicht
    // genau bis an die Grenze und liegt damit vollständig auf dem Grundstück.
    for seite in 0..4 {
        let e = ergebnis(&r, wand_id(&plan, UMRISS_HAUS, seite));
        if e.status != AbstandsStatus::Eingehalten {
            b.befund(format!(
                "Abstandsflächen: {} {:?} statt eingehalten (offen: {:?})",
                e.bezeichnung, e.status, e.offen
            ));
        }
        assert_eq!(e.ausserhalb_m2, Some(0.));
    }
    let west = ergebnis(&r, wand_id(&plan, UMRISS_HAUS, 3));
    assert!(
        west.herleitung
            .iter()
            .any(|h| h.contains("Berührung von innen")),
        "{:?}",
        west.herleitung
    );
    assert!(r.offen.is_empty(), "{:?}", r.offen);

    // --- 7. Proben: Werden widersprüchliche Eingaben bemerkt? (nicht gespeichert) ---
    let mut probe = plan.clone();
    probe.recht.gebiet = "MK".into();
    if let Ok(pr) = b.c.abstandsplan_pruefen(pid, &probe).await
        && pr
            .waende
            .iter()
            .all(|w| !w.offen.iter().any(|o| o.contains("Baugebiet")))
    {
        b.befund(
            "Abstandsflächen: Baugebiet im Abstandsplan (Freitext „MK“) wird nicht mit der Festsetzung des Projekts (WA, Bebauungsplan Nr. 42) \
             abgeglichen; der Core rechnet still mit 0,25 H statt 0,4 H (service.rs `abstandsplan_pruefen` prüft nur Flurstück-IDs). \
             Vorschlag: Gebiet aus `festgesetzte_werte.baugebiet` vorbelegen und Abweichung als offenen Punkt melden.",
        );
    }
    let mut probe = plan.clone();
    probe.grundstuecke[0].grenze = vec![
        APunkt { x: 0., y: 0. },
        APunkt { x: 20., y: 0. },
        APunkt { x: 20., y: 30. },
        APunkt { x: 0., y: 30. },
    ];
    if let Ok(pr) = b.c.abstandsplan_pruefen(pid, &probe).await
        && pr.offen.is_empty()
        && pr.waende.iter().all(|w| {
            !w.offen
                .iter()
                .any(|o| o.contains("Fläche") || o.contains("Flurstück"))
        })
    {
        b.befund(
            "Abstandsflächen: Grundstücksgrenze mit 600 m² wird für Flurstück 412 (Kataster 620 m²) ohne Hinweis akzeptiert; die Flurstücksfläche \
             aus Grundstücksinformationen wird nicht gegen die Grenzgeometrie geprüft (service.rs:20–29 prüft nur die Existenz der Flurstücks-ID). \
             Vorschlag: Abweichung > 1–2 % als offenen Prüfpunkt ausgeben.",
        );
    }
    let mut probe = plan.clone();
    for u in &mut probe.umrisse {
        u.grundstueck_id = Some(G411);
    }
    let _ = umrisse_einbinden(&mut probe);
    if let Ok(pr) = b.c.abstandsplan_pruefen(pid, &probe).await
        && pr.waende.iter().all(|w| {
            !w.offen
                .iter()
                .any(|o| o.contains("Flurstück") || o.contains("Gebäude"))
        })
    {
        b.info(
            "Probe: Umriss des Wohnhauses (Gebäude auf fs-412) dem Nachbargrundstück 411 zugeordnet – kein Hinweis auf die abweichende \
             Flurstückszuordnung des Gebäudes (nur Konflikte über die Geometrie).",
        );
    }

    // --- 8. Nachweis archivieren, wiederholen, freigeben ---
    let n =
        b.c.nachweis_erstellen(
            pid,
            Uuid::new_v4(),
            "Abstandsflächennachweis EFH Mustermann".into(),
            NachweisEingabe::AbstandProjekt {
                objekt: PLAN_ID,
                plan: Box::new(plan.clone()),
            },
            &b.u,
        )
        .await
        .unwrap();
    assert_eq!(
        b.c.nachweis_wiederholen(pid, n.id).await.unwrap(),
        n.ergebnis
    );
    let ansicht =
        b.c.nachweise(pid)
            .await
            .unwrap()
            .into_iter()
            .find(|a| a.stand.id == n.id)
            .unwrap();
    assert!(
        ansicht.aktuell,
        "frisch archivierter Abstandsnachweis aktuell"
    );
    match b
        .c
        .nachweis_freigeben(
            pid,
            n.id,
            "Abstandsflächen fachlich geprüft (E. Planer)".into(),
            &b.u,
        )
        .await
    {
        Ok(_) => assert!(n.vollstaendig),
        Err(e) => b.befund(format!(
            "Abstandsflächen: Freigabe des Nachweises abgelehnt: „{e}“"
        )),
    }

    // --- 9. Datenfluss Gebäude → Nachweis: Traufe ändern macht den Nachweis veraltet? ---
    let gf =
        b.c.fachobjekte(pid, ObjektArt::Gebaeude)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == b.haus.to_string())
            .unwrap();
    let mut geaendert = gf.daten.clone();
    geaendert["geometrie"]["traufe_m"] = serde_json::json!(4.6);
    b.speichern(ObjektArt::Gebaeude, b.haus, geaendert).await;
    let aktuell =
        b.c.nachweise(pid)
            .await
            .unwrap()
            .into_iter()
            .find(|a| a.stand.id == n.id)
            .unwrap()
            .aktuell;
    if aktuell {
        b.befund(
            "Abstandsflächen: Nach Änderung der Traufhöhe des verknüpften Gebäudes „Wohnhaus“ (4,20 → 4,60 m) bleibt der Abstandsnachweis „aktuell“. \
             Umrisse tragen `gebaeude_id`, die Oberkante ist aber nur eine Kopie; nachweise.rs:556–571 prüft für AbstandProjekt nur Plan-Hash und \
             Quelldokumente, nicht die Gebäudestände. Vorschlag: übernommene Gebäudehöhen (Traufe/First/Gelände) mit Gebäuderevision im Plan führen \
             und bei Abweichung veralten bzw. im Plan einen offenen Punkt „Gebäudehöhen geändert“ melden.",
        );
    }
    b.speichern(ObjektArt::Gebaeude, b.haus, gf.daten.clone())
        .await; // Basisstand wiederherstellen
    b.befund(
        "Abstandsflächen/Datenfluss: Aus Gebäude/Geschosse werden nur Gelände und Traufe angeboten (UI `traufeUebernehmen`/`hoehenUebernehmen`); \
         Giebelhöhe (First − Traufe) und Dachneigung müssen je Giebelwand von Hand eingegeben werden, obwohl `first_m` im Gebäude vorliegt. \
         Das Gebäudemodell (plattform/gebaeude.rs GebaeudeGeometrie) kennt weder Dachform noch Dachneigung noch Grundrissumriss; \
         Umrissecken erhalten auch die Geländehöhe des Gebäudes nicht automatisch. Vorschlag: Dachform/-neigung im Gebäude führen und beim \
         Umriss „Seiten als Wände übernehmen“ Dachart, Giebelhöhe und Eckgelände vorbelegen.",
    );

    // --- 10. PDF (Druckausgabe), Ablage und Exporte ---
    b.neu_laden().await;
    let auftrag = Druckauftrag {
        gestaltung: None,
        aktenzeichen: String::new(),
        datum: Some(stichtag()),
        titel: "Abstandsflächennachweis Neubau EFH Mustermann".into(),
        absender: "Planungsbüro Planer Architekten".into(),
        empfaenger: "Stadt Musterstadt – Untere Bauaufsicht".into(),
        anschreiben: String::new(),
        mit_anschreiben: false,
        inhaltsverzeichnis: true,
        deckblatt: true,
        trennblaetter: false,
        aufgestellt_von: None,
        projekt_version: b.p.version,
        quellen: vec![Druckquelle::Nachweis { id: n.id }],
    };
    let pdf = b.c.druckausgabe(pid, auftrag).await.unwrap();
    let d = lopdf::Document::load_mem(&pdf.pdf).unwrap();
    let seiten: Vec<String> = d
        .get_pages()
        .values()
        .map(|s| String::from_utf8_lossy(&d.get_page_content(*s)).into_owned())
        .collect();
    eprintln!("  PDF Abstandsflächennachweis: {} Seiten", seiten.len());
    assert!(seiten.iter().any(|s| s.contains("Wandabwicklungen")));
    assert!(seiten.last().unwrap().contains("W8"));
    // Text der Berichtsseiten ist im PDF-Strom nicht im Klartext; geprüft mit pdftotext.
    if plan.regeln.giebel.is_some()
        && plan
            .waende
            .iter()
            .any(|w| w.dachart == Dachart::Giebel && w.dachfaktor_manuell.is_none())
    {
        b.befund(
            "Abstandsflächen/PDF: Die Zeile „Dachansatz“ lautet bei jeder geneigten Dachart „<Dachgrundlage> · manueller Faktor Nicht ermittelt“ – \
             auch für Giebelwände mit automatischer Giebelregel (Faktor 1/3) und für Traufwände (Faktor 0). Das liest sich wie ein fehlender Wert. \
             Ursache druckausgabe/abstandsbericht.rs:245–253. Vorschlag: angewandten Faktor und Regelquelle aus dem Ergebnis ausgeben, \
             „manueller Faktor“ nur bei Dachart Manuell bzw. Giebel ohne Giebelregel.",
        );
    }
    if r.waende
        .iter()
        .flat_map(|w| &w.herleitung)
        .any(|h| h.starts_with("T = max(0.4") || h.starts_with("Dachfläche: 3.25"))
    {
        b.befund(
            "Abstandsflächen/Herleitung: Gemischte Dezimalschreibweise im Nachweis – „Dachfläche: 3.25 m × 0 bei 38°“ und „T = max(0.4 × H, 3 m)“ \
             mit Punkt, übrige Zeilen mit Komma (4,5 m; 0,333). Ursache abstandsflaechen/mod.rs:868–872, 994–997, 1054, 1076–1078 formatieren mit \
             `{}` statt `hoehen::zahl`. Ungerundete Werte (z. B. Garagenwandhöhe 2,799999999999997 aus 75,14 − 72,34) würden so ebenfalls \
             ungekürzt erscheinen.",
        );
    }
    if !n.vollstaendig && r.offen.is_empty() && r.waende.iter().all(|w| w.offen.is_empty()) {
        b.befund(
            "Abstandsflächen/PDF: Kopf „UNVOLLSTÄNDIG: Offene Prüfpunkte sind unten ausgewiesen.“, es gibt aber keine offenen Prüfpunkte – einziger \
             Grund ist der Status „Grenzfall – prüfen“ der Westwand (druckausgabe/berichte.rs:241). Vorschlag: Grund der Unvollständigkeit \
             (Grenzfall-Wände) ausdrücklich nennen.",
        );
    }
    let dok =
        b.c.druckpdf_ablegen(
            pid,
            "Abstandsflaechennachweis.pdf",
            Bytes::from(pdf.pdf.clone()),
            Uuid::new_v4(),
            &b.u,
        )
        .await
        .unwrap();
    if let Ok(out) = std::env::var("BEISPIEL_ARTEFAKTE") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            std::path::Path::new(&out).join("Abstandsflaechennachweis.pdf"),
            &pdf.pdf,
        )
        .unwrap();
    }
    let json =
        b.c.abstandsnachweis_export(pid, n.id, "json")
            .await
            .unwrap()
            .1;
    assert!(!json.is_empty());
    let dxf_export =
        b.c.abstandsnachweis_export(pid, n.id, "dxf")
            .await
            .unwrap()
            .1;
    let cad = core_lib::cad::dxf::lesen(&dxf_export).unwrap();
    assert!(cad.linien.iter().filter(|l| l.geschlossen).count() >= 3);
    b.info(format!(
        "Abstandsflächen: Plan {PLAN_ID}, Nachweis {} (vollständig {}, konfliktfrei {}), PDF {} Seiten als Dokument {:?}",
        n.id, n.vollstaendig, r.konfliktfrei, pdf.seiten, dok.document_id
    ));
}
