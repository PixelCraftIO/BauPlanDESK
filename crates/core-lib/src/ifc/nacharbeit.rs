//! Nacharbeit: Änderungen am Modell als Liste in der Modellakte, über die
//! GlobalId an die Elemente gebunden. Die IFC-Datei bleibt unverändert; der
//! Auszug der Modellakte ist das gelesene Modell mit angewendeter Liste.
//! So überstehen die Änderungen einen Reimport (Abgleich über die GlobalId)
//! und lassen sich beim Export in die Datei schreiben.
use super::geometrie::{self, Teil, multipolygon, planpolygone, vereinen};
use super::{Klassenzahl, Modellauszug, Modelleigenschaft, Modellelement, auszug, klassentitel};
use crate::planaufmass::Planpolygon;
use crate::{CoreError, CoreResult};
use geo::{Area, Contains, Orient};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Klassen, in die ein Bauteil umklassifiziert werden kann.
pub const KLASSEN: &[&str] = &[
    "IFCWALL",
    "IFCSLAB",
    "IFCROOF",
    "IFCCOLUMN",
    "IFCBEAM",
    "IFCSTAIR",
    "IFCRAMP",
    "IFCRAILING",
    "IFCCOVERING",
    "IFCCURTAINWALL",
    "IFCWINDOW",
    "IFCDOOR",
    "IFCPLATE",
    "IFCMEMBER",
    "IFCFOOTING",
    "IFCCHIMNEY",
    "IFCSHADINGDEVICE",
    "IFCFURNISHINGELEMENT",
    "IFCBUILDINGELEMENTPROXY",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "typ", rename_all = "snake_case", deny_unknown_fields)]
pub enum Eigenschaftswert {
    Text { wert: String },
    Zahl { wert: f64 },
    Logisch { wert: bool },
}

