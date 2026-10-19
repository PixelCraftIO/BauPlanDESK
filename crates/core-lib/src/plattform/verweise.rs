//! Verweise anderer Fachmodule auf Flurstücke der Grundstücksinformationen.
//!
//! Gebäude/Geschosse und Abstandsflächenpläne verweisen über stabile
//! Flurstück-IDs auf die Grundstücksinformationen. Vor dem Entfernen eines
//! Flurstücks zeigt die Oberfläche die betroffenen Angaben; nach dem Speichern
//! können die Verweise ausdrücklich bereinigt werden. Bereinigt wird nur, was
//! tatsächlich nicht mehr erfasst ist; jedes Objekt erhält eine neue Revision,
//! ein Abstandsplan dadurch auch eine erneute Prüfpflicht.
use super::{ObjektArt, objekte::Gebaeude};
use crate::{CoreError, CoreResult, CoreServices, benutzer::Benutzer, domain::ProjectId};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Flurstuecksverweis {
    pub flurstueck_id: String,
    /// Fachbereich des verweisenden Objekts: `gebaeude` oder `abstandsflaechen`.
    pub bereich: String,
    pub objekt: Uuid,
    pub bezeichnung: String,
}

fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::InvalidInput(e.to_string())
}

impl CoreServices {
    /// Objekte, die auf eines der Flurstücke `ids` verweisen.
    pub async fn flurstuecksverweise(
        &self,
        p: ProjectId,
        ids: &[String],
    ) -> CoreResult<Vec<Flurstuecksverweis>> {
        let mut r = Vec::new();
        for f in self.repos.commit.fachstaende(p, "gebaeude").await? {
            let g: Gebaeude = serde_json::from_value(f.daten).map_err(json_err)?;
            for id in g.flurstueck_ids.iter().filter(|id| ids.contains(id)) {
                r.push(Flurstuecksverweis {
                    flurstueck_id: id.clone(),
                    bereich: "gebaeude".into(),
                    objekt: f
                        .objekt
                        .parse()
                        .map_err(|_| CoreError::InvalidInput("Gebäude-ID ungültig".into()))?,
                    bezeichnung: format!("Gebäude {}", g.bezeichnung),
                });
            }
        }
        for f in self.repos.commit.fachstaende(p, "abstandsflaechen").await? {
            let a: crate::abstandsflaechen::AbstandsPlan =
                serde_json::from_value(f.daten).map_err(json_err)?;
            for g in &a.grundstuecke {
                for id in g.flurstueck_ids.iter().filter(|id| ids.contains(id)) {
                    r.push(Flurstuecksverweis {
                        flurstueck_id: id.clone(),
                        bereich: "abstandsflaechen".into(),
                        objekt: f.objekt.parse().map_err(|_| {
                            CoreError::InvalidInput("Abstandsplan-ID ungültig".into())
                        })?,
                        bezeichnung: format!(
                            "Abstandsflächen {} · {}",
                            a.bezeichnung, g.bezeichnung
                        ),
                    });
                }
            }
        }
        Ok(r)
    }

    /// Entfernt Verweise auf die Flurstücke `ids`, die in den gespeicherten
    /// Grundstücksinformationen nicht mehr erfasst sind. Liefert die Anzahl
    /// geänderter Objekte.
    pub async fn flurstuecksverweise_entfernen(
        &self,
        p: ProjectId,
        ids: &[String],
        wer: &Benutzer,
    ) -> CoreResult<usize> {
        let infos = crate::grundstuecksinformationen::informationen(
            &self.get_project(p).await?.planungsrecht,
        );
        if ids
            .iter()
            .any(|id| infos.flurstuecke.iter().any(|f| &f.id == id))
        {
            return Err(CoreError::InvalidInput(
                "Nur Verweise auf nicht mehr erfasste Flurstücke werden entfernt; Grundstücksinformationen zuerst speichern".into(),
            ));
        }
        let mut anzahl = 0;
        for f in self.repos.commit.fachstaende(p, "gebaeude").await? {
            let mut g: Gebaeude = serde_json::from_value(f.daten).map_err(json_err)?;
            let vorher = g.flurstueck_ids.len();
            g.flurstueck_ids.retain(|id| !ids.contains(id));
            if g.flurstueck_ids.len() == vorher {
                continue;
            }
            let id = f
                .objekt
                .parse()
                .map_err(|_| CoreError::InvalidInput("Gebäude-ID ungültig".into()))?;
            let daten = serde_json::to_value(g).map_err(json_err)?;
            self.fachobjekt_speichern(p, ObjektArt::Gebaeude, id, f.revision, daten, wer)
                .await?;
            anzahl += 1;
        }
        for f in self.repos.commit.fachstaende(p, "abstandsflaechen").await? {
            let mut a: crate::abstandsflaechen::AbstandsPlan =
                serde_json::from_value(f.daten).map_err(json_err)?;
            let mut geaendert = false;
            for g in &mut a.grundstuecke {
                let vorher = g.flurstueck_ids.len();
                g.flurstueck_ids.retain(|id| !ids.contains(id));
                geaendert |= g.flurstueck_ids.len() != vorher;
            }
            if !geaendert {
                continue;
            }
            let id = f
                .objekt
                .parse()
                .map_err(|_| CoreError::InvalidInput("Abstandsplan-ID ungültig".into()))?;
            let daten = serde_json::to_value(a).map_err(json_err)?;
            self.fachobjekt_speichern(p, ObjektArt::Abstandsflaechen, id, f.revision, daten, wer)
                .await?;
            anzahl += 1;
        }
        Ok(anzahl)
    }
}
