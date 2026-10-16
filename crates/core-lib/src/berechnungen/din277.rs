//! Explizite NUF/TF/VF/KGF-Klassifikation und davon getrennte äußere Volumenkörper.
use super::wohnflaechen::{Form, aufmass};
pub const RECHENSTAND: &str = "din277-1";
use super::*;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flaechenart {
    Nuf,
    Tf,
    Vf,
    Kgf,
}
impl Flaechenart {
    fn name(self) -> &'static str {
        match self {
            Self::Nuf => "NUF",
            Self::Tf => "TF",
            Self::Vf => "VF",
            Self::Kgf => "KGF",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flaeche {
    pub id: String,
    pub geschoss: String,
    pub raum: String,
    pub art: Flaechenart,
    pub bereich: String,
    pub form: Form,
    pub laenge_m: f64,
    pub breite_m: f64,
    pub breite2_m: f64,
    pub aufmass_m2: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Volumen {
    pub id: String,
    pub bezeichnung: String,
    pub form: Form,
    pub laenge_m: f64,
    pub breite_m: f64,
    pub breite2_m: f64,
    pub aufmass_m2: f64,
    pub hoehe_m: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub positionen: Vec<Flaeche>,
    pub volumen: Vec<Volumen>,
    pub klassifikation_geprueft: bool,
    pub kgf_vollstaendig: bool,
    pub volumen_vollstaendig: bool,
    #[serde(default)]
    pub rauminhalt_erforderlich: bool,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut r = FachErgebnis::neu(BerechnungsArt::Din277);
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    ids(e.volumen.iter().map(|p| p.id.as_str()))?;
    let mut sum: BTreeMap<Flaechenart, f64> = BTreeMap::new();
    let mut gruppen: BTreeMap<(String, String, Flaechenart), f64> = BTreeMap::new();
    let mut rows = vec![];
    for p in &e.positionen {
        text(&p.geschoss)?;
        text(&p.raum)?;
        if !["R", "S"].contains(&p.bereich.as_str()) {
            return Err(err("DIN-277-Bereich als R oder S klassifizieren"));
        }
        if p.quelle.trim().is_empty() {
            r.offen(format!("Aufmaßquelle {} fehlt", p.raum));
        }
        let fl = aufmass(p.form, p.laenge_m, p.breite_m, p.breite2_m, p.aufmass_m2)?;
        *sum.entry(p.art).or_default() += fl;
        *gruppen
            .entry((p.geschoss.clone(), p.bereich.clone(), p.art))
            .or_default() += fl;
        rows.push(vec![
            p.geschoss.clone(),
            p.raum.clone(),
            p.art.name().into(),
            p.bereich.clone(),
            format!(
                "{} / {} / {} / {}",
                zahl(p.laenge_m),
                zahl(p.breite_m),
                zahl(p.breite2_m),
                zahl(p.aufmass_m2)
            ),
            zahl(fl),
            p.quelle.clone(),
        ]);
    }
    r.tabelle(
        "Erfasste Flächen",
        &[
            "Geschoss",
            "Raum / Bauteil",
            "Flächenart",
            "Bereich R/S",
            "Länge / Breiten (m) / Aufmaß (m²)",
            "Fläche m²",
            "Quelle",
        ],
        rows,
    );
    r.tabelle(
        "Geschoss- und Bereichssummen",
        &["Geschoss", "Bereich", "Flächenart", "Fläche m²"],
        gruppen
            .iter()
            .map(|((g, b, a), f)| vec![g.clone(), b.clone(), a.name().into(), zahl(*f)])
            .collect(),
    );
    let get = |a| sum.get(&a).copied().unwrap_or(0.);
    let nuf = get(Flaechenart::Nuf);
    let tf = get(Flaechenart::Tf);
    let vf = get(Flaechenart::Vf);
    let kgf = get(Flaechenart::Kgf);
    let nrf = nuf + tf + vf;
    for (id, name, v, g) in [
        ("nuf", "Nutzungsfläche", nuf, Messgroesse::Nutzungsflaeche),
        ("tf", "Technikfläche", tf, Messgroesse::Tf),
        ("vf", "Verkehrsfläche", vf, Messgroesse::Vf),
        (
            "nrf",
            "Netto-Raumfläche (NUF + TF + VF)",
            nrf,
            Messgroesse::Nrf,
        ),
    ] {
        r.kennzahl(id, name, Some(v), Einheit::M2, g);
    }
    r.kennzahl(
        "kgf",
        "Konstruktions-Grundfläche",
        e.kgf_vollstaendig.then_some(kgf),
        Einheit::M2,
        Messgroesse::Kgf,
    );
    r.kennzahl(
        "bgf",
        "Brutto-Grundfläche (NRF + KGF)",
        e.kgf_vollstaendig.then_some(nrf + kgf),
        Einheit::M2,
        Messgroesse::Bgf,
    );
    let mut volumen = 0.;
    let mut vr = vec![];
    for p in &e.volumen {
        text(&p.bezeichnung)?;
        wert(p.hoehe_m, "Äußere Körperhöhe")?;
        let grund = aufmass(p.form, p.laenge_m, p.breite_m, p.breite2_m, p.aufmass_m2)?;
        let v = grund * p.hoehe_m;
        volumen += v;
        if p.quelle.trim().is_empty() {
            r.offen(format!("Äußeres Volumenaufmaß fehlt: {}", p.bezeichnung));
        }
        vr.push(vec![
            p.bezeichnung.clone(),
            zahl(grund),
            zahl(p.hoehe_m),
            zahl(v),
            p.quelle.clone(),
        ]);
    }
    r.tabelle(
        "Äußere Volumenkörper (getrennt von lichten Raumvolumen)",
        &[
            "Körper",
            "Grundfläche m²",
            "Äußere Höhe m",
            "Volumen m³",
            "Quelle",
        ],
        vr,
    );
    r.kennzahl(
        "bri",
        "Brutto-Rauminhalt",
        (e.volumen_vollstaendig && !e.volumen.is_empty()).then_some(volumen),
        Einheit::M3,
        Messgroesse::Bri,
    );
    if e.positionen.is_empty() || !e.klassifikation_geprueft {
        r.offen("Flächenumfang, R/S-Einordnung und NUF/TF/VF/KGF-Klassifikation anhand der gewählten Normfassung bestätigen");
    }
    if !e.kgf_vollstaendig {
        r.offen("Konstruktionsflächen noch nicht vollständig; keine verbindliche BGF");
    }
    if e.rauminhalt_erforderlich && (!e.volumen_vollstaendig || e.volumen.is_empty()) {
        r.offen("Erforderlicher Brutto-Rauminhalt ist noch nicht vollständig erfasst");
    }
    if !e.volumen_vollstaendig {
        r.hinweise
            .push("BRI nicht vollständig erfasst; kein Brutto-Rauminhalt ausgewiesen.".into());
    }
    r.hinweise.push("R/S-Zuordnung und äußere Maßbezüge sind nach der im Regelkontext festgehaltenen DIN-277-Fassung zu prüfen. Keine automatische Umrechnung in Wohnfläche oder GFZ.".into());
    Ok(r)
}
