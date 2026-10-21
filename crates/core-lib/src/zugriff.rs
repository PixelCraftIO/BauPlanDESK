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
/// Eingabevertrag (Desktop/Server) einer Projektmitgliedschaft; bleibt an der
/// API-Grenze streng. Gespeicherte Mitgliedschaften liest die Zugriffsprüfung
/// tolerant über [`gespeichertes_recht`], damit eine neuere Programmversion an
/// derselben Datenbank Felder ergänzen kann, ohne ältere Arbeitsplätze mit einem
/// Datenbankfehler auszusperren.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ProjektMitglied {
    pub benutzer_id: BenutzerId,
    pub recht: ProjektRecht,
}
const MITGLIED_FELDER: [&str; 2] = ["benutzer_id", "recht"];
/// Liest nur das für die Zugriffsprüfung nötige Recht aus einer gespeicherten
/// Mitgliedschaft; unbekannte weitere Felder werden ignoriert. Ein unbekanntes
/// Recht (neuere Version) gewährt nichts: Zugriff verweigert, mit Hinweis.
fn gespeichertes_recht(daten: &serde_json::Value) -> CoreResult<ProjektRecht> {
    let recht = daten.get("recht").ok_or_else(|| {
        CoreError::Database("Projektmitgliedschaft ohne Recht gespeichert".into())
    })?;
    serde_json::from_value(recht.clone()).map_err(|_| {
        CoreError::Forbidden(format!(
            "Die Projektberechtigung {recht} stammt aus einer neueren BauPlanIO-Version. \
             Bitte BauPlanIO aktualisieren."
        ))
    })
}
/// Enthält die gespeicherte Mitgliedschaft Angaben, die diese Version nicht kennt?
fn aus_neuerer_version(daten: &serde_json::Value) -> bool {
    daten
        .as_object()
        .is_some_and(|o| o.keys().any(|k| !MITGLIED_FELDER.contains(&k.as_str())))
        || matches!(gespeichertes_recht(daten), Err(CoreError::Forbidden(_)))
}
impl CoreServices {
    pub async fn einreichung_zugriff(
        &self,
        id: uuid::Uuid,
        wer: &Benutzer,
        min: ProjektRecht,
    ) -> CoreResult<()> {
        let e = self.repos.einreichungen.laden(id).await?;
        self.projekt_zugriff(e.stand.projekt_id, wer, min).await
    }

    /// Prüft den Projektzugriff anhand der tatsächlichen DMS-Zuordnung.
    /// Adapter dürfen eine vom Aufrufer genannte Projekt-ID hierfür nicht verwenden.
    pub async fn dokument_zugriff(
        &self,
        id: crate::domain::DocumentId,
        wer: &Benutzer,
        min: ProjektRecht,
    ) -> CoreResult<()> {
        let d = self.repos.documents.latest(id).await?;
        self.projekt_zugriff(d.project_id, wer, min).await
    }

