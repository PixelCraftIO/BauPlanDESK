//! Unveränderliche Nachweisstände mit expliziten Eingaben, Regelarchiv und Freigabe.
use super::*;
use crate::{
    baugrundstuecke::BaugrundstuecksPlan,
    domain::{Bundesland, Planungsrecht, Project},
    lbo::{AbstandsflaecheInput, LboEngine, RulePackSource},
    storage::ContentHash,
};
use chrono::{DateTime, Utc};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum NutzungsArt {
    Grz,
    Gfz,
    Geschossigkeit,
    Bmz,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum NachweisEingabe {
    Fach {
        objekt: Uuid,
        plan: Box<crate::berechnungen::FachPlan>,
    },
    AbstandProjekt {
        objekt: Uuid,
        #[ts(type = "AbstandsPlan")]
        plan: Box<crate::abstandsflaechen::AbstandsPlan>,
    },
    Nutzung {
        fach: NutzungsArt,
        #[ts(type = "BaugrundstuecksPlan")]
        plan: BaugrundstuecksPlan,
    },
    Abstand {
        gegenstand: String,
        eingabe: AbstandsflaecheInput,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct RegelArchiv {
    pub id: String,
    pub version: String,
    #[ts(type = "string")]
    pub sha256: ContentHash,
    pub format: String,
    pub inhalt: String,
    pub pruefstatus: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachweisstand {
    pub id: Uuid,
    pub bezeichnung: String,
    pub erstellt_am: DateTime<Utc>,
    pub erstellt_von: Uuid,
    pub land: Bundesland,
    #[ts(type = "Planungsrecht")]
    pub rechtskontext: Planungsrecht,
    #[ts(type = "number")]
    pub projekt_version: i64,
    pub eingabe: NachweisEingabe,
    #[ts(type = "string")]
    pub grundlagen_hash: ContentHash,
    pub regeln: Vec<RegelArchiv>,
    #[ts(type = "string")]
    pub gebaeude_hash: ContentHash,
    #[ts(type = "string")]
    pub anfrage_hash: ContentHash,
    pub rechenstand: String,
    pub ergebnis: serde_json::Value,
    pub vollstaendig: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Freigabe {
    pub nachweis_id: Uuid,
    #[ts(type = "string")]
    pub grundlagen_hash: ContentHash,
    pub freigegeben_am: DateTime<Utc>,
    pub freigegeben_von: Uuid,
    pub bemerkung: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct NachweisAnsicht {
    pub stand: Nachweisstand,
    pub aktuell: bool,
    pub freigabe: Option<Freigabe>,
    /// Technischer Fehler: kein Beleg für eine fachliche Quellenänderung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub prueffehler: Option<String>,
}
/// Fehlende/ungültige Quellen sind fachlich veraltet. Infrastrukturfehler
/// bleiben explizit erkennbar und sperren weiterhin eine positive Freigabe.
fn pruefergebnis<T>(r: CoreResult<T>) -> Result<Option<T>, String> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(
            CoreError::InvalidInput(_) | CoreError::NotFound(_) | CoreError::RulePackMissing(_),
        ) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn hash(v: &serde_json::Value) -> CoreResult<ContentHash> {
    Ok(ContentHash::of(&serde_json::to_vec(v).map_err(json_err)?))
}
fn archiv(core: &CoreServices, p: &Project, e: &NachweisEingabe) -> CoreResult<Vec<RegelArchiv>> {
    match e {
        NachweisEingabe::Fach { plan, .. } => {
            let inhalt=serde_json::to_string(&serde_json::json!({"kontext":plan.regelkontext,"regeln":plan.eingaben.get("regeln")})).map_err(json_err)?;
            Ok(vec![RegelArchiv {
                id: plan.modul.objekt().modul().into(),
                version: plan.regelkontext.version.clone(),
                sha256: ContentHash::of(inhalt.as_bytes()),
                format: "fachregeln-json-1".into(),
                inhalt,
                pruefstatus: if plan.regelkontext.anwendbarkeit_bestaetigt {
                    "Projektbezogene Methodenanwendung bestätigt"
                } else {
                    "Anwendbarkeit offen"
                }
                .into(),
            }])
        }
        NachweisEingabe::AbstandProjekt { plan, .. } => {
            let inhalt = serde_json::to_string(&plan.regeln).map_err(json_err)?;
            Ok(vec![RegelArchiv {
                id: plan.regeln.id.clone(),
                version: plan.regeln.version.clone(),
                sha256: ContentHash::of(inhalt.as_bytes()),
                format: "abstand-json-1".into(),
                inhalt,
                pruefstatus: if plan.recht.geprueft {
                    "Projektbezogene Rechtsprüfung bestätigt"
                } else {
                    "Rechtsprüfung offen"
                }
                .into(),
            }])
        }
        NachweisEingabe::Abstand { .. } => {
            let inhalt = core.lbo.quelltext(p.bundesland)?;
            let info = core
                .lbo
                .rule_packs()
                .into_iter()
                .find(|r| r.land == p.bundesland)
                .ok_or(CoreError::RulePackMissing(p.bundesland))?;
            Ok(vec![RegelArchiv {
                id: format!("lbo-{}", p.bundesland.code()),
                version: info.version,
                sha256: ContentHash::of(inhalt.as_bytes()),
                format: "rhai".into(),
                inhalt,
                pruefstatus:
                    "Implementierter Regelansatz – unabhängige fachliche Freigabe erforderlich"
                        .into(),
            }])
        }
        NachweisEingabe::Nutzung { plan, .. } => {
            let mut regeln = Vec::new();
            for b in &plan.berechnungen {
                // Bei einem Bebauungsplan nur die Fassung des Plans, ohne Rückfall.
                let fassung = p
                    .planungsrecht
                    .massgebende_bau_nvo(b.eingabe.aufstellung.as_ref().map(|a| a.bau_nvo))
                    .ok_or_else(|| {
                        CoreError::InvalidInput(if p.planungsrecht.mit_bebauungsplan() {
                            "BauNVO festlegen: Fassung des Bebauungsplans in den Rechtsgrundlagen erfassen".into()
                        } else {
                            "BauNVO festlegen".into()
                        })
                    })?;
                let r = crate::baunvo::regelwerk(fassung);
                if regeln.iter().any(|a: &RegelArchiv| a.id == r.id) {
                    continue;
                }
                let inhalt = serde_json::to_string(&serde_json::to_value(r).map_err(json_err)?)
                    .map_err(json_err)?;
                regeln.push(RegelArchiv {
                    id: r.id.clone(),
                    version: r.version.clone(),
                    sha256: ContentHash::of(inhalt.as_bytes()),
                    inhalt,
                    format: "baunvo-json".into(),
                    pruefstatus:
                        "Implementierter Regelansatz – unabhängige fachliche Freigabe erforderlich"
                            .into(),
                });
            }
            Ok(regeln)
        }
    }
}
fn grundlagen(p: &Project, regeln: &[RegelArchiv]) -> CoreResult<ContentHash> {
    // Semantisch gleicher Bestands-/Neuvertrag behält denselben Fingerprint.
    let plan = crate::grundstuecksinformationen::plan(&p.planungsrecht)?;
    let mut daten = serde_json::json!({"land":p.bundesland,"bau_nvo":p.planungsrecht.bau_nvo,"recht":p.planungsrecht.rechtsgrundlage,"plan":p.planungsrecht.bebauungsplan_art,"ergaenzung":p.planungsrecht.ergaenzende_grundlage,"plan_bezeichnung":p.planungsrecht.plan_bezeichnung,"plan_nummer":p.planungsrecht.plan_nummer,"rechtskraft":p.planungsrecht.plan_rechtskraft,"festsetzungen":p.planungsrecht.festgesetzte_werte,"grundstuecke":plan,"legacyflaeche":if plan.is_none(){p.grundstuecksflaeche_m2}else{None},"regeln":regeln.iter().filter(|r|r.format!="abstandsquellen-json-1").map(|r|(&r.id,&r.version,&r.sha256)).collect::<Vec<_>>() });
    if !crate::grundstuecksinformationen::quellen_aktuell(&p.planungsrecht)? {
        daten["berechnungsgrundlage_unbestaetigt"] = serde_json::json!(true);
    }
    // Ohne Plan werden Orientierungswerte seit Maßstab 2 nicht mehr als Verstoß gewertet;
    // ältere Nachweise ohne Plan gelten deshalb als veraltet. Planprojekte behalten ihren Stand.
    let r = &p.planungsrecht;
    let mit_plan = !matches!(
        r.bebauungsplan_art,
        crate::domain::BebauungsplanArt::Ungeklaert | crate::domain::BebauungsplanArt::Kein
    );
    if !mit_plan && r.rechtsgrundlage != Some(crate::domain::Rechtsgrundlage::Par35) {
        daten["massstab_ohne_plan"] = serde_json::json!({"version": 2, "umgebung": r.umgebung});
    }
    // Maßstab 3 im Plan-Fall: Ohne BauNVO-Fassung kein Urteil, Baufensterfläche nur
    // Information, Lage aus den Baugrenzen, beim einfachen Plan ergänzend die Umgebung.
    // Nur betroffene Projekte erhalten einen neuen Fingerprint.
    if mit_plan {
        if r.bau_nvo.is_none() {
            daten["massstab_plan_ohne_fassung"] = serde_json::json!(3);
        }
        if r.festgesetzte_werte.ueberbaubare_flaeche_m2.is_some() {
            daten["baufensterflaeche_ohne_urteil"] = serde_json::json!(3);
        }
        if let Some(g) = &r.baugrenzen {
            daten["baugrenzen"] = serde_json::to_value(g).map_err(json_err)?;
        }
        if r.bebauungsplan_art == crate::domain::BebauungsplanArt::Einfach
            && r.ergaenzende_grundlage == Some(crate::domain::Rechtsgrundlage::Par34)
        {
            daten["umgebung_ergaenzend"] =
                serde_json::json!({"version": 3, "umgebung": r.umgebung});
        }
    }
    hash(&daten)
}

/// Höhenangaben eines im Abstandsplan verknüpften Gebäudes zum Berechnungszeitpunkt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AbstandsGebaeudehoehen {
    pub bezeichnung: String,
    pub nullpunkt_m_nhn: Option<f64>,
    pub gelaende_m: Option<f64>,
    pub traufe_m: Option<f64>,
    pub first_m: Option<f64>,
    /// Dachform und -neigung; ohne Angabe nicht im Archiv, damit ältere
    /// Nachweise ohne Dachangaben unverändert aktuell bleiben.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dachform: Option<super::gebaeude::Dachform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dachneigung_grad: Option<f64>,
}
pub(crate) const ABSTANDSGEBAEUDE: &str = "abstandsgebaeude-json-1";
/// Archiv der Gebäudehöhen, auf die sich die Umrisse und Wände eines Abstandsplans
/// beziehen. Die Höhen sind im Plan nur Kopien; ändert sich ein verknüpftes Gebäude
/// (oder fehlt es), ist der Nachweis veraltet. Ältere Nachweise ohne diesen Eintrag
/// bleiben unverändert aktuell.
fn abstandsgebaeude(
    plan: &crate::abstandsflaechen::AbstandsPlan,
    gebaeude: &[Fachstand],
) -> CoreResult<RegelArchiv> {
    let ids = plan
        .umrisse
        .iter()
        .filter_map(|u| u.gebaeude_id)
        .chain(plan.waende.iter().filter_map(|w| w.gebaeude_id))
        .map(|id| id.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let mut stand = std::collections::BTreeMap::new();
    for id in ids {
        let g = gebaeude
            .iter()
            .find(|f| f.objekt == id)
            .and_then(|f| serde_json::from_value::<super::objekte::Gebaeude>(f.daten.clone()).ok())
            .map(|g| {
                let geo = g.geometrie.unwrap_or_default();
                AbstandsGebaeudehoehen {
                    bezeichnung: g.bezeichnung,
                    nullpunkt_m_nhn: geo.nullpunkt_m_nhn,
                    gelaende_m: geo.gelaende_m,
                    traufe_m: geo.traufe_m,
                    first_m: geo.first_m,
                    dachform: geo.dachform,
                    dachneigung_grad: geo.dachneigung_grad,
                }
            });
        stand.insert(id, g);
    }
    let inhalt = serde_json::to_string(&stand).map_err(json_err)?;
    Ok(RegelArchiv {
        id: "abstandsgebaeude".into(),
        version: "1".into(),
        sha256: ContentHash::of(inhalt.as_bytes()),
        format: ABSTANDSGEBAEUDE.into(),
        inhalt,
        pruefstatus: "Höhenangaben der verknüpften Gebäude zum Berechnungszeitpunkt".into(),
    })
}
fn abstandsflurstuecke(
    p: &Project,
    plan: &crate::abstandsflaechen::AbstandsPlan,
) -> Vec<crate::grundstuecksinformationen::FlurstuecksInfo> {
    let ids = plan
        .grundstuecke
        .iter()
        .flat_map(|g| &g.flurstueck_ids)
        .collect::<std::collections::HashSet<_>>();
    let mut infos = crate::grundstuecksinformationen::informationen(&p.planungsrecht)
        .flurstuecke
        .into_iter()
        .filter(|f| ids.contains(&f.id))
        .collect::<Vec<_>>();
    infos.sort_by(|a, b| a.id.cmp(&b.id));
    infos
}
fn abstandsgrundlagen(
    p: &Project,
    regeln: &[RegelArchiv],
    flurstuecke: serde_json::Value,
) -> CoreResult<ContentHash> {
    hash(
        &serde_json::json!({"land":p.bundesland,"flurstuecke":flurstuecke,"planrechtskontext":(&p.planungsrecht.rechtsgrundlage,&p.planungsrecht.plan_bezeichnung,&p.planungsrecht.plan_nummer,&p.planungsrecht.plan_rechtskraft,&p.planungsrecht.ergaenzende_grundlage),"regeln":regeln.iter().filter(|r|r.format!="abstandsquellen-json-1"&&r.format!=ABSTANDSGEBAEUDE).map(|r|(&r.id,&r.version,&r.sha256)).collect::<Vec<_>>()}),
    )
}
/// Grundlagenhash älterer Abstandsnachweise über die vollständigen Flurstücksangaben.
/// Sie bleiben aktuell, solange sich daran nichts ändert.
fn grundlagen_abstand_alt(
    p: &Project,
    regeln: &[RegelArchiv],
    e: &NachweisEingabe,
) -> CoreResult<Option<ContentHash>> {
    let NachweisEingabe::AbstandProjekt { plan, .. } = e else {
        return Ok(None);
    };
    let infos = abstandsflurstuecke(p, plan);
    abstandsgrundlagen(p, regeln, serde_json::to_value(infos).map_err(json_err)?).map(Some)
}
fn grundlagen_fuer(
    p: &Project,
    regeln: &[RegelArchiv],
    e: &NachweisEingabe,
) -> CoreResult<ContentHash> {
    if let NachweisEingabe::Fach { .. } = e {
        return hash(
            &serde_json::json!({"land":p.bundesland,"regeln":regeln.iter().map(|r|(&r.id,&r.version,&r.sha256)).collect::<Vec<_>>()}),
        );
    }
    if let NachweisEingabe::AbstandProjekt { plan, .. } = e {
        // Nur Identität, Fläche und Rolle der Flurstücke sind Rechengrundlage;
        // Grundbuch-, Eigentümer- und Belegangaben ändern das Ergebnis nicht.
        let flurstuecke = abstandsflurstuecke(p, plan)
            .into_iter()
            .map(|f| {
                serde_json::json!({"id":f.id,"gemarkung":f.gemarkung,"gemarkungsnummer":f.gemarkungsnummer,"flur":f.flur,"nummer":f.nummer,"flaeche_m2":f.flaeche_m2,"rolle":f.rolle})
            })
            .collect::<Vec<_>>();
        // Das festgesetzte Baugebiet geht ab Rechenstand 3 in den Projektabgleich ein.
        abstandsgrundlagen(
            p,
            regeln,
            serde_json::json!({"flurstuecke":flurstuecke,"baugebiet":p.planungsrecht.festgesetzte_werte.baugebiet}),
        )
    } else {
        grundlagen(p, regeln)
    }
}
fn auswerten(
    core: &CoreServices,
    p: &Project,
    e: &NachweisEingabe,
) -> CoreResult<(serde_json::Value, bool)> {
    match e {
        NachweisEingabe::Fach { plan, .. } => {
            let r = crate::berechnungen::berechnen(plan)?;
            let voll = r.vollstaendig;
            Ok((serde_json::to_value(r).map_err(json_err)?, voll))
        }
        NachweisEingabe::AbstandProjekt { plan, .. } => {
            let mut r = crate::abstandsflaechen::berechnen(plan)?;
            crate::abstandsflaechen::projektabgleich(plan, &p.planungsrecht, &mut r);
            let voll = r.vollstaendig && r.konfliktfrei;
            Ok((serde_json::to_value(r).map_err(json_err)?, voll))
        }
        NachweisEingabe::Abstand {
            gegenstand,
            eingabe,
        } => {
            text(gegenstand, 500)?;
            let r = core.lbo.abstandsflaeche(p.bundesland, eingabe)?;
            Ok((serde_json::to_value(r).map_err(json_err)?, true))
        }
        NachweisEingabe::Nutzung { fach, plan } => {
            let r = plan.pruefe(&p.planungsrecht, p.version)?;
            let voll = !r.baugrundstuecke.is_empty()
                && r.baugrundstuecke.iter().all(|g| match fach {
                    NutzungsArt::Grz => g
                        .pruefung
                        .aufstellung
                        .as_ref()
                        .is_some_and(|a| a.bauteile_vorhanden),
                    NutzungsArt::Gfz | NutzungsArt::Geschossigkeit => g
                        .pruefung
                        .aufstellung
                        .as_ref()
                        .is_some_and(|a| a.geschosse_vorhanden),
                    NutzungsArt::Bmz => g.pruefung.kennzahlen.bmz.is_some(),
                });
            Ok((serde_json::to_value(r).map_err(json_err)?, voll))
        }
    }
}
impl CoreServices {
    async fn nutzungs_planquellen_pruefen(
        &self,
        pid: ProjectId,
        e: &NachweisEingabe,
        gebaeude: &[Fachstand],
    ) -> CoreResult<()> {
        let ids = gebaeude_quellids(pid, e, gebaeude);
        for f in gebaeude.iter().filter(|f| ids.contains(&f.objekt)) {
            let g: objekte::Gebaeude = serde_json::from_value(f.daten.clone()).map_err(json_err)?;
            self.gebaeude_planquellen_pruefen(
                pid,
                f.objekt
                    .parse()
                    .map_err(|_| CoreError::InvalidInput("Gebäude-ID ungültig".into()))?,
                &g,
            )
            .await?;
        }
        // Abzugsflächen und GRZ-Bauteile des Nachweisplans mit Aufmaßbindung.
        if let NachweisEingabe::Nutzung { plan, .. } = e {
            use crate::grundstuecksinformationen::{BaugrundstuecksErmittlung, FlurstuecksAnsatz};
            let ermittlung = BaugrundstuecksErmittlung {
                ansaetze: plan
                    .flurstuecke
                    .iter()
                    .map(|f| FlurstuecksAnsatz {
                        flurstueck_id: f.id.clone(),
                        abzuege: f.abzuege.clone(),
                        abweichende_zuordnung: f.abweichende_zuordnung.clone(),
                    })
                    .collect(),
                berechnungen: plan.berechnungen.clone(),
                quellen_hash: None,
            };
            self.grundstueck_planquellen_pruefen(pid, &ermittlung)
                .await?;
        }
        Ok(())
    }
    async fn gebaeude_hash(&self, pid: ProjectId, e: &NachweisEingabe) -> CoreResult<ContentHash> {
        if let NachweisEingabe::Fach { objekt, plan } = e {
            return self.berechnungs_fingerprint(pid, *objekt, plan).await;
        }
        if let NachweisEingabe::AbstandProjekt { plan, .. } = e {
            return hash(&serde_json::to_value(plan).map_err(json_err)?);
        }
        let gebaeude = self.repos.commit.fachstaende(pid, "gebaeude").await?;
        self.nutzungs_planquellen_pruefen(pid, e, &gebaeude).await?;
        gebaeude_fingerprint(pid, e, &gebaeude)
    }
    pub async fn nachweis_erstellen(
        &self,
        pid: ProjectId,
        id: Uuid,
        bezeichnung: String,
        eingabe: NachweisEingabe,
        wer: &Benutzer,
    ) -> CoreResult<Nachweisstand> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        text(&bezeichnung, 500)?;
        let mut eingabe = eingabe;
        if let NachweisEingabe::Fach { plan, .. } = &mut eingabe {
            crate::berechnungen::normalisieren(plan)?;
        }
        let anfrage_hash = hash(&serde_json::to_value(&eingabe).map_err(json_err)?)?;
        if let Some(f) = self
            .repos
            .commit
            .fachstaende(pid, "nachweise")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
        {
            let alt: Nachweisstand = serde_json::from_value(f.daten).map_err(json_err)?;
            if alt.anfrage_hash == anfrage_hash
                && alt.bezeichnung == bezeichnung
                && alt.erstellt_von == wer.id.0
            {
                return Ok(alt);
            }
            return Err(CoreError::InvalidInput(
                "Vorgangskennung bereits für einen anderen Nachweis verwendet".into(),
            ));
        }
        let eingabe = match eingabe {
            NachweisEingabe::Nutzung { fach, plan } => NachweisEingabe::Nutzung {
                fach,
                plan: self.geschosszuordnung(pid, &plan).await?,
            },
            e => e,
        };
        if let NachweisEingabe::Fach { objekt, plan } = &eingabe {
            self.projekt_zugriff(pid, wer, crate::zugriff::ProjektRecht::Bearbeiten)
                .await?;
            let (kanonisch, _) = self.berechnung_pruefen(pid, *plan.clone()).await?;
            let f = self
                .fachobjekte(pid, plan.modul.objekt())
                .await?
                .into_iter()
                .find(|f| f.objekt == objekt.to_string())
                .ok_or_else(|| {
                    CoreError::InvalidInput("Berechnungsentwurf zuerst speichern".into())
                })?;
            if f.schema_version != 1
                || f.daten != serde_json::to_value(&kanonisch).map_err(json_err)?
                || kanonisch != **plan
            {
                return Err(CoreError::InvalidInput(
                    "Berechnungsentwurf / Quellen geändert. Aktuellen Stand prüfen und speichern."
                        .into(),
                ));
            }
        }
        if let NachweisEingabe::AbstandProjekt { objekt, plan } = &eingabe {
            self.projekt_zugriff(pid, wer, crate::zugriff::ProjektRecht::Bearbeiten)
                .await?;
            self.abstandsplan_pruefen(pid, plan).await?;
            let gespeichert = self
                .repos
                .commit
                .fachstaende(pid, "abstandsflaechen")
                .await?
                .into_iter()
                .find(|f| f.objekt == objekt.to_string())
                .ok_or_else(|| {
                    CoreError::InvalidInput("Abstandsplan zuerst im Projekt speichern".into())
                })?;
            if gespeichert.daten != serde_json::to_value(plan).map_err(json_err)? {
                return Err(CoreError::InvalidInput(
                    "Abstandsplan wurde geändert; aktuellen Entwurf zuerst speichern".into(),
                ));
            }
        }
        let gebaeude_hash = self.gebaeude_hash(pid, &eingabe).await?;
        let p = self.repos.projects.get(pid).await?;
        if let NachweisEingabe::Nutzung { plan, .. } = &eingabe {
            crate::grundstuecksinformationen::pruefe_bestaetigte_ermittlung(
                &p.planungsrecht,
                plan,
            )?;
        }
        let mut regeln = archiv(self, &p, &eingabe)?;
        if let NachweisEingabe::AbstandProjekt { plan, .. } = &eingabe {
            let mut docs = Vec::new();
            for d in plan.dokumente() {
                docs.push(
                    self.document_history(d.dokument_id)
                        .await?
                        .into_iter()
                        .find(|v| v.version == d.version && v.project_id == pid)
                        .ok_or_else(|| CoreError::InvalidInput("Quelldokument fehlt".into()))?,
                );
            }
            let inhalt = serde_json::to_string(&docs).map_err(json_err)?;
            regeln.push(RegelArchiv {
                id: "abstandsquellen".into(),
                version: "1".into(),
                sha256: ContentHash::of(inhalt.as_bytes()),
                format: "abstandsquellen-json-1".into(),
                inhalt,
                pruefstatus: "Originalquellen mit konkreten Dateiversionen und SHA-256".into(),
            });
            let gebaeude = self.repos.commit.fachstaende(pid, "gebaeude").await?;
            regeln.push(abstandsgebaeude(plan, &gebaeude)?);
        }
        let grundlagen_hash = grundlagen_fuer(&p, &regeln, &eingabe)?;
        let (ergebnis, vollstaendig) = auswerten(self, &p, &eingabe)?;
        let n = Nachweisstand {
            id,
            bezeichnung,
            erstellt_am: Utc::now(),
            erstellt_von: wer.id.0,
            land: p.bundesland,
            rechtskontext: p.planungsrecht.clone(),
            projekt_version: p.version,
            eingabe,
            grundlagen_hash,
            regeln,
            gebaeude_hash,
            anfrage_hash,
            rechenstand: "plattform-1".into(),
            ergebnis,
            vollstaendig,
        };
        let f = Fachstand {
            projekt_id: pid,
            modul: "nachweise".into(),
            objekt: id.to_string(),
            schema_version: 1,
            revision: 0,
            daten: serde_json::to_value(&n).map_err(json_err)?,
        };
        let audit = Self::audit(
            wer,
            Aktion::ProjektGeaendert,
            Some(pid),
            None,
            format!("Nachweis {} archiviert", n.bezeichnung),
        );
        match self.repos.commit.fachstand(f, 0, audit).await {
            Ok(_) => Ok(n),
            Err(CoreError::VersionConflict { .. }) => {
                let f = self
                    .repos
                    .commit
                    .fachstaende(pid, "nachweise")
                    .await?
                    .into_iter()
                    .find(|f| f.objekt == id.to_string())
                    .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
                let alt: Nachweisstand = serde_json::from_value(f.daten).map_err(json_err)?;
                if alt.anfrage_hash == n.anfrage_hash
                    && alt.erstellt_von == n.erstellt_von
                    && alt.bezeichnung == n.bezeichnung
                {
                    Ok(alt)
                } else {
                    Err(CoreError::InvalidInput(
                        "Vorgangskennung mit anderem Inhalt".into(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }
    pub async fn nachweise(&self, pid: ProjectId) -> CoreResult<Vec<NachweisAnsicht>> {
        let start = std::time::Instant::now();
        let (sicht, quellen) = self.lesesitzung(pid).await?;
        let ergebnis = sicht.nachweise_auswerten(pid).await;
        if ergebnis.is_ok() {
            quellen.unveraendert().await?;
        }
        tracing::debug!(
            vorgang = "nachweise",
            dauer_ms = start.elapsed().as_millis() as u64,
            anzahl = ergebnis.as_ref().map(|n| n.len()).unwrap_or(0),
            erfolgreich = ergebnis.is_ok()
        );
        ergebnis
    }
    async fn nachweise_auswerten(&self, pid: ProjectId) -> CoreResult<Vec<NachweisAnsicht>> {
        let p = self.repos.projects.get(pid).await?;
        let freigaben = self.repos.commit.fachstaende(pid, "freigaben").await?;
        let gebaeude = self.repos.commit.fachstaende(pid, "gebaeude").await?;
        let dateien = self.list_documents(pid).await?;
        let abstandsplaene = self
            .repos
            .commit
            .fachstaende(pid, "abstandsflaechen")
            .await?;
        let staende = self.repos.commit.fachstaende(pid, "nachweise").await?;
        let mut prueffehler = std::collections::BTreeMap::new();
        let mut abstandsfehler = std::collections::BTreeMap::new();
        let mut abstandsquellen = std::collections::BTreeMap::new();
        for f in &abstandsplaene {
            let plan: crate::abstandsflaechen::AbstandsPlan =
                serde_json::from_value(f.daten.clone()).map_err(json_err)?;
            let status = pruefergebnis(self.abstands_planquellen_pruefen(pid, &plan).await);
            if let Err(e) = &status {
                abstandsfehler.insert(f.objekt.clone(), e.clone());
            }
            abstandsquellen.insert(f.objekt.clone(), status.is_ok_and(|v| v.is_some()));
        }
        let mut fach_fingerprints = std::collections::BTreeMap::new();
        let mut nutzungsquellen = std::collections::BTreeMap::new();
        for f in &staende {
            let n: Nachweisstand = serde_json::from_value(f.daten.clone()).map_err(json_err)?;
            if matches!(&n.eingabe, NachweisEingabe::Nutzung { .. }) {
                nutzungsquellen.insert(
                    n.id,
                    match pruefergebnis(
                        self.nutzungs_planquellen_pruefen(pid, &n.eingabe, &gebaeude)
                            .await,
                    ) {
                        Ok(v) => v.is_some(),
                        Err(e) => {
                            prueffehler.insert(n.id, e);
                            false
                        }
                    },
                );
            }
            if let NachweisEingabe::Fach { objekt, plan } = &n.eingabe {
                fach_fingerprints.insert(
                    n.id,
                    match pruefergebnis(self.berechnungs_fingerprint(pid, *objekt, plan).await) {
                        Ok(v) => v,
                        Err(e) => {
                            prueffehler.insert(n.id, e);
                            None
                        }
                    },
                );
            }
        }
        let neueste_dateien = dateien
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut m, d| {
                m.entry(d.document_id.0)
                    .and_modify(|v: &mut i64| *v = (*v).max(d.version))
                    .or_insert(d.version);
                m
            });
        let freigaben = freigaben
            .into_iter()
            .map(|f| (f.objekt.clone(), f))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut ergebnisse = staende
            .into_iter()
            .map(|f| {
                let stand: Nachweisstand = serde_json::from_value(f.daten).map_err(json_err)?;
                let grundlagen = pruefergebnis(archiv(self, &p, &stand.eingabe).and_then(|r| {
                    Ok(
                        grundlagen_fuer(&p, &r, &stand.eingabe)? == stand.grundlagen_hash
                            || grundlagen_abstand_alt(&p, &r, &stand.eingabe)?
                                == Some(stand.grundlagen_hash.clone()),
                    )
                }));
                let grundlagenfehler = grundlagen.as_ref().err().cloned();
                let aktuell = grundlagen.ok().flatten().unwrap_or(false)
                    && match &stand.eingabe {
                        NachweisEingabe::Fach { plan, .. } => {
                            stand.ergebnis["rechenstand"] == plan.modul.rechenstand()
                                && fach_fingerprints
                                    .get(&stand.id)
                                    .and_then(|v| v.as_ref())
                                    .is_some_and(|h| *h == stand.gebaeude_hash)
                                && plan.quellen.iter().all(|q| {
                                    neueste_dateien.get(&q.dokument_id.0) == Some(&q.version)
                                })
                        }
                        NachweisEingabe::AbstandProjekt { objekt, plan } => {
                            abstandsquellen.get(&objekt.to_string()) == Some(&true)
                                && stand
                                    .regeln
                                    .iter()
                                    .filter(|r| r.format == ABSTANDSGEBAEUDE)
                                    .all(|r| {
                                        abstandsgebaeude(plan, &gebaeude)
                                            .is_ok_and(|a| a.sha256 == r.sha256)
                                    })
                                && abstandsplaene
                                    .iter()
                                    .find(|f| f.objekt == objekt.to_string())
                                    .is_some_and(|f| {
                                        hash(&f.daten).is_ok_and(|h| h == stand.gebaeude_hash)
                                    })
                                && plan.dokumente().iter().all(|d| {
                                    neueste_dateien.get(&d.dokument_id.0) == Some(&d.version)
                                })
                        }
                        _ => {
                            nutzungsquellen.get(&stand.id) != Some(&false)
                                && gebaeude_fingerprint(pid, &stand.eingabe, &gebaeude)
                                    .is_ok_and(|h| h == stand.gebaeude_hash)
                        }
                    };
                let freigabe = freigaben
                    .get(&stand.id.to_string())
                    .map(|f| serde_json::from_value::<Freigabe>(f.daten.clone()).map_err(json_err))
                    .transpose()?;
                let fehler = match &stand.eingabe {
                    NachweisEingabe::AbstandProjekt { objekt, .. } => {
                        abstandsfehler.get(&objekt.to_string()).cloned()
                    }
                    _ => prueffehler.get(&stand.id).cloned(),
                }
                .or(grundlagenfehler);
                Ok(NachweisAnsicht {
                    stand,
                    aktuell: aktuell && fehler.is_none(),
                    freigabe,
                    prueffehler: fehler,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        // Rückwärtskanten: jede veraltete/fehlende Quelle wird einmal propagiert.
        let mut abhaengigkeiten = std::collections::BTreeMap::<Uuid, Vec<usize>>::new();
        let vorhanden = ergebnisse
            .iter()
            .map(|n| n.stand.id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut queue = std::collections::VecDeque::new();
        for (index, n) in ergebnisse.iter().enumerate() {
            let mut quellen = Vec::new();
            match &n.stand.eingabe {
                NachweisEingabe::Fach { plan, .. } => {
                    quellen.extend(plan.uebernahmen.iter().map(|u| u.nachweis_id))
                }
                NachweisEingabe::Nutzung { plan, .. } => {
                    for b in &plan.berechnungen {
                        if let Some(a) = &b.eingabe.aufstellung {
                            for s in &a.geschosse {
                                if let (Some(gid), Some(sid)) = (&s.gebaeude_id, &s.geschoss_id)
                                    && let Some(g) = gebaeude.iter().find(|g| g.objekt == *gid)
                                {
                                    let g: objekte::Gebaeude =
                                        serde_json::from_value(g.daten.clone())
                                            .map_err(json_err)?;
                                    if let Some(q) = g
                                        .geschosse
                                        .iter()
                                        .find(|s| s.id.to_string() == *sid)
                                        .and_then(|s| s.einstufungsquelle.as_ref())
                                    {
                                        quellen.push(q.nachweis_id);
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            if !n.aktuell {
                queue.push_back(n.stand.id);
            }
            for q in quellen {
                abhaengigkeiten.entry(q).or_default().push(index);
                if !vorhanden.contains(&q) {
                    queue.push_back(q);
                }
            }
        }
        let mut technische_fehler = ergebnisse
            .iter()
            .filter(|n| n.prueffehler.is_some())
            .map(|n| n.stand.id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut besucht = std::collections::BTreeSet::new();
        while let Some(id) = queue.pop_front() {
            if !besucht.insert((id, technische_fehler.contains(&id))) {
                continue;
            }
            for &index in abhaengigkeiten.get(&id).into_iter().flatten() {
                let n = &mut ergebnisse[index];
                if n.prueffehler.is_none() && technische_fehler.contains(&id) {
                    technische_fehler.insert(n.stand.id);
                    n.prueffehler =
                        Some("Aktualität einer abhängigen Quelle technisch nicht prüfbar".into());
                }
                n.aktuell = false;
                queue.push_back(n.stand.id);
            }
        }
        Ok(ergebnisse)
    }
    pub async fn nachweis_freigeben(
        &self,
        pid: ProjectId,
        id: Uuid,
        bemerkung: String,
        wer: &Benutzer,
    ) -> CoreResult<Freigabe> {
        self.pruefe(wer, Recht::ProjektAnlegen).await?;
        if bemerkung.len() > 4000 {
            return Err(CoreError::InvalidInput("Bemerkung zu lang".into()));
        }
        let n = self
            .nachweise(pid)
            .await?
            .into_iter()
            .find(|n| n.stand.id == id)
            .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
        if !n.aktuell || !n.stand.vollstaendig {
            return Err(CoreError::InvalidInput(
                "Nur aktuelle, vollständige Nachweisstände freigeben".into(),
            ));
        }
        if let Some(f) = n.freigabe {
            return Ok(f);
        }
        let f = Freigabe {
            nachweis_id: id,
            grundlagen_hash: n.stand.grundlagen_hash,
            freigegeben_am: Utc::now(),
            freigegeben_von: wer.id.0,
            bemerkung,
        };
        self.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: pid,
                    modul: "freigaben".into(),
                    objekt: id.to_string(),
                    schema_version: 1,
                    revision: 0,
                    daten: serde_json::to_value(&f).map_err(json_err)?,
                },
                0,
                Self::audit(
                    wer,
                    Aktion::ProjektGeaendert,
                    Some(pid),
                    None,
                    format!("Nachweis {id} fachlich freigegeben"),
                ),
            )
            .await?;
        Ok(f)
    }
    pub async fn raumbuch_summen(
        &self,
        pid: ProjectId,
        id: Uuid,
    ) -> CoreResult<objekte::RaumSummen> {
        let f = self
            .repos
            .commit
            .fachstaende(pid, "raumbuch")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
        let r = serde_json::from_value::<objekte::Raumbuch>(f.daten).map_err(json_err)?;
        self.raumbuch_planquellen_pruefen(pid, &r).await?;
        r.summen()
    }
    /// Wiederholung nutzt den archivierten Rechts- und Skriptstand, nicht den aktiven Katalog.
    pub async fn nachweis_wiederholen(
        &self,
        pid: ProjectId,
        id: Uuid,
    ) -> CoreResult<serde_json::Value> {
        let f = self
            .repos
            .commit
            .fachstaende(pid, "nachweise")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| CoreError::NotFound(id.to_string()))?;
        let n: Nachweisstand = serde_json::from_value(f.daten).map_err(json_err)?;
        if n.rechenstand != "plattform-1" {
            return Err(CoreError::InvalidInput(
                "Die Wiederholung benötigt den archivierten Rechenkernstand".into(),
            ));
        }
        for r in &n.regeln {
            r.sha256.verify(r.inhalt.as_bytes())?;
        }
        let p = Project {
            id: pid,
            name: String::new(),
            bundesland: n.land,
            grundstuecksflaeche_m2: None,
            version: n.projekt_version,
            created_at: n.erstellt_am,
            updated_at: n.erstellt_am,
            planungsrecht: n.rechtskontext,
        };
        match &n.eingabe {
            NachweisEingabe::Fach { plan, .. } => {
                let r = n
                    .regeln
                    .iter()
                    .find(|r| r.format == "fachregeln-json-1")
                    .ok_or_else(|| {
                        CoreError::InvalidInput("Archivierte Fachregeln fehlen".into())
                    })?;
                let inhalt = serde_json::json!({"kontext":plan.regelkontext,"regeln":plan.eingaben.get("regeln")});
                if serde_json::from_str::<serde_json::Value>(&r.inhalt).map_err(json_err)? != inhalt
                    || n.ergebnis["rechenstand"] != plan.modul.rechenstand()
                {
                    return Err(CoreError::InvalidInput(
                        "Archivierter Rechen-/Regelstand passt nicht".into(),
                    ));
                }
                serde_json::to_value(crate::berechnungen::berechnen(plan)?).map_err(json_err)
            }
            NachweisEingabe::AbstandProjekt { plan, .. } => {
                let r = n
                    .regeln
                    .iter()
                    .find(|r| r.format == "abstand-json-1")
                    .ok_or_else(|| {
                        CoreError::InvalidInput("Archivierte Abstandsregeln fehlen".into())
                    })?;
                let regel: crate::abstandsflaechen::AbstandsRegeln =
                    serde_json::from_str(&r.inhalt).map_err(json_err)?;
                // Pläne ohne Höhenprofil und Wandhöhenansatz rechnet der
                // aktuelle Kern wie die älteren Rechenstände.
                let stand = n.ergebnis["rechenstand"].as_str().unwrap_or_default();
                if regel != plan.regeln
                    || (stand != crate::abstandsflaechen::RECHENSTAND
                        && !crate::abstandsflaechen::RECHENSTAENDE_KOMPATIBEL.contains(&stand))
                {
                    return Err(CoreError::InvalidInput(
                        "Archivierter Rechen-/Regelstand passt nicht zum verfügbaren Kern".into(),
                    ));
                }
                let mut r = crate::abstandsflaechen::berechnen_stand(plan, stand)?;
                crate::abstandsflaechen::projektabgleich(plan, &p.planungsrecht, &mut r);
                serde_json::to_value(r).map_err(json_err)
            }
            NachweisEingabe::Abstand { eingabe, .. } => {
                let engine = LboEngine::new();
                let r = n
                    .regeln
                    .iter()
                    .find(|r| r.format == "rhai")
                    .ok_or_else(|| {
                        CoreError::InvalidInput("Archiviertes LBO-Paket fehlt".into())
                    })?;
                engine.load_rule_pack(RulePackSource {
                    land: n.land,
                    script: r.inhalt.clone(),
                    origin: "Nachweisarchiv".into(),
                })?;
                let mut ergebnis = serde_json::to_value(engine.abstandsflaeche(n.land, eingabe)?)
                    .map_err(json_err)?;
                ergebnis["regelpaket"]["origin"] = n.ergebnis["regelpaket"]["origin"].clone();
                Ok(ergebnis)
            }
            NachweisEingabe::Nutzung { .. } => {
                let regeln = archivierte_baunvo(&n.regeln)?;
                let NachweisEingabe::Nutzung { plan, .. } = &n.eingabe else {
                    unreachable!()
                };
                serde_json::to_value(plan.pruefe_mit_regeln(
                    &p.planungsrecht,
                    p.version,
                    &regeln,
                )?)
                .map_err(json_err)
            }
        }
    }
}

fn gebaeude_quellids(
    pid: ProjectId,
    e: &NachweisEingabe,
    gebaeude: &[Fachstand],
) -> std::collections::BTreeSet<String> {
    let mut ids = std::collections::BTreeSet::new();
    if let NachweisEingabe::Nutzung { plan, .. } = e {
        // Auch zugeordnete, aber nicht verknüpfte Gebäude: eine neue Zuordnung
        // macht den Nachweis veraltet. Für vollständig verknüpfte Altstände
        // bleibt der Fingerprint unverändert.
        let bezug = super::einbindung::Lagebezug::neu(pid, plan);
        for f in gebaeude {
            if let Ok(g) = serde_json::from_value::<super::objekte::Gebaeude>(f.daten.clone())
                && bezug.irgendwo_eindeutig(&g)
            {
                ids.insert(f.objekt.clone());
            }
        }
        for b in &plan.berechnungen {
            if let Some(a) = &b.eingabe.aufstellung {
                for b in &a.bauteile {
                    if let Some(id) = b.gebaeude_id {
                        ids.insert(id.to_string());
                    }
                }
                for g in &a.geschosse {
                    if let Some(id) = &g.gebaeude_id {
                        ids.insert(id.clone());
                    }
                }
            }
        }
    }
    ids
}
fn gebaeude_fingerprint(
    pid: ProjectId,
    e: &NachweisEingabe,
    gebaeude: &[Fachstand],
) -> CoreResult<ContentHash> {
    let ids = gebaeude_quellids(pid, e, gebaeude);
    let mut stand = std::collections::BTreeMap::new();
    for f in gebaeude {
        if ids.contains(&f.objekt) {
            let mut daten = f.daten.clone();
            if daten
                .get("planbindungen")
                .and_then(|v| v.as_array())
                .is_some_and(|b| !b.is_empty())
            {
                daten["planaufmass_rechenstand"] =
                    serde_json::json!(crate::planaufmass::RECHENSTAND);
            }
            stand.insert(f.objekt.clone(), daten);
        }
    }
    hash(&serde_json::to_value(stand).map_err(json_err)?)
}

/// Archivinhalt und Metadaten müssen dieselbe Regel bezeichnen.
fn archivierte_baunvo(archiv: &[RegelArchiv]) -> CoreResult<Vec<crate::baunvo::Regelwerk>> {
    if archiv.is_empty() || archiv.len() > 5 {
        return Err(CoreError::InvalidInput(
            "BauNVO-Regelarchiv fehlt oder ist zu groß".into(),
        ));
    }
    let mut werke = Vec::new();
    for a in archiv {
        if a.format != "baunvo-json" || a.inhalt.len() > 1_000_000 {
            return Err(CoreError::InvalidInput(
                "Nicht unterstütztes BauNVO-Regelarchiv".into(),
            ));
        }
        a.sha256.verify(a.inhalt.as_bytes())?;
        let r: crate::baunvo::Regelwerk = serde_json::from_str(&a.inhalt).map_err(json_err)?;
        r.validate()?;
        if r.id != a.id
            || r.version != a.version
            || werke
                .iter()
                .any(|w: &crate::baunvo::Regelwerk| w.fassung == r.fassung || w.id == r.id)
        {
            return Err(CoreError::InvalidInput(
                "Widersprüchliches BauNVO-Regelarchiv".into(),
            ));
        }
        werke.push(r);
    }
    Ok(werke)
}

#[cfg(test)]
mod stabilitaets_tests {
    use super::*;
    #[test]
    fn quellenwechsel_und_infrastrukturfehler_werden_unterschieden() {
        assert_eq!(pruefergebnis(Ok(42)).unwrap(), Some(42));
        assert!(
            pruefergebnis::<()>(Err(CoreError::NotFound("Quelle".into())))
                .unwrap()
                .is_none()
        );
        assert!(
            pruefergebnis::<()>(Err(CoreError::InvalidInput("Quelle geändert".into())))
                .unwrap()
                .is_none()
        );
        assert!(
            pruefergebnis::<()>(Err(CoreError::Database("Verbindung unterbrochen".into())))
                .unwrap_err()
                .contains("Datenbank")
        );
        assert!(
            pruefergebnis::<()>(Err(CoreError::Storage("NAS nicht erreichbar".into()))).is_err()
        );
    }
}
