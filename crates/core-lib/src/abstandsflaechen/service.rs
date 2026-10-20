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
        self.abstands_planquellen_pruefen(p, plan).await?;
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
        let gebaeude = self.gebaeudenamen(p).await?;
        for (name, id) in plan
            .waende
            .iter()
            .map(|w| (&w.bezeichnung, w.gebaeude_id))
            .chain(plan.umrisse.iter().map(|u| (&u.bezeichnung, u.gebaeude_id)))
        {
            if let Some(id) = id
                && !gebaeude.iter().any(|(g, _)| *g == id)
            {
                return Err(err(format!(
                    "{name}: verknüpftes Gebäude gehört nicht zum Projekt"
                )));
            }
        }
        berechnen(plan)
    }
    /// Identität und Bezeichnung der Gebäude aus Gebäude/Geschosse.
    async fn gebaeudenamen(&self, p: ProjectId) -> CoreResult<Vec<(Uuid, String)>> {
        let mut r = vec![];
        for f in self.repos.commit.fachstaende(p, "gebaeude").await? {
            let g: crate::plattform::objekte::Gebaeude =
                serde_json::from_value(f.daten).map_err(|e| err(e.to_string()))?;
            r.push((
                f.objekt.parse().map_err(|_| err("Gebäude-ID ungültig"))?,
                g.bezeichnung,
            ));
        }
        Ok(r)
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
        self.planwerte_abstand(p, &mut neu).await?;
        neu.validate()?;
        // Umrisswände folgen ihrem Umriss; die Oberfläche bindet bereits beim
        // Bearbeiten ein, hier wird nur sichergestellt, dass nichts abweicht.
        umrisse_einbinden(&mut neu);
        // Verknüpfte Wände tragen die aktuelle Bezeichnung ihres Gebäudes.
        let gebaeude = self.gebaeudenamen(p).await?;
        for w in &mut neu.waende {
            if let Some((_, name)) = w
                .gebaeude_id
                .and_then(|id| gebaeude.iter().find(|(g, _)| *g == id))
            {
                w.gebaeude = name.clone();
            }
        }
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
    /// Höhenkoten (Text) und Punkte mit Höhe aus einer DXF-Version des Projekts.
    pub async fn abstandscad_hoehen(
        &self,
        p: ProjectId,
        d: crate::plattform::objekte::Dokumentbezug,
        einheit: crate::cad::Laengeneinheit,
    ) -> CoreResult<Vec<CadHoehe>> {
        let plan = self.abstandscad_lesen(p, &d).await?;
        let m = einheit.meter();
        let ebene = |i: usize| {
            plan.ebenen
                .get(i)
                .map(|e| e.name.clone())
                .unwrap_or_default()
        };
        Ok(plan
            .texte
            .iter()
            .filter_map(|t| {
                cadhoehen::hoehenkote(&t.inhalt).map(|h| CadHoehe {
                    x: t.position.0 * m,
                    y: t.position.1 * m,
                    // Koten sind unabhängig von der Zeichnungseinheit in Metern beschriftet.
                    hoehe_m: h,
                    ebene: ebene(t.ebene),
                    art: "Höhenkote".into(),
                    quelle: d.clone(),
                })
            })
            .chain(plan.raumpunkte.iter().map(|r| CadHoehe {
                x: r.position.0 * m,
                y: r.position.1 * m,
                hoehe_m: r.z * m,
                ebene: ebene(r.ebene),
                art: "Höhenpunkt".into(),
                quelle: d.clone(),
            }))
            .take(20_000)
            .collect())
    }
    async fn abstandscad_lesen(
        &self,
        p: ProjectId,
        d: &crate::plattform::objekte::Dokumentbezug,
    ) -> CoreResult<crate::cad::Plan> {
        let (meta, bytes) = self.load_version(d.dokument_id, d.version).await?;
        if meta.project_id != p
            || !meta.file_name.to_lowercase().ends_with(".dxf")
            || bytes.len() > 32 * 1024 * 1024
        {
            return Err(err(
                "DXF-Quelle muss zum Projekt gehören und höchstens 32 MB groß sein",
            ));
        }
        crate::cad::dxf::lesen(&bytes)
    }
    pub async fn abstandscad_konturen(
        &self,
        p: ProjectId,
        d: crate::plattform::objekte::Dokumentbezug,
        einheit: crate::cad::Laengeneinheit,
    ) -> CoreResult<Vec<CadKontur>> {
        let plan = self.abstandscad_lesen(p, &d).await?;
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
