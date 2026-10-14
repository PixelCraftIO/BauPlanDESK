//! Fachlicher Exportvertrag für lesbare Projektordner. DMS-Versionen bleiben
//! die Quelle; Dateisystempfade und konkrete Schreibvorgänge gehören zum Adapter.
use crate::{
    CoreError, CoreResult,
    domain::{DocumentVersion, Project},
};
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct ProjektordnerPlan {
    pub projekt_id: String,
    pub projektname: String,
    pub ordner: String,
    pub dateien: Vec<Projektdatei>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Projektdatei {
    pub relativer_pfad: String,
    pub dokument: DocumentVersion,
}
fn dateiname(value: &str) -> String {
    let clean: String = value
        .chars()
        .take(100)
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim_matches([' ', '.']);
    if clean.is_empty() {
        "Projekt".into()
    } else {
        clean.into()
    }
}
/// Wählt den gewünschten Dokumentstand unabhängig vom Ablage-Backend.
pub fn versionen(dokumente: Vec<DocumentVersion>, historie: bool) -> Vec<DocumentVersion> {
    let mut auswahl = std::collections::BTreeMap::new();
    for d in dokumente {
        let key = (d.document_id.0, if historie { d.version } else { 0 });
        if auswahl
            .get(&key)
            .is_none_or(|old: &DocumentVersion| old.version < d.version)
        {
            auswahl.insert(key, d);
        }
    }
    auswahl.into_values().collect()
}
pub fn planen(projekt: &Project, dokumente: &[DocumentVersion]) -> CoreResult<ProjektordnerPlan> {
    let mut dateien = vec![];
    for d in dokumente {
        if d.project_id != projekt.id {
            return Err(CoreError::InvalidInput(
                "Dokument gehört zu einem anderen Projekt".into(),
            ));
        }
        let name = dateiname(&d.file_name);
        let kind = if matches!(
            d.file_name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "dxf" | "dwg"
        ) {
            "Pläne"
        } else {
            "Dokumente"
        };
        // Identität und Version verhindern Namenskollisionen und stilles Überschreiben.
        dateien.push(Projektdatei {
            relativer_pfad: format!("{kind}/{}/v{}-{name}", d.document_id.0, d.version),
            dokument: d.clone(),
        });
    }
    Ok(ProjektordnerPlan {
        projekt_id: projekt.id.to_string(),
        projektname: projekt.name.clone(),
        ordner: format!("{}--{}", dateiname(&projekt.name), projekt.id),
        dateien,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProjectId;
    #[test]
    fn ordner_bleibt_im_ziel_und_identitaet_erhalten() {
        let p = Project {
            id: ProjectId::new(),
            name: "../../CON: Bauprojekt".into(),
            bundesland: crate::domain::Bundesland::By,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let result = planen(&p, &[]).unwrap();
        assert!(!result.ordner.contains('/'));
        assert!(!result.ordner.contains(':'));
        assert!(result.ordner.ends_with(&p.id.to_string()));
    }
}
