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
        plan.eingaben["positionen"]=serde_json::Value::Array(r.raeume.iter().map(|raum|{
    if modul==BerechnungsArt::Wohnflaechen {serde_json::json!({"id":raum.id.to_string(),"wohnung":"","raum":format!("{} {}",raum.nummer,raum.bezeichnung),"typ":"","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":raum.flaeche_m2,"lichte_hoehe_m":raum.hoehe_m,"sonderfaktor":null,"begruendung":"Raumart, Wohnungszuordnung und Höhenzonen prüfen","quelle":format!("Raumbuch {} · Revision {} · Raum {}",r.bezeichnung,f.revision,raum.nummer)})}
    else {serde_json::json!({"id":raum.id.to_string(),"geschoss":"","raum":format!("{} {}",raum.nummer,raum.bezeichnung),"art":"","bereich":"R","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":raum.flaeche_m2,"quelle":format!("Raumbuch {} · Revision {} · Klassifikation und Maßbezug prüfen",r.bezeichnung,f.revision)})}
  }).collect());
        Ok(plan)
    }
}
