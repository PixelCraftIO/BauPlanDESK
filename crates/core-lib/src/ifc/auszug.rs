//! Modellauszug: schemaunabhängige Sicht auf eine gelesene IFC-Datei.
use super::geometrie::{Geometrie, Teil, planpolygone, vereinen};
use super::step::{Instanz, StepDatei, Wert};
use super::*;
use crate::{CoreError, CoreResult};
use geo::Area;
use std::collections::{BTreeMap, HashMap};

const RAEUMLICH: [&str; 4] = ["IFCSITE", "IFCBUILDING", "IFCBUILDINGSTOREY", "IFCSPACE"];
/// Produkte ohne Bauteilcharakter: Öffnungen, Raster, Beschriftungen.
const AUSGENOMMEN: [&str; 9] = [
    "IFCOPENINGELEMENT",
    "IFCOPENINGSTANDARDCASE",
    "IFCVIRTUALELEMENT",
    "IFCANNOTATION",
    "IFCGRID",
    "IFCSITE",
    "IFCBUILDING",
    "IFCBUILDINGSTOREY",
    "IFCPROJECT",
];

/// Deutsche Bezeichnung einer IFC-Klasse für Listen und Berichte.
pub fn klassentitel(klasse: &str) -> String {
    let k = klasse
        .trim_end_matches("STANDARDCASE")
        .trim_end_matches("ELEMENTEDCASE");
    let t = match k {
        "IFCWALL" => "Wand",
        "IFCSLAB" => "Decke/Platte",
        "IFCROOF" => "Dach",
        "IFCSPACE" => "Raum",
        "IFCCOLUMN" => "Stütze",
        "IFCBEAM" => "Träger",
        "IFCDOOR" => "Tür",
        "IFCWINDOW" => "Fenster",
        "IFCSTAIR" => "Treppe",
        "IFCSTAIRFLIGHT" => "Treppenlauf",
        "IFCRAMP" => "Rampe",
        "IFCRAMPFLIGHT" => "Rampenlauf",
        "IFCRAILING" => "Geländer",
        "IFCCOVERING" => "Bekleidung",
        "IFCCURTAINWALL" => "Vorhangfassade",
        "IFCPLATE" => "Platte",
        "IFCMEMBER" => "Stab",
        "IFCFOOTING" => "Fundament",
        "IFCPILE" => "Pfahl",
        "IFCCHIMNEY" => "Schornstein",
        "IFCSHADINGDEVICE" => "Sonnenschutz",
        "IFCBUILDINGELEMENTPROXY" => "Unklassifiziertes Bauteil",
        "IFCFURNISHINGELEMENT" | "IFCFURNITURE" => "Möblierung",
        "IFCFLOWTERMINAL" | "IFCSANITARYTERMINAL" => "TGA-Endgerät",
        "IFCFLOWSEGMENT" | "IFCPIPESEGMENT" | "IFCDUCTSEGMENT" => "TGA-Leitung",
        "IFCTRANSPORTELEMENT" => "Aufzug/Fördertechnik",
        "IFCGEOGRAPHICELEMENT" => "Geländeelement",
        "IFCCIVILELEMENT" => "Ingenieurbauteil",
        "IFCSITE" => "Grundstück",
        "IFCBUILDING" => "Gebäude",
        "IFCBUILDINGSTOREY" => "Geschoss",
        _ => "",
    };
    if t.is_empty() {
        // Unbekannte Klasse: „IFCFLOWFITTING“ → „IfcFlowFitting“ wäre schöner, ist aber ohne Schema nicht ableitbar.
        format!(
            "Sonstiges ({})",
            klasse.trim_start_matches("IFC").to_lowercase()
        )
    } else {
        t.to_string()
    }
}

struct Einheiten {
    laenge: f64,
    flaeche: f64,
    volumen: f64,
    name: String,
}

fn vorsatz(p: Option<&str>) -> f64 {
    match p {
        Some("EXA") => 1e18,
        Some("PETA") => 1e15,
        Some("TERA") => 1e12,
        Some("GIGA") => 1e9,
        Some("MEGA") => 1e6,
        Some("KILO") => 1e3,
        Some("HECTO") => 1e2,
        Some("DECA") => 1e1,
        Some("DECI") => 1e-1,
        Some("CENTI") => 1e-2,
        Some("MILLI") => 1e-3,
        Some("MICRO") => 1e-6,
        Some("NANO") => 1e-9,
        _ => 1.,
    }
}

