//! Quellenzustand aller Aufmaßverknüpfungen eines Projekts: dieselben Prüfungen,
//! die beim Speichern greifen, als Auskunft beim Öffnen einer Maske. So wird
//! ein veraltetes Aufmaß oder eine gelöschte Markierung sichtbar, bevor ein
//! Speichern daran scheitert.
use super::*;
use crate::{
    CoreServices,
    berechnungen::{BerechnungsArt, FachPlan},
    domain::ProjectId,
    plattform::{
        ObjektArt,
        objekte::{Gebaeude, Raumbuch},
    },
};

/// Ein Verbraucher, dessen Aufmaßwerte nicht mehr dem Stand der Quelle entsprechen.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Planquellenhinweis {
    /// Bereich für den Sprung: `flurstuecke`, `grundstueck`, `gebaeude`,
    /// `raumbuch`, `abstand` oder das Modul einer Fachberechnung.
    pub bereich: String,
    #[ts(optional)]
    pub objekt: Option<String>,
    pub titel: String,
    pub text: String,
}

fn text(e: CoreError) -> String {
    match e {
        CoreError::InvalidInput(t) => t,
        e => e.to_string(),
    }
}

impl CoreServices {
    /// Alle gespeicherten Verknüpfungen prüfen; Ergebnis leer = alles aktuell.
    pub async fn planquellen_zustand(&self, pid: ProjectId) -> CoreResult<Vec<Planquellenhinweis>> {
        let projekt = self.get_project(pid).await?;
        let mut aus = Vec::new();
        let mut melde = |bereich: &str, objekt: Option<String>, titel: String, e: CoreError| {
            aus.push(Planquellenhinweis {
                bereich: bereich.into(),
                objekt,
                titel,
                text: text(e),
            })
        };
        let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
        if let Err(e) = self.flurstueck_planquellen_pruefen(pid, &infos).await {
            melde("flurstuecke", None, "Grundstücksinformationen".into(), e);
        }
        if let Some(e) = crate::grundstuecksinformationen::ermittlung(&projekt.planungsrecht)?
            && let Err(e) = self.grundstueck_planquellen_pruefen(pid, &e).await
        {
            melde("grundstueck", None, "Baugrundstück".into(), e);
        }
        for f in self.fachobjekte(pid, ObjektArt::Gebaeude).await? {
            let (Ok(id), Ok(g)) = (
                f.objekt.parse::<Uuid>(),
                serde_json::from_value::<Gebaeude>(f.daten),
            ) else {
                continue;
            };
            if g.planbindungen.is_empty() {
                continue;
            }
            if let Err(e) = self.gebaeude_planquellen_pruefen(pid, id, &g).await {
                melde(
                    "gebaeude",
                    Some(f.objekt),
                    format!("Gebäude {}", g.bezeichnung),
                    e,
                );
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Raumbuch).await? {
            let Ok(r) = serde_json::from_value::<Raumbuch>(f.daten) else {
                continue;
            };
            if r.planbindungen.is_empty() {
                continue;
            }
            if let Err(e) = self.raumbuch_planquellen_pruefen(pid, &r).await {
                melde(
                    "raumbuch",
                    Some(f.objekt),
                    format!("Raumbuch {}", r.bezeichnung),
                    e,
                );
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Abstandsflaechen).await? {
            let Ok(a) = serde_json::from_value::<crate::abstandsflaechen::AbstandsPlan>(f.daten)
            else {
                continue;
            };
            if a.planbindungen.is_empty() {
                continue;
            }
            if let Err(e) = self.abstands_planquellen_pruefen(pid, &a).await {
                melde(
                    "abstand",
                    Some(f.objekt),
                    format!("Abstandsflächen {}", a.bezeichnung),
                    e,
                );
            }
        }
        for art in BerechnungsArt::ALLE {
            for f in self.fachobjekte(pid, art.objekt()).await? {
                let Ok(plan) = serde_json::from_value::<FachPlan>(f.daten) else {
                    continue;
                };
                if plan.planbindungen.is_empty() {
                    continue;
                }
                let titel = format!("{} {}", art.titel(), plan.bezeichnung);
                let mut neu = plan.clone();
                match self.planwerte_einsetzen(pid, &mut neu).await {
                    Err(e) => melde(art.objekt().modul(), Some(f.objekt), titel, e),
                    Ok(()) if neu.eingaben != plan.eingaben => melde(
                        art.objekt().modul(),
                        Some(f.objekt),
                        titel,
                        fehler(
                            "Werte weichen vom Aufmaß ab. „Aktuellen Aufmaßstand übernehmen“ und speichern.",
                        ),
                    ),
                    Ok(()) => {}
                }
            }
        }
        Ok(aus)
    }
}
