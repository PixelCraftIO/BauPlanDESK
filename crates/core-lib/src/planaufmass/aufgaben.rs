//! Aufmaß-Aufgaben: Welche Zielfelder der Gebäude und manuellen Abstandswände
//! sind noch offen, welche Bedeutung und Geometrie braucht eine Markierung
//! dafür, und gibt es in diesem Aufmaß bereits eine passende Markierung?
//! Reine Auskunft für die Oberfläche; Übernahme und Prüfung bleiben bei den
//! bestehenden Diensten (`planaufmass_gebaeude`, `planaufmass_abstand`).
use super::*;
use crate::{
    CoreServices, abstandsflaechen::AbstandsPlan, domain::ProjectId, plattform::ObjektArt,
    plattform::objekte::Gebaeude,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aufgabengeometrie {
    Flaeche,
    Hoehenpunkt,
    Strecke,
}
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Planaufgabe {
    /// `gebaeude` oder `abstand`.
    pub verbraucher: String,
    /// Gebäude-ID bzw. ID des Abstandsplans.
    pub objekt: Uuid,
    pub objekt_name: String,
    /// Anzeigegruppe, z. B. „Lagerhalle · OG“ oder „Südwand“.
    pub gruppe: String,
    pub ziel: String,
    pub titel: String,
    pub groesse: Planmessgroesse,
    pub bedeutung: Planbedeutung,
    pub geometrie: Aufgabengeometrie,
    pub gebaeude_id: Option<Uuid>,
    pub geschoss_id: Option<Uuid>,
    pub grundstueck_id: Option<Uuid>,
    /// Zielfeld trägt bereits einen (manuellen) Wert.
    pub wert_vorhanden: bool,
    /// Bereits verknüpfte Markierung (aus einem beliebigen Aufmaß).
    pub gebunden: Option<Uuid>,
    /// Passende, noch nicht verknüpfte Markierung dieses Aufmaßes.
    pub markierung: Option<Uuid>,
    pub hinweis: Option<String>,
}

fn gebaeudefeld(
    feld: &str,
    geschoss: bool,
) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "grundflaeche_m2" => (Gebaeudegrundflaeche, Flaeche, None),
        "flaeche_m2" if geschoss => (GeschossAussenkante, Flaeche, None),
        "nullpunkt_m_nhn" => (
            Nullpunkt,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie im System „m ü. NHN“"),
        ),
        "gelaende_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Maßgebendes Gelände; Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "traufe_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "first_m" => (
            First,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "fundamentsohle_m" => (
            Fundamentsohle,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "fussboden_m" => (
            Fussboden,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "decke_oberkante_m" => (
            Decke,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        _ => (
            Hoehenmass,
            Strecke,
            Some("Senkrechte Strecke im Schnitt (Höhendifferenz)"),
        ),
    }
}
fn wandfeld(feld: &str) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "gelaende_a_m" | "gelaende_b_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "abschluss_a_m" | "abschluss_b_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "dachhoehe_m" => (
            Dach,
            Strecke,
            Some("Senkrechte Strecke vom Wandabschluss bis zur Dachoberkante"),
        ),
        _ => (
            Dach,
            Strecke,
            Some("Strecke entlang der Dachhaut; Neigung zur Horizontalen"),
        ),
    }
}
fn passt(m: &Planmarkierung, geometrie: Aufgabengeometrie) -> bool {
    matches!(
        (geometrie, &m.geometrie),
        (
            Aufgabengeometrie::Flaeche,
            Plangeometrie::Polygon { .. }
                | Plangeometrie::Rechteck { .. }
                | Plangeometrie::Kreis { .. }
        ) | (
            Aufgabengeometrie::Hoehenpunkt,
            Plangeometrie::Hoehenpunkt { .. }
        ) | (Aufgabengeometrie::Strecke, Plangeometrie::Strecke { .. })
    )
}
fn gefuellt(v: &serde_json::Value, ziel: &str) -> bool {
    // Pfade sind `tabelle.id.feld` oder `geometrie.feld`.
    let t: Vec<&str> = ziel.split('.').collect();
    let feld = match t.as_slice() {
        [tabelle, id, feld] => v
            .get(tabelle)
            .and_then(|a| a.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|z| z.get("id").and_then(|i| i.as_str()) == Some(id))
            })
            .and_then(|z| z.get("geometrie").or(Some(z)))
            .and_then(|z| z.get(feld))
            .cloned(),
        [gruppe, feld] => v.get(gruppe).and_then(|g| g.get(feld)).cloned(),
        _ => None,
    };
    feld.is_some_and(|f| !f.is_null())
}