fn einheiten(d: &StepDatei) -> (Einheiten, Vec<String>) {
    let mut e = Einheiten {
        laenge: 1.,
        flaeche: 1.,
        volumen: 1.,
        name: "Meter (angenommen)".into(),
    };
    let mut hinweise = Vec::new();
    let Some(projekt) = d.alle("IFCPROJECT").next() else {
        hinweise.push("Kein IfcProject: Einheit Meter angenommen.".into());
        return (e, hinweise);
    };
    let Some(zuordnung) = d.typ(projekt.attr(8), &["IFCUNITASSIGNMENT"]) else {
        hinweise.push("Keine Einheiten im Projekt: Meter angenommen.".into());
        return (e, hinweise);
    };
    let mut laenge_gefunden = false;
    for u in zuordnung.attr(0).liste() {
        let Some(u) = d.get(u.referenz().unwrap_or(0)) else {
            continue;
        };
        let art = u.attr(1).enum_wert().unwrap_or("");
        let faktor = match u.typ.as_str() {
            "IFCSIUNIT" => {
                let basis = match u.attr(3).enum_wert() {
                    Some("METRE") | Some("SQUARE_METRE") | Some("CUBIC_METRE") => 1.,
                    _ => continue,
                };
                basis * vorsatz(u.attr(2).enum_wert())
            }
            "IFCCONVERSIONBASEDUNIT" => {
                let Some(m) = d.typ(u.attr(3), &["IFCMEASUREWITHUNIT"]) else {
                    continue;
                };
                let Some(wert) = m.attr(0).zahl() else {
                    continue;
                };
                // Bezugseinheit (meist Meter) einrechnen.
                let bezug = d
                    .typ(m.attr(1), &["IFCSIUNIT"])
                    .map(|s| vorsatz(s.attr(2).enum_wert()))
                    .unwrap_or(1.);
                wert * bezug
            }
            _ => continue,
        };
        match art {
            "LENGTHUNIT" => {
                laenge_gefunden = true;
                e.laenge = faktor;
                e.name = match u.typ.as_str() {
                    "IFCSIUNIT" => match u.attr(2).enum_wert() {
                        Some("MILLI") => "Millimeter".into(),
                        Some("CENTI") => "Zentimeter".into(),
                        Some("DECI") => "Dezimeter".into(),
                        Some("KILO") => "Kilometer".into(),
                        None => "Meter".into(),
                        Some(p) => format!("{} Meter", p.to_lowercase()),
                    },
                    _ => u.attr(2).text().unwrap_or("Umrechnungseinheit").to_string(),
                };
            }
            "AREAUNIT" => {
                e.flaeche = if u.typ == "IFCSIUNIT" {
                    faktor * faktor
                } else {
                    faktor
                }
            }
            "VOLUMEUNIT" => {
                e.volumen = if u.typ == "IFCSIUNIT" {
                    faktor * faktor * faktor
                } else {
                    faktor
                }
            }
            _ => {}
        }
    }
    if !laenge_gefunden {
        hinweise.push("Keine Längeneinheit im Projekt: Meter angenommen.".into());
    }
    (e, hinweise)
}

fn grad(w: &Wert) -> Option<f64> {
    let t: Vec<f64> = w.liste().iter().filter_map(Wert::zahl).collect();
    let g = *t.first()?;
    let s = g.signum();
    Some(
        g + s * t.get(1).unwrap_or(&0.).abs() / 60.
            + s * t.get(2).unwrap_or(&0.).abs() / 3600.
            + s * t.get(3).unwrap_or(&0.).abs() / 3.6e9,
    )
}

