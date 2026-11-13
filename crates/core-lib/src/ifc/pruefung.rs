//! Modellprüfung für den Bauantrag: eingebaute Regeln (Einheiten, Geschosse,
//! Räume, Mengen gegen Geometrie …) und Information Delivery Specifications
//! (IDS 1.0, buildingSMART). Mitgeliefert wird `regelwerke/ifc/bauantrag.ids`;
//! ein eigenes IDS des Büros kann aus den Projektdateien gewählt werden.
//!
//! Vom IDS-Standard ausgewertet werden die Facetten `entity` (mit
//! `predefinedType`), `attribute` und `property` mit `simpleValue`,
//! Aufzählungen und Zahlengrenzen sowie die Kardinalitäten. Muster
//! (`xs:pattern`), `classification`, `material` und `partOf` werden als
//! „nicht geprüft“ ausgewiesen statt stillschweigend bestanden.
use super::{Modellauszug, Modelleigenschaft, Modellmenge};
use crate::{CoreError, CoreResult};
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};

pub const BAUANTRAG_IDS: &str = include_str!("../../../../regelwerke/ifc/bauantrag.ids");

/// Toleranz der Raummengen gegen die Geometrie: absolut (m²) und relativ.
const MENGE_ABS_M2: f64 = 0.5;
const MENGE_REL: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Befundstufe {
    Fehler,
    Warnung,
    Hinweis,
}

/// Nacharbeit, mit der sich ein Befund beheben lässt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Befundaktion {
    Klassifizieren,
    GeschossZuordnen,
    EigenschaftSetzen,
    RaumBilden,
    Benennen,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Befund {
    /// Regelkennzeichen, z. B. `proxy` oder `IDS:W-AUSSENWAND`.
    pub regel: String,
    pub stufe: Befundstufe,
    pub titel: String,
    pub text: String,
    /// Betroffene Elemente (GlobalIds), leer bei Modellbefunden.
    pub elemente: Vec<String>,
    pub aktion: Option<Befundaktion>,
    /// Bei IDS-Befunden: fehlende Eigenschaft (`Pset.Name`) für die Nacharbeit.
    pub eigenschaft: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellpruefung {
    /// Titel des IDS, gegen das geprüft wurde.
    pub regelwerk: String,
    pub befunde: Vec<Befund>,
    pub fehler: usize,
    pub warnungen: usize,
    pub hinweise: usize,
    /// IDS-Teile, die nicht ausgewertet werden konnten.
    pub nicht_geprueft: Vec<String>,
}

/// Zu prüfendes Objekt: Element oder Strukturelement.
struct Pruefling<'a> {
    gid: &'a str,
    klasse: &'a str,
    name: &'a str,
    langname: Option<&'a str>,
    objekttyp: Option<&'a str>,
    vordefiniert: Option<&'a str>,
    eigenschaften: &'a [Modelleigenschaft],
    mengen: &'a [Modellmenge],
}

fn prueflinge(a: &Modellauszug) -> Vec<Pruefling<'_>> {
    let mut v = Vec::new();
    for (klasse, liste) in [
        ("IFCSITE", &a.grundstuecke),
        ("IFCBUILDING", &a.gebaeude),
        ("IFCBUILDINGSTOREY", &a.geschosse),
    ] {
        for s in liste {
            v.push(Pruefling {
                gid: &s.global_id,
                klasse,
                name: &s.name,
                langname: s.langname.as_deref(),
                objekttyp: None,
                vordefiniert: None,
                eigenschaften: &s.eigenschaften,
                mengen: &s.mengen,
            });
        }
    }
    for e in &a.elemente {
        v.push(Pruefling {
            gid: &e.global_id,
            klasse: &e.klasse,
            name: &e.name,
            langname: e.langname.as_deref(),
            objekttyp: e.objekttyp.as_deref(),
            vordefiniert: e.vordefiniert.as_deref(),
            eigenschaften: &e.eigenschaften,
            mengen: &e.mengen,
        });
    }
    v
}

// ---------------------------------------------------------------- XML -----

