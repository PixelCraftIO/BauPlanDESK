//! Parametrisierte LBO-Prüfung: keine pauschale Definition für alle Bundesländer.
pub const RECHENSTAND: &str = "vollgeschosse-2";
use super::*;
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    pub mindesthoehe_m: f64,
    pub hoehe_streng: bool,
    pub mindestanteil: f64,
    pub anteil_streng: bool,
    pub vergleich_unterliegend: bool,
    pub oberirdisch_pruefen: bool,
    pub decke_ueber_gelaende_m: f64,
    pub oberirdisch_streng: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geschoss {
    pub id: String,
    #[serde(default)]
    pub gebaeude_geschoss_id: Option<Uuid>,
    pub bezeichnung: String,
    pub grundflaeche_m2: f64,
    pub unterliegend_m2: Option<f64>,
    pub decke_ueber_gelaende_m: Option<f64>,
    pub hoehenquelle: String,
    pub sonderentscheidung: Option<bool>,
    pub sondergrundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hoehenflaeche {
    pub id: String,
    pub geschoss_id: String,
    pub bezeichnung: String,
    pub flaeche_m2: f64,
    pub lichte_hoehe_m: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub regeln: Regeln,
    pub geschosse: Vec<Geschoss>,
    pub teilflaechen: Vec<Hoehenflaeche>,
    pub aufmass_geprueft: bool,
    pub sonderfaelle_geprueft: bool,
    pub maximal_vollgeschosse: Option<f64>,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Vollgeschosse);
    let a = &e.regeln;
    wert(a.mindesthoehe_m, "Mindesthöhe")?;
    quote(a.mindestanteil, "Mindestanteil")?;
    wert(a.decke_ueber_gelaende_m, "Geländeschwelle")?;
    if a.mindesthoehe_m <= 0. || a.mindestanteil <= 0. {
        return Err(err("Höhen- und Flächenkriterium müssen positiv sein"));
    }
    ids(e.geschosse.iter().map(|p| p.id.as_str()))?;
    ids(e.teilflaechen.iter().map(|p| p.id.as_str()))?;
    let mut fl: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut teile = vec![];
    for t in &e.teilflaechen {
        let g = e
            .geschosse
            .iter()
            .find(|g| g.id == t.geschoss_id)
            .ok_or_else(|| err("Höhenzone verweist auf unbekanntes Geschoss"))?;
        wert(t.flaeche_m2, "Teilfläche")?;
        wert(t.lichte_hoehe_m, "Lichte Höhe")?;
        let erfuellt = if a.hoehe_streng {
            t.lichte_hoehe_m > a.mindesthoehe_m
        } else {
            t.lichte_hoehe_m >= a.mindesthoehe_m
        };
        let f = fl.entry(t.geschoss_id.clone()).or_default();
        f.0 += t.flaeche_m2;
        if erfuellt {
            f.1 += t.flaeche_m2;
        }
        teile.push(vec![
            g.bezeichnung.clone(),
            t.bezeichnung.clone(),
            zahl(t.flaeche_m2),
            zahl(t.lichte_hoehe_m),
            format!(
                "{} {} m",
                if a.hoehe_streng { ">" } else { "≥" },
                zahl(a.mindesthoehe_m)
            ),
            if erfuellt { "Ja" } else { "Nein" }.into(),
        ]);
    }
    r.tabelle(
        "Höhenzonen",
        &[
            "Geschoss",
            "Zone",
            "Fläche m²",
            "Höhe m",
            "Kriterium",
            "Anrechenbar",
        ],
        teile,
    );
    let mut anzahl = 0.;
    let mut alle_eingestuft = !e.geschosse.is_empty();
    let mut rows = vec![];
    for g in &e.geschosse {
        text(&g.bezeichnung)?;
        wert(g.grundflaeche_m2, "Geschossgrundfläche")?;
        if let Some(n) = g.unterliegend_m2 {
            wert(n, "Vergleichsfläche")?;
        }
        let f = fl.get(&g.id).copied().unwrap_or_default();
        if f.0 > g.grundflaeche_m2 + 0.01 {
            return Err(err("Höhenzonen übersteigen die Geschossgrundfläche"));
        }
        if (f.0 - g.grundflaeche_m2).abs() > 0.01 {
            r.offen(format!(
                "{}: Höhenzonen decken die Grundfläche nicht vollständig ab",
                g.bezeichnung
            ));
        }
        if g.hoehenquelle.trim().is_empty() {
            r.offen(format!("{}: Höhen-/Geländequelle angeben", g.bezeichnung));
        }
        let bezug = if a.vergleich_unterliegend {
            g.unterliegend_m2
        } else {
            Some(g.grundflaeche_m2)
        };
        let anteil = bezug
            .filter(|v| *v > 0. && (f.0 - g.grundflaeche_m2).abs() <= 0.01)
            .map(|v| f.1 / v);
        let oberirdisch = if a.oberirdisch_pruefen {
            g.decke_ueber_gelaende_m.map(|v| {
                if a.oberirdisch_streng {
                    v > a.decke_ueber_gelaende_m
                } else {
                    v >= a.decke_ueber_gelaende_m
                }
            })
        } else {
            Some(true)
        };
        if let Some(v) = g.decke_ueber_gelaende_m
            && (!v.is_finite() || v.abs() > 1e6)
        {
            return Err(err("Ungültiger Geländeabstand"));
        }
        let fach = anteil.zip(oberirdisch).map(|(q, o)| {
            o && if a.anteil_streng {
                q > a.mindestanteil
            } else {
                q >= a.mindestanteil
            }
        });
        if fach.is_none() {
            r.offen(format!(
                "{}: Vergleichsfläche oder Geländebezug ungeklärt",
                g.bezeichnung
            ));
        }
        let ergebnis = if let Some(s) = g.sonderentscheidung {
            if g.sondergrundlage.trim().is_empty() || !e.sonderfaelle_geprueft {
                r.offen(format!(
                    "{}: Sonderentscheidung begründen und prüfen",
                    g.bezeichnung
                ));
            }
            Some(s)
        } else {
            fach
        };
        if ergebnis.is_none() {
            alle_eingestuft = false;
        }
        if ergebnis == Some(true) {
            anzahl += 1.;
        }
        r.kennzahl(
            format!("geschoss:{}", g.id),
            format!("Vollgeschoss {} (1 = ja)", g.bezeichnung),
            ergebnis.map(|v| if v { 1. } else { 0. }),
            Einheit::Anzahl,
            Messgroesse::Sonstiges,
        );
        rows.push(vec![
            g.bezeichnung.clone(),
            zahl(f.1),
            bezug.map(zahl).unwrap_or_else(|| "Offen".into()),
            anteil
                .map(|v| format!("{} %", zahl(v * 100.)))
                .unwrap_or_else(|| "Offen".into()),
            format!(
                "{} {} %",
                if a.anteil_streng { ">" } else { "≥" },
                zahl(a.mindestanteil * 100.)
            ),
            oberirdisch
                .map(|v| if v { "Ja" } else { "Nein" })
                .unwrap_or("Offen")
                .into(),
            ergebnis
                .map(|v| {
                    if v {
                        "Vollgeschoss"
                    } else {
                        "Kein Vollgeschoss"
                    }
                })
                .unwrap_or("Offen")
                .into(),
            format!("{} · {}", g.hoehenquelle, g.sondergrundlage),
        ]);
    }
    r.tabelle(
        "Vollgeschossprüfung",
        &[
            "Geschoss",
            "Qualifizierte Fläche m²",
            "Vergleichsfläche m²",
            "Anteil",
            "Flächenkriterium",
            "Oberirdisch",
            "Einordnung",
            "Quelle / Sonderentscheidung",
        ],
        rows,
    );
    if e.geschosse.is_empty() || !e.aufmass_geprueft || !e.sonderfaelle_geprueft {
        r.offen(
            "Vollständigkeit von Aufmaß, Gelände-/Höhendefinition und LBO-Sonderfällen bestätigen",
        );
    }
    if let Some(max) = e.maximal_vollgeschosse {
        wert(max, "Maximale Vollgeschosszahl")?;
        if max.fract() != 0. {
            return Err(err("Vollgeschosszahl muss ganzzahlig sein"));
        }
        if anzahl > max {
            r.offen(format!(
                "{anzahl} Vollgeschosse überschreiten die Festsetzung {}",
                zahl(max)
            ));
        }
    }
    r.kennzahl(
        "vollgeschosse",
        "Anzahl Vollgeschosse",
        alle_eingestuft.then_some(anzahl),
        Einheit::Anzahl,
        Messgroesse::Sonstiges,
    );
    r.hinweise.push("Flächen-, Höhen- und Oberirdisch-Kriterien sowie Sonderentscheidungen beziehen sich ausschließlich auf das bestätigte LBO-Regelpaket. Keine bundesweit einheitliche Vollgeschossdefinition.".into());
    Ok(r)
}
