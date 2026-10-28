//! Reimport: eine neue Modellversion (neue Dokumentversion oder andere
//! IFC-Datei) gegen das bisherige Modell der Akte abgleichen, über die
//! GlobalId. Die Nacharbeit wandert mit; was die neue Version selbst schon
//! richtig hat, ist „überholt“ und kann entfallen.
use super::nacharbeit::{Eigenschaftswert, Modellaenderung};
use super::{Modellauszug, Modellelement, klassentitel};
use geo::{Contains, InteriorPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Nacharbeitsstand {
    /// Wirkt weiter auf die neue Version.
    Wirksam,
    /// Die neue Version enthält das Element nicht mehr.
    Verwaist,
    /// Die neue Version hat den Zustand schon selbst.
    Ueberholt,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Elementaenderung {
    pub global_id: String,
    pub titel: String,
    /// Was sich geändert hat, z. B. „Klasse Proxy → Wand“, „Fläche +1,20 m²“.
    pub aenderungen: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Elementverweis {
    pub global_id: String,
    pub titel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellabgleich {
    pub alt_schema: String,
    pub neu_schema: String,
    pub neu: Vec<Elementverweis>,
    pub entfernt: Vec<Elementverweis>,
    pub geaendert: Vec<Elementaenderung>,
    pub unveraendert: usize,
    /// Geschosse: hinzugekommen, entfallen, Höhe geändert (Texte).
    pub geschosse: Vec<String>,
    /// Stand jeder Nacharbeit, in der Reihenfolge der Liste.
    pub nacharbeit: Vec<Nacharbeitsstand>,
    pub hinweise: Vec<String>,
}

fn titel(e: &Modellelement) -> String {
    let k = klassentitel(&e.klasse);
    match (e.langname.as_deref(), e.name.as_str()) {
        (Some(l), n) if !l.is_empty() => format!("{k} {n} {l}").trim().to_string(),
        (_, n) if !n.is_empty() => format!("{k} {n}"),
        _ => k,
    }
}

fn z(v: f64) -> String {
    format!("{v:+.2}").replace('.', ",")
}

fn unterschiede(
    a: &Modellauszug,
    b: &Modellauszug,
    x: &Modellelement,
    y: &Modellelement,
) -> Vec<String> {
    let mut d = Vec::new();
    if x.klasse != y.klasse {
        d.push(format!(
            "Klasse {} → {}",
            klassentitel(&x.klasse),
            klassentitel(&y.klasse)
        ));
    }
    if x.vordefiniert != y.vordefiniert {
        d.push(format!(
            "Art {} → {}",
            x.vordefiniert.as_deref().unwrap_or("–"),
            y.vordefiniert.as_deref().unwrap_or("–")
        ));
    }
    if x.name != y.name {
        d.push(format!("Name „{}“ → „{}“", x.name, y.name));
    }
    if x.langname != y.langname {
        d.push(format!(
            "Langname „{}“ → „{}“",
            x.langname.as_deref().unwrap_or(""),
            y.langname.as_deref().unwrap_or("")
        ));
    }
    let gname = |m: &Modellauszug, g: &Option<String>| {
        g.as_deref()
            .and_then(|g| m.geschoss(g))
            .map_or("ohne Geschoss".to_string(), |s| s.name.clone())
    };
    if gname(a, &x.geschoss) != gname(b, &y.geschoss) {
        d.push(format!(
            "Geschoss {} → {}",
            gname(a, &x.geschoss),
            gname(b, &y.geschoss)
        ));
    }
    match (&x.geometrie, &y.geometrie) {
        (Some(g), Some(h)) => {
            let df = h.grundflaeche_m2 - g.grundflaeche_m2;
            if df.abs() > 0.01 {
                d.push(format!("Grundfläche {} m²", z(df)));
            }
            if (h.oben_m - g.oben_m).abs() > 0.01 {
                d.push(format!("Oberkante {} m", z(h.oben_m - g.oben_m)));
            }
            if (h.unten_m - g.unten_m).abs() > 0.01 {
                d.push(format!("Unterkante {} m", z(h.unten_m - g.unten_m)));
            }
        }
        (None, Some(_)) => d.push("Geometrie neu".into()),
        (Some(_), None) => d.push("Geometrie entfallen".into()),
        (None, None) => {}
    }
    let schluessel = |e: &Modellelement| {
        let mut v: Vec<_> = e
            .eigenschaften
            .iter()
            .map(|p| format!("{}.{}={}", p.satz, p.name, p.wert))
            .collect();
        v.sort();
        v
    };
    let (ex, ey) = (schluessel(x), schluessel(y));
    if ex != ey {
        let n = ey.iter().filter(|p| !ex.contains(p)).count()
            + ex.iter().filter(|p| !ey.contains(p)).count();
        d.push(format!("{n} Eigenschaftsänderung(en)"));
    }
    d
}

/// Gleicht zwei Auszüge (ohne Nacharbeit) ab und bewertet die Nacharbeit.
pub fn abgleichen(
    alt: &Modellauszug,
    neu: &Modellauszug,
    nacharbeit: &[Modellaenderung],
) -> Modellabgleich {
    let mut v = Modellabgleich {
        alt_schema: alt.schema.clone(),
        neu_schema: neu.schema.clone(),
        neu: Vec::new(),
        entfernt: Vec::new(),
        geaendert: Vec::new(),
        unveraendert: 0,
        geschosse: Vec::new(),
        nacharbeit: Vec::new(),
        hinweise: Vec::new(),
    };
    for y in &neu.elemente {
        match alt.element(&y.global_id) {
            None => v.neu.push(Elementverweis {
                global_id: y.global_id.clone(),
                titel: titel(y),
            }),
            Some(x) => {
                let d = unterschiede(alt, neu, x, y);
                if d.is_empty() {
                    v.unveraendert += 1;
                } else {
                    v.geaendert.push(Elementaenderung {
                        global_id: y.global_id.clone(),
                        titel: titel(y),
                        aenderungen: d,
                    });
                }
            }
        }
    }
    for x in &alt.elemente {
        if neu.element(&x.global_id).is_none() {
            v.entfernt.push(Elementverweis {
                global_id: x.global_id.clone(),
                titel: titel(x),
            });
        }
    }
    for g in &neu.geschosse {
        match alt.geschoss(&g.global_id) {
            None => v.geschosse.push(format!("Geschoss „{}“ neu", g.name)),
            Some(a) if (a.hoehe_m.unwrap_or(0.) - g.hoehe_m.unwrap_or(0.)).abs() > 0.005 => {
                v.geschosse.push(format!(
                    "Geschoss „{}“: Höhe {} m",
                    g.name,
                    z(g.hoehe_m.unwrap_or(0.) - a.hoehe_m.unwrap_or(0.))
                ))
            }
            Some(a) if a.name != g.name => v
                .geschosse
                .push(format!("Geschoss „{}“ heißt jetzt „{}“", a.name, g.name)),
            _ => {}
        }
    }
    for a in &alt.geschosse {
        if neu.geschoss(&a.global_id).is_none() {
            v.geschosse.push(format!("Geschoss „{}“ entfallen", a.name));
        }
    }
    let wert_gleich = |e: &Modellelement, satz: &str, name: &str, w: &Eigenschaftswert| {
        e.eigenschaften.iter().any(|p| {
            p.satz == satz
                && p.name == name
                && match w {
                    Eigenschaftswert::Text { wert } => p.wert == *wert,
                    Eigenschaftswert::Zahl { wert } => {
                        p.zahl.is_some_and(|z| (z - wert).abs() < 1e-9)
                    }
                    Eigenschaftswert::Logisch { wert } => {
                        p.wert == if *wert { "Ja" } else { "Nein" }
                    }
                }
        })
    };
    for c in nacharbeit {
        let gid = c.global_id();
        let el = neu.element(gid);
        let struktur = neu
            .geschosse
            .iter()
            .chain(&neu.gebaeude)
            .chain(&neu.grundstuecke)
            .find(|s| s.global_id == gid);
        let stand = match c {
            Modellaenderung::RaumBilden {
                geschoss, umriss, ..
            } => {
                if neu.geschoss(geschoss).is_none() {
                    Nacharbeitsstand::Verwaist
                } else {
                    // Überholt, wenn die neue Version dort selbst einen Raum hat.
                    let p = geo::Polygon::new(
                        geo::LineString::from(
                            umriss.iter().map(|q| (q[0], q[1])).collect::<Vec<_>>(),
                        ),
                        vec![],
                    );
                    let innen = p.interior_point();
                    let belegt = innen.is_some_and(|q| {
                        neu.elemente.iter().any(|e| {
                            e.klasse == "IFCSPACE"
                                && e.geschoss.as_deref() == Some(geschoss.as_str())
                                && e.geometrie.as_ref().is_some_and(|g| {
                                    super::geometrie::multipolygon(&g.grundriss).contains(&q)
                                })
                        })
                    });
                    if belegt {
                        Nacharbeitsstand::Ueberholt
                    } else {
                        Nacharbeitsstand::Wirksam
                    }
                }
            }
            _ if el.is_none() && struktur.is_none() => Nacharbeitsstand::Verwaist,
            Modellaenderung::Klassifizieren {
                klasse,
                vordefiniert,
                ..
            } => {
                let e = el.expect("geprüft");
                if e.klasse == *klasse
                    && (vordefiniert.is_none() || e.vordefiniert == *vordefiniert)
                {
                    Nacharbeitsstand::Ueberholt
                } else {
                    Nacharbeitsstand::Wirksam
                }
            }
            Modellaenderung::GeschossZuordnen { geschoss, .. } => match el {
                Some(e) if e.geschoss.as_deref() == Some(geschoss.as_str()) => {
                    Nacharbeitsstand::Ueberholt
                }
                Some(_) if neu.geschoss(geschoss).is_none() => Nacharbeitsstand::Verwaist,
                _ => Nacharbeitsstand::Wirksam,
            },
            Modellaenderung::EigenschaftSetzen {
                satz, name, wert, ..
            } => match el {
                Some(e) if wert_gleich(e, satz, name, wert) => Nacharbeitsstand::Ueberholt,
                _ => Nacharbeitsstand::Wirksam,
            },
            Modellaenderung::Benennen { name, langname, .. } => {
                let (n, l) = el
                    .map(|e| (e.name.as_str(), e.langname.as_deref()))
                    .or(struktur.map(|s| (s.name.as_str(), s.langname.as_deref())))
                    .expect("geprüft");
                if n == name && l == langname.as_deref() {
                    Nacharbeitsstand::Ueberholt
                } else {
                    Nacharbeitsstand::Wirksam
                }
            }
            Modellaenderung::Ausblenden { .. } => Nacharbeitsstand::Wirksam,
            Modellaenderung::HoeheSetzen {
                unten_m, oben_m, ..
            } => match el.and_then(|e| e.geometrie.as_ref()) {
                None => Nacharbeitsstand::Verwaist,
                Some(g)
                    if unten_m.is_none_or(|u| (g.unten_m - u).abs() < 0.005)
                        && oben_m.is_none_or(|o| (g.oben_m - o).abs() < 0.005) =>
                {
                    Nacharbeitsstand::Ueberholt
                }
                Some(_) => Nacharbeitsstand::Wirksam,
            },
            Modellaenderung::UmrissSetzen { umriss, .. } => {
                match el.and_then(|e| e.geometrie.as_ref()) {
                    None => Nacharbeitsstand::Verwaist,
                    Some(g)
                        if (g.grundflaeche_m2 - super::nacharbeit::umriss_flaeche(umriss))
                            .abs()
                            < 0.01 =>
                    {
                        Nacharbeitsstand::Ueberholt
                    }
                    Some(_) => Nacharbeitsstand::Wirksam,
                }
            }
        };
        v.nacharbeit.push(stand);
    }
    let verwaist = v
        .nacharbeit
        .iter()
        .filter(|s| **s == Nacharbeitsstand::Verwaist)
        .count();
    if verwaist > 0 {
        v.hinweise.push(format!(
            "{verwaist} Nacharbeit(en) betreffen Elemente, die die neue Version nicht mehr enthält."
        ));
    }
    let gleiche = alt
        .elemente
        .iter()
        .filter(|e| neu.element(&e.global_id).is_some())
        .count();
    if !alt.elemente.is_empty() && gleiche * 2 < alt.elemente.len() {
        v.hinweise.push(format!(
            "Nur {gleiche} von {} Elementen haben ihre GlobalId behalten. Erzeugt die Software beim Export neue GlobalIds, geht die Zuordnung der Nacharbeit verloren; im Export die GlobalIds erhalten.",
            alt.elemente.len()
        ));
    }
    v
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Reimportwunsch {
    pub akte_id: uuid::Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    /// Neue IFC-Datei (neue Version derselben Datei oder eine andere).
    pub dokument: crate::plattform::objekte::Dokumentbezug,
    /// Überholte Nacharbeit (die neue Version hat den Zustand selbst) streichen.
    #[serde(default)]
    pub ueberholte_entfernen: bool,
    /// Verwaiste Nacharbeit (Element fehlt in der neuen Version) streichen.
    #[serde(default)]
    pub verwaiste_entfernen: bool,
}