#[derive(Debug, Default)]
struct Knoten {
    name: String,
    attr: Vec<(String, String)>,
    kinder: Vec<Knoten>,
    text: String,
}

impl Knoten {
    fn kind(&self, n: &str) -> Option<&Knoten> {
        self.kinder.iter().find(|k| k.name == n)
    }
    fn alle<'a>(&'a self, n: &'a str) -> impl Iterator<Item = &'a Knoten> + 'a {
        self.kinder.iter().filter(move |k| k.name == n)
    }
    fn attr(&self, n: &str) -> Option<&str> {
        self.attr
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.as_str())
    }
}

fn xml_fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("IDS-Datei nicht lesbar: {e}"))
}

fn knoten(e: &BytesStart) -> CoreResult<Knoten> {
    let name = e.local_name().as_ref().to_string();
    let mut attr = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(xml_fehler)?;
        let k = a.key.local_name().as_ref().to_string();
        let v = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(xml_fehler)?
            .to_string();
        attr.push((k, v));
    }
    Ok(Knoten {
        name,
        attr,
        ..Default::default()
    })
}

fn dom(xml: &str) -> CoreResult<Knoten> {
    let mut r = quick_xml::Reader::from_str(xml);
    let mut stapel: Vec<Knoten> = vec![Knoten::default()];
    loop {
        match r.read_event().map_err(xml_fehler)? {
            Event::Start(e) => stapel.push(knoten(&e)?),
            Event::Empty(e) => {
                let k = knoten(&e)?;
                stapel.last_mut().expect("Wurzel").kinder.push(k);
            }
            Event::End(_) => {
                let k = stapel
                    .pop()
                    .ok_or_else(|| xml_fehler("Endelement ohne Anfang"))?;
                stapel
                    .last_mut()
                    .ok_or_else(|| xml_fehler("Endelement ohne Anfang"))?
                    .kinder
                    .push(k);
            }
            Event::Text(t) => {
                let s = quick_xml::escape::unescape(&t.xml10_content())
                    .map_err(xml_fehler)?
                    .to_string();
                stapel.last_mut().expect("Wurzel").text.push_str(&s);
            }
            Event::CData(t) => stapel
                .last_mut()
                .expect("Wurzel")
                .text
                .push_str(&t.xml10_content()),
            Event::GeneralRef(e) => {
                let s = match e.resolve_char_ref().map_err(xml_fehler)? {
                    Some(c) => c.to_string(),
                    None => match e.as_ref() {
                        "amp" => "&".into(),
                        "lt" => "<".into(),
                        "gt" => ">".into(),
                        "quot" => "\"".into(),
                        "apos" => "'".into(),
                        _ => return Err(xml_fehler("unbekannte Entität")),
                    },
                };
                stapel.last_mut().expect("Wurzel").text.push_str(&s);
            }
            Event::DocType(_) => return Err(xml_fehler("DTD ist nicht zulässig")),
            Event::Eof => break,
            _ => {}
        }
    }
    let mut wurzel = stapel
        .pop()
        .filter(|_| stapel.is_empty())
        .ok_or_else(|| xml_fehler("Element nicht geschlossen"))?;
    wurzel
        .kinder
        .pop()
        .filter(|k| k.name == "ids")
        .ok_or_else(|| xml_fehler("kein <ids>-Element"))
}

// ---------------------------------------------------------- IDS-Werte -----

/// Wertvorgabe einer Facette.
#[derive(Debug, Clone)]
enum Vorgabe {
    Gleich(String),
    EinerVon(Vec<String>),
    Bereich {
        min: Option<(f64, bool)>,
        max: Option<(f64, bool)>,
    },
    /// Nicht auswertbar (Muster, Längen …): gilt als erfüllt, wird gemeldet.
    Offen(String),
}

