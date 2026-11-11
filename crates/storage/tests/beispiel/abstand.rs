//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“: Abstandsflächen nach
//! § 6 BauO NRW 2018 (Paket `abstand-nw`) auf dem gezeichneten Weg: Lageplan
//! als PDF (M 1:200), Flurstücke, Gebäudeumrisse und Linien „Abstandsfläche“
//! im Aufmaß des Lageplans, Übernahme der Linien als Wände und der Flurstücke
//! als Grundstücksgrenzen, Satteldach mit Giebelanrechnung, Grenzgarage,
//! Rechts-/Umfangsprüfung, Nachweis, Freigabe, PDF sowie JSON-/DXF-Export.
//!
//! Lage (Meter, x nach Osten, y nach Norden, Ursprung Südwestecke Flst. 412):
//! Flurstück 412 = 20,00 × 31,00 m, Straße Lindenweg an der Nordseite
//! (y = 31). Wohnhaus 11,00 × 9,00 m, 5,00 m hinter der Straßengrenze und
//! 3,00 m von der Westgrenze: x 3–14, y 17–26, Traufseiten Nord/Süd.
//! Grenzgarage 3,50 × 6,00 m an der Ostgrenze: x 16,5–20, y 20–26.
#![allow(dead_code)]
use super::basis::*;
use bytes::Bytes;
use core_lib::{
    abstandsflaechen::*,
    druckausgabe::{Druckauftrag, Druckquelle},
    grundstuecksinformationen::informationen,
    planaufmass::{
        Aufmassstand, Planaufmass, Planbedeutung, Planbindung, Planebene, Plangeometrie,
        Plankalibrierung, Planmarkierung, Planmessgroesse,
    },
    plattform::{ObjektArt, nachweise::NachweisEingabe, objekte::Dokumentbezug},
    projektbereiche::ProjektBereich,
};
use uuid::Uuid;

/// Fachobjekt des Abstandsplans (fest, damit spätere Module darauf verweisen können).
pub const PLAN_ID: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0000_0412);
pub const G412: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0412);
pub const G411: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0411);
pub const G413: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0001_0413);
/// Aufmaß des Lageplans (fest, damit Abstandsplan und Flurstücke darauf verweisen).
pub const LAGEPLAN_ID: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0003_0412);
/// Baukörper für [`wand_id`].
pub const UMRISS_HAUS: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0002_0001);
pub const UMRISS_GARAGE: Uuid = Uuid::from_u128(0x4b5f_0000_0000_0000_0000_0000_0002_0002);
pub const DACHNEIGUNG: f64 = 36.;
/// Wände je Baukörper im Gegenuhrzeigersinn ab Südwestecke: Süd, Ost, Nord, West.
const HAUSWAENDE: [&str; 4] = [
    "A1 Süd (Traufe)",
    "A2 Ost (Giebel)",
    "A3 Nord (Traufe)",
    "A4 West (Giebel)",
];
const GARAGENWAENDE: [&str; 4] = [
    "A5 Garage Süd",
    "A6 Garage Ost (Grenze)",
    "A7 Garage Nord",
    "A8 Garage West",
];

/// Lageplan M 1:200 auf A3: 1 m = 5 mm; die Südwestecke von Flst. 412 liegt
/// bei (150 | 220) mm ab Blattecke links oben.
const LP_X: f64 = 150.;
const LP_Y: f64 = 220.;
const LP_MM: f64 = 5.;
/// Dieselbe Ecke im Koordinatenbezug des Aufmaßes (Meter ab Blattecke, y nach oben).
pub const URSPRUNG: (f64, f64) = (LP_X / LP_MM, -LP_Y / LP_MM);
/// Blattpunkt (mm) eines Lagepunkts in Metern.
fn blatt(x: f64, y: f64) -> (f64, f64) {
    (LP_X + LP_MM * x, LP_Y - LP_MM * y)
}
/// Rechteck in Metern: (x, y, Breite, Tiefe) ab Südwestecke.
const FLST_411: (f64, f64, f64, f64) = (-21.13, 0., 21.13, 31.);
const FLST_412: (f64, f64, f64, f64) = (0., 0., 20., 31.);
const FLST_413: (f64, f64, f64, f64) = (20., 0., 19.29, 31.);
const HAUS: (f64, f64, f64, f64) = (3., 17., 11., 9.);
const GARAGE: (f64, f64, f64, f64) = (16.5, 20., 3.5, 6.);
/// Ecken im Gegenuhrzeigersinn ab Südwestecke.
fn ecken(r: (f64, f64, f64, f64)) -> [(f64, f64); 4] {
    let (x, y, b, t) = r;
    [(x, y), (x + b, y), (x + b, y + t), (x, y + t)]
}