fn georeferenz(d: &StepDatei, m: f64) -> Option<Georeferenz> {
    if let Some(k) = d.alle("IFCMAPCONVERSION").next() {
        let crs = d
            .get(k.attr(1).referenz().unwrap_or(0))
            .and_then(|c| c.attr(0).text())
            .map(str::to_string);
        let (ab, or) = (k.attr(5).zahl(), k.attr(6).zahl());
        return Some(Georeferenz {
            crs,
            ostwert: k.attr(2).zahl(),
            nordwert: k.attr(3).zahl(),
            hoehe: k.attr(4).zahl(),
            drehung_grad: match (ab, or) {
                (Some(a), Some(o)) => Some(o.atan2(a).to_degrees()),
                _ => None,
            },
            breite: None,
            laenge: None,
        });
    }
    let site = d.alle("IFCSITE").next()?;
    let (b, l) = (grad(site.attr(9)), grad(site.attr(10)));
    (b.is_some() || l.is_some()).then(|| Georeferenz {
        crs: None,
        ostwert: None,
        nordwert: None,
        hoehe: site.attr(11).zahl().map(|h| h * m),
        drehung_grad: None,
        breite: b,
        laenge: l,
    })
}

fn zahl_text(z: f64) -> String {
    let t = format!("{:.3}", z);
    let t = t.trim_end_matches('0').trim_end_matches('.');
    t.replace('.', ",")
}

/// Anzeigewert und SI-Zahl eines (typisierten) Eigenschaftswerts.
fn eigenschaftswert(w: &Wert, e: &Einheiten) -> (String, Option<f64>) {
    match w {
        Wert::Typisiert(t, inner) => {
            let roh = inner.zahl();
            let (faktor, einheit) = match t.as_str() {
                "IFCLENGTHMEASURE" | "IFCPOSITIVELENGTHMEASURE" | "IFCNONNEGATIVELENGTHMEASURE" => {
                    (e.laenge, " m")
                }
                "IFCAREAMEASURE" => (e.flaeche, " m²"),
                "IFCVOLUMEMEASURE" => (e.volumen, " m³"),
                _ => (1., ""),
            };
            match (inner.as_ref(), roh) {
                (Wert::Enum(b), _) if t == "IFCBOOLEAN" || t == "IFCLOGICAL" => (
                    match b.as_str() {
                        "T" => "Ja",
                        "F" => "Nein",
                        _ => "Unbekannt",
                    }
                    .into(),
                    None,
                ),
                (_, Some(z)) => (
                    format!("{}{einheit}", zahl_text(z * faktor)),
                    Some(z * faktor),
                ),
                (Wert::Text(s), _) => (s.clone(), None),
                (Wert::Enum(s), _) => (s.clone(), None),
                _ => (String::new(), None),
            }
        }
        Wert::Text(s) => (s.clone(), None),
        Wert::Enum(s) => (s.clone(), None),
        Wert::Liste(l) => (
            l.iter()
                .map(|v| eigenschaftswert(v, e).0)
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
            None,
        ),
        w => (String::new(), w.zahl()),
    }
}

struct Saetze {
    eigenschaften: HashMap<u64, Vec<Modelleigenschaft>>,
    mengen: HashMap<u64, Vec<Modellmenge>>,
}

