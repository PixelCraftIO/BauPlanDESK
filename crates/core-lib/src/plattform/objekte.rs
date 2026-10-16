use super::*;
use crate::{
    benutzer::BenutzerId,
    domain::{DocumentId, Project},
};
use chrono::NaiveDate;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Projektphase {
    Vorbereitung,
    Vorplanung,
    Entwurf,
    Genehmigung,
    Ausfuehrung,
    Abgeschlossen,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Projektorganisation {
    pub kennzeichen: String,
    pub phase: Projektphase,
    pub verantwortlich: Option<BenutzerId>,
    pub beschreibung: String,
}
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geometrie: Option<super::gebaeude::GebaeudeGeometrie>,
    pub bezeichnung: String,
    pub grundstueck_ids: Vec<Uuid>,
    pub geschosse: Vec<GebaeudeGeschoss>,
    pub quelle: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GebaeudeGeschoss {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geometrie: Option<super::gebaeude::GeschossGeometrie>,
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub einstufungsquelle: Option<crate::berechnungen::Einstufungsquelle>,
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
        ObjektArt::Stellplaetze
        | ObjektArt::Wohnflaechen
        | ObjektArt::Din277
        | ObjektArt::Vollgeschosse
        | ObjektArt::Ueberbaubarkeit
        | ObjektArt::Hoehen
        | ObjektArt::Freiflaechen
        | ObjektArt::Entwaesserung => {
            let plan: crate::berechnungen::FachPlan =
                serde_json::from_value(daten).map_err(json_err)?;
            if plan.modul.objekt() != art {
                return Err(CoreError::InvalidInput(
                    "Fachplan gehört zu einem anderen Modul".into(),
                ));
            }
            crate::berechnungen::berechnen(&plan)?;
            serde_json::to_value(plan)
        }
        ObjektArt::Abstandsflaechen => {
            let plan = serde_json::from_value(daten).map_err(json_err)?;
            return core.abstandsplan_validieren(p.id, plan, wer).await;
        }
        ObjektArt::Verfahren => {
            return Err(CoreError::InvalidInput(
                "Verfahren benötigen ihren eigenen Commit".into(),
            ));
        }
        ObjektArt::Projektorganisation => {
            let o: Projektorganisation = serde_json::from_value(daten).map_err(json_err)?;
            if o.kennzeichen.len() > 120 || o.beschreibung.len() > 10000 {
                return Err(CoreError::InvalidInput(
                    "Projektorganisation: Texte sind zu lang".into(),
                ));
            }
            if let Some(id) = o.verantwortlich
                && !core.repos.benutzer.get(id).await?.aktiv
            {
                return Err(CoreError::InvalidInput(
                    "Projektverantwortlicher ist inaktiv".into(),
                ));
            }
            serde_json::to_value(o)
        }
        ObjektArt::Gebaeude => {
            let g: Gebaeude = serde_json::from_value(daten).map_err(json_err)?;
            text(&g.bezeichnung, 500)?;
            g.grundlagen(Uuid::nil())?;
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
                core.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
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
        let projekt = self.get_project(p).await?;
        let grund = crate::grundstueckskontext::ermitteln(&projekt)?;
        let mut neu = plan.clone();
        for b in &mut neu.berechnungen {
            let grund_id = grund
                .iter()
                .find(|g| g.schluessel == b.schluessel)
                .map(|g| g.id);
            if let Some(a) = &mut b.eingabe.aufstellung {
                let mut bauteil_ids = std::collections::HashSet::new();
                for teil in &mut a.bauteile {
                    if let Some(id) = teil.gebaeude_id {
                        if !bauteil_ids.insert(id) {
                            return Err(CoreError::InvalidInput(
                                "Gebäudegrundfläche je Baugrundstück nur einmal übernehmen".into(),
                            ));
                        }
                        let f = gebaeude
                            .iter()
                            .find(|f| f.objekt == id.to_string())
                            .ok_or_else(|| {
                                CoreError::InvalidInput("Gebäudequelle für GRZ fehlt".into())
                            })?;
                        let g: Gebaeude =
                            serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                        if g.grundstueck_ids.len() != 1
                            || g.grundstueck_ids.first().copied() != grund_id
                        {
                            return Err(CoreError::InvalidInput("Flächenübernahme benötigt ein Gebäude mit eindeutiger Zuordnung zu diesem Baugrundstück; bei mehreren Grundstücken Flächen getrennt manuell aufteilen".into()));
                        }
                        let basis = g.grundlagen(id)?;
                        teil.flaeche_m2 = basis.grundflaeche_m2.ok_or_else(|| {
                            CoreError::InvalidInput(
                                "Gebäudegrundfläche in den Grundlagen ergänzen".into(),
                            )
                        })?;
                        teil.typ = basis.typ.ok_or_else(|| {
                            CoreError::InvalidInput("Gebäudetyp in den Grundlagen ergänzen".into())
                        })?;
                        teil.bezeichnung = g.bezeichnung;
                    }
                }
                let mut quellgeschosse = std::collections::HashSet::new();
                for g in &mut a.geschosse {
                    if g.flaeche_aus_grundlagen == Some(true)
                        && (g.gebaeude_id.is_none()
                            || !quellgeschosse
                                .insert((g.gebaeude_id.clone(), g.geschoss_id.clone())))
                    {
                        return Err(CoreError::InvalidInput("Übernommene Geschossfläche benötigt eine eindeutige Gebäude-/Geschosszuordnung".into()));
                    }
                    if let Some(id) = &g.gebaeude_id {
                        let f = gebaeude.iter().find(|f| &f.objekt == id).ok_or_else(|| {
                            CoreError::InvalidInput(
                                "Geschoss verweist auf unbekanntes Gebäude".into(),
                            )
                        })?;
                        let geb: Gebaeude =
                            serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                        if !geb.grundstueck_ids.is_empty()
                            && !grund_id.is_some_and(|id| geb.grundstueck_ids.contains(&id))
                        {
                            return Err(CoreError::InvalidInput(
                                "Geschoss gehört zu einem anderen Baugrundstück".into(),
                            ));
                        }
                        if g.flaeche_aus_grundlagen == Some(true) {
                            if geb.grundstueck_ids.len() != 1 {
                                return Err(CoreError::InvalidInput(
                                    "Geschossfläche für mehrere Grundstücke getrennt aufteilen"
                                        .into(),
                                ));
                            }
                            let basis = geb.grundlagen(f.objekt.parse().map_err(|_| {
                                CoreError::InvalidInput("Gebäude-ID ungültig".into())
                            })?)?;
                            let quelle = basis
                                .geschosse
                                .iter()
                                .find(|s| {
                                    g.geschoss_id.as_deref() == Some(s.id.to_string().as_str())
                                        || (s.implizit && g.geschoss_id.is_none())
                                })
                                .ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Quellgeschoss wurde geändert. Geschosse erneut übernehmen"
                                            .into(),
                                    )
                                })?;
                            g.aussenmass_m2 = quelle.flaeche_m2.ok_or_else(|| {
                                CoreError::InvalidInput(
                                    "Geschossfläche in den Grundlagen ergänzen".into(),
                                )
                            })?;
                        }
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
                            g.vollgeschoss = if let Some(q) = &quelle.einstufungsquelle {
                                let projekt = self.get_project(p).await?;
                                let grund = crate::grundstueckskontext::ermitteln(&projekt)?
                                    .into_iter()
                                    .find(|k| k.schluessel == b.schluessel)
                                    .map(|k| k.id);
                                self.geschosseinstufung_aus_nachweis(
                                    p,
                                    q,
                                    g.gebaeude_id.as_deref().unwrap_or("").parse().map_err(
                                        |_| CoreError::InvalidInput("Gebäude-ID ungültig".into()),
                                    )?,
                                    quelle.id,
                                    grund,
                                )
                                .await?
                                .0
                            } else {
                                quelle.vollgeschoss.ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Vollgeschosseinstufung zuerst im Gebäude festlegen".into(),
                                    )
                                })?
                            };
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
