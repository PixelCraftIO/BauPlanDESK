//! Einzelbearbeitung erhält ungeprüfte Nachbareinheiten unverändert.
use crate::{
    CoreError, CoreResult, CoreServices,
    baugrundstuecke::{BaugrundstueckBerechnung, BaugrundstuecksPlan, BaugrundstuecksPruefung},
    benutzer::{Benutzer, Recht},
    domain::{Aktion, Project, ProjectId},
};
impl CoreServices {
    pub async fn baugrundstueck_gebaeude_einbinden(
        &self,
        pid: ProjectId,
        plan: &BaugrundstuecksPlan,
        schluessel: &str,
    ) -> CoreResult<super::einbindung::GebaeudeEinbindung> {
        let p = self.get_project(pid).await?;
        crate::grundstuecksinformationen::pruefe_quellen(&p.planungsrecht, plan)?;
        self.gebaeude_einbinden_fuer(pid, plan, Some(schluessel))
            .await
    }
    pub async fn baugrundstueck_einzeln_pruefen(
        &self,
        pid: ProjectId,
        plan: &BaugrundstuecksPlan,
        schluessel: &str,
    ) -> CoreResult<BaugrundstuecksPruefung> {
        let p = self.get_project(pid).await?;
        crate::grundstuecksinformationen::pruefe_quellen(&p.planungsrecht, plan)?;
        self.gebaeude_einbinden_fuer(pid, plan, Some(schluessel))
            .await?
            .plan
            .pruefe_einheit(&p.planungsrecht, p.version, schluessel)
    }
    /// Schreibt nur eine geprüfte Berechnung; andere Eingaben, Identitäten und
    /// ihre Aktualität werden nicht neu bestätigt. Veränderte Flurstücksquellen
    /// benötigen weiterhin die vollständige Grundstücksermittlung.
    pub async fn baugrundstueck_einzeln_speichern(
        &self,
        pid: ProjectId,
        version: i64,
        mut b: BaugrundstueckBerechnung,
        wer: &Benutzer,
    ) -> CoreResult<Project> {
        self.berechtigung_pruefen(wer, Recht::ProjektBearbeiten)
            .await?;
        self.projekt_zugriff(pid, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        let mut p = self.get_project(pid).await?;
        if !crate::grundstuecksinformationen::quellen_aktuell(&p.planungsrecht)? {
            return Err(CoreError::InvalidInput("Grundstücksinformationen geändert. Baugrundstücksermittlung zuerst prüfen und speichern.".into()));
        }
        if p.version != version {
            return Err(CoreError::VersionConflict {
                expected: version,
                actual: p.version,
            });
        }
        let mut plan =
            crate::grundstuecksinformationen::plan(&p.planungsrecht)?.ok_or_else(|| {
                CoreError::InvalidInput("Baugrundstücksermittlung zuerst speichern".into())
            })?;
        crate::grundstuecksinformationen::pruefe_quellen(&p.planungsrecht, &plan)?;
        let gruppen = plan.gruppen(p.planungsrecht.mit_bebauungsplan())?;
        let gruppe = gruppen
            .iter()
            .find(|g| g.schluessel == b.schluessel)
            .ok_or_else(|| CoreError::InvalidInput("Baugrundstück besteht nicht mehr".into()))?;
        b.flurstueck_ids = gruppe.flurstueck_ids.clone();
        let s = b.schluessel.clone();
        let ziel = plan
            .berechnungen
            .iter_mut()
            .find(|x| x.schluessel == s)
            .ok_or_else(|| CoreError::InvalidInput("Berechnung fehlt".into()))?;
        *ziel = b;
        let geprueft = self
            .gebaeude_einbinden_fuer(pid, &plan, Some(&s))
            .await?
            .plan;
        geprueft.pruefe_einheit(&p.planungsrecht, p.version, &s)?;
        let b = geprueft
            .berechnungen
            .into_iter()
            .find(|b| b.schluessel == s)
            .ok_or_else(|| CoreError::InvalidInput("Berechnung fehlt".into()))?;
        // GRZ-Bauteile mit Aufmaßbindung nur im Stand ihrer Quelle speichern.
        if let Some(a) = &b.eingabe.aufstellung {
            self.bauteil_planquellen_pruefen(pid, &s, a).await?;
        }
        if let Some(e) = &mut p.planungsrecht.baugrundstuecksermittlung {
            let ziel = e
                .berechnungen
                .iter_mut()
                .find(|x| x.schluessel == s)
                .ok_or_else(|| CoreError::InvalidInput("Berechnungsansatz fehlt".into()))?;
            *ziel = b;
        } else if let Some(e) = &mut p.planungsrecht.baugrundstuecke {
            let ziel = e
                .berechnungen
                .iter_mut()
                .find(|x| x.schluessel == s)
                .ok_or_else(|| CoreError::InvalidInput("Berechnungsansatz fehlt".into()))?;
            *ziel = b;
        } else {
            return Err(CoreError::InvalidInput(
                "Baugrundstücksermittlung zuerst speichern".into(),
            ));
        }
        p.updated_at = chrono::Utc::now();
        let audit = Self::audit(
            wer,
            Aktion::ProjektGeaendert,
            Some(pid),
            None,
            format!("Berechnung eines Baugrundstücks aktualisiert: {s}"),
        );
        self.repos.commit.projekt(p, Some(version), audit).await
    }
}
