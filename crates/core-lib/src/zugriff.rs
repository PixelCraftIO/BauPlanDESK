//! Projektmitgliedschaften ergänzen die globale Rolle; Identitäten werden vom Zugang geprüft.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, BenutzerId, Recht, Rolle},
    commit::Fachstand,
    domain::{Aktion, Project, ProjectId},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ProjektRecht {
    Kein,
    Lesen,
    Bearbeiten,
    Leitung,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ProjektMitglied {
    pub benutzer_id: BenutzerId,
    pub recht: ProjektRecht,
}
impl CoreServices {
    pub async fn projekt_zugriff(
        &self,
        p: ProjectId,
        wer: &Benutzer,
        min: ProjektRecht,
    ) -> CoreResult<()> {
        let aktuell = self.repos.benutzer.get(wer.id).await?;
        if !aktuell.aktiv {
            return Err(CoreError::Forbidden("Benutzerkonto ist deaktiviert".into()));
        }
        if aktuell.rolle == Rolle::Administrator {
            return Ok(());
        }
        let f = self
            .repos
            .commit
            .fachstaende(p, "mitgliedschaften")
            .await?
            .into_iter()
            .find(|f| f.objekt == wer.id.to_string())
            .ok_or_else(|| CoreError::Forbidden("Keine Projektmitgliedschaft".into()))?;
        let m: ProjektMitglied =
            serde_json::from_value(f.daten).map_err(|e| CoreError::Database(e.to_string()))?;
        if m.recht < min {
            return Err(CoreError::Forbidden(
                "Projektberechtigung reicht nicht aus".into(),
            ));
        }
        Ok(())
    }
    pub async fn projekte_fuer_benutzer(&self, wer: &Benutzer) -> CoreResult<Vec<Project>> {
        let mut result = Vec::new();
        for p in self.list_projects().await? {
            match self.projekt_zugriff(p.id, wer, ProjektRecht::Lesen).await {
                Ok(()) => result.push(p),
                Err(CoreError::Forbidden(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(result)
    }
    pub async fn projektmitglieder(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Vec<Fachstand>> {
        self.projekt_zugriff(p, wer, ProjektRecht::Leitung).await?;
        self.repos.commit.fachstaende(p, "mitgliedschaften").await
    }
    pub async fn projektmitglied_setzen(
        &self,
        p: ProjectId,
        m: ProjektMitglied,
        basis: i64,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.pruefe(wer, Recht::ProjektAnlegen).await?;
        self.projekt_zugriff(p, wer, ProjektRecht::Leitung).await?;
        self.repos.projects.get(p).await?;
        let b = self.repos.benutzer.get(m.benutzer_id).await?;
        if !b.aktiv {
            return Err(CoreError::InvalidInput(
                "Benutzerkonto ist deaktiviert".into(),
            ));
        }
        let objekt = m.benutzer_id.to_string();
        let daten = serde_json::to_value(m).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        self.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p,
                    modul: "mitgliedschaften".into(),
                    objekt,
                    schema_version: 1,
                    revision: basis,
                    daten,
                },
                basis,
                Self::audit(
                    wer,
                    Aktion::BenutzerGeaendert,
                    Some(p),
                    None,
                    format!("Projektberechtigung für {} geändert", b.anzeigename),
                ),
            )
            .await
    }
}
