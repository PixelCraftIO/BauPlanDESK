use super::*;
use crate::{
    benutzer::BenutzerId,
    domain::{DocumentId, Project},
};
use chrono::NaiveDate;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dokumentbezug {
    pub dokument_id: DocumentId,
    #[ts(type = "number")]
    pub version: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Gebaeude {
    pub bezeichnung: String,
    pub grundstueck_ids: Vec<Uuid>,
    pub geschosse: Vec<GebaeudeGeschoss>,
    pub quelle: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GebaeudeGeschoss {
    pub id: Uuid,
    pub bezeichnung: String,
    pub vollgeschoss: Option<bool>,
    pub grundlage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AufgabenStatus {
    Offen,
    InArbeit,
    Erledigt,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Aufgabe {
    pub titel: String,
    pub beschreibung: String,
    pub faellig: Option<NaiveDate>,
    pub verantwortlich: Option<BenutzerId>,
    pub status: AufgabenStatus,
    pub bezug: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum UnterlagenStatus {
    Entwurf,
    Pruefung,
    Freigegeben,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Unterlage {
    #[serde(default)]
    pub vorlage: Option<super::ablauf::ChecklistenQuelle>,
    pub bezeichnung: String,
    pub erforderlich: bool,
    pub dokument: Option<Dokumentbezug>,
    pub status: UnterlagenStatus,
    pub bemerkung: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dokumentdaten {
    pub dokument: Dokumentbezug,
    pub dokumentart: String,
    pub plannummer: String,
    pub revisionsindex: String,
    pub massstab: String,
    pub verfasser: String,
    pub datum: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Raumbuch {
    pub bezeichnung: String,
    pub gebaeude_id: Uuid,
    pub geschoss_id: Option<Uuid>,
    pub raeume: Vec<Raum>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Raum {
    pub id: Uuid,
    pub nummer: String,
    pub bezeichnung: String,
    pub nutzung: String,
    pub flaeche_m2: f64,
    pub hoehe_m: Option<f64>,
}
#[derive(Debug, Serialize, ts_rs::TS)]
pub struct RaumSummen {
    pub anzahl: usize,
    pub flaeche_m2: f64,
    pub volumen_m3: Option<f64>,
}
impl Raumbuch {
    pub fn summen(&self) -> CoreResult<RaumSummen> {
        let mut ids = std::collections::HashSet::new();
        let mut nummern = std::collections::HashSet::new();
        let mut flaeche = 0.;
        let mut volumen = Some(0.);
        if self.raeume.len() > 5000 {
            return Err(CoreError::InvalidInput(
                "Höchstens 5000 Räume je Raumbuch".into(),
            ));
        }
        for r in &self.raeume {
            text(&r.nummer, 120)?;
            text(&r.bezeichnung, 500)?;
            if !ids.insert(r.id)
                || !nummern.insert(r.nummer.trim().to_lowercase())
                || !r.flaeche_m2.is_finite()
                || r.flaeche_m2 < 0.
                || r.hoehe_m.is_some_and(|h| !h.is_finite() || h <= 0.)
            {
                return Err(CoreError::InvalidInput("Räume benötigen eindeutige IDs/Nummern, endliche Fläche und gegebenenfalls positive Höhe".into()));
            }
            flaeche += r.flaeche_m2;
            volumen = volumen.zip(r.hoehe_m).map(|(v, h)| v + r.flaeche_m2 * h);
        }
        if !flaeche.is_finite() || volumen.is_some_and(|v| !v.is_finite()) {
            return Err(CoreError::InvalidInput(
                "Raumsummen außerhalb des Zahlenbereichs".into(),
            ));
        }
        Ok(RaumSummen {
            anzahl: self.raeume.len(),
            flaeche_m2: flaeche,
            volumen_m3: volumen,
        })
    }
}
async fn dokument(core: &CoreServices, p: ProjectId, d: &Dokumentbezug) -> CoreResult<()> {
    let meta = core
        .repos
        .documents
        .history(d.dokument_id)
        .await?
        .into_iter()
        .find(|v| v.version == d.version && v.project_id == p);
    if meta.is_none() {
        return Err(CoreError::InvalidInput(
            "Dokumentversion gehört nicht zu diesem Projekt".into(),
        ));
    }
    Ok(())
}
pub(super) async fn validieren(
    core: &CoreServices,
    p: &Project,
    art: ObjektArt,
    daten: serde_json::Value,
    wer: &Benutzer,
) -> CoreResult<serde_json::Value> {
    let neu = match art {
        ObjektArt::Gebaeude => {
            let g: Gebaeude = serde_json::from_value(daten).map_err(json_err)?;
            text(&g.bezeichnung, 500)?;
            if g.geschosse.len() > 200 || g.grundstueck_ids.len() > 100 {
                return Err(CoreError::InvalidInput(
                    "Zu viele Gebäudezuordnungen oder Geschosse".into(),
                ));
            }
            if !g.grundstueck_ids.is_empty() {
                let grund = crate::grundstueckskontext::ermitteln(p)?;
                for id in &g.grundstueck_ids {
                    if !grund.iter().any(|k| k.id == *id) {
                        return Err(CoreError::InvalidInput(
                            "Gebäude verweist auf unbekanntes Baugrundstück".into(),
                        ));
                    }
                }
            }
            let mut ids = std::collections::HashSet::new();
            for s in &g.geschosse {
                text(&s.bezeichnung, 500)?;
                if !ids.insert(s.id) {
                    return Err(CoreError::InvalidInput(
                        "Geschoss doppelt zugeordnet".into(),
                    ));
                }
                if s.vollgeschoss.is_some() {
                    text(&s.grundlage, 4000)?;
                }
            }
            if let Some(d) = &g.quelle {
                dokument(core, p.id, d).await?;
            }
            serde_json::to_value(g)
        }
        ObjektArt::Aufgaben => {
            let a: Aufgabe = serde_json::from_value(daten).map_err(json_err)?;
            text(&a.titel, 500)?;
            if a.beschreibung.len() > 10000 {
                return Err(CoreError::InvalidInput("Beschreibung zu lang".into()));
            }
            if let Some(id) = a.verantwortlich {
                let b = core.repos.benutzer.get(id).await?;
                if !b.aktiv {
                    return Err(CoreError::InvalidInput(
                        "Verantwortlicher Benutzer ist inaktiv".into(),
                    ));
                }
            }
            serde_json::to_value(a)
        }
        ObjektArt::Unterlagen => {
            let u: Unterlage = serde_json::from_value(daten).map_err(json_err)?;
            text(&u.bezeichnung, 500)?;
            if u.bemerkung.len() > 4000 {
                return Err(CoreError::InvalidInput("Bemerkung zu lang".into()));
            }
            if let Some(d) = &u.dokument {
                dokument(core, p.id, d).await?;
            } else if u.status != UnterlagenStatus::Entwurf {
                return Err(CoreError::InvalidInput(
                    "Unterlage ohne Dokument bleibt ein Entwurf".into(),
                ));
            }
            if u.status == UnterlagenStatus::Freigegeben {
                wer.pruefe(Recht::ProjektAnlegen)?;
            }
            serde_json::to_value(u)
        }
        ObjektArt::Dokumentdaten => {
            let d: Dokumentdaten = serde_json::from_value(daten).map_err(json_err)?;
            text(&d.dokumentart, 120)?;
            for s in [&d.plannummer, &d.revisionsindex, &d.massstab, &d.verfasser] {
                if s.len() > 500 {
                    return Err(CoreError::InvalidInput("Dokumentmetadaten zu lang".into()));
                }
            }
            dokument(core, p.id, &d.dokument).await?;
            serde_json::to_value(d)
        }
        ObjektArt::Raumbuch => {
            let r: Raumbuch = serde_json::from_value(daten).map_err(json_err)?;
            text(&r.bezeichnung, 500)?;
            r.summen()?;
            let g = core
                .repos
                .commit
                .fachstaende(p.id, "gebaeude")
                .await?
                .into_iter()
                .find(|g| g.objekt == r.gebaeude_id.to_string())
                .ok_or_else(|| {
                    CoreError::InvalidInput("Raumbuch benötigt ein Gebäude dieses Projekts".into())
                })?;
            let g: Gebaeude = serde_json::from_value(g.daten).map_err(json_err)?;
            if let Some(id) = r.geschoss_id
                && !g.geschosse.iter().any(|s| s.id == id)
            {
                return Err(CoreError::InvalidInput(
                    "Geschoss gehört nicht zum gewählten Gebäude".into(),
                ));
            }
            serde_json::to_value(r)
        }
    };
    neu.map_err(json_err)
}

impl CoreServices {
    /// Adapter vom Gebäudevertrag zu den bestehenden Flächeneingaben.
    pub(crate) async fn geschosszuordnung(
        &self,
        p: ProjectId,
        plan: &crate::baugrundstuecke::BaugrundstuecksPlan,
    ) -> CoreResult<crate::baugrundstuecke::BaugrundstuecksPlan> {
        let gebaeude = self.repos.commit.fachstaende(p, "gebaeude").await?;
        let mut neu = plan.clone();
        for b in &mut neu.berechnungen {
            if let Some(a) = &mut b.eingabe.aufstellung {
                for g in &mut a.geschosse {
                    if let Some(id) = &g.gebaeude_id {
                        let f = gebaeude.iter().find(|f| &f.objekt == id).ok_or_else(|| {
                            CoreError::InvalidInput(
                                "Geschoss verweist auf unbekanntes Gebäude".into(),
                            )
                        })?;
                        let geb: Gebaeude =
                            serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                        if let Some(id) = &g.geschoss_id {
                            let quelle = geb
                                .geschosse
                                .iter()
                                .find(|s| s.id.to_string() == *id)
                                .ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Geschoss gehört nicht zum gewählten Gebäude".into(),
                                    )
                                })?;
                            g.vollgeschoss = quelle.vollgeschoss.ok_or_else(|| {
                                CoreError::InvalidInput(
                                    "Vollgeschosseinstufung zuerst im Gebäude festlegen".into(),
                                )
                            })?;
                            g.bezeichnung = quelle.bezeichnung.clone();
                        } else if !geb.geschosse.is_empty() {
                            return Err(CoreError::InvalidInput(
                                "Geschoss aus dem Gebäudemodell auswählen".into(),
                            ));
                        }
                    } else if g.geschoss_id.is_some() || gebaeude.len() > 1 {
                        return Err(CoreError::InvalidInput(
                            "Geschosse den einzelnen Gebäuden zuordnen".into(),
                        ));
                    }
                }
            }
        }
        Ok(neu)
    }
}