/// Amtlicher Lageplan des Vermessers als Vektor-PDF.
fn lageplan_pdf() -> Vec<u8> {
    let mut z = Zeichnung::default();
    z.stift(0.5, false);
    z.rechteck((10., 10.), (410., 287.));
    z.text(20., 24., 11., "Lageplan zum Bauantrag");
    let rechteck = |z: &mut Zeichnung, r: (f64, f64, f64, f64)| {
        let e = ecken(r);
        z.rechteck(blatt(e[0].0, e[0].1), blatt(e[2].0, e[2].1));
    };
    // Flurstücke
    z.stift(0.35, false);
    for (r, nr) in [(FLST_411, "411"), (FLST_412, "412"), (FLST_413, "413")] {
        rechteck(&mut z, r);
        let m = blatt(r.0 + r.2 / 2. - 1.5, 6.);
        z.text(m.0, m.1, 9., nr);
    }
    let t = blatt(1., 2.);
    z.text(t.0, t.1, 4., "620 m²");
    // Straße Lindenweg (6,00 m) nördlich der Flurstücke
    z.stift(0.25, false);
    z.linie(blatt(-21.13, 37.), blatt(39.29, 37.));
    let t = blatt(4., 33.2);
    z.text(t.0, t.1, 8., "Lindenweg");
    let t = blatt(-20., 31.4);
    z.text(t.0, t.1, 3.5, "OK Straße 72,40 m ü. NHN");
    // Vorhaben
    z.stift(0.7, false);
    rechteck(&mut z, HAUS);
    rechteck(&mut z, GARAGE);
    let t = blatt(4.2, 24.4);
    z.text(t.0, t.1, 6., "Wohnhaus II");
    let t = blatt(4.2, 23.4);
    z.text(
        t.0,
        t.1,
        4.,
        "11,00 × 9,00 · SD 36° · Traufe 76,84 · First 80,09",
    );
    let t = blatt(17.3, 24.6);
    z.text(t.0, t.1, 4., "Garage");
    let t = blatt(17.3, 21.2);
    z.text(t.0, t.1, 3., "OK 75,14");
    // Firstlinie und Maße zu den Grenzen
    z.stift(0.18, true);
    z.linie(blatt(3., 21.5), blatt(14., 21.5));
    z.stift(0.13, false);
    z.linie(blatt(0., 18.), blatt(3., 18.));
    let t = blatt(0.6, 18.3);
    z.text(t.0, t.1, 3.5, "3,00");
    z.linie(blatt(8., 26.), blatt(8., 31.));
    let t = blatt(8.3, 28.2);
    z.text(t.0, t.1, 3.5, "5,00");
    z.linie(blatt(14., 24.), blatt(16.5, 24.));
    let t = blatt(14.4, 24.3);
    z.text(t.0, t.1, 3.5, "2,50");
    let t = blatt(1., 0.6);
    z.text(t.0, t.1, 3.5, "Gelände eben 72,34 m ü. NHN");
    // Nordpfeil
    z.stift(0.35, false);
    z.zug(&[
        (380., 60.),
        (385., 35.),
        (390., 60.),
        (385., 54.),
        (380., 60.),
    ]);
    z.text(383., 32., 8., "N");
    // Schriftfeld
    z.rechteck((300., 245.), (410., 287.));
    for (i, t) in [
        "Neubau Einfamilienhaus Mustermann",
        "Lindenweg 12, 59999 Musterstadt · Flurstück 412",
        "Lageplan zum Bauantrag · Gemarkung Musterstadt, Flur 3",
        "ÖbVI-Vermessung 09/2026 · Plan-Nr. LP-01 · Index a",
        "M 1:200 · A3 · 21.10.2026",
        "Planungsbüro Planer Architekten · E. Planer",
    ]
    .iter()
    .enumerate()
    {
        z.text(303., 252. + i as f64 * 6., if i == 0 { 7. } else { 5. }, t);
    }
    blatt_pdf(z)
}

