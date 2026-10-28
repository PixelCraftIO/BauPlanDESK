//! Abgleich gezeichneter Flächen mit bereits manuell erfassten Zeilen. Eine
//! Übernahme aus dem Aufmaß legt nie stillschweigend eine zweite Zeile für eine
//! Fläche an, die schon von Hand erfasst ist: Passt eine ungebundene Zeile
//! gleicher Art und gleichen Bezugs, entscheidet der Nutzer zwischen
//! „verknüpfen“ und „als neue Zeile anlegen“ (docs/planaufmass.md, Abgleich).
use super::*;

/// Entscheidung zu einer Markierung, für die [`Abgleichsfrage`]n gestellt wurden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Zeilenzuordnung {
    pub markierung_id: Uuid,
    /// Schlüssel der Zeile ([`Abgleichskandidat::zeile`]), die die gezeichnete
    /// Fläche übernimmt; leer legt eine neue Zeile an.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub zeile: Option<String>,
}

/// Manuell erfasste Zeile, die zu einer Markierung passt.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Abgleichskandidat {
    /// Zeilenkennung oder, für Altzeilen ohne Kennung, `#<Position>` in ihrer
    /// Liste. Gilt nur für den Entwurf, aus dem die Frage entstand.
    pub zeile: String,
    pub bezeichnung: String,
    pub flaeche_m2: f64,
}

/// Rückfrage vor einer Übernahme: die Markierung würde eine neue Zeile
/// anlegen, obwohl schon passende Handzeilen vorhanden sind.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Abgleichsfrage {
    pub markierung_id: Uuid,
    pub markierung: String,
    /// Ziel der Zeile, etwa „Flurstück Musterdorf 12 · Verkehrsfläche“.
    pub ziel: String,
    /// Gezeichnete Fläche; fehlt, wenn die Markierung noch nicht messbar ist.
    #[ts(optional)]
    pub flaeche_m2: Option<f64>,
    pub kandidaten: Vec<Abgleichskandidat>,
    /// Vorbelegung: die Handzeile mit der geringsten Flächendifferenz.
    #[ts(optional)]
    pub vorschlag: Option<String>,
}

pub(super) fn zeilenschluessel(id: Option<&str>, position: usize) -> String {
    id.map(str::to_string)
        .unwrap_or_else(|| format!("#{position}"))
}

/// Vorbelegung: kleinste Flächendifferenz, sonst die erste Handzeile.
pub(super) fn vorschlag(flaeche: Option<f64>, kandidaten: &[Abgleichskandidat]) -> Option<String> {
    let naechste = flaeche.and_then(|f| {
        kandidaten.iter().min_by(|a, b| {
            (a.flaeche_m2 - f)
                .abs()
                .total_cmp(&(b.flaeche_m2 - f).abs())
        })
    });
    naechste.or(kandidaten.first()).map(|k| k.zeile.clone())
}

/// Wahl für eine Markierung, zu der `kandidaten` (Positionen) passen:
/// `Ok(Some(i))` verknüpft Zeile `i`, `Ok(None)` legt eine neue Zeile an.
/// Ohne Entscheidung wird nicht geraten.
pub(super) fn wahl(
    zuordnung: &[Zeilenzuordnung],
    m: &Planmarkierung,
    kandidaten: &[(usize, String)],
) -> CoreResult<Option<usize>> {
    let Some(z) = zuordnung.iter().find(|z| z.markierung_id == m.id) else {
        if kandidaten.is_empty() {
            return Ok(None);
        }
        return Err(fehler(format!(
            "Markierung „{}“: Zu dieser Fläche ist bereits eine Zeile von Hand erfasst. Abgleich bestätigen (mit der Zeile verknüpfen oder als neue Zeile anlegen).",
            m.bezeichnung
        )));
    };
    match &z.zeile {
        None => Ok(None),
        Some(k) => kandidaten
            .iter()
            .find(|(_, s)| s == k)
            .map(|(i, _)| Some(*i))
            .ok_or_else(|| {
                fehler(format!(
                    "Markierung „{}“: Gewählte Zeile passt nicht (andere Art, bereits verknüpft oder entfernt). Abgleich erneut ausführen.",
                    m.bezeichnung
                ))
            }),
    }
}

