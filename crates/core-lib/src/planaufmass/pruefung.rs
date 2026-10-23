//! Additiver Prüfvertrag: fehlerhafte Konturen verdecken keine gültigen Werte.
use super::*;
use crate::{CoreServices, domain::ProjectId};

#[derive(Debug, Serialize, ts_rs::TS)]
pub struct Markierungspruefung {
    pub id: Uuid,
    pub wert: Option<Planmesswert>,
    pub fehler: Option<String>,
}
#[derive(Debug, Serialize, ts_rs::TS)]
pub struct Planpruefung {
    pub markierungen: Vec<Markierungspruefung>,
}
impl CoreServices {
    /// Gemeinsame Grundlagen werden strikt geprüft; Konturfehler sind einzeln
    /// zugeordnet. Der bestehende Speichervertrag bleibt unverändert streng.
    pub async fn planaufmass_diagnose(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Planpruefung> {
        let (sicht, quellen) = self.lesesitzung(pid).await?;
        let mut basis = plan.clone();
        basis.markierungen.clear();
        basis.messen()?;
        let kontext = sicht.planaufmass_kontext(pid, plan).await?;
        if plan.markierungen.len() > 2000 {
            return Err(fehler("Zu viele Zeichenobjekte"));
        }
        let mut markierungen = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for m in &plan.markierungen {
            let mut einzeln = basis.clone();
            einzeln.markierungen = plan
                .markierungen
                .iter()
                .filter(|x| x.id == m.id || Some(x.id) == m.hoehenzone_von)
                .cloned()
                .collect();
            let result = if !ids.insert(m.id) {
                Err(fehler("Markierungs-ID doppelt"))
            } else {
                einzeln.messen().and_then(|werte| {
                    for m in &einzeln.markierungen {
                        kontext.zuordnung(m)?;
                    }
                    Ok(werte)
                })
            };
            markierungen.push(match result {
                Ok(werte) => Markierungspruefung {
                    id: m.id,
                    wert: werte.into_iter().find(|w| w.id == m.id),
                    fehler: None,
                },
                Err(e) => Markierungspruefung {
                    id: m.id,
                    wert: None,
                    fehler: Some(e.to_string()),
                },
            });
        }
        quellen.unveraendert().await?;
        Ok(Planpruefung { markierungen })
    }
}
