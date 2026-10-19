//! Geländehöhen aus einem CAD-Lageplan: Höhenkoten als Text und Punkte mit
//! Höhe. Der Vorschlag ersetzt keine Prüfung, ob die Koten das maßgebliche
//! Gelände an der Wand beschreiben.
use super::{hoehen::zahl, *};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CadHoehe {
    /// Lage in Metern.
    pub x: f64,
    pub y: f64,
    /// Höhe wie in der Zeichnung angegeben (m).
    pub hoehe_m: f64,
    pub ebene: String,
    /// „Höhenkote“ (Text) oder „Höhenpunkt“ (Punkt mit z).
    pub art: String,
    pub quelle: Dokumentbezug,
}
/// Liest eine Höhenkote wie „101,35“, „+101.35 m“ oder „GOK 101,35 NHN“.
/// Zahlen ohne Dezimalstellen gelten nicht als Kote (Hausnummern, Maße).
pub(crate) fn hoehenkote(t: &str) -> Option<f64> {
    let mut s = t.trim().to_lowercase().replace(['\u{a0}', ' '], "");
    for vorne in ["gok", "fok", "ok", "ug", "h=", "h:", "+", "±"] {
        if let Some(r) = s.strip_prefix(vorne) {
            s = r.to_string();
        }
    }
    for hinten in ["mü.nhn", "mü.nn", "ünhn", "ünn", "nhn", "nn", "m"] {
        if let Some(r) = s.strip_suffix(hinten) {
            s = r.to_string();
        }
    }
    let (ganz, dez) = s.split_once([',', '.'])?;
    let ziffern = ganz.strip_prefix('-').unwrap_or(ganz);
    if !(1..=4).contains(&ziffern.len())
        || !(1..=3).contains(&dez.len())
        || !ziffern.chars().all(|c| c.is_ascii_digit())
        || !dez.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    format!("{ganz}.{dez}").parse().ok()
}
/// Geländepunkte entlang der Wand aus CAD-Höhen, die höchstens `max_abstand_m`
/// neben der Wandlinie liegen. Nahe beieinanderliegende Stationen werden auf
/// die wandnächste Höhe reduziert.
pub fn gelaende_vorschlag(
    w: &AbstandsWand,
    hoehen: &[CadHoehe],
    max_abstand_m: f64,
) -> CoreResult<Vec<Hoehenpunkt>> {
    if !max_abstand_m.is_finite() || !(0. ..=10.).contains(&max_abstand_m) {
        return Err(err("Suchabstand zwischen 0 und 10 m angeben"));
    }
    if hoehen.len() > 20_000 {
        return Err(err("Höchstens 20.000 CAD-Höhen"));
    }
    let (dx, dy) = (w.bis.x - w.von.x, w.bis.y - w.von.y);
    let l = dx.hypot(dy);
    if l < 0.001 {
        return Err(err("Wand ohne Länge"));
    }
    let mut treffer: Vec<(f64, f64, &CadHoehe)> = hoehen
        .iter()
        .filter(|h| h.x.is_finite() && h.y.is_finite() && h.hoehe_m.is_finite())
        .filter_map(|h| {
            let (px, py) = (h.x - w.von.x, h.y - w.von.y);
            let station = (px * dx + py * dy) / l;
            let abstand = (px * dy - py * dx).abs() / l;
            (abstand <= max_abstand_m && station >= -max_abstand_m && station <= l + max_abstand_m)
                .then_some((station.clamp(0., l), abstand, h))
        })
        .collect();
    treffer.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut punkte: Vec<(f64, f64, &CadHoehe)> = vec![];
    for t in treffer {
        match punkte.last_mut() {
            Some(p) if t.0 - p.0 < 0.05 => {
                if t.1 < p.1 {
                    *p = t;
                }
            }
            _ => punkte.push(t),
        }
    }
    Ok(punkte
        .into_iter()
        .map(|(station, abstand, h)| Hoehenpunkt {
            station_m: (station * 1000.).round() / 1000.,
            hoehe_m: h.hoehe_m,
            quelle: format!(
                "{} auf Ebene {}, {} m neben der Wand (CAD-Quelle Version {})",
                h.art,
                h.ebene,
                zahl(abstand),
                h.quelle.version
            ),
        })
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn koten_erkennen() {
        for (t, h) in [
            ("101,35", Some(101.35)),
            ("+101.35 m", Some(101.35)),
            ("GOK 99,8 NHN", Some(99.8)),
            ("-2,10", Some(-2.1)),
            ("12", None),
            ("1:500", None),
            ("Haus 3.5a", None),
        ] {
            assert_eq!(hoehenkote(t), h, "{t}");
        }
    }
}
