//! Kleine fachliche Schreibgrenzen mit Drei-Wege-Abgleich unabhängiger Entwürfe.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    domain::{Aktion, Planungsrecht, Project, ProjectId, ProjectUpdate},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjektBereich {
    Stammdaten,
    Grundlagen,
    Festsetzungen,
    Grundstuecke,
    Projektdaten,
}
impl ProjektBereich {
    fn felder(self) -> &'static [&'static str] {
        match self {
            Self::Stammdaten => &["gemeinde", "standort", "vorhaben", "nutzung", "begruendung"],
            Self::Grundlagen => &[
                "bau_nvo",
                "rechtsgrundlage",
                "bebauungsplan_art",
                "ergaenzende_grundlage",
                "aussenbereich_vorhaben",
                "plan_bezeichnung",
                "plan_nummer",
                "plan_rechtskraft",
            ],
            Self::Festsetzungen => &["festgesetzte_werte", "festsetzungen"],
            Self::Grundstuecke => &["baugrundstuecke"],
            Self::Projektdaten => &[],
        }
    }
    fn auszug(self, p: &Project) -> CoreResult<Value> {
        let plan = serde_json::to_value(&p.planungsrecht).map_err(json_err)?;
        let mut map = serde_json::Map::new();
        for k in self.felder() {
            map.insert((*k).into(), plan[k].clone());
        }
        if self == Self::Grundlagen {
            map.insert(
                "bundesland".into(),
                serde_json::to_value(p.bundesland).map_err(json_err)?,
            );
        }
        if self == Self::Projektdaten {
            map.insert("name".into(), Value::String(p.name.clone()));
            map.insert(
                "grundstuecksflaeche_m2".into(),
                serde_json::to_value(p.grundstuecksflaeche_m2).map_err(json_err)?,
            );
        }
        Ok(Value::Object(map))
    }
}
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::InvalidInput(e.to_string())
}
impl CoreServices {
    pub async fn projekt_bereich_speichern(
        &self,
        basis: Project,
        bereich: ProjektBereich,
        a: ProjectUpdate,
        wer: &Benutzer,
    ) -> CoreResult<Project> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        if basis.id != a.id {
            return Err(CoreError::InvalidInput(
                "Entwurf gehört zu einem anderen Projekt".into(),
            ));
        }
        let mut ziel = basis.clone();
        ziel.name = a.name;
        ziel.bundesland = a.bundesland;
        ziel.grundstuecksflaeche_m2 = a.grundstuecksflaeche_m2;
        if let Some(p) = a.planungsrecht {
            ziel.planungsrecht = p;
        }
        let alt = bereich.auszug(&basis)?;
        let neu = bereich.auszug(&ziel)?;
        for _ in 0..5 {
            let mut aktuell = self.repos.projects.get(basis.id).await?;
            let stand = bereich.auszug(&aktuell)?;
            // Nur tatsächlich geänderte Felder kollidieren. Gleichlautende Änderungen sind wiederholbar.
            for (k, v) in neu.as_object().unwrap() {
                if alt[k] != *v && stand[k] != alt[k] && stand[k] != *v {
                    return Err(CoreError::VersionConflict {
                        expected: basis.version,
                        actual: aktuell.version,
                    });
                }
            }
            let mut plan = serde_json::to_value(&aktuell.planungsrecht).map_err(json_err)?;
            for k in bereich.felder() {
                if alt[k] != neu[k] {
                    plan[k] = neu[k].clone();
                }
            }
            aktuell.planungsrecht = serde_json::from_value(plan).map_err(json_err)?;
            if bereich == ProjektBereich::Grundlagen && alt["bundesland"] != neu["bundesland"] {
                aktuell.bundesland = ziel.bundesland;
            }
            if bereich == ProjektBereich::Projektdaten {
                if alt["name"] != neu["name"] {
                    aktuell.name = ziel.name.clone();
                }
                if alt["grundstuecksflaeche_m2"] != neu["grundstuecksflaeche_m2"] {
                    aktuell.grundstuecksflaeche_m2 = ziel.grundstuecksflaeche_m2;
                }
            }
            // Nur das verantwortete Fachobjekt validieren. Andere Nachweise dürfen veraltet sein.
            if bereich == ProjektBereich::Projektdaten {
                if aktuell.name.trim().is_empty()
                    || aktuell.name.len() > 500
                    || aktuell
                        .grundstuecksflaeche_m2
                        .is_some_and(|f| !f.is_finite() || f <= 0.)
                {
                    return Err(CoreError::InvalidInput(
                        "Projektname und positive, endliche Fläche erforderlich".into(),
                    ));
                }
            } else {
                let mut teil = serde_json::to_value(Planungsrecht::default()).map_err(json_err)?;
                for k in bereich.felder() {
                    teil[k] = neu[k].clone();
                }
                let teil: Planungsrecht = serde_json::from_value(teil).map_err(json_err)?;
                if bereich == ProjektBereich::Grundstuecke {
                    if let Some(plan) = &teil.baugrundstuecke {
                        plan.pruefe(&aktuell.planungsrecht, aktuell.version)?;
                    }
                } else {
                    teil.validate()?;
                }
            }
            if bereich.auszug(&aktuell)? == stand {
                return Ok(aktuell);
            }
            let version = aktuell.version;
            aktuell.updated_at = chrono::Utc::now();
            let audit = Self::audit(
                wer,
                Aktion::ProjektGeaendert,
                Some(aktuell.id),
                None,
                format!("Fachbereich {bereich:?} geändert"),
            );
            match self
                .repos
                .commit
                .projekt(aktuell, Some(version), audit)
                .await
            {
                Err(CoreError::VersionConflict { .. }) => continue,
                result => return result,
            }
        }
        let p = self.repos.projects.get(basis.id).await?;
        Err(CoreError::VersionConflict {
            expected: basis.version,
            actual: p.version,
        })
    }
    pub async fn grundstueckskontext(
        &self,
        projekt: ProjectId,
    ) -> CoreResult<Vec<crate::grundstueckskontext::GrundstueckKontext>> {
        crate::grundstueckskontext::ermitteln(&self.repos.projects.get(projekt).await?)
    }
}
