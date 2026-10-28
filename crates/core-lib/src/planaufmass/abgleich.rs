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
