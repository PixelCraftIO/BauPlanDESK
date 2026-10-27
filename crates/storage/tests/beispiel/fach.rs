//! Beispielprojekt „Neubau Einfamilienhaus Mustermann“: PDF-Aufmaß, Raumbuch,
//! alle acht Fachberechnungen, Nachweise (Fach und Maß der Nutzung) und
//! Druckausgabe – ausschließlich über die öffentlichen Core-Dienste.
//!
//! Der Plan (A3, M 1:100) wird maßstäblich als Vektor-PDF erzeugt, damit die im
//! Aufmaß gemessenen Werte mit den Gebäudedaten des Basismoduls übereinstimmen
//! (11,00 × 9,00 m, ±0,00 = 72,64 m ü. NHN, Traufe +4,20, First +7,45).
#![allow(dead_code)]
use super::basis::*;
use bytes::Bytes;
use core_lib::{
    berechnungen::{BerechnungsArt, FachErgebnis, FachPlan, Uebernahme, vorlagen},
    druckausgabe::{Druckauftrag, Druckquelle},
    planaufmass::*,
    plattform::{
        ObjektArt,
        nachweise::{NachweisEingabe, NutzungsArt},
        objekte::{Dokumentbezug, Gebaeude, Raumbuch},
    },
    projektbereiche::ProjektBereich,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Plan-PDF (A3 quer, 420 × 297 mm, M 1:100 → 1 m = 10 mm)
// ---------------------------------------------------------------------------

const MM: f64 = 72.0 / 25.4;
const BLATT_B: f64 = 420.;
const BLATT_H: f64 = 297.;
/// Außenecke (links oben) der Grundrisse in mm, y nach unten.
const O_KG: (f64, f64) = (25., 35.);
const O_EG: (f64, f64) = (160., 35.);
const O_DG: (f64, f64) = (25., 170.);
/// Garage im Grundriss EG (Außenecke links oben, 3,50 × 6,00 m).
const O_GA: (f64, f64) = (285., 35.);
/// Schnitt A-A: linke Außenkante x, Linie ±0,00 y (mm).
const S_X: f64 = 175.;
const S_NULL: f64 = 235.;
/// Wandstärke außen 0,30 m.
const WAND: f64 = 0.30;

/// Seitenpunkt des Aufmaßes (PDF-Punkte ab links oben, y nach unten).
fn pt(x: f64, y: f64) -> Planpunkt {
    [x * MM, y * MM]
}
/// Schnitt: y-Position (mm) einer relativen Höhe über ±0,00.
fn sy(h: f64) -> f64 {
    S_NULL - h * 10.
}
/// Grundriss: Punkt (mm) aus Innenmaß in m ab innerer Ecke.
fn innen(o: (f64, f64), x: f64, y: f64) -> (f64, f64) {
    (o.0 + (WAND + x) * 10., o.1 + (WAND + y) * 10.)
}

fn pdf_text(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                o.push('\\');
                o.push(c);
            }
            'ä' => o.push_str("\\344"),
            'ö' => o.push_str("\\366"),
            'ü' => o.push_str("\\374"),
            'Ä' => o.push_str("\\304"),
            'Ö' => o.push_str("\\326"),
            'Ü' => o.push_str("\\334"),
            'ß' => o.push_str("\\337"),
            '±' => o.push_str("\\261"),
            '²' => o.push_str("\\262"),
            '°' => o.push_str("\\260"),
            '−' | '–' => o.push('-'),
            '·' => o.push_str("\\267"),
            c if c.is_ascii() => o.push(c),
            _ => o.push('?'),
        }
    }
    o
}

#[derive(Default)]
struct Zeichnung {
    ops: String,
}
impl Zeichnung {
    fn y(y: f64) -> f64 {
        (BLATT_H - y) * MM
    }
    fn stift(&mut self, breite_mm: f64, gestrichelt: bool) {
        let d = if gestrichelt { "[4 2] 0 d" } else { "[] 0 d" };
        self.ops.push_str(&format!("{:.3} w {d}\n", breite_mm * MM));
    }
    fn linie(&mut self, a: (f64, f64), b: (f64, f64)) {
        self.ops.push_str(&format!(
            "{:.3} {:.3} m {:.3} {:.3} l S\n",
            a.0 * MM,
            Self::y(a.1),
            b.0 * MM,
            Self::y(b.1)
        ));
    }
    fn zug(&mut self, p: &[(f64, f64)]) {
        for w in p.windows(2) {
            self.linie(w[0], w[1]);
        }
    }
    fn rechteck(&mut self, a: (f64, f64), b: (f64, f64)) {
        self.ops.push_str(&format!(
            "{:.3} {:.3} {:.3} {:.3} re S\n",
            a.0.min(b.0) * MM,
            Self::y(a.1.max(b.1)),
            (b.0 - a.0).abs() * MM,
            (b.1 - a.1).abs() * MM
        ));
    }
    fn text(&mut self, x: f64, y: f64, groesse: f64, s: &str) {
        self.ops.push_str(&format!(
            "BT /F1 {groesse} Tf {:.3} {:.3} Td ({}) Tj ET\n",
            x * MM,
            Self::y(y),
            pdf_text(s)
        ));
    }
}

/// Grundriss: Außen- und Innenkontur, Räume (Innenmaß m) mit Beschriftung.
fn grundriss(z: &mut Zeichnung, o: (f64, f64), titel: &str, raeume: &[Planraum]) {
    z.stift(0.5, false);
    z.rechteck(o, (o.0 + 110., o.1 + 90.));
    z.rechteck(innen(o, 0., 0.), innen(o, 10.4, 8.4));
    z.stift(0.18, false);
    for r in raeume {
        let a = innen(o, r.x0, r.y0);
        let b = innen(o, r.x1, r.y1);
        z.rechteck(a, b);
        let m = ((a.0 + b.0) / 2. - 7., (a.1 + b.1) / 2.);
        z.text(m.0, m.1, 5., r.name);
        z.text(
            m.0,
            m.1 + 3.,
            4.,
            &format!("{:.2} m²", (r.x1 - r.x0) * (r.y1 - r.y0)).replace('.', ","),
        );
    }
    z.text(o.0, o.1 - 6., 9., titel);
    // Bemaßung Außenmaß
    z.stift(0.13, false);
    z.linie((o.0, o.1 + 95.), (o.0 + 110., o.1 + 95.));
    z.text(o.0 + 48., o.1 + 99., 5., "11,00");
    z.linie((o.0 - 5., o.1), (o.0 - 5., o.1 + 90.));
    z.text(o.0 - 13., o.1 + 46., 5., "9,00");
}

/// Raum im Innenmaß (m ab innerer Ecke) mit Planbedeutung.
#[derive(Clone)]
struct Planraum {
    name: &'static str,
    bedeutung: Planbedeutung,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    /// Lichte Höhe für das Raumbuch (Dachschräge: mittlere Höhe).
    hoehe: f64,
    /// WoFlV-Raumtyp; `None` = nicht Teil der Wohnflächenberechnung.
    woflv: Option<&'static str>,
    /// DIN-277-Flächenart.
    din: &'static str,
}
#[allow(clippy::too_many_arguments)]
fn raum(
    name: &'static str,
    bedeutung: Planbedeutung,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    hoehe: f64,
    woflv: Option<&'static str>,
    din: &'static str,
) -> Planraum {
    Planraum {
        name,
        bedeutung,
        x0,
        y0,
        x1,
        y1,
        hoehe,
        woflv,
        din,
    }
}
fn flaeche(r: &Planraum) -> f64 {
    (r.x1 - r.x0) * (r.y1 - r.y0)
}
fn raeume_kg() -> Vec<Planraum> {
    use Planbedeutung::*;
    vec![
        raum("Hobbyraum", Nuf, 0., 0., 5.0, 4.6, 2.4, None, "nuf"),
        raum("Vorrat", Nuf, 5.15, 0., 7.5, 4.6, 2.4, None, "nuf"),
        raum(
            "Technik / Hausanschluss",
            Tf,
            7.65,
            0.,
            10.4,
            4.6,
            2.4,
            None,
            "tf",
        ),
        raum("Flur / Treppe KG", Vf, 0., 4.75, 10.4, 6.0, 2.4, None, "vf"),
        raum(
            "Waschen / Trocknen",
            Nuf,
            0.,
            6.15,
            5.0,
            8.4,
            2.4,
            None,
            "nuf",
        ),
        raum("Werkstatt", Nuf, 5.15, 6.15, 10.4, 8.4, 2.4, None, "nuf"),
    ]
}
fn raeume_eg() -> Vec<Planraum> {
    use Planbedeutung::*;
    vec![
        raum(
            "Wohnen / Essen",
            Aufenthaltsraum,
            0.,
            0.,
            6.0,
            5.2,
            2.6,
            Some("wohnraum"),
            "nuf",
        ),
        raum(
            "Küche",
            Aufenthaltsraum,
            6.15,
            0.,
            10.4,
            3.8,
            2.6,
            Some("wohnraum"),
            "nuf",
        ),
        raum(
            "HWR",
            Nuf,
            6.15,
            3.95,
            10.4,
            5.4,
            2.6,
            Some("wohnraum"),
            "nuf",
        ),
        raum("WC", Nuf, 0., 5.55, 1.5, 8.4, 2.6, Some("wohnraum"), "nuf"),
        raum(
            "Diele mit Treppe",
            Vf,
            1.65,
            5.35,
            6.0,
            8.4,
            2.6,
            Some("wohnraum"),
            "vf",
        ),
        raum(
            "Arbeiten",
            Aufenthaltsraum,
            6.15,
            5.55,
            10.4,
            8.4,
            2.6,
            Some("wohnraum"),
            "nuf",
        ),
    ]
}
/// Dachgeschoss: Räume an den Traufseiten sind in den vollen Bereich (lichte
/// Höhe ≥ 2,00 m) und den Schrägenstreifen 1,00–2,00 m (1,10 m breit) geteilt.
fn raeume_dg() -> Vec<Planraum> {
    use Planbedeutung::*;
    let w = Some("wohnraum");
    vec![
        raum(
            "Schlafen",
            Aufenthaltsraum,
            0.,
            1.1,
            4.0,
            3.7,
            2.3,
            w,
            "nuf",
        ),
        raum(
            "Schlafen – Schräge 1–2 m",
            Aufenthaltsraum,
            0.,
            0.,
            4.0,
            1.1,
            1.6,
            w,
            "nuf",
        ),
        raum("Bad", Nuf, 4.15, 1.1, 6.6, 3.7, 2.3, w, "nuf"),
        raum(
            "Bad – Schräge 1–2 m",
            Nuf,
            4.15,
            0.,
            6.6,
            1.1,
            1.6,
            w,
            "nuf",
        ),
        raum(
            "Kind 1",
            Aufenthaltsraum,
            6.75,
            1.1,
            10.4,
            3.7,
            2.3,
            w,
            "nuf",
        ),
        raum(
            "Kind 1 – Schräge 1–2 m",
            Aufenthaltsraum,
            6.75,
            0.,
            10.4,
            1.1,
            1.6,
            w,
            "nuf",
        ),
        raum("Flur DG", Vf, 2.0, 3.85, 10.4, 5.0, 2.3, w, "vf"),
        raum("Kind 2", Aufenthaltsraum, 0., 5.15, 5.1, 7.3, 2.3, w, "nuf"),
        raum(
            "Kind 2 – Schräge 1–2 m",
            Aufenthaltsraum,
            0.,
            7.3,
            5.1,
            8.4,
            1.6,
            w,
            "nuf",
        ),
        raum(
            "Gast / Arbeiten",
            Aufenthaltsraum,
            5.25,
            5.15,
            10.4,
            7.3,
            2.3,
            w,
            "nuf",
        ),
        raum(
            "Gast / Arbeiten – Schräge 1–2 m",
            Aufenthaltsraum,
            5.25,
            7.3,
            10.4,
            8.4,
            1.6,
            w,
            "nuf",
        ),
    ]
}
/// Luftraum der Treppe im DG (keine Grundfläche nach DIN 277 / WoFlV).
const LUFTRAUM_DG: (f64, f64, f64, f64) = (0., 3.85, 1.85, 5.0);

