use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
    domain::ProjectId,
    zugriff::ProjektRecht,
};
impl CoreServices {
    pub async fn abstandsplan_pruefen(
        &self,
        p: ProjectId,
        plan: &AbstandsPlan,
    ) -> CoreResult<AbstandsErgebnis> {
        if self.get_project(p).await?.bundesland != plan.regeln.land {
            return Err(err("Regelpaket gehört nicht zum Bundesland des Projekts"));
        }
        plan.validate()?;
        let infos = crate::grundstuecksinformationen::informationen(
            &self.get_project(p).await?.planungsrecht,
        );
        for g in &plan.grundstuecke {
            for id in &g.flurstueck_ids {
                if !infos.flurstuecke.iter().any(|f| &f.id == id) {
                    return Err(err(
                        "Grundstück verweist auf ein unbekanntes Flurstück des Projekts",
                    ));
                }
            }
        }
        for d in plan.dokumente() {
            if !self
                .document_history(d.dokument_id)
                .await?
                .iter()
                .any(|v| v.project_id == p && v.version == d.version)
            {
                return Err(err(
                    "Nachweisquelle gehört nicht zum Projekt oder zur angegebenen Version",
                ));
            }
        }
        berechnen(plan)
    }
    pub(crate) async fn abstandsplan_vorbereiten(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        daten: serde_json::Value,
        wer: &Benutzer,
    ) -> CoreResult<serde_json::Value> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        let mut neu: AbstandsPlan =
            serde_json::from_value(daten).map_err(|e| err(e.to_string()))?;
        let alt = self
            .repos
            .commit
            .fachstaende(p, "abstandsflaechen")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string());
        let actual = alt.as_ref().map(|f| f.revision).unwrap_or(0);
        if actual != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual,
            });
        }
        if let Some(alt) = alt {
            if alt.schema_version != 1 {
                return Err(err(
                    "Unbekannte Abstandsplanversion; Anwendung aktualisieren",
                ));
            }
            let vorher: AbstandsPlan =
                serde_json::from_value(alt.daten).map_err(|e| err(e.to_string()))?;
            if vorher.regeln.id == neu.regeln.id
                && vorher.regeln.version == neu.regeln.version
                && vorher.regeln != neu.regeln
            {
                return Err(err(
                    "Geänderter Regelinhalt benötigt eine neue Paketversion",
                ));
            }
            let mut vergleich = neu.clone();
            vergleich.recht.geprueft = vorher.recht.geprueft;
            vergleich.umfang_geprueft = vorher.umfang_geprueft;
            if vergleich != vorher {
                neu.recht.geprueft = false;
                neu.umfang_geprueft = false;
            } else if (neu.recht.geprueft && !vorher.recht.geprueft)
                || (neu.umfang_geprueft && !vorher.umfang_geprueft)
            {
                self.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
            }
        } else if neu.recht.geprueft || neu.umfang_geprueft {
            self.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                .await?;
        }
        serde_json::to_value(neu).map_err(|e| err(e.to_string()))
    }
    pub(crate) async fn abstandsplan_validieren(
        &self,
        p: ProjectId,
        plan: AbstandsPlan,
        wer: &Benutzer,
    ) -> CoreResult<serde_json::Value> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.abstandsplan_pruefen(p, &plan).await?;
        serde_json::to_value(plan).map_err(|e| err(e.to_string()))
    }
}
impl CoreServices {
    pub async fn abstandsnachweis_export(
        &self,
        p: ProjectId,
        id: Uuid,
        format: &str,
    ) -> CoreResult<(String, Vec<u8>)> {
        let n = self
            .nachweise(p)
            .await?
            .into_iter()
            .find(|n| n.stand.id == id)
            .ok_or_else(|| err("Nachweis nicht im Projekt gefunden"))?;
        let crate::plattform::nachweise::NachweisEingabe::AbstandProjekt { plan, .. } =
            &n.stand.eingabe
        else {
            return Err(err("Kein projektbezogener Abstandsflächennachweis"));
        };
        let r: AbstandsErgebnis =
            serde_json::from_value(n.stand.ergebnis.clone()).map_err(|e| err(e.to_string()))?;
        match format {
            "json" => Ok((
                "Abstandsflächennachweis.json".into(),
                serde_json::to_vec_pretty(&n.stand).map_err(|e| err(e.to_string()))?,
            )),
            "dxf" => Ok((
                "Abstandsflaechen.dxf".into(),
                bericht::dxf(plan, &r).into_bytes(),
            )),
            _ => Err(err(
                "Exportformat muss json oder dxf sein; PDF über Druckausgabe",
            )),
        }
    }
}
impl CoreServices {
    pub async fn abstandscad_konturen(
        &self,
        p: ProjectId,
        d: crate::plattform::objekte::Dokumentbezug,
        einheit: crate::cad::Laengeneinheit,
    ) -> CoreResult<Vec<CadKontur>> {
        let (meta, bytes) = self.load_version(d.dokument_id, d.version).await?;
        if meta.project_id != p
            || !meta.file_name.to_lowercase().ends_with(".dxf")
            || bytes.len() > 32 * 1024 * 1024
        {
            return Err(err(
                "DXF-Quelle muss zum Projekt gehören und höchstens 32 MB groß sein",
            ));
        }
        let plan = crate::cad::dxf::lesen(&bytes)?;
        Ok(plan
            .linien
            .iter()
            .enumerate()
            .filter(|(_, l)| l.geschlossen && (3..=2000).contains(&l.punkte.len()))
            .take(200)
            .map(|(i, l)| CadKontur {
                linie: i,
                ebene: plan
                    .ebenen
                    .get(l.ebene)
                    .map(|e| e.name.clone())
                    .unwrap_or_default(),
                punkte: l
                    .punkte
                    .iter()
                    .map(|v| APunkt {
                        x: v.0 * einheit.meter(),
                        y: v.1 * einheit.meter(),
                    })
                    .collect(),
                quelle: d.clone(),
            })
            .collect())
    }
}
