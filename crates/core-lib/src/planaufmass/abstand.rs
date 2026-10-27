use super::*;
use crate::{CoreServices, abstandsflaechen::AbstandsPlan, domain::ProjectId};

fn zielgroesse(feld: &str) -> Option<Planmessgroesse> {
    match feld {
        "gelaende_a_m" | "gelaende_b_m" | "abschluss_a_m" | "abschluss_b_m" | "dachhoehe_m" => {
            Some(Planmessgroesse::Hoehe)
        }
        "dachneigung_grad" | "dachneigung_b_grad" => Some(Planmessgroesse::Neigung),
        _ => None,
    }
}
pub fn abstandsziele(plan: &AbstandsPlan) -> Vec<Planziel> {
    plan.waende
        .iter()
        .filter(|w| w.umriss.is_none())
        .flat_map(|w| {
            [
                ("gelaende_a_m", "Gelände am Wandanfang"),
                ("gelaende_b_m", "Gelände am Wandende"),
                ("abschluss_a_m", "Wandabschluss am Anfang"),
                ("abschluss_b_m", "Wandabschluss am Ende"),
                ("dachhoehe_m", "Dachhöhe über Wandabschluss"),
                ("dachneigung_grad", "Dachneigung"),
                ("dachneigung_b_grad", "Dachneigung zweite Seite"),
            ]
            .into_iter()
            .map(move |(feld, titel)| Planziel {
                ziel: format!("waende.{}.{feld}", w.id),
                titel: format!("{} · {titel}", w.bezeichnung),
                groesse: zielgroesse(feld).unwrap(),
            })
        })
        .collect()
}
impl CoreServices {
    pub async fn planaufmass_abstand(
        &self,
        pid: ProjectId,
        mut plan: AbstandsPlan,
    ) -> CoreResult<AbstandsPlan> {
        self.planwerte_abstand(pid, &mut plan).await?;
        plan.umfang_geprueft = false;
        plan.recht.geprueft = false;
        Ok(plan)
    }
    pub(crate) async fn planwerte_abstand(
        &self,
        pid: ProjectId,
        plan: &mut AbstandsPlan,
    ) -> CoreResult<()> {
        if plan.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Wandverknüpfungen"));
        }
        let mut seen = std::collections::HashSet::new();
        // Herkunft je Wand aus allen ihren Bindungen, nicht nur der letzten.
        let mut herkunft: std::collections::BTreeMap<Uuid, Vec<String>> = Default::default();
        for b in plan.planbindungen.clone() {
            if !seen.insert(b.ziel.clone()) {
                return Err(fehler("Wandfeld mehrfach verknüpft"));
            }
            let t: Vec<_> = b.ziel.split('.').collect();
            if t.len() != 3 || t[0] != "waende" || zielgroesse(t[2]) != Some(b.groesse) {
                return Err(fehler("Ungültiges Wandfeld / Messgröße"));
            }
            let (a, m, wert) = self.planbindung_aufloesen(pid, &b).await?;
            let w = plan
                .waende
                .iter_mut()
                .find(|w| w.id.to_string() == t[1])
                .ok_or_else(|| fehler("Zielwand fehlt"))?;
            if w.umriss.is_some() {
                return Err(fehler(
                    "Umrisswände erhalten ihre Eckhöhen vom Gebäudeumriss",
                ));
            }
            if m.gebaeude_id != w.gebaeude_id {
                return Err(fehler("Höhenquelle gehört zu einem anderen Gebäude"));
            }
            if let Some(grund) = m.grundstueck_id {
                let zid = w
                    .grundstueck_id
                    .ok_or_else(|| fehler("Wandgrundstück fehlt"))?;
                if zid != grund {
                    let projekt = self.get_project(pid).await?;
                    let kontexte = crate::grundstueckskontext::ermitteln(&projekt)?;
                    let g = kontexte
                        .iter()
                        .find(|g| g.id == grund)
                        .ok_or_else(|| fehler("Quellgrundstück fehlt"))?;
                    let ziel = plan
                        .grundstuecke
                        .iter()
                        .find(|g| g.id == zid)
                        .ok_or_else(|| fehler("Wandgrundstück fehlt"))?;
                    if ziel.flurstueck_ids.is_empty()
                        || !ziel
                            .flurstueck_ids
                            .iter()
                            .all(|id| g.flurstueck_ids.contains(id))
                    {
                        return Err(fehler(
                            "Quellgrundstück und Wandgrundstück besitzen keine übereinstimmende Flurstückszuordnung",
                        ));
                    }
                }
            }
            let absolut = matches!(
                t[2],
                "gelaende_a_m" | "gelaende_b_m" | "abschluss_a_m" | "abschluss_b_m"
            );
            if absolut {
                if !matches!(m.geometrie, Plangeometrie::Hoehenpunkt { .. })
                    || a.hoehensystem(&m) != Some(plan.hoehenbezug.as_str())
                {
                    return Err(fehler(
                        "Wandhöhe benötigt einen absoluten Höhenpunkt im selben Höhensystem",
                    ));
                }
                if (t[2].starts_with("gelaende") && !w.gelaende.is_empty())
                    || (t[2].starts_with("abschluss") && !w.oberkante.is_empty())
                {
                    return Err(fehler(
                        "Vorhandenes Wandprofil zuerst ausdrücklich entfernen; Eckwerte dürfen es nicht verdeckt übersteuern",
                    ));
                }
            } else if !matches!(m.geometrie, Plangeometrie::Strecke { .. }) {
                return Err(fehler("Dachhöhe / Neigung benötigt eine gemessene Strecke"));
            }
            match t[2] {
                "gelaende_a_m" => w.gelaende_a_m = Some(wert),
                "gelaende_b_m" => w.gelaende_b_m = Some(wert),
                "abschluss_a_m" => w.abschluss_a_m = Some(wert),
                "abschluss_b_m" => w.abschluss_b_m = Some(wert),
                "dachhoehe_m" => w.dachhoehe_m = Some(wert),
                "dachneigung_grad" => w.dachneigung_grad = Some(wert),
                "dachneigung_b_grad" => w.dachneigung_b_grad = Some(wert),
                _ => unreachable!(),
            }
            let feld = match t[2] {
                "gelaende_a_m" => "Gelände A",
                "gelaende_b_m" => "Gelände B",
                "abschluss_a_m" => "Wandabschluss A",
                "abschluss_b_m" => "Wandabschluss B",
                "dachhoehe_m" => "Dachhöhe",
                "dachneigung_grad" => "Dachneigung",
                _ => "Dachneigung B",
            };
            herkunft.entry(w.id).or_default().push(match &a.dokument {
                Some(d) => format!(
                    "{feld}: Aufmaß „{}“ R{} · Markierung „{}“ · PDF V{} · Seite {}",
                    a.bezeichnung, b.revision, m.bezeichnung, d.version, a.seite
                ),
                None => format!(
                    "{feld}: Zeichnung „{}“ R{} · Markierung „{}“ · eigene Zeichnung in Metern",
                    a.bezeichnung, b.revision, m.bezeichnung
                ),
            });
            if let Some(d) = a.dokument
                && !plan.quellen.contains(&d)
            {
                plan.quellen.push(d);
            }
        }
        for w in &mut plan.waende {
            if let Some(h) = herkunft.remove(&w.id) {
                w.hoehenquelle = format!("{} · {RECHENSTAND}", h.join("; "));
            }
        }
        Ok(())
    }
    pub async fn abstands_planquellen_pruefen(
        &self,
        pid: ProjectId,
        plan: &AbstandsPlan,
    ) -> CoreResult<()> {
        let mut neu = plan.clone();
        self.planwerte_abstand(pid, &mut neu).await?;
        if neu != *plan {
            return Err(fehler(
                "Wandwerte weichen vom Aufmaß ab. Quellen erneut übernehmen und speichern.",
            ));
        }
        Ok(())
    }
}