/// Mögliche Doppelerfassung: eine gezeichnete (gebundene) Zeile und eine
/// Handzeile gleicher Art am selben Bezug mit fast gleicher Fläche. Nur ein
/// Hinweis, kein Fehler: zwei gleich große Stellplätze sind zulässig.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Doppelerfassung {
    /// `abzug` oder `bauteil`.
    pub bereich: String,
    /// Flurstück (Abzug) bzw. Schlüssel des Baugrundstücks (Bauteil).
    pub bezug: String,
    /// Kennung der gezeichneten Zeile.
    pub gebunden: String,
    /// Position der Handzeile in ihrer Liste.
    #[ts(type = "number")]
    pub hand: usize,
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct Toleranzen {
    pub flurstueck: FlurstueckToleranz,
    doppelerfassung: DoppelToleranz,
}
#[derive(Deserialize)]
pub(crate) struct FlurstueckToleranz {
    pub unauffaellig_m2: f64,
    pub unauffaellig_prozent: f64,
    pub fehler_prozent: f64,
}
#[derive(Deserialize)]
struct DoppelToleranz {
    m2: f64,
    prozent: f64,
}
/// Gemeinsame Schwellen mit der Oberfläche: `regelwerke/aufmass-toleranzen.json`.
pub(crate) fn toleranzen() -> &'static Toleranzen {
    static T: std::sync::OnceLock<Toleranzen> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../regelwerke/aufmass-toleranzen.json"
        ))
        .expect("Ungültige Aufmaß-Toleranzen")
    })
}

/// Fast gleich: höchstens 1 m² oder 2 % der gezeichneten Fläche Unterschied
/// (`doppelerfassung` in den Toleranzen).
pub fn fast_gleich(gezeichnet: f64, hand: f64) -> bool {
    let t = &toleranzen().doppelerfassung;
    (gezeichnet - hand).abs() <= (gezeichnet.abs() * t.prozent / 100.).max(t.m2)
}

/// Paare (gebundene Zeile, Handzeile) gleicher `art`; jede Handzeile höchstens einmal.
fn paare<T>(
    zeilen: &[T],
    gebunden: impl Fn(&T) -> bool,
    art: impl Fn(&T) -> Option<String>,
    flaeche: impl Fn(&T) -> f64,
) -> Vec<(usize, usize)> {
    let mut vergeben = std::collections::HashSet::new();
    let mut paare = Vec::new();
    for (g, x) in zeilen.iter().enumerate().filter(|(_, x)| gebunden(x)) {
        let Some(a) = art(x) else { continue };
        let treffer = zeilen.iter().enumerate().find(|(h, y)| {
            !gebunden(y)
                && !vergeben.contains(h)
                && art(y).as_deref() == Some(a.as_str())
                && fast_gleich(flaeche(x), flaeche(y))
        });
        if let Some((h, _)) = treffer {
            vergeben.insert(h);
            paare.push((g, h));
        }
    }
    paare
}

fn m2(v: f64) -> String {
    format!(
        "{} m²",
        ((v * 100.).round() / 100.).to_string().replace('.', ",")
    )
}

/// Doppelerfassungen der Abzugszeilen eines Flurstücks (`name` für den Text).
pub fn abzugsdubletten(
    flurstueck_id: &str,
    name: &str,
    zeilen: &[crate::baugrundstuecke::FlaechenAbzug],
) -> Vec<Doppelerfassung> {
    paare(
        zeilen,
        |x| x.planbindung.is_some() && x.id.is_some(),
        |x| Some(x.art.clone()),
        |x| x.flaeche_m2,
    )
    .into_iter()
    .map(|(g, h)| {
        let art = crate::baugrundstuecke::abzugsregeln()
            .get(&zeilen[g].art)
            .map(|r| r.bezeichnung.clone())
            .unwrap_or_else(|| zeilen[g].art.clone());
        Doppelerfassung {
            bereich: "abzug".into(),
            bezug: flurstueck_id.into(),
            gebunden: zeilen[g].id.clone().unwrap_or_default(),
            hand: h,
            text: format!(
                "Mögliche Doppelerfassung: Abzug „{art}“ auf Flurstück {name} ist gezeichnet ({}) und zusätzlich von Hand erfasst ({}). Beide Zeilen werden abgezogen; Handzeile entfernen, wenn es dieselbe Fläche ist.",
                m2(zeilen[g].flaeche_m2),
                m2(zeilen[h].flaeche_m2)
            ),
        }
    })
    .collect()
}

