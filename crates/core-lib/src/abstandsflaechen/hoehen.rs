//! Wandhöhe aus Gelände- und Oberkantenprofil, getrennt je Wandteil.
//! Zwischen den Profilpunkten wird linear interpoliert.
use super::*;

/// Toleranz für Stationen und Höhen (m).
pub(super) const TOL: f64 = 0.001;
/// Abweichung vom gleichmäßigen Verlauf, ab der der Mittelansatz einen Hinweis erhält.
const UNGLEICHMAESSIG_M: f64 = 0.25;

/// Profilstelle eines Wandteils. Bei Sprüngen erscheint eine Station zweimal
/// (Wert vor und nach der Stelle).
#[derive(Debug, Clone)]
pub(super) struct Stelle {
    pub s: f64,
    pub g: f64,
    pub o: f64,
}
impl Stelle {
    pub fn h(&self) -> f64 {
        self.o - self.g
    }
}
pub(super) struct Teil {
    pub von: f64,
    pub bis: f64,
    pub stellen: Vec<Stelle>,
    /// Maßgebliche Wandhöhe ohne Dachanteil.
    pub wandhoehe: f64,
}
pub(super) fn zahl(v: f64) -> String {
    format!("{v:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
fn sortiert(p: &[Hoehenpunkt]) -> Vec<(f64, f64)> {
    let mut v: Vec<_> = p.iter().map(|h| (h.station_m, h.hoehe_m)).collect();
    // Stabil: bei gleicher Station bleibt die erfasste Reihenfolge (Sprung).
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    v
}
/// Wert bei `s`; `rechts` wählt bei einem Sprung den Wert nach der Stelle.
fn wert(p: &[(f64, f64)], s: f64, rechts: bool) -> Option<f64> {
    let gleich: Vec<_> = p.iter().filter(|q| (q.0 - s).abs() <= TOL).collect();
    if let (Some(a), Some(b)) = (gleich.first(), gleich.last()) {
        return Some(if rechts { b.1 } else { a.1 });
    }
    p.windows(2)
        .find(|w| w[0].0 < s && s < w[1].0)
        .map(|w| w[0].1 + (w[1].1 - w[0].1) * (s - w[0].0) / (w[1].0 - w[0].0))
}
/// Neue Punktliste oder – in älteren Plänen – die Werte an beiden Wandenden.
fn profil(neu: &[Hoehenpunkt], a: Option<f64>, b: Option<f64>, l: f64) -> Option<Vec<(f64, f64)>> {
    if neu.is_empty() {
        a.zip(b).map(|(a, b)| vec![(0., a), (l, b)])
    } else {
        Some(sortiert(neu))
    }
}
/// Wandteile mit maßgeblicher Wandhöhe; `None`, wenn das Profil unvollständig ist
/// (Prüfpunkte stehen dann in `e.offen`).
pub(super) fn auswerten(
    w: &AbstandsWand,
    l: f64,
    ansatz: Wandhoehenansatz,
    e: &mut WandErgebnis,
) -> Option<Vec<Teil>> {
    let (Some(g), Some(o)) = (
        profil(&w.gelaende, w.gelaende_a_m, w.gelaende_b_m, l),
        profil(&w.oberkante, w.abschluss_a_m, w.abschluss_b_m, l),
    ) else {
        e.offen.push(
            "Gelände und Wandoberkante mindestens an beiden Wandenden eingeben (Höhenprofil)"
                .into(),
        );
        return None;
    };
    let mut vollstaendig = true;
    for (name, p) in [("Geländeprofil", &g), ("Oberkantenprofil", &o)] {
        if p.iter().any(|q| q.0 < -TOL || q.0 > l + TOL) {
            e.offen.push(format!(
                "{name}: Station außerhalb der Wandlänge (0 bis {} m)",
                zahl(l)
            ));
            vollstaendig = false;
        } else if p.first().is_none_or(|q| q.0 > TOL) || p.last().is_none_or(|q| q.0 < l - TOL) {
            e.offen.push(format!(
                "{name} deckt die Wand nicht vom Anfang (0 m) bis zum Ende ({} m) ab",
                zahl(l)
            ));
            vollstaendig = false;
        }
    }
    if !vollstaendig {
        return None;
    }
    // Wandteilgrenzen: Höhensprünge der Oberkante und ausdrückliche Grenzen.
    let mut grenzen: Vec<f64> = o
        .windows(2)
        .filter(|p| (p[1].0 - p[0].0).abs() <= TOL && (p[1].1 - p[0].1).abs() > TOL)
        .map(|p| p[0].0)
        .chain(w.wandteilgrenzen.iter().copied())
        .filter(|s| *s > TOL && *s < l - TOL)
        .collect();
    grenzen.sort_by(f64::total_cmp);
    grenzen.dedup_by(|a, b| (*a - *b).abs() <= TOL);
    let kanten: Vec<f64> = std::iter::once(0.)
        .chain(grenzen)
        .chain(std::iter::once(l))
        .collect();
    let stelle = |s: f64, rechts: bool| -> Option<Stelle> {
        Some(Stelle {
            s,
            g: wert(&g, s, rechts)?,
            o: wert(&o, s, rechts)?,
        })
    };
    let mut teile = vec![];
    for (n, k) in kanten.windows(2).enumerate() {
        let (a, b) = (k[0], k[1]);
        let mut innen: Vec<f64> = g
            .iter()
            .chain(o.iter())
            .map(|q| q.0)
            .filter(|s| *s > a + TOL && *s < b - TOL)
            .collect();
        innen.sort_by(f64::total_cmp);
        innen.dedup_by(|x, y| (*x - *y).abs() <= TOL);
        let mut stellen = vec![stelle(a, true)?];
        for s in innen {
            let (li, re) = (stelle(s, false)?, stelle(s, true)?);
            let sprung = (li.g - re.g).abs() > TOL || (li.o - re.o).abs() > TOL;
            stellen.push(li);
            if sprung {
                stellen.push(re);
            }
        }
        stellen.push(stelle(b, false)?);
        if let Some(s) = stellen.iter().find(|s| s.h() < -TOL) {
            e.offen.push(format!(
                "Wandoberkante liegt bei Station {} m unter dem Gelände",
                zahl(s.s)
            ));
            return None;
        }
        let h: Vec<f64> = stellen.iter().map(Stelle::h).collect();
        let (ha, hb) = (h[0], h[h.len() - 1]);
        let maximum = h.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let bereich = format!("Wandteil {} ({}–{} m)", n + 1, zahl(a), zahl(b));
        let wandhoehe = match ansatz {
            Wandhoehenansatz::Maximal => {
                e.herleitung.push(format!(
                    "{bereich}: größte Wandhöhe aus {} Profilstellen = {} m (konservativer Maximalansatz)",
                    h.len(),
                    zahl(maximum)
                ));
                maximum
            }
            Wandhoehenansatz::Punktweise => {
                e.herleitung.push(format!(
                    "{bereich}: Wandhöhe an jeder Stelle zwischen {} und {} m; Tiefe je Stelle",
                    zahl(h.iter().copied().fold(f64::INFINITY, f64::min)),
                    zahl(maximum)
                ));
                maximum
            }
            Wandhoehenansatz::MittelKanten => {
                let m = (ha + hb) / 2.;
                e.herleitung.push(format!(
                    "{bereich}: im Mittel gemessene Wandhöhe aus den Kanten ({} m + {} m) / 2 = {} m",
                    zahl(ha),
                    zahl(hb),
                    zahl(m)
                ));
                // Liegt das Profil zwischen den Kanten deutlich außerhalb des
                // gleichmäßigen Verlaufs, bildet der Kantenmittelwert es nicht ab.
                let abweichung = stellen
                    .iter()
                    .map(|s| (s.h() - (ha + (hb - ha) * (s.s - a) / (b - a).max(TOL))).abs())
                    .fold(0., f64::max);
                if abweichung > UNGLEICHMAESSIG_M {
                    e.herleitung.push(format!(
                        "Hinweis {bereich}: Die Wandhöhe zwischen den Kanten weicht um bis zu {} m vom gleichmäßigen Verlauf ab (größte Wandhöhe {} m). Wandteilung oder Ansatz fachlich prüfen.",
                        zahl(abweichung),
                        zahl(maximum)
                    ));
                }
                m
            }
            Wandhoehenansatz::MittelProfil => {
                let flaeche: f64 = stellen
                    .windows(2)
                    .map(|p| (p[1].s - p[0].s) * (p[0].h() + p[1].h()) / 2.)
                    .sum();
                let m = flaeche / (b - a);
                e.herleitung.push(format!(
                    "{bereich}: Wandfläche {} m² / Länge {} m = mittlere Wandhöhe {} m",
                    zahl(flaeche),
                    zahl(b - a),
                    zahl(m)
                ));
                m
            }
        };
        teile.push(Teil {
            von: a,
            bis: b,
            stellen,
            wandhoehe,
        });
    }
    Some(teile)
}
/// Tiefenverlauf (Station, Tiefe) über die ganze Wand. Bei punktweisem Ansatz
/// kommt dort ein Knick hinzu, wo die Mindesttiefe die Rechentiefe ablöst.
pub(super) fn tiefen(
    teile: &[Teil],
    dach: f64,
    faktor: f64,
    min: f64,
    ansatz: Wandhoehenansatz,
) -> Vec<(f64, f64)> {
    let t = |h: f64| (h * faktor).max(min);
    let mut v = vec![];
    for teil in teile {
        if ansatz == Wandhoehenansatz::Punktweise {
            for (i, s) in teil.stellen.iter().enumerate() {
                if let Some(p) = i.checked_sub(1).map(|j| &teil.stellen[j])
                    && faktor > 0.
                    && s.s - p.s > TOL
                {
                    let (h1, h2, k) = (p.h() + dach, s.h() + dach, min / faktor);
                    if (h1 - k) * (h2 - k) < 0. {
                        v.push((p.s + (s.s - p.s) * (k - h1) / (h2 - h1), min));
                    }
                }
                v.push((s.s, t(s.h() + dach)));
            }
        } else {
            let tt = t(teil.wandhoehe + dach);
            v.push((teil.von, tt));
            v.push((teil.bis, tt));
        }
    }
    v
}
/// Profil- und Wandteilzeilen für Bericht und Abwicklung.
pub(super) fn ergebnis(
    teile: &[Teil],
    dach: Option<f64>,
    regel: Option<(f64, f64)>,
    ansatz: Wandhoehenansatz,
    e: &mut WandErgebnis,
) {
    let t = |h: f64| regel.map(|(f, m)| (h * f).max(m));
    for (n, teil) in teile.iter().enumerate() {
        let h = dach.map(|d| teil.wandhoehe + d);
        let mut groesste: Option<f64> = None;
        for s in &teil.stellen {
            let tiefe = if ansatz == Wandhoehenansatz::Punktweise {
                dach.and_then(|d| t(s.h() + d))
            } else {
                h.and_then(t)
            };
            groesste = match (groesste, tiefe) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            e.profil.push(Profilzeile {
                station_m: s.s,
                gelaende_m: s.g,
                oberkante_m: s.o,
                wandhoehe_m: s.h(),
                tiefe_m: tiefe,
                wandteil: n + 1,
            });
        }
        e.wandteile.push(Wandteilergebnis {
            von_m: teil.von,
            bis_m: teil.bis,
            h_anfang_m: teil.stellen[0].h(),
            h_ende_m: teil.stellen[teil.stellen.len() - 1].h(),
            wandhoehe_m: teil.wandhoehe,
            h_m: h,
            tiefe_m: groesste,
        });
    }
}
