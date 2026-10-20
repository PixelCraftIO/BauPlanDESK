use super::*;
use crate::{
    CoreServices,
    domain::ProjectId,
    plattform::{
        ObjektArt,
        objekte::{Raum, Raumbuch},
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
            if t.len() != 3
                || t[0] != "raeume"
                || t[2] != "flaeche_m2"
                || b.groesse != Planmessgroesse::Flaeche
            {
                return Err(fehler(
                    "Raumbuch unterstützt Planflächen über stabile Raum-IDs",
                ));
            }
            let (_, m, wert) = self.planbindung_aufloesen(pid, &b).await?;
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
            raum.flaeche_m2 = wert;
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
