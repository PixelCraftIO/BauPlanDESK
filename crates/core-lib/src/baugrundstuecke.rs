//! Flurstücke, grundbuchliche Grundstückseinheiten und getrennte Nutzungsnachweise.
use crate::{
    CoreError, CoreResult,
    domain::Planungsrecht,
    festsetzungen::{Festsetzungen, NutzungsEingabe, NutzungsPruefung},
    flaechen::GrundstueckAufstellung,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaechenAbzug {
    pub art: String,
    pub flaeche_m2: f64,
    pub grundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbweichendeZuordnung {
    pub kennung: String,
    pub grundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flurstueck {
    pub id: String,
    pub gemarkung: String,
    pub flur: String,
    pub nummer: String,
    pub grundbuchbezirk: String,
    pub grundbuchblatt: String,
    pub laufende_nummer: u32,
    pub flaeche_m2: f64,
    pub abzuege: Vec<FlaechenAbzug>,
    #[serde(default)]
    pub abweichende_zuordnung: Option<AbweichendeZuordnung>,
}
impl Flurstueck {
    pub fn schluessel(&self) -> String {
        if let Some(a) = &self.abweichende_zuordnung {
            return format!("zuordnung:{}", a.kennung.trim().to_lowercase());
        }
        serde_json::to_string(&(
            self.grundbuchbezirk.trim().to_lowercase(),
            self.grundbuchblatt.trim().to_lowercase(),
            self.laufende_nummer,
        ))
        .expect("Grundbuchschlüssel")
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaugrundstueckBerechnung {
    pub schluessel: String,
    pub eingabe: NutzungsEingabe,
    pub eigene_festsetzungen: Option<Festsetzungen>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BaugrundstuecksPlan {
    pub flurstuecke: Vec<Flurstueck>,
    pub berechnungen: Vec<BaugrundstueckBerechnung>,
}
/// Identität hängt an den zugeordneten Flurstücken, nicht an deren veränderlichen Grundbuchtexten.
pub fn einheitsschluessel(ids: &[String]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    let bytes = serde_json::to_vec(&ids).expect("Flurstückidentitäten");
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, &bytes).to_string()
}
impl<'de> Deserialize<'de> for BaugrundstuecksPlan {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Daten {
            flurstuecke: Vec<Flurstueck>,
            berechnungen: Vec<BaugrundstueckBerechnung>,
        }
        let Daten {
            flurstuecke,
            mut berechnungen,
        } = Daten::deserialize(d)?;
        let mut alt = std::collections::BTreeMap::<String, Vec<String>>::new();
        for f in &flurstuecke {
            alt.entry(f.schluessel()).or_default().push(f.id.clone());
        }
        for b in &mut berechnungen {
            if let Some(ids) = alt.get(&b.schluessel) {
                b.schluessel = einheitsschluessel(ids);
            }
        }
        Ok(Self {
            flurstuecke,
            berechnungen,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaugrundstueckGruppe {
    pub schluessel: String,
    pub bezeichnung: String,
    pub flurstueck_ids: Vec<String>,
    pub eigentumsflaeche_m2: f64,
    pub abzuege_m2: f64,
    pub bauland_m2: f64,
}
#[derive(Debug, Serialize)]
pub struct BaugrundstueckNachweis {
    pub grundstueck: BaugrundstueckGruppe,
    pub pruefung: NutzungsPruefung,
}
#[derive(Debug, Serialize)]
pub struct BaugrundstuecksPruefung {
    pub projekt_version: i64,
    pub baugrundstuecke: Vec<BaugrundstueckNachweis>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AbzugsRegel {
    pub bezeichnung: String,
    pub bebauungsplan_erforderlich: bool,
}
pub fn abzugsregeln() -> &'static BTreeMap<String, AbzugsRegel> {
    static REGELN: std::sync::OnceLock<BTreeMap<String, AbzugsRegel>> = std::sync::OnceLock::new();
    REGELN.get_or_init(|| {
        serde_json::from_str(include_str!("../../../regelwerke/grundstueck-abzuege.json"))
            .expect("Ungültige Grundstücks-Abzugsregeln")
    })
}
fn text_gueltig(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 500
}
impl BaugrundstuecksPlan {
    pub fn gruppen(&self, mit_plan: bool) -> CoreResult<Vec<BaugrundstueckGruppe>> {
        if self.flurstuecke.is_empty() || self.flurstuecke.len() > 1000 {
            return Err(CoreError::InvalidInput(
                "1 bis 1000 Flurstücke erfassen".into(),
            ));
        }
        let mut ids = HashSet::new();
        let mut kataster = HashSet::new();
        let mut gruppen = BTreeMap::<String, BaugrundstueckGruppe>::new();
        for f in &self.flurstuecke {
            if !text_gueltig(&f.id)
                || !text_gueltig(&f.gemarkung)
                || !text_gueltig(&f.nummer)
                || f.grundbuchbezirk.len() > 500
                || f.grundbuchblatt.len() > 500
                || f.flur.len() > 500
                || !f.flaeche_m2.is_finite()
                || f.flaeche_m2 <= 0.0
                || f.abzuege.len() > 1000
            {
                return Err(CoreError::InvalidInput(
                    "Flurstück benötigt gültige Katasterangaben und eine positive Fläche".into(),
                ));
            }
            if f.abweichende_zuordnung.is_none()
                && (f.grundbuchbezirk.trim().is_empty()
                    || f.grundbuchblatt.trim().is_empty()
                    || f.laufende_nummer == 0)
            {
                return Err(CoreError::InvalidInput(format!(
                    "Baugrundstückszuordnung für Flurstück {} noch ungeklärt: Grundbuchangaben ergänzen oder eine begründete besondere Zuordnung erfassen",
                    f.nummer
                )));
            }
            if !ids.insert(&f.id)
                || !kataster.insert((
                    f.gemarkung.trim().to_lowercase(),
                    f.flur.trim().to_lowercase(),
                    f.nummer.trim().to_lowercase(),
                ))
            {
                return Err(CoreError::InvalidInput(
                    "Flurstück wurde mehrfach erfasst".into(),
                ));
            }
            if let Some(a) = &f.abweichende_zuordnung
                && (!text_gueltig(&a.kennung)
                    || a.grundlage.trim().is_empty()
                    || a.grundlage.len() > 4000)
            {
                return Err(CoreError::InvalidInput(
                    "Abweichende Grundstückszuordnung benötigt Kennung und rechtliche Grundlage"
                        .into(),
                ));
            }
            let mut abzug = 0.0;
            for a in &f.abzuege {
                let regel = abzugsregeln()
                    .get(&a.art)
                    .ok_or_else(|| CoreError::InvalidInput("Unbekannter Flächenabzug".into()))?;
                if (regel.bebauungsplan_erforderlich && !mit_plan)
                    || !a.flaeche_m2.is_finite()
                    || a.flaeche_m2 < 0.0
                    || a.grundlage.trim().is_empty()
                    || a.grundlage.len() > 4000
                {
                    return Err(CoreError::InvalidInput("Flächenabzug benötigt Fläche und Grundlage; Planfestsetzungen benötigen einen Bebauungsplan".into()));
                }
                abzug += a.flaeche_m2;
            }
            if !abzug.is_finite() || abzug > f.flaeche_m2 {
                return Err(CoreError::InvalidInput(
                    "Abzüge überschreiten die Fläche des Flurstücks".into(),
                ));
            }
            let key = f.schluessel();
            let g = gruppen
                .entry(key.clone())
                .or_insert_with(|| BaugrundstueckGruppe {
                    schluessel: key,
                    bezeichnung: f
                        .abweichende_zuordnung
                        .as_ref()
                        .map(|a| format!("Zuordnung {}", a.kennung))
                        .unwrap_or_else(|| {
                            format!(
                                "{} · Blatt {} · lfd. Nr. {}",
                                f.grundbuchbezirk, f.grundbuchblatt, f.laufende_nummer
                            )
                        }),
                    flurstueck_ids: vec![],
                    eigentumsflaeche_m2: 0.0,
                    abzuege_m2: 0.0,
                    bauland_m2: 0.0,
                });
            g.flurstueck_ids.push(f.id.clone());
            g.eigentumsflaeche_m2 += f.flaeche_m2;
            g.abzuege_m2 += abzug;
            g.bauland_m2 = g.eigentumsflaeche_m2 - g.abzuege_m2;
        }
        if gruppen.len() > 100
            || gruppen.values().any(|g| {
                !g.eigentumsflaeche_m2.is_finite()
                    || !g.abzuege_m2.is_finite()
                    || !g.bauland_m2.is_finite()
            })
        {
            return Err(CoreError::InvalidInput(
                "Zu viele Baugrundstücke oder nicht darstellbare Flächensummen".into(),
            ));
        }
        Ok(gruppen
            .into_values()
            .map(|mut g| {
                g.schluessel = einheitsschluessel(&g.flurstueck_ids);
                g
            })
            .collect())
    }
    pub fn pruefe(
        &self,
        plan: &Planungsrecht,
        version: i64,
    ) -> CoreResult<BaugrundstuecksPruefung> {
        self.pruefe_mit_regeln(plan, version, crate::baunvo::regelwerke())
    }
    pub fn pruefe_mit_regeln(
        &self,
        plan: &Planungsrecht,
        version: i64,
        regeln: &[crate::baunvo::Regelwerk],
    ) -> CoreResult<BaugrundstuecksPruefung> {
        let mit_plan = matches!(
            plan.bebauungsplan_art,
            crate::domain::BebauungsplanArt::Qualifiziert
                | crate::domain::BebauungsplanArt::Einfach
                | crate::domain::BebauungsplanArt::Vorhabenbezogen
        );
        let gruppen = self.gruppen(mit_plan)?;
        if self.berechnungen.len() != gruppen.len() {
            return Err(CoreError::InvalidInput(
                "Für jedes Baugrundstück genau eine Berechnung anlegen".into(),
            ));
        }
        let mut keys = HashSet::new();
        let mut ergebnisse = vec![];
        for g in gruppen {
            let b = self
                .berechnungen
                .iter()
                .find(|b| b.schluessel == g.schluessel)
                .ok_or_else(|| {
                    CoreError::InvalidInput(format!("Berechnung fehlt für {}", g.bezeichnung))
                })?;
            if !keys.insert(&b.schluessel) {
                return Err(CoreError::InvalidInput(
                    "Baugrundstück mehrfach berechnet".into(),
                ));
            }
            let mut e = b.eingabe.clone();
            let a = e.aufstellung.as_mut().ok_or_else(|| {
                CoreError::InvalidInput(
                    "Baugrundstücke benötigen eine Bauteil- und Geschossaufstellung".into(),
                )
            })?;
            let grond = a.grundstueck.get_or_insert_with(|| GrundstueckAufstellung {
                basisflaeche_m2: 0.0,
                nicht_massgebend_m2: 0.0,
                abzug_begruendung: String::new(),
                gemeinschaftsanteile: vec![],
            });
            // Bezugsfläche ausschließlich aus Flurstücken ableiten; gespeicherte Summen niemals erneut abziehen.
            grond.basisflaeche_m2 = g.eigentumsflaeche_m2;
            grond.nicht_massgebend_m2 = g.abzuege_m2;
            grond.abzug_begruendung = "Einzelne Flurstücksabzüge".into();
            let p = Planungsrecht {
                festgesetzte_werte: b
                    .eigene_festsetzungen
                    .clone()
                    .unwrap_or_else(|| plan.festgesetzte_werte.clone()),
                ..plan.clone()
            };
            let r = crate::festsetzungen::pruefe_mit_regeln(&e, &p, version, regeln)?;
            ergebnisse.push(BaugrundstueckNachweis {
                grundstueck: g,
                pruefung: r,
            });
        }
        // Eine externe Gemeinschaftsanlage darf über mehrere Grundstücke insgesamt nicht über 100 % zugeordnet werden.
        let mut anteile = BTreeMap::<String, (f64, f64)>::new();
        for b in &self.berechnungen {
            if let Some(g) = b
                .eingabe
                .aufstellung
                .as_ref()
                .and_then(|a| a.grundstueck.as_ref())
            {
                for a in &g.gemeinschaftsanteile {
                    let x = anteile
                        .entry(a.bezeichnung.trim().to_lowercase())
                        .or_insert((a.gesamtflaeche_m2, 0.0));
                    if (x.0 - a.gesamtflaeche_m2).abs() > 1e-9 {
                        return Err(CoreError::InvalidInput(
                            "Gemeinschaftsanlage hat widersprüchliche Gesamtflächen".into(),
                        ));
                    }
                    x.1 += a.anteil_prozent;
                    if x.1 > 100.0 + 1e-9 {
                        return Err(CoreError::InvalidInput(
                            "Gemeinschaftsanlage wurde insgesamt mit mehr als 100 % zugeordnet"
                                .into(),
                        ));
                    }
                }
            }
        }
        Ok(BaugrundstuecksPruefung {
            projekt_version: version,
            baugrundstuecke: ergebnisse,
        })
    }
}

#[cfg(any(test, feature = "testdaten"))]
pub fn musterplan() -> BaugrundstuecksPlan {
    let flur = |id: &str, nummer: &str, lfd: u32, flaeche: f64, abzug: f64| Flurstueck {
        id: id.into(),
        gemarkung: "München".into(),
        flur: "1".into(),
        nummer: nummer.into(),
        grundbuchbezirk: "München".into(),
        grundbuchblatt: "42".into(),
        laufende_nummer: lfd,
        flaeche_m2: flaeche,
        abzuege: if abzug > 0. {
            vec![FlaechenAbzug {
                art: "private_gruenflaeche".into(),
                flaeche_m2: abzug,
                grundlage: "Planfestsetzung private Grünfläche".into(),
            }]
        } else {
            vec![]
        },
        abweichende_zuordnung: None,
    };
    let mut plan = BaugrundstuecksPlan {
        flurstuecke: vec![
            flur("a", "100/1", 1, 300., 50.),
            flur("b", "100/2", 1, 200., 20.),
            flur("c", "101/1", 2, 400., 0.),
        ],
        berechnungen: vec![],
    };
    for g in plan.gruppen(true).unwrap() {
        let gr = if g.flurstueck_ids.len() == 2 {
            100.
        } else {
            200.
        };
        let e=serde_json::from_value(serde_json::json!({"gebiet":"WA","mass":{"grundstuecksflaeche_m2":9999,"grundflaeche_m2":9999,"grundflaeche_nebenanlagen_m2":9999,"geschossflaeche_m2":9999,"baumasse_m3":1000},"aufstellung":{"bau_nvo":"2017","bauteile":[{"bezeichnung":"Haus","typ":"hauptanlage","flaeche_m2":gr,"abweichende_anrechnung":null,"begruendung":""}],"geschosse":[{"bezeichnung":"EG","vollgeschoss":true,"aussenmass_m2":gr*2.,"aufenthalt_m2":0,"waende_m2":0,"erschliessung_m2":0}],"nichtvollgeschoss_faktor":0,"nichtvollgeschoss_festsetzung":""}})).unwrap();
        plan.berechnungen.push(BaugrundstueckBerechnung {
            schluessel: g.schluessel,
            eingabe: e,
            eigene_festsetzungen: None,
        });
    }
    plan
}
#[cfg(test)]
mod tests {
    use super::*;
    fn planrecht() -> Planungsrecht {
        Planungsrecht {
            bebauungsplan_art: crate::domain::BebauungsplanArt::Qualifiziert,
            festgesetzte_werte: Festsetzungen {
                grz: Some(0.3),
                gfz: Some(0.6),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn grundbuchkorrektur_erhaelt_identitaet_aufteilung_aendert_sie() {
        let mut plan = musterplan();
        let alt = plan.gruppen(true).unwrap();
        for f in &mut plan.flurstuecke {
            f.grundbuchblatt = "Korrigiertes Blatt".into();
        }
        let neu = plan.gruppen(true).unwrap();
        assert_eq!(
            alt.iter().map(|g| &g.schluessel).collect::<Vec<_>>(),
            neu.iter().map(|g| &g.schluessel).collect::<Vec<_>>()
        );
        plan.pruefe(&planrecht(), 1).unwrap();
        plan.flurstuecke[0].laufende_nummer = 3;
        assert!(plan.pruefe(&planrecht(), 1).is_err());
    }
    #[test]
    fn rechtskontext_bleibt_in_jeder_grundstueckspruefung_erhalten() {
        for grundlage in [
            crate::domain::Rechtsgrundlage::Par34,
            crate::domain::Rechtsgrundlage::Par35,
        ] {
            let mut plan = musterplan();
            for f in &mut plan.flurstuecke {
                f.abzuege.clear();
            }
            let recht = Planungsrecht {
                rechtsgrundlage: Some(grundlage),
                bebauungsplan_art: crate::domain::BebauungsplanArt::Kein,
                ..Default::default()
            };
            let gruppiert = plan.pruefe(&recht, 1).unwrap();
            for (eingabe, nachweis) in plan.berechnungen.iter().zip(&gruppiert.baugrundstuecke) {
                let direkt = crate::festsetzungen::pruefe(&eingabe.eingabe, &recht, 1).unwrap();
                for art in ["grz", "gfz", "bmz"] {
                    let a = direkt.nachweise.iter().find(|n| n.art == art).unwrap();
                    let b = nachweis
                        .pruefung
                        .nachweise
                        .iter()
                        .find(|n| n.art == art)
                        .unwrap();
                    assert_eq!((a.soll, a.quelle), (b.soll, b.quelle));
                }
            }
        }
    }
    #[test]
    fn gleiche_einheit_zusammen_unterschiedliche_getrennt_mit_eigenen_kennzahlen() {
        let p = musterplan();
        let r = p.pruefe(&planrecht(), 8).unwrap();
        assert_eq!(r.baugrundstuecke.len(), 2);
        let a = &r.baugrundstuecke[0];
        let b = &r.baugrundstuecke[1];
        assert_eq!(a.grundstueck.eigentumsflaeche_m2, 500.);
        assert_eq!(a.grundstueck.abzuege_m2, 70.);
        assert_eq!(a.grundstueck.bauland_m2, 430.);
        assert_eq!(a.grundstueck.flurstueck_ids.len(), 2);
        assert!((a.pruefung.kennzahlen.grz - 100. / 430.).abs() < 1e-10);
        assert!(a.pruefung.nachweise[0].eingehalten.unwrap());
        assert_eq!(b.pruefung.kennzahlen.grz, 0.5);
        assert_eq!(b.pruefung.kennzahlen.gfz, 1.0);
        assert_eq!(b.pruefung.nachweise[0].eingehalten, Some(false));
        assert_eq!(
            b.pruefung.nachweise[0].flaechenueberschreitung_m2,
            Some(80.)
        );
        assert_eq!(r.projekt_version, 8);
        assert_eq!(
            p.pruefe(&planrecht(), 8).unwrap().baugrundstuecke[0]
                .grundstueck
                .bauland_m2,
            430.
        );
    }
    #[test]
    fn laufende_nummer_allein_reicht_nicht() {
        let mut p = musterplan();
        p.flurstuecke[1].grundbuchblatt = "43".into();
        assert_eq!(p.gruppen(true).unwrap().len(), 3);
        p.flurstuecke[1].grundbuchblatt = "42".into();
        p.flurstuecke[1].grundbuchbezirk = "Anderer Bezirk".into();
        assert_eq!(p.gruppen(true).unwrap().len(), 3);
    }
    #[test]
    fn fehlende_und_doppelte_daten_werden_abgelehnt() {
        let mut p = musterplan();
        p.flurstuecke[0].grundbuchblatt.clear();
        assert!(p.gruppen(true).is_err());
        p = musterplan();
        p.flurstuecke.push(p.flurstuecke[0].clone());
        assert!(p.gruppen(true).is_err());
        p = musterplan();
        p.flurstuecke[0].abzuege[0].flaeche_m2 = 301.;
        assert!(p.gruppen(true).is_err());
        assert!(musterplan().gruppen(false).is_err());
    }
    #[test]
    fn verwaiste_oder_fehlende_berechnungen_werden_nicht_gemischt() {
        let mut p = musterplan();
        p.berechnungen.pop();
        assert!(p.pruefe(&planrecht(), 1).is_err());
        p = musterplan();
        p.flurstuecke[2].laufende_nummer = 1;
        assert!(p.pruefe(&planrecht(), 1).is_err());
    }
    #[test]
    fn eigene_festsetzungen_wirken_nur_fuer_dieses_grundstueck() {
        let mut p = musterplan();
        p.berechnungen[1].eigene_festsetzungen = Some(Festsetzungen {
            grz: Some(0.6),
            ..Default::default()
        });
        let r = p.pruefe(&planrecht(), 1).unwrap();
        assert_eq!(r.baugrundstuecke[0].pruefung.nachweise[0].soll, Some(0.3));
        assert_eq!(
            r.baugrundstuecke[1].pruefung.nachweise[0].eingehalten,
            Some(true)
        );
    }
    #[test]
    fn besondere_zuordnung_benoetigt_grundlage() {
        let mut p = musterplan();
        for f in &mut p.flurstuecke {
            f.abweichende_zuordnung = Some(AbweichendeZuordnung {
                kennung: "Baugrundstück A".into(),
                grundlage: "Dokumentierte rechtliche Zuordnung".into(),
            });
        }
        assert_eq!(p.gruppen(true).unwrap().len(), 1);
        p.flurstuecke[0]
            .abweichende_zuordnung
            .as_mut()
            .unwrap()
            .grundlage
            .clear();
        assert!(p.gruppen(true).is_err());
    }
    #[test]
    fn gemeinschaftsanlage_darf_nicht_ueber_mehrere_gruppen_ueberverteilt_werden() {
        let mut p = musterplan();
        for b in &mut p.berechnungen {
            b.eingabe.aufstellung.as_mut().unwrap().grundstueck = Some(GrundstueckAufstellung {
                basisflaeche_m2: 9999.,
                nicht_massgebend_m2: 9999.,
                abzug_begruendung: String::new(),
                gemeinschaftsanteile: vec![crate::flaechen::Gemeinschaftsanteil {
                    bezeichnung: "Spielplatz".into(),
                    gesamtflaeche_m2: 400.,
                    anteil_prozent: 60.,
                    festsetzung: "Planzuordnung".into(),
                    im_plangebiet: true,
                    ausserhalb_baugrundstueck: true,
                }],
            });
        }
        assert!(p.pruefe(&planrecht(), 1).is_err());
        p.berechnungen[0]
            .eingabe
            .aufstellung
            .as_mut()
            .unwrap()
            .grundstueck
            .as_mut()
            .unwrap()
            .gemeinschaftsanteile[0]
            .anteil_prozent = 40.;
        let r = p.pruefe(&planrecht(), 1).unwrap();
        assert_eq!(
            r.baugrundstuecke[0]
                .pruefung
                .aufstellung
                .as_ref()
                .unwrap()
                .mass
                .grundstuecksflaeche_m2,
            590.
        );
        assert_eq!(
            r.baugrundstuecke[1]
                .pruefung
                .aufstellung
                .as_ref()
                .unwrap()
                .mass
                .grundstuecksflaeche_m2,
            640.
        );
    }
}
