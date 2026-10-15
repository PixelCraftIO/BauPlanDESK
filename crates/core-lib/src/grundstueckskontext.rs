//! Gemeinsame Lesesicht für Anzeigen, Formulare und weitere Fachverbraucher.
use crate::{
    CoreResult,
    domain::{BebauungsplanArt, Project, ProjectId},
    flaechen::GrundstueckAufstellung,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GrundstueckKontext {
    pub id: uuid::Uuid,
    pub projekt_id: ProjectId,
    pub projekt_version: i64,
    pub schluessel: String,
    pub bezeichnung: String,
    pub flurstueck_ids: Vec<String>,
    pub eigentumsflaeche_m2: f64,
    pub abzug_m2: f64,
    pub zuschlag_m2: f64,
    pub bezugsflaeche_m2: Option<f64>,
    pub quelle: String,
}

pub fn ermitteln(p: &Project) -> CoreResult<Vec<GrundstueckKontext>> {
    let Some(plan) = &p.planungsrecht.baugrundstuecke else {
        // Dokumentierter Adapter für noch nicht aufgeschlüsselte Bestandsprojekte.
        return Ok(p
            .grundstuecksflaeche_m2
            .map(|f| GrundstueckKontext {
                id: uuid::Uuid::new_v5(&p.id.0, b"projektflaeche"),
                projekt_id: p.id,
                projekt_version: p.version,
                schluessel: "projektflaeche".into(),
                bezeichnung: "Noch nicht aufgeschlüsselte Projektfläche".into(),
                flurstueck_ids: vec![],
                eigentumsflaeche_m2: f,
                abzug_m2: 0.,
                zuschlag_m2: 0.,
                bezugsflaeche_m2: Some(f),
                quelle: "Projektfläche – noch keine Flurstücksermittlung".into(),
            })
            .into_iter()
            .collect());
    };
    let mit_plan = matches!(
        p.planungsrecht.bebauungsplan_art,
        BebauungsplanArt::Qualifiziert
            | BebauungsplanArt::Einfach
            | BebauungsplanArt::Vorhabenbezogen
    );
    plan.gruppen(mit_plan)?
        .into_iter()
        .map(|g| {
            let aufstellung = plan
                .berechnungen
                .iter()
                .find(|b| b.schluessel == g.schluessel)
                .and_then(|b| b.eingabe.aufstellung.as_ref());
            let anteile = aufstellung
                .and_then(|a| a.grundstueck.as_ref())
                .map(|a| a.gemeinschaftsanteile.clone())
                .unwrap_or_default();
            let mut grund = GrundstueckAufstellung {
                basisflaeche_m2: g.eigentumsflaeche_m2,
                nicht_massgebend_m2: g.abzuege_m2,
                abzug_begruendung: "Flurstücksabzüge".into(),
                gemeinschaftsanteile: anteile,
            };
            // Ohne explizite BauNVO keine Sonderzurechnung als bestätigt ausgeben.
            let basis = if grund.gemeinschaftsanteile.is_empty() {
                Some(g.bauland_m2)
            } else if mit_plan {
                if let Some(fassung) = p.planungsrecht.bau_nvo.or(aufstellung.map(|a| a.bau_nvo)) {
                    Some(
                        grund
                            .berechne(crate::baunvo::regelwerk(fassung))?
                            .massgebend_m2,
                    )
                } else {
                    None
                }
            } else {
                grund.gemeinschaftsanteile.clear();
                None
            };
            Ok(GrundstueckKontext {
                id: uuid::Uuid::new_v5(&p.id.0, g.schluessel.as_bytes()),
                projekt_id: p.id,
                projekt_version: p.version,
                schluessel: g.schluessel,
                bezeichnung: g.bezeichnung,
                flurstueck_ids: g.flurstueck_ids,
                eigentumsflaeche_m2: g.eigentumsflaeche_m2,
                abzug_m2: g.abzuege_m2,
                zuschlag_m2: basis.map(|f| f - g.bauland_m2).unwrap_or(0.),
                bezugsflaeche_m2: basis,
                quelle: "Flurstücke, dokumentierte Abzüge und zugeordnete Gemeinschaftsanteile"
                    .into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{Bundesland, Planungsrecht},
        flaechen::BauNvo,
        formular::vorlage::Projektkontext,
    };
    #[test]
    fn pdf_uebernimmt_keine_alte_summe_bei_mehreren_grundstuecken() {
        let mut p = Project {
            id: ProjectId::new(),
            name: "Test".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: Some(640.),
            version: 1,
            created_at: Default::default(),
            updated_at: Default::default(),
            planungsrecht: Planungsrecht {
                bau_nvo: Some(BauNvo::V2017),
                bebauungsplan_art: BebauungsplanArt::Qualifiziert,
                baugrundstuecke: Some(crate::baugrundstuecke::musterplan()),
                ..Default::default()
            },
        };
        assert_eq!(ermitteln(&p).unwrap().len(), 2);
        assert!(
            Projektkontext::aus_projekt(&p)
                .wert("grundstueck.flaeche_m2")
                .is_none()
        );
        let plan = p.planungsrecht.baugrundstuecke.as_mut().unwrap();
        plan.flurstuecke.retain(|f| f.id != "c");
        plan.berechnungen
            .retain(|b| b.schluessel == plan.flurstuecke[0].schluessel());
        let k = ermitteln(&p).unwrap();
        assert_eq!(k[0].bezugsflaeche_m2, Some(430.));
        assert_eq!(
            Projektkontext::aus_projekt(&p).wert("grundstueck.flaeche_m2"),
            Some("430,00")
        );
    }
}
