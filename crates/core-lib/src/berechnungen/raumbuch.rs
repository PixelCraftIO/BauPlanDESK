//! Explizite Übernahme von Raumfakten. Anrechnung/Klassifikation bleibt beim Zielmodul.
use super::*;
use crate::{CoreServices, domain::ProjectId, plattform::objekte::Raumbuch};
impl CoreServices {
    pub async fn berechnung_aus_raumbuch(
        &self,
        p: ProjectId,
        id: Uuid,
        modul: BerechnungsArt,
    ) -> CoreResult<FachPlan> {
        if !matches!(modul, BerechnungsArt::Wohnflaechen | BerechnungsArt::Din277) {
            return Err(err(
                "Raumfakten sind für dieses Modul nicht als direkter Import vorgesehen",
            ));
        }
        let f = self
            .fachobjekte(p, ObjektArt::Raumbuch)
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| err("Raumbuch gehört nicht zu diesem Projekt"))?;
        let r: Raumbuch = decode(&f.daten)?;
        self.raumbuch_planquellen_pruefen(p, &r).await?;
        let mut plan = vorlagen()?
            .into_iter()
            .find(|v| v.plan.modul == modul)
            .ok_or_else(|| err("Modulvorlage fehlt"))?
            .plan;
        plan.gebaeude_id = Some(r.gebaeude_id);
        plan.fachquellen.push(Fachquelle {
            art: ObjektArt::Raumbuch,
            objekt: id,
            revision: f.revision,
        });
        // DIN 277: Flächenart aus der Klassifikation der gebundenen Planmarkierung,
        // sonst aus der Raumbuch-Nutzung; Geschoss aus dem Gebäude.
        let mut din: std::collections::HashMap<Uuid, (&'static str, Option<Uuid>)> =
            std::collections::HashMap::new();
        let mut geschosse: std::collections::HashMap<Uuid, String> =
            std::collections::HashMap::new();
        if modul == BerechnungsArt::Din277 {
            for b in &r.planbindungen {
                let Some(raum) = b
                    .ziel
                    .strip_prefix("raeume.")
                    .and_then(|z| z.split('.').next())
                    .and_then(|z| z.parse::<Uuid>().ok())
                else {
                    continue;
                };
                let (_, m, _) = self.planbindung_aufloesen(p, b).await?;
                din.insert(
                    raum,
                    (flaechenart(m.bedeutung).unwrap_or(""), m.geschoss_id),
                );
            }
            if let Some(g) = self
                .fachobjekte(p, ObjektArt::Gebaeude)
                .await?
                .into_iter()
                .find(|f| f.objekt == r.gebaeude_id.to_string())
            {
                let g: crate::plattform::objekte::Gebaeude = decode(&g.daten)?;
                for (i, s) in g.geschosse.iter().enumerate() {
                    let name = match s.bezeichnung.trim() {
                        "" => format!("Geschoss {}", i + 1),
                        b => b.to_string(),
                    };
                    geschosse.insert(s.id, name);
                }
            }
        }
        let din_art = |raum: &crate::plattform::objekte::Raum| {
            din.get(&raum.id)
                .map(|(a, _)| *a)
                .filter(|a| !a.is_empty())
                .or_else(|| nutzungsart(&raum.nutzung))
                .unwrap_or("")
        };
        let din_geschoss = |raum: &crate::plattform::objekte::Raum| {
            din.get(&raum.id)
                .and_then(|(_, g)| *g)
                .or(r.geschoss_id)
                .and_then(|g| geschosse.get(&g).cloned())
                .unwrap_or_default()
        };
        plan.eingaben["positionen"]=serde_json::Value::Array(r.raeume.iter().map(|raum|{
    if modul==BerechnungsArt::Wohnflaechen {serde_json::json!({"id":raum.id.to_string(),"wohnung":"","raum":format!("{} {}",raum.nummer,raum.bezeichnung),"typ":"","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":raum.flaeche_m2,"lichte_hoehe_m":raum.hoehe_m,"sonderfaktor":null,"begruendung":"Raumart, Wohnungszuordnung und Höhenzonen prüfen","quelle":format!("Raumbuch {} · Revision {} · Raum {}",r.bezeichnung,f.revision,raum.nummer)})}
    else {serde_json::json!({"id":raum.id.to_string(),"geschoss":din_geschoss(raum),"raum":format!("{} {}",raum.nummer,raum.bezeichnung),"art":din_art(raum),"bereich":"R","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":raum.flaeche_m2,"quelle":format!("Raumbuch {} · Revision {} · Klassifikation und Maßbezug prüfen",r.bezeichnung,f.revision)})}
  }).collect());
        Ok(plan)
    }
}

/// DIN-277-Flächenart einer klassifizierten Planmarkierung.
fn flaechenart(b: crate::planaufmass::Planbedeutung) -> Option<&'static str> {
    use crate::planaufmass::Planbedeutung::*;
    match b {
        Nuf => Some("nuf"),
        Tf => Some("tf"),
        Vf => Some("vf"),
        Kgf => Some("kgf"),
        _ => None,
    }
}
/// DIN-277-Flächenart aus einer eindeutig klassifizierenden Raumbuch-Nutzung
/// (so führt die Planübernahme NUF/TF/VF); andere Nutzungen bleiben offen.
fn nutzungsart(nutzung: &str) -> Option<&'static str> {
    match nutzung.trim().to_lowercase().as_str() {
        "nuf" => Some("nuf"),
        "tf" | "technik" => Some("tf"),
        "vf" | "verkehr" => Some("vf"),
        "kgf" => Some("kgf"),
        _ => None,
    }
}
