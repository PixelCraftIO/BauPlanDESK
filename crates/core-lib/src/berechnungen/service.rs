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
        if plan.modul == BerechnungsArt::Hoehen {
            hoehenfestsetzungen_einsetzen(&mut plan, &self.get_project(pid).await?.planungsrecht)?;
        }
        self.gebaeudedaten_einsetzen(pid, &mut plan).await?;
        self.planwerte_einsetzen(pid, &mut plan).await?;
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
    async fn gebaeudedaten_einsetzen(&self, pid: ProjectId, plan: &mut FachPlan) -> CoreResult<()> {
        if plan.gebaeudedaten_uebernehmen != Some(true) {
            return Ok(());
        }
        if !matches!(
            plan.modul,
            BerechnungsArt::Vollgeschosse | BerechnungsArt::Hoehen
        ) {
            return Err(err(
                "Gebäudeübernahme wird für dieses Fachmodul nicht unterstützt",
            ));
        }
        let id = plan
            .gebaeude_id
            .ok_or_else(|| err("Für die Übernahme ein Gebäude auswählen"))?;
        let stand = self
            .fachobjekte(pid, ObjektArt::Gebaeude)
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| err("Gebäude gehört nicht zum Projekt"))?;
        let g: crate::plattform::objekte::Gebaeude = decode(&stand.daten)?;
        self.gebaeude_planquellen_pruefen(pid, id, &g).await?;
        let basis = g.grundlagen(id)?;
        if plan.modul == BerechnungsArt::Hoehen {
            return hoehen_einsetzen(plan, &g, &basis);
        }
        let mut e: vollgeschosse::Eingabe = decode(&plan.eingaben)?;
        let vorher = (e.geschosse.clone(), e.teilflaechen.clone());
        let mut geschosse = Vec::new();
        let mut zonen = Vec::new();
        for s in basis.geschosse {
            let flaeche = s.flaeche_m2.ok_or_else(|| {
                err(format!(
                    "{}: Geschossfläche in den Grundlagen ergänzen",
                    s.bezeichnung
                ))
            })?;
            let herkunft = g
                .geometrie
                .as_ref()
                .and_then(|g| g.herkunft.as_deref())
                .filter(|h| !h.trim().is_empty())
                .map(str::to_owned)
                .or_else(|| {
                    g.quelle
                        .as_ref()
                        .map(|d| format!("Dokument {} · Version {}", d.dokument_id.0, d.version))
                });
            let quelle = herkunft
                .map(|h| {
                    format!(
                        "Gebäude {} · {} · {} · {}",
                        g.bezeichnung,
                        h,
                        crate::plattform::gebaeude::RECHENSTAND,
                        s.herleitung
                    )
                })
                .unwrap_or_default();
            let alt = e
                .geschosse
                .iter()
                .find(|a| a.gebaeude_geschoss_id == Some(s.id));
            geschosse.push(vollgeschosse::Geschoss {
                id: s.id.to_string(),
                gebaeude_geschoss_id: Some(s.id),
                bezeichnung: s.bezeichnung,
                grundflaeche_m2: flaeche,
                unterliegend_m2: s.unterliegend_m2,
                decke_ueber_gelaende_m: s.decke_ueber_gelaende_m,
                hoehenquelle: quelle,
                sonderentscheidung: alt.and_then(|a| a.sonderentscheidung),
                sondergrundlage: alt.map(|a| a.sondergrundlage.clone()).unwrap_or_default(),
            });
            let masse = if s.implizit {
                g.geometrie.as_ref().and_then(|g| g.einzelgeschoss.as_ref())
            } else {
                g.geschosse
                    .iter()
                    .find(|a| a.id == s.id)
                    .and_then(|a| a.geometrie.as_ref())
            };
            if masse.and_then(|m| m.lichte_hoehe_einheitlich) == Some(true)
                && let Some(h) = s.lichte_hoehe_m
            {
                zonen.push(vollgeschosse::Hoehenflaeche {
                    id: format!("zone-{}", s.id),
                    geschoss_id: s.id.to_string(),
                    bezeichnung: "Einheitliche lichte Raumhöhe laut Grundlagen".into(),
                    flaeche_m2: flaeche,
                    lichte_hoehe_m: h,
                });
            }
        }
        // Geschosse ohne einheitliche lichte Höhe (z. B. Dachgeschoss mit
        // Schrägen) liefert die Übernahme keine Zone; dort bleiben die manuell
        // erfassten Höhenzonen erhalten. Frühere automatische Zonen solcher
        // Geschosse und Zonen nicht mehr vorhandener Geschosse entfallen.
        let manuell: Vec<_> = e
            .teilflaechen
            .iter()
            .filter(|t| {
                geschosse.iter().any(|s| s.id == t.geschoss_id)
                    && !zonen.iter().any(|z| z.geschoss_id == t.geschoss_id)
                    && t.id != format!("zone-{}", t.geschoss_id)
            })
            .cloned()
            .collect();
        zonen.extend(manuell);
        e.geschosse = geschosse;
        e.teilflaechen = zonen;
        if vorher != (e.geschosse.clone(), e.teilflaechen.clone()) {
            e.aufmass_geprueft = false;
            e.sonderfaelle_geprueft = false;
            plan.regelkontext.anwendbarkeit_bestaetigt = false;
        }
        plan.eingaben = serde_json::to_value(e).map_err(|e| err(e.to_string()))?;
        Ok(())
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
            if plan.gebaeudedaten_uebernehmen == Some(true) {
                self.gebaeude_planquellen_pruefen(pid, id, &g).await?;
            }
            if plan.modul == BerechnungsArt::Vollgeschosse {
                let e: vollgeschosse::Eingabe = decode(&plan.eingaben)?;
                for s in e.geschosse {
                    if let Some(sid) = s.gebaeude_geschoss_id
                        && !g.grundlagen(id)?.geschosse.iter().any(|a| a.id == sid)
                    {
                        return Err(err("Prüfgeschoss gehört nicht zum ausgewählten Gebäude"));
                    }
                }
            }

            // Das Baugrundstück des Gebäudes folgt aus seinen Flurstücken.
            if let Some(id) = plan.grundstueck_id
                && let Some(grundplan) = crate::grundstuecksinformationen::plan(&p.planungsrecht)?
                && !crate::plattform::einbindung::Lagebezug::neu(pid, &grundplan)
                    .moeglich_id(&g, id)
            {
                return Err(err(
                    "Das Gebäude steht laut seinen Flurstücken auf einer anderen Grundstückseinheit",
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
            self.raumbuch_planquellen_pruefen(pid, &r).await?;
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
            // Ein neuerer Stand derselben Quelle mit unveränderten Werten ändert
            // die bestätigte Prüfaussage nicht.
            if v.uebernahmen.len() == vorher.uebernahmen.len() {
                for (a, b) in v.uebernahmen.iter_mut().zip(&vorher.uebernahmen) {
                    if a.ziel == b.ziel && a.kennzahl == b.kennzahl {
                        a.nachweis_id = b.nachweis_id;
                    }
                }
            }
            // Dasselbe gilt für einen neueren Raumbuchstand mit gleichen Flächen
            // und Höhen: Raumname und Quellenangabe ändern die Prüfaussage nicht.
            let ohne_raumtexte = |mut plan: FachPlan| {
                plan.fachquellen.clear();
                if let Some(zeilen) = plan
                    .eingaben
                    .get_mut("positionen")
                    .and_then(|z| z.as_array_mut())
                {
                    for z in zeilen {
                        if z["quelle"]
                            .as_str()
                            .is_some_and(|q| q.starts_with("Raumbuch"))
                            && let Some(o) = z.as_object_mut()
                        {
                            o.remove("quelle");
                            o.remove("raum");
                        }
                    }
                }
                plan
            };
            let gleich = v == vorher
                || (v.fachquellen != vorher.fachquellen
                    && ohne_raumtexte(v) == ohne_raumtexte(vorher.clone()));
            if !gleich {
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
        // Vom Gebäude zählt nur, was dieser Nachweis daraus verwendet: Name und
        // Lage immer; bei Gebäudeübernahme zusätzlich die daraus abgeleiteten
        // Eingaben. Andere Gebäudeangaben machen den Nachweis nicht veraltet.
        let stand = match plan.gebaeude_id {
            Some(id) => {
                self.repos
                    .commit
                    .fachobjekt(pid, ObjektArt::Gebaeude.modul(), &id.to_string())
                    .await?
            }
            None => None,
        };
        let mit_planbindung = stand.as_ref().is_some_and(|g| {
            g.daten
                .get("planbindungen")
                .and_then(|v| v.as_array())
                .is_some_and(|b| !b.is_empty())
        });
        let geb = match &stand {
            Some(f) => {
                let g: crate::plattform::objekte::Gebaeude = decode(&f.daten)?;
                let abgeleitet = if plan.gebaeudedaten_uebernehmen == Some(true) {
                    let mut p = plan.clone();
                    self.gebaeudedaten_einsetzen(pid, &mut p).await?;
                    normalisieren(&mut p)?;
                    Some((p.eingaben, p.regelkontext.anwendbarkeit_bestaetigt))
                } else {
                    None
                };
                Some(serde_json::json!({
                    "bezeichnung": g.bezeichnung,
                    "flurstueck_ids": g.flurstueck_ids,
                    "flurstuecke_zu_pruefen": g.flurstuecke_zu_pruefen,
                    "grundstueck_ids": g.grundstueck_ids,
                    "abgeleitet": abgeleitet,
                }))
            }
            None => None,
        };
        let mut fachquellen = Vec::new();
        if let (Some(id), true) = (plan.gebaeude_id, geb.is_some())
            && plan.gebaeudedaten_uebernehmen == Some(true)
        {
            self.gebaeudegrundlagen(pid, id).await?;
            if mit_planbindung {
                fachquellen.push(Some((
                    "gebaeudeaufmass-rechenstand".into(),
                    1,
                    serde_json::json!(crate::planaufmass::RECHENSTAND),
                )));
            }
        }
        if !plan.planbindungen.is_empty() {
            fachquellen.push(Some((
                "planaufmass-rechenstand".into(),
                1,
                serde_json::json!(crate::planaufmass::RECHENSTAND),
            )));
        }
        // Fingerprint berücksichtigt sowohl Zeichenmaske als auch die neueste PDF-Version.
        for b in &plan.planbindungen {
            let f = self
                .repos
                .commit
                .fachobjekt(
                    pid,
                    ObjektArt::Planaufmass.modul(),
                    &b.aufmass_id.to_string(),
                )
                .await?;
            if let Some(f) = &f {
                let a: crate::planaufmass::Planaufmass = decode(&f.daten)?;
                // Aufmaß: die Unterlage; eigene Zeichnung: ihre Hintergründe.
                let unterlagen = a
                    .dokument
                    .iter()
                    .chain(a.hintergruende.iter().map(|h| &h.dokument));
                for d in unterlagen {
                    let version = match self.repos.documents.latest(d.dokument_id).await {
                        Ok(d) if d.project_id == pid => Some(d.version),
                        Ok(_) | Err(CoreError::NotFound(_)) => None,
                        Err(e) => return Err(e),
                    };
                    fachquellen.push(Some((
                        format!("pdf:{}", d.dokument_id.0),
                        version.unwrap_or(0),
                        serde_json::Value::Null,
                    )));
                }
            }
            // Nur der für diese Bindung maßgebende Stand; Ebenen oder andere
            // Markierungen desselben Aufmaßes machen den Nachweis nicht veraltet.
            let stand = match &f {
                Some(f) => {
                    let a: crate::planaufmass::Planaufmass = decode(&f.daten)?;
                    Some((
                        f.objekt.clone(),
                        b.revision,
                        crate::planaufmass::bindungsstand(&a, b.markierung_id),
                    ))
                }
                None => None,
            };
            fachquellen.push(stand);
        }
        for q in &plan.fachquellen {
            if q.art == ObjektArt::Raumbuch {
                let f = self
                    .repos
                    .commit
                    .fachobjekt(pid, q.art.modul(), &q.objekt.to_string())
                    .await?
                    .ok_or_else(|| err("Raumbuch fehlt"))?;
                let r: crate::plattform::objekte::Raumbuch = decode(&f.daten)?;
                self.raumbuch_planquellen_pruefen(pid, &r).await?;
                if !r.planbindungen.is_empty() {
                    fachquellen.push(Some((
                        "raumaufmass-rechenstand".into(),
                        1,
                        serde_json::json!(crate::planaufmass::RECHENSTAND),
                    )));
                }
            }
            fachquellen.push(
                self.repos
                    .commit
                    .fachobjekt(pid, q.art.modul(), &q.objekt.to_string())
                    .await?
                    .map(|f| (f.objekt, f.revision, f.daten)),
            );
        }
        let gespeichert = self
            .repos
            .commit
            .fachobjekt(pid, plan.modul.objekt().modul(), &objekt.to_string())
            .await?
            .is_some_and(|f| decode::<FachPlan>(&f.daten).is_ok_and(|p| p == *plan));
        Ok(ContentHash::of(&serde_json::to_vec(&serde_json::json!({"plan":plan,"gespeichert":gespeichert,"fachquellen":fachquellen,"land":p.bundesland,"grund":g,"gebaeude":geb,"planrecht":(&p.planungsrecht.rechtsgrundlage,&p.planungsrecht.plan_bezeichnung,&p.planungsrecht.plan_nummer,&p.planungsrecht.plan_rechtskraft)})).map_err(|e|err(e.to_string()))?))
    }
}
/// Höhen- und Dachneigungsgrenzen aus den Festsetzungen des Projekts: Der
/// Höhennachweis prüft gegen die dort erfassten Werte und führt keine eigene
/// Fassung davon. Ohne Höhenfestsetzungen im Projekt bleibt der Plan unverändert.
fn hoehenfestsetzungen_einsetzen(
    plan: &mut FachPlan,
    recht: &crate::domain::Planungsrecht,
) -> CoreResult<()> {
    let f = &recht.festgesetzte_werte;
    if !f.mit_hoehen() {
        return Ok(());
    }
    let mut e: hoehen::Eingabe = decode(&plan.eingaben)?;
    let vorher = e.clone();
    // Ohne Planbezeichnung: Ein umbenannter Plan ändert keine Grenze. Welcher
    // Plan gilt, hält der Nachweis mit seinen Rechtsgrundlagen fest.
    let quelle = "Festsetzungen des Bebauungsplans laut Projekt".to_string();
    for (art, max) in [
        ("traufe", f.traufhoehe_max_m),
        ("first", f.firsthoehe_max_m),
        ("gebaeude", f.gebaeudehoehe_max_m),
    ] {
        let Some(max) = max else { continue };
        let minimum_m = e
            .regeln
            .iter()
            .find(|g| g.art == art)
            .and_then(|g| g.minimum_m)
            .filter(|min| *min <= max);
        e.regeln.retain(|g| g.art != art);
        e.regeln.push(hoehen::Grenze {
            art: art.into(),
            minimum_m,
            maximum_m: Some(max),
            absolut: f.hoehen_ueber_nhn,
            quelle: quelle.clone(),
        });
    }
    if !f.hoehen_ueber_nhn
        && let Some(b) = f.hoehenbezug_m_nhn
    {
        // `hoehenbezug` nennt das Höhensystem des Plans und bleibt unberührt.
        e.bezugshoehe_m = Some(b);
    }
    for d in &mut e.daecher {
        if f.dachneigung_min_grad.is_some() || f.dachneigung_max_grad.is_some() {
            d.min_neigung_grad = f.dachneigung_min_grad;
            d.max_neigung_grad = f.dachneigung_max_grad;
        }
    }
    if e != vorher {
        // Geänderte Grenzen sind eine neue Regelfassung dieses Nachweises.
        let stand = serde_json::to_vec(&(
            &e.regeln,
            e.bezugshoehe_m,
            &f.dachneigung_min_grad,
            &f.dachneigung_max_grad,
        ))
        .map_err(|e| err(e.to_string()))?;
        plan.regelkontext.version = format!(
            "Festsetzungen {}",
            &ContentHash::of(&stand).to_string()[..8]
        );
        // Ein anderer Bezugspunkt verlangt eine neue Prüfung des Höhenbezugs.
        if e.bezugshoehe_m != vorher.bezugshoehe_m {
            e.bezug_geprueft = false;
        }
        plan.regelkontext.anwendbarkeit_bestaetigt = false;
        plan.eingaben = serde_json::to_value(e).map_err(|e| err(e.to_string()))?;
    }
    Ok(())
}
/// Traufe und First aus Gebäude/Geschosse als Höhenpunkte (m ü. NHN), bezogen
/// auf die gewählte Bezugshöhe (`bezugshoehe_m`), sonst auf die mittlere
/// Geländehöhe des Gebäudes. Eigene Punkte bleiben erhalten;
/// Übernommene Punkte tragen die gültige Positionskennung `gebaeude_<id>_<art>` und werden bei
/// jeder Prüfung aktualisiert. Geänderte Werte setzen die Bezugsprüfung zurück.
fn hoehen_einsetzen(
    plan: &mut FachPlan,
    g: &crate::plattform::objekte::Gebaeude,
    basis: &crate::plattform::gebaeude::GebaeudeGrundlagen,
) -> CoreResult<()> {
    let h = &basis.hoehen_nhn;
    let praefix = format!("gebaeude_{}_", basis.gebaeude_id);
    let altpraefix = format!("gebaeude:{}:", basis.gebaeude_id);
    let mut e: hoehen::Eingabe = decode(&plan.eingaben)?;
    // Eine gewählte Bezugshöhe (z. B. festgesetzte Straßenoberkante) bleibt
    // maßgebend; nur ohne sie gilt die mittlere Geländehöhe des Gebäudes.
    let (bezug, bezugstext) = match e.bezugshoehe_m {
        Some(b) => (b, format!("Bezug gewählte Bezugshöhe {} m", zahl(b))),
        None => (
            h.gelaende_m.ok_or_else(|| {
                err("Gebäudenullpunkt (m ü. NHN) und mittlere Geländehöhe in Gebäude/Geschosse ergänzen oder eine Bezugshöhe angeben")
            })?,
            "Bezug mittlere Geländehöhe".to_string(),
        ),
    };
    let vorher = e.punkte.clone();
    e.punkte
        .retain(|p| !p.id.starts_with(&praefix) && !p.id.starts_with(&altpraefix));
    // Entwürfe mit der früheren, vom Kennungsvalidator abgewiesenen Schreibweise
    // erhalten ihre Dachverweise beim expliziten erneuten Gebäudeimport.
    for d in &mut e.daecher {
        for id in [&mut d.traufe_id, &mut d.first_id] {
            if let Some(art) = id.strip_prefix(&altpraefix) {
                *id = format!("{praefix}{art}");
            }
        }
    }
    let herkunft = g
        .geometrie
        .as_ref()
        .and_then(|g| g.herkunft.as_deref())
        .filter(|h| !h.trim().is_empty())
        .unwrap_or("Gebäude/Geschosse");
    let mut neu = 0;
    for (art, name, wert) in [
        ("traufe", "Traufe", h.traufe_m),
        ("first", "First", h.first_m),
    ] {
        let Some(wert) = wert else { continue };
        neu += 1;
        e.punkte.push(hoehen::Punkt {
            id: format!("{praefix}{art}"),
            bezeichnung: format!("{} · {name}", g.bezeichnung),
            art: art.into(),
            absolut_m: wert,
            bezug_m: bezug,
            quelle: format!(
                "Gebäude {} · {herkunft} · Nullpunkt + relative Höhe; {bezugstext} · {}",
                g.bezeichnung,
                crate::plattform::gebaeude::RECHENSTAND
            ),
        });
    }
    if neu == 0 {
        return Err(err("Trauf- oder Firsthöhe in Gebäude/Geschosse ergänzen"));
    }
    if e.punkte != vorher {
        e.bezug_geprueft = false;
        plan.regelkontext.anwendbarkeit_bestaetigt = false;
    }
    plan.eingaben = serde_json::to_value(e).map_err(|e| err(e.to_string()))?;
    Ok(())
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

/// Ergebnisse der Fachberechnungen für Formulare: Art, Kennzahl, Schlüssel im
/// Formularkontext, Nachkommastellen.
const FORMULARWERTE: &[(BerechnungsArt, &str, &str, usize)] = &[
    (
        BerechnungsArt::Wohnflaechen,
        "wohnflaeche",
        "wohnflaeche.gesamt_m2",
        2,
    ),
    (
        BerechnungsArt::Wohnflaechen,
        "wohneinheiten",
        "wohnflaeche.wohneinheiten",
        0,
    ),
    (BerechnungsArt::Din277, "bgf", "din277.bgf_m2", 2),
    (BerechnungsArt::Din277, "kgf", "din277.kgf_m2", 2),
    (BerechnungsArt::Din277, "bri", "din277.bri_m3", 2),
    (
        BerechnungsArt::Stellplaetze,
        "kfz_bedarf",
        "stellplaetze.kfz_bedarf",
        0,
    ),
    (
        BerechnungsArt::Stellplaetze,
        "fahrrad_bedarf",
        "stellplaetze.fahrrad_bedarf",
        0,
    ),
    (
        BerechnungsArt::Freiflaechen,
        "versiegelt_m2",
        "freiflaechen.versiegelt_m2",
        2,
    ),
    (
        BerechnungsArt::Freiflaechen,
        "gruen_m2",
        "freiflaechen.gruen_m2",
        2,
    ),
    (
        BerechnungsArt::Entwaesserung,
        "au",
        "entwaesserung.abflusswirksame_flaeche_m2",
        2,
    ),
    (
        BerechnungsArt::Entwaesserung,
        "rueckhaltung",
        "entwaesserung.rueckhaltevolumen_m3",
        2,
    ),
];
fn artname(art: BerechnungsArt) -> &'static str {
    match art {
        BerechnungsArt::Wohnflaechen => "Wohnflächenberechnung",
        BerechnungsArt::Din277 => "Berechnung nach DIN 277",
        BerechnungsArt::Stellplaetze => "Stellplatzberechnung",
        BerechnungsArt::Freiflaechen => "Freiflächenberechnung",
        BerechnungsArt::Entwaesserung => "Entwässerungsberechnung",
        BerechnungsArt::Hoehen => "Höhenberechnung",
        _ => "Berechnung",
    }
}
/// Schlüsselteil aus einer Bezeichnung: „Traufe Süd“ → `traufe_sued`.
fn schluesselteil(name: &str) -> String {
    let mut s = String::new();
    for c in name.trim().to_lowercase().chars() {
        match c {
            'ä' => s.push_str("ae"),
            'ö' => s.push_str("oe"),
            'ü' => s.push_str("ue"),
            'ß' => s.push_str("ss"),
            c if c.is_ascii_alphanumeric() => s.push(c),
            _ if !s.ends_with('_') && !s.is_empty() => s.push('_'),
            _ => {}
        }
    }
    s.trim_end_matches('_').to_string()
}

impl CoreServices {
    /// Ergebnisvertrag der Fachberechnungen für den Formularkontext:
    /// Schlüssel und formatierter Wert je Kennzahl sowie Hinweise.
    ///
    /// Übernommen wird je Art nur eine einzige, vollständige Berechnung des
    /// Projekts mit ihrem aktuell gerechneten Ergebnis. Bei mehreren
    /// Berechnungen einer Art oder offenen Punkten bleibt der Wert leer, und
    /// ein Hinweis nennt den Grund; es wird nichts still ausgewählt.
    pub(crate) async fn berechnungswerte_fuer_formulare(
        &self,
        pid: ProjectId,
    ) -> CoreResult<(Vec<(String, String)>, Vec<String>)> {
        let (mut werte, mut hinweise) = (vec![], vec![]);
        let arten = [
            BerechnungsArt::Wohnflaechen,
            BerechnungsArt::Din277,
            BerechnungsArt::Stellplaetze,
            BerechnungsArt::Freiflaechen,
            BerechnungsArt::Entwaesserung,
            BerechnungsArt::Hoehen,
        ];
        for art in arten {
            let modul = art.objekt().modul();
            let mut plaene: Vec<FachPlan> = self
                .repos
                .commit
                .fachstaende(pid, modul)
                .await?
                .into_iter()
                .filter_map(|f| serde_json::from_value(f.daten).ok())
                .collect();
            let plan = match plaene.len() {
                0 => continue,
                1 => plaene.remove(0),
                n => {
                    hinweise.push(format!(
                        "{}: {n} Berechnungen im Projekt; Werte bitte von Hand eintragen.",
                        artname(art)
                    ));
                    continue;
                }
            };
            let name = plan.bezeichnung.clone();
            let ergebnis = match Box::pin(self.berechnung_pruefen(pid, plan)).await {
                Ok((_, e)) if e.vollstaendig => e,
                Ok(_) => {
                    hinweise.push(format!(
                        "{} „{name}“ hat offene Punkte; ihre Werte werden nicht übernommen.",
                        artname(art)
                    ));
                    continue;
                }
                Err(e) => {
                    hinweise.push(format!(
                        "{} „{name}“ lässt sich nicht rechnen ({e}); ihre Werte werden nicht übernommen.",
                        artname(art)
                    ));
                    continue;
                }
            };
            for k in &ergebnis.kennzahlen {
                let Some(wert) = k.wert else { continue };
                if art == BerechnungsArt::Hoehen {
                    // Höhenpunkte und Dächer tragen die Namen aus der Berechnung.
                    let (gruppe, stellen) = match k.id.split_once(':') {
                        Some(("hoehe", _)) => ("hoehe", 2),
                        Some(("dach", _)) => ("dachneigung", 1),
                        _ => continue,
                    };
                    let teil = schluesselteil(
                        k.bezeichnung
                            .trim_start_matches("Relative Höhe ")
                            .trim_start_matches("Dachneigung "),
                    );
                    if !teil.is_empty() {
                        werte.push((
                            format!("hoehen.{gruppe}_{teil}"),
                            crate::formular::vorlage::zahl(wert, stellen),
                        ));
                    }
                } else if let Some((.., schluessel, stellen)) = FORMULARWERTE
                    .iter()
                    .find(|(a, id, ..)| *a == art && *id == k.id)
                {
                    werte.push((
                        (*schluessel).to_string(),
                        crate::formular::vorlage::zahl(wert, *stellen),
                    ));
                }
            }
        }
        Ok((werte, hinweise))
    }
}