/// Doppelerfassungen der GRZ-Bauteile ohne Gebäude eines Baugrundstücks.
pub fn bauteildubletten(
    schluessel: &str,
    zeilen: &[crate::flaechen::Bauteil],
) -> Vec<Doppelerfassung> {
    paare(
        zeilen,
        |x| x.planbindung.is_some() && x.id.is_some(),
        |x| {
            if x.gebaeude_id.is_some() {
                return None;
            }
            Planbedeutung::fuer_bauteiltyp(x.typ).map(|b| format!("{b:?}"))
        },
        |x| x.flaeche_m2,
    )
    .into_iter()
    .map(|(g, h)| {
        let name = |i: usize| match zeilen[i].bezeichnung.trim() {
            "" => "ohne Bezeichnung".to_string(),
            b => format!("„{b}“"),
        };
        Doppelerfassung {
            bereich: "bauteil".into(),
            bezug: schluessel.into(),
            gebunden: zeilen[g].id.clone().unwrap_or_default(),
            hand: h,
            text: format!(
                "Mögliche Doppelerfassung: Bauteil {} ist gezeichnet ({}) und Bauteil {} mit gleicher Art zusätzlich von Hand erfasst ({}). Beide gehen in die GRZ ein; Handzeile entfernen, wenn es dieselbe Fläche ist.",
                name(g),
                m2(zeilen[g].flaeche_m2),
                name(h),
                m2(zeilen[h].flaeche_m2)
            ),
        }
    })
    .collect()
}

/// Eine Markierung versorgt höchstens eine Zeile (Schutz gegen Import oder
/// zusammengeführte Bearbeitungsstände).
pub(super) fn markierung_einmal<'a>(
    bindungen: impl Iterator<Item = &'a Planbindung>,
) -> CoreResult<()> {
    let mut gesehen = std::collections::HashSet::new();
    for b in bindungen {
        if !gesehen.insert(b.markierung_id) {
            return Err(fehler(
                "Eine Planmarkierung ist mit mehreren Zeilen verknüpft; doppelte Zeile entfernen oder Verknüpfung lösen",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod pruefungen {
    use super::*;
    use crate::baugrundstuecke::FlaechenAbzug;
    fn abzug(art: &str, flaeche_m2: f64, gebunden: bool) -> FlaechenAbzug {
        let m = Uuid::new_v4();
        FlaechenAbzug {
            id: gebunden.then(|| m.to_string()),
            art: art.into(),
            flaeche_m2,
            grundlage: String::new(),
            planbindung: gebunden.then(|| Planbindung {
                aufmass_id: Uuid::new_v4(),
                revision: 1,
                markierung_id: m,
                groesse: Planmessgroesse::Flaeche,
                ziel: format!("abzuege.{m}.flaeche_m2"),
            }),
        }
    }
    #[test]
    fn toleranz_ist_ein_quadratmeter_oder_zwei_prozent() {
        assert!(fast_gleich(40., 39.1));
        assert!(!fast_gleich(40., 38.9));
        assert!(fast_gleich(200., 196.));
        assert!(!fast_gleich(200., 195.9));
    }
    #[test]
    fn gezeichnete_und_handzeile_derselben_flaeche_werden_gemeldet() {
        let zeilen = [
            abzug("verkehrsflaeche", 40., true),
            abzug("private_gruenflaeche", 40., false),
            abzug("verkehrsflaeche", 39.6, false),
            abzug("verkehrsflaeche", 39.8, false),
        ];
        let d = abzugsdubletten("a", "Linden 10/1", &zeilen);
        assert_eq!(d.len(), 1, "jede gezeichnete Zeile höchstens ein Paar");
        assert_eq!(d[0].hand, 2);
        assert_eq!(d[0].gebunden, zeilen[0].id.clone().unwrap());
        assert!(d[0].text.contains("Linden 10/1") && d[0].text.contains("39,6 m²"));
        // Zwei Handzeilen oder zwei gezeichnete Zeilen sind keine Doppelerfassung.
        assert!(abzugsdubletten("a", "x", &zeilen[1..]).is_empty());
        let beide = [
            abzug("verkehrsflaeche", 40., true),
            abzug("verkehrsflaeche", 40., true),
        ];
        assert!(abzugsdubletten("a", "x", &beide).is_empty());
    }
    #[test]
    fn eine_markierung_versorgt_hoechstens_eine_zeile() {
        let a = abzug("verkehrsflaeche", 40., true);
        let b = a.planbindung.clone().unwrap();
        assert!(markierung_einmal([&b].into_iter()).is_ok());
        assert!(markierung_einmal([&b, &b].into_iter()).is_err());
    }
}
