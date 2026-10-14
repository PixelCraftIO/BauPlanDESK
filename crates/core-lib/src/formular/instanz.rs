//! Quellenwahl und unveränderlicher Datenstand einer ausgefüllten PDF-Version.
use super::{Feldwert, Projektkontext};
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::ProjectId,
    kunden::KundeId,
    storage::{ContentHash, StorageKey},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormularBezug {
    pub grundstueck_id: Option<uuid::Uuid>,
    pub bauherr_id: Option<KundeId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormularDatenstand {
    pub projekt_id: ProjectId,
    pub projekt_version: i64,
    pub bezug: FormularBezug,
    pub daten_hash: ContentHash,
    pub werte: Projektkontext,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quellenwahl {
    pub id: uuid::Uuid,
    pub bezeichnung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormularQuellen {
    pub datenstand: FormularDatenstand,
    pub grundstuecke: Vec<Quellenwahl>,
    pub bauherren: Vec<Quellenwahl>,
    pub hinweise: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormularInstanz {
    pub vorlagen_name: String,
    pub vorlagen_hash: ContentHash,
    pub vorlagen_storage_key: String,
    pub quellen: FormularQuellen,
    pub feldwerte: BTreeMap<String, Feldwert>,
}
impl CoreServices {
    pub async fn formular_quellen(
        &self,
        projekt: ProjectId,
        mut bezug: FormularBezug,
    ) -> CoreResult<FormularQuellen> {
        let p = self.repos.projects.get(projekt).await?;
        let mut werte = Projektkontext::aus_projekt(&p);
        // Die konkrete Auswahl ist maßgebend, nie eine zufällige erste Einheit.
        werte.0.remove("grundstueck.flaeche_m2");
        let mut hinweise = Vec::new();
        let grundstuecke = match crate::grundstueckskontext::ermitteln(&p) {
            Ok(g) => g,
            Err(e) => {
                hinweise.push(format!("Baugrundstücksdaten sind zu prüfen: {e}"));
                vec![]
            }
        };
        let bauherren = self.repos.kunden.bauherren(projekt).await?;
        if bezug.grundstueck_id.is_none() && grundstuecke.len() == 1 {
            bezug.grundstueck_id = Some(grundstuecke[0].id);
        }
        if bezug.bauherr_id.is_none() && bauherren.len() == 1 {
            bezug.bauherr_id = Some(bauherren[0].id);
        }
        if let Some(id) = bezug.grundstueck_id {
            let g = grundstuecke.iter().find(|g| g.id == id).ok_or_else(|| {
                CoreError::InvalidInput("Gewähltes Baugrundstück ist nicht mehr verfügbar".into())
            })?;
            if let Some(f) = g.bezugsflaeche_m2 {
                werte.setze("grundstueck.flaeche_m2", super::vorlage::zahl(f, 2));
            } else {
                hinweise
                    .push("Für das gewählte Baugrundstück fehlt eine gültige Bezugsfläche.".into());
            }
            werte.setze("grundstueck.bezeichnung", &g.bezeichnung);
        } else if grundstuecke.len() > 1 {
            hinweise.push("Mehrere Baugrundstücke: Bitte die Quelle wählen oder die Formularwerte manuell eingeben.".into());
        }
        if let Some(id) = bezug.bauherr_id {
            werte.bauherr(bauherren.iter().find(|b| b.id == id).ok_or_else(|| {
                CoreError::InvalidInput("Gewählter Bauherr gehört nicht zum Projekt".into())
            })?);
        } else if bauherren.len() > 1 {
            hinweise.push("Mehrere Bauherren: Bitte die Quelle wählen oder die Formularwerte manuell eingeben.".into());
        }
        let daten_hash = ContentHash::of(
            &serde_json::to_vec(&(&bezug, &werte))
                .map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        );
        Ok(FormularQuellen {
            datenstand: FormularDatenstand {
                projekt_id: projekt,
                projekt_version: p.version,
                bezug,
                daten_hash,
                werte,
            },
            grundstuecke: grundstuecke
                .iter()
                .map(|g| Quellenwahl {
                    id: g.id,
                    bezeichnung: g.bezeichnung.clone(),
                })
                .collect(),
            bauherren: bauherren
                .iter()
                .map(|b| Quellenwahl {
                    id: b.id.0,
                    bezeichnung: b.anzeigename(),
                })
                .collect(),
            hinweise,
        })
    }
    pub(crate) async fn formular_instanz_zusatz(
        &self,
        projekt: ProjectId,
        name: &str,
        pdf: &[u8],
        werte: &BTreeMap<String, Feldwert>,
        stand: Option<FormularDatenstand>,
    ) -> CoreResult<crate::commit::DokumentZusatz> {
        let quellen = self
            .formular_quellen(
                projekt,
                stand.as_ref().map(|s| s.bezug.clone()).unwrap_or_default(),
            )
            .await?;
        if let Some(s) = stand
            && (s.projekt_id != projekt || s.daten_hash != quellen.datenstand.daten_hash)
        {
            return Err(CoreError::InvalidInput("Die Formularquellen haben sich geändert. Bitte Projektdaten erneut laden und die Eingaben prüfen. Ihre Eingaben bleiben erhalten.".into()));
        }
        let hash = ContentHash::of(pdf);
        // Inhaltsadressierte Vorlage außerhalb der Projektdateiliste. Spätere Bibliotheksänderungen betreffen sie nicht.
        let key = StorageKey::new(format!("formularvorlagen/{hash}.pdf"))?;
        self.storage
            .save(&key, bytes::Bytes::copy_from_slice(pdf))
            .await?;
        let instanz = FormularInstanz {
            vorlagen_name: name.into(),
            vorlagen_hash: hash,
            vorlagen_storage_key: key.to_string(),
            quellen,
            feldwerte: werte.clone(),
        };
        Ok(crate::commit::DokumentZusatz {
            modul: "formularinstanz".into(),
            schema_version: 1,
            daten: serde_json::to_value(instanz)
                .map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        })
    }
    pub async fn formular_instanz(
        &self,
        id: crate::domain::DocumentId,
        version: i64,
    ) -> CoreResult<Option<FormularInstanz>> {
        self.load_version(id, version).await?;
        self.repos
            .commit
            .dokumentzusatz(id, version, "formularinstanz")
            .await?
            .map(|z| {
                if z.schema_version != 1 {
                    return Err(CoreError::InvalidInput(
                        "Unbekannte Formularinstanzversion".into(),
                    ));
                }
                serde_json::from_value(z.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))
            })
            .transpose()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormularSpeicheroptionen {
    pub vorgang: Option<uuid::Uuid>,
    pub datenstand: Option<FormularDatenstand>,
}
