//! Fachmodule mit eigenen Schemas, Revisionen und öffentlichen Anwendungsfällen.
pub mod ablauf;
pub mod ausgabe;
pub mod nachweise;
pub mod objekte;
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::{Aktion, ProjectId},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ObjektArt {
    Stellplaetze,
    Wohnflaechen,
    Din277,
    Vollgeschosse,
    Hoehen,
    Freiflaechen,
    Entwaesserung,
    Ueberbaubarkeit,
    Abstandsflaechen,
    Verfahren,
    Projektorganisation,
    Gebaeude,
    Aufgaben,
    Unterlagen,
    Dokumentdaten,
    Raumbuch,
}
impl ObjektArt {
    pub fn modul(self) -> &'static str {
        match self {
            Self::Stellplaetze => "stellplaetze",
            Self::Wohnflaechen => "wohnflaechen",
            Self::Din277 => "din277",
            Self::Vollgeschosse => "vollgeschosse",
            Self::Hoehen => "hoehen",
            Self::Freiflaechen => "freiflaechen",
            Self::Entwaesserung => "entwaesserung",
            Self::Ueberbaubarkeit => "ueberbaubarkeit",
            Self::Abstandsflaechen => "abstandsflaechen",
            Self::Verfahren => "verfahren",
            Self::Projektorganisation => "projektorganisation",
            Self::Gebaeude => "gebaeude",
            Self::Aufgaben => "aufgaben",
            Self::Unterlagen => "unterlagen",
            Self::Dokumentdaten => "dokumentdaten",
            Self::Raumbuch => "raumbuch",
        }
    }
}
impl CoreServices {
    pub async fn fachobjekte(&self, p: ProjectId, art: ObjektArt) -> CoreResult<Vec<Fachstand>> {
        self.repos.projects.get(p).await?;
        self.repos.commit.fachstaende(p, art.modul()).await
    }
    pub async fn fachverlauf(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
    ) -> CoreResult<Vec<Fachstand>> {
        self.repos.projects.get(p).await?;
        self.repos
            .commit
            .fachverlauf(p, art.modul(), &id.to_string())
            .await
    }
    pub async fn fachobjekt_speichern(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
        basis: i64,
        daten: serde_json::Value,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        if art == ObjektArt::Verfahren {
            let a = serde_json::from_value(daten).map_err(json_err)?;
            return self.verfahren_commit(p, id, basis, a, wer, false).await;
        }
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        if basis < 0 {
            return Err(CoreError::InvalidInput("Ungültige Basisrevision".into()));
        }
        let projekt = self.repos.projects.get(p).await?;
        if serde_json::to_vec(&daten).map_err(json_err)?.len() > 2 * 1024 * 1024 {
            return Err(CoreError::InvalidInput("Fachdatensatz ist zu groß".into()));
        }
        let daten = if crate::berechnungen::BerechnungsArt::ALLE
            .iter()
            .any(|m| m.objekt() == art)
        {
            self.berechnungsplan_vorbereiten(p, id, basis, daten, wer)
                .await?
        } else if art == ObjektArt::Abstandsflaechen {
            self.abstandsplan_vorbereiten(p, id, basis, daten, wer)
                .await?
        } else {
            daten
        };
        let daten = if art == ObjektArt::Gebaeude {
            let mut g: objekte::Gebaeude = serde_json::from_value(daten).map_err(json_err)?;
            for geschoss in &mut g.geschosse {
                if let Some(q) = &geschoss.einstufungsquelle {
                    let (wert, grundlage) = self
                        .geschosseinstufung_aus_nachweis(p, q, id, geschoss.id, None)
                        .await?;
                    geschoss.vollgeschoss = Some(wert);
                    geschoss.grundlage = grundlage;
                }
            }
            serde_json::to_value(g).map_err(json_err)?
        } else {
            daten
        };
        let daten = objekte::validieren(self, &projekt, art, daten, wer).await?;
        if art == ObjektArt::Projektorganisation && id != p.0 {
            return Err(CoreError::InvalidInput(
                "Projektorganisation benötigt die Identität des Projekts".into(),
            ));
        }
        if art == ObjektArt::Dokumentdaten {
            let d: objekte::Dokumentdaten =
                serde_json::from_value(daten.clone()).map_err(json_err)?;
            if d.dokument.dokument_id.0 != id {
                return Err(CoreError::InvalidInput(
                    "Dokumentangaben benötigen die Identität des zugehörigen Dokuments".into(),
                ));
            }
        }
        if let Some(alt) = self
            .repos
            .commit
            .fachstaende(p, art.modul())
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
        {
            if alt.schema_version > 1 {
                return Err(CoreError::InvalidInput(
                    "Fachdatensatz wurde mit einer neueren Anwendung erstellt".into(),
                ));
            }
            if alt.daten == daten {
                return Ok(alt);
            }
        }
        let f = Fachstand {
            projekt_id: p,
            modul: art.modul().into(),
            objekt: id.to_string(),
            schema_version: 1,
            revision: basis,
            daten,
        };
        let audit = Self::audit(
            wer,
            Aktion::ProjektGeaendert,
            Some(p),
            None,
            format!("{} {} gespeichert", art.modul(), id),
        );
        self.repos.commit.fachstand(f, basis, audit).await
    }
}
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::InvalidInput(e.to_string())
}
pub fn text(s: &str, max: usize) -> CoreResult<()> {
    if s.trim().is_empty() || s.chars().count() > max {
        Err(CoreError::InvalidInput(format!(
            "Text mit 1 bis {max} Zeichen erforderlich"
        )))
    } else {
        Ok(())
    }
}