impl Eigenschaftswert {
    fn anzeige(&self) -> (String, Option<f64>) {
        match self {
            Self::Text { wert } => (wert.clone(), None),
            Self::Zahl { wert } => (format!("{wert}").replace('.', ","), Some(*wert)),
            Self::Logisch { wert } => ((if *wert { "Ja" } else { "Nein" }).into(), None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Modellaenderung {
    /// Klasse (und Art) ändern, z. B. Proxy → Wand, Platte → Dach.
    Klassifizieren {
        global_id: String,
        klasse: String,
        vordefiniert: Option<String>,
    },
    /// Element einem Geschoss (GlobalId) zuordnen.
    GeschossZuordnen { global_id: String, geschoss: String },
    /// Eigenschaft setzen oder ändern (Pset, Name, Wert).
    EigenschaftSetzen {
        global_id: String,
        satz: String,
        name: String,
        wert: Eigenschaftswert,
    },
    /// Name und Langname (bei Räumen: Nummer und Nutzung).
    Benennen {
        global_id: String,
        name: String,
        langname: Option<String>,
    },
    /// Neuer Raum aus einem Umriss in Modellkoordinaten (m), meist aus den
    /// Wänden vorgeschlagen.
    RaumBilden {
        global_id: String,
        geschoss: String,
        name: String,
        langname: Option<String>,
        umriss: Vec<[f64; 2]>,
        /// Lichte Höhe; ohne Angabe bis zur nächsten Geschossebene.
        hoehe_m: Option<f64>,
    },
    /// Element bei Auswertung, Übernahme und Prüfung nicht berücksichtigen
    /// (z. B. Möblierung, Hilfskörper).
    Ausblenden { global_id: String },
}

impl Modellaenderung {
    pub fn global_id(&self) -> &str {
        match self {
            Self::Klassifizieren { global_id, .. }
            | Self::GeschossZuordnen { global_id, .. }
            | Self::EigenschaftSetzen { global_id, .. }
            | Self::Benennen { global_id, .. }
            | Self::RaumBilden { global_id, .. }
            | Self::Ausblenden { global_id } => global_id,
        }
    }
}

fn fehler(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}

fn ist_global_id(s: &str) -> bool {
    s.len() == 22
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
}

fn umriss_polygon(umriss: &[[f64; 2]]) -> CoreResult<geo::Polygon<f64>> {
    if umriss.len() < 3 || umriss.len() > 2000 || umriss.iter().flatten().any(|v| !v.is_finite()) {
        return Err(fehler("Raumumriss braucht 3 bis 2000 gültige Punkte"));
    }
    let p = geo::Polygon::new(
        geo::LineString::from(umriss.iter().map(|q| (q[0], q[1])).collect::<Vec<_>>()),
        vec![],
    )
    .orient(geo::orient::Direction::Default);
    if p.unsigned_area() < 0.1 {
        return Err(fehler("Raumumriss ist kleiner als 0,1 m²"));
    }
    Ok(p)
}

/// Fläche eines Raumumrisses (m², 0 bei ungültigem Umriss).
pub(super) fn umriss_flaeche(umriss: &[[f64; 2]]) -> f64 {
    umriss_polygon(umriss).map_or(0., |p| geometrie::rund(p.unsigned_area()))
}

/// Höhe der nächsten Geschossebene über `g` (für Räume ohne Höhenangabe).
fn naechste_ebene(a: &Modellauszug, geschoss: &str) -> Option<f64> {
    let i = a.geschosse.iter().position(|g| g.global_id == geschoss)?;
    let h = a.geschosse[i].hoehe_m.unwrap_or(0.);
    a.geschosse
        .get(i + 1)
        .and_then(|n| n.hoehe_m)
        .map(|n| n - h)
}

/// Wendet die Nacharbeit an. Änderungen für Elemente, die das Modell nicht
/// (mehr) enthält, bleiben wirkungslos und werden als Hinweis gemeldet.
pub fn anwenden(a: &mut Modellauszug, liste: &[Modellaenderung]) -> Vec<String> {
    let mut verwaist = Vec::new();
    let mut aus = std::collections::HashSet::new();
    for c in liste {
        let gid = c.global_id().to_string();
        if let Modellaenderung::RaumBilden {
            global_id,
            geschoss,
            name,
            langname,
            umriss,
            hoehe_m,
        } = c
        {
            let Some(g) = a.geschoss(geschoss) else {
                verwaist.push(gid);
                continue;
            };
            let Ok(p) = umriss_polygon(umriss) else {
                verwaist.push(gid);
                continue;
            };
            let unten = g.hoehe_m.unwrap_or(0.);
            let gebaeude = g.teil_von.clone();
            let h = hoehe_m
                .or_else(|| naechste_ebene(a, geschoss))
                .unwrap_or(2.5);
            let teile = vec![Teil::Prisma {
                grundriss: geo::MultiPolygon(vec![p]),
                unten,
                oben: unten + h,
            }];
            a.elemente.retain(|e| e.global_id != *global_id);
            a.elemente.push(Modellelement {
                global_id: global_id.clone(),
                klasse: "IFCSPACE".into(),
                name: name.clone(),
                langname: langname.clone(),
                objekttyp: None,
                vordefiniert: Some("INTERNAL".into()),
                geschoss: Some(geschoss.clone()),
                gebaeude,
                teil_von: Some(geschoss.clone()),
                eigenschaften: Vec::new(),
                mengen: Vec::new(),
                geometrie: super::auszug::geometrie_aus(&teile, false),
                geometrie_offen: Vec::new(),
                step_id: 0,
                teile,
            });
            continue;
        }
        let geschosse: Vec<(String, Option<String>)> = a
            .geschosse
            .iter()
            .map(|g| (g.global_id.clone(), g.teil_von.clone()))
            .collect();
        // Strukturelemente lassen sich benennen und mit Eigenschaften versehen.
        if let Some(s) = a
            .geschosse
            .iter_mut()
            .chain(a.gebaeude.iter_mut())
            .chain(a.grundstuecke.iter_mut())
            .find(|s| s.global_id == gid)
        {
            match c {
                Modellaenderung::Benennen { name, langname, .. } => {
                    s.name = name.clone();
                    s.langname = langname.clone();
                }
                Modellaenderung::EigenschaftSetzen {
                    satz, name, wert, ..
                } => eigenschaft(&mut s.eigenschaften, satz, name, wert),
                _ => verwaist.push(gid),
            }
            continue;
        }
        let Some(e) = a.elemente.iter_mut().find(|e| e.global_id == gid) else {
            verwaist.push(gid);
            continue;
        };
        match c {
            Modellaenderung::Klassifizieren {
                klasse,
                vordefiniert,
                ..
            } => {
                e.klasse = klasse.clone();
                e.vordefiniert = vordefiniert.clone();
            }
            Modellaenderung::GeschossZuordnen { geschoss, .. } => {
                match geschosse.iter().find(|(g, _)| g == geschoss) {
                    Some((g, b)) => {
                        e.geschoss = Some(g.clone());
                        e.gebaeude = b.clone().or(e.gebaeude.take());
                    }
                    None => verwaist.push(gid),
                }
            }
            Modellaenderung::EigenschaftSetzen {
                satz, name, wert, ..
            } => eigenschaft(&mut e.eigenschaften, satz, name, wert),
            Modellaenderung::Benennen { name, langname, .. } => {
                e.name = name.clone();
                if e.klasse == "IFCSPACE" || langname.is_some() {
                    e.langname = langname.clone();
                }
            }
            Modellaenderung::Ausblenden { .. } => {
                aus.insert(gid);
            }
            Modellaenderung::RaumBilden { .. } => unreachable!(),
        }
    }
    a.elemente.retain(|e| !aus.contains(&e.global_id));
    bericht_neu(a);
    verwaist.sort();
    verwaist.dedup();
    if !verwaist.is_empty() {
        a.bericht.hinweise.push(format!(
            "{} Nacharbeit(en) betreffen Elemente, die das Modell nicht enthält (z. B. {}).",
            verwaist.len(),
            verwaist[0]
        ));
    }
    if !aus.is_empty() {
        a.bericht.hinweise.push(format!(
            "{} Element(e) per Nacharbeit ausgeblendet.",
            aus.len()
        ));
    }
    verwaist
}

fn eigenschaft(l: &mut Vec<Modelleigenschaft>, satz: &str, name: &str, wert: &Eigenschaftswert) {
    let (text, zahl) = wert.anzeige();
    l.retain(|p| !(p.satz == satz && p.name == name));
    l.push(Modelleigenschaft {
        satz: satz.into(),
        name: name.into(),
        wert: text,
        zahl,
        vom_typ: false,
    });
}

/// Zählungen des Berichts nach der Nacharbeit neu.
fn bericht_neu(a: &mut Modellauszug) {
    let mut klassen: BTreeMap<String, usize> = BTreeMap::new();
    for e in &a.elemente {
        *klassen.entry(e.klasse.clone()).or_default() += 1;
    }
    let b = &mut a.bericht;
    b.klassen = klassen
        .into_iter()
        .map(|(klasse, anzahl)| Klassenzahl {
            titel: klassentitel(&klasse),
            klasse,
            anzahl,
        })
        .collect();
    b.elemente = a.elemente.len();
    b.mit_geometrie = a.elemente.iter().filter(|e| e.geometrie.is_some()).count();
    b.ohne_geometrie = b.elemente - b.mit_geometrie;
    b.naeherungen = a
        .elemente
        .iter()
        .filter(|e| e.geometrie.as_ref().is_some_and(|g| g.naeherung))
        .count();
    let raeume: Vec<_> = a
        .elemente
        .iter()
        .filter(|e| e.klasse == "IFCSPACE")
        .collect();
    b.raeume = raeume.len();
    b.raeume_mit_flaechenmenge = raeume
        .iter()
        .filter(|r| r.mengen.iter().any(|m| m.art == super::Mengenart::Flaeche))
        .count();
}

/// Prüft die Liste gegen das gelesene Modell (vor dem Speichern).
pub fn pruefen(a: &Modellauszug, liste: &[Modellaenderung]) -> CoreResult<()> {
    if liste.len() > 20_000 {
        return Err(fehler("Zu viele Nacharbeiten (höchstens 20 000)"));
    }
    let mut neu: Vec<&str> = Vec::new();
    let bekannt = |g: &str, neu: &[&str]| {
        a.element(g).is_some()
            || a.geschosse
                .iter()
                .chain(&a.gebaeude)
                .chain(&a.grundstuecke)
                .any(|s| s.global_id == g)
            || neu.contains(&g)
    };
    for c in liste {
        let g = c.global_id();
        if !ist_global_id(g) {
            return Err(fehler(format!("„{g}“ ist keine IFC-GlobalId")));
        }
        match c {
            Modellaenderung::RaumBilden {
                geschoss,
                name,
                umriss,
                hoehe_m,
                ..
            } => {
                if a.element(g).is_some() {
                    return Err(fehler("Neuer Raum braucht eine neue GlobalId"));
                }
                if a.geschoss(geschoss).is_none() {
                    return Err(fehler("Geschoss des Raums ist nicht im Modell"));
                }
                crate::plattform::text(name, 200)?;
                umriss_polygon(umriss)?;
                if hoehe_m.is_some_and(|h| !(0.5..=20.).contains(&h)) {
                    return Err(fehler("Raumhöhe zwischen 0,5 und 20 m angeben"));
                }
                neu.push(g);
            }
            // Ältere Änderungen dürfen verwaist sein (Reimport); neue nicht:
            // der Prüfer sieht nur die Gesamtliste und lässt beides zu.
            _ if !bekannt(g, &neu) => {}
            Modellaenderung::Klassifizieren {
                klasse,
                vordefiniert,
                ..
            } => {
                if !KLASSEN.contains(&klasse.as_str()) {
                    return Err(fehler(format!(
                        "Klasse {klasse} ist für die Nacharbeit nicht vorgesehen"
                    )));
                }
                if vordefiniert.as_deref().is_some_and(|v| {
                    v.is_empty()
                        || v.len() > 64
                        || !v.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
                }) {
                    return Err(fehler(
                        "Art (PredefinedType) in Großbuchstaben angeben, z. B. FLOOR",
                    ));
                }
            }
            Modellaenderung::GeschossZuordnen { geschoss, .. } => {
                if a.geschoss(geschoss).is_none() {
                    return Err(fehler("Geschoss ist nicht im Modell"));
                }
            }
            Modellaenderung::EigenschaftSetzen {
                satz, name, wert, ..
            } => {
                crate::plattform::text(satz, 100)?;
                crate::plattform::text(name, 100)?;
                if satz.trim().is_empty() || name.trim().is_empty() {
                    return Err(fehler("Eigenschaftssatz und Name angeben"));
                }
                match wert {
                    Eigenschaftswert::Text { wert } => crate::plattform::text(wert, 500)?,
                    Eigenschaftswert::Zahl { wert } if !wert.is_finite() => {
                        return Err(fehler("Zahlenwert ist ungültig"));
                    }
                    _ => {}
                }
            }
            Modellaenderung::Benennen { name, langname, .. } => {
                crate::plattform::text(name, 200)?;
                if let Some(l) = langname {
                    crate::plattform::text(l, 200)?;
                }
            }
            Modellaenderung::Ausblenden { .. } => {}
        }
    }
    Ok(())
}

/// Raumvorschlag aus den Wänden eines Geschosses.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Raumvorschlag {
    /// Neue GlobalId für den Raum, falls er gebildet wird.
    pub global_id: String,
    /// Umriss in Modellkoordinaten (m).
    pub umriss: Vec<[f64; 2]>,
    pub flaeche_m2: f64,
    /// Vorhandener Raum, der diesen Umriss schon abdeckt.
    pub vorhanden: Option<String>,
}

/// Geschlossene Flächen zwischen den Wänden (Wände, Stützen, Vorhangfassaden
/// im Schnitt), größer als 1 m². Türen in Wänden zählen als geschlossen.
pub fn raumvorschlaege(a: &Modellauszug, geschoss: &str) -> CoreResult<Vec<Raumvorschlag>> {
    let gr = super::grundriss(a, geschoss, None)?;
    let waende: Vec<geo::Polygon<f64>> = gr
        .teile
        .iter()
        .filter(|t| {
            !t.raum
                && matches!(
                    t.klasse
                        .trim_end_matches("STANDARDCASE")
                        .trim_end_matches("ELEMENTEDCASE"),
                    "IFCWALL" | "IFCCOLUMN" | "IFCCURTAINWALL" | "IFCBUILDINGELEMENTPROXY"
                )
        })
        .flat_map(|t| multipolygon(&t.polygone).0)
        .collect();
    if waende.is_empty() {
        return Ok(Vec::new());
    }
    let raeume: Vec<(String, geo::MultiPolygon<f64>)> = gr
        .teile
        .iter()
        .filter(|t| t.raum)
        .map(|t| (t.global_id.clone(), multipolygon(&t.polygone)))
        .collect();
    let mut v = Vec::new();
    for p in vereinen(waende) {
        for ring in p.interiors() {
            let loch =
                geo::Polygon::new(ring.clone(), vec![]).orient(geo::orient::Direction::Default);
            let f = loch.unsigned_area();
            if f < 1. {
                continue;
            }
            let pp: Vec<Planpolygon> = planpolygone(&geo::MultiPolygon(vec![loch.clone()]));
            let Some(umriss) = pp.into_iter().next().map(|p| p.aussen) else {
                continue;
            };
            let innen = geo::InteriorPoint::interior_point(&loch);
            let vorhanden = innen.and_then(|q| {
                raeume
                    .iter()
                    .find(|(_, m)| m.contains(&q))
                    .map(|(g, _)| g.clone())
            });
            v.push(Raumvorschlag {
                global_id: super::step::global_id(uuid::Uuid::new_v4()),
                umriss,
                flaeche_m2: (f * 1e4).round() / 1e4,
                vorhanden,
            });
        }
    }
    v.sort_by(|x, y| y.flaeche_m2.total_cmp(&x.flaeche_m2));
    Ok(v)
}

/// Auszug einer Datei mit angewendeter Nacharbeit (für Tests und Beispiele).
pub fn auszug_mit(
    d: &super::step::StepDatei,
    liste: &[Modellaenderung],
) -> CoreResult<Modellauszug> {
    let mut a = auszug(d)?;
    anwenden(&mut a, liste);
    Ok(a)
}