/// Lageplan ablegen, im Aufmaß Flurstücke, Gebäudeumrisse und Linien
/// „Abstandsfläche“ zeichnen und die Flurstücke mit ihrer Zeichnung verbinden.
pub async fn lageplan(b: &mut Beispiel) -> core_lib::commit::Fachstand {
    let pid = b.p.id;
    let pdf = lageplan_pdf();
    if let Ok(ziel) = std::env::var("BAUPROJEKT_BEISPIEL_ARTEFAKTE") {
        std::fs::create_dir_all(&ziel).unwrap();
        std::fs::write(std::path::Path::new(&ziel).join("LP-01_Lageplan.pdf"), &pdf).unwrap();
    }
    let doc =
        b.c.store_document_kategorisiert(
            pid,
            "LP-01_Lageplan_M200.pdf",
            Bytes::from(pdf),
            Some("lageplan".into()),
            &b.u,
        )
        .await
        .unwrap();
    let dokument = Dokumentbezug {
        dokument_id: doc.document_id,
        version: doc.version,
    };
    b.speichern(
        ObjektArt::Dokumentdaten,
        doc.document_id.0,
        serde_json::json!({"dokument": dokument, "dokumentart": "Lageplan",
            "plannummer": "LP-01", "revisionsindex": "a", "massstab": "1:200",
            "verfasser": "ÖbVI Musterstadt / Erika Planer, Architektin", "datum": "2026-10-21"}),
    )
    .await;
    let (e_flur, e_geb, e_abstand) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let pt = |p: (f64, f64)| {
        let (x, y) = blatt(p.0, p.1);
        [x * MM, y * MM]
    };
    let mut markierungen = vec![];
    let mut m =
        |ebene, name: &str, bedeutung, geometrie, gebaeude: Option<Uuid>, flur: Option<&str>| {
            markierungen.push(Planmarkierung {
                id: Uuid::new_v4(),
                ebene_id: ebene,
                bezeichnung: name.into(),
                bedeutung,
                geometrie,
                grundstueck_id: None,
                gebaeude_id: gebaeude,
                geschoss_id: None,
                flurstueck_id: flur.map(Into::into),
                ausschnitt_id: None,
                hoehenzone_von: None,
                kontur_von: None,
            });
        };
    let flaeche = |r| {
        let e = ecken(r);
        Plangeometrie::Rechteck {
            von: pt(e[3]),
            bis: pt(e[1]),
        }
    };
    for (r, name, id) in [
        (FLST_412, "Flurstück 412", "fs-412"),
        (FLST_411, "Flurstück 411", "fs-411"),
        (FLST_413, "Flurstück 413", "fs-413"),
    ] {
        m(
            e_flur,
            name,
            Planbedeutung::Flurstueck,
            flaeche(r),
            None,
            Some(id),
        );
    }
    for (r, name, gebaeude, waende) in [
        (HAUS, "Wohnhaus", b.haus, HAUSWAENDE),
        (GARAGE, "Garage", b.garage, GARAGENWAENDE),
    ] {
        m(
            e_geb,
            &format!("Umriss {name}"),
            Planbedeutung::Vorhabenumriss,
            flaeche(r),
            Some(gebaeude),
            None,
        );
        let e = ecken(r);
        for (seite, wand) in waende.iter().enumerate() {
            m(
                e_abstand,
                wand,
                Planbedeutung::Abstandsflaeche,
                Plangeometrie::Strecke {
                    von: pt(e[seite]),
                    bis: pt(e[(seite + 1) % 4]),
                },
                Some(gebaeude),
                None,
            );
        }
    }
    let aufmass = Planaufmass {
        bezeichnung: "LP-01 Lageplan".into(),
        dokument: Some(dokument),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Massstab { nenner: 200. }),
        hoehenbezug: None,
        ebenen: [
            (e_flur, "Flurstücke"),
            (e_geb, "Gebäude"),
            (e_abstand, "Abstandsflächen"),
        ]
        .into_iter()
        .map(|(id, n)| Planebene {
            id,
            bezeichnung: n.into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        })
        .collect(),
        markierungen,
        ausschnitte: vec![],
    };
    let f = b
        .speichern(
            ObjektArt::Planaufmass,
            LAGEPLAN_ID,
            serde_json::to_value(&aufmass).unwrap(),
        )
        .await;
    // Flurstücke mit ihrer Zeichnung verbinden: Die Katasterfläche bleibt
    // Rechenwert, die gezeichnete Fläche steht als Gegenprobe daneben.
    let mut infos = informationen(&b.p.planungsrecht);
    for fl in &mut infos.flurstuecke {
        let markierung = aufmass
            .markierungen
            .iter()
            .find(|m| m.flurstueck_id.as_deref() == Some(fl.id.as_str()))
            .expect("Flurstück gezeichnet");
        fl.planbindung = Some(Planbindung {
            aufmass_id: LAGEPLAN_ID,
            revision: f.revision,
            markierung_id: markierung.id,
            groesse: Planmessgroesse::Flaeche,
            ziel: fl.planziel(),
        });
    }
    let infos = b.c.planaufmass_flurstuecke(pid, infos).await.unwrap();
    for fl in &infos.flurstuecke {
        let d = fl.flaechenabweichung_m2().expect("gezeichnete Fläche");
        assert!(d.abs() < 0.05, "Flurstück {}: Abweichung {d}", fl.nummer);
    }
    let mut r = b.p.planungsrecht.clone();
    r.grundstuecksinformationen = Some(infos);
    b.bereich(ProjektBereich::Grundstuecksinformationen, r)
        .await;
    f
}
fn nah(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}
fn wandname(umriss: Uuid, seite: usize) -> &'static str {
    if umriss == UMRISS_HAUS {
        HAUSWAENDE[seite]
    } else {
        GARAGENWAENDE[seite]
    }
}
fn wand_mut(p: &mut AbstandsPlan, umriss: Uuid, seite: usize) -> &mut AbstandsWand {
    p.waende
        .iter_mut()
        .find(|w| w.bezeichnung == wandname(umriss, seite))
        .expect("Wand aus dem Lageplan")
}
pub fn wand_id(p: &AbstandsPlan, umriss: Uuid, seite: usize) -> Uuid {
    p.waende
        .iter()
        .find(|w| w.bezeichnung == wandname(umriss, seite))
        .expect("Wand aus dem Lageplan")
        .id
}
fn ergebnis(r: &AbstandsErgebnis, id: Uuid) -> &WandErgebnis {
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
    let dach =
        b.c.gebaeudegrundlagen(pid, b.haus)
            .await
            .unwrap()
            .dach
            .expect("Dachform Wohnhaus");
    if dach.form != core_lib::plattform::gebaeude::Dachform::Satteldach
        || dach.neigung_grad != Some(DACHNEIGUNG)
        || !dach.dachhoehe_m.is_some_and(|h| nah(h, giebelhoehe))
    {
        b.befund(format!("Dach aus Gebäude/Geschosse: {dach:?}"));
    }
    // Plausibilität Satteldach: halbe Gebäudetiefe 4,50 m × tan 36° = 3,27 m (First − Traufe 3,25 m).
    let erwartet = 4.5 * DACHNEIGUNG.to_radians().tan();
    if (erwartet - giebelhoehe).abs() > 0.1 {
        b.befund(format!(
            "Abstandsflächen/Gebäude: Giebelhöhe First − Traufe = {giebelhoehe:.2} m passt nicht zur Dachneigung {DACHNEIGUNG}° bei 9,00 m Giebelbreite \
             (erwartet 4,50 × tan 36° = {erwartet:.2} m, entspricht {:.1}°). Der Core prüft Giebelhöhe, beide Dachneigungen und Wandlänge nicht \
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
        b.info("Garage hat in Gebäude/Geschosse keinen Nullpunkt/keine Wandhöhe (Basis); Oberkante 2,80 m über Gelände wird an den Garagenwänden von Hand erfasst.");
    }

    // --- 2. Lageplan mit Flurstücken, Umrissen und Linien „Abstandsfläche“ ---
    let vorhanden =
        b.c.fachobjekte(pid, ObjektArt::Planaufmass)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == LAGEPLAN_ID.to_string());
    let lp = match vorhanden {
        Some(f) => f,
        None => lageplan(b).await,
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
    // Grenzen kommen aus den gezeichneten Flurstücken des Lageplans.
    let grundstueck = |id, name: &str, fs: &str| AbstandsGrundstueck {
        id,
        bezeichnung: name.into(),
        flurstueck_ids: vec![fs.into()],
        grenze: vec![],
        innenringe: vec![],
        grundlage: String::new(),
    };
    let plan = AbstandsPlan {
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
            pruefvermerk: "Geprüft E. Planer 21.10.2026: Traufwände H = 76,84 − 72,34 = 4,50 m (Dach 36° ≤ 45° nicht angerechnet), 0,4 H = 1,80 m < 3,00 m → 3,00 m; \
                Giebelwände H = 4,50 + 3,25/3 = 5,58 m, 0,4 H = 2,23 m → 3,00 m; Grenzgarage § 6 Abs. 8 Satz 1 Nr. 1: Wandhöhe 2,80 m ≤ 3 m, Grenzlänge 6,00 m ≤ 9 m"
                .into(),
        },
        hoehenbezug: format!(
            "m ü. NHN (DHHN2016); ±0,00 = OK FFB EG = {NULLPUNKT_NHN:.2} m ü. NHN; maßgebliche Geländeoberfläche eben {gelaende:.2} m ü. NHN (natürliches Gelände, keine Abgrabungen)"
        ),
        koordinatenbezug: String::new(), // aus dem Lageplan
        grundstuecke: vec![
            grundstueck(G412, "Baugrundstück Flst. 412 (Bauherr Mustermann)", "fs-412"),
            grundstueck(G411, "Nachbar Flst. 411 (Nachbar, West)", "fs-411"),
            grundstueck(G413, "Nachbar Flst. 413 (Beispiel, Ost)", "fs-413"),
        ],
        waende: vec![],
        flaechen: vec![],
        quellen: vec![],
        umfang_geprueft: true,
        umfang_vermerk: "Alle Außenwände von Wohnhaus (4) und Garage (4) als Linien im Lageplan LP-01; Gelände eben, keine Vorbauten > 1,50 m, \
            keine Gauben, Dachüberstand 0,50 m unter § 6 Abs. 6; Garage ohne Aufenthaltsraum geprüft"
            .into(),
        umrisse: vec![],
        regelwerk: None,
    };
    // Linien als Wände, Flurstücke als Grenzen übernehmen (wie „Linien übernehmen“).
    let u =
        b.c.abstandslinien_uebernehmen(
            pid,
            plan,
            Aufmassstand {
                aufmass_id: LAGEPLAN_ID,
                revision: lp.revision,
            },
        )
        .await
        .unwrap();
    for h in &u.hinweise {
        b.info(format!("Linienübernahme: {h}"));
    }
    let mut plan = u.plan;
    plan.recht.fassung = plan.regeln.rechtsstand.clone();
    plan.recht.geprueft = true;
    plan.umfang_geprueft = true;
    assert_eq!(plan.waende.len(), 8);
    assert!(
        plan.koordinatenbezug
            .starts_with("Lageplan „LP-01 Lageplan“")
    );
    assert!(nah(flaeche(&plan.grundstuecke[0].grenze), GRUNDSTUECK_M2));
    assert!(plan.grundstuecke.iter().all(|g| g.grenze.len() == 4));
    // Seiten im Gegenuhrzeigersinn ab Südwestecke: 0 Süd, 1 Ost, 2 Nord, 3 West.
    for seite in 0..4 {
        for (umriss, gebaeude, oberkante) in [
            (UMRISS_HAUS, b.haus, traufe),
            (UMRISS_GARAGE, b.garage, gelaende + 2.8),
        ] {
            let w = wand_mut(&mut plan, umriss, seite);
            assert_eq!(w.gebaeude_id, Some(gebaeude));
            // Außenseite aus dem Gebäudeumriss im Lageplan.
            assert_eq!(w.aussen, Aussenrichtung::Rechts, "{}", w.bezeichnung);
            w.grundstueck_id = Some(G412);
            w.gelaende_a_m = Some(gelaende);
            w.gelaende_b_m = Some(gelaende);
            w.abschluss_a_m = Some(oberkante);
            w.abschluss_b_m = Some(oberkante);
        }
    }
    let namen = ["Süd", "Ost", "Nord", "West"];
    for (seite, name) in namen.iter().enumerate() {
        let traufseite = seite % 2 == 0;
        let w = wand_mut(&mut plan, UMRISS_HAUS, seite);
        assert!(w.bezeichnung.contains(name));
        // Dachhöhe und Neigung aus Gebäude/Geschosse (wie „Höhen übernehmen“).
        w.dachart = if traufseite {
            Dachart::Dachflaeche
        } else {
            Dachart::Giebel
        };
        w.dachhoehe_m = dach.dachhoehe_m;
        w.dachneigung_grad = dach.neigung_grad;
        if !traufseite {
            w.dachneigung_b_grad = dach.neigung_grad;
        }
        w.dachgrundlage = format!("Gebäude Wohnhaus: {}", dach.herleitung);
    }
    for (seite, name) in namen.iter().enumerate() {
        let w = wand_mut(&mut plan, UMRISS_GARAGE, seite);
        assert!(w.bezeichnung.contains(name));
        w.dachart = Dachart::Flach;
        w.dachgrundlage = "Flachdach, OK Attika 2,80 m über Gelände (Entwurf 10/2026)".into();
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
    let pr = b.c.abstandsplan_pruefen(pid, &probe).await.unwrap();
    assert!(
        pr.offen
            .iter()
            .any(|o| o.contains("Festsetzung des Projekts (WA)")),
        "{:?}",
        pr.offen
    );
    assert!(!pr.vollstaendig);
    let mut probe = plan.clone();
    probe
        .planbindungen
        .retain(|b| b.ziel != format!("grundstuecke.{G412}.grenze"));
    let (ox, oy) = URSPRUNG;
    probe.grundstuecke[0].grenze = [(0., 0.), (20., 0.), (20., 30.), (0., 30.)]
        .into_iter()
        .map(|(x, y)| APunkt {
            x: ox + x,
            y: oy + y,
        })
        .collect();
    let pr = b.c.abstandsplan_pruefen(pid, &probe).await.unwrap();
    assert!(
        pr.offen.iter().any(|o| o.contains("Katasterfläche 620 m²")),
        "{:?}",
        pr.offen
    );

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
    let gespeichert = b.speichern(ObjektArt::Gebaeude, b.haus, geaendert).await;
    if gespeichert.daten["geometrie"]["traufe_m"] == serde_json::json!(4.6) {
        let aktuell =
            b.c.nachweise(pid)
                .await
                .unwrap()
                .into_iter()
                .find(|a| a.stand.id == n.id)
                .unwrap()
                .aktuell;
        assert!(
            !aktuell,
            "Abstandsnachweis veraltet nach Änderung der Gebäudehöhen"
        );
    } else {
        // Nach dem PDF-Aufmaß ist die Traufe an die Planmarkierung gebunden.
        b.info("Probe Gebäudehöhe übersprungen: Traufe ist an das PDF-Aufmaß gebunden.");
    }
    b.speichern(ObjektArt::Gebaeude, b.haus, gf.daten.clone())
        .await; // Basisstand wiederherstellen
    let aktuell =
        b.c.nachweise(pid)
            .await
            .unwrap()
            .into_iter()
            .find(|a| a.stand.id == n.id)
            .unwrap()
            .aktuell;
    assert!(
        aktuell,
        "mit den ursprünglichen Gebäudehöhen wieder aktuell"
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
    if r.waende.iter().flat_map(|w| &w.herleitung).any(|h| {
        h.contains("0000000")
            || h.contains("9999999")
            || h.starts_with("T = max(0.")
            || h.starts_with("Dachfläche: 3.")
    }) {
        b.befund("Abstandsflächen/Herleitung: Dezimalschreibweise uneinheitlich oder ungerundet.");
    }
    if !n.vollstaendig && r.offen.is_empty() && r.waende.iter().all(|w| w.offen.is_empty()) {
        b.befund(
            "Abstandsflächen/PDF: Kopf „UNVOLLSTÄNDIG: Offene Prüfpunkte sind unten ausgewiesen.“, es gibt aber keine offenen Prüfpunkte – einziger \
             Grund ist der Status „Grenzfall – prüfen“ der Westwand (druckausgabe/berichte.rs:241). Vorschlag: Grund der Unvollständigkeit \
             (Grenzfall-Wände) ausdrücklich nennen.",
        );
    }
    // Abstandsflächenplan auf dem Blatt des Lageplans.
    let flaechenplan = b.c.abstandsflaechenplan(pid, &plan, &r).await.unwrap();
    assert!(flaechenplan.starts_with(b"%PDF"));
    if let Ok(out) = std::env::var("BEISPIEL_ARTEFAKTE") {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            std::path::Path::new(&out).join("Abstandsflaechenplan.pdf"),
            &flaechenplan,
        )
        .unwrap();
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
