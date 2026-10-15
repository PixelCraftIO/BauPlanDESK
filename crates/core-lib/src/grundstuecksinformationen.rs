//! Kataster-/Grundbuchfakten und davon getrennte Ansätze der Baugrundstücksermittlung.
//! `BaugrundstuecksPlan` bleibt der vollständige Eingabe-/Archivvertrag für Verbraucher.
use crate::{
    CoreError, CoreResult, baugrundstuecke::*, domain::Planungsrecht,
    plattform::objekte::Dokumentbezug, storage::ContentHash,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Grundstuecksinformationen {
    pub flurstuecke: Vec<FlurstuecksInfo>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FlurstuecksInfo {
    pub id: String,
    pub gemarkung: String,
    pub flur: String,
    pub nummer: String,
    pub flaeche_m2: f64,
    #[serde(default)]
    pub grundbuchbezirk: String,
    #[serde(default)]
    pub grundbuchblatt: String,
    #[serde(default)]
    pub laufende_nummer: Option<u32>,
    #[serde(default)]
    pub grundbuchauszug: Option<Dokumentbezug>,
    #[serde(default)]
    pub eigentuemer: String,
    #[serde(default)]
    pub bemerkung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FlurstuecksAnsatz {
    pub flurstueck_id: String,
    #[ts(type = "Array<FlaechenAbzug>")]
    pub abzuege: Vec<FlaechenAbzug>,
    #[ts(type = "{ kennung: string; grundlage: string } | null")]
    pub abweichende_zuordnung: Option<AbweichendeZuordnung>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BaugrundstuecksErmittlung {
    pub ansaetze: Vec<FlurstuecksAnsatz>,
    #[ts(type = "Array<BaugrundstueckBerechnung>")]
    pub berechnungen: Vec<BaugrundstueckBerechnung>,
    /// Bei Speicherung durch den Core gesetzt; keine zweite Kopie der Katasterflächen.
    #[ts(type = "string | null")]
    pub quellen_hash: Option<ContentHash>,
}
impl Grundstuecksinformationen {
    pub fn validate(&self) -> CoreResult<()> {
        if self.flurstuecke.len() > 1000 {
            return Err(CoreError::InvalidInput(
                "Höchstens 1000 Flurstücke erfassen".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        let mut kataster = std::collections::HashSet::new();
        for f in &self.flurstuecke {
            if [&f.id, &f.gemarkung, &f.nummer]
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 500)
                || [
                    &f.flur,
                    &f.grundbuchbezirk,
                    &f.grundbuchblatt,
                    &f.eigentuemer,
                ]
                .iter()
                .any(|s| s.len() > 500)
                || f.bemerkung.len() > 4000
                || f.laufende_nummer == Some(0)
                || !f.flaeche_m2.is_finite()
                || f.flaeche_m2 <= 0.
            {
                return Err(CoreError::InvalidInput("Flurstück benötigt Gemarkung, Nummer und positive Fläche. Grundbuchangaben sind optional; eine angegebene laufende Nummer muss positiv sein.".into()));
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
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> CoreResult<ContentHash> {
        Ok(ContentHash::of(
            &serde_json::to_vec(self).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        ))
    }
}
/// Lesender Adapter für Bestandsprojekte; die erste getrennte Speicherung übernimmt beide Teile.
pub fn informationen(p: &Planungsrecht) -> Grundstuecksinformationen {
    p.grundstuecksinformationen
        .clone()
        .unwrap_or_else(|| Grundstuecksinformationen {
            flurstuecke: p
                .baugrundstuecke
                .as_ref()
                .map(|b| {
                    b.flurstuecke
                        .iter()
                        .map(|f| FlurstuecksInfo {
                            id: f.id.clone(),
                            gemarkung: f.gemarkung.clone(),
                            flur: f.flur.clone(),
                            nummer: f.nummer.clone(),
                            flaeche_m2: f.flaeche_m2,
                            grundbuchbezirk: f.grundbuchbezirk.clone(),
                            grundbuchblatt: f.grundbuchblatt.clone(),
                            laufende_nummer: Some(f.laufende_nummer).filter(|n| *n > 0),
                            grundbuchauszug: None,
                            eigentuemer: String::new(),
                            bemerkung: String::new(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
}
pub fn ermittlung(p: &Planungsrecht) -> CoreResult<Option<BaugrundstuecksErmittlung>> {
    if let Some(e) = &p.baugrundstuecksermittlung {
        return Ok(Some(e.clone()));
    }
    p.baugrundstuecke
        .as_ref()
        .map(|b| {
            Ok(BaugrundstuecksErmittlung {
                ansaetze: b
                    .flurstuecke
                    .iter()
                    .map(|f| FlurstuecksAnsatz {
                        flurstueck_id: f.id.clone(),
                        abzuege: f.abzuege.clone(),
                        abweichende_zuordnung: f.abweichende_zuordnung.clone(),
                    })
                    .collect(),
                berechnungen: b.berechnungen.clone(),
                quellen_hash: Some(
                    informationen(&Planungsrecht {
                        baugrundstuecke: p.baugrundstuecke.clone(),
                        ..Default::default()
                    })
                    .fingerprint()?,
                ),
            })
        })
        .transpose()
}
pub fn normalisieren(p: &mut Planungsrecht) -> CoreResult<()> {
    if p.baugrundstuecke.is_some() {
        let infos = informationen(p);
        let e = ermittlung(p)?;
        p.grundstuecksinformationen = Some(infos);
        p.baugrundstuecksermittlung = e;
        p.baugrundstuecke = None;
    }
    Ok(())
}
pub fn plan(p: &Planungsrecht) -> CoreResult<Option<BaugrundstuecksPlan>> {
    if p.grundstuecksinformationen.is_none() && p.baugrundstuecksermittlung.is_none() {
        return Ok(p.baugrundstuecke.clone());
    }
    let infos = informationen(p);
    infos.validate()?;
    let e = ermittlung(p)?.unwrap_or_default();
    Ok(Some(BaugrundstuecksPlan {
        flurstuecke: infos
            .flurstuecke
            .iter()
            .map(|f| {
                let a = e.ansaetze.iter().find(|a| a.flurstueck_id == f.id);
                Flurstueck {
                    id: f.id.clone(),
                    gemarkung: f.gemarkung.clone(),
                    flur: f.flur.clone(),
                    nummer: f.nummer.clone(),
                    grundbuchbezirk: f.grundbuchbezirk.clone(),
                    grundbuchblatt: f.grundbuchblatt.clone(),
                    laufende_nummer: f.laufende_nummer.unwrap_or(0),
                    flaeche_m2: f.flaeche_m2,
                    abzuege: a.map(|a| a.abzuege.clone()).unwrap_or_default(),
                    abweichende_zuordnung: a.and_then(|a| a.abweichende_zuordnung.clone()),
                }
            })
            .collect(),
        berechnungen: e.berechnungen,
    }))
}
pub fn quellen_aktuell(p: &Planungsrecht) -> CoreResult<bool> {
    if p.grundstuecksinformationen.is_none() {
        return Ok(true);
    }
    Ok(ermittlung(p)?.and_then(|e| e.quellen_hash).as_ref()
        == Some(&informationen(p).fingerprint()?))
}
/// Neue Berechnungseingaben dürfen keine eigenen Katasterfakten unterschieben.
pub fn pruefe_quellen(p: &Planungsrecht, b: &BaugrundstuecksPlan) -> CoreResult<()> {
    if p.grundstuecksinformationen.is_none() {
        return Ok(());
    }
    let mut erwartung = plan(p)?.unwrap_or_default().flurstuecke;
    let mut eingabe = b.flurstuecke.clone();
    for f in erwartung.iter_mut().chain(eingabe.iter_mut()) {
        f.abzuege.clear();
        f.abweichende_zuordnung = None;
    }
    erwartung.sort_by(|a, b| a.id.cmp(&b.id));
    eingabe.sort_by(|a, b| a.id.cmp(&b.id));
    if erwartung != eingabe {
        return Err(CoreError::InvalidInput("Flurstücksdaten wurden geändert. Grundstücksinformationen zuerst speichern und die Baugrundstücksberechnung neu laden.".into()));
    }
    Ok(())
}
pub fn validiere_ansaetze(p: &Planungsrecht, e: &BaugrundstuecksErmittlung) -> CoreResult<()> {
    let infos = informationen(p);
    let mut ids = std::collections::HashSet::new();
    if e.ansaetze.len() > 1000
        || e.ansaetze.iter().any(|a| {
            !ids.insert(&a.flurstueck_id)
                || !infos.flurstuecke.iter().any(|f| f.id == a.flurstueck_id)
        })
    {
        return Err(CoreError::InvalidInput(
            "Berechnungsansätze müssen eindeutig auf vorhandene Flurstücke verweisen".into(),
        ));
    }
    Ok(())
}

/// Nachweise verbrauchen die bestätigten Grundlagen des Grundstücksmoduls.
/// Eigene GRZ-/GFZ-Eingaben bleiben zulässig; geänderte Zuordnungen/Abzüge/Zuschläge
/// müssen dagegen zuerst im Liefermodul gespeichert werden.
pub fn pruefe_bestaetigte_ermittlung(p: &Planungsrecht, b: &BaugrundstuecksPlan) -> CoreResult<()> {
    pruefe_quellen(p, b)?;
    if p.grundstuecksinformationen.is_none() {
        return Ok(());
    }
    let fehler = || {
        CoreError::InvalidInput("Baugrundstücksermittlung zuerst prüfen und speichern. Zuordnungen, Abzüge und Gemeinschaftsanteile müssen dem bestätigten Stand entsprechen.".into())
    };
    if !quellen_aktuell(p)? {
        return Err(fehler());
    }
    let gespeichert = plan(p)?.ok_or_else(fehler)?;
    for f in &b.flurstuecke {
        let alt = gespeichert
            .flurstuecke
            .iter()
            .find(|a| a.id == f.id)
            .ok_or_else(fehler)?;
        if f.abzuege != alt.abzuege || f.abweichende_zuordnung != alt.abweichende_zuordnung {
            return Err(fehler());
        }
    }
    let anteile = |b: &BaugrundstueckBerechnung| {
        b.eingabe
            .aufstellung
            .as_ref()
            .and_then(|a| a.grundstueck.as_ref())
            .map(|g| g.gemeinschaftsanteile.clone())
            .unwrap_or_default()
    };
    for berechnung in &b.berechnungen {
        let alt = gespeichert
            .berechnungen
            .iter()
            .find(|a| a.schluessel == berechnung.schluessel)
            .ok_or_else(fehler)?;
        if anteile(berechnung) != anteile(alt) {
            return Err(fehler());
        }
    }
    Ok(())
}

pub(crate) async fn dokumente_pruefen(
    core: &crate::CoreServices,
    projekt: crate::domain::ProjectId,
    p: &Planungsrecht,
) -> CoreResult<()> {
    let mut geprueft = std::collections::HashSet::new();
    for f in informationen(p).flurstuecke {
        if let Some(d) = f.grundbuchauszug
            && geprueft.insert((d.dokument_id, d.version))
            && !core
                .repos
                .documents
                .history(d.dokument_id)
                .await?
                .iter()
                .any(|v| v.version == d.version && v.project_id == projekt)
        {
            return Err(CoreError::InvalidInput(
                "Grundbuchauszug gehört nicht zu diesem Projekt".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optionale_grundbuchangaben_fuehren_nicht_zu_heimlicher_gruppierung() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        for f in &mut infos.flurstuecke {
            f.grundbuchbezirk.clear();
            f.grundbuchblatt.clear();
            f.laufende_nummer = None;
        }
        infos.validate().unwrap();
        let gruppen = plan(&p).unwrap().unwrap().gruppen(true).unwrap();
        assert_eq!(gruppen.len(), 3);
        assert!(gruppen.iter().all(|g| g.flurstueck_ids.len() == 1));
        for a in &mut p.baugrundstuecksermittlung.as_mut().unwrap().ansaetze {
            a.abweichende_zuordnung = Some(AbweichendeZuordnung {
                kennung: a.flurstueck_id.clone(),
                grundlage: "Dokumentierte Einzelfallzuordnung".into(),
            });
        }
        assert_eq!(plan(&p).unwrap().unwrap().gruppen(true).unwrap().len(), 3);
    }
    #[test]
    fn gleiche_nummer_in_unterschiedlichen_grundbuchblaettern_bleibt_getrennt() {
        let mut b = musterplan();
        b.flurstuecke[1].grundbuchblatt = "Anderes Blatt".into();
        assert_eq!(b.gruppen(true).unwrap().len(), 3);
    }
    #[test]
    fn rein_formatbedingte_trennung_erhaelt_plan_und_quellenstand() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        let vorher = plan(&p).unwrap();
        normalisieren(&mut p).unwrap();
        assert_eq!(plan(&p).unwrap(), vorher);
        assert!(quellen_aktuell(&p).unwrap());
        normalisieren(&mut p).unwrap();
        assert_eq!(plan(&p).unwrap(), vorher);
    }
}