fn vorgabe(k: &Knoten) -> Option<Vorgabe> {
    if let Some(s) = k.kind("simpleValue") {
        return Some(Vorgabe::Gleich(s.text.trim().to_string()));
    }
    let r = k.kind("restriction")?;
    let werte: Vec<String> = r
        .alle("enumeration")
        .filter_map(|e| e.attr("value").map(str::to_string))
        .collect();
    if !werte.is_empty() {
        return Some(Vorgabe::EinerVon(werte));
    }
    let zahl = |n: &str| r.kind(n).and_then(|e| e.attr("value")?.parse::<f64>().ok());
    let min = zahl("minInclusive")
        .map(|v| (v, true))
        .or(zahl("minExclusive").map(|v| (v, false)));
    let max = zahl("maxInclusive")
        .map(|v| (v, true))
        .or(zahl("maxExclusive").map(|v| (v, false)));
    if min.is_some() || max.is_some() {
        return Some(Vorgabe::Bereich { min, max });
    }
    let art = r.kinder.first().map_or("restriction", |k| k.name.as_str());
    Some(Vorgabe::Offen(art.to_string()))
}

impl Vorgabe {
    /// `wert`: Text; `zahl`: Zahlenwert, sofern vorhanden.
    fn passt(&self, wert: &str, zahl: Option<f64>) -> bool {
        let gleich = |soll: &str| {
            if soll == wert {
                return true;
            }
            match soll.to_ascii_uppercase().as_str() {
                "TRUE" => return wert == "Ja",
                "FALSE" => return wert == "Nein",
                _ => {}
            }
            match (soll.parse::<f64>(), zahl) {
                (Ok(s), Some(z)) => (s - z).abs() <= 1e-6 * s.abs().max(1.),
                _ => false,
            }
        };
        match self {
            Self::Gleich(s) => gleich(s),
            Self::EinerVon(l) => l.iter().any(|s| gleich(s)),
            Self::Bereich { min, max } => zahl.is_some_and(|z| {
                min.is_none_or(|(m, inkl)| if inkl { z >= m } else { z > m })
                    && max.is_none_or(|(m, inkl)| if inkl { z <= m } else { z < m })
            }),
            Self::Offen(_) => true,
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Gleich(s) => format!("„{s}“"),
            Self::EinerVon(l) => format!("eine von {}", l.join(", ")),
            Self::Bereich { min, max } => {
                let mut t = Vec::new();
                if let Some((m, i)) = min {
                    t.push(format!("{} {m}", if *i { "≥" } else { ">" }));
                }
                if let Some((m, i)) = max {
                    t.push(format!("{} {m}", if *i { "≤" } else { "<" }));
                }
                t.join(" und ")
            }
            Self::Offen(a) => format!("({a}, nicht geprüft)"),
        }
    }
}

fn klasse_passt(soll: &str, ist: &str) -> bool {
    let norm = |k: &str| {
        k.to_ascii_uppercase()
            .trim_end_matches("STANDARDCASE")
            .trim_end_matches("ELEMENTEDCASE")
            .to_string()
    };
    norm(soll) == norm(ist)
}

