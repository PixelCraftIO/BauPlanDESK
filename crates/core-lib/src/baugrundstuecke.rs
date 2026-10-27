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
    /// Stabile Zeilenkennung für Planbindungen (`abzuege.<id>.flaeche_m2`);
    /// bei Übernahme aus dem Aufmaß die Markierungs-ID. Altdaten ohne ID
    /// bleiben gültig, können aber nicht gebunden werden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub art: String,
    pub flaeche_m2: f64,
    pub grundlage: String,
    /// Fläche aus einer Markierung des Aufmaßes (Bedeutung passend zur
    /// Art, dem Flurstück zugeordnet). Setzt der Core; siehe `planaufmass_abzuege`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planbindung: Option<crate::planaufmass::Planbindung>,
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
    pub fn hat_grundbuchzuordnung(&self) -> bool {
        !self.grundbuchbezirk.trim().is_empty()
            && !self.grundbuchblatt.trim().is_empty()
            && self.laufende_nummer > 0
    }
    pub fn schluessel(&self) -> String {
        if let Some(a) = &self.abweichende_zuordnung {
            return format!("zuordnung:{}", a.kennung.trim().to_lowercase());
        }
        if !self.hat_grundbuchzuordnung() {
            return format!("flurstueck:{}", self.id);
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
    /// Flurstücke der Einheit beim letzten Speichern. Erlaubt nach einer
    /// Umgruppierung die eindeutige Nachfolge (siehe
    /// [`BaugrundstueckGruppe::vorgaenger`]); fehlt in älteren Ständen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flurstueck_ids: Vec<String>,
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
    /// Nicht mehr bestehende Berechnungen (Schlüssel), deren verbliebene
    /// Flurstücke vollständig in dieser Einheit liegen, ohne dass eine andere
    /// Einheit sie ebenfalls beerbt. Deren Eingaben können übernommen werden.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vorgaenger: Vec<String>,
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
    /// Grundstückseinheit (Schlüssel) je Flurstück, gebildet wie in
    /// [`Self::gruppen`], jedoch ohne Validierung der Angaben.
    pub fn einheit_je_flurstueck(&self) -> BTreeMap<String, String> {
        let mut gruppen = BTreeMap::<String, Vec<String>>::new();
        for f in &self.flurstuecke {
            gruppen
                .entry(f.schluessel())
                .or_default()
                .push(f.id.clone());
        }
        gruppen
            .into_values()
            .flat_map(|ids| {
                let s = einheitsschluessel(&ids);
                ids.into_iter().map(move |id| (id, s.clone()))
            })
            .collect()
    }
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
            let mut abzug_ids = HashSet::new();
            for a in &f.abzuege {
                if let Some(id) = &a.id
                    && (!text_gueltig(id) || !abzug_ids.insert(id.clone()))
                {
                    return Err(CoreError::InvalidInput(
                        "Flächenabzug: Kennung ungültig oder doppelt".into(),
                    ));
                }
                if let Some(b) = &a.planbindung
                    && (a.id.is_none()
                        || b.ziel
                            != format!("abzuege.{}.flaeche_m2", a.id.as_deref().unwrap_or(""))
                        || b.groesse != crate::planaufmass::Planmessgroesse::Flaeche)
                {
                    return Err(CoreError::InvalidInput(
                        "Flächenabzug: Aufmaßverknüpfung gehört zu einem anderen Ziel".into(),
                    ));
                }
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
                            if f.hat_grundbuchzuordnung() {
                                format!(
                                    "{} · Blatt {} · lfd. Nr. {}",
                                    f.grundbuchbezirk, f.grundbuchblatt, f.laufende_nummer
                                )
                            } else {
                                format!(
                                    "{} · Flur {} · Flurstück {} (einzeln angesetzt)",
                                    f.gemarkung,
                                    if f.flur.is_empty() { "—" } else { &f.flur },
                                    f.nummer
                                )
                            }
                        }),
                    flurstueck_ids: vec![],
                    eigentumsflaeche_m2: 0.0,
                    abzuege_m2: 0.0,
                    bauland_m2: 0.0,
                    vorgaenger: vec![],
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
        let mut gruppen: Vec<BaugrundstueckGruppe> = gruppen
            .into_values()
            .map(|mut g| {
                g.schluessel = einheitsschluessel(&g.flurstueck_ids);
                g
            })
            .collect();
        // Eindeutige Nachfolge verwaister Berechnungen: Alle verbliebenen
        // Flurstücke liegen in genau einer Einheit (Hinzunahme oder Wegfall
        // einzelner Flurstücke). Aufteilungen bleiben ausdrückliche Entscheidung.
        for b in &self.berechnungen {
            if b.flurstueck_ids.is_empty() || gruppen.iter().any(|g| g.schluessel == b.schluessel) {
                continue;
            }
            let ziele: HashSet<usize> = b
                .flurstueck_ids
                .iter()
                .filter_map(|f| gruppen.iter().position(|g| g.flurstueck_ids.contains(f)))
                .collect();
            if let [ziel] = ziele.into_iter().collect::<Vec<_>>().as_slice() {
                gruppen[*ziel].vorgaenger.push(b.schluessel.clone());
            }
        }
        for g in &mut gruppen {
            // Mehrere Vorgänger (Zusammenlegung) bleiben eine Entscheidung.
            if g.vorgaenger.len() > 1 {
                g.vorgaenger.clear();
            }
        }
        Ok(gruppen)
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
        self.pruefe_auswahl(plan, version, regeln, None)
    }
    /// Liefert ausschließlich das Ergebnis der gewählten Einheit. Alle
    /// Grundstücksidentitäten und gemeinschaftlichen Zuordnungen bleiben im
    /// Kontext des vollständigen Plans erhalten.
    pub fn pruefe_einheit(
        &self,
        plan: &Planungsrecht,
        version: i64,
        schluessel: &str,
    ) -> CoreResult<BaugrundstuecksPruefung> {
        self.pruefe_auswahl(plan, version, crate::baunvo::regelwerke(), Some(schluessel))
    }
    fn pruefe_auswahl(
        &self,
        plan: &Planungsrecht,
        version: i64,
        regeln: &[crate::baunvo::Regelwerk],
        auswahl: Option<&str>,
    ) -> CoreResult<BaugrundstuecksPruefung> {
        let mit_plan = matches!(
            plan.bebauungsplan_art,
            crate::domain::BebauungsplanArt::Qualifiziert
                | crate::domain::BebauungsplanArt::Einfach
                | crate::domain::BebauungsplanArt::Vorhabenbezogen
        );
        let gruppen = self.gruppen(mit_plan)?;
        if auswahl.is_some_and(|s| !gruppen.iter().any(|g| g.schluessel == s)) {
            return Err(CoreError::InvalidInput(
                "Baugrundstück nicht vorhanden".into(),
            ));
        }
        if self.berechnungen.len() != gruppen.len() {
            return Err(CoreError::InvalidInput(
                "Für jedes Baugrundstück genau eine Berechnung anlegen".into(),
            ));
        }
        let mut keys = HashSet::new();
        let mut ergebnisse = vec![];
        let anzahl = gruppen.len();
        let schluessel: HashSet<String> = gruppen.iter().map(|g| g.schluessel.clone()).collect();
        let s_aktuell = |s: &str| schluessel.contains(s);
        for g in gruppen {
            if auswahl.is_some_and(|s| g.schluessel != s) {
                continue;
            }
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
            // Der Umrissvergleich mit dem Baufenster gilt nur für das Grundstück,
            // dem der Vorhabenumriss zugeordnet ist (ohne Angabe nur bei einem).
            let baugrenzen =
                plan.baugrenzen
                    .as_ref()
                    .filter(|bg| match bg.grundstueck_schluessel.as_deref() {
                        Some(s) => s == g.schluessel,
                        None => anzahl == 1,
                    });
            let p = Planungsrecht {
                festgesetzte_werte: b
                    .eigene_festsetzungen
                    .clone()
                    .unwrap_or_else(|| plan.festgesetzte_werte.clone()),
                baugrenzen: baugrenzen.cloned(),
                ..plan.clone()
            };
            let mut r = crate::festsetzungen::pruefe_mit_regeln(&e, &p, version, regeln)?;
            if let Some(bg) = &plan.baugrenzen
                && baugrenzen.is_none()
                && mit_plan
            {
                r.hinweise.push(match &bg.grundstueck_schluessel {
                    Some(s) if s_aktuell(s) => "Lage im Baufenster: Der Vorhabenumriss gehört zu einem anderen Baugrundstück und wird hier nicht geprüft.".into(),
                    _ => "Lage im Baufenster nicht geprüft: Der Vorhabenumriss ist keinem aktuellen Baugrundstück zugeordnet. Unter Pläne → Baugrenzenprüfung das Baugrundstück wählen und speichern.".into(),
                });
            }
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
                id: None,
                planbindung: None,
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
            flurstueck_ids: g.flurstueck_ids,
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
            bau_nvo: Some(crate::flaechen::BauNvo::V2017),
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
    fn ohne_grundbuch_einzeln_teilangaben_nicht_zusammenfassen() {
        let mut p = musterplan();
        for f in &mut p.flurstuecke {
            f.grundbuchbezirk.clear();
            f.grundbuchblatt.clear();
            f.laufende_nummer = 0;
        }
        let einzeln = p.gruppen(true).unwrap();
        assert_eq!(einzeln.len(), 3);
        assert!(einzeln.iter().all(|g| g.flurstueck_ids.len() == 1));
        assert_eq!(einzeln.iter().map(|g| g.bauland_m2).sum::<f64>(), 830.);
        let vorlage = p.berechnungen[0].clone();
        p.berechnungen = einzeln
            .iter()
            .map(|g| BaugrundstueckBerechnung {
                schluessel: g.schluessel.clone(),
                ..vorlage.clone()
            })
            .collect();
        let nutzung = p.pruefe(&planrecht(), 1).unwrap();
        assert_eq!(nutzung.baugrundstuecke.len(), 3);
        assert!(
            nutzung
                .baugrundstuecke
                .iter()
                .all(|n| n.grundstueck.flurstueck_ids.len() == 1)
        );
        for f in &mut p.flurstuecke {
            f.laufende_nummer = 1;
        }
        assert_eq!(p.gruppen(true).unwrap().len(), 3);
        for f in &mut p.flurstuecke {
            f.grundbuchbezirk = "Bezirk".into();
            f.grundbuchblatt = "42".into();
        }
        assert_eq!(p.gruppen(true).unwrap().len(), 1);
        p.flurstuecke[2].laufende_nummer = 2;
        assert_eq!(p.gruppen(true).unwrap().len(), 2);
        for f in &mut p.flurstuecke {
            f.abweichende_zuordnung = Some(AbweichendeZuordnung {
                kennung: "gemeinsam".into(),
                grundlage: "Dokumentierte gemeinsame Einheit".into(),
            });
        }
        assert_eq!(p.gruppen(true).unwrap().len(), 1);
    }
    #[test]
    fn optionale_grundbuchdaten_und_ungueltige_flaechen() {
        let mut p = musterplan();
        p.flurstuecke[0].grundbuchblatt.clear();
        assert_eq!(p.gruppen(true).unwrap().len(), 3);
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
    #[test]
    fn baufensterlage_nur_fuer_zugeordnetes_baugrundstueck() {
        let plan = musterplan();
        let mut recht = planrecht();
        let rechteck = |x: f64, w: f64| {
            vec![
                crate::cad::Punkt(x, 0.),
                crate::cad::Punkt(x + w, 0.),
                crate::cad::Punkt(x + w, 10.),
                crate::cad::Punkt(x, 10.),
            ]
        };
        recht.baugrenzen = Some(crate::cad::baugrenzen::BaugrenzenGeometrie {
            dokument_id: crate::domain::DocumentId::new(),
            dokument_version: 1,
            einheit: crate::cad::Laengeneinheit::Meter,
            baufenster: rechteck(0., 30.),
            vorhaben: rechteck(28., 4.),
            grundstueck_schluessel: None,
        });
        let lage = |r: &BaugrundstuecksPruefung, i: usize| {
            r.baugrundstuecke[i]
                .pruefung
                .nachweise
                .iter()
                .find(|n| n.art == "baufenster_lage")
                .unwrap()
                .eingehalten
        };
        // Zwei Grundstücke ohne Zuordnung: keinem still zugerechnet.
        let r = plan.pruefe(&recht, 1).unwrap();
        assert_eq!((lage(&r, 0), lage(&r, 1)), (None, None));
        assert!(
            r.baugrundstuecke[0]
                .pruefung
                .hinweise
                .iter()
                .any(|h| h.contains("keinem aktuellen Baugrundstück"))
        );
        let ziel = r.baugrundstuecke[1].grundstueck.schluessel.clone();
        recht.baugrenzen.as_mut().unwrap().grundstueck_schluessel = Some(ziel);
        let r = plan.pruefe(&recht, 1).unwrap();
        assert_eq!((lage(&r, 0), lage(&r, 1)), (None, Some(false)));
    }
    #[test]
    fn eindeutige_nachfolger_nach_umgruppierung() {
        let plan = musterplan();
        let alt_c = plan
            .berechnungen
            .iter()
            .find(|b| b.flurstueck_ids == ["c"])
            .unwrap()
            .schluessel
            .clone();
        // Flurstück d kommt zur Grundbuchstelle von c hinzu: eindeutige Nachfolge.
        let mut neu = plan.clone();
        let mut d = neu.flurstuecke[2].clone();
        d.id = "d".into();
        d.nummer = "101/2".into();
        neu.flurstuecke.push(d);
        let g = neu.gruppen(true).unwrap();
        let cd = g
            .iter()
            .find(|g| g.flurstueck_ids.contains(&"d".into()))
            .unwrap();
        assert_eq!(cd.vorgaenger, [alt_c]);
        assert!(
            g.iter()
                .all(|g| g.flurstueck_ids.contains(&"d".into()) || g.vorgaenger.is_empty())
        );
        // Zusammenlegung zweier berechneter Einheiten bleibt eine Entscheidung.
        let mut zusammen = plan.clone();
        zusammen.flurstuecke[2].laufende_nummer = 1;
        let g = zusammen.gruppen(true).unwrap();
        assert_eq!(g.len(), 1);
        assert!(g[0].vorgaenger.is_empty());
    }
}
