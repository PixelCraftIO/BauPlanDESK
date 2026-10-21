//! Vorschlag der XBau-Anlagenart ([`super::codeliste::ANLAGEN`]) aus der
//! Dokumentkategorie. Die Kategorie wird einmal im DMS gepflegt; Bauantrag und
//! Abgabe übernehmen daraus die Anlagenart, solange keine eigene Wahl besteht.
//!
//! Vertrag: Ein Vorschlag nennt nur Codes, die in der geladenen Anlagenliste
//! stehen. Kategorien ohne eindeutige Anlagenart (z. B. „Bescheinigungen und
//! Nachweise“, „Bescheid“) liefern keinen Vorschlag; Unterkategorien ohne eigene
//! Zuordnung übernehmen die ihrer Hauptkategorie.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::codeliste::Codeliste;
use crate::dokumentkategorien::KategorienKatalog;
use crate::domain::{DocumentId, ProjectId};
use crate::error::CoreResult;

/// Standardkategorie → Anlagenart (Codeliste Anlagen, Version 4.0).
const ZUORDNUNG: &[(&str, &str)] = &[
    ("bauantrag", "1101"),
    ("baubeschreibung", "1501"),
    ("lageplan", "1803"),
    ("grundriss", "2001"),
    ("schnitt", "2002"),
    ("ansicht", "2003"),
    ("berechnung", "2100"),
    ("grz", "1904"),
    ("gfz", "1903"),
    ("vollgeschosse", "2103"),
    ("abstandsflaechen", "2102"),
    ("bauvolumen", "2201"),
    ("statik", "2801"),
    ("gutachten", "2900"),
    ("anschreiben", "1001"),
    ("email", "1003"),
    ("schriftverkehr", "1003"),
    ("foto", "3700"),
    ("grundbuch", "2008"),
    ("entwaesserung", "1806"),
];

/// Vorgeschlagene Anlagenart eines Dokuments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anlagenartvorschlag {
    /// Code aus der Anlagenliste.
    pub anlageart: String,
    /// Name der Dokumentkategorie, aus der der Vorschlag stammt.
    pub kategorie: String,
}

/// Anlagenart für eine Kategorie, sofern zugeordnet und in `anlagen` enthalten.
pub fn vorschlag(
    katalog: &KategorienKatalog,
    kategorie: &str,
    anlagen: &Codeliste,
) -> Option<Anlagenartvorschlag> {
    let k = katalog.kategorie(kategorie).ok()?;
    let code = |id: &str| ZUORDNUNG.iter().find(|(k, _)| *k == id).map(|(_, c)| *c);
    let anlageart = code(&k.id).or_else(|| k.eltern_id.as_deref().and_then(code))?;
    anlagen.enthaelt(anlageart).then(|| Anlagenartvorschlag {
        anlageart: anlageart.into(),
        kategorie: k.name.clone(),
    })
}

impl crate::CoreServices {
    /// Vorschläge je Dokument des Projekts (neueste Kategorie-Zuordnung).
    /// Ohne geladene Anlagenliste gibt es keine Vorschläge.
    pub async fn anlagenart_vorschlaege(
        &self,
        p: ProjectId,
    ) -> CoreResult<HashMap<DocumentId, Anlagenartvorschlag>> {
        let codelisten = self.codelisten.aktuell();
        let Some(anlagen) = codelisten.liste(super::codeliste::ANLAGEN) else {
            return Ok(HashMap::new());
        };
        let katalog = self.kategorienkatalog().await?;
        Ok(self
            .dokumentkategorien(p)
            .await?
            .into_iter()
            .filter_map(|d| {
                let k = d.zuordnung.kategorie?;
                Some((d.dokument_id, vorschlag(&katalog, &k, anlagen)?))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dokumentkategorien::Dokumentkategorie;
    use crate::xbau::codeliste::Codeeintrag;

    fn anlagen(codes: &[&str]) -> Codeliste {
        Codeliste {
            uri: super::super::codeliste::ANLAGEN.into(),
            version: "4.0".into(),
            name: "Anlagen".into(),
            eintraege: codes
                .iter()
                .map(|c| Codeeintrag {
                    code: (*c).into(),
                    beschreibung: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn vorschlag_aus_kategorie_unterkategorie_und_nur_mit_geladenem_code() {
        let mut katalog = KategorienKatalog::default();
        katalog.kategorien.push(Dokumentkategorie {
            id: "lageplan-amtlich".into(),
            name: "Amtlicher Lageplan".into(),
            eltern_id: Some("lageplan".into()),
            aktiv: true,
        });
        let liste = anlagen(&["1803", "2008", "1904"]);
        let v = vorschlag(&katalog, "grundbuch", &liste).unwrap();
        assert_eq!(v.anlageart, "2008");
        assert_eq!(v.kategorie, "Grundbuchauszug");
        assert_eq!(
            vorschlag(&katalog, "lageplan-amtlich", &liste)
                .unwrap()
                .anlageart,
            "1803"
        );
        assert_eq!(
            vorschlag(&katalog, "grz", &liste).unwrap().anlageart,
            "1904"
        );
        // Nicht eindeutig zuordenbar bzw. Code nicht in der Liste.
        assert!(vorschlag(&katalog, "bescheinigung", &liste).is_none());
        assert!(vorschlag(&katalog, "bescheid", &liste).is_none());
        assert!(vorschlag(&katalog, "schnitt", &liste).is_none());
        assert!(vorschlag(&katalog, "unbekannt", &liste).is_none());
    }

    #[test]
    fn alle_zuordnungen_stehen_in_der_mitgelieferten_anlagenliste() {
        let xml = include_str!("../../../../codelisten/xbau-anlagen_4.0.xml");
        let liste = Codeliste::genericode(xml).unwrap();
        let katalog = KategorienKatalog::default();
        for (kategorie, code) in ZUORDNUNG {
            assert!(liste.enthaelt(code), "{kategorie}: {code}");
            assert!(katalog.kategorie(kategorie).is_ok(), "{kategorie}");
        }
    }
}
