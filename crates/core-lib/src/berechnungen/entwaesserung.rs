//! Transparente Volumenbilanz mit konstanten Regenblöcken; kein Kanalnetz-/Überflutungsmodell.
pub const RECHENSTAND: &str = "entwaesserung-1";
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Abflussklasse {
    pub typ: String,
    pub bezeichnung: String,
    pub beiwert: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regen {
    pub id: String,
    pub dauer_min: f64,
    pub intensitaet_l_s_ha: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    pub klassen: Vec<Abflussklasse>,
    pub regen: Vec<Regen>,
    pub sicherheitsfaktor: f64,
    pub drossel_l_s: f64,
    pub kf_m_s: f64,
    pub sickerflaeche_m2: f64,
    pub versickerungsfaktor: f64,
    pub versickerung_geprueft: bool,
    pub boden_grundwasser_vermerk: String,
    pub vorhandenes_volumen_m3: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flaeche {
    pub id: String,
    pub bezeichnung: String,
    pub typ: String,
    pub flaeche_m2: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub regeln: Regeln,
    pub positionen: Vec<Flaeche>,
    pub flaechen_geprueft: bool,
    pub regendaten_geprueft: bool,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Entwaesserung);
    let a = &e.regeln;
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    ids(a.klassen.iter().map(|p| p.typ.as_str()))?;
    ids(a.regen.iter().map(|p| p.id.as_str()))?;
    for n in [
        a.sicherheitsfaktor,
        a.drossel_l_s,
        a.kf_m_s,
        a.sickerflaeche_m2,
        a.versickerungsfaktor,
    ] {
        wert(n, "Entwässerungsansatz")?;
    }
    if a.sicherheitsfaktor < 1.
        || a.sicherheitsfaktor > 10.
        || a.versickerungsfaktor > 1.
        || a.kf_m_s > 0.1
    {
        return Err(err(
            "Sicherheits-/Versickerungsparameter außerhalb plausibler Grenzen",
        ));
    }
    if let Some(v) = a.vorhandenes_volumen_m3 {
        wert(v, "Vorhandenes Rückhaltevolumen")?;
    }
    for k in &a.klassen {
        quote(k.beiwert, "Abflussbeiwert")?;
        text(&k.quelle)?;
    }
    let mut au = 0.;
    let mut rows = vec![];
    for p in &e.positionen {
        text(&p.bezeichnung)?;
        wert(p.flaeche_m2, "Entwässerte Fläche")?;
        let k = a
            .klassen
            .iter()
            .find(|k| k.typ == p.typ)
            .ok_or_else(|| err("Abflussklasse fehlt im gewählten Regelpaket"))?;
        au += p.flaeche_m2 * k.beiwert;
        if p.quelle.trim().is_empty() {
            r.offen(format!("{}: Flächenquelle fehlt", p.bezeichnung));
        }
        rows.push(vec![
            p.bezeichnung.clone(),
            k.bezeichnung.clone(),
            zahl(p.flaeche_m2),
            zahl(k.beiwert),
            zahl(p.flaeche_m2 * k.beiwert),
            format!("{} · {}", p.quelle, k.quelle),
        ]);
    }
    r.tabelle(
        "Abflusswirksame Flächen",
        &[
            "Fläche",
            "Abflussklasse",
            "Fläche m²",
            "Beiwert",
            "Wirksam m²",
            "Quelle",
        ],
        rows,
    );
    let infiltration = if a.versickerung_geprueft && !a.boden_grundwasser_vermerk.trim().is_empty()
    {
        a.kf_m_s * a.sickerflaeche_m2 * a.versickerungsfaktor * 1000.
    } else {
        if a.kf_m_s > 0. || a.sickerflaeche_m2 > 0. {
            r.offen(
                "Versickerung erst nach Boden-/Grundwasser- und Zulässigkeitsprüfung anrechnen",
            );
        }
        0.
    };
    let q_ab = a.drossel_l_s + infiltration;
    let mut vmax = 0.;
    let mut dauer = None;
    let mut regen = vec![];
    let mut seen = std::collections::HashSet::new();
    for d in &a.regen {
        wert(d.dauer_min, "Regendauer")?;
        wert(d.intensitaet_l_s_ha, "Regenspende")?;
        if d.dauer_min <= 0. || d.intensitaet_l_s_ha <= 0. || !seen.insert(d.dauer_min.to_bits()) {
            return Err(err(
                "Regendauer/-intensität positiv, Dauerstufen eindeutig erfassen",
            ));
        }
        if d.quelle.trim().is_empty() {
            r.offen("Standort, Wiederkehrzeit und Quelle der Regendaten dokumentieren");
        }
        let q_zu = d.intensitaet_l_s_ha * au / 10000.;
        let v = (q_zu - q_ab).max(0.) * d.dauer_min * 60. / 1000. * a.sicherheitsfaktor;
        if v > vmax || dauer.is_none() {
            vmax = v;
            dauer = Some(d.dauer_min);
        }
        regen.push(vec![
            zahl(d.dauer_min),
            zahl(d.intensitaet_l_s_ha),
            zahl(q_zu),
            zahl(q_ab),
            format!(
                "max(0; {} − {}) × {} × 60 / 1000 × {}",
                zahl(q_zu),
                zahl(q_ab),
                zahl(d.dauer_min),
                zahl(a.sicherheitsfaktor)
            ),
            zahl(v),
            d.quelle.clone(),
        ]);
    }
    r.tabelle(
        "Rückhaltung je Regenblock",
        &[
            "Dauer min",
            "Regen l/(s·ha)",
            "Zufluss l/s",
            "Abfluss l/s",
            "Rechenweg",
            "Volumen m³",
            "Regendatenquelle",
        ],
        regen,
    );
    r.tabelle(
        "Bemessungsansatz",
        &["Größe", "Wert / Herleitung"],
        vec![
            vec!["Abflusswirksame Fläche".into(), format!("{} m²", zahl(au))],
            vec![
                "Gesicherte Drossel".into(),
                format!("{} l/s", zahl(a.drossel_l_s)),
            ],
            vec![
                "Versickerung".into(),
                format!(
                    "{} × {} × {} × 1000 = {} l/s",
                    zahl(a.kf_m_s),
                    zahl(a.sickerflaeche_m2),
                    zahl(a.versickerungsfaktor),
                    zahl(infiltration)
                ),
            ],
            vec![
                "Boden-/Grundwasserprüfung".into(),
                a.boden_grundwasser_vermerk.clone(),
            ],
            vec![
                "Vorhandenes Volumen".into(),
                a.vorhandenes_volumen_m3
                    .map(|n| format!("{} m³", zahl(n)))
                    .unwrap_or_else(|| "Nicht angegeben".into()),
            ],
        ],
    );
    r.kennzahl(
        "au",
        "Abflusswirksame Fläche",
        Some(au),
        Einheit::M2,
        Messgroesse::Grundflaeche,
    );
    r.kennzahl(
        "rueckhaltung",
        "Erforderliches Volumen im Regenblockmodell",
        (!a.regen.is_empty()).then_some(vmax),
        Einheit::M3,
        Messgroesse::Volumen,
    );
    r.kennzahl(
        "abfluss",
        "Angesetzter Abfluss",
        Some(q_ab),
        Einheit::LProS,
        Messgroesse::Sonstiges,
    );
    if let Some(d) = dauer {
        r.hinweise.push(format!(
            "Maßgebende untersuchte Dauerstufe: {} min",
            zahl(d)
        ));
    }
    if a.vorhandenes_volumen_m3.is_some_and(|n| n + 1e-9 < vmax) {
        r.offen("Vorhandenes Rückhaltevolumen kleiner als ermittelter Bedarf");
    }
    if e.positionen.is_empty()
        || a.regen.is_empty()
        || !e.flaechen_geprueft
        || !e.regendaten_geprueft
    {
        r.offen("Flächen, Standort, Wiederkehrzeit und ausreichenden Umfang der Regendauerstufen prüfen");
    }
    r.hinweise.push("Leistungsumfang: vereinfachte Speicherbilanz mit konstanten Regenblöcken und konstantem Abfluss. Kein vollständiger DWA-/DIN-Nachweis, keine hydraulische Netz-, Überflutungs- oder dynamische Versickerungsberechnung. Diese Fachnachweise sind bei Erfordernis gesondert beizufügen.".into());
    Ok(r)
}