    pub async fn projekt_zugriff(
        &self,
        p: ProjectId,
        wer: &Benutzer,
        min: ProjektRecht,
    ) -> CoreResult<()> {
        let aktuell = self.angemeldeter_benutzer(wer).await?;
        if !aktuell.aktiv {
            return Err(CoreError::Forbidden("Benutzerkonto ist deaktiviert".into()));
        }
        if aktuell.rolle == Rolle::Administrator {
            return Ok(());
        }
        let recht = self
            .mitgliedsrecht(p, wer)
            .await?
            .ok_or_else(|| CoreError::Forbidden("Keine Projektmitgliedschaft".into()))?;
        if recht < min {
            return Err(CoreError::Forbidden(
                "Projektberechtigung reicht nicht aus".into(),
            ));
        }
        Ok(())
    }
    /// Wirksames Projektrecht des Benutzers nach derselben Regel wie
    /// [`Self::projekt_zugriff`]: Administratoren haben `Leitung`, Mitglieder
    /// ihr eingetragenes Recht. Wer das Projekt nicht lesen darf (deaktiviertes
    /// Konto, keine Mitgliedschaft, Recht `Kein`), erhält wie beim Öffnen
    /// `Forbidden`. Die Oberfläche richtet Bearbeitungs- und Leitungsfunktionen
    /// danach aus; geprüft wird weiterhin an der Servicegrenze.
    pub async fn eigenes_projektrecht(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<ProjektRecht> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        self.repos.projects.get(p).await?;
        if self.repos.benutzer.get(wer.id).await?.rolle == Rolle::Administrator {
            return Ok(ProjektRecht::Leitung);
        }
        Ok(self
            .mitgliedsrecht(p, wer)
            .await?
            .unwrap_or(ProjektRecht::Kein))
    }
    /// Gespeichertes Recht der Mitgliedschaft, tolerant gelesen über
    /// [`gespeichertes_recht`]; `None` ohne Mitgliedschaft.
    async fn mitgliedsrecht(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Option<ProjektRecht>> {
        let Some(f) = self
            .repos
            .commit
            .fachstaende(p, "mitgliedschaften")
            .await?
            .into_iter()
            .find(|f| f.objekt == wer.id.to_string())
        else {
            return Ok(None);
        };
        gespeichertes_recht(&f.daten).map(Some)
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
        // Eine von einer neueren Version gespeicherte Mitgliedschaft nicht mit dem
        // eigenen, kleineren Format überschreiben (Felder gingen still verloren).
        if let Some(alt) = self
            .repos
            .commit
            .fachstaende(p, "mitgliedschaften")
            .await?
            .into_iter()
            .find(|f| f.objekt == objekt)
            && aus_neuerer_version(&alt.daten)
        {
            return Err(CoreError::InvalidInput(
                "Diese Projektberechtigung wurde mit einer neueren BauPlanIO-Version \
                 gespeichert. Bitte BauPlanIO aktualisieren; die Berechtigung wurde nicht \
                 geändert."
                    .into(),
            ));
        }
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

    // --- Sichtbarkeit von Kundendaten (Bauherren, Beteiligte; DSGVO) ---------
    //
    // Regel: Wer Kunden pflegen darf (Recht `KundenBearbeiten`, also Bearbeitung,
    // Projektleitung, Administration), sieht die gesamte Kundendatenbank. Alle
    // anderen sehen nur Kunden, die als Bauherr oder Beteiligter in einem Projekt
    // eingetragen sind, das sie lesen dürfen. Adapter verwenden ausschließlich
    // [`Self::kunden_fuer_benutzer`] und [`Self::kunde_fuer_benutzer`].

    /// Darf `wer` die gesamte Kundendatenbank sehen? Maßgebend ist die aktuell
    /// gespeicherte Rolle; deaktivierte Konten sehen nichts.
    async fn alle_kunden_sichtbar(&self, wer: &Benutzer) -> CoreResult<bool> {
        let aktuell = self.repos.benutzer.get(wer.id).await?;
        if !aktuell.aktiv {
            return Err(CoreError::Forbidden("Benutzerkonto ist deaktiviert".into()));
        }
        Ok(aktuell.rolle.darf(Recht::KundenBearbeiten) && wer.rolle.darf(Recht::KundenBearbeiten))
    }
    /// Kunden aus den Projekten, die `wer` lesen darf.
    async fn kunden_aus_sichtbaren_projekten(
        &self,
        wer: &Benutzer,
    ) -> CoreResult<std::collections::HashSet<crate::kunden::KundeId>> {
        let mut ids = std::collections::HashSet::new();
        for p in self.projekte_fuer_benutzer(wer).await? {
            ids.extend(self.bauherren(p.id).await?.into_iter().map(|k| k.id));
            ids.extend(self.beteiligte(p.id).await?.into_iter().map(|b| b.kunde.id));
        }
        Ok(ids)
    }
    /// Kundenliste in der Sicht von `wer` (siehe Regel oben).
    pub async fn kunden_fuer_benutzer(
        &self,
        wer: &Benutzer,
    ) -> CoreResult<Vec<crate::kunden::Kunde>> {
        if self.alle_kunden_sichtbar(wer).await? {
            return self.kunden_auflisten().await;
        }
        let ids = self.kunden_aus_sichtbaren_projekten(wer).await?;
        let mut k = self.kunden_auflisten().await?;
        k.retain(|k| ids.contains(&k.id));
        Ok(k)
    }
    /// Einzelner Kunde in der Sicht von `wer`; sonst `Forbidden`.
    pub async fn kunde_fuer_benutzer(
        &self,
        id: crate::kunden::KundeId,
        wer: &Benutzer,
    ) -> CoreResult<crate::kunden::Kunde> {
        if !self.alle_kunden_sichtbar(wer).await? {
            let sichtbar = self
                .repos
                .kunden
                .projekte_des_kunden(id)
                .await?
                .into_iter()
                .map(|p| p.id)
                .collect::<Vec<_>>();
            let mut erlaubt = false;
            for p in sichtbar {
                if self
                    .projekt_zugriff(p, wer, ProjektRecht::Lesen)
                    .await
                    .is_ok()
                {
                    erlaubt = true;
                    break;
                }
            }
            if !erlaubt {
                return Err(CoreError::Forbidden(
                    "Kunde ist in keinem sichtbaren Projekt eingetragen".into(),
                ));
            }
        }
        self.kunde(id).await
    }
}
