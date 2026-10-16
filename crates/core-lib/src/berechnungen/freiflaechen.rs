//! Flächenbilanz getrennt von GRZ; körperliche und planbezogene Bezugsflächen bleiben benannt.
pub const RECHENSTAND: &str = "freiflaechen-1";
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flaechenregel {
    pub typ: String,
    pub bezeichnung: String,
    pub versiegelungsfaktor: f64,
    pub gruenfaktor: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    pub klassen: Vec<Flaechenregel>,
    pub gruen_bezug: String,
    pub min_gruenanteil: Option<f64>,
    pub max_versiegelungsanteil: Option<f64>,
    pub baum_je_m2: Option<f64>,
    pub spiel_m2_je_we: Option<f64>,
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
    pub grundstuecksflaeche_m2: f64,
    pub bauland_m2: Option<f64>,
    pub positionen: Vec<Flaeche>,
    pub baeume_angebot: f64,
    pub wohneinheiten: Option<f64>,
    pub spielflaeche_m2: Option<f64>,
    pub bilanz_geprueft: bool,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Freiflaechen);
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    ids(e.regeln.klassen.iter().map(|p| p.typ.as_str()))?;
    wert(e.grundstuecksflaeche_m2, "Eigentumsfläche")?;
    if let Some(a) = e.bauland_m2 {
        wert(a, "Baulandbezugsfläche")?;
    }
    wert(e.baeume_angebot, "Baumanzahl")?;
    if e.baeume_angebot.fract() != 0. {
        return Err(err("Bäume ganzzahlig angeben"));
    }
    for a in &e.regeln.klassen {
        quote(a.versiegelungsfaktor, "Versiegelungsfaktor")?;
        quote(a.gruenfaktor, "Grünfaktor")?;
        text(&a.quelle)?;
    }
    let mut fl = 0.;
    let mut siegel = 0.;
    let mut gruen = 0.;
    let mut rows = vec![];
    for p in &e.positionen {
        text(&p.bezeichnung)?;
        wert(p.flaeche_m2, "Teilfläche")?;
        let a = e
            .regeln
            .klassen
            .iter()
            .find(|a| a.typ == p.typ)
            .ok_or_else(|| err("Flächenklasse fehlt im Regelpaket"))?;
        fl += p.flaeche_m2;
        siegel += p.flaeche_m2 * a.versiegelungsfaktor;
        gruen += p.flaeche_m2 * a.gruenfaktor;
        if p.quelle.trim().is_empty() {
            r.offen(format!("{}: Flächenquelle fehlt", p.bezeichnung));
        }
        r.kennzahl(
            format!("flaeche:{}", p.id),
            format!("Grundfläche {}", p.bezeichnung),
            Some(p.flaeche_m2),
            Einheit::M2,
            Messgroesse::Grundflaeche,
        );
        rows.push(vec![
            p.bezeichnung.clone(),
            a.bezeichnung.clone(),
            zahl(p.flaeche_m2),
            zahl(a.versiegelungsfaktor),
            zahl(p.flaeche_m2 * a.versiegelungsfaktor),
            zahl(a.gruenfaktor),
            zahl(p.flaeche_m2 * a.gruenfaktor),
            format!("{} · {}", p.quelle, a.quelle),
        ]);
    }
    r.tabelle(
        "Flächenbilanz",
        &[
            "Teilfläche",
            "Klasse",
            "Fläche m²",
            "Versiegelungsfaktor",
            "Versiegelt m²",
            "Grünfaktor",
            "Grünansatz m²",
            "Quelle / Regel",
        ],
        rows,
    );
    let rest = e.grundstuecksflaeche_m2 - fl;
    if rest.abs() > 0.01 {
        r.offen(format!("Flächenbilanz nicht geschlossen: Eigentumsfläche {} m² − erfasste Flächen {} m² = {} m²",zahl(e.grundstuecksflaeche_m2),zahl(fl),zahl(rest)));
    }
    let basis = match e.regeln.gruen_bezug.as_str() {
        "eigentum" => Some(e.grundstuecksflaeche_m2),
        "bauland" => e.bauland_m2,
        _ => return Err(err("Grünflächenbezug als eigentum oder bauland wählen")),
    };
    if basis.is_none_or(|n| n <= 0.) {
        r.offen("Gewählte Grünflächenbezugsfläche ist nicht ermittelt");
    }
    if basis.is_some_and(|n| gruen > n + 0.01) {
        r.offen("Anrechenbare Grünfläche übersteigt die gewählte Bezugsfläche; Flächenabgrenzung und Festsetzung prüfen");
    }
    let gruenquote = basis.filter(|n| *n > 0.).map(|n| gruen / n);
    let versquote = (e.grundstuecksflaeche_m2 > 0.).then(|| siegel / e.grundstuecksflaeche_m2);
    let mut vergleiche = vec![];
    if let Some(min) = e.regeln.min_gruenanteil {
        quote(min, "Mindestgrünanteil")?;
        let ok = gruenquote.is_some_and(|q| q + 1e-9 >= min);
        if !ok {
            r.offen("Mindestgrünanteil nicht nachgewiesen");
        }
        vergleiche.push(vec![
            "Grünanteil".into(),
            gruenquote
                .map(|q| format!("{} %", zahl(q * 100.)))
                .unwrap_or_else(|| "Offen".into()),
            format!("mindestens {} %", zahl(min * 100.)),
            if ok {
                "Eingehalten"
            } else {
                "Nicht nachgewiesen"
            }
            .into(),
        ]);
    }
    if let Some(max) = e.regeln.max_versiegelungsanteil {
        quote(max, "Maximalversiegelung")?;
        let ok = versquote.is_some_and(|q| q <= max + 1e-9);
        if !ok {
            r.offen("Maximale Versiegelung überschritten / ungeklärt");
        }
        vergleiche.push(vec![
            "Versiegelungsanteil".into(),
            versquote
                .map(|q| format!("{} %", zahl(q * 100.)))
                .unwrap_or_else(|| "Offen".into()),
            format!("höchstens {} %", zahl(max * 100.)),
            if ok {
                "Eingehalten"
            } else {
                "Nicht nachgewiesen"
            }
            .into(),
        ]);
    }
    if let Some(je) = e.regeln.baum_je_m2 {
        wert(je, "Baumpflanzschlüssel")?;
        if je <= 0. {
            return Err(err("Baumpflanzschlüssel muss positiv sein"));
        }
        let bedarf = basis.map(|n| (n / je - 1e-9).max(0.).ceil());
        let ok = bedarf.is_some_and(|n| e.baeume_angebot >= n);
        if !ok {
            r.offen("Baumpflanzpflicht nicht nachgewiesen");
        }
        vergleiche.push(vec![
            "Bäume".into(),
            zahl(e.baeume_angebot),
            bedarf.map(zahl).unwrap_or_else(|| "Offen".into()),
            if ok { "Gedeckt" } else { "Nicht gedeckt" }.into(),
        ]);
    }
    if let Some(je) = e.regeln.spiel_m2_je_we {
        wert(je, "Spielfläche je WE")?;
        if let Some(n) = e.wohneinheiten {
            wert(n, "Wohneinheiten")?;
            if n.fract() != 0. {
                return Err(err("Wohneinheiten ganzzahlig angeben"));
            }
        }
        if let Some(n) = e.spielflaeche_m2 {
            wert(n, "Spielfläche")?;
        }
        let bedarf = e.wohneinheiten.map(|n| n * je);
        let ok = bedarf.zip(e.spielflaeche_m2).is_some_and(|(n, a)| a >= n);
        if !ok {
            r.offen("Spielflächennachweis fehlt / ist nicht gedeckt");
        }
        vergleiche.push(vec![
            "Spielfläche m²".into(),
            e.spielflaeche_m2
                .map(zahl)
                .unwrap_or_else(|| "Offen".into()),
            bedarf.map(zahl).unwrap_or_else(|| "Offen".into()),
            if ok { "Gedeckt" } else { "Nicht gedeckt" }.into(),
        ]);
    }
    r.tabelle(
        "Festsetzungen und besondere Flächenanforderungen",
        &[
            "Anforderung",
            "Vorhanden",
            "Erforderlich / zulässig",
            "Prüfung",
        ],
        vergleiche,
    );
    r.kennzahl(
        "versiegelt_m2",
        "Versiegelte Fläche (konfigurierter Ansatz)",
        Some(siegel),
        Einheit::M2,
        Messgroesse::Versiegelungsansatz,
    );
    r.kennzahl(
        "gruen_m2",
        "Anrechenbare Grünfläche",
        Some(gruen),
        Einheit::M2,
        Messgroesse::Gruenansatz,
    );
    r.kennzahl(
        "gruenanteil",
        "Grünanteil",
        gruenquote.map(|n| n * 100.),
        Einheit::Prozent,
        Messgroesse::Sonstiges,
    );
    if e.grundstuecksflaeche_m2 <= 0. || !e.bilanz_geprueft || e.positionen.is_empty() {
        r.offen("Grundstückszuordnung, überschneidungsfreie Flächenaufteilung und Vollständigkeit prüfen");
    }
    r.hinweise.push(format!("Grünflächenbezug: {} = {} m²; Versiegelungsbezug: Eigentumsfläche {} m². Keine GRZ-Berechnung; Abflussbeiwerte werden im Entwässerungsmodul gesondert angesetzt.",e.regeln.gruen_bezug,basis.map(zahl).unwrap_or_else(||"Offen".into()),zahl(e.grundstuecksflaeche_m2)));
    Ok(r)
}
