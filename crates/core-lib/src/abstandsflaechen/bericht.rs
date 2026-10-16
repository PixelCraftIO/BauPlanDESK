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
            z.push(format!(
                "Gelände A/B: {} / {} m · Wandabschluss A/B: {} / {} m · Quelle: {}",
                opt(w.gelaende_a_m),
                opt(w.gelaende_b_m),
                opt(w.abschluss_a_m),
                opt(w.abschluss_b_m),
                w.hoehenquelle
            ));
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
pub fn planseite(
    d: &mut lopdf::Document,
    p: &AbstandsPlan,
    r: &AbstandsErgebnis,
) -> CoreResult<()> {
    use lopdf::{Object, Stream, dictionary};
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
        "q 0.6 w 0 0 0 RG BT /PlanFont 13 Tf 50 752 Td (Abstandsflaechen - Vektorplanskizze) Tj ET BT /PlanFont 10 Tf 50 732 Td (1:{nenner:.0} bei 100 Prozent PDF-Ausgabe; G/W/F siehe Bericht) Tj ET\n"
    );
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
            out.push_str(&format!(
                "1.4 w {x} {y} m {a} {b} l S BT /PlanFont 10 Tf {} {} Td (W{}) Tj ET\n",
                (x + a) / 2.,
                (y + b) / 2. + 5.,
                i + 1
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
    let font =
        d.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content = d.add_object(Stream::new(dictionary! {}, out.into_bytes()));
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