fn plan_pdf() -> Vec<u8> {
    use lopdf::{Document, Stream, dictionary};
    let mut z = Zeichnung::default();
    // Rahmen
    z.stift(0.5, false);
    z.rechteck((10., 10.), (410., 287.));
    grundriss(&mut z, O_KG, "Grundriss Kellergeschoss", &raeume_kg());
    grundriss(&mut z, O_EG, "Grundriss Erdgeschoss", &raeume_eg());
    grundriss(&mut z, O_DG, "Grundriss Dachgeschoss", &raeume_dg());
    // EG: Garage, Dachüberstände (gestrichelt), Schnittführung A-A
    z.stift(0.5, false);
    z.rechteck(O_GA, (O_GA.0 + 35., O_GA.1 + 60.));
    z.text(O_GA.0 + 5., O_GA.1 + 30., 6., "Garage");
    z.text(O_GA.0 + 5., O_GA.1 + 34., 4., "21,00 m²");
    z.stift(0.18, true);
    z.rechteck((O_EG.0 - 3., O_EG.1 - 5.), (O_EG.0 + 113., O_EG.1 + 95.));
    z.rechteck((O_GA.0 - 1., O_GA.1 - 1.), (O_GA.0 + 36., O_GA.1 + 61.));
    z.text(
        O_EG.0 + 70.,
        O_EG.1 - 6.5,
        4.,
        "Dachüberstand Traufe 0,50 / Ortgang 0,30",
    );
    z.stift(0.35, false);
    z.linie((O_EG.0 + 55., O_EG.1 - 12.), (O_EG.0 + 55., O_EG.1 + 102.));
    z.text(O_EG.0 + 56., O_EG.1 - 12., 7., "A");
    z.text(O_EG.0 + 56., O_EG.1 + 104., 7., "A");
    // DG: Linien der lichten Höhe 2,00 m und 2,30 m, Luftraum Treppe
    z.stift(0.13, true);
    for y in [1.1, 7.3] {
        z.linie(innen(O_DG, 0., y), innen(O_DG, 10.4, y));
    }
    for y in [1.55, 6.85] {
        z.linie(innen(O_DG, 0., y), innen(O_DG, 10.4, y));
    }
    z.text(
        innen(O_DG, 0., 1.55).0 + 72.,
        innen(O_DG, 0., 1.55).1 - 0.5,
        3.5,
        "h = 2,30",
    );
    z.text(
        innen(O_DG, 0., 1.1).0 + 72.,
        innen(O_DG, 0., 1.1).1 - 0.5,
        3.5,
        "h = 2,00",
    );
    let (a, b) = (
        innen(O_DG, LUFTRAUM_DG.0, LUFTRAUM_DG.1),
        innen(O_DG, LUFTRAUM_DG.2, LUFTRAUM_DG.3),
    );
    z.stift(0.18, false);
    z.rechteck(a, b);
    z.linie(a, b);
    z.text(a.0 + 1., a.1 + 7., 3.5, "Luftraum");
    // Schnitt A-A (Querschnitt über 9,00 m, Traufen an den Längsseiten)
    let xl = S_X;
    let xr = S_X + 90.;
    let xm = S_X + 45.;
    z.text(S_X, 150. + 6., 9., "Schnitt A-A");
    z.stift(0.5, false);
    // Außenwände KG bis Traufe
    for x in [xl, xr - 3.] {
        z.rechteck((x, sy(-3.10)), (x + 3., sy(4.20)));
    }
    // Bodenplatte, Decken
    z.rechteck((xl, sy(-3.10)), (xr, sy(-2.80)));
    z.rechteck((xl + 3., sy(-0.40)), (xr - 3., sy(-0.10)));
    z.rechteck((xl + 3., sy(2.60)), (xr - 3., sy(2.85)));
    // Dachhaut (Oberkante) mit Traufüberstand 0,50 m, Unterseite, Kehlbalkenlage
    let neig = (7.45 - 4.20) / 4.5;
    z.zug(&[
        (xl - 5., sy(4.20 - 0.5 * neig)),
        (xm, sy(7.45)),
        (xr + 5., sy(4.20 - 0.5 * neig)),
    ]);
    z.stift(0.25, false);
    z.zug(&[
        (xl, sy(4.20 - 0.37)),
        (xm, sy(7.45 - 0.37)),
        (xr, sy(4.20 - 0.37)),
    ]);
    let kehl_dx = (7.45 - 0.37 - 5.35) / neig * 10.;
    z.rechteck((xm - kehl_dx, sy(5.35)), (xm + kehl_dx, sy(5.15)));
    // Gelände und Straßenoberkante
    z.stift(0.35, false);
    z.linie((160., sy(-0.30)), (xl, sy(-0.30)));
    z.linie((xr, sy(-0.30)), (290., sy(-0.30)));
    z.stift(0.13, true);
    z.linie((160., sy(-0.24)), (170., sy(-0.24)));
    z.text(152., sy(-0.24) + 4., 3.5, "OK Straße 72,40 m ü. NHN");
    // Bezugslinie ±0,00
    z.stift(0.18, true);
    z.linie((168., S_NULL), (285., S_NULL));
    z.text(268., S_NULL - 1., 4., "±0,00 = 72,64 m ü. NHN");
    // Höhenkoten
    for (h, t) in [
        (7.45, "First +7,45 (80,09)"),
        (4.20, "Traufe +4,20 (76,84)"),
        (5.35, "OK Decke DG +5,35"),
        (2.85, "OKFF DG +2,85"),
        (0.0, "OKFF EG ±0,00"),
        (-0.30, "Gelände -0,30 (72,34)"),
        (-2.80, "OKFF KG -2,80"),
        (-3.10, "Fundamentsohle -3,10"),
    ] {
        z.text(xr + 8., sy(h) + 1., 3.5, t);
    }
    z.text(xl + 10., sy(1.3), 5., "Wohnen");
    z.text(xl + 10., sy(4.0), 5., "Schlafen");
    z.text(xl + 10., sy(-1.6), 5., "Hobby");
    // Schriftfeld
    z.stift(0.35, false);
    z.rechteck((300., 245.), (410., 287.));
    for (i, t) in [
        "Neubau Einfamilienhaus Mustermann",
        "Lindenweg 12, 59999 Musterstadt · Flurstück 412",
        "Grundrisse KG / EG / DG und Schnitt A-A",
        "Entwurfsplanung · Plan-Nr. EP-01 · Index a",
        "M 1:100 · A3 · 21.10.2026",
        "Planungsbüro Planer Architekten · E. Planer",
    ]
    .iter()
    .enumerate()
    {
        z.text(303., 252. + i as f64 * 6., if i == 0 { 7. } else { 5. }, t);
    }

    let mut d = Document::with_version("1.7");
    let pages = d.new_object_id();
    let font = d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let c = d.add_object(Stream::new(dictionary! {}, z.ops.into_bytes()));
    let page = d.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,
    "MediaBox"=>vec![0.into(),0.into(),(BLATT_B*MM).into(),(BLATT_H*MM).into()],"Contents"=>c});
    d.objects.insert(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1,
        "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}}
        .into(),
    );
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    d.trailer.set("Root", cat);
    let mut bytes = Vec::new();
    d.save_to(&mut bytes).unwrap();
    bytes
}

// ---------------------------------------------------------------------------
// Hilfen
// ---------------------------------------------------------------------------

/// Toleranz 1 mm bzw. 0,001 m²: Markierungen liegen auf 1e-4 pt genau.
fn nahe(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}
fn vorlage(art: BerechnungsArt) -> FachPlan {
    vorlagen()
        .unwrap()
        .into_iter()
        .find(|v| v.plan.modul == art)
        .unwrap()
        .plan
}
fn kennzahl(r: &FachErgebnis, id: &str) -> Option<f64> {
    r.kennzahlen
        .iter()
        .find(|k| k.id == id)
        .and_then(|k| k.wert)
}
fn regel(p: &mut FachPlan, quelle: &str, vermerk: &str) {
    p.regelkontext.quelle = quelle.into();
    p.regelkontext.pruefvermerk = vermerk.into();
    p.regelkontext.anwendbarkeit_bestaetigt = true;
}

/// Prüfen, mit bestätigter Anwendbarkeit speichern (ggf. zweiter Schritt, weil
/// eine geänderte Berechnung die Bestätigung zurücksetzt) und erneut prüfen.
async fn fach_sichern(b: &mut Beispiel, id: Uuid, plan: FachPlan) -> (FachPlan, FachErgebnis) {
    let art = plan.modul;
    let (mut kanonisch, _) =
        b.c.berechnung_pruefen(b.p.id, plan)
            .await
            .unwrap_or_else(|e| panic!("{art:?} prüfen: {e}"));
    kanonisch.regelkontext.anwendbarkeit_bestaetigt = true;
    let f = b
        .speichern(art.objekt(), id, serde_json::to_value(&kanonisch).unwrap())
        .await;
    let mut gespeichert: FachPlan = serde_json::from_value(f.daten).unwrap();
    if !gespeichert.regelkontext.anwendbarkeit_bestaetigt {
        gespeichert.regelkontext.anwendbarkeit_bestaetigt = true;
        let f = b
            .speichern(
                art.objekt(),
                id,
                serde_json::to_value(&gespeichert).unwrap(),
            )
            .await;
        gespeichert = serde_json::from_value(f.daten).unwrap();
    }
    let (geprueft, r) =
        b.c.berechnung_pruefen(b.p.id, gespeichert.clone())
            .await
            .unwrap();
    assert_eq!(
        geprueft, gespeichert,
        "{art:?}: gespeicherter Stand ist kanonisch"
    );
    for o in &r.offen {
        b.info(format!("{}: offen – {o}", art.titel()));
    }
    (gespeichert, r)
}

async fn fach_nachweis(
    b: &mut Beispiel,
    nachweise: &mut Vec<(String, Uuid)>,
    id: Uuid,
    plan: &FachPlan,
) -> Uuid {
    let n =
        b.c.nachweis_erstellen(
            b.p.id,
            Uuid::new_v4(),
            plan.bezeichnung.clone(),
            NachweisEingabe::Fach {
                objekt: id,
                plan: Box::new(plan.clone()),
            },
            &b.u,
        )
        .await
        .unwrap_or_else(|e| panic!("Nachweis {}: {e}", plan.bezeichnung));
    if !n.vollstaendig {
        b.befund(format!(
            "Nachweis „{}“ ist nicht vollständig archiviert",
            plan.bezeichnung
        ));
    }
    nachweise.push((plan.bezeichnung.clone(), n.id));
    n.id
}

/// Gebäude nach Änderungen erneut in GRZ/GFZ einbinden (wie beim Öffnen von
/// GRZ/GFZ) und die Grundstückseingaben speichern.
async fn grundstueck_einbinden(b: &mut Beispiel) {
    b.neu_laden().await;
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let e =
        b.c.projekt_baugrundstuecke_gebaeude_einbinden(b.p.id, &plan)
            .await
            .unwrap_or_else(|e| panic!("Gebäudeeinbindung: {e}"));
    let mut r = b.p.planungsrecht.clone();
    let erm = r.baugrundstuecksermittlung.as_mut().unwrap();
    if erm.berechnungen != e.plan.berechnungen {
        erm.berechnungen = e.plan.berechnungen.clone();
        b.bereich(ProjektBereich::Grundstuecke, r).await;
    }
    b.grund = Some(
        core_lib::grundstueckskontext::ermitteln(&b.p)
            .unwrap()
            .remove(0),
    );
}

/// Enthält der Text eine rohe UUID (technische Kennung statt Bezeichnung)?
fn enthaelt_uuid(s: &str) -> bool {
    s.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .any(|t| t.len() == 36 && Uuid::parse_str(t).is_ok())
}

/// Übersteht der Wert die JSON-Rundreise der Ablage (serde_json to_string/from_str)?
fn rundreise(v: f64) -> bool {
    serde_json::from_str::<f64>(&serde_json::to_string(&v).unwrap()).unwrap() == v
}
/// Für die Übernahme maßgebende Messwerte einer Markierung (inkl. relativer Höhe).
fn massgebend(a: &Planaufmass, null_id: Uuid) -> Vec<(String, Vec<f64>)> {
    let werte = a.messen().unwrap();
    let null = werte
        .iter()
        .find(|w| w.id == null_id)
        .and_then(|w| w.hoehe_m)
        .unwrap_or(0.);
    a.markierungen
        .iter()
        .zip(&werte)
        .map(|(m, w)| {
            let mut v: Vec<f64> = [w.flaeche_m2, w.hoehe_m, w.horizontale_m]
                .into_iter()
                .flatten()
                .collect();
            if let (Plangeometrie::Hoehenpunkt { .. }, Some(h)) = (&m.geometrie, w.hoehe_m) {
                v.push(h - null);
            }
            (m.bezeichnung.clone(), v)
        })
        .collect()
}
fn rundreise_fehler(a: &Planaufmass, null_id: Uuid) -> Vec<String> {
    massgebend(a, null_id)
        .into_iter()
        .flat_map(|(n, v)| {
            v.into_iter().filter(|x| !rundreise(*x)).map(move |x| {
                format!(
                    "„{n}“ {x:?} → {:?}",
                    serde_json::from_str::<f64>(&serde_json::to_string(&x).unwrap()).unwrap()
                )
            })
        })
        .take(4)
        .collect()
}
/// Koordinaten auf 1e-4 pt runden und Markierungen minimal verschieben, bis alle
/// maßgebenden Messwerte die Rundreise überstehen. Gibt die Anzahl verschobener zurück.
fn stabilisieren(a: &mut Planaufmass, null_id: Uuid) -> usize {
    let r = |p: &mut Planpunkt| {
        for c in p.iter_mut() {
            *c = (*c * 1e4).round() / 1e4;
        }
    };
    for m in &mut a.markierungen {
        match &mut m.geometrie {
            Plangeometrie::Rechteck { von, bis }
            | Plangeometrie::Strecke { von, bis }
            | Plangeometrie::Kreis {
                mitte: von,
                rand: bis,
            } => {
                r(von);
                r(bis);
            }
            Plangeometrie::Hoehenpunkt { punkt } => r(punkt),
            Plangeometrie::Polygon { punkte } => punkte.iter_mut().for_each(r),
            Plangeometrie::Flaeche { polygone } => {
                for p in polygone {
                    p.aussen.iter_mut().for_each(r);
                    for innen in &mut p.innen {
                        innen.iter_mut().for_each(r);
                    }
                }
            }
        }
    }
    let mut verschoben = 0;
    for i in 0..a.markierungen.len() {
        for versuch in 0..400 {
            if massgebend(a, null_id)[i].1.iter().all(|v| rundreise(*v)) {
                if versuch > 0 {
                    verschoben += 1;
                }
                break;
            }
            let d = 1e-4;
            match &mut a.markierungen[i].geometrie {
                Plangeometrie::Rechteck { bis, .. } => bis[0] += d,
                Plangeometrie::Strecke { von, bis } => {
                    if (von[0] - bis[0]).abs() > 1e-6 {
                        bis[0] += d
                    } else {
                        bis[1] += d
                    }
                }
                Plangeometrie::Hoehenpunkt { punkt } => punkt[1] += d,
                _ => break,
            }
        }
    }
    verschoben
}