fn saetze(d: &StepDatei, e: &Einheiten) -> Saetze {
    let mut s = Saetze {
        eigenschaften: HashMap::new(),
        mengen: HashMap::new(),
    };
    let lesen = |def: &Instanz, vom_typ: bool| -> (Vec<Modelleigenschaft>, Vec<Modellmenge>) {
        let satz = def.attr(2).text().unwrap_or("").to_string();
        let mut eig = Vec::new();
        let mut men = Vec::new();
        match def.typ.as_str() {
            "IFCPROPERTYSET" => {
                for p in def.attr(4).liste() {
                    let Some(p) = d.get(p.referenz().unwrap_or(0)) else {
                        continue;
                    };
                    let name = p.attr(0).text().unwrap_or("").to_string();
                    let (wert, zahl) = match p.typ.as_str() {
                        "IFCPROPERTYSINGLEVALUE"
                        | "IFCPROPERTYENUMERATEDVALUE"
                        | "IFCPROPERTYLISTVALUE" => eigenschaftswert(p.attr(2), e),
                        "IFCPROPERTYBOUNDEDVALUE" => {
                            let (a, _) = eigenschaftswert(p.attr(3), e);
                            let (b, _) = eigenschaftswert(p.attr(2), e);
                            (format!("{a} bis {b}"), None)
                        }
                        _ => continue,
                    };
                    eig.push(Modelleigenschaft {
                        satz: satz.clone(),
                        name,
                        wert,
                        zahl,
                        vom_typ,
                    });
                }
            }
            "IFCELEMENTQUANTITY" => {
                for q in def.attr(5).liste() {
                    let Some(q) = d.get(q.referenz().unwrap_or(0)) else {
                        continue;
                    };
                    let (art, faktor) = match q.typ.as_str() {
                        "IFCQUANTITYLENGTH" => (Mengenart::Laenge, e.laenge),
                        "IFCQUANTITYAREA" => (Mengenart::Flaeche, e.flaeche),
                        "IFCQUANTITYVOLUME" => (Mengenart::Volumen, e.volumen),
                        "IFCQUANTITYCOUNT" => (Mengenart::Anzahl, 1.),
                        "IFCQUANTITYWEIGHT" => (Mengenart::Gewicht, 1.),
                        "IFCQUANTITYTIME" => (Mengenart::Zeit, 1.),
                        _ => continue,
                    };
                    let Some(w) = q.attr(3).zahl() else { continue };
                    men.push(Modellmenge {
                        satz: satz.clone(),
                        name: q.attr(0).text().unwrap_or("").to_string(),
                        art,
                        wert: w * faktor,
                    });
                }
            }
            _ => {}
        }
        (eig, men)
    };
    for r in d.alle("IFCRELDEFINESBYPROPERTIES") {
        let Some(def) = d.get(r.attr(5).referenz().unwrap_or(0)) else {
            continue;
        };
        let (eig, men) = lesen(def, false);
        for o in r.attr(4).referenzen() {
            s.eigenschaften
                .entry(o)
                .or_default()
                .extend(eig.iter().cloned());
            s.mengen.entry(o).or_default().extend(men.iter().cloned());
        }
    }
    for r in d.alle("IFCRELDEFINESBYTYPE") {
        let Some(typ) = d.get(r.attr(5).referenz().unwrap_or(0)) else {
            continue;
        };
        let mut eig = Vec::new();
        for ps in typ.attr(5).referenzen() {
            if let Some(def) = d.get(ps) {
                eig.extend(lesen(def, true).0);
            }
        }
        for o in r.attr(4).referenzen() {
            let liste = s.eigenschaften.entry(o).or_default();
            for e in &eig {
                // Am Element gesetzte Werte gehen dem Typ vor.
                if !liste.iter().any(|x| x.satz == e.satz && x.name == e.name) {
                    liste.push(e.clone());
                }
            }
        }
    }
    s
}

fn produkt(i: &Instanz) -> bool {
    if AUSGENOMMEN.contains(&i.typ.as_str())
        || i.typ.ends_with("TYPE")
        || i.typ.ends_with("STYLE")
        || i.typ.starts_with("IFCREL")
    {
        return false;
    }
    let gid = i.attr(0).text().is_some_and(|t| t.len() == 22);
    let platz = matches!(i.attr(5), Wert::Ref(_) | Wert::Null);
    let rep = matches!(i.attr(6), Wert::Ref(_) | Wert::Null);
    gid && platz
        && rep
        && !(matches!(i.attr(5), Wert::Null)
            && matches!(i.attr(6), Wert::Null)
            && i.attribute.len() < 8)
}

pub(super) fn geometrie_aus(teile: &[Teil], naeherung: bool) -> Option<Modellgeometrie> {
    if teile.is_empty() {
        return None;
    }
    let polys: Vec<geo::Polygon<f64>> =
        teile.iter().flat_map(|t| t.grundriss().0.clone()).collect();
    let g = vereinen(polys);
    let (mut unten, mut oben) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut vol = Some(0.);
    for t in teile {
        let (u, o) = t.hoehen();
        unten = unten.min(u);
        oben = oben.max(o);
        vol = vol.zip(t.volumen()).map(|(a, b)| a + b);
    }
    Some(Modellgeometrie {
        grundriss: planpolygone(&g),
        grundflaeche_m2: geometrie::rund(g.unsigned_area()),
        unten_m: geometrie::rund(unten),
        oben_m: geometrie::rund(oben),
        volumen_m3: vol.map(geometrie::rund),
        naeherung,
    })
}