/// Facette auf einen Prüfling: `Some(true)` erfüllt, `Some(false)` verletzt,
/// `None` nicht auswertbar (Art wird in `offen` gemeldet).
fn facette(f: &Knoten, p: &Pruefling, offen: &mut Vec<String>) -> Option<bool> {
    match f.name.as_str() {
        "entity" => {
            let name = f.kind("name").and_then(vorgabe)?;
            let klasse_ok = match &name {
                Vorgabe::Gleich(s) => klasse_passt(s, p.klasse),
                Vorgabe::EinerVon(l) => l.iter().any(|s| klasse_passt(s, p.klasse)),
                _ => false,
            };
            let typ_ok = match f.kind("predefinedType").and_then(vorgabe) {
                None => true,
                Some(v) => p.vordefiniert.is_some_and(|t| v.passt(t, None)),
            };
            Some(klasse_ok && typ_ok)
        }
        "attribute" => {
            let Some(Vorgabe::Gleich(n)) = f.kind("name").and_then(vorgabe) else {
                offen.push("attribute ohne festen Namen".into());
                return None;
            };
            let wert = match n.as_str() {
                "Name" => Some(p.name).filter(|s| !s.is_empty()),
                "LongName" => p.langname.filter(|s| !s.is_empty()),
                "ObjectType" => p.objekttyp.filter(|s| !s.is_empty()),
                "PredefinedType" => p
                    .vordefiniert
                    .filter(|s| !s.is_empty() && *s != "NOTDEFINED"),
                "GlobalId" => Some(p.gid),
                _ => {
                    offen.push(format!("Attribut {n}"));
                    return None;
                }
            };
            let vorgabe_wert = f.kind("value").and_then(vorgabe);
            if let Some(Vorgabe::Offen(art)) = &vorgabe_wert {
                offen.push(format!("Attributwert ({art})"));
            }
            Some(match (wert, vorgabe_wert) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(w), Some(v)) => v.passt(w, w.parse().ok()),
            })
        }
        "property" => {
            let satz = f.kind("propertySet").and_then(vorgabe)?;
            let name = f.kind("baseName").and_then(vorgabe)?;
            let wert = f.kind("value").and_then(vorgabe);
            if let Some(Vorgabe::Offen(a)) = &wert {
                offen.push(format!("Eigenschaftswert ({a})"));
            }
            let gefunden = p
                .eigenschaften
                .iter()
                .filter(|e| satz.passt(&e.satz, None) && name.passt(&e.name, None))
                .map(|e| (e.wert.clone(), e.zahl))
                .chain(
                    p.mengen
                        .iter()
                        .filter(|m| satz.passt(&m.satz, None) && name.passt(&m.name, None))
                        .map(|m| (m.wert.to_string(), Some(m.wert))),
                )
                .collect::<Vec<_>>();
            Some(
                !gefunden.is_empty()
                    && wert
                        .as_ref()
                        .is_none_or(|v| gefunden.iter().any(|(w, z)| v.passt(w, *z))),
            )
        }
        andere => {
            offen.push(format!("Facette {andere}"));
            None
        }
    }
}

fn facettentext(f: &Knoten) -> (String, Option<String>) {
    let v = |n: &str| match f.kind(n).and_then(vorgabe) {
        Some(Vorgabe::Gleich(s)) => s,
        Some(v) => v.text(),
        None => String::new(),
    };
    match f.name.as_str() {
        "attribute" => {
            let wert = f
                .kind("value")
                .and_then(vorgabe)
                .map(|w| format!(" = {}", w.text()));
            (
                format!("Attribut {}{}", v("name"), wert.unwrap_or_default()),
                None,
            )
        }
        "property" => {
            let pset = format!("{}.{}", v("propertySet"), v("baseName"));
            let wert = f
                .kind("value")
                .and_then(vorgabe)
                .map(|w| format!(" = {}", w.text()));
            (
                format!("Eigenschaft {pset}{}", wert.unwrap_or_default()),
                Some(pset),
            )
        }
        "entity" => (format!("Klasse {}", v("name")), None),
        n => (n.to_string(), None),
    }
}

