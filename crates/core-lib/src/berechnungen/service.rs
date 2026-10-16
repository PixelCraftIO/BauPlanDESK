use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
    domain::ProjectId,
    storage::ContentHash,
};
impl CoreServices {
    pub async fn berechnung_pruefen(
        &self,
        pid: ProjectId,
        mut plan: FachPlan,
    ) -> CoreResult<(FachPlan, FachErgebnis)> {
        self.berechnung_quellen(pid, &plan).await?;
        if let Some(id) = plan.grundstueck_id {
            let p = self.get_project(pid).await?;
            let grund = crate::grundstueckskontext::ermitteln(&p)?;
            let g = grund
                .iter()
                .find(|g| g.id == id)
                .ok_or_else(|| err("Baugrundstück gehört nicht zur aktuellen Ermittlung"))?;
            if plan.modul == BerechnungsArt::Freiflaechen {
                let o = plan
                    .eingaben
                    .as_object_mut()
                    .ok_or_else(|| err("Berechnungseingaben als Objekt erforderlich"))?;
                o.insert(
                    "grundstuecksflaeche_m2".into(),
                    serde_json::json!(g.eigentumsflaeche_m2),
                );
                o.insert("bauland_m2".into(), serde_json::json!(g.bezugsflaeche_m2));
            }
        } else if plan.modul == BerechnungsArt::Freiflaechen {
            let o = plan
                .eingaben
                .as_object_mut()
                .ok_or_else(|| err("Berechnungseingaben als Objekt erforderlich"))?;
            o.insert("grundstuecksflaeche_m2".into(), serde_json::json!(0));
            o.insert("bauland_m2".into(), serde_json::Value::Null);
        }
        if !plan.uebernahmen.is_empty() {
            let n = self.nachweise(pid).await?;
            let mut seen = std::collections::HashSet::new();
            for u in plan.uebernahmen.clone() {
                if !seen.insert(u.ziel.clone()) {
                    return Err(err("Übernahmeziel mehrfach belegt"));
                }
                let s = n
                    .iter()
                    .find(|n| n.stand.id == u.nachweis_id)
                    .ok_or_else(|| err("Quellnachweis gehört nicht zu diesem Projekt"))?;
                if !s.aktuell || !s.stand.vollstaendig {
                    return Err(err(
                        "Quellnachweis ist veraltet oder unvollständig. Quelle zuerst aktualisieren.",
                    ));
                }
                let crate::plattform::nachweise::NachweisEingabe::Fach { plan: quelle, .. } =
                    &s.stand.eingabe
                else {
                    return Err(err(
                        "Quelle besitzt keinen passenden typisierten Ergebnisvertrag",
                    ));
                };
                if quelle.grundstueck_id != plan.grundstueck_id
                    || (quelle.gebaeude_id.is_some()
                        && plan.gebaeude_id.is_some()
                        && quelle.gebaeude_id != plan.gebaeude_id)
                {
                    return Err(err(
                        "Quellnachweis gehört zu einer anderen Grundstücks-/Gebäudeeinheit",
                    ));
                }
                let result: FachErgebnis = decode(&s.stand.ergebnis)?;
                let k = result
                    .kennzahlen
                    .iter()
                    .find(|k| k.id == u.kennzahl)
                    .ok_or_else(|| err("Quellkennzahl nicht gefunden"))?;
                let erwartung = importgroesse(&plan, &u.ziel)?;
                if k.einheit != erwartung.0 || k.groesse != erwartung.1 {
                    return Err(err(
                        "Fachliche Bedeutung / Einheit der Quellfläche passt nicht zum Eingabefeld",
                    ));
                }
                let value = k
                    .wert
                    .ok_or_else(|| err("Quellkennzahl ist noch nicht ermittelt"))?;
                setzen(&mut plan.eingaben, &u.ziel, serde_json::json!(value))?;
            }
        }
        normalisieren(&mut plan)?;
        let r = berechnen(&plan)?;
        Ok((plan, r))
    }
    pub(crate) async fn berechnung_quellen(
        &self,
        pid: ProjectId,
        plan: &FachPlan,
    ) -> CoreResult<()> {
        if serde_json::to_vec(plan)
            .map_err(|e| err(e.to_string()))?
            .len()
            > 2 * 1024 * 1024
        {
            return Err(err("Berechnungsplan größer als 2 MB"));
        }
        let p = self.get_project(pid).await?;
        if plan.quellen.len() > 100 || plan.uebernahmen.len() > 100 {
            return Err(err("Zu viele Quellen / Übernahmen"));
        }
        if let Some(id) = plan.grundstueck_id
            && !crate::grundstueckskontext::ermitteln(&p)?
                .iter()
                .any(|g| g.id == id)
        {
            return Err(err("Unbekanntes / geändertes Baugrundstück"));
        }
        if let Some(id) = plan.gebaeude_id {
            let geb = self
                .fachobjekte(pid, ObjektArt::Gebaeude)
                .await?
                .into_iter()
                .find(|g| g.objekt == id.to_string())
                .ok_or_else(|| err("Gebäude gehört nicht zum Projekt"))?;
            let g: crate::plattform::objekte::Gebaeude = decode(&geb.daten)?;
            if plan.modul == BerechnungsArt::Vollgeschosse {
                let e: vollgeschosse::Eingabe = decode(&plan.eingaben)?;
                for s in e.geschosse {
                    if let Some(sid) = s.gebaeude_geschoss_id
                        && !g.geschosse.iter().any(|a| a.id == sid)
                    {
                        return Err(err("Prüfgeschoss gehört nicht zum ausgewählten Gebäude"));
                    }
                }
            }

            if let Some(id) = plan.grundstueck_id
                && !g.grundstueck_ids.is_empty()
                && !g.grundstueck_ids.contains(&id)
            {
                return Err(err(
                    "Gebäude ist einer anderen Grundstückseinheit zugeordnet",
                ));
            }
        }
        for q in &plan.fachquellen {
            if q.art != ObjektArt::Raumbuch || q.revision < 1 {
                return Err(err("Unzulässige Fachquelle"));
            }
            let f = self
                .fachobjekte(pid, q.art)
                .await?
                .into_iter()
                .find(|f| f.objekt == q.objekt.to_string())
                .ok_or_else(|| err("Fachquelle fehlt im Projekt"))?;
            if f.revision != q.revision {
                return Err(err(
                    "Raumbuch wurde geändert. Raumdaten erneut übernehmen und prüfen.",
                ));
            }
            let r: crate::plattform::objekte::Raumbuch = decode(&f.daten)?;
            if plan.gebaeude_id != Some(r.gebaeude_id) {
                return Err(err("Raumbuch gehört zu einem anderen Gebäude"));
            }
        }
        for q in &plan.quellen {
            if q.version < 1
                || !self
                    .document_history(q.dokument_id)
                    .await?
                    .iter()
                    .any(|d| d.project_id == pid && d.version == q.version)
            {
                return Err(err("Quelldokument / Version gehört nicht zum Projekt"));
            }
        }
        Ok(())
    }
    pub(crate) async fn berechnungsplan_vorbereiten(
        &self,
        pid: ProjectId,
        id: Uuid,
        basis: i64,
        daten: serde_json::Value,
        wer: &Benutzer,
    ) -> CoreResult<serde_json::Value> {
        let neu: FachPlan = decode(&daten)?;
        let (mut neu, _) = self.berechnung_pruefen(pid, neu).await?;
        if let Some(alt) = self
            .fachobjekte(pid, neu.modul.objekt())
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
        {
            if alt.revision != basis {
                return Err(crate::CoreError::VersionConflict {
                    expected: basis,
                    actual: alt.revision,
                });
            }
            let vorher: FachPlan = decode(&alt.daten)?;
            if vorher.eingaben.get("regeln") != neu.eingaben.get("regeln")
                && vorher.regelkontext.version == neu.regelkontext.version
            {
                return Err(err("Geänderte Regeln benötigen eine neue Regelversion"));
            }
            let mut v = neu.clone();
            v.regelkontext.anwendbarkeit_bestaetigt = vorher.regelkontext.anwendbarkeit_bestaetigt;
            if v != vorher {
                neu.regelkontext.anwendbarkeit_bestaetigt = false;
            } else if neu.regelkontext.anwendbarkeit_bestaetigt
                && !vorher.regelkontext.anwendbarkeit_bestaetigt
            {
                self.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
            }
        } else if neu.regelkontext.anwendbarkeit_bestaetigt {
            self.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                .await?;
        }
        serde_json::to_value(neu).map_err(|e| err(e.to_string()))
    }
    pub(crate) async fn berechnungs_fingerprint(
        &self,
        pid: ProjectId,
        objekt: Uuid,
        plan: &FachPlan,
    ) -> CoreResult<ContentHash> {
        let p = self.get_project(pid).await?;
        let g = match plan.grundstueck_id {
            Some(id) => crate::grundstueckskontext::ermitteln(&p)?
                .into_iter()
                .find(|g| g.id == id)
                .map(|mut g| {
                    g.projekt_version = 0;
                    g
                }),
            None => None,
        };
        let geb = if let Some(id) = plan.gebaeude_id {
            self.fachobjekte(pid, ObjektArt::Gebaeude)
                .await?
                .into_iter()
                .find(|g| g.objekt == id.to_string())
                .map(|g| {
                    let mut d = g.daten;
                    if let Some(geschosse) = d.get_mut("geschosse").and_then(|v| v.as_array_mut()) {
                        for s in geschosse {
                            if let Some(s) = s.as_object_mut() {
                                s.remove("vollgeschoss");
                                s.remove("grundlage");
                                s.remove("einstufungsquelle");
                            }
                        }
                    }
                    d
                })
        } else {
            None
        };
        let mut fachquellen = Vec::new();
        for q in &plan.fachquellen {
            fachquellen.push(
                self.fachobjekte(pid, q.art)
                    .await?
                    .into_iter()
                    .find(|f| f.objekt == q.objekt.to_string())
                    .map(|f| (f.objekt, f.revision, f.daten)),
            );
        }
        let f = self.fachobjekte(pid, plan.modul.objekt()).await?;
        let daten = f
            .iter()
            .filter(|f| f.objekt == objekt.to_string())
            .filter_map(|f| {
                decode::<FachPlan>(&f.daten)
                    .ok()
                    .filter(|p| p == plan)
                    .map(|_| f.daten.clone())
            })
            .collect::<Vec<_>>();
        Ok(ContentHash::of(&serde_json::to_vec(&serde_json::json!({"plan":plan,"gespeichert":!daten.is_empty(),"fachquellen":fachquellen,"land":p.bundesland,"grund":g,"gebaeude":geb,"planrecht":(&p.planungsrecht.rechtsgrundlage,&p.planungsrecht.plan_bezeichnung,&p.planungsrecht.plan_nummer,&p.planungsrecht.plan_rechtskraft)})).map_err(|e|err(e.to_string()))?))
    }
}
fn setzen(v: &mut serde_json::Value, pfad: &str, wert: serde_json::Value) -> CoreResult<()> {
    let mut current = v;
    let teile = pfad.split('.').collect::<Vec<_>>();
    if teile.len() > 8 {
        return Err(err("Übernahmeziel zu tief"));
    }
    for (i, k) in teile.iter().enumerate() {
        let slot = match current {
            serde_json::Value::Array(a) => a
                .iter_mut()
                .find(|v| v.get("id").and_then(|id| id.as_str()) == Some(*k)),
            _ => current.get_mut(*k),
        }
        .ok_or_else(|| err("Übernahmeziel existiert nicht"))?;
        if i + 1 == teile.len() {
            *slot = wert;
            return Ok(());
        }
        current = slot;
    }
    Err(err("Leeres Übernahmeziel"))
}
/// Explizite fachliche Übernahmeziele verhindern Wohnfläche → GFZ/Entwässerung durch bloße m²-Gleichheit.
pub(crate) fn importgroesse(p: &FachPlan, ziel: &str) -> CoreResult<(Einheit, Messgroesse)> {
    let teile = ziel.split('.').collect::<Vec<_>>();
    if p.modul == BerechnungsArt::Freiflaechen && ziel == "wohneinheiten" {
        return Ok((Einheit::Anzahl, Messgroesse::Wohneinheiten));
    }
    if teile.len() != 3
        || teile[0] != "positionen"
        || p.eingaben
            .get("positionen")
            .and_then(|v| v.as_array())
            .is_none_or(|a| {
                !a.iter()
                    .any(|v| v.get("id").and_then(|v| v.as_str()) == Some(teile[1]))
            })
    {
        return Err(err("Kein zulässiges Übernahmeziel"));
    }
    let g = match (p.modul, teile[2]) {
        (BerechnungsArt::Stellplaetze, "wohneinheiten") => Messgroesse::Wohneinheiten,
        (BerechnungsArt::Stellplaetze, "wohnflaeche_m2") => Messgroesse::Wohnflaeche,
        (BerechnungsArt::Stellplaetze, "nutzungsflaeche_m2") => Messgroesse::Nutzungsflaeche,
        (BerechnungsArt::Stellplaetze, "verkaufsflaeche_m2") => Messgroesse::Verkaufsflaeche,
        (BerechnungsArt::Stellplaetze, "sitzplaetze") => Messgroesse::Sitzplaetze,
        (BerechnungsArt::Stellplaetze, "betten") => Messgroesse::Betten,
        (BerechnungsArt::Stellplaetze, "beschaeftigte") => Messgroesse::Beschaeftigte,
        (BerechnungsArt::Entwaesserung, "flaeche_m2") => Messgroesse::Grundflaeche,
        _ => {
            return Err(err(
                "Dieses Eingabefeld unterstützt keine fachlich passende Übernahme",
            ));
        }
    };
    Ok((
        if matches!(
            g,
            Messgroesse::Wohneinheiten
                | Messgroesse::Sitzplaetze
                | Messgroesse::Betten
                | Messgroesse::Beschaeftigte
        ) {
            Einheit::Anzahl
        } else {
            Einheit::M2
        },
        g,
    ))
}