// ---------------------------------------------------------------------------
// Fachmodul
// ---------------------------------------------------------------------------

pub async fn fach(b: &mut Beispiel) {
    let pid = b.p.id;
    let grund = b.grund().id;
    let mut nachweise: Vec<(String, Uuid)> = vec![];

    // 1. Plan-PDF im Projekt-DMS ---------------------------------------------
    let pdf = plan_pdf();
    if let Ok(ziel) = std::env::var("BAUPROJEKT_BEISPIEL_ARTEFAKTE") {
        std::fs::create_dir_all(&ziel).unwrap();
        std::fs::write(
            std::path::Path::new(&ziel).join("EP-01_Grundrisse_Schnitt.pdf"),
            &pdf,
        )
        .unwrap();
    }
    let doc =
        b.c.store_document_kategorisiert(
            pid,
            "EP-01_Grundrisse_Schnitt_M100.pdf",
            Bytes::from(pdf),
            Some("grundriss".into()),
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
        json!({"dokument": dokument, "dokumentart": "Bauzeichnung (Grundrisse/Schnitt)",
            "plannummer": "EP-01", "revisionsindex": "a", "massstab": "1:100",
            "verfasser": "Erika Planer, Architektin", "datum": "2026-10-21"}),
    )
    .await;

    // 2. PDF-Aufmaß ----------------------------------------------------------
    let aid = Uuid::new_v4();
    let (e_geb, e_raum, e_hoehe, e_bilanz) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let (x_kg, x_eg, x_dg, x_schnitt) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let ausschnitt =
        |id, name: &str, art, von: (f64, f64), bis: (f64, f64), linie: bool| Planausschnitt {
            ebene_id: None,
            id,
            bezeichnung: name.into(),
            art,
            von: pt(von.0, von.1),
            bis: pt(bis.0, bis.1),
            kalibrierung: None,
            bezugslinie: linie.then(|| Planbezugslinie {
                von: pt(168., S_NULL),
                bis: pt(285., S_NULL),
                hoehe_m: NULLPUNKT_NHN,
                system: "m ü. NHN".into(),
            }),
        };
    let mut aufmass = Planaufmass {
        bezeichnung: "EP-01 Grundrisse und Schnitt A-A".into(),
        dokument: Some(dokument.clone()),
        seite: 1,
        hintergruende: vec![],
        elemente: vec![],
        kalibrierung: Some(Plankalibrierung::Massstab { nenner: 100. }),
        hoehenbezug: None,
        ebenen: [
            (e_geb, "Gebäude / Geschosse"),
            (e_raum, "Räume"),
            (e_hoehe, "Höhen (Schnitt)"),
            (e_bilanz, "Flächenbilanz / Entwässerung"),
        ]
        .into_iter()
        .map(|(id, n)| Planebene {
            id,
            bezeichnung: n.into(),
            sichtbar: true,
            gesperrt: false,
        })
        .collect(),
        markierungen: vec![],
        ausschnitte: vec![
            ausschnitt(
                x_kg,
                "Grundriss KG",
                Ausschnittart::Grundriss,
                (15., 25.),
                (145., 135.),
                false,
            ),
            ausschnitt(
                x_eg,
                "Grundriss EG",
                Ausschnittart::Grundriss,
                (150., 20.),
                (330., 140.),
                false,
            ),
            ausschnitt(
                x_dg,
                "Grundriss DG",
                Ausschnittart::Grundriss,
                (15., 160.),
                (145., 270.),
                false,
            ),
            ausschnitt(
                x_schnitt,
                "Schnitt A-A",
                Ausschnittart::Schnitt,
                (150., 150.),
                (295., 285.),
                true,
            ),
        ],
    };
    let haus = b.haus;
    let garage = b.garage;
    let mut m = |ebene: Uuid,
                 name: &str,
                 bedeutung: Planbedeutung,
                 geometrie: Plangeometrie,
                 gebaeude: Option<Uuid>,
                 geschoss: Option<Uuid>,
                 aus: Uuid|
     -> Uuid {
        let id = Uuid::new_v4();
        aufmass.markierungen.push(Planmarkierung {
            id,
            ebene_id: ebene,
            bezeichnung: name.into(),
            bedeutung,
            geometrie,
            grundstueck_id: Some(grund),
            gebaeude_id: gebaeude,
            geschoss_id: geschoss,
            flurstueck_id: None,
            ausschnitt_id: Some(aus),
            hoehenzone_von: None,
        });
        id
    };
    let rechteck = |a: (f64, f64), b: (f64, f64)| Plangeometrie::Rechteck {
        von: pt(a.0, a.1),
        bis: pt(b.0, b.1),
    };
    let aussen = |o: (f64, f64)| rechteck(o, (o.0 + 110., o.1 + 90.));
    let hp = |h: f64, x: f64| Plangeometrie::Hoehenpunkt {
        punkt: pt(x, sy(h)),
    };
    let strecke = |x: f64, h0: f64, h1: f64| Plangeometrie::Strecke {
        von: pt(x, sy(h0)),
        bis: pt(x, sy(h1)),
    };
    use Planbedeutung as B;
    let (kg, eg, dg) = (Some(b.kg), Some(b.eg), Some(b.dg));
    let m_grund = m(
        e_geb,
        "Gebäudegrundfläche Wohnhaus",
        B::Gebaeudegrundflaeche,
        aussen(O_EG),
        Some(haus),
        None,
        x_eg,
    );
    let m_garage = m(
        e_geb,
        "Gebäudegrundfläche Garage",
        B::Gebaeudegrundflaeche,
        rechteck(O_GA, (O_GA.0 + 35., O_GA.1 + 60.)),
        Some(garage),
        None,
        x_eg,
    );
    let m_aussen_kg = m(
        e_geb,
        "Außenkante KG",
        B::GeschossAussenkante,
        aussen(O_KG),
        Some(haus),
        kg,
        x_kg,
    );
    let m_aussen_eg = m(
        e_geb,
        "Außenkante EG",
        B::GeschossAussenkante,
        aussen(O_EG),
        Some(haus),
        eg,
        x_eg,
    );
    let m_aussen_dg = m(
        e_geb,
        "Außenkante DG",
        B::GeschossAussenkante,
        aussen(O_DG),
        Some(haus),
        dg,
        x_dg,
    );
    let mut raum_ids: BTreeMap<Uuid, Planraum> = BTreeMap::new();
    // Schrägenstreifen „<Raum> – Schräge …“ sind Höhenzonen ihres Raums: Die
    // Raummarkierung umfasst den Streifen, der Streifen wird als Zone gekennzeichnet.
    let mut zonen: Vec<(Uuid, Uuid)> = Vec::new();
    for (o, liste, geschoss, aus) in [
        (O_KG, raeume_kg(), kg, x_kg),
        (O_EG, raeume_eg(), eg, x_eg),
        (O_DG, raeume_dg(), dg, x_dg),
    ] {
        let zone_von = |r: &Planraum, z: &Planraum| z.name.starts_with(&format!("{} – ", r.name));
        for r in liste
            .iter()
            .filter(|r| !liste.iter().any(|e| zone_von(e, r)))
        {
            let mut ganz = r.clone();
            for z in liste.iter().filter(|z| zone_von(r, z)) {
                ganz.y0 = ganz.y0.min(z.y0);
                ganz.y1 = ganz.y1.max(z.y1);
            }
            let id = m(
                e_raum,
                r.name,
                r.bedeutung,
                rechteck(innen(o, ganz.x0, ganz.y0), innen(o, ganz.x1, ganz.y1)),
                Some(haus),
                geschoss,
                aus,
            );
            for z in liste.iter().filter(|z| zone_von(r, z)) {
                let zid = m(
                    e_raum,
                    z.name,
                    z.bedeutung,
                    rechteck(innen(o, z.x0, z.y0), innen(o, z.x1, z.y1)),
                    Some(haus),
                    geschoss,
                    aus,
                );
                zonen.push((zid, id));
                raum_ids.insert(zid, z.clone());
            }
            raum_ids.insert(id, ganz);
        }
    }
    let m_null = m(
        e_hoehe,
        "±0,00 OKFF EG",
        B::Nullpunkt,
        hp(0., S_X + 5.),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_gelaende = m(
        e_hoehe,
        "Gelände an der Außenwand (Mittel)",
        B::Gelaende,
        hp(-0.30, 168.),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_traufe = m(
        e_hoehe,
        "Traufe (Schnitt Außenwand/Dachhaut)",
        B::Traufe,
        hp(4.20, S_X),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_first = m(
        e_hoehe,
        "First",
        B::First,
        hp(7.45, S_X + 45.),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_fundament = m(
        e_hoehe,
        "Fundamentsohle",
        B::Fundamentsohle,
        hp(-3.10, S_X + 15.),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_ff_kg = m(
        e_hoehe,
        "OKFF KG",
        B::Fussboden,
        hp(-2.80, S_X + 55.),
        Some(haus),
        kg,
        x_schnitt,
    );
    let m_ff_eg = m(
        e_hoehe,
        "OKFF EG",
        B::Fussboden,
        hp(0., S_X + 55.),
        Some(haus),
        eg,
        x_schnitt,
    );
    let m_ff_dg = m(
        e_hoehe,
        "OKFF DG",
        B::Fussboden,
        hp(2.85, S_X + 55.),
        Some(haus),
        dg,
        x_schnitt,
    );
    let m_lh_kg = m(
        e_hoehe,
        "Lichte Höhe KG",
        B::Hoehenmass,
        strecke(S_X + 65., -2.80, -0.40),
        Some(haus),
        kg,
        x_schnitt,
    );
    let m_lh_eg = m(
        e_hoehe,
        "Lichte Höhe EG",
        B::Hoehenmass,
        strecke(S_X + 65., 0., 2.60),
        Some(haus),
        eg,
        x_schnitt,
    );
    let m_lh_dg = m(
        e_hoehe,
        "Lichte Höhe DG (unter Kehlbalkenlage)",
        B::Hoehenmass,
        strecke(S_X + 50., 2.85, 5.15),
        Some(haus),
        dg,
        x_schnitt,
    );
    let m_dd_kg = m(
        e_hoehe,
        "Deckendicke über KG",
        B::Hoehenmass,
        strecke(S_X + 70., -0.40, -0.10),
        Some(haus),
        kg,
        x_schnitt,
    );
    let m_dd_eg = m(
        e_hoehe,
        "Deckendicke über EG",
        B::Hoehenmass,
        strecke(S_X + 70., 2.60, 2.85),
        Some(haus),
        eg,
        x_schnitt,
    );
    let m_dd_dg = m(
        e_hoehe,
        "Kehlbalkenlage DG",
        B::Hoehenmass,
        strecke(S_X + 40., 5.15, 5.35),
        Some(haus),
        dg,
        x_schnitt,
    );
    let m_koerper = m(
        e_hoehe,
        "Körperhöhe Fundamentsohle bis Traufe",
        B::Hoehenmass,
        strecke(S_X + 80., -3.10, 4.20),
        Some(haus),
        None,
        x_schnitt,
    );
    let m_dach = m(
        e_hoehe,
        "Dachhaut Traufe–First",
        B::Dach,
        Plangeometrie::Strecke {
            von: pt(S_X, sy(4.20)),
            bis: pt(S_X + 45., sy(7.45)),
        },
        Some(haus),
        None,
        x_schnitt,
    );
    let m_dach_haus = {
        let id = Uuid::new_v4();
        aufmass.markierungen.push(Planmarkierung {
            id,
            ebene_id: e_bilanz,
            bezeichnung: "Dachprojektion Wohnhaus inkl. Überstände".into(),
            bedeutung: B::Versiegelung,
            geometrie: rechteck((O_EG.0 - 3., O_EG.1 - 5.), (O_EG.0 + 113., O_EG.1 + 95.)),
            grundstueck_id: Some(grund),
            gebaeude_id: None,
            geschoss_id: None,
            flurstueck_id: None,
            ausschnitt_id: Some(x_eg),
            hoehenzone_von: None,
        });
        id
    };
    let m_dach_garage = {
        let id = Uuid::new_v4();
        aufmass.markierungen.push(Planmarkierung {
            id,
            ebene_id: e_bilanz,
            bezeichnung: "Dachprojektion Garage (Flachdach, Kies)".into(),
            bedeutung: B::Versiegelung,
            geometrie: rechteck((O_GA.0 - 1., O_GA.1 - 1.), (O_GA.0 + 36., O_GA.1 + 61.)),
            grundstueck_id: Some(grund),
            gebaeude_id: None,
            geschoss_id: None,
            flurstueck_id: None,
            ausschnitt_id: Some(x_eg),
            hoehenzone_von: None,
        });
        id
    };

    // Messwerte prüfen
    for (zone, raum) in &zonen {
        if let Some(z) = aufmass.markierungen.iter_mut().find(|x| x.id == *zone) {
            z.hoehenzone_von = Some(*raum);
        }
    }
    let werte = b.c.planaufmass_pruefen(pid, &aufmass).await.unwrap();
    let wert = |id: Uuid| werte.iter().find(|w| w.id == id).unwrap().clone();
    for w in &werte {
        if !w.offen.is_empty() {
            let name = aufmass
                .markierungen
                .iter()
                .find(|m| m.id == w.id)
                .unwrap()
                .bezeichnung
                .clone();
            b.befund(format!(
                "PDF-Aufmaß: Markierung „{name}“ meldet offen: {:?}",
                w.offen
            ));
        }
    }
    assert!(
        nahe(wert(m_grund).flaeche_m2.unwrap(), 99.),
        "{:?}",
        wert(m_grund)
    );
    assert!(nahe(wert(m_garage).flaeche_m2.unwrap(), 21.));
    assert!(nahe(wert(m_traufe).hoehe_m.unwrap(), NULLPUNKT_NHN + 4.20));
    assert!(nahe(wert(m_first).hoehe_m.unwrap(), NULLPUNKT_NHN + 7.45));
    assert!(nahe(wert(m_lh_eg).hoehe_m.unwrap(), 2.60));
    assert!(nahe(wert(m_dach).horizontale_m.unwrap(), 4.5));
    b.info(format!(
        "PDF-Aufmaß: Dachneigung aus Traufe +4,20 / First +7,45 über 4,50 m = {:.2}°",
        wert(m_dach).neigung_grad.unwrap()
    ));
    let f_aufmass = b
        .speichern(
            ObjektArt::Planaufmass,
            aid,
            serde_json::to_value(&aufmass).unwrap(),
        )
        .await;

    // 2b. Aufgaben und Übernahme in Gebäude/Geschosse
    let aufgaben = b.c.planaufmass_aufgaben(pid, &aufmass).await.unwrap();
    let vorschlag = |ziel: &str| -> Option<Uuid> {
        aufgaben
            .iter()
            .find(|a| a.verbraucher == "gebaeude" && a.objekt == haus.to_string() && a.ziel == ziel)
            .and_then(|a| a.markierung)
    };
    let s = |g: Uuid, f: &str| format!("geschosse.{g}.{f}");
    let soll: Vec<(String, Uuid, Planmessgroesse)> = vec![
        (
            "geometrie.grundflaeche_m2".into(),
            m_grund,
            Planmessgroesse::Flaeche,
        ),
        (
            "geometrie.nullpunkt_m_nhn".into(),
            m_null,
            Planmessgroesse::Hoehe,
        ),
        (
            "geometrie.gelaende_m".into(),
            m_gelaende,
            Planmessgroesse::Hoehe,
        ),
        (
            "geometrie.traufe_m".into(),
            m_traufe,
            Planmessgroesse::Hoehe,
        ),
        ("geometrie.first_m".into(), m_first, Planmessgroesse::Hoehe),
        (
            "geometrie.fundamentsohle_m".into(),
            m_fundament,
            Planmessgroesse::Hoehe,
        ),
        (s(b.kg, "flaeche_m2"), m_aussen_kg, Planmessgroesse::Flaeche),
        (s(b.eg, "flaeche_m2"), m_aussen_eg, Planmessgroesse::Flaeche),
        (s(b.dg, "flaeche_m2"), m_aussen_dg, Planmessgroesse::Flaeche),
        (s(b.kg, "fussboden_m"), m_ff_kg, Planmessgroesse::Hoehe),
        (s(b.eg, "fussboden_m"), m_ff_eg, Planmessgroesse::Hoehe),
        (s(b.dg, "fussboden_m"), m_ff_dg, Planmessgroesse::Hoehe),
        (s(b.kg, "lichte_hoehe_m"), m_lh_kg, Planmessgroesse::Hoehe),
        (s(b.eg, "lichte_hoehe_m"), m_lh_eg, Planmessgroesse::Hoehe),
        (s(b.dg, "lichte_hoehe_m"), m_lh_dg, Planmessgroesse::Hoehe),
        (s(b.kg, "deckendicke_m"), m_dd_kg, Planmessgroesse::Hoehe),
        (s(b.eg, "deckendicke_m"), m_dd_eg, Planmessgroesse::Hoehe),
        (s(b.dg, "deckendicke_m"), m_dd_dg, Planmessgroesse::Hoehe),
    ];
    let mut falsche_vorschlaege = vec![];
    for (ziel, markierung, _) in &soll {
        match vorschlag(ziel) {
            Some(v) if v == *markierung => {}
            Some(v) => {
                let name = |id: Uuid| {
                    aufmass
                        .markierungen
                        .iter()
                        .find(|m| m.id == id)
                        .unwrap()
                        .bezeichnung
                        .clone()
                };
                falsche_vorschlaege.push(format!(
                    "{ziel} → „{}“ statt „{}“",
                    name(v),
                    name(*markierung)
                ));
            }
            None => falsche_vorschlaege.push(format!("{ziel} → kein Vorschlag")),
        }
    }
    if !falsche_vorschlaege.is_empty() {
        b.befund(format!(
            "Aufmaß-Aufgaben schlagen für Höhenabmessungen dieselbe erste Höhenmaß-Strecke des Geschosses vor (lichte Höhe, Geschosshöhe und Deckendicke teilen die Bedeutung „Höhenmaß“); „Übernehmen“ würde falsche Werte binden: {}",
            falsche_vorschlaege.join("; ")
        ));
    }
    let offene: Vec<_> = aufgaben
        .iter()
        .filter(|a| a.verbraucher == "gebaeude" && a.gebunden.is_none() && a.markierung.is_none())
        .map(|a| format!("{} · {}", a.gruppe, a.titel))
        .collect();
    b.info(format!(
        "Aufmaß-Aufgaben ohne passende Markierung (bewusst nicht gezeichnet): {}",
        offene.join(", ")
    ));

    let mut g_haus: Gebaeude = serde_json::from_value(
        b.c.fachobjekte(pid, ObjektArt::Gebaeude)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == haus.to_string())
            .unwrap()
            .daten,
    )
    .unwrap();
    let vorher = g_haus.clone();
    let bindungen = |rev: i64| -> Vec<Planbindung> {
        soll.iter()
            .map(|(ziel, markierung, groesse)| Planbindung {
                aufmass_id: aid,
                revision: rev,
                markierung_id: *markierung,
                groesse: *groesse,
                ziel: ziel.clone(),
            })
            .collect()
    };
    g_haus.planbindungen = bindungen(f_aufmass.revision);
    // Höhen aus einem Schnitt-Ausschnitt mit Bezugslinie „m ü. NHN“, ohne blattweiten Höhenbezug.
    if let Err(e) = b.c.planaufmass_gebaeude(pid, haus, g_haus.clone()).await {
        b.befund(format!(
            "PDF-Aufmaß → Gebäude: Höhenpunkte im Schnitt-Ausschnitt mit Bezugslinie im System „m ü. NHN“ werden abgewiesen („{e}“)."
        ));
    }
    // Speichern, binden, prüfen. Runde 2 nur, falls der gespeicherte Stand sofort
    // als vom Aufmaß abweichend gilt (Gleitkomma-Rundreise, siehe Befund).
    let mut runde = 0;
    let f_aufmass = loop {
        runde += 1;
        let f_aufmass = b
            .speichern(
                ObjektArt::Planaufmass,
                aid,
                serde_json::to_value(&aufmass).unwrap(),
            )
            .await;
        g_haus.planbindungen = bindungen(f_aufmass.revision);
        let g_neu =
            b.c.planaufmass_gebaeude(pid, haus, g_haus.clone())
                .await
                .unwrap_or_else(|e| panic!("Planaufmaß → Wohnhaus: {e}"));
        // Werte müssen den Basisdaten entsprechen.
        let geo_alt = vorher.geometrie.clone().unwrap();
        let geo_neu = g_neu.geometrie.clone().unwrap();
        let mut abweichungen = vec![];
        for (n, a, c) in [
            (
                "Grundfläche",
                geo_alt.grundflaeche_m2,
                geo_neu.grundflaeche_m2,
            ),
            (
                "Nullpunkt",
                geo_alt.nullpunkt_m_nhn,
                geo_neu.nullpunkt_m_nhn,
            ),
            ("Gelände", geo_alt.gelaende_m, geo_neu.gelaende_m),
            ("Traufe", geo_alt.traufe_m, geo_neu.traufe_m),
            ("First", geo_alt.first_m, geo_neu.first_m),
            (
                "Fundamentsohle",
                geo_alt.fundamentsohle_m,
                geo_neu.fundamentsohle_m,
            ),
        ] {
            if !a.zip(c).is_some_and(|(a, c)| nahe(a, c)) {
                abweichungen.push(format!("{n}: {a:?} → {c:?}"));
            }
        }
        for (alt, neu) in vorher.geschosse.iter().zip(&g_neu.geschosse) {
            let (a, c) = (
                alt.geometrie.clone().unwrap(),
                neu.geometrie.clone().unwrap(),
            );
            for (n, x, y) in [
                ("Fläche", a.flaeche_m2, c.flaeche_m2),
                ("Fußboden", a.fussboden_m, c.fussboden_m),
                ("lichte Höhe", a.lichte_hoehe_m, c.lichte_hoehe_m),
                ("Deckendicke", a.deckendicke_m, c.deckendicke_m),
            ] {
                if !x.zip(y).is_some_and(|(x, y)| nahe(x, y)) {
                    abweichungen.push(format!("{} {n}: {x:?} → {y:?}", alt.bezeichnung));
                }
            }
        }
        assert!(
            abweichungen.is_empty(),
            "Aufmaß weicht von Basisdaten ab: {abweichungen:?}"
        );
        let schwebend: Vec<String> = [
            ("Grundfläche", geo_neu.grundflaeche_m2),
            ("Traufe", geo_neu.traufe_m),
            ("First", geo_neu.first_m),
        ]
        .iter()
        .filter(|(_, v)| v.is_some_and(|v| v != (v * 100.).round() / 100.))
        .map(|(n, v)| format!("{n} = {}", v.unwrap()))
        .collect();
        if !schwebend.is_empty() {
            b.info(format!(
                "PDF-Aufmaß liefert Gleitkommareste (werden ungerundet gespeichert): {}",
                schwebend.join(", ")
            ));
        }
        b.speichern(
            ObjektArt::Gebaeude,
            haus,
            serde_json::to_value(&g_neu).unwrap(),
        )
        .await;
        let mut g_garage: Gebaeude = serde_json::from_value(
            b.c.fachobjekte(pid, ObjektArt::Gebaeude)
                .await
                .unwrap()
                .into_iter()
                .find(|f| f.objekt == garage.to_string())
                .unwrap()
                .daten,
        )
        .unwrap();
        g_garage.planbindungen = vec![Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_garage,
            groesse: Planmessgroesse::Flaeche,
            ziel: "geometrie.grundflaeche_m2".into(),
        }];
        let g_garage =
            b.c.planaufmass_gebaeude(pid, garage, g_garage)
                .await
                .unwrap();
        assert!(nahe(
            g_garage
                .geometrie
                .as_ref()
                .unwrap()
                .grundflaeche_m2
                .unwrap(),
            21.
        ));
        b.speichern(
            ObjektArt::Gebaeude,
            garage,
            serde_json::to_value(&g_garage).unwrap(),
        )
        .await;
        match b.c.gebaeudegrundlagen(pid, haus).await {
            Ok(_) => break f_aufmass,
            Err(e) if runde == 1 => {
                let beispiele = rundreise_fehler(&aufmass, m_null);
                b.befund(format!(
                    "Gebäude mit PDF-Planbindungen ist unmittelbar nach dem Speichern „veraltet“ („{e}“): Messwerte mit 16–17 signifikanten Stellen (z. B. {}) werden beim Speichern als JSON-Text abgelegt und beim Lesen von serde_json ohne Feature „float_roundtrip“ um 1 ULP verfälscht; die strikte JSON-Gleichheit in gebaeude_planquellen_pruefen schlägt dann fehl. Folge: Gebäudegrundlagen, GRZ/GFZ-Einbindung, Vollgeschoss-/Höhenübernahme gesperrt. Workaround: Markierungskoordinaten so verschieben, dass alle Messwerte die Rundreise überstehen.",
                    beispiele.join(", ")
                ));
                let n = stabilisieren(&mut aufmass, m_null);
                b.info(format!(
                    "Workaround: {n} Markierungen um < 0,01 pt verschoben"
                ));
            }
            Err(e) => panic!("Gebäude nach Stabilisierung weiterhin veraltet: {e}"),
        }
    };
    let gl = b.c.gebaeudegrundlagen(pid, haus).await.unwrap();
    for s in &gl.geschosse {
        b.info(format!(
            "Gebäudegrundlagen {}: Fläche {:?} m², OK Decke {:?}, Decke über Gelände {:?} m",
            s.bezeichnung, s.flaeche_m2, s.decke_oberkante_m, s.decke_ueber_gelaende_m
        ));
    }
    grundstueck_einbinden(b).await;

    // 3. Raumbücher KG / EG / DG -------------------------------------------------
    let mut raumbuecher = BTreeMap::new();
    for (name, kurz, geschoss) in [
        ("Raumbuch Kellergeschoss", "KG", b.kg),
        ("Raumbuch Erdgeschoss", "EG", b.eg),
        ("Raumbuch Dachgeschoss", "DG", b.dg),
    ] {
        let leer = Raumbuch {
            planbindungen: vec![],
            bezeichnung: name.into(),
            gebaeude_id: haus,
            geschoss_id: Some(geschoss),
            raeume: vec![],
        };
        let mut r =
            b.c.planaufmass_raumbuch(pid, aid, f_aufmass.revision, leer)
                .await
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        for (i, raum) in r.raeume.iter_mut().enumerate() {
            let p = &raum_ids[&raum.id];
            raum.nummer = format!("{kurz}.{:02}", i + 1);
            raum.hoehe_m = Some(p.hoehe);
            assert!(
                nahe(raum.flaeche_m2, flaeche(p)),
                "{}: {}",
                raum.bezeichnung,
                raum.flaeche_m2
            );
            if raum.nutzung.is_empty() || raum.nutzung == "Aufenthaltsraum" {
                raum.nutzung = "Wohnen".into();
            }
            // Schrägenstreifen zwischen den Linien 1,00 m und 2,00 m lichter Höhe.
            for z in &mut raum.hoehenzonen {
                assert!(nahe(z.flaeche_m2, flaeche(&raum_ids[&z.id])));
                z.bezeichnung = "Schräge 1,00–2,00 m".into();
                z.lichte_hoehe_min_m = Some(1.0);
                z.lichte_hoehe_max_m = Some(2.0);
            }
        }
        if kurz == "DG" {
            let n: usize = r.raeume.iter().map(|x| x.hoehenzonen.len()).sum();
            if r.raeume.len() != 6 || n != 5 {
                b.befund(format!(
                    "Raumbuch DG: {} Räume mit {n} Höhenzonen statt 6 Räume mit 5 Zonen",
                    r.raeume.len()
                ));
            }
        }
        // Zweite Übernahme erhält Nummern/Höhen und erzeugt keine Dubletten.
        let r2 =
            b.c.planaufmass_raumbuch(pid, aid, f_aufmass.revision, r.clone())
                .await
                .unwrap();
        assert_eq!(r2.raeume.len(), r.raeume.len());
        let rid = Uuid::new_v4();
        b.speichern(ObjektArt::Raumbuch, rid, serde_json::to_value(&r2).unwrap())
            .await;
        let summen = b.c.raumbuch_summen(pid, rid).await.unwrap();
        b.info(format!(
            "{name}: {} Räume, {:.2} m², {:?} m³",
            summen.anzahl,
            summen.flaeche_m2,
            summen.volumen_m3.map(|v| (v * 100.).round() / 100.)
        ));
        raumbuecher.insert(kurz, rid);
    }

    // 4. Fachberechnungen ----------------------------------------------------
    // 4a. Vollgeschosse (Gebäudeübernahme, § 2 Abs. 6 BauO NRW)
    let vg_id = Uuid::new_v4();
    let mut vp = vorlage(BerechnungsArt::Vollgeschosse);
    vp.bezeichnung = "Vollgeschossnachweis Wohnhaus (§ 2 Abs. 6 BauO NRW)".into();
    vp.grundstueck_id = Some(grund);
    vp.gebaeude_id = Some(haus);
    vp.gebaeudedaten_uebernehmen = Some(true);
    vp.regelkontext.bezeichnung = "BauO NRW 2018 – Vollgeschossbegriff".into();
    vp.regelkontext.version = "BauO NRW 2018, § 2 Abs. 6".into();
    vp.regelkontext.geltungsbereich =
        "Nordrhein-Westfalen; Bebauungsplan Nr. 42 (Zahl der Vollgeschosse)".into();
    vp.eingaben["regeln"] = json!({"mindesthoehe_m": 2.3, "hoehe_streng": false, "mindestanteil": 2.0/3.0,
        "anteil_streng": false, "vergleich_unterliegend": false, "oberirdisch_pruefen": true,
        "decke_ueber_gelaende_m": 1.6, "oberirdisch_streng": true});
    vp.eingaben["maximal_vollgeschosse"] = json!(2);
    regel(
        &mut vp,
        "§ 2 Abs. 6 BauO NRW 2018: Deckenoberkante im Mittel mehr als 1,60 m über Gelände, lichte Höhe ≥ 2,30 m über mindestens zwei Drittel der Grundfläche; i. V. m. § 20 Abs. 1 BauNVO",
        "Geprüft E. Planer, 21.10.2026: Satteldach ohne Staffelgeschoss; Bezug eigene Grundfläche nach Außenmaß",
    );
    let (vp1, r1) =
        b.c.berechnung_pruefen(pid, vp.clone())
            .await
            .unwrap_or_else(|e| panic!("Vollgeschosse: {e}"));
    let dg_id = b.dg;
    let dg_zone = |p: &mut FachPlan| {
        let mut teil = p.eingaben["teilflaechen"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let voll = 5.30 * 10.4;
        teil.push(json!({"id": "dg-hoch", "geschoss_id": dg_id.to_string(), "bezeichnung": "DG lichte Höhe ≥ 2,30 m (Planaufmaß EP-01)", "flaeche_m2": voll, "lichte_hoehe_m": 2.30}));
        teil.push(json!({"id": "dg-niedrig", "geschoss_id": dg_id.to_string(), "bezeichnung": "DG Schrägen, Kniestock, Außenwände < 2,30 m", "flaeche_m2": 99. - voll, "lichte_hoehe_m": 1.6}));
        p.eingaben["teilflaechen"] = Value::Array(teil);
    };
    let mut vp2 = vp1.clone();
    dg_zone(&mut vp2);
    let (vp3, _) = b.c.berechnung_pruefen(pid, vp2).await.unwrap();
    let zonen_dg = |p: &FachPlan| {
        p.eingaben["teilflaechen"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["geschoss_id"] == json!(dg_id.to_string()))
            .count()
    };
    b.info(format!(
        "Vollgeschosse nach Gebäudeübernahme offen: {:?}",
        r1.offen
    ));
    // Die Gebäudeübernahme erhält die manuell erfassten Höhenzonen des DG (Dachschrägen).
    if zonen_dg(&vp3) != 2 {
        b.befund(format!(
            "Vollgeschosse mit Gebäudeübernahme: manuell erfasste Höhenzonen des Dachgeschosses werden verworfen ({} statt 2 Zonen).",
            zonen_dg(&vp3)
        ));
    }
    let mut vp4 = vp3.clone();
    vp4.eingaben["aufmass_geprueft"] = json!(true);
    vp4.eingaben["sonderfaelle_geprueft"] = json!(true);
    let (vp, vr) = fach_sichern(b, vg_id, vp4).await;
    let einstufung: BTreeMap<String, Option<f64>> = vp.eingaben["geschosse"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            (
                g["bezeichnung"].as_str().unwrap().to_string(),
                kennzahl(&vr, &format!("geschoss:{}", g["id"].as_str().unwrap())),
            )
        })
        .collect();
    b.info(format!(
        "Vollgeschosseinstufung: {einstufung:?}, offen {:?}",
        vr.offen
    ));
    assert_eq!(einstufung["KG"], Some(0.));
    assert_eq!(einstufung["EG"], Some(1.));
    assert_eq!(einstufung["DG"], Some(0.));
    assert_eq!(kennzahl(&vr, "vollgeschosse"), Some(1.));
    let vg_nachweis = fach_nachweis(b, &mut nachweise, vg_id, &vp).await;
    // Einstufung aus dem Nachweis in Gebäude/Geschosse übernehmen.
    let mut g: Value =
        b.c.fachobjekte(pid, ObjektArt::Gebaeude)
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.objekt == haus.to_string())
            .unwrap()
            .daten;
    for s in g["geschosse"].as_array_mut().unwrap() {
        let id = s["id"].as_str().unwrap().to_string();
        s["einstufungsquelle"] =
            json!({"nachweis_id": vg_nachweis, "kennzahl": format!("geschoss:{id}")});
    }
    let gs = b.speichern(ObjektArt::Gebaeude, haus, g).await;
    let flags: Vec<_> = gs.daten["geschosse"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["vollgeschoss"].clone())
        .collect();
    assert_eq!(flags, vec![json!(false), json!(true), json!(false)]);
    grundstueck_einbinden(b).await;

    // 4b. Wohnflächen (WoFlV) aus den Raumbüchern EG und DG
    let wf_id = Uuid::new_v4();
    let roh =
        b.c.berechnung_aus_raumbuch(pid, raumbuecher["EG"], BerechnungsArt::Wohnflaechen)
            .await
            .unwrap();
    let mut roh_mit_grund = roh.clone();
    roh_mit_grund.grundstueck_id = Some(grund);
    match b.c.berechnung_pruefen(pid, roh_mit_grund).await {
        Err(e) if !e.to_string().contains("Wohnung") && !e.to_string().contains("Raumtyp") => b.befund(format!(
            "Wohnflächen aus Raumbuch: Der unveränderte Import ist nicht prüfbar; die Meldung „{e}“ nennt weder Position noch Feld (leere Wohnungszuordnung / Raumart der importierten Räume)."
        )),
        Err(e) => b.info(format!("Wohnflächenimport vor Zuordnung: {e}")),
        Ok((_, r)) => b.info(format!("Wohnflächenimport vor Zuordnung prüfbar, offen: {:?}", r.offen)),
    }
    let dg_import =
        b.c.berechnung_aus_raumbuch(pid, raumbuecher["DG"], BerechnungsArt::Wohnflaechen)
            .await
            .unwrap();
    let mut wp = roh;
    wp.bezeichnung = "Wohnflächenberechnung nach WoFlV – Wohnung 1".into();
    wp.grundstueck_id = Some(grund);
    wp.fachquellen.extend(dg_import.fachquellen.clone());
    wp.quellen.push(dokument.clone());
    let mut pos = wp.eingaben["positionen"].as_array().cloned().unwrap();
    pos.extend(
        dg_import.eingaben["positionen"]
            .as_array()
            .cloned()
            .unwrap(),
    );
    let mut diele = String::new();
    for p in &mut pos {
        // Höhenzonen haben die ID „Raum_Zone“; maßgebend ist die Zone.
        let id: Uuid = p["id"]
            .as_str()
            .unwrap()
            .rsplit('_')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let r = &raum_ids[&id];
        p["wohnung"] = json!("Wohnung 1 (Einfamilienhaus)");
        p["typ"] = json!(r.woflv.unwrap());
        p["begruendung"] = json!(if r.hoehe < 2.0 {
            "WoFlV § 4 Nr. 2: lichte Höhe 1,00–2,00 m, zur Hälfte"
        } else {
            "WoFlV § 4 Nr. 1: lichte Höhe ≥ 2,00 m, voll"
        });
        if r.name == "Diele mit Treppe" {
            diele = p["id"].as_str().unwrap().to_string();
        }
    }
    pos.push(
        json!({"id": "terrasse", "wohnung": "Wohnung 1 (Einfamilienhaus)", "raum": "Terrasse Süd",
        "typ": "balkon", "form": "rechteck", "laenge_m": 5.0, "breite_m": 4.0, "breite2_m": 0,
        "aufmass_m2": 0, "lichte_hoehe_m": null, "sonderfaktor": null,
        "begruendung": "WoFlV § 4 Nr. 4: Terrasse in der Regel zu einem Viertel",
        "quelle": "Lageplan / GRZ-Aufstellung: Terrasse Süd 5,00 × 4,00 m"}),
    );
    wp.eingaben["positionen"] = Value::Array(pos);
    wp.eingaben["abzuege"] = json!([{"id": "treppe-eg", "teilflaeche": diele, "bezeichnung": "Treppe EG–DG, 16 Steigungen",
        "flaeche_m2": 2.85, "typ": "treppe", "hoehe_m": 2.85, "stufen": 16,
        "grundlage": "WoFlV § 3 Abs. 3 Nr. 2: Treppen mit über drei Steigungen"}]);
    wp.eingaben["aufmass_vollstaendig"] = json!(true);
    wp.eingaben["wohneinheiten_bestaetigt"] = json!(true);
    wp.regelkontext.version = "WoFlV vom 25.11.2003".into();
    regel(
        &mut wp,
        "Wohnflächenverordnung (WoFlV) vom 25.11.2003, §§ 2–4",
        "Geprüft E. Planer, 21.10.2026: Raumbuch EG/DG aus PDF-Aufmaß EP-01, Dachschrägen als Höhenzonen erfasst",
    );
    let (wp, wr) = fach_sichern(b, wf_id, wp).await;
    let wohnflaeche = kennzahl(&wr, "wohnflaeche").unwrap();
    b.info(format!(
        "Wohnfläche gesamt {wohnflaeche:.2} m², offen {:?}",
        wr.offen
    ));
    assert!(
        (130. ..=160.).contains(&wohnflaeche),
        "Wohnfläche {wohnflaeche}"
    );
    let wf_nachweis = fach_nachweis(b, &mut nachweise, wf_id, &wp).await;

    // 4c. Stellplätze (Übernahme aus Wohnflächennachweis)
    let sp_id = Uuid::new_v4();
    let mut sp = vorlage(BerechnungsArt::Stellplaetze);
    sp.bezeichnung = "Stellplatznachweis Einfamilienhaus".into();
    sp.grundstueck_id = Some(grund);
    sp.regelkontext.bezeichnung = "Stellplatzsatzung der Stadt Musterstadt".into();
    sp.regelkontext.version = "Satzung vom 01.03.2022".into();
    sp.regelkontext.geltungsbereich = "Stadtgebiet Musterstadt, Zone II".into();
    sp.regelkontext.gueltig_ab = "2022-03-01".into();
    sp.eingaben["regeln"]["richtzahlen"] = json!([
        {"id": "efh-kfz", "art": "kfz", "nutzung": "Wohnen", "gebaeudetyp": "Einfamilienhaus", "zone": "", "groesse_von_m2": null, "groesse_bis_m2": null,
         "bezugsart": "wohneinheiten", "faktor": 2, "bezugsmenge": 1, "minimum": 0, "besucheranteil": 0, "besucher_zusaetzlich": false,
         "quelle": "Stellplatzsatzung Musterstadt Anlage 1 Nr. 1.1: Einfamilienhaus 2 Stellplätze je Wohnung"},
        {"id": "efh-rad", "art": "fahrrad", "nutzung": "Wohnen", "gebaeudetyp": "Einfamilienhaus", "zone": "", "groesse_von_m2": null, "groesse_bis_m2": null,
         "bezugsart": "wohneinheiten", "faktor": 2, "bezugsmenge": 1, "minimum": 0, "besucheranteil": 0, "besucher_zusaetzlich": false,
         "quelle": "Stellplatzsatzung Musterstadt Anlage 1 Nr. 1.1: 2 Fahrradabstellplätze je Wohnung"}
    ]);
    sp.eingaben["positionen"][0]["bezeichnung"] = json!("Einfamilienhaus, 1 Wohnung");
    sp.eingaben["positionen"][0]["gebaeudetyp"] = json!("Einfamilienhaus");
    sp.eingaben["angebot"] = json!([
        {"id": "garage", "bezeichnung": "Garage (6,00 × 3,50 m)", "art": "kfz", "anzahl": 1, "besucher": 0, "barrierefrei": 0, "ausserhalb": false, "anerkannt": true, "lage_und_sicherung": "Lageplan, Baugrundstück Flurstück 412"},
        {"id": "vorplatz", "bezeichnung": "Stellplatz vor der Garage (2,50 × 5,00 m)", "art": "kfz", "anzahl": 1, "besucher": 0, "barrierefrei": 0, "ausserhalb": false, "anerkannt": true, "lage_und_sicherung": "Lageplan; Stauraum bei Einfamilienhaus nach Satzung § 3 Abs. 2 anrechenbar"},
        {"id": "rad", "bezeichnung": "Fahrradabstellplätze in der Garage", "art": "fahrrad", "anzahl": 2, "besucher": 0, "barrierefrei": 0, "ausserhalb": false, "anerkannt": true, "lage_und_sicherung": "Garage, Rückwand"}
    ]);
    sp.uebernahmen = vec![
        Uebernahme {
            nachweis_id: wf_nachweis,
            kennzahl: "wohneinheiten".into(),
            ziel: "positionen.nutzung1.wohneinheiten".into(),
        },
        Uebernahme {
            nachweis_id: wf_nachweis,
            kennzahl: "wohnflaeche".into(),
            ziel: "positionen.nutzung1.wohnflaeche_m2".into(),
        },
    ];
    regel(
        &mut sp,
        "Satzung der Stadt Musterstadt über Stellplätze und Fahrradabstellplätze vom 01.03.2022 (§ 48 Abs. 3 BauO NRW)",
        "Geprüft E. Planer, 21.10.2026",
    );
    let (sp, sr) = fach_sichern(b, sp_id, sp).await;
    assert_eq!(kennzahl(&sr, "kfz_bedarf"), Some(2.));
    fach_nachweis(b, &mut nachweise, sp_id, &sp).await;

    // 4d. DIN 277 aus den drei Raumbüchern + Rauminhalt aus dem Schnitt
    let din_id = Uuid::new_v4();
    let mut dp =
        b.c.berechnung_aus_raumbuch(pid, raumbuecher["KG"], BerechnungsArt::Din277)
            .await
            .unwrap();
    let mut probe = dp.clone();
    probe.grundstueck_id = Some(grund);
    if let Err(e) = b.c.berechnung_pruefen(pid, probe).await {
        b.befund(format!(
            "DIN 277 aus Raumbuch: Die Klassifikation der Planmarkierungen (NUF/TF/VF, im Raumbuch als Nutzung „NUF“/„Technik“/„Verkehr“ geführt) wird nicht übernommen; „art“ und „geschoss“ bleiben leer, die Prüfung bricht technisch ab: „{e}“"
        ));
    }
    for kurz in ["EG", "DG"] {
        let weitere =
            b.c.berechnung_aus_raumbuch(pid, raumbuecher[kurz], BerechnungsArt::Din277)
                .await
                .unwrap();
        let mut pos = dp.eingaben["positionen"].as_array().cloned().unwrap();
        pos.extend(weitere.eingaben["positionen"].as_array().cloned().unwrap());
        dp.eingaben["positionen"] = Value::Array(pos);
        dp.fachquellen.extend(weitere.fachquellen);
    }
    b.info("DIN 277: Gebäudeweite Berechnung aus drei Geschoss-Raumbüchern durch Zusammenführen dreier Importe (Core bietet nur einen Raumbuch-Import je Berechnung)");
    let mut nrf: BTreeMap<&str, f64> = BTreeMap::new();
    for p in dp.eingaben["positionen"].as_array_mut().unwrap() {
        let id: Uuid = p["id"].as_str().unwrap().parse().unwrap();
        let r = &raum_ids[&id];
        let geschoss = if raeume_kg().iter().any(|x| x.name == r.name) {
            "KG"
        } else if raeume_eg().iter().any(|x| x.name == r.name) {
            "EG"
        } else {
            "DG"
        };
        // Geschoss und Klassifikation (NUF/TF/VF) kommen aus dem Import; nur als
        // Aufenthaltsraum gezeichnete Räume werden hier nach DIN 277 eingeordnet.
        if p["geschoss"] != json!(geschoss) || (p["art"] != json!("") && p["art"] != json!(r.din)) {
            b.befund(format!(
                "DIN 277 aus Raumbuch: „{}“ übernimmt Geschoss/Flächenart {}/{} statt {geschoss}/{}",
                r.name, p["geschoss"], p["art"], r.din
            ));
        }
        if p["art"] == json!("") {
            p["art"] = json!(r.din);
        }
        *nrf.entry(geschoss).or_default() += p["aufmass_m2"].as_f64().unwrap();
    }
    let luftraum = (LUFTRAUM_DG.2 - LUFTRAUM_DG.0) * (LUFTRAUM_DG.3 - LUFTRAUM_DG.1);
    let mut pos = dp.eingaben["positionen"].as_array().cloned().unwrap();
    for (g, n) in &nrf {
        let abzug = if *g == "DG" { luftraum } else { 0. };
        pos.push(json!({"id": format!("kgf-{}", g.to_lowercase()), "geschoss": g, "raum": format!("Konstruktionsfläche {g}"),
            "art": "kgf", "bereich": "R", "form": "aufmass", "laenge_m": 0, "breite_m": 0, "breite2_m": 0,
            "aufmass_m2": ((99. - n - abzug) * 10000.).round() / 10000.,
            "quelle": format!("Geschossfläche Außenkante 99,00 m² (PDF-Aufmaß) − NRF {:.2} m²{}", n,
                if abzug > 0. { format!(" − Luftraum Treppe {:.2} m²", abzug) } else { String::new() })}));
    }
    dp.eingaben["positionen"] = Value::Array(pos);
    dp.eingaben["volumen"] = json!([
        {"id": "koerper", "bezeichnung": "Baukörper UK Fundament bis Traufe", "form": "aufmass", "laenge_m": 0, "breite_m": 0, "breite2_m": 0, "aufmass_m2": 0, "hoehe_m": 0, "quelle": ""},
        {"id": "dach", "bezeichnung": "Dachkörper Satteldach (mittlere Höhe (First − Traufe) / 2)", "form": "rechteck", "laenge_m": 11.0, "breite_m": 9.0, "breite2_m": 0, "aufmass_m2": 0, "hoehe_m": 1.625, "quelle": "Schnitt A-A EP-01: First +7,45 − Traufe +4,20 = 3,25 m"}
    ]);
    dp.planbindungen = vec![
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_grund,
            groesse: Planmessgroesse::Flaeche,
            ziel: "volumen.koerper.aufmass_m2".into(),
        },
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_koerper,
            groesse: Planmessgroesse::Hoehe,
            ziel: "volumen.koerper.hoehe_m".into(),
        },
    ];
    dp.bezeichnung = "Flächen und Rauminhalte nach DIN 277 – Wohnhaus".into();
    dp.grundstueck_id = Some(grund);
    dp.eingaben["klassifikation_geprueft"] = json!(true);
    dp.eingaben["kgf_vollstaendig"] = json!(true);
    dp.eingaben["volumen_vollstaendig"] = json!(true);
    dp.eingaben["rauminhalt_erforderlich"] = json!(true);
    dp.regelkontext.version = "DIN 277:2021-08".into();
    regel(
        &mut dp,
        "DIN 277:2021-08 Grundflächen und Rauminhalte im Hochbau",
        "Geprüft E. Planer, 21.10.2026: Regelfall R, Raumbücher KG/EG/DG und Schnitt A-A",
    );
    let (dp, dr) = fach_sichern(b, din_id, dp).await;
    b.info(format!(
        "DIN 277: NUF {:.2} / TF {:.2} / VF {:.2} / NRF {:.2} / KGF {:?} / BGF {:?} / BRI {:?}",
        kennzahl(&dr, "nuf").unwrap(),
        kennzahl(&dr, "tf").unwrap(),
        kennzahl(&dr, "vf").unwrap(),
        kennzahl(&dr, "nrf").unwrap(),
        kennzahl(&dr, "kgf"),
        kennzahl(&dr, "bgf"),
        kennzahl(&dr, "bri")
    ));
    assert!((kennzahl(&dr, "bgf").unwrap() - (297. - luftraum)).abs() < 0.01);
    let q = dp.eingaben["volumen"][0]["quelle"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if enthaelt_uuid(&q) {
        b.befund(format!(
            "Planbindung in Fachberechnungen: Die automatisch gesetzte Quellangabe enthält rohe Dokument- und Aufmaß-UUIDs statt Dateiname/Plannummer („{q}“) und erscheint so im gedruckten Nachweis (DIN 277, Gebäudehöhen, Entwässerung). Bei zwei Bindungen in einer Zeile (Grundfläche + Höhe des Volumenkörpers) überschreibt die zweite die Quelle der ersten; die Herkunft der Grundfläche fehlt."
        ));
    }
    assert!((kennzahl(&dr, "bri").unwrap() - (99. * 7.30 + 99. * 1.625)).abs() < 0.01);
    fach_nachweis(b, &mut nachweise, din_id, &dp).await;

    // 4e. Gebäudehöhen: Traufe/First aus dem Schnitt, Bezug Straßenoberkante
    let h_id = Uuid::new_v4();
    let mut probe = vorlage(BerechnungsArt::Hoehen);
    probe.grundstueck_id = Some(grund);
    probe.gebaeude_id = Some(haus);
    probe.gebaeudedaten_uebernehmen = Some(true);
    // Bezugspunkt laut Bebauungsplan: Straßenoberkante statt mittlerem Gelände.
    probe.eingaben["bezugshoehe_m"] = json!(72.40);
    match b.c.berechnung_pruefen(pid, probe.clone()).await {
        Ok((p, _)) => {
            let bezug: Vec<f64> = p.eingaben["punkte"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["id"].as_str().unwrap().starts_with("gebaeude_"))
                .map(|x| x["bezug_m"].as_f64().unwrap())
                .collect();
            if bezug.is_empty() || !bezug.iter().all(|v| nahe(*v, 72.40)) {
                b.befund(format!("Gebäudehöhen mit Gebäudeübernahme: gewählte Bezugshöhe (Straßenoberkante 72,40 m ü. NHN) wird nicht übernommen: {bezug:?}"));
            }
        }
        Err(e) => b.befund(format!(
            "Gebäudehöhen mit Gebäudeübernahme nicht prüfbar: {e}"
        )),
    }
    probe.eingaben["daecher"] = json!([{"id": "satteldach", "bezeichnung": "Satteldach", "traufe_id": format!("gebaeude_{haus}_traufe"), "first_id": format!("gebaeude_{haus}_first"), "horizontale_m": 0, "min_neigung_grad": 30, "max_neigung_grad": 45, "quelle": ""}]);
    probe.planbindungen = vec![Planbindung {
        aufmass_id: aid,
        revision: f_aufmass.revision,
        markierung_id: m_dach,
        groesse: Planmessgroesse::Horizontale,
        ziel: "daecher.satteldach.horizontale_m".into(),
    }];
    if let Err(e) = b.c.berechnung_pruefen(pid, probe).await {
        b.info(format!("Gebäudehöhen: Gebäudeübernahme (Traufe/First) und PDF-Dachstrecke schließen sich aus, obwohl verschiedene Felder versorgt werden: {e}"));
    }
    let mut hp = vorlage(BerechnungsArt::Hoehen);
    hp.bezeichnung = "Trauf- und Firsthöhe, Dachneigung – Wohnhaus".into();
    hp.grundstueck_id = Some(grund);
    hp.gebaeude_id = Some(haus);
    hp.eingaben = json!({
        "hoehenbezug": "m ü. NHN",
        "regeln": [
            {"art": "traufe", "minimum_m": null, "maximum_m": 6.5, "absolut": false, "quelle": "B-Plan Nr. 42, textl. Festsetzung: Traufhöhe ≤ 6,50 m über Straßenoberkante 72,40 m ü. NHN"},
            {"art": "first", "minimum_m": null, "maximum_m": 10.0, "absolut": false, "quelle": "B-Plan Nr. 42, textl. Festsetzung: Firsthöhe ≤ 10,00 m über Straßenoberkante 72,40 m ü. NHN"}
        ],
        "punkte": [
            {"id": "traufe", "bezeichnung": "Traufe Wohnhaus", "art": "traufe", "absolut_m": 0, "bezug_m": 72.40, "quelle": ""},
            {"id": "first", "bezeichnung": "First Wohnhaus", "art": "first", "absolut_m": 0, "bezug_m": 72.40, "quelle": ""}
        ],
        "daecher": [{"id": "satteldach", "bezeichnung": "Satteldach", "traufe_id": "traufe", "first_id": "first", "horizontale_m": 0, "min_neigung_grad": 30, "max_neigung_grad": 45, "quelle": "Schnitt A-A EP-01"}],
        "bezug_geprueft": true
    });
    hp.planbindungen = vec![
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_traufe,
            groesse: Planmessgroesse::Hoehe,
            ziel: "punkte.traufe.absolut_m".into(),
        },
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_first,
            groesse: Planmessgroesse::Hoehe,
            ziel: "punkte.first.absolut_m".into(),
        },
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_dach,
            groesse: Planmessgroesse::Horizontale,
            ziel: "daecher.satteldach.horizontale_m".into(),
        },
    ];
    hp.regelkontext.version = "B-Plan Nr. 42, rechtskräftig seit 15.05.2019".into();
    regel(
        &mut hp,
        "Bebauungsplan Nr. 42 „Am Lindenweg“, textliche Festsetzungen Nr. 3 (Höhen) und Nr. 5 (Dachform Satteldach 30–45°)",
        "Geprüft E. Planer, 21.10.2026: Bezugspunkt Straßenoberkante laut Festsetzung, Höhen aus Schnitt A-A",
    );
    let (hp, hr) = fach_sichern(b, h_id, hp).await;
    let traufe = kennzahl(&hr, "hoehe:traufe").unwrap();
    let first = kennzahl(&hr, "hoehe:first").unwrap();
    let neigung = kennzahl(&hr, "dach:satteldach").unwrap();
    b.info(format!("Höhen über Straßenoberkante: Traufe {traufe:.2} m, First {first:.2} m, Dachneigung {neigung:.1}°"));
    if (neigung - 38.).abs() > 1. {
        b.info(format!("Basisdaten: Satteldach 38° laut Gebäudeangabe, aus Traufe/First über 4,50 m ergeben sich {neigung:.1}° (Traufpunkt an der Außenwand)"));
    }
    assert!(traufe <= 6.5 && first <= 10.);
    fach_nachweis(b, &mut nachweise, h_id, &hp).await;

    // 4f. Freiflächen / Versiegelung
    let ff_id = Uuid::new_v4();
    let mut fp = vorlage(BerechnungsArt::Freiflaechen);
    fp.bezeichnung = "Versiegelungs- und Freiflächenbilanz Flurstück 412".into();
    fp.grundstueck_id = Some(grund);
    let ff_pos = |id: &str, n: &str, typ: &str, f: f64, q: &str| json!({"id": id, "bezeichnung": n, "typ": typ, "flaeche_m2": f, "quelle": q});
    let gruen = GRUNDSTUECK_M2 - 99. - 21. - 30. - 20. - 12.5;
    fp.eingaben["positionen"] = json!([
        ff_pos("wohnhaus", "Wohnhaus (Gebäudegrundfläche)", "dach", 0., ""),
        ff_pos(
            "garage",
            "Garage",
            "dach",
            21.,
            "PDF-Aufmaß EP-01 / Gebäude Garage"
        ),
        ff_pos(
            "zufahrt",
            "Zufahrt Garage (Pflaster)",
            "befestigt",
            30.,
            "Lageplan / GRZ-Aufstellung"
        ),
        ff_pos(
            "terrasse",
            "Terrasse Süd",
            "befestigt",
            20.,
            "Lageplan / GRZ-Aufstellung"
        ),
        ff_pos(
            "stellplatz",
            "Stellplatz vor der Garage (Rasenfugenpflaster)",
            "teilversiegelt",
            12.5,
            "Lageplan / GRZ-Aufstellung"
        ),
        ff_pos(
            "garten",
            "Gartenfläche",
            "gruen",
            gruen,
            "Restfläche Flurstück 412 (620 m²) laut Lageplan"
        )
    ]);
    fp.eingaben["regeln"]["min_gruenanteil"] = json!(0.5);
    fp.eingaben["regeln"]["baum_je_m2"] = json!(400);
    fp.eingaben["baeume_angebot"] = json!(2);
    fp.eingaben["bilanz_geprueft"] = json!(true);
    // Gebäudegrundfläche des Wohnhauses (steht auf diesem Grundstück) direkt aus dem Aufmaß binden.
    fp.planbindungen = vec![Planbindung {
        aufmass_id: aid,
        revision: f_aufmass.revision,
        markierung_id: m_grund,
        groesse: Planmessgroesse::Flaeche,
        ziel: "positionen.wohnhaus.flaeche_m2".into(),
    }];
    if let Err(e) = b.c.berechnung_pruefen(pid, fp.clone()).await {
        b.befund(format!("Freiflächenbilanz: Die Gebäudegrundfläche (Markierung des Wohnhauses) lässt sich nicht binden („{e}“)."));
        fp.planbindungen.clear();
        fp.eingaben["positionen"][0]["flaeche_m2"] = json!(99.);
        fp.eingaben["positionen"][0]["quelle"] =
            json!("Gebäude Wohnhaus: Gebäudegrundfläche 99,00 m² (PDF-Aufmaß EP-01)");
    }
    fp.uebernahmen = vec![Uebernahme {
        nachweis_id: wf_nachweis,
        kennzahl: "wohneinheiten".into(),
        ziel: "wohneinheiten".into(),
    }];
    fp.regelkontext.version = "B-Plan Nr. 42 (2019)".into();
    regel(
        &mut fp,
        "Bebauungsplan Nr. 42, textl. Festsetzung Nr. 7: mind. 50 % des Baugrundstücks begrünt, je angefangene 400 m² ein Laubbaum; § 8 Abs. 1 BauO NRW",
        "Geprüft E. Planer, 21.10.2026",
    );
    let (fp, fr) = fach_sichern(b, ff_id, fp).await;
    b.info(format!(
        "Freiflächen: versiegelt {:?} m², Grün {:?} m², Grünanteil {:?} %",
        kennzahl(&fr, "versiegelt_m2"),
        kennzahl(&fr, "gruen_m2"),
        kennzahl(&fr, "gruenanteil")
    ));
    fach_nachweis(b, &mut nachweise, ff_id, &fp).await;

    // 4g. Entwässerung (Dachflächen aus dem Aufmaß, Pflaster, Abflussbeiwerte)
    let ew_id = Uuid::new_v4();
    let mut ep = vorlage(BerechnungsArt::Entwaesserung);
    ep.bezeichnung = "Regenwasserbewirtschaftung – Rückhaltung mit Drosselabfluss".into();
    ep.grundstueck_id = Some(grund);
    let kl = |typ: &str, n: &str, c: f64| json!({"typ": typ, "bezeichnung": n, "beiwert": c, "quelle": "DIN 1986-100:2016-12, Tabelle 9 (Spitzenabflussbeiwert C_s)"});
    ep.eingaben["regeln"]["klassen"] = json!([
        kl("dach", "Schrägdach (Dachziegel)", 1.0),
        kl("kiesdach", "Flachdach mit Kiesschüttung", 0.8),
        kl("pflaster", "Betonsteinpflaster, Fugenanteil < 15 %", 0.9),
        kl("rasenfuge", "Rasenfugenpflaster", 0.4),
        kl("gruen", "Garten / Rasen", 0.2)
    ]);
    ep.eingaben["regeln"]["regen"] = json!([
        {"id": "d5", "dauer_min": 5, "intensitaet_l_s_ha": 300.0, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"},
        {"id": "d10", "dauer_min": 10, "intensitaet_l_s_ha": 211.7, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"},
        {"id": "d15", "dauer_min": 15, "intensitaet_l_s_ha": 166.7, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"},
        {"id": "d30", "dauer_min": 30, "intensitaet_l_s_ha": 105.6, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"},
        {"id": "d60", "dauer_min": 60, "intensitaet_l_s_ha": 62.5, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"},
        {"id": "d120", "dauer_min": 120, "intensitaet_l_s_ha": 37.5, "quelle": "KOSTRA-DWA 2020, Rasterfeld Musterstadt, T = 5 a"}
    ]);
    ep.eingaben["regeln"]["sicherheitsfaktor"] = json!(1.15);
    ep.eingaben["regeln"]["drossel_l_s"] = json!(1.0);
    ep.eingaben["regeln"]["boden_grundwasser_vermerk"] =
        json!("Keine Versickerung angesetzt (Lehmboden, kf < 1e-6 m/s laut Baugrunderkundung)");
    ep.eingaben["regeln"]["vorhandenes_volumen_m3"] = json!(6.0);
    let ep_pos = |id: &str, n: &str, typ: &str, f: f64, q: &str| json!({"id": id, "bezeichnung": n, "typ": typ, "flaeche_m2": f, "quelle": q});
    ep.eingaben["positionen"] = json!([
        ep_pos(
            "dach-haus",
            "Dach Wohnhaus (Projektion inkl. Überstände)",
            "dach",
            0.,
            ""
        ),
        ep_pos("dach-garage", "Dach Garage", "kiesdach", 0., ""),
        ep_pos(
            "zufahrt",
            "Zufahrt Garage (Pflaster)",
            "pflaster",
            30.,
            "Lageplan"
        ),
        ep_pos("terrasse", "Terrasse Süd", "pflaster", 20., "Lageplan"),
        ep_pos(
            "stellplatz",
            "Stellplatz vor der Garage",
            "rasenfuge",
            12.5,
            "Lageplan"
        )
    ]);
    ep.planbindungen = vec![
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_dach_haus,
            groesse: Planmessgroesse::Flaeche,
            ziel: "positionen.dach-haus.flaeche_m2".into(),
        },
        Planbindung {
            aufmass_id: aid,
            revision: f_aufmass.revision,
            markierung_id: m_dach_garage,
            groesse: Planmessgroesse::Flaeche,
            ziel: "positionen.dach-garage.flaeche_m2".into(),
        },
    ];
    ep.eingaben["flaechen_geprueft"] = json!(true);
    ep.eingaben["regendaten_geprueft"] = json!(true);
    ep.regelkontext.version = "DIN 1986-100:2016-12 / KOSTRA-DWA 2020".into();
    regel(
        &mut ep,
        "Entwässerungssatzung Musterstadt: Drosselabfluss 1,0 l/s je Grundstück; DIN 1986-100:2016-12; KOSTRA-DWA 2020",
        "Geprüft E. Planer, 21.10.2026: Retentionszisterne 6 m³",
    );
    let (ep, er) = fach_sichern(b, ew_id, ep).await;
    let au = kennzahl(&er, "au").unwrap();
    b.info(format!(
        "Entwässerung: A_u {au:.2} m², Q(r5,5 = 300 l/(s·ha)) {:.2} l/s, erforderliches Volumen {:?} m³, offen {:?}",
        300. * au / 10000., kennzahl(&er, "rueckhaltung"), er.offen
    ));
    fach_nachweis(b, &mut nachweise, ew_id, &ep).await;

    // 4h. Überbaubarkeit (Baugrenzen)
    let ub_id = Uuid::new_v4();
    let mut up = vorlage(BerechnungsArt::Ueberbaubarkeit);
    up.bezeichnung = "Überbaubare Grundstücksfläche – Baugrenzen".into();
    up.grundstueck_id = Some(grund);
    let ub = |id: &str,
              n: &str,
              art: &str,
              a: f64,
              c: f64,
              aus: Option<f64>,
              best: bool,
              ent: &str| {
        json!({
        "id": id, "bezeichnung": n, "art": art, "abstand_min_m": a, "abstand_max_m": c, "ausserhalb_m2": aus,
        "quelle": "Lageplan M 1:500 mit Baugrenzen B-Plan Nr. 42", "entscheidung_bestaetigt": best, "entscheidung": ent})
    };
    up.eingaben["positionen"] = json!([
        ub(
            "nord",
            "Wohnhaus Nordfassade (Straßenseite)",
            "baugrenze",
            -3.0,
            -3.0,
            None,
            false,
            ""
        ),
        ub(
            "ost",
            "Wohnhaus Ostfassade",
            "baugrenze",
            -1.5,
            -1.5,
            None,
            false,
            ""
        ),
        ub(
            "sued",
            "Wohnhaus Südfassade",
            "baugrenze",
            -4.0,
            -4.0,
            None,
            false,
            ""
        ),
        ub(
            "west",
            "Wohnhaus Westfassade",
            "baugrenze",
            -0.5,
            -0.5,
            None,
            false,
            ""
        ),
        ub(
            "garage",
            "Garage Ostseite (seitliche Baugrenze)",
            "baugrenze",
            0.5,
            1.0,
            Some(4.5),
            true,
            "§ 23 Abs. 5 Satz 2 BauNVO i. V. m. § 12 BauNVO: Garage außerhalb der Baugrenze zulässig, B-Plan schließt dies nicht aus (textl. Festsetzung Nr. 6); Zustimmung im Antrag beantragt"
        )
    ]);
    up.eingaben["aufmass_geprueft"] = json!(true);
    up.eingaben["flaechen_abgegrenzt"] = json!(true);
    up.regelkontext.version = "B-Plan Nr. 42 (2019) / BauNVO 2017".into();
    regel(
        &mut up,
        "Bebauungsplan Nr. 42: Baugrenzen laut Planzeichnung; § 23 BauNVO 2017",
        "Geprüft E. Planer, 21.10.2026",
    );
    let (up, ur) = fach_sichern(b, ub_id, up).await;
    b.info(format!(
        "Überbaubarkeit: größter Überstand {:?} m, außerhalb {:?} m², offen {:?}",
        kennzahl(&ur, "ueberstand_m"),
        kennzahl(&ur, "ausserhalb_m2"),
        ur.offen
    ));
    fach_nachweis(b, &mut nachweise, ub_id, &up).await;

    // 5. Nachweise GRZ / GFZ / Geschossigkeit / BMZ --------------------------
    b.neu_laden().await;
    let plan = core_lib::grundstuecksinformationen::plan(&b.p.planungsrecht)
        .unwrap()
        .unwrap();
    let pr =
        b.c.projekt_baugrundstuecke_pruefen(pid, &plan)
            .await
            .unwrap();
    let k = &pr.baugrundstuecke[0].pruefung.kennzahlen;
    b.info(format!(
        "Maß der Nutzung: GRZ {:.3}, GRZ II {:.3}, GFZ {:.3}, BMZ {:?}",
        k.grz, k.grz_gesamt, k.gfz, k.bmz
    ));
    for (fach, name) in [
        (NutzungsArt::Grz, "GRZ-Nachweis Baugrundstück Flurstück 412"),
        (NutzungsArt::Gfz, "GFZ-Nachweis Baugrundstück Flurstück 412"),
        (
            NutzungsArt::Geschossigkeit,
            "Nachweis Zahl der Vollgeschosse",
        ),
        (
            NutzungsArt::Bmz,
            "BMZ-Nachweis (informativ, BMZ nicht festgesetzt)",
        ),
    ] {
        let geschossigkeit = fach == NutzungsArt::Geschossigkeit;
        match b
            .c
            .nachweis_erstellen(
                pid,
                Uuid::new_v4(),
                name.into(),
                NachweisEingabe::Nutzung {
                    fach,
                    plan: plan.clone(),
                },
                &b.u,
            )
            .await
        {
            Ok(n) => {
                if !n.vollstaendig {
                    b.info(format!("{name}: archiviert, aber nicht vollständig"));
                }
                // Das Archiv führt Gebäude über ihre Identität; der Druck löst die
                // Bezeichnung auf (siehe Berechnungsheft).
                let _ = geschossigkeit;
                nachweise.push((name.into(), n.id));
            }
            Err(e) => b.befund(format!("{name}: Archivierung abgelehnt: {e}")),
        }
    }

    // Freigabe aller aktuellen, vollständigen Nachweise
    for (name, id) in &nachweise {
        if let Err(e) =
            b.c.nachweis_freigeben(
                pid,
                *id,
                "Fachlich geprüft und freigegeben – E. Planer".into(),
                &b.u,
            )
            .await
        {
            eprintln!("BEFUND-KANDIDAT: Freigabe {name}: {e}");
        }
    }
    let mut nicht_aktuell = vec![];
    for n in b.c.nachweise(pid).await.unwrap() {
        if !n.aktuell || n.freigabe.is_none() {
            nicht_aktuell.push(format!(
                "{} (aktuell {}, vollständig {}, frei {})",
                n.stand.bezeichnung,
                n.aktuell,
                n.stand.vollstaendig,
                n.freigabe.is_some()
            ));
        }
    }
    if !nicht_aktuell.is_empty() {
        b.befund(format!(
            "Nachweise nach Abschluss nicht aktuell/freigegeben: {}",
            nicht_aktuell.join("; ")
        ));
    }

    // 6. Druckausgabe ----------------------------------------------------------
    let katalog = b.c.druckkatalog(pid).await.unwrap();
    for (name, id) in &nachweise {
        if !katalog.eintraege.iter().any(|e| {
            matches!(e.quelle, Druckquelle::Nachweis { id: x } if x == *id) && e.verfuegbar
        }) {
            b.befund(format!(
                "Druckkatalog bietet Nachweis „{name}“ nicht als verfügbar an"
            ));
        }
    }
    let mut quellen: Vec<Druckquelle> = katalog
        .eintraege
        .iter()
        .filter(|e| e.empfohlen && e.verfuegbar)
        .map(|e| e.quelle.clone())
        .collect();
    for (name, id) in &nachweise {
        if !quellen
            .iter()
            .any(|q| matches!(q, Druckquelle::Nachweis { id: x } if x == id))
        {
            b.info(format!(
                "Nachweis „{name}“ nicht im empfohlenen Berechnungsheft; ergänzt"
            ));
            quellen.push(Druckquelle::Nachweis { id: *id });
        }
    }
    for e in katalog
        .eintraege
        .iter()
        .filter(|e| matches!(e.quelle, Druckquelle::Raumbuch { .. }) && e.verfuegbar)
    {
        if !quellen
            .iter()
            .any(|q| serde_json::to_value(q).unwrap() == serde_json::to_value(&e.quelle).unwrap())
        {
            quellen.push(e.quelle.clone());
        }
    }
    quellen.push(Druckquelle::Dokument {
        dokument_id: doc.document_id,
        version: doc.version,
        seiten: vec![],
    });
    b.neu_laden().await;
    let auftrag = Druckauftrag {
        gestaltung: None,
        aktenzeichen: "BV 2026-017 Mustermann".into(),
        datum: Some(stichtag()),
        titel: "Berechnungen und Nachweise – Neubau Einfamilienhaus Mustermann".into(),
        absender: "Planungsbüro Planer Architekten\nAm Markt 5\n59999 Musterstadt".into(),
        empfaenger: "Stadt Musterstadt\nBauaufsicht".into(),
        anschreiben: String::new(),
        mit_anschreiben: false,
        inhaltsverzeichnis: true,
        deckblatt: true,
        trennblaetter: true,
        aufgestellt_von: None,
        projekt_version: b.p.version,
        quellen: quellen.clone(),
    };
    let heft =
        b.c.druckausgabe(pid, auftrag.clone())
            .await
            .unwrap_or_else(|e| panic!("Berechnungsheft: {e}"));
    assert!(heft.pdf.starts_with(b"%PDF-"));
    b.info(format!(
        "Berechnungsheft: {} Seiten, {} Positionen, aufgestellt von „{}“",
        heft.seiten,
        heft.positionen.len(),
        katalog.aufgestellt_von
    ));
    // Einzeldruck exemplarisch (Fachnachweis mit PDF-Bindungen und Nutzungsnachweis).
    for (name, id) in nachweise
        .iter()
        .filter(|(n, _)| n.starts_with("Wohnfläche") || n.starts_with("GRZ"))
    {
        let mut einzel = auftrag.clone();
        einzel.deckblatt = false;
        einzel.trennblaetter = false;
        einzel.inhaltsverzeichnis = false;
        einzel.quellen = vec![Druckquelle::Nachweis { id: *id }];
        let d =
            b.c.druckausgabe(pid, einzel)
                .await
                .unwrap_or_else(|e| panic!("Einzeldruck {name}: {e}"));
        assert!(d.pdf.starts_with(b"%PDF-") && d.seiten > 0);
    }
    if let Ok(ziel) = std::env::var("BAUPROJEKT_BEISPIEL_ARTEFAKTE") {
        std::fs::write(
            std::path::Path::new(&ziel).join("Berechnungsheft.pdf"),
            &heft.pdf,
        )
        .unwrap();
    }
    let abgelegt =
        b.c.druckpdf_ablegen(
            pid,
            "Berechnungsheft Neubau EFH Mustermann",
            Bytes::from(heft.pdf),
            Uuid::new_v4(),
            &b.u,
        )
        .await
        .unwrap();
    b.info(format!(
        "Berechnungsheft im DMS abgelegt: {} V{}",
        abgelegt.file_name, abgelegt.version
    ));
    let veraltet: Vec<_> =
        b.c.nachweise(pid)
            .await
            .unwrap()
            .into_iter()
            .filter(|n| !n.aktuell)
            .map(|n| n.stand.bezeichnung)
            .collect();
    assert!(veraltet.is_empty(), "Nach Ablage veraltet: {veraltet:?}");

    // Offene Punkte der Projektübersicht in diesem Fachbereich
    let u =
        b.c.projektuebersicht_am(pid, stichtag(), &b.u)
            .await
            .unwrap();
    for p in &u.punkte {
        b.info(format!(
            "Projektübersicht: {} – {} ({})",
            p.titel, p.hinweis, p.ziel.bereich
        ));
    }
}
