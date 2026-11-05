//! Lesesicht und Vektorplan aus dem gespeicherten Ergebnis, keine Neuberechnung.
use super::*;
fn opt(n: Option<f64>) -> String {
    n.map(|v| {
        format!("{v:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .replace('.', ",")
    })
    .unwrap_or_else(|| "nicht ermittelt".into())
}
pub fn zeilen(p: &AbstandsPlan, r: &AbstandsErgebnis) -> Vec<String> {
    let mut z = vec![
        format!(
            "Projektbezogener Abstandsflächennachweis: {}",
            p.bezeichnung
        ),
        format!(
            "Rechenkern: {} · Regelpaket {} / {}",
            r.rechenstand, p.regeln.id, p.regeln.version
        ),
        format!("Regelstand: {}", p.regeln.rechtsstand),
        p.regeln.hinweis.clone(),
        format!(
            "Gemeinde: {} · Rechtsfassung: {} · Baugebiet: {}",
            p.recht.gemeinde, p.recht.fassung, p.recht.gebiet
        ),
        format!("Anwendbarkeit / Übergangsrecht: {}", p.recht.anwendung),
        format!(
            "Satzungsprüfung: {} · Planfestsetzungen: {}",
            p.recht.satzung, p.recht.planfestsetzung
        ),
        format!(
            "Projektbezogene Rechtsprüfung: {} · {}",
            p.recht.geprueft, p.recht.pruefvermerk
        ),
        format!(
            "Höhen-/Geländebezug: {} · Koordinaten: {} (Meter)",
            p.hoehenbezug, p.koordinatenbezug
        ),
        format!(
            "Umfangsprüfung: {} · {}",
            p.umfang_geprueft, p.umfang_vermerk
        ),
        format!(
            "Vollständig und konfliktfrei: {}. Fachliche Bürofreigabe siehe Nachweisstatus.",
            r.vollstaendig && r.konfliktfrei
        ),
    ];
    z.extend(r.offen.iter().map(|s| format!("OFFEN: {s}")));
    for (i, g) in p.grundstuecke.iter().enumerate() {
        z.push(format!(
            "G{}: {} · Einheit {} · Flurstücke {} · {}",
            i + 1,
            g.bezeichnung,
            g.id,
            g.flurstueck_ids.join(", "),
            g.grundlage
        ));
    }
    for (i, w) in p.waende.iter().enumerate() {
        if let Some(e) = r.waende.iter().find(|e| e.id == w.id) {
            z.push(format!(
                "W{}: {} / {} · Wand-ID {} · {:?}",
                i + 1,
                w.gebaeude,
                w.bezeichnung,
                w.id,
                e.status
            ));
            z.push(format!(
                "Einheit {:?} · Wand von ({}, {}) nach ({}, {}) m · Außenseite {:?} · Länge {} m",
                w.grundstueck_id, w.von.x, w.von.y, w.bis.x, w.bis.y, w.aussen, e.laenge_m
            ));
            if e.profil.is_empty() {
                z.push(format!(
                    "Gelände A/B: {} / {} m · Wandabschluss A/B: {} / {} m · Quelle: {}",
                    opt(w.gelaende_a_m),
                    opt(w.gelaende_b_m),
                    opt(w.abschluss_a_m),
                    opt(w.abschluss_b_m),
                    w.hoehenquelle
                ));
            } else {
                z.push(format!("Höhenprofil · Quelle: {}", w.hoehenquelle));
                z.extend(e.profil.iter().map(|q| {
                    format!(
                        "Station {} m · Gelände {} m · Oberkante {} m · Wandhöhe {} m · Tiefe {} m · Wandteil {}",
                        opt(Some(q.station_m)),
                        opt(Some(q.gelaende_m)),
                        opt(Some(q.oberkante_m)),
                        opt(Some(q.wandhoehe_m)),
                        opt(q.tiefe_m),
                        q.wandteil
                    )
                }));
            }
            z.push(format!(
                "Dach {:?}: Höhe {} m · Neigung {}° · manueller Faktor {} · {}",
                w.dachart,
                opt(w.dachhoehe_m),
                opt(w.dachneigung_grad),
                opt(w.dachfaktor_manuell),
                w.dachgrundlage
            ));
            z.extend(e.herleitung.clone());
            z.push(format!("H {} m · erforderliche Tiefe {} m · Abstandsfläche {} m² · außerhalb gesicherter Fläche {} m²",opt(e.h_m),opt(e.tiefe_m),opt(e.flaeche_m2),opt(e.ausserhalb_m2)));
            z.extend(e.offen.iter().map(|s| format!("OFFEN W{}: {s}", i + 1)));
            for (j, a) in e.konflikte.iter().enumerate() {
                z.push(format!(
                    "Konflikt W{}/{}: {}",
                    i + 1,
                    j + 1,
                    koordinaten(&a.aussen)
                ));
                for l in &a.innen {
                    z.push(format!("Innenring: {}", koordinaten(l)));
                }
            }
            if let Some(s) = &w.sonderansatz {
                z.push(format!(
                    "Besonderer Ansatz: {} m · wirksam {} · {} · Beleg {:?}",
                    s.tiefe_m, s.wirksam, s.grundlage, s.beleg
                ));
            }
        }
    }
    for (i, f) in p.flaechen.iter().enumerate() {
        z.push(format!(
            "F{}: {:?} · {} · wirksam {} · {} · Beleg {:?}",
            i + 1,
            f.art,
            f.bezeichnung,
            f.wirksam,
            f.grundlage,
            f.beleg
        ));
    }
    for u in &r.ueberdeckungen {
        z.push(format!(
            "Überdeckung {} / {}: {} m² – Zulässigkeit gesondert prüfen",
            u.wand_a, u.wand_b, u.flaeche_m2
        ));
    }
    for d in p.dokumente() {
        z.push(format!(
            "Quellbezug: {} · Version {}",
            d.dokument_id.0, d.version
        ));
    }
    z.push("Vektorplanskizze: G = Grundstück, W = Wand/Abstandsfläche, F = Sicherungs-/öffentliche Fläche oder Baukörper. Konflikte rot, Überdeckungen violett. Maßstab gilt bei unveränderter PDF-Ausgabe (100 %); grafischer Maßstab enthalten. Grundstücksflächen werden unabhängig von GRZ-/GFZ-Abzügen verwendet.".into());
    z
}
fn koordinaten(p: &[APunkt]) -> String {
    p.iter()
        .map(|v| format!("({}, {})", v.x, v.y))
        .collect::<Vec<_>>()
        .join("; ")
}
/// Erzeugt [`planseite`] ein Blatt? Ohne Geometrie entfällt die Skizze.
pub fn hat_planskizze(p: &AbstandsPlan, r: &AbstandsErgebnis) -> bool {
    p.grundstuecke.iter().any(|g| !g.grenze.is_empty())
        || p.flaechen.iter().any(|f| !f.polygon.is_empty())
        || r.waende.iter().any(|w| !w.polygon.is_empty())
        || !p.waende.is_empty()
}
/// Text für einen PDF-String der Planblätter: Klammern und Backslash
/// maskiert, höchstens 110 Zeichen. Die Kodierung übernimmt [`seite_anhaengen`].
fn ascii(t: &str) -> String {
    t.chars()
        .filter(|c| !c.is_control())
        .take(110)
        .flat_map(|c| match c {
            '(' | ')' | '\\' => vec!['\\', c],
            c => vec![c],
        })
        .collect()
}
/// WinAnsiEncoding (Windows-1252) für die Standardschrift; nicht darstellbare
/// Zeichen werden zu „?“.
fn winansi(t: &str) -> Vec<u8> {
    t.chars()
        .map(|c| match c {
            '€' => 0x80,
            '‚' => 0x82,
            '„' => 0x84,
            '…' => 0x85,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            c if (c as u32) < 0x80 || (0xA0..=0xFF).contains(&(c as u32)) => c as u32 as u8,
            _ => b'?',
        })
        .collect()
}
/// `kennung`: Zuordnung im Paket, z. B. Anlage und Blatt; erscheint unter dem Titel.
pub fn planseite(
    d: &mut lopdf::Document,
    p: &AbstandsPlan,
    r: &AbstandsErgebnis,
    kennung: Option<&str>,
) -> CoreResult<()> {
    let points = p
        .grundstuecke
        .iter()
        .flat_map(|g| &g.grenze)
        .chain(p.flaechen.iter().flat_map(|f| &f.polygon))
        .chain(r.waende.iter().flat_map(|w| &w.polygon))
        .chain(p.waende.iter().flat_map(|w| [&w.von, &w.bis]))
        .collect::<Vec<_>>();
    if points.is_empty() {
        return Ok(());
    }
    let minx = points.iter().map(|v| v.x).fold(f64::INFINITY, f64::min);
    let miny = points.iter().map(|v| v.y).fold(f64::INFINITY, f64::min);
    let maxx = points.iter().map(|v| v.x).fold(f64::NEG_INFINITY, f64::max);
    let maxy = points.iter().map(|v| v.y).fold(f64::NEG_INFINITY, f64::max);
    let nenner = ((2834.64567 * ((maxx - minx).max(1.) / 480.).max((maxy - miny).max(1.) / 595.))
        / 50.)
        .ceil()
        .max(1.)
        * 50.;
    let scale = 2834.64567 / nenner;
    let xy = |v: &APunkt| ((v.x - minx) * scale + 55., (v.y - miny) * scale + 115.);
    let mut out = format!(
        "q 0.6 w 0 0 0 RG BT /PlanFont 13 Tf 50 752 Td (Abstandsflächen – Vektorplanskizze) Tj ET BT /PlanFont 10 Tf 50 732 Td (1:{nenner:.0} bei 100 % PDF-Ausgabe; G/W/F siehe Bericht) Tj ET\n"
    );
    if let Some(k) = kennung {
        out.push_str(&format!(
            "BT /PlanFont 10 Tf 50 718 Td ({}) Tj ET\n",
            ascii(k)
        ));
    }
    let path = |pts: &[APunkt]| -> String {
        let mut s = String::new();
        for (i, v) in pts.iter().enumerate() {
            let (x, y) = xy(v);
            s.push_str(&format!(
                "{x:.5} {y:.5} {}\n",
                if i == 0 { "m" } else { "l" }
            ));
        }
        s.push_str("h\n");
        s
    };
    for (i, g) in p.grundstuecke.iter().enumerate() {
        if g.grenze.len() >= 3 {
            out.push_str("0 0 0 RG 1 w\n");
            out.push_str(&path(&g.grenze));
            for ring in &g.innenringe {
                out.push_str(&path(ring));
            }
            out.push_str("S\n");
            let (x, y) = xy(&g.grenze[0]);
            out.push_str(&format!(
                "BT /PlanFont 10 Tf {x} {y} Td (G{}) Tj ET\n",
                i + 1
            ));
        }
    }
    for (i, f) in p.flaechen.iter().enumerate() {
        if f.polygon.len() >= 3 {
            out.push_str(if f.art == Flaechenart::Baukoerper {
                "0.4 0.4 0.4 RG\n"
            } else {
                "0 0.5 0.1 RG\n"
            });
            out.push_str(&path(&f.polygon));
            out.push_str("S\n");
            let (x, y) = xy(&f.polygon[0]);
            out.push_str(&format!(
                "BT /PlanFont 10 Tf {x} {y} Td (F{}) Tj ET\n",
                i + 1
            ));
        }
    }
    for (i, w) in p.waende.iter().enumerate() {
        if let Some(e) = r.waende.iter().find(|e| e.id == w.id) {
            out.push_str("0 0.3 0.8 RG 0.6 w\n");
            if e.polygon.len() >= 3 {
                out.push_str(&path(&e.polygon));
                out.push_str("S\n");
            }
            let (x, y) = xy(&w.von);
            let (a, b) = xy(&w.bis);
            let werte = match (e.h_m, e.tiefe_m) {
                (Some(h), Some(t)) => format!(
                    " · H {} m · T {} m",
                    format!("{h:.2}").replace('.', ","),
                    format!("{t:.2}").replace('.', ",")
                ),
                _ => String::new(),
            };
            out.push_str(&format!(
                "1.4 w {x} {y} m {a} {b} l S BT /PlanFont 8 Tf {} {} Td (W{} · {}{}) Tj ET\n",
                (x + a) / 2.,
                (y + b) / 2. + 5.,
                i + 1,
                ascii(&w.bezeichnung),
                werte
            ));
            for k in &e.konflikte {
                out.push_str("1 0 0 RG 0.97 0.8 0.8 rg\n");
                out.push_str(&path(&k.aussen));
                for l in &k.innen {
                    out.push_str(&path(l));
                }
                out.push_str("B*\n");
            }
        }
    }
    for u in &r.ueberdeckungen {
        for poly in &u.polygone {
            out.push_str("0.7 0 0.7 RG\n");
            out.push_str(&path(&poly.aussen));
            out.push_str("S\n");
        }
    }
    let meter = (80. / scale).max(0.1);
    let b = meter * scale;
    out.push_str(&format!(
        "0 0 0 RG 2 w 55 95 m {} 95 l S BT /PlanFont 10 Tf 55 78 Td ({meter:.3} m) Tj ET Q",
        55. + b
    ));
    seite_anhaengen(d, out)
}
/// Hängt ein Blatt mit dem Inhaltsstrom `out` (Schrift `/PlanFont`) an.
fn seite_anhaengen(d: &mut lopdf::Document, out: String) -> CoreResult<()> {
    use lopdf::{Object, Stream, dictionary};
    let font = d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let content = d.add_object(Stream::new(dictionary! {}, winansi(&out)));
    let cat = d
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(|e| err(e.to_string()))?;
    let pages = d
        .get_dictionary(cat)
        .and_then(|v| v.get(b"Pages"))
        .and_then(Object::as_reference)
        .map_err(|e| err(e.to_string()))?;
    let page=d.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"PlanFont"=>font}},"Contents"=>content,"BauBericht"=>true});
    let count = d.get_pages().len() as i64;
    let root = d
        .get_dictionary_mut(pages)
        .map_err(|e| err(e.to_string()))?;
    root.get_mut(b"Kids")
        .and_then(Object::as_array_mut)
        .map_err(|e| err(e.to_string()))?
        .push(page.into());
    root.set("Count", count + 1);
    Ok(())
}
pub fn dxf(p: &AbstandsPlan, r: &AbstandsErgebnis) -> String {
    let mut s = String::from(
        "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n",
    );
    let mut poly = |name: &str, ps: &[APunkt]| {
        if ps.len() < 2 {
            return;
        }
        s.push_str(&format!(
            "0\nLWPOLYLINE\n8\n{name}\n90\n{}\n70\n1\n",
            ps.len()
        ));
        for p in ps {
            s.push_str(&format!("10\n{}\n20\n{}\n", p.x, p.y));
        }
    };
    for (i, g) in p.grundstuecke.iter().enumerate() {
        poly(&format!("GRUNDSTUECK_G{}", i + 1), &g.grenze);
        for r in &g.innenringe {
            poly(&format!("INNENRING_G{}", i + 1), r);
        }
    }
    for f in &p.flaechen {
        poly("ZUSATZFLAECHE", &f.polygon);
    }
    for (i, w) in r.waende.iter().enumerate() {
        poly(&format!("ABSTAND_W{}", i + 1), &w.polygon);
        for k in &w.konflikte {
            poly("KONFLIKT", &k.aussen);
            for l in &k.innen {
                poly("KONFLIKT_INNENRING", l);
            }
        }
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// Wände je Abwicklungsblatt.
const ABWICKLUNGEN_JE_BLATT: usize = 3;
/// Wände mit Höhenprofil im gespeicherten Ergebnis (ältere Ergebnisse haben keins).
fn abwicklungen<'a>(
    p: &'a AbstandsPlan,
    r: &'a AbstandsErgebnis,
) -> Vec<(usize, &'a AbstandsWand, &'a WandErgebnis)> {
    p.waende
        .iter()
        .enumerate()
        .filter_map(|(i, w)| {
            r.waende
                .iter()
                .find(|e| e.id == w.id && e.profil.len() >= 2 && e.laenge_m > 0.)
                .map(|e| (i, w, e))
        })
        .collect()
}
/// Anzahl der Blätter, die [`abwicklungsseiten`] erzeugt.
pub fn abwicklungsblaetter(p: &AbstandsPlan, r: &AbstandsErgebnis) -> usize {
    abwicklungen(p, r).len().div_ceil(ABWICKLUNGEN_JE_BLATT)
}
/// Wandabwicklungen: Geländelinie, Wandoberkante, Wandteile und Wandhöhen je
/// Profilstelle, maßstäblich in Wandrichtung (Anfang links). `kennung` erhält
/// die laufende Blattnummer innerhalb dieser Blätter (ab 1).
pub fn abwicklungsseiten(
    d: &mut lopdf::Document,
    p: &AbstandsPlan,
    r: &AbstandsErgebnis,
    kennung: impl Fn(usize) -> String,
) -> CoreResult<()> {
    let liste = abwicklungen(p, r);
    for (blatt, gruppe) in liste.chunks(ABWICKLUNGEN_JE_BLATT).enumerate() {
        let mut out = format!(
            "q 0 0 0 RG 0 0 0 rg BT /PlanFont 13 Tf 50 752 Td (Abstandsflächen – Wandabwicklungen) Tj ET BT /PlanFont 9 Tf 50 734 Td (Ansicht von außen, Höhen in m. Gelände braun, Wandoberkante schwarz, Wandteilgrenzen gestrichelt.) Tj ET BT /PlanFont 10 Tf 50 718 Td ({}) Tj ET\n",
            ascii(&kennung(blatt + 1))
        );
        for (band, (i, w, e)) in gruppe.iter().enumerate() {
            out.push_str(&abwicklung(*i, w, e, 700. - band as f64 * 220.));
        }
        out.push('Q');
        seite_anhaengen(d, out)?;
    }
    Ok(())
}
fn abwicklung(i: usize, w: &AbstandsWand, e: &WandErgebnis, oben: f64) -> String {
    let z = |v: f64| opt(Some(v));
    let (x0, breite, hoehe) = (70., 470., 120.);
    let unten = oben - 40. - hoehe;
    let hmin = e
        .profil
        .iter()
        .map(|q| q.gelaende_m)
        .fold(f64::INFINITY, f64::min);
    let hmax = e
        .profil
        .iter()
        .map(|q| q.oberkante_m)
        .fold(f64::NEG_INFINITY, f64::max);
    // Ein Maßstab für Länge und Höhe, damit die Ansicht nicht verzerrt.
    let m = (breite / e.laenge_m).min(hoehe / (hmax - hmin).max(0.5));
    // Ansicht von außen: Liegt die Außenseite links der Wandrichtung, steht
    // der Wandanfang für den Betrachter rechts.
    let gespiegelt = w.aussen == Aussenrichtung::Links;
    let xy = |s: f64, h: f64| {
        let s = if gespiegelt { e.laenge_m - s } else { s };
        (x0 + s * m, unten + (h - hmin) * m)
    };
    let mut o = format!(
        "BT /PlanFont 10 Tf 50 {oben} Td ({}) Tj ET BT /PlanFont 7 Tf 50 {} Td ({}) Tj ET\n",
        ascii(&format!(
            "W{} – {} / {} – Länge {} m",
            i + 1,
            w.gebaeude,
            w.bezeichnung,
            z(e.laenge_m)
        )),
        oben - 11.,
        ascii(&format!(
            "{}; Wandanfang (Station 0) {}",
            e.ansatz.unwrap_or_default().bezeichnung(),
            if gespiegelt { "rechts" } else { "links" }
        ))
    );
    let linie = |o: &mut String, pts: Vec<(f64, f64)>| {
        for (k, (x, y)) in pts.iter().enumerate() {
            o.push_str(&format!(
                "{x:.2} {y:.2} {}\n",
                if k == 0 { "m" } else { "l" }
            ));
        }
    };
    // Wandfläche
    let mut flaeche: Vec<_> = e
        .profil
        .iter()
        .map(|q| xy(q.station_m, q.oberkante_m))
        .collect();
    flaeche.extend(e.profil.iter().rev().map(|q| xy(q.station_m, q.gelaende_m)));
    o.push_str("0.92 g\n");
    linie(&mut o, flaeche);
    o.push_str("h f 0 g\n");
    o.push_str("0 0 0 RG 1.2 w\n");
    linie(
        &mut o,
        e.profil
            .iter()
            .map(|q| xy(q.station_m, q.oberkante_m))
            .collect(),
    );
    o.push_str("S 0.55 0.35 0.1 RG 1.6 w\n");
    linie(
        &mut o,
        e.profil
            .iter()
            .map(|q| xy(q.station_m, q.gelaende_m))
            .collect(),
    );
    o.push_str("S 0 0 0 RG 0.6 w\n");
    for a in [e.profil.first(), e.profil.last()].into_iter().flatten() {
        let (x, y1) = xy(a.station_m, a.gelaende_m);
        let (_, y2) = xy(a.station_m, a.oberkante_m);
        o.push_str(&format!("{x:.2} {y1:.2} m {x:.2} {y2:.2} l S\n"));
    }
    o.push_str("[3 2] 0 d\n");
    for t in e.wandteile.iter().skip(1) {
        let q = e
            .profil
            .iter()
            .filter(|q| (q.station_m - t.von_m).abs() < 1e-6)
            .fold(None::<(f64, f64)>, |acc, q| {
                Some(acc.map_or((q.gelaende_m, q.oberkante_m), |(g, ok)| {
                    (g.min(q.gelaende_m), ok.max(q.oberkante_m))
                }))
            });
        if let Some((g, ok)) = q {
            let (x, y1) = xy(t.von_m, g);
            let (_, y2) = xy(t.von_m, ok);
            o.push_str(&format!("{x:.2} {y1:.2} m {x:.2} {y2:.2} l S\n"));
        }
    }
    o.push_str("[] 0 d\n");
    // Beschriftung je Profilstelle, bei vielen Stellen ausgedünnt.
    let schritt = e.profil.len().div_ceil(12).max(1);
    let mut letzte = f64::NEG_INFINITY;
    for q in e.profil.iter().step_by(schritt) {
        let (x, yg) = xy(q.station_m, q.gelaende_m);
        let (_, yo) = xy(q.station_m, q.oberkante_m);
        if (x - letzte).abs() < 34. {
            continue;
        }
        letzte = x;
        o.push_str(&format!(
            "BT /PlanFont 6 Tf {:.2} {:.2} Td ({} m) Tj ET BT /PlanFont 6 Tf {:.2} {:.2} Td (G {}) Tj ET BT /PlanFont 6 Tf {:.2} {:.2} Td (OK {}) Tj ET BT /PlanFont 6 Tf {:.2} {:.2} Td (H {}) Tj ET\n",
            x - 6.,
            unten - 17.,
            z(q.station_m),
            x + 2.,
            yg - 8.,
            z(q.gelaende_m),
            x + 2.,
            yo + 3.,
            z(q.oberkante_m),
            x + 2.,
            (yg + yo) / 2.,
            z(q.wandhoehe_m)
        ));
    }
    // Wandteile als Zeilen unter der Ansicht.
    for (k, t) in e.wandteile.iter().take(3).enumerate() {
        o.push_str(&format!(
            "BT /PlanFont 7 Tf 50 {:.2} Td (Wandteil {}: Station {} bis {} m – H an den Kanten {} / {} m – maßgeblich {} m – mit Dach {} m – Tiefe {} m) Tj ET\n",
            unten - 28. - k as f64 * 9.,
            k + 1,
            z(t.von_m),
            z(t.bis_m),
            z(t.h_anfang_m),
            z(t.h_ende_m),
            z(t.wandhoehe_m),
            opt(t.h_m),
            opt(t.tiefe_m)
        ));
    }
    if e.wandteile.len() > 3 {
        o.push_str(&format!(
            "BT /PlanFont 7 Tf 50 {:.2} Td (Weitere {} Wandteile siehe Bericht) Tj ET\n",
            unten - 55.,
            e.wandteile.len() - 3
        ));
    }
    o
}
