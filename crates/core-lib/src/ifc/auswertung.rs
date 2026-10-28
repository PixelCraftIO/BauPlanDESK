//! Auswertung eines Modells für den Bauantrag: Gebäudegrundfläche,
//! Brutto-Grundfläche und Brutto-Rauminhalt je Geschoss, Netto-Raumflächen
//! nach Nutzung und Gebäudehöhen. Gerechnet wird aus der Geometrie (mit
//! Nacharbeit), nicht aus den Mengen der Datei.
//!
//! GRZ und GFZ sind rechnerische Verhältnisse zur Bezugsfläche des
//! Baugrundstücks. Anrechnungen (Vollgeschoss, Nebenanlagen, Garagen nach
//! § 19 Abs. 4 BauNVO) bleiben bei den Fachmodulen.
use super::geometrie::{multipolygon, rund};
use super::uebernahme::{erdgeschoss, geschossname, traegt_aussenkante, umriss};
use super::{Modellauszug, Modellelement};
use crate::planaufmass::Planpolygon;
use geo::Area;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Geschossauswertung {
    pub global_id: String,
    pub name: String,
    /// Geschossebene über dem Projektnullpunkt (m).
    pub ebene_m: f64,
    /// Rechnerische Höhe für den Rauminhalt: bis zur nächsten Geschossebene,
    /// im obersten Geschoss bis zur Oberkante der Bauteile, im untersten ab
    /// der Unterkante der Bodenplatte.
    pub hoehe_m: f64,
    /// Liegt auf oder über der Ebene des Erdgeschosses.
    pub oberirdisch: bool,
    /// Brutto-Grundfläche (Außenkante von Wänden, Stützen und Decken).
    pub bgf_m2: f64,
    pub bri_m3: f64,
    /// Netto-Raumfläche (Summe der Raumgeometrie).
    pub nrf_m2: f64,
    /// Summe `NetFloorArea` der Räume, soweit eingetragen.
    pub nrf_menge_m2: Option<f64>,
    pub raeume: usize,
    pub aussenkante: Vec<Planpolygon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Nutzungsflaeche {
    /// Langname des Raums (Nutzung) oder „ohne Nutzung“.
    pub nutzung: String,
    pub raeume: usize,
    pub flaeche_m2: f64,
}

/// Höhen über der Ebene des Erdgeschosses (m), Näherung aus der Geometrie.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellhoehen {
    /// Ebene des Erdgeschosses über dem Projektnullpunkt (m).
    pub bezug_m: f64,
    /// Bezugshöhe in m über NHN, wenn das Modell georeferenziert ist.
    pub bezug_nhn_m: Option<f64>,
    /// Höchster Punkt der Dachbauteile.
    pub first_m: Option<f64>,
    /// Flachdach: Oberkante Dach; geneigtes Dach: tiefste Unterkante der Dachbauteile.
    pub traufe_m: Option<f64>,
    /// Höchster Punkt aller Bauteile (auch Schornsteine, Aufbauten).
    pub oberkante_m: Option<f64>,
    pub flachdach: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Bezugsflaeche {
    pub grundstueck_id: uuid::Uuid,
    pub bezeichnung: String,
    pub flaeche_m2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Modellauswertung {
    pub rechenstand: String,
    /// Gebäudegrundfläche: Projektion der Außenkanten aller oberirdischen Geschosse.
    pub grundflaeche_m2: f64,
    pub grundflaeche: Vec<Planpolygon>,
    pub bgf_m2: f64,
    pub bgf_oberirdisch_m2: f64,
    pub bri_m3: f64,
    pub nrf_m2: f64,
    pub geschosse: Vec<Geschossauswertung>,
    pub nutzungen: Vec<Nutzungsflaeche>,
    pub hoehen: Modellhoehen,
    pub grundstueck: Option<Bezugsflaeche>,
    /// Gebäudegrundfläche / Bezugsfläche (rechnerisch).
    pub grz: Option<f64>,
    /// BGF der oberirdischen Geschosse / Bezugsfläche (rechnerisch).
    pub gfz: Option<f64>,
    pub hinweise: Vec<String>,
}

fn flaeche(ps: &[Planpolygon]) -> f64 {
    multipolygon(ps).unsigned_area()
}

fn ist_dach(e: &Modellelement) -> bool {
    e.klasse == "IFCROOF"
        || (e.klasse.starts_with("IFCSLAB") && e.vordefiniert.as_deref() == Some("ROOF"))
}

fn zahl(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

fn im<'a>(a: &'a Modellauszug, gid: &'a str) -> impl Iterator<Item = &'a Modellelement> {
    a.elemente
        .iter()
        .filter(move |e| e.geschoss.as_deref() == Some(gid))
}

/// Wertet den Auszug aus; `grundstueck` ist die Bezugsfläche für GRZ/GFZ.
pub fn auswerten(a: &Modellauszug, grundstueck: Option<Bezugsflaeche>) -> Modellauswertung {
    let mut hinweise = Vec::new();
    let eg = erdgeschoss(a);
    let bezug = eg.and_then(|g| g.hoehe_m).unwrap_or(0.);
    let mut geschosse = Vec::new();
    for (i, g) in a.geschosse.iter().enumerate() {
        let ebene = g.hoehe_m.unwrap_or(0.);
        let kante = umriss(im(a, &g.global_id).filter(|e| traegt_aussenkante(e)));
        let bgf = flaeche(&kante);
        let geo = |e: &Modellelement| e.geometrie.clone();
        let oben = match a.geschosse.get(i + 1).and_then(|n| n.hoehe_m) {
            Some(n) => n,
            None => im(a, &g.global_id)
                .filter(|e| e.klasse != "IFCSPACE")
                .filter_map(geo)
                .map(|x| x.oben_m)
                .fold(f64::NAN, f64::max),
        };
        let unten = if i == 0 {
            im(a, &g.global_id)
                .filter(|e| e.klasse.starts_with("IFCSLAB") || e.klasse == "IFCFOOTING")
                .filter_map(geo)
                .map(|x| x.unten_m)
                .fold(ebene, f64::min)
        } else {
            ebene
        };
        let hoehe = if oben.is_finite() && oben > unten {
            oben - unten
        } else {
            0.
        };
        let raeume: Vec<&Modellelement> = im(a, &g.global_id)
            .filter(|e| e.klasse == "IFCSPACE")
            .collect();
        let nrf: f64 = raeume
            .iter()
            .filter_map(|e| e.geometrie.as_ref())
            .map(|x| x.grundflaeche_m2)
            .sum();
        let mengen: Vec<f64> = raeume
            .iter()
            .filter_map(|e| {
                e.mengen
                    .iter()
                    .find(|m| m.name == "NetFloorArea")
                    .map(|m| m.wert)
            })
            .collect();
        if bgf == 0. {
            hinweise.push(format!(
                "{}: keine Außenkante (keine Wände, Stützen oder Decken mit Geometrie).",
                geschossname(g)
            ));
        }
        geschosse.push(Geschossauswertung {
            global_id: g.global_id.clone(),
            name: geschossname(g),
            ebene_m: rund(ebene),
            hoehe_m: rund(hoehe),
            oberirdisch: ebene >= bezug - 1e-6,
            bgf_m2: rund(bgf),
            bri_m3: rund(bgf * hoehe),
            nrf_m2: rund(nrf),
            nrf_menge_m2: (!mengen.is_empty()).then(|| rund(mengen.iter().sum())),
            raeume: raeume.len(),
            aussenkante: kante,
        });
    }
    let oberirdisch: Vec<&str> = geschosse
        .iter()
        .filter(|g| g.oberirdisch)
        .map(|g| g.global_id.as_str())
        .collect();
    let grundflaeche = umriss(a.elemente.iter().filter(|e| {
        traegt_aussenkante(e)
            && e.geschoss
                .as_deref()
                .is_some_and(|s| oberirdisch.contains(&s))
    }));
    let gf = flaeche(&grundflaeche);

    let mut nutzungen: Vec<Nutzungsflaeche> = Vec::new();
    for e in a.elemente.iter().filter(|e| e.klasse == "IFCSPACE") {
        let n = e
            .langname
            .clone()
            .filter(|l| !l.trim().is_empty())
            .unwrap_or_else(|| "ohne Nutzung".into());
        let f = e.geometrie.as_ref().map_or(0., |g| g.grundflaeche_m2);
        match nutzungen.iter_mut().find(|x| x.nutzung == n) {
            Some(x) => {
                x.raeume += 1;
                x.flaeche_m2 += f;
            }
            None => nutzungen.push(Nutzungsflaeche {
                nutzung: n,
                raeume: 1,
                flaeche_m2: f,
            }),
        }
    }
    for n in &mut nutzungen {
        n.flaeche_m2 = rund(n.flaeche_m2);
    }
    nutzungen.sort_by(|x, y| x.nutzung.cmp(&y.nutzung));

    let dach: Vec<_> = a
        .elemente
        .iter()
        .filter(|e| ist_dach(e))
        .filter_map(|e| e.geometrie.as_ref())
        .collect();
    let first = dach.iter().map(|g| g.oben_m).reduce(f64::max);
    // Flach: kein Dachbauteil ist höher als 0,6 m (Dicke einer Dachplatte).
    let flach = !dach.is_empty() && dach.iter().all(|g| g.oben_m - g.unten_m <= 0.6);
    let traufe = if flach {
        first
    } else {
        dach.iter().map(|g| g.unten_m).reduce(f64::min)
    };
    let oberkante = a
        .elemente
        .iter()
        .filter(|e| e.klasse != "IFCSPACE")
        .filter_map(|e| e.geometrie.as_ref())
        .map(|g| g.oben_m)
        .reduce(f64::max);
    if dach.is_empty() {
        hinweise.push(
            "Keine Dachbauteile (IfcRoof oder IfcSlab ROOF): First- und Traufhöhe fehlen.".into(),
        );
    }
    let rel = |v: Option<f64>| v.map(|v| rund(v - bezug));
    let hoehen = Modellhoehen {
        bezug_m: rund(bezug),
        bezug_nhn_m: a
            .georeferenz
            .as_ref()
            .and_then(|g| g.hoehe)
            .map(|h| rund(h + bezug)),
        first_m: rel(first),
        traufe_m: rel(traufe),
        oberkante_m: rel(oberkante),
        flachdach: flach,
    };

    let bgf: f64 = geschosse.iter().map(|g| g.bgf_m2).sum();
    let bgf_ob: f64 = geschosse
        .iter()
        .filter(|g| g.oberirdisch)
        .map(|g| g.bgf_m2)
        .sum();
    let verhaeltnis = |z: f64| {
        grundstueck
            .as_ref()
            .filter(|g| g.flaeche_m2 > 0.)
            .map(|g| (z / g.flaeche_m2 * 1e4).round() / 1e4)
    };
    match &grundstueck {
        Some(g) => hinweise.push(format!(
            "GRZ und GFZ rechnerisch zur Bezugsfläche {} ({} m²). GFZ mit allen oberirdischen Geschossen; ob ein Geschoss Vollgeschoss ist und welche Flächen angerechnet werden, entscheiden die Fachmodule.",
            g.bezeichnung,
            zahl(g.flaeche_m2)
        )),
        None => hinweise.push(
            "Kein Baugrundstück mit Bezugsfläche: GRZ und GFZ werden nicht gerechnet.".into(),
        ),
    }
    if a.elemente.iter().any(|e| {
        e.geometrie
            .as_ref()
            .is_some_and(|g| g.naeherung && traegt_aussenkante(e))
    }) {
        hinweise.push(
            "Einige Wände oder Decken sind angenähert (boolesche Operation oder schräger Körper)."
                .into(),
        );
    }
    Modellauswertung {
        rechenstand: super::RECHENSTAND.into(),
        grundflaeche_m2: rund(gf),
        grundflaeche,
        bgf_m2: rund(bgf),
        bgf_oberirdisch_m2: rund(bgf_ob),
        bri_m3: rund(geschosse.iter().map(|g| g.bri_m3).sum()),
        nrf_m2: rund(geschosse.iter().map(|g| g.nrf_m2).sum()),
        grz: verhaeltnis(gf),
        gfz: verhaeltnis(bgf_ob),
        geschosse,
        nutzungen,
        hoehen,
        grundstueck,
        hinweise,
    }
}
