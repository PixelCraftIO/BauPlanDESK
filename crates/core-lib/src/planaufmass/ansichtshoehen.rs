//! Höhenabgriff in der Ansicht einer Abstandsfläche: Gelände, Traufe, Attika,
//! First, Dachneigung, Wandteilgrenzen und Gauben, die in derselben Ansicht
//! (Ausschnitt) über der Linie „Abstandsfläche“ einer Wand gezeichnet sind.
//!
//! Die waagerechte Lage ergibt die Stelle an der Wand (Anteil der Linie von
//! Wandanfang bis Wandende mal Wandbreite), die Höhe kommt von der
//! Bezugslinie der Ansicht. Linien werden auf die Wandbreite gekürzt; so
//! gehört jeder Abschnitt einer durchlaufenden Traufe genau einer Wand.
use super::*;
use crate::abstandsflaechen::{AbstandsWand, Hoehenpunkt};

/// Gleiche Station bzw. Höhe (m).
const TOL: f64 = 0.001;

/// Waagerechte Lage der Linienenden in der Ansicht (m).
pub(crate) fn linienbereich(a: &Planaufmass, linie: &Planmarkierung) -> Option<(f64, f64)> {
    let Plangeometrie::Strecke { von, bis } = &linie.geometrie else {
        return None;
    };
    let r = a.hoehenrahmen(linie.ausschnitt_id).ok()?;
    Some((r.waagerecht_m(von)?, r.waagerecht_m(bis)?))
}

