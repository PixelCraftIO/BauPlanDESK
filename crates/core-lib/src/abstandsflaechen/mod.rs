//! Projektbezogene, archivfähige Abstandsprüfung. Rechtliche Entscheidungen,
//! Geometrie und Nachweisarchiv besitzen getrennte Verträge.
pub mod bericht;
mod geometrie;
use crate::{CoreError, CoreResult, domain::Bundesland, plattform::objekte::Dokumentbezug};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const RECHENSTAND: &str = "abstand-projekt-1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct APunkt {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Tiefenregel {
    pub gebiete: Vec<String>,
    pub faktor: f64,
    pub mindesttiefe_m: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dachregel {
    pub ueber_grad: f64,
    pub faktor: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Grenzanlagenregel {
    pub typen: Vec<String>,
    pub hoehe_m: f64,
    pub laenge_je_grenze_m: f64,
    pub laenge_gesamt_m: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsRegeln {
    pub id: String,
    pub version: String,
    pub land: Bundesland,
    pub rechtsstand: String,
    pub hinweis: String,
    pub tiefen: Vec<Tiefenregel>,
    pub dach: Vec<Dachregel>,
    pub grenzanlagen: Option<Grenzanlagenregel>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsRecht {
    pub gemeinde: String,
    pub fassung: String,
    pub anwendung: String,
    pub gebiet: String,
    pub satzung: String,
    pub planfestsetzung: String,
    pub geprueft: bool,
    pub pruefvermerk: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsGrundstueck {
    pub id: Uuid,
    pub bezeichnung: String,
    pub flurstueck_ids: Vec<String>,
    pub grenze: Vec<APunkt>,
    #[serde(default)]
    pub innenringe: Vec<Vec<APunkt>>,
    pub grundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Flaechenart {
    Sicherung,
    OeffentlicheFlaeche,
    Baukoerper,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsFlaeche {
    pub id: Uuid,
    pub grundstueck_id: Uuid,
    pub art: Flaechenart,
    pub bezeichnung: String,
    pub polygon: Vec<APunkt>,
    pub grundlage: String,
    pub wirksam: bool,
    pub beleg: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aussenrichtung {
    Links,
    Rechts,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Dachart {
    Flach,
    Dachflaeche,
    Giebel,
    Manuell,
    Ungeklaert,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Wandfall {
    Regel,
    Garage,
    Carport,
    Vorbau,
    Grenzbebauung,
    Bestand,
    Sonderbau,
    Sonstiges,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Sonderansatz {
    pub tiefe_m: f64,
    pub grundlage: String,
    pub wirksam: bool,
    pub beleg: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsWand {
    pub id: Uuid,
    pub gebaeude: String,
    pub bezeichnung: String,
    pub grundstueck_id: Option<Uuid>,
    pub von: APunkt,
    pub bis: APunkt,
    pub aussen: Aussenrichtung,
    pub gelaende_a_m: Option<f64>,
    pub gelaende_b_m: Option<f64>,
    pub abschluss_a_m: Option<f64>,
    pub abschluss_b_m: Option<f64>,
    pub dachart: Dachart,
    pub dachhoehe_m: Option<f64>,
    pub dachneigung_grad: Option<f64>,
    pub dachfaktor_manuell: Option<f64>,
    pub hoehenquelle: String,
    pub dachgrundlage: String,
    pub fall: Wandfall,
    pub grenzkennung: String,
    pub nutzung_geprueft: bool,
    pub sonderansatz: Option<Sonderansatz>,
    pub polygon_manuell: Option<Vec<APunkt>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsPlan {
    pub bezeichnung: String,
    pub regeln: AbstandsRegeln,
    pub recht: AbstandsRecht,
    pub hoehenbezug: String,
    pub koordinatenbezug: String,
    pub grundstuecke: Vec<AbstandsGrundstueck>,
    pub waende: Vec<AbstandsWand>,
    pub flaechen: Vec<AbstandsFlaeche>,
    pub quellen: Vec<Dokumentbezug>,
    pub umfang_geprueft: bool,
    pub umfang_vermerk: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AbstandsPolygon {
    pub aussen: Vec<APunkt>,
    pub innen: Vec<Vec<APunkt>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AbstandsStatus {
    Offen,
    Tiefenvorpruefung,
    Eingehalten,
    Konflikt,
    Grenzfall,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct WandErgebnis {
    pub id: Uuid,
    pub bezeichnung: String,
    pub laenge_m: f64,
    pub wandhoehe_m: Option<f64>,
    pub dachanteil_m: Option<f64>,
    pub h_m: Option<f64>,
    pub faktor: Option<f64>,
    pub mindesttiefe_m: Option<f64>,
    pub tiefe_m: Option<f64>,
    pub herleitung: Vec<String>,
    pub offen: Vec<String>,
    pub status: AbstandsStatus,
    pub flaeche_m2: Option<f64>,
    pub ausserhalb_m2: Option<f64>,
    pub polygon: Vec<APunkt>,
    pub konflikte: Vec<AbstandsPolygon>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Ueberdeckung {
    pub wand_a: Uuid,
    pub wand_b: Uuid,
    pub flaeche_m2: f64,
    pub polygone: Vec<AbstandsPolygon>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AbstandsErgebnis {
    pub rechenstand: String,
    pub waende: Vec<WandErgebnis>,
    pub ueberdeckungen: Vec<Ueberdeckung>,
    pub offen: Vec<String>,
    pub vollstaendig: bool,
    pub konfliktfrei: bool,
}
pub(super) fn err(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn text(s: &str, max: usize) -> CoreResult<()> {
    if s.len() > max {
        Err(err("Abstandsangabe zu lang"))
    } else {
        Ok(())
    }
}
pub fn katalog() -> CoreResult<Vec<AbstandsRegeln>> {
    serde_json::from_str(include_str!(
        "../../../../regelwerke/abstandsflaechen/pilot.json"
    ))
    .map_err(|e| err(e.to_string()))
}
impl AbstandsPlan {
    pub fn dokumente(&self) -> Vec<Dokumentbezug> {
        self.quellen
            .iter()
            .chain(self.flaechen.iter().filter_map(|f| f.beleg.as_ref()))
            .chain(
                self.waende
                    .iter()
                    .filter_map(|w| w.sonderansatz.as_ref().and_then(|s| s.beleg.as_ref())),
            )
            .cloned()
            .collect()
    }
    pub fn validate(&self) -> CoreResult<()> {
        text(&self.bezeichnung, 500)?;
        text(&self.hoehenbezug, 1000)?;
        text(&self.koordinatenbezug, 1000)?;
        if self.waende.len() > 200
            || self.grundstuecke.len() > 100
            || self.flaechen.len() > 200
            || self.quellen.len() > 200
        {
            return Err(err(
                "Höchstens 200 Wände/Flächen/Quellen und 100 Grundstücke",
            ));
        }
        for s in [
            &self.recht.gemeinde,
            &self.recht.fassung,
            &self.recht.anwendung,
            &self.recht.gebiet,
            &self.recht.satzung,
            &self.recht.planfestsetzung,
            &self.recht.pruefvermerk,
            &self.umfang_vermerk,
        ] {
            text(s, 8000)?;
        }
        let r = &self.regeln;
        if r.id.trim().is_empty()
            || r.version.trim().is_empty()
            || r.tiefen.is_empty()
            || r.tiefen.len() > 30
            || r.dach.len() > 10
        {
            return Err(err(
                "Abstandsregelpaket benötigt Kennung, Version und Tiefenregeln",
            ));
        }
        for s in [&r.id, &r.version, &r.rechtsstand, &r.hinweis] {
            text(s, 8000)?;
        }
        for t in &r.tiefen {
            if t.gebiete.is_empty()
                || t.gebiete.len() > 30
                || !t.faktor.is_finite()
                || t.faktor <= 0.
                || t.faktor > 10.
                || !t.mindesttiefe_m.is_finite()
                || t.mindesttiefe_m < 0.
                || t.mindesttiefe_m > 1000.
                || t.quelle.trim().is_empty()
            {
                return Err(err("Ungültige Tiefenregel"));
            }
            text(&t.quelle, 4000)?;
        }
        for d in &r.dach {
            if !d.ueber_grad.is_finite()
                || !(0. ..=90.).contains(&d.ueber_grad)
                || !d.faktor.is_finite()
                || !(0. ..=10.).contains(&d.faktor)
                || d.quelle.trim().is_empty()
            {
                return Err(err("Ungültige Dachregel"));
            }
        }
        if let Some(g) = &r.grenzanlagen {
            for n in [g.hoehe_m, g.laenge_je_grenze_m, g.laenge_gesamt_m] {
                if !n.is_finite() || n <= 0. || n > 1000. {
                    return Err(err("Ungültige Grenzanlagenregel"));
                }
            }
            text(&g.quelle, 4000)?;
        }
        let mut ids = std::collections::HashSet::new();
        for id in self
            .waende
            .iter()
            .map(|w| w.id)
            .chain(self.grundstuecke.iter().map(|g| g.id))
            .chain(self.flaechen.iter().map(|f| f.id))
        {
            if id.is_nil() || !ids.insert(id) {
                return Err(err("Eindeutige Abschnitts-/Flächenkennungen erforderlich"));
            }
        }
        for g in &self.grundstuecke {
            text(&g.bezeichnung, 500)?;
            text(&g.grundlage, 4000)?;
            if g.flurstueck_ids.len() > 1000 {
                return Err(err("Zu viele Flurstücksbezüge"));
            }
            if !g.grenze.is_empty() {
                geometrie::grundstueck(g)?;
            }
        }
        for f in &self.flaechen {
            text(&f.bezeichnung, 500)?;
            text(&f.grundlage, 4000)?;
            if !self.grundstuecke.iter().any(|g| g.id == f.grundstueck_id) {
                return Err(err("Fläche benötigt eine vorhandene Grundstückseinheit"));
            }
            if !f.polygon.is_empty() {
                geometrie::polygon(&f.polygon)?;
            }
        }
        for w in &self.waende {
            for s in [
                &w.bezeichnung,
                &w.gebaeude,
                &w.hoehenquelle,
                &w.dachgrundlage,
                &w.grenzkennung,
            ] {
                text(s, 4000)?;
            }
            for p in [&w.von, &w.bis] {
                geometrie::punkt(p)?;
            }
            for n in [
                w.gelaende_a_m,
                w.gelaende_b_m,
                w.abschluss_a_m,
                w.abschluss_b_m,
                w.dachhoehe_m,
                w.dachneigung_grad,
                w.dachfaktor_manuell,
            ]
            .into_iter()
            .flatten()
            {
                if !n.is_finite() || n.abs() > 10000. {
                    return Err(err("Ungültige Höhen-/Dachangabe"));
                }
            }
            if w.dachhoehe_m.is_some_and(|v| v < 0.)
                || w.dachneigung_grad
                    .is_some_and(|v| !(0. ..=90.).contains(&v))
                || w.dachfaktor_manuell
                    .is_some_and(|v| !(0. ..=10.).contains(&v))
            {
                return Err(err(
                    "Dachhöhe/Faktor positiv, Neigung zwischen 0 und 90 Grad erforderlich",
                ));
            }
            if w.grundstueck_id
                .is_some_and(|id| !self.grundstuecke.iter().any(|g| g.id == id))
            {
                return Err(err("Wand verweist auf unbekanntes Grundstück"));
            }
            if let Some(s) = &w.sonderansatz {
                text(&s.grundlage, 8000)?;
                if !s.tiefe_m.is_finite() || s.tiefe_m < 0. || s.tiefe_m > 1000. {
                    return Err(err("Ungültige besondere Tiefe"));
                }
            }
            if let Some(p) = &w.polygon_manuell {
                geometrie::polygon(p)?;
            }
        }
        Ok(())
    }
}
pub fn berechnen(plan: &AbstandsPlan) -> CoreResult<AbstandsErgebnis> {
    plan.validate()?;
    let mut offen = Vec::new();
    if plan.waende.is_empty() {
        offen.push("Noch keine Außenwände erfasst".into());
    }
    if plan.recht.satzung.trim().is_empty() || plan.recht.planfestsetzung.trim().is_empty() {
        offen.push("Ergebnis der Satzungs- und Planfestsetzungsprüfung dokumentieren (auch wenn keine Abweichung vorliegt)".into());
    }
    if !plan.recht.geprueft
        || plan.recht.pruefvermerk.trim().is_empty()
        || plan.recht.fassung.trim().is_empty()
        || plan.recht.anwendung.trim().is_empty()
        || plan.recht.gemeinde.trim().is_empty()
    {
        offen.push("Anwendbare Rechtsfassung, Gemeinde, Satzungs-/Planprüfung und Prüfvermerk fachlich bestätigen".into());
    }
    if !plan.umfang_geprueft || plan.umfang_vermerk.trim().is_empty() {
        offen.push("Vollständigkeit aller Wände, Ecken, Geländeabschnitte und Sonderfälle prüfen und begründen".into());
    }
    if plan.hoehenbezug.trim().is_empty() {
        offen.push("Maßgebliche Geländeoberfläche und Höhenbezug festlegen".into());
    }
    if plan.koordinatenbezug.trim().is_empty() {
        offen.push("Koordinatenbezug (Meter) bestätigen".into());
    }
    for f in &plan.flaechen {
        if f.polygon.len() < 3 {
            offen.push(format!("Fläche {} ohne gültigen Umriss", f.bezeichnung));
        }
    }
    let mut waende = Vec::new();
    for w in &plan.waende {
        let laenge = (w.bis.x - w.von.x).hypot(w.bis.y - w.von.y);
        let mut e = WandErgebnis {
            id: w.id,
            bezeichnung: w.bezeichnung.clone(),
            laenge_m: laenge,
            wandhoehe_m: None,
            dachanteil_m: None,
            h_m: None,
            faktor: None,
            mindesttiefe_m: None,
            tiefe_m: None,
            herleitung: vec![],
            offen: vec![],
            status: AbstandsStatus::Offen,
            flaeche_m2: None,
            ausserhalb_m2: None,
            polygon: vec![],
            konflikte: vec![],
        };
        if w.bezeichnung.trim().is_empty() || w.gebaeude.trim().is_empty() {
            e.offen.push("Gebäude und Wandbezeichnung angeben".into());
        }
        if laenge < 0.001 {
            e.offen
                .push("Wandlänge mindestens 1 mm erforderlich".into());
        }
        if w.hoehenquelle.trim().is_empty() {
            e.offen
                .push("Gelände-/Höhenherleitung und Planquelle dokumentieren".into());
        }
        if let (Some(ga), Some(gb), Some(oa), Some(ob)) = (
            w.gelaende_a_m,
            w.gelaende_b_m,
            w.abschluss_a_m,
            w.abschluss_b_m,
        ) {
            let h = (oa - ga).max(ob - gb);
            if h < 0. || oa < ga || ob < gb {
                e.offen
                    .push("Wandabschluss darf nicht unter maßgeblichem Gelände liegen".into());
            } else {
                e.wandhoehe_m = Some(h);
                e.herleitung.push(format!("Konservativer Maximalansatz: max({oa} − {ga}, {ob} − {gb}) = {h} m. Bei nichtlinearem Verlauf Wand in Abschnitte aufteilen."));
            }
        } else {
            e.offen
                .push("Gelände und Wandabschluss an beiden Abschnittsenden eingeben".into());
        }
        let dach = match w.dachart {
            Dachart::Flach => Some(0.),
            Dachart::Dachflaeche => {
                if plan.regeln.dach.is_empty() {
                    None
                } else if let (Some(h), Some(n)) = (w.dachhoehe_m, w.dachneigung_grad) {
                    let r = plan
                        .regeln
                        .dach
                        .iter()
                        .filter(|r| n > r.ueber_grad)
                        .max_by(|a, b| a.ueber_grad.total_cmp(&b.ueber_grad));
                    let f = r.map(|r| r.faktor).unwrap_or(0.);
                    e.herleitung.push(format!(
                        "Dachfläche: {h} m × {f} bei {n}°; {}",
                        r.map(|r| r.quelle.as_str())
                            .unwrap_or("unterhalb erster konfigurierter Schwelle")
                    ));
                    Some(h * f)
                } else {
                    None
                }
            }
            Dachart::Giebel | Dachart::Manuell => {
                if !w.dachgrundlage.trim().is_empty() {
                    w.dachhoehe_m.zip(w.dachfaktor_manuell).map(|(h, f)| {
                        e.herleitung.push(format!(
                            "Fachlich angesetzter Dach-/Giebelanteil: {h} m × {f}; {}",
                            w.dachgrundlage
                        ));
                        h * f
                    })
                } else {
                    None
                }
            }
            Dachart::Ungeklaert => None,
        };
        e.dachanteil_m = dach;
        if dach.is_none() {
            e.offen
                .push("Dach-/Giebelansatz einschließlich Rechtsgrundlage klären".into());
        }
        e.h_m = e.wandhoehe_m.zip(dach).map(|(a, b)| a + b);
        let regeln = plan
            .regeln
            .tiefen
            .iter()
            .filter(|r| r.gebiete.contains(&plan.recht.gebiet))
            .collect::<Vec<_>>();
        if regeln.len() == 1 {
            let r = regeln[0];
            e.faktor = Some(r.faktor);
            e.mindesttiefe_m = Some(r.mindesttiefe_m);
            e.tiefe_m = e.h_m.map(|h| (h * r.faktor).max(r.mindesttiefe_m));
            e.herleitung.push(format!(
                "T = max({} × H, {} m); {}",
                r.faktor, r.mindesttiefe_m, r.quelle
            ));
        } else {
            e.offen
                .push("Baugebiet hat keine eindeutige konfigurierte Tiefenregel".into());
        }
        if matches!(w.fall, Wandfall::Garage | Wandfall::Carport) {
            let typ = if w.fall == Wandfall::Garage {
                "garage"
            } else {
                "carport"
            };
            if let Some(r) = &plan.regeln.grenzanlagen {
                let anlagen = plan
                    .waende
                    .iter()
                    .filter(|a| {
                        a.grundstueck_id == w.grundstueck_id
                            && matches!(a.fall, Wandfall::Garage | Wandfall::Carport)
                    })
                    .collect::<Vec<_>>();
                let sum = |grenze: bool| {
                    anlagen
                        .iter()
                        .filter(|a| !grenze || a.grenzkennung == w.grenzkennung)
                        .map(|a| (a.bis.x - a.von.x).hypot(a.bis.y - a.von.y))
                        .sum::<f64>()
                };
                if r.typen.iter().any(|s| s == typ)
                    && w.nutzung_geprueft
                    && !w.grenzkennung.trim().is_empty()
                    && e.h_m.is_some_and(|h| h <= r.hoehe_m)
                    && sum(true) <= r.laenge_je_grenze_m
                    && sum(false) <= r.laenge_gesamt_m
                {
                    e.tiefe_m = Some(0.);
                    e.herleitung.push(format!("Grenzanlage: Höhe ≤ {}, Grenzlänge {} ≤ {}, Gesamtlänge {} ≤ {}; Nutzung/Bedingungen bestätigt; {}",r.hoehe_m,sum(true),r.laenge_je_grenze_m,sum(false),r.laenge_gesamt_m,r.quelle));
                } else {
                    e.offen.push("Grenzanlagenprivileg nicht vollständig belegt oder Höchstmaße/Summen überschritten; Regelabstand bleibt angesetzt".into());
                }
            } else {
                e.offen
                    .push("Keine konfigurierte Privilegierung für diese Grenzanlage".into());
            }
        } else if w.fall != Wandfall::Regel {
            e.offen.push(
                "Sonderfall benötigt dokumentierten besonderen Ansatz oder Einzelprüfung".into(),
            );
        }
        if let Some(s) = &w.sonderansatz {
            if s.wirksam && s.beleg.is_some() && !s.grundlage.trim().is_empty() {
                e.tiefe_m = Some(s.tiefe_m);
                e.offen.retain(|v| {
                    !v.starts_with("Sonderfall")
                        && !v.starts_with("Grenzanlagenprivileg")
                        && !v.starts_with("Keine konfigurierte")
                });
                e.herleitung.push(format!(
                    "Besonderer belegter Ansatz: {} m; {}",
                    s.tiefe_m, s.grundlage
                ));
            } else {
                e.offen
                    .push("Besonderer Ansatz ist noch nicht wirksam oder nicht belegt".into());
            }
        }
        if let Some(t) = e.tiefe_m
            && laenge >= 0.001
        {
            geometrie::wand(plan, w, &mut e, t)?;
        }
        if !e.offen.is_empty() {
            e.status = AbstandsStatus::Offen;
        }
        waende.push(e);
    }
    let ueberdeckungen = geometrie::ueberdeckungen(&mut waende)?;
    if !ueberdeckungen.is_empty() {
        offen.push("Überdeckungen zwischen Abstandsflächen: Zulässigkeit gesondert prüfen; keine automatische Ausnahme".into());
    }
    let vollstaendig = offen.is_empty()
        && !waende.is_empty()
        && waende.iter().all(|w| {
            w.offen.is_empty()
                && w.status != AbstandsStatus::Tiefenvorpruefung
                && w.status != AbstandsStatus::Grenzfall
        });
    let konfliktfrei = vollstaendig
        && waende
            .iter()
            .all(|w| w.status == AbstandsStatus::Eingehalten)
        && ueberdeckungen.is_empty();
    Ok(AbstandsErgebnis {
        rechenstand: RECHENSTAND.into(),
        waende,
        ueberdeckungen,
        offen,
        vollstaendig,
        konfliktfrei,
    })
}
mod service;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;
#[cfg(test)]
mod tests;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct CadKontur {
    pub linie: usize,
    pub ebene: String,
    pub punkte: Vec<APunkt>,
    pub quelle: Dokumentbezug,
}
