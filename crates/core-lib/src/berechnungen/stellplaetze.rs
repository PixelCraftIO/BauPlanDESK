//! Satzungsabhängiger Kfz-/Fahrradbedarf; kein bundesweiter Standardfaktor.
pub const RECHENSTAND: &str = "stellplaetze-1";
use super::*;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Art {
    Kfz,
    Fahrrad,
}
impl Art {
    fn name(self) -> &'static str {
        match self {
            Self::Kfz => "Kfz",
            Self::Fahrrad => "Fahrrad",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rundung {
    Aufrunden,
    Kaufmaennisch,
    Abrunden,
}
impl Rundung {
    fn anwenden(self, n: f64) -> f64 {
        match self {
            Self::Aufrunden => n.ceil(),
            Self::Kaufmaennisch => n.round(),
            Self::Abrunden => n.floor(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rundungsstufe {
    JePosition,
    JeNutzung,
    Gesamt,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Richtzahl {
    pub id: String,
    pub art: Art,
    pub nutzung: String,
    pub gebaeudetyp: String,
    pub zone: String,
    pub groesse_von_m2: Option<f64>,
    pub groesse_bis_m2: Option<f64>,
    pub bezugsart: Messgroesse,
    pub faktor: f64,
    pub bezugsmenge: f64,
    pub minimum: f64,
    pub besucheranteil: f64,
    pub besucher_zusaetzlich: bool,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regeln {
    pub arten: Vec<Art>,
    pub richtzahlen: Vec<Richtzahl>,
    pub rundung: Rundung,
    pub rundungsstufe: Rundungsstufe,
    pub barrierefrei_anteil: Option<f64>,
    pub barrierefrei_minimum: f64,
    pub keine_pflicht: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nutzung {
    pub id: String,
    pub bezeichnung: String,
    pub nutzung: String,
    pub gebaeudetyp: String,
    pub zone: String,
    pub klassengroesse_m2: Option<f64>,
    pub wohneinheiten: Option<f64>,
    pub wohnflaeche_m2: Option<f64>,
    pub nutzungsflaeche_m2: Option<f64>,
    pub verkaufsflaeche_m2: Option<f64>,
    pub sitzplaetze: Option<f64>,
    pub betten: Option<f64>,
    pub beschaeftigte: Option<f64>,
}
impl Nutzung {
    pub(super) fn menge(&self, g: Messgroesse) -> Option<f64> {
        match g {
            Messgroesse::Wohneinheiten => self.wohneinheiten,
            Messgroesse::Wohnflaeche => self.wohnflaeche_m2,
            Messgroesse::Nutzungsflaeche => self.nutzungsflaeche_m2,
            Messgroesse::Verkaufsflaeche => self.verkaufsflaeche_m2,
            Messgroesse::Sitzplaetze => self.sitzplaetze,
            Messgroesse::Betten => self.betten,
            Messgroesse::Beschaeftigte => self.beschaeftigte,
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Angebot {
    pub id: String,
    pub bezeichnung: String,
    pub art: Art,
    pub anzahl: f64,
    pub besucher: f64,
    pub barrierefrei: f64,
    pub ausserhalb: bool,
    pub anerkannt: bool,
    pub lage_und_sicherung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anrechnung {
    pub id: String,
    pub art: Art,
    pub grund: String,
    pub anzahl: f64,
    pub bestaetigt: bool,
    pub typ: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eingabe {
    pub regeln: Regeln,
    pub positionen: Vec<Nutzung>,
    pub angebot: Vec<Angebot>,
    pub anrechnungen: Vec<Anrechnung>,
}
pub fn berechnen(e: &Eingabe) -> CoreResult<FachErgebnis> {
    let mut out = FachErgebnis::neu(BerechnungsArt::Stellplaetze);
    let r = &e.regeln;
    ids(e.positionen.iter().map(|p| p.id.as_str()))?;
    ids(r.richtzahlen.iter().map(|p| p.id.as_str()))?;
    ids(e.angebot.iter().map(|p| p.id.as_str()))?;
    ids(e.anrechnungen.iter().map(|p| p.id.as_str()))?;
    if r.arten.is_empty() || r.arten.len() > 2 || (r.arten.len() == 2 && r.arten[0] == r.arten[1]) {
        return Err(err("Kfz/Fahrrad-Geltungsumfang eindeutig auswählen"));
    }
    if e.positionen.is_empty() && !r.keine_pflicht {
        out.offen("Nutzungen erfassen");
    }
    wert(r.barrierefrei_minimum, "Barrierefreies Minimum")?;
    if let Some(q) = r.barrierefrei_anteil {
        quote(q, "Barrierefreier Anteil")?;
    }
    for a in &r.richtzahlen {
        text(&a.id)?;
        text(&a.nutzung)?;
        text(&a.quelle)?;
        wert(a.faktor, "Richtzahl")?;
        wert(a.bezugsmenge, "Bezugsmenge")?;
        if a.bezugsmenge <= 0. {
            return Err(err("Bezugsmenge muss positiv sein"));
        }
        wert(a.minimum, "Mindestbedarf")?;
        quote(a.besucheranteil, "Besucheranteil")?;
        for n in [a.groesse_von_m2, a.groesse_bis_m2].into_iter().flatten() {
            wert(n, "Größenklasse")?;
        }
        if a.groesse_von_m2
            .zip(a.groesse_bis_m2)
            .is_some_and(|(a, b)| a >= b)
        {
            return Err(err(
                "Größenstaffel: Untergrenze muss kleiner als Obergrenze sein",
            ));
        }
    }
    let mut gruppen: BTreeMap<(Art, String), f64> = BTreeMap::new();
    let mut besucher: BTreeMap<Art, f64> = BTreeMap::new();
    let mut rows = vec![];
    for p in &e.positionen {
        text(&p.bezeichnung)?;
        text(&p.nutzung)?;
        for n in [
            p.wohneinheiten,
            p.wohnflaeche_m2,
            p.nutzungsflaeche_m2,
            p.verkaufsflaeche_m2,
            p.sitzplaetze,
            p.betten,
            p.beschaeftigte,
            p.klassengroesse_m2,
        ]
        .into_iter()
        .flatten()
        {
            wert(n, "Nutzungsmenge")?;
        }
        for n in [p.wohneinheiten, p.sitzplaetze, p.betten, p.beschaeftigte]
            .into_iter()
            .flatten()
        {
            if n.fract() != 0. {
                return Err(err("Anzahlen müssen ganzzahlig sein"));
            }
        }
        for art in &r.arten {
            if r.keine_pflicht {
                continue;
            }
            let kandidaten = r
                .richtzahlen
                .iter()
                .filter(|a| {
                    a.art == *art
                        && a.nutzung == p.nutzung
                        && (a.gebaeudetyp.is_empty() || a.gebaeudetyp == p.gebaeudetyp)
                        && (a.zone.is_empty() || a.zone == p.zone)
                        && a.groesse_von_m2
                            .is_none_or(|v| p.klassengroesse_m2.is_some_and(|x| x >= v))
                        && a.groesse_bis_m2
                            .is_none_or(|v| p.klassengroesse_m2.is_some_and(|x| x < v))
                })
                .collect::<Vec<_>>();
            if kandidaten.len() != 1 {
                out.offen(format!("{} / {}: {} passende Satzungsregeln. Nutzung, Zone, Größenklasse und Regelüberschneidungen prüfen",p.bezeichnung,art.name(),kandidaten.len()));
                continue;
            }
            let a = kandidaten[0];
            let Some(m) = p.menge(a.bezugsart) else {
                out.offen(format!(
                    "{}: erforderliche Bezugsgröße {:?} fehlt",
                    p.bezeichnung, a.bezugsart
                ));
                continue;
            };
            let roh = (m / a.bezugsmenge * a.faktor).max(a.minimum);
            let gast = roh * a.besucheranteil;
            let bedarf = roh + if a.besucher_zusaetzlich { gast } else { 0. };
            let key = match r.rundungsstufe {
                Rundungsstufe::JePosition => p.id.clone(),
                Rundungsstufe::JeNutzung => p.nutzung.clone(),
                Rundungsstufe::Gesamt => "gesamt".into(),
            };
            *gruppen.entry((*art, key)).or_default() += bedarf;
            *besucher.entry(*art).or_default() += gast;
            rows.push(vec![
                p.bezeichnung.clone(),
                art.name().into(),
                a.bezugsart.text().into(),
                format!("{} / {} × {}", zahl(m), zahl(a.bezugsmenge), zahl(a.faktor)),
                zahl(roh),
                zahl(gast),
                if a.besucher_zusaetzlich {
                    "Zusätzlich"
                } else {
                    "Enthalten"
                }
                .into(),
                a.quelle.clone(),
            ]);
        }
    }
    out.tabelle(
        "Nutzungsbezogene Bedarfsermittlung",
        &[
            "Nutzung",
            "Art",
            "Bezugsgröße",
            "Rechenansatz",
            "Grundbedarf",
            "Besucher",
            "Ansatz Besucher",
            "Satzungsgrundlage",
        ],
        rows,
    );
    let mut angeboten: BTreeMap<Art, (f64, f64, f64)> = BTreeMap::new();
    let mut angebot_rows = vec![];
    for a in &e.angebot {
        text(&a.bezeichnung)?;
        for n in [a.anzahl, a.besucher, a.barrierefrei] {
            wert(n, "Stellplatzanzahl")?;
            if n.fract() != 0. {
                return Err(err("Angebotene Stellplätze ganzzahlig erfassen"));
            }
        }
        if a.besucher > a.anzahl || a.barrierefrei > a.anzahl {
            return Err(err(
                "Besucher/barrierefreie Plätze sind Teilmengen des Angebots",
            ));
        }
        let wirksam = a.anerkannt && (!a.ausserhalb || !a.lage_und_sicherung.trim().is_empty());
        if !wirksam && a.anzahl > 0. {
            out.offen(format!(
                "Angebot {} noch nicht anerkannt / gesichert",
                a.bezeichnung
            ));
        }
        if wirksam {
            let v = angeboten.entry(a.art).or_default();
            v.0 += a.anzahl;
            v.1 += a.besucher;
            v.2 += a.barrierefrei;
        }
        angebot_rows.push(vec![
            a.bezeichnung.clone(),
            a.art.name().into(),
            zahl(a.anzahl),
            zahl(a.besucher),
            zahl(a.barrierefrei),
            a.lage_und_sicherung.clone(),
            if wirksam { "Angerechnet" } else { "Noch offen" }.into(),
        ]);
    }
    out.tabelle(
        "Tatsächlich angebotene Stellplätze",
        &[
            "Angebot",
            "Art",
            "Gesamt",
            "Besucher (Teilmenge)",
            "Barrierefrei (Teilmenge)",
            "Lage / Sicherung",
            "Status",
        ],
        angebot_rows,
    );
    let mut reduziert: BTreeMap<Art, f64> = BTreeMap::new();
    let mut red_rows = vec![];
    for a in &e.anrechnungen {
        wert(a.anzahl, "Anrechnung")?;
        text(&a.grund)?;
        if !["bestand", "reduktion", "abloesung"].contains(&a.typ.as_str()) {
            return Err(err("Anrechnungstyp unbekannt"));
        }
        if a.bestaetigt {
            *reduziert.entry(a.art).or_default() += a.anzahl;
        } else {
            out.offen(format!("Anrechnung {} nicht bestätigt", a.grund));
        }
        red_rows.push(vec![
            match a.typ.as_str() {
                "bestand" => "Altbedarf",
                "abloesung" => "Ablösung",
                _ => "Reduktion",
            }
            .into(),
            a.art.name().into(),
            zahl(a.anzahl),
            a.grund.clone(),
            if a.bestaetigt { "Bestätigt" } else { "Offen" }.into(),
        ]);
    }
    out.tabelle(
        "Bestand, Reduktionen und Ablösung (keine vorhandenen Stellplätze)",
        &[
            "Art der Anrechnung",
            "Verkehrsart",
            "Anzahl",
            "Grundlage / Beleg",
            "Prüfung",
        ],
        red_rows,
    );
    let mut summen = vec![];
    let mut rund_rows = vec![];
    for ((art, gruppe), roh) in &gruppen {
        rund_rows.push(vec![
            art.name().into(),
            if matches!(r.rundungsstufe, Rundungsstufe::JePosition) {
                e.positionen
                    .iter()
                    .find(|p| p.id == *gruppe)
                    .map(|p| p.bezeichnung.clone())
                    .unwrap_or_default()
            } else {
                gruppe.clone()
            },
            zahl(*roh),
            match r.rundung {
                Rundung::Aufrunden => "Aufrunden",
                Rundung::Kaufmaennisch => "Kaufmännisch runden",
                Rundung::Abrunden => "Abrunden",
            }
            .into(),
            zahl(r.rundung.anwenden(*roh)),
        ]);
    }
    out.tabelle(
        "Satzungsgemäße Rundung",
        &[
            "Art",
            "Rundungsgruppe",
            "Ungerundet",
            "Verfahren",
            "Gerundet",
        ],
        rund_rows,
    );
    for art in &r.arten {
        let brutto = gruppen
            .iter()
            .filter(|((a, _), _)| a == art)
            .map(|(_, n)| r.rundung.anwenden(*n))
            .sum::<f64>();
        let abzug = *reduziert.get(art).unwrap_or(&0.);
        if abzug > brutto {
            out.offen(format!("{}: Anrechnung übersteigt Bedarf", art.name()));
        }
        let netto = r.rundung.anwenden((brutto - abzug).max(0.));
        let gast = r.rundung.anwenden(*besucher.get(art).unwrap_or(&0.));
        let barriere = if *art == Art::Kfz {
            if netto > 0. {
                r.barrierefrei_anteil
                    .map(|q| (netto * q).ceil())
                    .unwrap_or(0.)
                    .max(r.barrierefrei_minimum)
            } else {
                0.
            }
        } else {
            0.
        };
        if gast > netto || barriere > netto {
            out.offen("Besucher-/Sonderanteile sind mit Bedarf/Reduktionen nicht vereinbar");
        }
        let a = angeboten.get(art).copied().unwrap_or_default();
        let gedeckt = a.0 >= netto && a.1 >= gast && a.2 >= barriere;
        if !gedeckt {
            out.offen(format!(
                "{}: Bedarf oder Besucher-/Sonderanteile nicht gedeckt",
                art.name()
            ));
        }
        summen.push(vec![
            art.name().into(),
            zahl(brutto),
            zahl(abzug),
            zahl(netto),
            zahl(gast),
            zahl(barriere),
            zahl(a.0),
            if gedeckt { "Gedeckt" } else { "Fehlbedarf" }.into(),
        ]);
        out.kennzahl(
            if *art == Art::Kfz {
                "kfz_bedarf"
            } else {
                "fahrrad_bedarf"
            },
            format!("{}-Stellplatzbedarf", art.name()),
            Some(netto),
            Einheit::Anzahl,
            Messgroesse::Stellplaetze,
        );
    }
    if r.keine_pflicht {
        out.hinweise.push("Keine Stellplatzpflicht laut ausdrücklich ausgewähltem Regelansatz. Geltung ist projektbezogen zu bestätigen.".into());
    }
    out.tabelle(
        "Bedarf und Deckung",
        &[
            "Art",
            "Brutto",
            "Anrechnung",
            "Netto",
            "Besucher",
            "Barrierefrei",
            "Angeboten",
            "Ergebnis",
        ],
        summen,
    );
    out.hinweise.push("Besucher- und barrierefreie Plätze sind ausgewiesene Teilmengen des Gesamtangebots. Ablösung zählt nicht als tatsächlich vorhandener Stellplatz.".into());
    Ok(out)
}