/// Profilpunkt mit Reihenfolge an derselben Station: Ende eines Abschnitts (0)
/// vor Einzelpunkt (1) vor Anfang eines Abschnitts (2). So bilden eine
/// endende und eine beginnende Linie an einer Stelle einen Höhensprung.
struct Punkt {
    s: f64,
    ord: u8,
    h: f64,
    quelle: String,
}
fn profil(mut p: Vec<Punkt>) -> Vec<Hoehenpunkt> {
    p.sort_by(|a, b| a.s.total_cmp(&b.s).then(a.ord.cmp(&b.ord)));
    let mut v: Vec<Hoehenpunkt> = Vec::new();
    for q in p {
        if v.last()
            .is_some_and(|l| (l.station_m - q.s).abs() <= TOL && (l.hoehe_m - q.h).abs() <= TOL)
        {
            continue;
        }
        v.push(Hoehenpunkt {
            station_m: q.s,
            hoehe_m: q.h,
            quelle: q.quelle,
        });
    }
    v
}
fn zahl(v: f64) -> String {
    format!("{v:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// Setzt die Höhen der Wand `w` aus der Ansicht `a`, in der `linie` ihr
/// Wandbezug ist. `gebunden` nennt die einzeln verknüpften Wandfelder; sie
/// haben Vorrang. Offene Punkte landen in `w.ansicht_hinweise`, die
/// Herleitung im Rückgabewert.
pub(crate) fn uebernehmen(
    w: &mut AbstandsWand,
    a: &Planaufmass,
    linie: &Planmarkierung,
    hoehenbezug: &str,
    gebunden: &[&str],
) -> CoreResult<Vec<String>> {
    let ansicht = a
        .ausschnitte
        .iter()
        .find(|x| Some(x.id) == linie.ausschnitt_id)
        .map(|x| x.bezeichnung.clone())
        .unwrap_or_default();
    let mut herleitung = vec![];
    let r = a.hoehenrahmen(linie.ausschnitt_id)?;
    let Plangeometrie::Strecke { von, bis } = &linie.geometrie else {
        return Err(fehler("Wandbezug ist eine Strecke"));
    };
    let (Some(s0), Some(s1)) = (r.waagerecht_m(von), r.waagerecht_m(bis)) else {
        w.ansicht_hinweise.push(format!(
            "Ansicht „{ansicht}“: Maßstab fehlt; Höhen nicht übernommen"
        ));
        return Ok(herleitung);
    };
    let l = (w.bis.x - w.von.x).hypot(w.bis.y - w.von.y);
    if (s1 - s0).abs() < 0.01 || l < 0.001 {
        w.ansicht_hinweise.push(format!(
            "Ansicht „{ansicht}“: Linie „{}“ ohne waagerechte Breite; Höhen nicht übernommen",
            linie.bezeichnung
        ));
        return Ok(herleitung);
    }
    // Zeichenungenauigkeit an den Gebäudekanten wie beim Längenabgleich.
    let tol = &toleranzen().abstandsflaeche_breite;
    let tol_t = tol.m.max(tol.prozent / 100. * (s1 - s0).abs()) / (s1 - s0).abs();
    let anteil = |p: &Planpunkt| r.waagerecht_m(p).map(|s| (s - s0) / (s1 - s0));
    let fang = |t: f64| {
        if t.abs() <= tol_t {
            0.
        } else if (t - 1.).abs() <= tol_t {
            1.
        } else {
            t
        }
    };
    let hoehen_ok = r.bezug.is_some() && a.hoehensystem(linie) == Some(hoehenbezug);
    let quelle = |m: &Planmarkierung| format!("Ansicht „{ansicht}“ · {}", m.bezeichnung);

    let (mut gelaende, mut oberkante) = (vec![], vec![]);
    let (mut first, mut neigungen, mut grenzen) = (None::<f64>, vec![], vec![]);
    let (mut gauben, mut abgrabungen, mut hoehen_noetig) = (None::<f64>, vec![], false);
    for m in &a.markierungen {
        if m.id == linie.id
            || m.ausschnitt_id != linie.ausschnitt_id
            || m.gebaeude_id.is_some_and(|g| Some(g) != w.gebaeude_id)
        {
            continue;
        }
        use Planbedeutung::*;
        if !matches!(
            m.bedeutung,
            Gelaende | Abgrabung | Traufe | Attika | First | Dach | Wandteilgrenze | Gaube
        ) {
            continue;
        }
        // Einzelpunkt oder auf die Wand gekürzter Abschnitt (Anteile, Höhen).
        let abschnitt: Option<(f64, f64, Option<f64>, Option<f64>)> = match &m.geometrie {
            Plangeometrie::Hoehenpunkt { punkt } => {
                let Some(t) = anteil(punkt) else { continue };
                if !(-tol_t..=1. + tol_t).contains(&t) {
                    continue;
                }
                let t = fang(t).clamp(0., 1.);
                let h = r.hoehe_m(punkt);
                Some((t, t, h, h))
            }
            Plangeometrie::Strecke { von, bis } => {
                let (Some(ta), Some(tb)) = (anteil(von), anteil(bis)) else {
                    continue;
                };
                let (ha, hb) = (r.hoehe_m(von), r.hoehe_m(bis));
                let ((ta, ha), (tb, hb)) = if ta <= tb {
                    ((ta, ha), (tb, hb))
                } else {
                    ((tb, hb), (ta, ha))
                };
                let (lo, hi) = (fang(ta.max(0.)), fang(tb.min(1.)));
                if m.bedeutung == Wandteilgrenze {
                    // Senkrechte Linie: ihre Mitte ist die Grenze.
                    let t = (ta + tb) / 2.;
                    if t > tol_t && t < 1. - tol_t {
                        grenzen.push(t * l);
                    }
                    continue;
                }
                // Nur an der Kante berührende Linien gehören zur Nachbarwand.
                if hi - lo <= 1e-9 {
                    continue;
                }
                let bei = |t: f64| {
                    ha.zip(hb).map(|(ha, hb)| {
                        if tb - ta <= 1e-12 {
                            ha
                        } else {
                            let t = t.clamp(ta, tb);
                            ha + (hb - ha) * (t - ta) / (tb - ta)
                        }
                    })
                };
                if m.bedeutung == Dach {
                    let v = [bis[0] - von[0], bis[1] - von[1]];
                    let dx = (v[0] * r.waagerecht[0] + v[1] * r.waagerecht[1]).abs();
                    let dy = (v[0] * r.aufwaerts[0] + v[1] * r.aufwaerts[1]).abs();
                    neigungen.push((
                        (lo + hi) / 2.,
                        dy.atan2(dx).to_degrees(),
                        m.bezeichnung.clone(),
                    ));
                    continue;
                }
                if m.bedeutung == Gaube {
                    *gauben.get_or_insert(0.) += (hi - lo) * l;
                    continue;
                }
                Some((lo, hi, bei(lo), bei(hi)))
            }
            _ => continue,
        };
        let Some((lo, hi, hlo, hhi)) = abschnitt else {
            continue;
        };
        match m.bedeutung {
            Gelaende | Traufe | Attika | First | Abgrabung => hoehen_noetig = true,
            _ => {}
        }
        let (Some(hlo), Some(hhi)) = (hlo, hhi) else {
            continue;
        };
        if !hoehen_ok {
            continue;
        }
        let punkte = |ziel: &mut Vec<Punkt>| {
            if hi - lo <= 1e-9 {
                ziel.push(Punkt {
                    s: lo * l,
                    ord: 1,
                    h: hlo,
                    quelle: quelle(m),
                });
            } else {
                ziel.push(Punkt {
                    s: lo * l,
                    ord: 2,
                    h: hlo,
                    quelle: quelle(m),
                });
                ziel.push(Punkt {
                    s: hi * l,
                    ord: 0,
                    h: hhi,
                    quelle: quelle(m),
                });
            }
        };
        match m.bedeutung {
            Gelaende => punkte(&mut gelaende),
            Traufe | Attika => punkte(&mut oberkante),
            First => first = Some(first.map_or(hlo.max(hhi), |f: f64| f.max(hlo).max(hhi))),
            Abgrabung => abgrabungen.push(format!(
                "Abgrabung „{}“ bei {} m ({} m) nicht maßgebend",
                m.bezeichnung,
                zahl(lo * l),
                zahl(hlo)
            )),
            _ => {}
        }
    }
    if hoehen_noetig && !hoehen_ok {
        w.ansicht_hinweise.push(match a.hoehensystem(linie) {
            None => format!("Ansicht „{ansicht}“: Bezugslinie fehlt; Höhen nicht übernommen"),
            Some(s) => format!(
                "Ansicht „{ansicht}“ misst Höhen im System „{s}“, der Abstandsplan in „{hoehenbezug}“; Höhen nicht übernommen"
            ),
        });
    }
    let setzen = |feld: &'static str, w: &mut AbstandsWand, ja: bool| {
        if !ja {
            return false;
        }
        let einzeln: &[&str] = match feld {
            "gelaende" => &["gelaende_a_m", "gelaende_b_m"],
            "oberkante" => &["abschluss_a_m", "abschluss_b_m"],
            f => &[f][..],
        };
        if let Some(f) = einzeln.iter().find(|f| gebunden.contains(f)) {
            w.ansicht_hinweise.push(format!(
                "{}: „{f}“ ist einzeln verknüpft; Wert aus der Ansicht „{ansicht}“ nicht übernommen",
                w.bezeichnung
            ));
            return false;
        }
        w.ansicht_felder.push(feld.into());
        true
    };
    let gelaende = profil(gelaende);
    let oberkante = profil(oberkante);
    let mut teile = vec![];
    if setzen("gelaende", w, !gelaende.is_empty()) {
        teile.push(format!("Gelände {} Punkte", gelaende.len()));
        w.gelaende = gelaende;
    }
    let traufe = oberkante.iter().map(|p| p.hoehe_m).reduce(f64::max);
    if setzen("oberkante", w, !oberkante.is_empty()) {
        teile.push(format!("Wandoberkante {} Punkte", oberkante.len()));
        w.oberkante = oberkante;
    }
    if !grenzen.is_empty() {
        grenzen.sort_by(f64::total_cmp);
        grenzen.dedup_by(|a, b| (*a - *b).abs() <= TOL);
        if setzen("wandteilgrenzen", w, true) {
            teile.push(format!(
                "Wandteilgrenzen bei {} m",
                grenzen
                    .iter()
                    .map(|s| zahl(*s))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
            w.wandteilgrenzen = grenzen;
        }
    }
    if let Some(f) = first {
        match traufe {
            Some(t) if f > t + TOL => {
                if setzen("dachhoehe_m", w, true) {
                    teile.push(format!(
                        "Dachhöhe {} m (First {} m − Wandoberkante {} m)",
                        zahl(f - t),
                        zahl(f),
                        zahl(t)
                    ));
                    w.dachhoehe_m = Some(f - t);
                }
            }
            _ => w.ansicht_hinweise.push(format!(
                "Ansicht „{ansicht}“: First ohne tiefer liegende Traufe oder Attika über der Wand; Dachhöhe nicht ermittelt"
            )),
        }
    }
    neigungen.sort_by(|a, b| a.0.total_cmp(&b.0));
    if neigungen.len() > 2 {
        w.ansicht_hinweise.push(format!(
            "Ansicht „{ansicht}“: mehr als zwei Dachneigungen über der Wand; nur die ersten beiden übernommen"
        ));
    }
    for (feld, n) in ["dachneigung_grad", "dachneigung_b_grad"]
        .into_iter()
        .zip(&neigungen)
    {
        if setzen(feld, w, true) {
            teile.push(format!("Dachneigung {}° („{}“)", zahl(n.1), n.2));
            if feld == "dachneigung_grad" {
                w.dachneigung_grad = Some(n.1);
            } else {
                w.dachneigung_b_grad = Some(n.1);
            }
        }
    }
    if let Some(g) = gauben
        && setzen("gauben_laenge_m", w, true)
    {
        teile.push(format!("Gauben {} m", zahl(g)));
        w.gauben_laenge_m = Some(g);
    }
    if !teile.is_empty() || !abgrabungen.is_empty() {
        teile.extend(abgrabungen);
        herleitung.push(format!(
            "Höhen aus Ansicht „{ansicht}“: {}",
            teile.join(", ")
        ));
    }
    Ok(herleitung)
}

/// Felder, die bisher aus der Ansicht kamen und es nicht mehr tun, werden
/// geleert, damit kein veralteter Abgriff stehen bleibt.
pub(crate) fn veraltete_leeren(w: &mut AbstandsWand, vorher: &[String]) {
    for f in vorher.iter().filter(|f| !w.ansicht_felder.contains(f)) {
        match f.as_str() {
            "gelaende" => w.gelaende.clear(),
            "oberkante" => w.oberkante.clear(),
            "wandteilgrenzen" => w.wandteilgrenzen.clear(),
            "dachhoehe_m" => w.dachhoehe_m = None,
            "dachneigung_grad" => w.dachneigung_grad = None,
            "dachneigung_b_grad" => w.dachneigung_b_grad = None,
            "gauben_laenge_m" => w.gauben_laenge_m = None,
            _ => {}
        }
    }
}
