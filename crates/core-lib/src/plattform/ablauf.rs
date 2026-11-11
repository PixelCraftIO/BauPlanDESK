//! Projektstatus und konfigurierbare Bürochecklisten. Keine gesetzliche Vollständigkeitszusage.
use super::objekte::{Aufgabe, AufgabenStatus, Unterlage, UnterlagenStatus};
use super::*;
use chrono::Utc;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ChecklistenQuelle {
    pub id: String,
    pub version: String,
    pub punkt: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Checklistenpunkt {
    pub id: String,
    pub bezeichnung: String,
    pub erforderlich: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Checkliste {
    pub id: String,
    pub version: String,
    pub titel: String,
    pub hinweis: String,
    pub unterlagen: Vec<Checklistenpunkt>,
}
impl Checkliste {
    pub fn pruefen(&self) -> CoreResult<()> {
        text(&self.id, 120)?;
        text(&self.version, 120)?;
        text(&self.titel, 500)?;
        if self.hinweis.len() > 4000 || self.unterlagen.len() > 200 {
            return Err(CoreError::InvalidInput("Checkliste ist zu groß".into()));
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.unterlagen {
            text(&p.id, 120)?;
            text(&p.bezeichnung, 500)?;
            if !ids.insert(&p.id) {
                return Err(CoreError::InvalidInput(
                    "Doppelte Checklistenkennung".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn standard() -> CoreResult<Self> {
        let s: Self = serde_json::from_str(include_str!(
            "../../../../workflow-regeln/bauantrag-buero.json"
        ))
        .map_err(json_err)?;
        s.pruefen()?;
        Ok(s)
    }
}
#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
pub struct Statushinweis {
    pub bereich: String,
    pub text: String,
}
#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
pub struct Projektstatus {
    pub ermittelt_am: chrono::DateTime<Utc>,
    pub offene_aufgaben: usize,
    pub ueberfaellige_aufgaben: usize,
    pub erforderliche_unterlagen: usize,
    pub fehlende_unterlagen: usize,
    pub ungepruefte_unterlagen: usize,
    pub nachweise: usize,
    pub veraltete_nachweise: usize,
    pub offene_freigaben: usize,
    pub exportierte_pakete: usize,
    pub bestaetigte_einreichungen: usize,
    pub hinweise: Vec<Statushinweis>,
}
impl CoreServices {
    pub async fn checkliste_anwenden(
        &self,
        p: ProjectId,
        vorlage: Checkliste,
        wer: &Benutzer,
    ) -> CoreResult<usize> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        vorlage.pruefen()?;
        let vorhandene = self.fachobjekte(p, ObjektArt::Unterlagen).await?;
        let mut neu = 0;
        for punkt in &vorlage.unterlagen {
            let id = Uuid::new_v5(
                &p.0,
                format!("checkliste/{}/{}", vorlage.id, punkt.id).as_bytes(),
            );
            if vorhandene.iter().any(|v| v.objekt == id.to_string()) {
                continue;
            }
            let u = Unterlage {
                bezeichnung: punkt.bezeichnung.clone(),
                erforderlich: punkt.erforderlich,
                dokument: None,
                status: UnterlagenStatus::Entwurf,
                bemerkung: vorlage.hinweis.clone(),
                vorlage: Some(ChecklistenQuelle {
                    id: vorlage.id.clone(),
                    version: vorlage.version.clone(),
                    punkt: punkt.id.clone(),
                }),
            };
            match self
                .fachobjekt_speichern(
                    p,
                    ObjektArt::Unterlagen,
                    id,
                    0,
                    serde_json::to_value(u).map_err(json_err)?,
                    wer,
                )
                .await
            {
                Ok(_) => neu += 1,
                Err(CoreError::VersionConflict { .. }) => {} // Parallel angewendete Vorlage wird nicht überschrieben.
                Err(e) => return Err(e),
            }
        }
        Ok(neu)
    }
    pub async fn projektstatus(&self, p: ProjectId) -> CoreResult<Projektstatus> {
        let projekt = self.repos.projects.get(p).await?;
        let aufgaben = self
            .fachobjekte(p, ObjektArt::Aufgaben)
            .await?
            .into_iter()
            .map(|f| serde_json::from_value::<Aufgabe>(f.daten).map_err(json_err))
            .collect::<CoreResult<Vec<_>>>()?;
        let unterlagen = self
            .fachobjekte(p, ObjektArt::Unterlagen)
            .await?
            .into_iter()
            .map(|f| serde_json::from_value::<Unterlage>(f.daten).map_err(json_err))
            .collect::<CoreResult<Vec<_>>>()?;
        let nachweise = self.nachweise(p).await?;
        let pakete = self.einreichungen(p).await?;
        let docs = self.list_documents(p).await?;
        let mut s = Projektstatus {
            ermittelt_am: Utc::now(),
            offene_aufgaben: 0,
            ueberfaellige_aufgaben: 0,
            erforderliche_unterlagen: 0,
            fehlende_unterlagen: 0,
            ungepruefte_unterlagen: 0,
            nachweise: nachweise.len(),
            veraltete_nachweise: 0,
            offene_freigaben: 0,
            exportierte_pakete: pakete.len(),
            bestaetigte_einreichungen: pakete.iter().filter(|p| p.einreichung.is_some()).count(),
            hinweise: vec![],
        };
        for a in aufgaben {
            if a.status != AufgabenStatus::Erledigt {
                s.offene_aufgaben += 1;
                if a.faellig.is_some_and(|d| d < crate::kalender::heute()) {
                    s.ueberfaellige_aufgaben += 1;
                }
            }
        }
        for u in unterlagen {
            if u.erforderlich {
                s.erforderliche_unterlagen += 1;
                match &u.dokument {
                    None => s.fehlende_unterlagen += 1,
                    Some(d) => {
                        let aktuell = docs
                            .iter()
                            .filter(|v| v.document_id == d.dokument_id)
                            .map(|v| v.version)
                            .max();
                        if u.status != UnterlagenStatus::Freigegeben || aktuell != Some(d.version) {
                            s.ungepruefte_unterlagen += 1;
                        }
                    }
                }
            }
        }
        for n in nachweise {
            if n.ersetzt_durch.is_some() {
                continue;
            }
            if !n.aktuell {
                s.veraltete_nachweise += 1;
            }
            if n.freigabe.is_none() {
                s.offene_freigaben += 1;
            }
        }
        let mut hinweis = |bereich: &str, text: String| {
            s.hinweise.push(Statushinweis {
                bereich: bereich.into(),
                text,
            })
        };
        if projekt.planungsrecht.rechtsgrundlage.is_none() {
            hinweis("rechtsgrundlagen", "Rechtsgrundlage noch festlegen".into());
        }
        match crate::grundstueckskontext::ermitteln(&projekt) {
            Ok(g) if g.is_empty() || g.iter().any(|g| g.bezugsflaeche_m2.is_none()) => hinweis(
                "grundstueck",
                "Baugrundstücke und Bezugsflächen prüfen".into(),
            ),
            Err(e) => hinweis("grundstueck", format!("Grundstücksermittlung prüfen: {e}")),
            _ => {}
        }
        if s.ueberfaellige_aufgaben > 0 {
            hinweis(
                "aufgaben",
                format!("{} Aufgaben überfällig", s.ueberfaellige_aufgaben),
            );
        }
        if s.fehlende_unterlagen + s.ungepruefte_unterlagen > 0 {
            hinweis(
                "unterlagen",
                format!(
                    "{} erforderliche Unterlagen fehlen, {} sind zu prüfen",
                    s.fehlende_unterlagen, s.ungepruefte_unterlagen
                ),
            );
        }
        if s.veraltete_nachweise > 0 {
            hinweis(
                "nachweise",
                format!(
                    "{} Nachweisstände benötigen eine neue Prüfung",
                    s.veraltete_nachweise
                ),
            );
        }
        let akten = self.fachobjekte(p, ObjektArt::Verfahren).await?;
        for f in akten {
            let a: crate::verfahren::Verfahrensakte =
                serde_json::from_value(f.daten).map_err(json_err)?;
            if a.status == crate::verfahren::Verfahrensstatus::Zurueckgenommen {
                continue;
            }
            let punkte = crate::verfahren::termine::offene_forderungspunkte(&a).count();
            let pflichten = crate::verfahren::termine::offene_pflichten(&a).count();
            let unklar = a
                .anforderungen
                .iter()
                .filter(|r| r.erforderlich == crate::verfahren::Erforderlichkeit::Ungeklaert)
                .count();
            if punkte + pflichten + unklar > 0 {
                hinweis(
                    "verfahren",
                    format!(
                        "{}: {} Forderungspunkte offen, {} Nebenbestimmungen offen, {} Anforderungen ungeklärt",
                        a.bezeichnung, punkte, pflichten, unklar
                    ),
                );
            }
        }
        let ueberfaellig = self
            .verfahren_erinnerungen(p, crate::kalender::heute())
            .await?
            .iter()
            .filter(|e| e.ueberfaellig)
            .count();
        if ueberfaellig > 0 {
            hinweis(
                "aktensuche",
                format!("{ueberfaellig} Verfahrensfristen überfällig – Wiedervorlagen öffnen"),
            );
        }
        Ok(s)
    }
}
