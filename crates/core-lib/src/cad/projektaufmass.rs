//! Dauerhafte DXF-Messungen, gebunden an Dokumentversion und Zeichnungseinheit.
use super::*;
use crate::{
    CoreServices,
    benutzer::Benutzer,
    commit::Fachstand,
    domain::ProjectId,
    plattform::{ObjektArt, objekte::Dokumentbezug},
};
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CadMarkierung {
    pub id: Uuid,
    pub bezeichnung: String,
    pub punkte: Vec<[f64; 2]>,
    pub geschlossen: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct CadProjektaufmass {
    pub dokument: Dokumentbezug,
    pub einheit: Laengeneinheit,
    pub markierungen: Vec<CadMarkierung>,
}
impl CadProjektaufmass {
    pub fn messen(&self) -> CoreResult<Vec<Messung>> {
        if self.markierungen.len() > 2000 {
            return Err(CoreError::InvalidInput("Zu viele CAD-Messungen".into()));
        }
        self.markierungen
            .iter()
            .map(|m| {
                if m.punkte.len() > 10000 {
                    return Err(CoreError::InvalidInput("Zu viele Aufmaßpunkte".into()));
                }
                messen(
                    &m.punkte
                        .iter()
                        .map(|p| Punkt(p[0], p[1]))
                        .collect::<Vec<_>>(),
                    m.geschlossen,
                    self.einheit,
                )
            })
            .collect()
    }
}
fn id(d: &Dokumentbezug) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("bauprojekt:cadaufmass:{}:{}", d.dokument_id.0, d.version).as_bytes(),
    )
}
impl CoreServices {
    pub async fn cadaufmass_csv(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
        revision: i64,
    ) -> CoreResult<Vec<u8>> {
        let f = self
            .cadaufmass_laden(p, d)
            .await?
            .ok_or_else(|| CoreError::NotFound("CAD-Aufmaß".into()))?;
        if f.revision != revision {
            return Err(CoreError::VersionConflict {
                expected: revision,
                actual: f.revision,
            });
        }
        if f.schema_version != 1 {
            return Err(CoreError::InvalidInput(
                "CAD-Aufmaß-Datenversion nicht unterstützt".into(),
            ));
        }
        let a: CadProjektaufmass =
            serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        let werte = a.messen()?;
        let text = |s: &str| {
            let s = if s.trim_start().starts_with(['=', '+', '-', '@']) {
                format!("'{s}")
            } else {
                s.into()
            };
            format!("\"{}\"", s.replace('"', "\"\""))
        };
        let zahl = |n: f64| format!("{n:.6}").replace('.', ",");
        let mut out = String::from(
            "\u{feff}Dokument-ID;Dokumentversion;Aufmassrevision;Markierungs-ID;Bezeichnung;Laenge_m;Flaeche_m2;Geschlossen;Zeichnungspunkte\r\n",
        );
        for (m, v) in a.markierungen.iter().zip(werte) {
            let row = vec![
                text(&d.dokument_id.0.to_string()),
                d.version.to_string(),
                revision.to_string(),
                text(&m.id.to_string()),
                text(&m.bezeichnung),
                zahl(v.laenge_m),
                v.flaeche_m2.map(zahl).unwrap_or_default(),
                m.geschlossen.to_string(),
                text(
                    &serde_json::to_string(&m.punkte)
                        .map_err(|e| CoreError::InvalidInput(e.to_string()))?,
                ),
            ];
            out.push_str(&row.join(";"));
            out.push_str("\r\n");
        }
        Ok(out.into_bytes())
    }
    pub async fn cadaufmass_pruefen(&self, p: ProjectId, a: &CadProjektaufmass) -> CoreResult<()> {
        let d = self
            .document_history(a.dokument.dokument_id)
            .await?
            .into_iter()
            .find(|d| d.project_id == p && d.version == a.dokument.version)
            .ok_or_else(|| {
                CoreError::InvalidInput(
                    "CAD-Unterlage gehört nicht zu dieser Projektversion".into(),
                )
            })?;
        if !d.file_name.to_lowercase().ends_with(".dxf") {
            return Err(CoreError::InvalidInput(
                "CAD-Aufmaß benötigt eine DXF-Unterlage".into(),
            ));
        }
        if a.markierungen.len() > 2000 {
            return Err(CoreError::InvalidInput("Zu viele CAD-Messungen".into()));
        }
        let mut ids = std::collections::HashSet::new();
        for m in &a.markierungen {
            crate::plattform::text(&m.bezeichnung, 500)?;
            if !ids.insert(m.id)
                || m.punkte.len() > 10000
                || m.punkte
                    .iter()
                    .flatten()
                    .any(|p| !p.is_finite() || p.abs() > 1e12)
            {
                return Err(CoreError::InvalidInput(
                    "CAD-Messung besitzt ungültige Punkte oder Identität".into(),
                ));
            }
            let v = messen(
                &m.punkte
                    .iter()
                    .map(|p| Punkt(p[0], p[1]))
                    .collect::<Vec<_>>(),
                m.geschlossen,
                a.einheit,
            )?;
            if v.selbstschneidend {
                return Err(CoreError::InvalidInput(format!(
                    "{}: Kontur überschneidet sich",
                    m.bezeichnung
                )));
            }
        }
        Ok(())
    }
    pub async fn cadaufmass_laden(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
    ) -> CoreResult<Option<Fachstand>> {
        self.dokument_zugriff_auf_projekt(p, d).await?;
        self.fachobjekt_lesen(p, ObjektArt::Cadaufmass, id(d)).await
    }
    async fn dokument_zugriff_auf_projekt(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
    ) -> CoreResult<()> {
        if !self
            .document_history(d.dokument_id)
            .await?
            .iter()
            .any(|v| v.project_id == p && v.version == d.version)
        {
            return Err(CoreError::InvalidInput(
                "CAD-Dokumentversion gehört nicht zum Projekt".into(),
            ));
        }
        Ok(())
    }
    pub async fn cadaufmass_speichern(
        &self,
        p: ProjectId,
        a: CadProjektaufmass,
        basis: i64,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.fachobjekt_speichern(
            p,
            ObjektArt::Cadaufmass,
            id(&a.dokument),
            basis,
            serde_json::to_value(a).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
            wer,
        )
        .await
    }
}