/// Erstellt den Modellauszug einer gelesenen Datei.
pub fn auszug(d: &StepDatei) -> CoreResult<Modellauszug> {
    if !d.schema.starts_with("IFC") {
        return Err(CoreError::InvalidInput(format!(
            "Unbekanntes Schema „{}“: Erwartet wird IFC2X3, IFC4 oder IFC4X3.",
            d.schema
        )));
    }
    let (e, mut hinweise) = einheiten(d);
    let g = Geometrie { d, m: e.laenge };
    let saetze = saetze(d, &e);
    // Räumliche Struktur: Element → Behälter (Enthaltensein), Kind → Eltern (Aggregation).
    let mut behaelter: HashMap<u64, u64> = HashMap::new();
    for r in d.alle("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
        if let Some(s) = r.attr(5).referenz() {
            for x in r.attr(4).referenzen() {
                behaelter.insert(x, s);
            }
        }
    }
    let mut eltern: HashMap<u64, u64> = HashMap::new();
    for typ in ["IFCRELAGGREGATES", "IFCRELNESTS"] {
        for r in d.alle(typ) {
            if let Some(p) = r.attr(4).referenz() {
                for x in r.attr(5).referenzen() {
                    eltern.entry(x).or_insert(p);
                }
            }
        }
    }
    let gid = |id: u64| d.get(id).and_then(|i| i.attr(0).text()).map(str::to_string);
    let oben = |id: u64| behaelter.get(&id).or(eltern.get(&id)).copied();
    let vorfahre = |start: u64, typ: &str| -> Option<u64> {
        let mut x = start;
        for _ in 0..32 {
            x = oben(x)?;
            if d.get(x).is_some_and(|i| i.typ == typ) {
                return Some(x);
            }
        }
        None
    };
    let text = |w: &Wert| {
        w.text()
            .filter(|t| !t.trim().is_empty())
            .map(str::to_string)
    };
    let lage = |i: &Instanz| g.platzierung(i.attr(5));
    let struktur = |i: &Instanz, hoehe: Option<f64>| -> Strukturelement {
        let a = g.produkt(i.attr(6), &lage(i));
        Strukturelement {
            global_id: text(i.attr(0)).unwrap_or_default(),
            name: text(i.attr(2)).unwrap_or_default(),
            langname: text(i.attr(7)),
            teil_von: oben(i.id).and_then(gid),
            hoehe_m: hoehe,
            eigenschaften: saetze.eigenschaften.get(&i.id).cloned().unwrap_or_default(),
            mengen: saetze.mengen.get(&i.id).cloned().unwrap_or_default(),
            geometrie: geometrie_aus(&a.teile, a.naeherung),
        }
    };
    let grundstuecke: Vec<_> = d.alle("IFCSITE").map(|i| struktur(i, None)).collect();
    let gebaeude: Vec<_> = d.alle("IFCBUILDING").map(|i| struktur(i, None)).collect();
    let mut geschosse: Vec<_> = d
        .alle("IFCBUILDINGSTOREY")
        .map(|i| {
            // Höhe aus `Elevation`; ohne Angabe aus der Platzierung.
            let h = i
                .attr(9)
                .zahl()
                .map(|z| z * e.laenge)
                .unwrap_or_else(|| lage(i).o[2]);
            struktur(i, Some(geometrie::rund(h)))
        })
        .collect();
    geschosse.sort_by(|a, b| {
        a.hoehe_m
            .partial_cmp(&b.hoehe_m)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut elemente = Vec::new();
    let mut klassen: BTreeMap<String, usize> = BTreeMap::new();
    let mut offen: BTreeMap<String, usize> = BTreeMap::new();
    for i in d.instanzen.values() {
        if !(i.typ == "IFCSPACE" || (!RAEUMLICH.contains(&i.typ.as_str()) && produkt(i))) {
            continue;
        }
        let a = g.produkt(i.attr(6), &lage(i));
        *klassen.entry(i.typ.clone()).or_default() += 1;
        for u in &a.unbekannt {
            *offen.entry(u.clone()).or_default() += 1;
        }
        let vordefiniert = match i.typ.as_str() {
            "IFCSPACE" => i.attr(9).enum_wert().map(str::to_string),
            _ => i.attr(8).enum_wert().map(str::to_string),
        }
        .filter(|v| v != "NOTDEFINED" && v != "USERDEFINED");
        let teil_von = oben(i.id)
            .filter(|p| {
                d.get(*p).is_some_and(|x| {
                    !matches!(
                        x.typ.as_str(),
                        "IFCBUILDINGSTOREY" | "IFCBUILDING" | "IFCSITE"
                    )
                })
            })
            .and_then(gid);
        elemente.push(Modellelement {
            global_id: text(i.attr(0)).unwrap_or_default(),
            klasse: i.typ.clone(),
            name: text(i.attr(2)).unwrap_or_default(),
            langname: if i.typ == "IFCSPACE" {
                text(i.attr(7))
            } else {
                None
            },
            objekttyp: text(i.attr(4)),
            vordefiniert,
            geschoss: vorfahre(i.id, "IFCBUILDINGSTOREY").and_then(gid),
            gebaeude: vorfahre(i.id, "IFCBUILDING").and_then(gid),
            teil_von,
            eigenschaften: saetze.eigenschaften.get(&i.id).cloned().unwrap_or_default(),
            mengen: saetze.mengen.get(&i.id).cloned().unwrap_or_default(),
            geometrie: geometrie_aus(&a.teile, a.naeherung),
            geometrie_offen: a.unbekannt,
            step_id: i.id,
            teile: a.teile,
        });
    }
    // Doppelte GlobalIds machen Reimport und Nacharbeit mehrdeutig.
    let mut gesehen = std::collections::HashSet::new();
    let doppelt: Vec<&str> = elemente
        .iter()
        .map(|e| e.global_id.as_str())
        .chain(geschosse.iter().map(|g| g.global_id.as_str()))
        .filter(|g| !gesehen.insert(*g))
        .collect();
    if !doppelt.is_empty() {
        hinweise.push(format!(
            "{} doppelte GlobalId(s), z. B. {}: Zuordnung bei Reimport und Nacharbeit unsicher.",
            doppelt.len(),
            doppelt[0]
        ));
    }
    if geschosse.is_empty() {
        hinweise.push("Keine Geschosse (IfcBuildingStorey) im Modell.".into());
    }
    let raeume: Vec<&Modellelement> = elemente.iter().filter(|e| e.klasse == "IFCSPACE").collect();
    let bericht = Modellbericht {
        klassen: klassen
            .into_iter()
            .map(|(klasse, anzahl)| Klassenzahl {
                titel: klassentitel(&klasse),
                klasse,
                anzahl,
            })
            .collect(),
        elemente: elemente.len(),
        mit_geometrie: elemente.iter().filter(|e| e.geometrie.is_some()).count(),
        ohne_geometrie: elemente.iter().filter(|e| e.geometrie.is_none()).count(),
        naeherungen: elemente
            .iter()
            .filter(|e| e.geometrie.as_ref().is_some_and(|g| g.naeherung))
            .count(),
        raeume: raeume.len(),
        raeume_mit_flaechenmenge: raeume
            .iter()
            .filter(|r| r.mengen.iter().any(|m| m.art == Mengenart::Flaeche))
            .count(),
        offene_darstellungen: offen
            .into_iter()
            .map(|(klasse, anzahl)| Klassenzahl {
                titel: klasse.trim_start_matches("IFC").to_lowercase(),
                klasse,
                anzahl,
            })
            .collect(),
        hinweise,
    };
    Ok(Modellauszug {
        rechenstand: RECHENSTAND.into(),
        schema: d.schema.clone(),
        system: d.system.clone(),
        projekt: d
            .alle("IFCPROJECT")
            .next()
            .and_then(|p| text(p.attr(5)).or_else(|| text(p.attr(2)))),
        einheit: e.name,
        meter_je_einheit: e.laenge,
        georeferenz: georeferenz(d, e.laenge),
        grundstuecke,
        gebaeude,
        geschosse,
        elemente,
        bericht,
    })
}
