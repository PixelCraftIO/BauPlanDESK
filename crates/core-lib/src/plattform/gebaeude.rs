//! Öffentlicher Geometrievertrag für bekannte Gebäude- und Geschossdaten.
//! Keine rechtliche Anrechnung: GRZ/GFZ und Vollgeschosse bleiben bei ihren Fachmodulen.
use super::objekte::Gebaeude;
use crate::{CoreError, CoreResult, CoreServices, domain::ProjectId, flaechen::Bauteiltyp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const RECHENSTAND: &str = "gebaeudegrundlagen-1";
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GebaeudeGeometrie {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub typ: Option<Bauteiltyp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub grundflaeche_m2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub nullpunkt_m_nhn: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gelaende_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fundamentsohle_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub traufe_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub first_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub einzelgeschoss: Option<GeschossGeometrie>,
    /// Geprüfte Vollgeschosseinstufung des Einzelgeschosses (nur ohne einzeln
    /// erfasste Geschosse); `None` = noch zu prüfen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub einzelgeschoss_vollgeschoss: Option<bool>,
    /// Grundlage der Einstufung des Einzelgeschosses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub einzelgeschoss_grundlage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub herkunft: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GeschossGeometrie {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lichte_hoehe_einheitlich: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flaeche_m2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fussboden_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lichte_hoehe_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geschosshoehe_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub deckendicke_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub decke_oberkante_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub unterliegend_geschoss_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct GeschossGrundlagen {
    pub id: Uuid,
    pub bezeichnung: String,
    pub implizit: bool,
    pub flaeche_m2: Option<f64>,
    pub vollgeschoss: Option<bool>,
    pub lichte_hoehe_m: Option<f64>,
    pub decke_oberkante_m: Option<f64>,
    pub decke_ueber_gelaende_m: Option<f64>,
    pub unterliegend_m2: Option<f64>,
    pub herleitung: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct GebaeudeGrundlagen {
    pub rechenstand: String,
    pub gebaeude_id: Uuid,
    pub bezeichnung: String,
    pub typ: Option<Bauteiltyp>,
    pub grundflaeche_m2: Option<f64>,
    pub geschossflaechensumme_m2: Option<f64>,
    pub geschosse: Vec<GeschossGrundlagen>,
    pub offen: Vec<String>,
    /// Absolute Höhen (m ü. NHN) aus Gebäudenullpunkt und relativer Angabe;
    /// `None`, solange eine der beiden Angaben fehlt. Fehlt in älteren Ständen.
    #[serde(default)]
    pub hoehen_nhn: HoehenNhn,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct HoehenNhn {
    pub gelaende_m: Option<f64>,
    pub traufe_m: Option<f64>,
    pub first_m: Option<f64>,
}
fn fehler(s: &str) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn pruefe(v: Option<f64>, positiv: bool) -> CoreResult<()> {
    if v.is_some_and(|v| !v.is_finite() || v.abs() > 1e9 || (positiv && v < 0.)) {
        return Err(fehler(
            "Flächen und Höhen müssen endlich sein; Flächen und Abmessungen dürfen nicht negativ sein",
        ));
    }
    Ok(())
}
impl GeschossGeometrie {
    fn decke(&self) -> CoreResult<Option<f64>> {
        for v in [
            self.flaeche_m2,
            self.lichte_hoehe_m,
            self.geschosshoehe_m,
            self.deckendicke_m,
        ] {
            pruefe(v, true)?;
        }
        for v in [self.fussboden_m, self.decke_oberkante_m] {
            pruefe(v, false)?;
        }
        let abgeleitet = self
            .fussboden_m
            .zip(self.lichte_hoehe_m)
            .zip(self.deckendicke_m)
            .map(|((f, h), d)| f + h + d);
        if self
            .decke_oberkante_m
            .zip(abgeleitet)
            .is_some_and(|(a, b)| (a - b).abs() > 0.01)
        {
            return Err(fehler(
                "Deckenoberkante widerspricht Fußbodenhöhe + lichter Höhe + Deckendicke (Toleranz 1 cm)",
            ));
        }
        if self
            .geschosshoehe_m
            .zip(self.lichte_hoehe_m)
            .is_some_and(|(g, h)| g < h)
        {
            return Err(fehler(
                "Geschosshöhe darf die lichte Raumhöhe nicht unterschreiten",
            ));
        }
        if self
            .decke_oberkante_m
            .zip(self.fussboden_m)
            .is_some_and(|(d, f)| d < f)
        {
            return Err(fehler(
                "Deckenoberkante darf nicht unter dem Fußboden liegen",
            ));
        }
        Ok(self.decke_oberkante_m.or(abgeleitet))
    }
}
impl Gebaeude {
    /// Meter relativ zu ±0,00; nur der Nullpunkt selbst ist eine absolute NHN-Höhe.
    /// Ohne explizite Geschosse entsteht genau eine vorläufige Ebene mit Gebäude-ID.
    /// Explizite Geschosse erben niemals stillschweigend die Gebäudegrundfläche.
    pub fn grundlagen(&self, id: Uuid) -> CoreResult<GebaeudeGrundlagen> {
        let leer = GebaeudeGeometrie::default();
        let g = self.geometrie.as_ref().unwrap_or(&leer);
        pruefe(g.grundflaeche_m2, true)?;
        for v in [
            g.nullpunkt_m_nhn,
            g.gelaende_m,
            g.fundamentsohle_m,
            g.traufe_m,
            g.first_m,
        ] {
            pruefe(v, false)?;
        }
        if g.traufe_m.zip(g.first_m).is_some_and(|(t, f)| t > f) {
            return Err(fehler("Firsthöhe darf nicht unter der Traufhöhe liegen"));
        }
        if g.herkunft
            .as_ref()
            .is_some_and(|s| s.chars().count() > 4000)
        {
            return Err(fehler(
                "Herkunft der Gebäudeangaben: höchstens 4000 Zeichen",
            ));
        }
        if let Some(e) = &g.einzelgeschoss {
            e.decke()?;
        }
        if self.geschosse.len() > 200 {
            return Err(fehler("Höchstens 200 Geschosse je Gebäude"));
        }
        let mut ids = std::collections::HashSet::new();
        let mut geschosse = Vec::new();
        for s in &self.geschosse {
            if !ids.insert(s.id) {
                return Err(fehler("Geschoss-ID doppelt vergeben"));
            }
            let masse = s.geometrie.clone().unwrap_or_default();
            geschosse.push(self.geschossgrundlagen(
                s.id,
                &s.bezeichnung,
                false,
                &masse,
                s.vollgeschoss,
            )?);
        }
        if self.geschosse.is_empty() {
            let mut masse = g.einzelgeschoss.clone().unwrap_or_default();
            // Eine eigene Geschossfläche würde die zugesagte Grundflächenannahme verschleiern.
            masse.flaeche_m2 = g.grundflaeche_m2;
            if g.einzelgeschoss_vollgeschoss.is_some()
                && g.einzelgeschoss_grundlage
                    .as_deref()
                    .is_none_or(|t| t.trim().is_empty() || t.chars().count() > 4000)
            {
                return Err(fehler(
                    "Einstufung des Einzelgeschosses benötigt eine Grundlage (höchstens 4000 Zeichen)",
                ));
            }
            geschosse.push(self.geschossgrundlagen(
                id,
                "Einzelgeschoss",
                true,
                &masse,
                g.einzelgeschoss_vollgeschoss,
            )?);
        }
        for (i, s) in self.geschosse.iter().enumerate() {
            if let Some(u) = s
                .geometrie
                .as_ref()
                .and_then(|m| m.unterliegend_geschoss_id)
            {
                if u == s.id {
                    return Err(fehler("Ein Geschoss kann nicht unter sich selbst liegen"));
                }
                geschosse[i].unterliegend_m2 = geschosse
                    .iter()
                    .find(|v| v.id == u)
                    .ok_or_else(|| {
                        fehler("Unterliegendes Geschoss gehört nicht zu diesem Gebäude")
                    })?
                    .flaeche_m2;
            }
        }
        let summe = geschosse.iter().map(|s| s.flaeche_m2).sum::<Option<f64>>();
        if summe.is_some_and(|v| !v.is_finite()) {
            return Err(fehler("Geschossflächensumme außerhalb des Zahlenbereichs"));
        }
        let mut offen = vec![];
        if g.grundflaeche_m2.is_none() {
            offen.push("Gebäudegrundfläche noch unbekannt".into());
        }
        if g.typ.is_none() {
            offen.push("Gebäudetyp für GRZ noch zuordnen".into());
        }
        for s in &geschosse {
            if s.flaeche_m2.is_none() {
                offen.push(format!("{}: Geschossfläche noch unbekannt", s.bezeichnung));
            }
            if s.vollgeschoss.is_none() {
                offen.push(format!(
                    "{}: Vollgeschosseinstufung noch offen",
                    s.bezeichnung
                ));
            }
        }
        Ok(GebaeudeGrundlagen {
            rechenstand: RECHENSTAND.into(),
            gebaeude_id: id,
            bezeichnung: self.bezeichnung.clone(),
            typ: g.typ,
            grundflaeche_m2: g.grundflaeche_m2,
            geschossflaechensumme_m2: summe,
            geschosse,
            offen,
            hoehen_nhn: HoehenNhn {
                gelaende_m: g.nullpunkt_m_nhn.zip(g.gelaende_m).map(|(n, h)| n + h),
                traufe_m: g.nullpunkt_m_nhn.zip(g.traufe_m).map(|(n, h)| n + h),
                first_m: g.nullpunkt_m_nhn.zip(g.first_m).map(|(n, h)| n + h),
            },
        })
    }
    fn geschossgrundlagen(
        &self,
        id: Uuid,
        name: &str,
        implizit: bool,
        m: &GeschossGeometrie,
        vollgeschoss: Option<bool>,
    ) -> CoreResult<GeschossGrundlagen> {
        let decke = m.decke()?;
        let terrain = self.geometrie.as_ref().and_then(|g| g.gelaende_m);
        let herleitung = if m.decke_oberkante_m.is_some() {
            "Deckenoberkante erfasst"
        } else if decke.is_some() {
            "Deckenoberkante = OK Fertigfußboden + lichte Raumhöhe + Deckendicke"
        } else {
            "Deckenoberkante noch unbekannt"
        };
        Ok(GeschossGrundlagen {
            id,
            bezeichnung: name.into(),
            implizit,
            flaeche_m2: m.flaeche_m2,
            vollgeschoss,
            lichte_hoehe_m: m.lichte_hoehe_m,
            decke_oberkante_m: decke,
            decke_ueber_gelaende_m: decke.zip(terrain).map(|(d, t)| d - t),
            unterliegend_m2: None,
            herleitung: herleitung.into(),
        })
    }
}
impl CoreServices {
    pub async fn gebaeude_auswerten(&self, p: ProjectId, id: Uuid, g: &Gebaeude) -> CoreResult<GebaeudeGrundlagen> {
        self.gebaeude_planquellen_pruefen(p, id, g).await?;
        g.grundlagen(id)
    }
    pub async fn gebaeudegrundlagen(
        &self,
        p: ProjectId,
        id: Uuid,
    ) -> CoreResult<GebaeudeGrundlagen> {
        let f = self
            .fachobjekte(p, super::ObjektArt::Gebaeude)
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| fehler("Gebäude gehört nicht zu diesem Projekt"))?;
        let g: Gebaeude = serde_json::from_value(f.daten).map_err(|e| fehler(&e.to_string()))?;
        self.gebaeude_auswerten(p, id, &g).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn geb(daten: serde_json::Value) -> Gebaeude {
        serde_json::from_value(daten).unwrap()
    }
    fn alt() -> serde_json::Value {
        json!({"bezeichnung":"Wohnhaus01","grundstueck_ids":[],"geschosse":[],"quelle":null})
    }
    #[test]
    fn altprojekte_bleiben_unveraendert_und_fehlend_ist_nicht_nullflaeche() {
        let g = geb(alt());
        assert_eq!(serde_json::to_value(&g).unwrap(), alt());
        let r = g.grundlagen(Uuid::new_v4()).unwrap();
        assert_eq!(r.geschosse.len(), 1);
        assert_eq!(r.geschossflaechensumme_m2, None);
        let mut d = alt();
        d["geometrie"] = json!({"grundflaeche_m2":0});
        assert_eq!(
            geb(d)
                .grundlagen(Uuid::new_v4())
                .unwrap()
                .geschossflaechensumme_m2,
            Some(0.)
        );
    }
    #[test]
    fn wohnhaus_mit_vier_geschossen_und_garage_ohne_eg() {
        let id = Uuid::new_v4();
        let mut d = alt();
        d["geometrie"] = json!({"grundflaeche_m2":100,"typ":"hauptanlage"});
        d["geschosse"]=json!(["KG","EG","OG","DG"].map(|s|json!({"id":Uuid::new_v4(),"bezeichnung":s,"vollgeschoss":null,"grundlage":"","geometrie":{"flaeche_m2":100}})));
        let r = geb(d.clone()).grundlagen(id).unwrap();
        assert_eq!(r.grundflaeche_m2, Some(100.));
        assert_eq!(r.geschossflaechensumme_m2, Some(400.));
        assert_eq!(r.geschosse.len(), 4);
        assert!(
            r.geschosse
                .iter()
                .all(|s| !s.implizit && s.vollgeschoss.is_none())
        );
        d["geschosse"][3]["geometrie"] = json!({});
        assert_eq!(
            geb(d).grundlagen(id).unwrap().geschossflaechensumme_m2,
            None
        );
        let mut garage = alt();
        garage["geometrie"] = json!({"grundflaeche_m2":20,"typ":"garage"});
        let r = geb(garage).grundlagen(id).unwrap();
        assert_eq!(r.geschosse.len(), 1);
        assert!(r.geschosse[0].implizit);
        assert_eq!(r.geschosse[0].id, id);
        assert_eq!(r.geschossflaechensumme_m2, Some(20.));
    }
    #[test]
    fn kellerdecke_und_gelaende_mit_negativen_hoehen() {
        let mut d = alt();
        d["geometrie"] = json!({"grundflaeche_m2":100,"nullpunkt_m_nhn":125.5,"gelaende_m":-0.8,"fundamentsohle_m":-3.2,"einzelgeschoss":{"fussboden_m":-2.5,"lichte_hoehe_m":2.4,"deckendicke_m":0.3}});
        let r = geb(d.clone()).grundlagen(Uuid::new_v4()).unwrap();
        assert!((r.geschosse[0].decke_oberkante_m.unwrap() - 0.2).abs() < 1e-9);
        assert!((r.geschosse[0].decke_ueber_gelaende_m.unwrap() - 1.).abs() < 1e-9);
        d["geometrie"]["einzelgeschoss"]["decke_oberkante_m"] = json!(2.);
        assert!(geb(d.clone()).grundlagen(Uuid::new_v4()).is_err());
        d["geometrie"]["einzelgeschoss"] = json!({"fussboden_m":-2.5,"lichte_hoehe_m":2.4});
        assert_eq!(
            geb(d).grundlagen(Uuid::new_v4()).unwrap().geschosse[0].decke_oberkante_m,
            None
        );
    }
    #[test]
    fn negative_flaechen_und_fremde_vergleichsgeschosse_ablehnen() {
        let mut d = alt();
        d["geometrie"] = json!({"grundflaeche_m2":-1});
        assert!(geb(d.clone()).grundlagen(Uuid::new_v4()).is_err());
        d["geometrie"] = json!({"first_m":5,"traufe_m":6});
        assert!(geb(d.clone()).grundlagen(Uuid::new_v4()).is_err());
        d["geometrie"] = json!({});
        d["geschosse"] = json!([{"id":Uuid::new_v4(),"bezeichnung":"EG","vollgeschoss":null,"grundlage":"","geometrie":{"unterliegend_geschoss_id":Uuid::new_v4()}}]);
        assert!(geb(d).grundlagen(Uuid::new_v4()).is_err());
    }
}