/// Prüft ein Modell gegen ein IDS.
pub fn ids_pruefen(a: &Modellauszug, ids: &str) -> CoreResult<(String, Vec<Befund>, Vec<String>)> {
    let wurzel = dom(ids)?;
    let titel = wurzel
        .kind("info")
        .and_then(|i| i.kind("title"))
        .map(|t| t.text.trim().to_string())
        .unwrap_or_else(|| "IDS".into());
    let specs = wurzel
        .kind("specifications")
        .ok_or_else(|| xml_fehler("keine <specifications>"))?;
    let schema = a.schema.to_ascii_uppercase();
    let alle = prueflinge(a);
    let mut befunde = Vec::new();
    let mut offen = Vec::new();
    for s in specs.alle("specification") {
        let name = s.attr("name").unwrap_or("Spezifikation").to_string();
        let kennung = s.attr("identifier").unwrap_or(&name).to_string();
        let versionen = s.attr("ifcVersion").unwrap_or("");
        let schema_passt = versionen.split_whitespace().any(|v| {
            let v = v.to_ascii_uppercase();
            schema == v || (v.starts_with("IFC4X3") && schema.starts_with("IFC4X3"))
        });
        if !versionen.is_empty() && !schema_passt {
            offen.push(format!("„{name}“: gilt nicht für {schema}"));
            continue;
        }
        let stufe = if kennung.starts_with("W-") {
            Befundstufe::Warnung
        } else if kennung.starts_with("H-") {
            Befundstufe::Hinweis
        } else {
            Befundstufe::Fehler
        };
        let anwendbar_k = s.kind("applicability");
        let mut nicht_auswertbar = Vec::new();
        let betroffen: Vec<&Pruefling> = alle
            .iter()
            .filter(|p| {
                anwendbar_k.is_some_and(|ak| {
                    ak.kinder
                        .iter()
                        .all(|f| facette(f, p, &mut nicht_auswertbar) == Some(true))
                })
            })
            .collect();
        let min: u64 = anwendbar_k
            .and_then(|k| k.attr("minOccurs")?.parse().ok())
            .unwrap_or(0);
        let max = anwendbar_k
            .and_then(|k| k.attr("maxOccurs"))
            .unwrap_or("unbounded");
        let regel = format!("IDS:{kennung}");
        let hinweis = s
            .attr("instructions")
            .or(s.attr("description"))
            .unwrap_or("")
            .to_string();
        if (betroffen.len() as u64) < min {
            befunde.push(Befund {
                regel: regel.clone(),
                stufe,
                titel: name.clone(),
                text: format!("Im Modell fehlt ein solches Element. {hinweis}")
                    .trim()
                    .into(),
                elemente: Vec::new(),
                aktion: None,
                eigenschaft: None,
            });
        }
        if max == "0" && !betroffen.is_empty() {
            befunde.push(Befund {
                regel: regel.clone(),
                stufe,
                titel: name.clone(),
                text: "Diese Elemente sind nicht zulässig.".into(),
                elemente: betroffen.iter().map(|p| p.gid.to_string()).collect(),
                aktion: None,
                eigenschaft: None,
            });
        }
        if let Some(req) = s.kind("requirements") {
            for f in &req.kinder {
                let karte = f.attr("cardinality").unwrap_or("required");
                if karte == "optional" {
                    continue;
                }
                let mut verletzt = Vec::new();
                for p in &betroffen {
                    match facette(f, p, &mut nicht_auswertbar) {
                        Some(ok) if (karte == "prohibited") == ok => {
                            verletzt.push(p.gid.to_string())
                        }
                        _ => {}
                    }
                }
                if verletzt.is_empty() {
                    continue;
                }
                let (was, pset) = facettentext(f);
                let aktion = match f.name.as_str() {
                    "property" => Some(Befundaktion::EigenschaftSetzen),
                    "attribute" if was.contains("PredefinedType") => {
                        Some(Befundaktion::Klassifizieren)
                    }
                    "attribute" => Some(Befundaktion::Benennen),
                    "entity" => Some(Befundaktion::Klassifizieren),
                    _ => None,
                };
                befunde.push(Befund {
                    regel: regel.clone(),
                    stufe,
                    titel: name.clone(),
                    text: format!(
                        "{} von {} Element(en): {was} {}. {hinweis}",
                        verletzt.len(),
                        betroffen.len(),
                        if karte == "prohibited" {
                            "ist unzulässig"
                        } else {
                            "fehlt oder passt nicht"
                        }
                    )
                    .trim()
                    .into(),
                    elemente: verletzt,
                    aktion,
                    eigenschaft: pset,
                });
            }
        }
        nicht_auswertbar.sort();
        nicht_auswertbar.dedup();
        offen.extend(
            nicht_auswertbar
                .into_iter()
                .map(|n| format!("„{name}“: {n}")),
        );
    }
    Ok((titel, befunde, offen))
}

