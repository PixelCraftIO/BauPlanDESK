//! Maßbezogener Festsetzungsnachweis. Geometrie bleibt beim CAD-Fachmodul.
use super::*;
pub const RECHENSTAND: &str = "ueberbaubarkeit-1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    /// Nur ausdrücklich festgelegte und belegte Abweichungstoleranz, kein pauschales Baurecht.
    pub toleranz_m: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Art {
    Baugrenze,
    Baulinie,
    Vorbau,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub id: String,
    pub bezeichnung: String,
    pub art: Art,
    /// Vorzeichen: außerhalb positiv; innerhalb / Zurücktreten negativ.
    pub abstand_min_m: Option<f64>,
    pub abstand_max_m: Option<f64>,
    pub ausserhalb_m2: Option<f64>,
    pub quelle: String,
    pub entscheidung_bestaetigt: bool,
    pub entscheidung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub regeln: Regeln,
    pub positionen: Vec<Position>,
    pub aufmass_geprueft: bool,
    pub flaechen_abgegrenzt: bool,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    wert(e.regeln.toleranz_m, "Belegte Toleranz")?;
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    let mut r = FachErgebnis::neu(BerechnungsArt::Ueberbaubarkeit);
    let mut rows = vec![];
    let mut flaeche = 0.;
    let mut max: f64 = 0.;
    let mut min: f64 = 0.;
    let mut voll = !e.positionen.is_empty();
    let mut ausnahmen = 0.;
    for p in &e.positionen {
        for s in [&p.bezeichnung, &p.quelle, &p.entscheidung] {
            if !s.is_empty() {
                text(s)?;
            }
        }
        for v in [p.abstand_min_m, p.abstand_max_m].into_iter().flatten() {
            if !v.is_finite() || v.abs() > 1e6 {
                return Err(err("Abstand muss endlich und plausibel sein"));
            }
        }
        if let Some(a) = p.ausserhalb_m2 {
            wert(a, "Überschreitungsfläche")?;
        }
        let belegt = p.entscheidung_bestaetigt && !p.entscheidung.trim().is_empty();
        let mut status = "Nicht ermittelt";
        if let Some((a, b)) = p.abstand_min_m.zip(p.abstand_max_m) {
            if a > b {
                return Err(err("Kleinster Abstand größer als größter Abstand"));
            }
            max = max.max(b);
            min = min.min(a);
            let geometrisch = match p.art {
                Art::Baugrenze => b <= e.regeln.toleranz_m,
                Art::Baulinie => a >= -e.regeln.toleranz_m && b <= e.regeln.toleranz_m,
                // Vorbauten brauchen auch innerhalb der Grenzen die belegte örtliche Beurteilung.
                Art::Vorbau => false,
            };
            if geometrisch {
                status = "Maßvorgabe eingehalten";
            } else if belegt {
                status = "Abweichung / Vorbau mit bestätigtem Beleg";
                ausnahmen += 1.;
            } else {
                status = "Entscheidung / Beleg offen";
                r.offen(format!(
                    "{}: Vorbau / Abweichung benötigt eine belegte Entscheidung",
                    p.bezeichnung
                ));
            }
            if b > 0. && p.ausserhalb_m2.is_none() {
                r.offen(format!("{}: Überschreitungsfläche fehlt", p.bezeichnung));
                voll = false;
            }
            if b <= 0. && p.ausserhalb_m2.is_some_and(|v| v > 0.) {
                return Err(err("Positive Überschreitungsfläche ohne äußeren Überstand"));
            }
        } else {
            r.offen(format!(
                "{}: beide vorzeichenbehafteten Grenzmaße fehlen / unvollständig",
                p.bezeichnung
            ));
            voll = false;
        }
        if p.quelle.trim().is_empty() || p.bezeichnung.trim().is_empty() {
            r.offen("Plan-/Aufmaßbezug und Bauteilbezeichnung vervollständigen");
        }
        if p.entscheidung_bestaetigt && !belegt {
            r.offen("Bestätigte Entscheidung benötigt Rechtsgrund und Beleg");
        }
        flaeche += p.ausserhalb_m2.unwrap_or(0.);
        rows.push(vec![
            p.bezeichnung.clone(),
            match p.art {
                Art::Baugrenze => "Baugrenze",
                Art::Baulinie => "Baulinie",
                Art::Vorbau => "Vorbau",
            }
            .into(),
            format!(
                "{} / {}",
                optional(p.abstand_min_m),
                optional(p.abstand_max_m)
            ),
            optional(p.ausserhalb_m2),
            status.into(),
            format!("{}\n{}", p.quelle, p.entscheidung),
        ]);
    }
    if !e.aufmass_geprueft || e.positionen.is_empty() {
        r.offen("Vollständigkeit und Extremmaße sämtlicher betroffener Bauteile am Plan prüfen");
    }
    if !e.flaechen_abgegrenzt {
        r.offen("Überschreitungsflächen ohne Mehrfachzählung abgrenzen");
    }
    r.tabelle(
        "Baulinien, Baugrenzen und Vorbauten",
        &[
            "Bauteil / Abschnitt",
            "Festsetzung",
            "Min. / Max. Abstand (m)",
            "Außerhalb (m²)",
            "Prüfung",
            "Planbezug / Entscheidung",
        ],
        rows,
    );
    r.kennzahl(
        "ueberstand_m",
        "Größter äußerer Überstand",
        voll.then_some(max),
        Einheit::M,
        Messgroesse::Sonstiges,
    );
    r.kennzahl(
        "ruecktritt_m",
        "Größtes Zurücktreten",
        voll.then_some(-min),
        Einheit::M,
        Messgroesse::Sonstiges,
    );
    r.kennzahl(
        "ausserhalb_m2",
        "Abgegrenzte Überschreitungsfläche",
        (voll && e.flaechen_abgegrenzt).then_some(flaeche),
        Einheit::M2,
        Messgroesse::Grundflaeche,
    );
    r.kennzahl(
        "entscheidungen",
        "Bestätigte Abweichungs- / Vorbauentscheidungen",
        Some(ausnahmen),
        Einheit::Anzahl,
        Messgroesse::Sonstiges,
    );
    r.hinweise.push("Maßbezogener Nachweis aus geprüften Extremmaßen: positiv außerhalb, negativ innerhalb / zurücktretend. Keine automatische CAD-Abstandsableitung. Die geometrische Baufensterprüfung erfolgt getrennt im Planbereich. Belegbestätigung ersetzt keine erforderliche behördliche Entscheidung; eine Toleranz erzeugt kein Baurecht.".into());
    Ok(r)
}

fn optional(v: Option<f64>) -> String {
    v.map(zahl).unwrap_or_else(|| "Nicht ermittelt".into())
}
