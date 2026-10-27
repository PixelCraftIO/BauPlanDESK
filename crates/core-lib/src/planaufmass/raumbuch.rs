use super::*;
use crate::{
    CoreServices,
    domain::ProjectId,
    plattform::{
        ObjektArt,
        objekte::{Hoehenzone, Raum, Raumbuch},
    },
};
fn nutzung(b: Planbedeutung) -> Option<&'static str> {
    match b {
        Planbedeutung::Raum => Some(""),
        Planbedeutung::Aufenthaltsraum => Some("Aufenthaltsraum"),
        Planbedeutung::Nuf => Some("NUF"),
        Planbedeutung::Tf => Some("Technik"),
        Planbedeutung::Vf => Some("Verkehr"),
        _ => None,
    }
}
impl CoreServices {
    pub async fn planaufmass_raumbuch(
        &self,
        pid: ProjectId,
        aufmass_id: Uuid,
        revision: i64,
        mut r: Raumbuch,
    ) -> CoreResult<Raumbuch> {
        let f = self
            .fachobjekte(pid, ObjektArt::Planaufmass)
            .await?
            .into_iter()
            .find(|f| f.objekt == aufmass_id.to_string())
            .ok_or_else(|| fehler("Planaufmaß fehlt im Projekt"))?;
        if f.revision != revision || f.schema_version != 1 {
            return Err(fehler("Planaufmaß geändert. Auswahl aktualisieren."));
        }
        let a: Planaufmass = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
        let mut anzahl = 0;
        for m in &a.markierungen {
            let Some(nutzung) = nutzung(m.bedeutung) else {
                continue;
            };
            if m.hoehenzone_von.is_some() {
                continue;
            }
            if m.gebaeude_id != Some(r.gebaeude_id)
                || (r.geschoss_id.is_some() && m.geschoss_id != r.geschoss_id)
            {
                continue;
            }
            anzahl += 1;
            let ziel = format!("raeume.{}.flaeche_m2", m.id);
            let b = Planbindung {
                aufmass_id,
                revision,
                markierung_id: m.id,
                groesse: Planmessgroesse::Flaeche,
                ziel: ziel.clone(),
            };
            let (_, _, flaeche) = self.planbindung_aufloesen(pid, &b).await?;
            if let Some(raum) = r.raeume.iter_mut().find(|r| r.id == m.id) {
                // Bereits vom Benutzer vergebene Raumnummer, Nutzung und Raumhöhe erhalten.
                raum.flaeche_m2 = flaeche;
            } else {
                let mut nummer = r.raeume.len() + 1;
                while r.raeume.iter().any(|r| r.nummer == nummer.to_string()) {
                    nummer += 1;
                }
                r.raeume.push(Raum {
                    id: m.id,
                    nummer: nummer.to_string(),
                    bezeichnung: m.bezeichnung.clone(),
                    nutzung: nutzung.into(),
                    flaeche_m2: flaeche,
                    hoehe_m: None,
                    hoehenzonen: vec![],
                });
            }
            r.planbindungen.retain(|b| b.ziel != ziel);
            r.planbindungen.push(b);
        }
        if anzahl == 0 {
            return Err(fehler(
                "Keine Raum-/NUF-/TF-/VF-Markierungen für das gewählte Gebäude und Geschoss vorhanden",
            ));
        }
        // Höhenzonen: Teilflächen der übernommenen Räume. Eine früher als
        // eigener Raum übernommene Fläche wird zur Zone (Höhe bleibt erhalten).
        for m in &a.markierungen {
            let Some(raum_id) = m.hoehenzone_von else {
                continue;
            };
            if nutzung(m.bedeutung).is_none() || !r.raeume.iter().any(|r| r.id == raum_id) {
                continue;
            }
            let ziel = format!("raeume.{raum_id}.hoehenzonen.{}.flaeche_m2", m.id);
            let b = Planbindung {
                aufmass_id,
                revision,
                markierung_id: m.id,
                groesse: Planmessgroesse::Flaeche,
                ziel: ziel.clone(),
            };
            let (_, _, flaeche) = self.planbindung_aufloesen(pid, &b).await?;
            let frueher = r
                .raeume
                .iter()
                .position(|x| x.id == m.id)
                .map(|i| r.raeume.remove(i));
            if frueher.is_some() {
                let alt = format!("raeume.{}.", m.id);
                r.planbindungen.retain(|b| !b.ziel.starts_with(&alt));
            }
            let raum = r
                .raeume
                .iter_mut()
                .find(|x| x.id == raum_id)
                .expect("geprüft");
            match raum.hoehenzonen.iter_mut().find(|z| z.id == m.id) {
                Some(z) => z.flaeche_m2 = flaeche,
                None => raum.hoehenzonen.push(Hoehenzone {
                    id: m.id,
                    bezeichnung: m.bezeichnung.clone(),
                    flaeche_m2: flaeche,
                    lichte_hoehe_min_m: frueher.and_then(|f| f.hoehe_m),
                    lichte_hoehe_max_m: None,
                }),
            }
            r.planbindungen.retain(|b| b.ziel != ziel);
            r.planbindungen.push(b);
        }
        self.planwerte_raumbuch(pid, &mut r).await?;
        r.summen()?;
        Ok(r)
    }
    pub(crate) async fn planwerte_raumbuch(
        &self,
        pid: ProjectId,
        r: &mut Raumbuch,
    ) -> CoreResult<()> {
        if r.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Planverknüpfungen"));
        }
        let mut seen = std::collections::HashSet::new();
        for b in r.planbindungen.clone() {
            if !seen.insert(b.ziel.clone()) {
                return Err(fehler("Raumfeld mehrfach verknüpft"));
            }
            let t: Vec<_> = b.ziel.split('.').collect();
            // raeume.<raum>.flaeche_m2 oder raeume.<raum>.hoehenzonen.<zone>.flaeche_m2
            let ungueltig = || fehler("Raumbuch unterstützt Planflächen über stabile Raum-IDs");
            let zone = match t.as_slice() {
                ["raeume", _, "flaeche_m2"] => None,
                ["raeume", _, "hoehenzonen", z, "flaeche_m2"] => Some(*z),
                _ => return Err(ungueltig()),
            };
            if b.groesse != Planmessgroesse::Flaeche {
                return Err(ungueltig());
            }
            // Fehler nennen den Raum, damit die Quelle im Raumbuch auffindbar ist.
            let name = r
                .raeume
                .iter()
                .find(|r| r.id.to_string() == t[1])
                .map(|r| format!("Raum {} „{}“", r.nummer, r.bezeichnung))
                .unwrap_or_else(|| "Verknüpfter Raum".into());
            let (_, m, wert) = match self.planbindung_aufloesen(pid, &b).await {
                Ok(v) => v,
                Err(crate::CoreError::InvalidInput(t)) => {
                    return Err(fehler(format!(
                        "{name}: {t} Im Raumbuch erneut aus dem Aufmaß übernehmen oder die Verknüpfung lösen."
                    )));
                }
                Err(e) => return Err(e),
            };
            if nutzung(m.bedeutung).is_none()
                || m.gebaeude_id != Some(r.gebaeude_id)
                || (r.geschoss_id.is_some() && m.geschoss_id != r.geschoss_id)
            {
                return Err(fehler(
                    "Planraum gehört nicht zum Gebäude/Geschoss dieses Raumbuchs",
                ));
            }
            let raum = r
                .raeume
                .iter_mut()
                .find(|r| r.id.to_string() == t[1])
                .ok_or_else(|| fehler("Verknüpfter Raum fehlt"))?;
            match zone {
                None => raum.flaeche_m2 = wert,
                Some(z) => {
                    if m.hoehenzone_von != Some(raum.id) {
                        return Err(fehler(format!(
                            "{name}: Die Planfläche „{}“ ist im Aufmaß keine Höhenzone dieses Raums mehr. Im Raumbuch erneut aus dem Aufmaß übernehmen oder die Verknüpfung lösen.",
                            m.bezeichnung
                        )));
                    }
                    raum.hoehenzonen
                        .iter_mut()
                        .find(|x| x.id.to_string() == z)
                        .ok_or_else(|| fehler(format!("{name}: Verknüpfte Höhenzone fehlt")))?
                        .flaeche_m2 = wert;
                }
            }
        }
        Ok(())
    }
    /// Verbraucher verwenden nur den weiterhin aktuellen gespeicherten Quellstand.
    pub async fn raumbuch_planquellen_pruefen(
        &self,
        pid: ProjectId,
        r: &Raumbuch,
    ) -> CoreResult<()> {
        let mut aufgeloest = r.clone();
        self.planwerte_raumbuch(pid, &mut aufgeloest).await?;
        if serde_json::to_value(&aufgeloest).map_err(|e| fehler(e.to_string()))?
            != serde_json::to_value(r).map_err(|e| fehler(e.to_string()))?
        {
            return Err(fehler(
                "Raumflächen weichen vom Planaufmaß ab. Raumbuch aktualisieren.",
            ));
        }
        Ok(())
    }
}
