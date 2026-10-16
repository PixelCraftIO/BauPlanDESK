//! Raum-/Wohnungssummen mit aufgeschlüsseltem Aufmaß und getrennten Abzügen.
pub const RECHENSTAND: &str = "wohnflaechen-1";
use super::*;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    Rechteck,
    Dreieck,
    Trapez,
    Aufmass,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raumregel {
    pub typ: String,
    pub bezeichnung: String,
    pub faktor: f64,
    pub maximaler_faktor: f64,
    pub hoehenabhaengig: bool,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    pub niedrige_hoehe_m: f64,
    pub volle_hoehe_m: f64,
    pub faktor_niedrig: f64,
    pub faktor_mittel: f64,
    pub faktor_hoch: f64,
    pub raumregeln: Vec<Raumregel>,
    pub treppe_ab_stufen: u32,
    pub pfeiler_hoehe_ueber_m: f64,
    pub pfeiler_flaeche_ueber_m2: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Teilflaeche {
    pub id: String,
    pub wohnung: String,
    pub raum: String,
    pub typ: String,
    pub form: Form,
    pub laenge_m: f64,
    pub breite_m: f64,
    pub breite2_m: f64,
    pub aufmass_m2: f64,
    pub lichte_hoehe_m: Option<f64>,
    pub sonderfaktor: Option<f64>,
    pub begruendung: String,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Abzug {
    pub id: String,
    pub teilflaeche: String,
    pub bezeichnung: String,
    pub flaeche_m2: f64,
    pub typ: String,
    pub hoehe_m: f64,
    pub stufen: u32,
    pub grundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub regeln: Regeln,
    pub positionen: Vec<Teilflaeche>,
    pub abzuege: Vec<Abzug>,
    pub aufmass_vollstaendig: bool,
    #[serde(default)]
    pub wohneinheiten_bestaetigt: bool,
}
pub(crate) fn aufmass(form: Form, l: f64, b: f64, b2: f64, area: f64) -> CoreResult<f64> {
    for n in [l, b, b2, area] {
        wert(n, "Aufmaß")?;
    }
    Ok(match form {
        Form::Rechteck => l * b,
        Form::Dreieck => l * b / 2.,
        Form::Trapez => l * (b + b2) / 2.,
        Form::Aufmass => area,
    })
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Wohnflaechen);
    let a = &e.regeln;
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    ids(e.abzuege.iter().map(|p| p.id.as_str()))?;
    ids(a.raumregeln.iter().map(|p| p.typ.as_str()))?;
    wert(a.pfeiler_hoehe_ueber_m, "Abzugshöhe")?;
    wert(a.pfeiler_flaeche_ueber_m2, "Abzugsgrenze")?;
    if a.treppe_ab_stufen < 1 {
        return Err(err("Treppenschwelle muss positiv sein"));
    }
    wert(a.niedrige_hoehe_m, "Höhengrenze")?;
    wert(a.volle_hoehe_m, "Höhengrenze")?;
    if a.niedrige_hoehe_m >= a.volle_hoehe_m {
        return Err(err("Höhenzonen müssen aufsteigend sein"));
    }
    for q in [a.faktor_niedrig, a.faktor_mittel, a.faktor_hoch] {
        quote(q, "Höhenfaktor")?;
    }
    for q in &a.raumregeln {
        quote(q.faktor, "Raumfaktor")?;
        quote(q.maximaler_faktor, "Maximaler Raumfaktor")?;
        text(&q.bezeichnung)?;
        text(&q.quelle)?;
        if q.faktor > q.maximaler_faktor {
            return Err(err("Raumfaktor über maximalem Faktor"));
        }
    }
    let mut abzuege: BTreeMap<String, f64> = BTreeMap::new();
    let mut az = vec![];
    for b in &e.abzuege {
        let p = e
            .positionen
            .iter()
            .find(|p| p.id == b.teilflaeche)
            .ok_or_else(|| err("Abzug verweist auf unbekannte Teilfläche"))?;
        wert(b.flaeche_m2, "Abzugsfläche")?;
        wert(b.hoehe_m, "Bauteilhöhe")?;
        text(&b.bezeichnung)?;
        text(&b.grundlage)?;
        let abziehen = match b.typ.as_str() {
            "treppe" => b.stufen >= a.treppe_ab_stufen,
            "pfeiler_schornstein" => {
                b.hoehe_m > a.pfeiler_hoehe_ueber_m && b.flaeche_m2 > a.pfeiler_flaeche_ueber_m2
            }
            "sonstiges" => true,
            _ => return Err(err("Unbekannter Abzugstyp")),
        };
        if abziehen {
            *abzuege.entry(b.teilflaeche.clone()).or_default() += b.flaeche_m2;
        }
        az.push(vec![
            format!("{} / {}", p.wohnung, p.raum),
            b.bezeichnung.clone(),
            zahl(b.flaeche_m2),
            if abziehen {
                "Abgezogen"
            } else {
                "Schwellenwerte nicht erreicht"
            }
            .into(),
            b.grundlage.clone(),
        ]);
    }
    let mut gruppen: BTreeMap<(String, String), f64> = BTreeMap::new();
    let mut wohnungen: BTreeMap<String, f64> = BTreeMap::new();
    let mut rows = vec![];
    for p in &e.positionen {
        text(&p.wohnung)?;
        text(&p.raum)?;
        if p.quelle.trim().is_empty() {
            r.offen(format!("Aufmaßquelle fehlt: {} / {}", p.wohnung, p.raum));
        }
        if let Some(h) = p.lichte_hoehe_m {
            wert(h, "Lichte Höhe")?;
        }
        let regel = a
            .raumregeln
            .iter()
            .find(|r| r.typ == p.typ)
            .ok_or_else(|| err("Raumtyp fehlt im gewählten Regelpaket"))?;
        let brutto = aufmass(p.form, p.laenge_m, p.breite_m, p.breite2_m, p.aufmass_m2)?;
        if brutto <= 0. {
            r.offen(format!("{}: Fläche noch nicht erfasst", p.raum));
        }
        let abzug = *abzuege.get(&p.id).unwrap_or(&0.);
        if abzug > brutto + 1e-8 {
            return Err(err("Abzugsfläche ist größer als die Teilfläche"));
        }
        let hoehenfaktor = if !regel.hoehenabhaengig {
            1.
        } else if let Some(h) = p.lichte_hoehe_m {
            if h < a.niedrige_hoehe_m {
                a.faktor_niedrig
            } else if h < a.volle_hoehe_m {
                a.faktor_mittel
            } else {
                a.faktor_hoch
            }
        } else {
            r.offen(format!("{}: Lichte Höhe fehlt", p.raum));
            0.
        };
        let faktor = p.sonderfaktor.unwrap_or(regel.faktor);
        quote(faktor, "Sonderfaktor")?;
        if faktor > regel.maximaler_faktor + 1e-9 {
            return Err(err(
                "Sonderfaktor überschreitet den konfigurierten Höchstansatz",
            ));
        }
        if p.sonderfaktor.is_some() && p.begruendung.trim().is_empty() {
            r.offen(format!("Abweichenden Raumfaktor begründen: {}", p.raum));
        }
        let fl = (brutto - abzug).max(0.) * hoehenfaktor * faktor;
        *gruppen
            .entry((p.wohnung.clone(), p.raum.clone()))
            .or_default() += fl;
        *wohnungen.entry(p.wohnung.clone()).or_default() += fl;
        let rechenweg = match p.form {
            Form::Rechteck => format!("{} × {}", zahl(p.laenge_m), zahl(p.breite_m)),
            Form::Dreieck => format!("{} × {} / 2", zahl(p.laenge_m), zahl(p.breite_m)),
            Form::Trapez => format!(
                "{} × ({} + {}) / 2",
                zahl(p.laenge_m),
                zahl(p.breite_m),
                zahl(p.breite2_m)
            ),
            Form::Aufmass => "Nach Aufmaß".into(),
        };
        rows.push(vec![
            format!("{} / {}", p.wohnung, p.raum),
            regel.bezeichnung.clone(),
            rechenweg,
            zahl(brutto),
            zahl(abzug),
            p.lichte_hoehe_m
                .map(zahl)
                .unwrap_or_else(|| "Nicht erfasst".into()),
            zahl(hoehenfaktor),
            zahl(faktor),
            zahl(fl),
            format!("{} · {}", p.quelle, p.begruendung),
        ]);
    }
    r.tabelle(
        "Teilflächen und Anrechnung",
        &[
            "Wohnung / Raum",
            "Raumart",
            "Aufmaß (m)",
            "Brutto m²",
            "Abzug m²",
            "Lichte Höhe m",
            "Höhenfaktor",
            "Raumfaktor",
            "Wohnfläche m²",
            "Quelle / Begründung",
        ],
        rows,
    );
    r.tabelle(
        "Abzugsprüfung",
        &[
            "Wohnung / Raum",
            "Bauteil",
            "Fläche m²",
            "Anrechnung",
            "Grundlage",
        ],
        az,
    );
    r.tabelle(
        "Raumsummen",
        &["Wohnung", "Raum", "Wohnfläche m²"],
        gruppen
            .iter()
            .map(|((w, raum), a)| vec![w.clone(), raum.clone(), zahl(*a)])
            .collect(),
    );
    r.tabelle(
        "Wohnungssummen",
        &["Wohnung", "Wohnfläche m²"],
        wohnungen
            .iter()
            .map(|(w, a)| vec![w.clone(), zahl(*a)])
            .collect(),
    );
    for (w, a) in &wohnungen {
        r.kennzahl(
            format!("wohnung:{w}"),
            format!("Wohnfläche {w}"),
            Some(*a),
            Einheit::M2,
            Messgroesse::Wohnflaeche,
        );
    }
    r.kennzahl(
        "wohnflaeche",
        "Wohnfläche gesamt",
        Some(wohnungen.values().sum()),
        Einheit::M2,
        Messgroesse::Wohnflaeche,
    );
    r.kennzahl(
        "wohneinheiten",
        "Erfasste Wohneinheiten",
        e.wohneinheiten_bestaetigt.then_some(wohnungen.len() as f64),
        Einheit::Anzahl,
        Messgroesse::Wohneinheiten,
    );
    if e.positionen.is_empty() || !e.aufmass_vollstaendig {
        r.offen("Vollständigkeit des Aufmaßes, der Höhenzonen und der Abzüge prüfen");
    }
    r.hinweise.push("Die Berechnung setzt korrekt zugeordnete Raumarten und überschneidungsfreie Teilflächen voraus. Wohnfläche ist weder DIN-277-Fläche noch GFZ-Geschossfläche.".into());
    Ok(r)
}
