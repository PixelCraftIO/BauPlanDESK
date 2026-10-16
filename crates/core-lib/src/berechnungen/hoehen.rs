//! Nachvollziehbare absolute/relative Höhen und Dachneigungen aus vermessenen Punkten.
pub const RECHENSTAND: &str = "hoehen-1";
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grenze {
    pub art: String,
    pub minimum_m: Option<f64>,
    pub maximum_m: Option<f64>,
    pub absolut: bool,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Punkt {
    pub id: String,
    pub bezeichnung: String,
    pub art: String,
    pub absolut_m: f64,
    pub bezug_m: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dach {
    pub id: String,
    pub bezeichnung: String,
    pub traufe_id: String,
    pub first_id: String,
    pub horizontale_m: f64,
    pub min_neigung_grad: Option<f64>,
    pub max_neigung_grad: Option<f64>,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub hoehenbezug: String,
    pub regeln: Vec<Grenze>,
    pub punkte: Vec<Punkt>,
    pub daecher: Vec<Dach>,
    pub bezug_geprueft: bool,
}
fn hoehe(v: f64) -> CoreResult<()> {
    if !v.is_finite() || v.abs() > 1e6 {
        Err(err("Höhe muss endlich und plausibel sein"))
    } else {
        Ok(())
    }
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Hoehen);
    ids(e.punkte.iter().map(|p| p.id.as_str()))?;
    ids(e.daecher.iter().map(|p| p.id.as_str()))?;
    ids(e.regeln.iter().map(|p| p.art.as_str()))?;
    let mut rows = vec![];
    for g in &e.regeln {
        for n in [g.minimum_m, g.maximum_m].into_iter().flatten() {
            hoehe(n)?;
        }
        text(&g.quelle)?;
        if g.minimum_m.zip(g.maximum_m).is_some_and(|(a, b)| a > b) {
            return Err(err("Höhengrenzen vertauscht"));
        }
    }
    for p in &e.punkte {
        text(&p.bezeichnung)?;
        hoehe(p.absolut_m)?;
        hoehe(p.bezug_m)?;
        let h = p.absolut_m - p.bezug_m;
        let gr = e.regeln.iter().find(|g| g.art == p.art);
        let ist = if gr.is_some_and(|g| g.absolut) {
            p.absolut_m
        } else {
            h
        };
        let ok = gr.map(|g| {
            g.minimum_m.is_none_or(|min| ist >= min) && g.maximum_m.is_none_or(|max| ist <= max)
        });
        if ok == Some(false) {
            r.offen(format!(
                "{}: Höhenfestsetzung überschritten / unterschritten",
                p.bezeichnung
            ));
        }
        if p.quelle.trim().is_empty() {
            r.offen(format!("{}: Höhenquelle fehlt", p.bezeichnung));
        }
        rows.push(vec![
            p.bezeichnung.clone(),
            match p.art.as_str() {
                "traufe" => "Traufhöhe",
                "first" => "Firsthöhe",
                "gebaeude" => "Gebäudehöhe",
                _ => &p.art,
            }
            .into(),
            zahl(p.absolut_m),
            zahl(p.bezug_m),
            zahl(h),
            gr.map(|g| {
                format!(
                    "{} / {} ({})",
                    g.minimum_m
                        .map(zahl)
                        .unwrap_or_else(|| "Keine Untergrenze".into()),
                    g.maximum_m
                        .map(zahl)
                        .unwrap_or_else(|| "Keine Obergrenze".into()),
                    if g.absolut { "absolut" } else { "relativ" }
                )
            })
            .unwrap_or_else(|| "Keine Festsetzung hinterlegt".into()),
            ok.map(|v| if v { "Eingehalten" } else { "Abweichung" })
                .unwrap_or("Nicht verglichen")
                .into(),
            p.quelle.clone(),
        ]);
        r.kennzahl(
            format!("hoehe:{}", p.id),
            format!("Relative Höhe {}", p.bezeichnung),
            Some(h),
            Einheit::M,
            Messgroesse::Hoehe,
        );
    }
    r.tabelle(
        "Höhenprüfung",
        &[
            "Punkt",
            "Art",
            "Absolute Höhe m",
            "Bezugshöhe m",
            "Differenz m",
            "Vorgabe min/max m",
            "Ergebnis",
            "Quelle",
        ],
        rows,
    );
    let mut dr = vec![];
    for d in &e.daecher {
        text(&d.bezeichnung)?;
        wert(d.horizontale_m, "Horizontale Dachlänge")?;
        if d.horizontale_m <= 0. {
            return Err(err("Horizontale Dachlänge muss positiv sein"));
        }
        let a = e
            .punkte
            .iter()
            .find(|p| p.id == d.traufe_id)
            .ok_or_else(|| err("Traufpunkt fehlt"))?;
        let b = e
            .punkte
            .iter()
            .find(|p| p.id == d.first_id)
            .ok_or_else(|| err("Firstpunkt fehlt"))?;
        if a.art != "traufe" || b.art != "first" {
            return Err(err(
                "Dachneigung benötigt einen Trauf- und einen Firstpunkt im gemeinsamen Höhensystem",
            ));
        }
        let dh = b.absolut_m - a.absolut_m;
        if dh < 0. {
            return Err(err("First liegt unter Traufe"));
        }
        let winkel = (dh / d.horizontale_m).atan().to_degrees();
        for n in [d.min_neigung_grad, d.max_neigung_grad]
            .into_iter()
            .flatten()
        {
            if !n.is_finite() || !(0. ..90.).contains(&n) {
                return Err(err("Dachneigung zwischen 0 und unter 90 Grad erforderlich"));
            }
        }
        if d.min_neigung_grad
            .zip(d.max_neigung_grad)
            .is_some_and(|(a, b)| a > b)
        {
            return Err(err("Neigungsgrenzen vertauscht"));
        }
        let ok = d.min_neigung_grad.is_none_or(|m| winkel >= m)
            && d.max_neigung_grad.is_none_or(|m| winkel <= m);
        if !ok {
            r.offen(format!(
                "{}: Dachneigung weicht von Festsetzung ab",
                d.bezeichnung
            ));
        }
        if d.quelle.trim().is_empty() {
            r.offen(format!("{}: Dachaufmaßquelle fehlt", d.bezeichnung));
        }
        dr.push(vec![
            d.bezeichnung.clone(),
            a.bezeichnung.clone(),
            b.bezeichnung.clone(),
            zahl(dh),
            zahl(d.horizontale_m),
            format!("atan({} / {})", zahl(dh), zahl(d.horizontale_m)),
            zahl(winkel),
            if ok {
                "Kein Widerspruch zu hinterlegten Grenzen"
            } else {
                "Abweichung"
            }
            .into(),
            d.quelle.clone(),
        ]);
        r.kennzahl(
            format!("dach:{}", d.id),
            format!("Dachneigung {}", d.bezeichnung),
            Some(winkel),
            Einheit::Grad,
            Messgroesse::Sonstiges,
        );
    }
    r.tabelle(
        "Dachneigungen",
        &[
            "Dach",
            "Traufe",
            "First",
            "Höhendifferenz m",
            "Horizontale m",
            "Rechenweg",
            "Neigung °",
            "Vergleich",
            "Quelle",
        ],
        dr,
    );
    if e.hoehenbezug.trim().is_empty() || !e.bezug_geprueft || e.punkte.is_empty() {
        r.offen("Höhenbezug, Gelände-/Bezugspunkte und Aufmaß vollständig prüfen");
    }
    r.hinweise.push(format!(
        "Höhenbezug: {}. Fehlende Festsetzungen bedeuten keinen positiven Zulässigkeitsnachweis.",
        e.hoehenbezug
    ));
    Ok(r)
}