/// Eingebaute Regeln, unabhängig vom IDS.
pub fn eingebaute_regeln(a: &Modellauszug) -> Vec<Befund> {
    let mut b = Vec::new();
    let mut add = |regel: &str, stufe, titel: &str, text: String, elemente: Vec<String>, aktion| {
        b.push(Befund {
            regel: regel.into(),
            stufe,
            titel: titel.into(),
            text,
            elemente,
            aktion,
            eigenschaft: None,
        })
    };
    use Befundstufe::*;
    for h in &a.bericht.hinweise {
        if h.contains("Meter angenommen") {
            add(
                "einheit",
                Fehler,
                "Einheiten fehlen",
                format!(
                    "{h} Längen und Flächen vor der Auswertung mit einer bekannten Strecke kontrollieren."
                ),
                vec![],
                None,
            );
        }
        if h.contains("doppelte GlobalId") {
            add(
                "globalid",
                Fehler,
                "Doppelte GlobalIds",
                h.clone(),
                vec![],
                None,
            );
        }
    }
    if a.georeferenz.is_none() {
        add("georeferenz", Hinweis, "Keine Georeferenz", "Das Modell liegt nicht in Landeskoordinaten. Für den Lageplan das Gebäude von Hand einpassen.".into(), vec![], None);
    }
    if a.geschosse.is_empty() {
        add("geschosse", Fehler, "Keine Geschosse", "Ohne Geschosse sind Geschossflächen, GFZ und Raumbuch nicht auswertbar. Elemente in der Nacharbeit Geschossen zuordnen.".into(), vec![], Some(Befundaktion::GeschossZuordnen));
    }
    let gids = |f: &dyn Fn(&super::Modellelement) -> bool| -> Vec<String> {
        a.elemente
            .iter()
            .filter(|e| f(e))
            .map(|e| e.global_id.clone())
            .collect()
    };
    let ohne = gids(&|e| e.geschoss.is_none());
    if !ohne.is_empty() && !a.geschosse.is_empty() {
        add(
            "ohne_geschoss",
            Warnung,
            "Elemente ohne Geschoss",
            format!(
                "{} Element(e) gehören zu keinem Geschoss und fehlen in Geschossflächen und Raumbuch.",
                ohne.len()
            ),
            ohne,
            Some(Befundaktion::GeschossZuordnen),
        );
    }
    let proxy = gids(&|e| e.klasse == "IFCBUILDINGELEMENTPROXY");
    if !proxy.is_empty() {
        add(
            "proxy",
            Warnung,
            "Unklassifizierte Bauteile",
            format!(
                "{} Bauteil(e) sind nur als IfcBuildingElementProxy exportiert. Für Außenkante und GRZ als Wand, Decke, Stütze … klassifizieren.",
                proxy.len()
            ),
            proxy,
            Some(Befundaktion::Klassifizieren),
        );
    }
    // Aggregate (Dach → Dachplatten) haben ihre Geometrie in den Teilen.
    let ohne_geo = gids(&|e| {
        e.geometrie.is_none()
            && !a
                .elemente
                .iter()
                .any(|t| t.teil_von.as_deref() == Some(&e.global_id) && t.geometrie.is_some())
    });
    if !ohne_geo.is_empty() {
        add(
            "ohne_geometrie",
            Warnung,
            "Elemente ohne auswertbare Geometrie",
            format!(
                "{} Element(e) gehen nicht in Flächen ein (Darstellung fehlt oder wird nicht ausgewertet).",
                ohne_geo.len()
            ),
            ohne_geo,
            None,
        );
    }
    let naeh = gids(&|e| e.geometrie.as_ref().is_some_and(|g| g.naeherung));
    if !naeh.is_empty() {
        add(
            "naeherung",
            Hinweis,
            "Angenäherte Geometrie",
            format!(
                "{} Element(e) mit booleschen Operationen oder schrägen Körpern sind angenähert (erster Operand bzw. Hülle).",
                naeh.len()
            ),
            naeh,
            None,
        );
    }
    for o in &a.bericht.offene_darstellungen {
        add(
            "darstellung",
            Hinweis,
            "Nicht ausgewertete Darstellung",
            format!("{}: {} Element(e).", o.titel, o.anzahl),
            vec![],
            None,
        );
    }
    // Geschosse ohne Räume und Raummengen gegen Geometrie.
    for g in &a.geschosse {
        let raeume = a
            .elemente
            .iter()
            .filter(|e| e.klasse == "IFCSPACE" && e.geschoss.as_deref() == Some(&g.global_id))
            .count();
        let bauteile = a
            .elemente
            .iter()
            .filter(|e| e.klasse != "IFCSPACE" && e.geschoss.as_deref() == Some(&g.global_id))
            .count();
        if raeume == 0 && bauteile > 0 {
            add(
                "raeume_fehlen",
                Warnung,
                "Geschoss ohne Räume",
                format!(
                    "„{}“ hat keine Räume (IfcSpace). Für Raumbuch und Wohnfläche Räume aus den Wänden bilden.",
                    g.name
                ),
                vec![g.global_id.clone()],
                Some(Befundaktion::RaumBilden),
            );
        }
    }
    let mut abweichend = Vec::new();
    let mut beispiel = String::new();
    for e in a.elemente.iter().filter(|e| e.klasse == "IFCSPACE") {
        let (Some(geo), Some(menge)) = (
            e.geometrie.as_ref(),
            e.mengen
                .iter()
                .find(|m| m.name == "NetFloorArea" || m.name == "GrossFloorArea"),
        ) else {
            continue;
        };
        let d = (menge.wert - geo.grundflaeche_m2).abs();
        if d > MENGE_ABS_M2.max(MENGE_REL * geo.grundflaeche_m2) {
            if beispiel.is_empty() {
                let z = |v: f64| format!("{v:.2}").replace('.', ",");
                beispiel = format!(
                    "z. B. {}: {} {} m², Geometrie {} m²",
                    if e.name.is_empty() { "Raum" } else { &e.name },
                    menge.name,
                    z(menge.wert),
                    z(geo.grundflaeche_m2)
                );
            }
            abweichend.push(e.global_id.clone());
        }
    }
    if !abweichend.is_empty() {
        add(
            "raummenge",
            Warnung,
            "Raummengen weichen von der Geometrie ab",
            format!(
                "{} Raum/Räume ({beispiel}). BauPlanDESK rechnet mit der Geometrie; die Mengen der Datei sind vermutlich veraltet oder nach anderer Regel ermittelt.",
                abweichend.len()
            ),
            abweichend,
            None,
        );
    }
    let unbenannt = gids(&|e| {
        e.klasse == "IFCSPACE"
            && e.name.is_empty()
            && e.langname.as_deref().is_none_or(str::is_empty)
    });
    if !unbenannt.is_empty() {
        add(
            "raumname",
            Hinweis,
            "Räume ohne Namen",
            format!("{} Raum/Räume ohne Nummer und Nutzung.", unbenannt.len()),
            unbenannt,
            Some(Befundaktion::Benennen),
        );
    }
    for w in a.geschosse.windows(2) {
        if let (Some(h0), Some(h1)) = (w[0].hoehe_m, w[1].hoehe_m) {
            let d = h1 - h0;
            if !(2.0..=6.0).contains(&d) {
                add(
                    "geschosshoehe",
                    Hinweis,
                    "Ungewöhnliche Geschosshöhe",
                    format!(
                        "„{}“ → „{}“: {} m. Geschossebenen im Modell prüfen.",
                        w[0].name,
                        w[1].name,
                        format!("{d:.2}").replace('.', ",")
                    ),
                    vec![w[0].global_id.clone(), w[1].global_id.clone()],
                    None,
                );
            }
        }
    }
    b
}

pub fn pruefen(a: &Modellauszug, ids: &str) -> CoreResult<Modellpruefung> {
    let (regelwerk, mut befunde, nicht_geprueft) = ids_pruefen(a, ids)?;
    befunde.extend(eingebaute_regeln(a));
    befunde.sort_by_key(|b| b.stufe);
    let zahl = |s| befunde.iter().filter(|b| b.stufe == s).count();
    Ok(Modellpruefung {
        regelwerk,
        fehler: zahl(Befundstufe::Fehler),
        warnungen: zahl(Befundstufe::Warnung),
        hinweise: zahl(Befundstufe::Hinweis),
        befunde,
        nicht_geprueft,
    })
}