impl CoreServices {
    pub async fn planaufmass_aufgaben(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Vec<Planaufgabe>> {
        let mut aufgaben = Vec::new();
        let frei = |gebunden: Option<Uuid>, geometrie, filter: &dyn Fn(&Planmarkierung) -> bool| {
            if gebunden.is_some() {
                return None;
            }
            plan.markierungen
                .iter()
                .find(|m| passt(m, geometrie) && filter(m))
                .map(|m| m.id)
        };
        for f in self.fachobjekte(pid, ObjektArt::Gebaeude).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Gebäude-ID ungültig"))?;
            let g: Gebaeude =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            for z in gebaeudeziele(&g) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let geschoss = t[0] == "geschosse";
                let geschoss_id = if geschoss { t[1].parse().ok() } else { None };
                let (bedeutung, geometrie, hinweis) = gebaeudefeld(t[t.len() - 1], geschoss);
                let gebunden = g
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let markierung = frei(gebunden, geometrie, &|m| {
                    m.gebaeude_id == Some(id)
                        && m.bedeutung == bedeutung
                        && (!geschoss || m.geschoss_id == geschoss_id)
                });
                let gruppe = match geschoss_id
                    .and_then(|s| g.geschosse.iter().enumerate().find(|(_, x)| x.id == s))
                {
                    Some((i, s)) => format!(
                        "{} · {}",
                        g.bezeichnung,
                        if s.bezeichnung.trim().is_empty() {
                            format!("Geschoss {}", i + 1)
                        } else {
                            s.bezeichnung.clone()
                        }
                    ),
                    None => g.bezeichnung.clone(),
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "gebaeude".into(),
                    objekt: id,
                    objekt_name: g.bezeichnung.clone(),
                    gruppe,
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: Some(id),
                    geschoss_id,
                    grundstueck_id: None,
                    wert_vorhanden: gefuellt(&f.daten, &z.ziel),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Abstandsflaechen).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Abstandsplan-ID ungültig"))?;
            let p: AbstandsPlan =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            for z in abstandsziele(&p) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let Some(w) = p.waende.iter().find(|w| w.id.to_string() == t[1]) else {
                    continue;
                };
                let (bedeutung, geometrie, hinweis) = wandfeld(t[2]);
                let gebunden = p
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let markierung = frei(gebunden, geometrie, &|m| {
                    m.gebaeude_id == w.gebaeude_id && m.bedeutung == bedeutung
                });
                let wert = match t[2] {
                    "gelaende_a_m" => w.gelaende_a_m,
                    "gelaende_b_m" => w.gelaende_b_m,
                    "abschluss_a_m" => w.abschluss_a_m,
                    "abschluss_b_m" => w.abschluss_b_m,
                    "dachhoehe_m" => w.dachhoehe_m,
                    "dachneigung_grad" => w.dachneigung_grad,
                    _ => w.dachneigung_b_grad,
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "abstand".into(),
                    objekt: id,
                    objekt_name: p.bezeichnung.clone(),
                    gruppe: format!("{} · {}", p.bezeichnung, w.bezeichnung),
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: w.gebaeude_id,
                    geschoss_id: None,
                    grundstueck_id: w.grundstueck_id,
                    wert_vorhanden: wert.is_some(),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        Ok(aufgaben)
    }
}
