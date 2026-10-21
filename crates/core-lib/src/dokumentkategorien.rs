//! Fachliche Dokumentkategorien. Katalog und Vorlagenvorgaben sind installationsweit;
//! Dokumentzuordnungen besitzen eigene Revisionen, unabhängig von Dateiversionen.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    commit::{DokumentZusatz, Fachstand},
    domain::{Aktion, DocumentId, ProjectId},
    storage::ContentHash,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const KATALOG: &str = "dokumentkategorien/v1";
const MODUL: &str = "dokumentkategorie";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Dokumentkategorie {
    pub id: String,
    pub name: String,
    pub eltern_id: Option<String>,
    pub aktiv: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct KategorienKatalog {
    #[ts(type = "number")]
    pub revision: i64,
    pub kategorien: Vec<Dokumentkategorie>,
}
impl Default for KategorienKatalog {
    fn default() -> Self {
        let haupt = [
            ("bauantrag", "Bauantragsformular"),
            ("baubeschreibung", "Baubeschreibung"),
            ("lageplan", "Lageplan"),
            ("grundriss", "Grundriss"),
            ("schnitt", "Schnitt"),
            ("ansicht", "Ansicht"),
            ("berechnung", "Berechnung"),
            ("statik", "Standsicherheitsnachweis / Statik"),
            ("gutachten", "Gutachten"),
            ("anschreiben", "Anschreiben"),
            ("email", "E-Mail"),
            ("bescheid", "Bescheid"),
            ("foto", "Foto"),
            ("grundbuch", "Grundbuchauszug"),
            ("schriftverkehr", "Schriftverkehr Behörde"),
            ("bescheinigung", "Bescheinigungen und Nachweise"),
            ("entwaesserung", "Entwässerung"),
        ];
        let mut kategorien: Vec<_> = haupt
            .into_iter()
            .map(|(id, name)| Dokumentkategorie {
                id: id.into(),
                name: name.into(),
                eltern_id: None,
                aktiv: true,
            })
            .collect();
        for (id, name) in [
            ("baugrundstueck", "Baugrundstück"),
            ("grz", "GRZ-Berechnung"),
            ("gfz", "GFZ-Berechnung"),
            ("vollgeschosse", "Vollgeschosse"),
            ("abstandsflaechen", "Abstandsflächen"),
            ("bauvolumen", "Bauvolumen"),
        ] {
            kategorien.push(Dokumentkategorie {
                id: id.into(),
                name: name.into(),
                eltern_id: Some("berechnung".into()),
                aktiv: true,
            });
        }
        Self {
            revision: 0,
            kategorien,
        }
    }
}
impl KategorienKatalog {
    /// Neue Standardkategorien auch in bereits gespeicherten Bürokatalogen anbieten.
    /// Bestehende Namen, Hierarchien, Archivierungen und Revisionen bleiben erhalten.
    /// Hat das Büro bereits eine gleichnamige Kategorie auf derselben Ebene angelegt,
    /// wird die Standardkategorie nicht zusätzlich angeboten.
    fn standard_ergaenzen(mut self) -> Self {
        for mut k in Self::default().kategorien {
            let gleichnamig = self.kategorien.iter().any(|v| {
                v.eltern_id == k.eltern_id && v.name.to_lowercase() == k.name.to_lowercase()
            });
            if !gleichnamig && !self.kategorien.iter().any(|vorhanden| vorhanden.id == k.id) {
                if let Some(parent) = &k.eltern_id
                    && self.kategorien.iter().any(|p| &p.id == parent && !p.aktiv)
                {
                    k.aktiv = false;
                }
                self.kategorien.push(k);
            }
        }
        self
    }

    pub fn kategorie(&self, id: &str) -> CoreResult<&Dokumentkategorie> {
        self.kategorien
            .iter()
            .find(|k| k.id == id)
            .ok_or_else(|| CoreError::InvalidInput("Unbekannte Dokumentkategorie".into()))
    }
    fn zuweisbar(&self, id: Option<&str>) -> CoreResult<()> {
        if let Some(id) = id
            && !self.kategorie(id)?.aktiv
        {
            return Err(CoreError::InvalidInput(
                "Archivierte Kategorie kann nicht neu zugeordnet werden".into(),
            ));
        }
        Ok(())
    }
    fn validieren(&self, alt: &Self) -> CoreResult<()> {
        if self.kategorien.len() > 500 || self.kategorien.len() < alt.kategorien.len() {
            return Err(CoreError::InvalidInput(
                "Höchstens 500 Kategorien; bestehende Kategorien archivieren statt löschen".into(),
            ));
        }
        let mut ids = HashSet::new();
        let mut namen = HashSet::new();
        for k in &self.kategorien {
            if k.id.is_empty()
                || k.id.len() > 80
                || !k
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !ids.insert(&k.id)
                || k.name.trim().is_empty()
                || k.name != k.name.trim()
                || k.name.chars().count() > 120
                || k.name.chars().any(char::is_control)
                || !namen.insert((k.eltern_id.clone(), k.name.to_lowercase()))
            {
                return Err(CoreError::InvalidInput(
                    "Kategorien benötigen eindeutige Kennungen und Namen (maximal 120 Zeichen)"
                        .into(),
                ));
            }
            if let Some(id) = &k.eltern_id {
                let eltern = self.kategorie(id)?;
                if id == &k.id || eltern.eltern_id.is_some() || (k.aktiv && !eltern.aktiv) {
                    return Err(CoreError::InvalidInput("Nur zwei Kategorieebenen; aktive Unterkategorien benötigen eine aktive Hauptkategorie".into()));
                }
            }
        }
        for k in &alt.kategorien {
            self.kategorie(&k.id)?;
        }
        Ok(())
    }
    pub fn snapshot(&self, id: &str) -> CoreResult<KategorieStand> {
        let k = self.kategorie(id)?;
        Ok(KategorieStand {
            id: k.id.clone(),
            name: k.name.clone(),
            hauptkategorie: k
                .eltern_id
                .as_deref()
                .map(|p| self.kategorie(p).map(|v| v.name.clone()))
                .transpose()?,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct KategorieStand {
    pub id: String,
    pub name: String,
    pub hauptkategorie: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct KategorieZuordnung {
    #[ts(type = "number")]
    pub revision: i64,
    pub kategorie: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct DokumentKategorie {
    pub dokument_id: DocumentId,
    #[serde(flatten)]
    pub zuordnung: KategorieZuordnung,
}
fn json(e: serde_json::Error) -> CoreError {
    CoreError::Database(e.to_string())
}
impl CoreServices {
    pub(crate) async fn kategorie_pruefen(&self, id: Option<&str>) -> CoreResult<()> {
        self.kategorienkatalog().await?.zuweisbar(id)
    }

    pub async fn kategorienkatalog(&self) -> CoreResult<KategorienKatalog> {
        self.repos
            .lizenz
            .einstellung(KATALOG)
            .await?
            .map(|s| {
                serde_json::from_str::<KategorienKatalog>(&s)
                    .map(KategorienKatalog::standard_ergaenzen)
                    .map_err(json)
            })
            .unwrap_or_else(|| Ok(KategorienKatalog::default()))
    }
    pub async fn kategorienkatalog_speichern(
        &self,
        mut neu: KategorienKatalog,
        wer: &Benutzer,
    ) -> CoreResult<KategorienKatalog> {
        self.pruefe(wer, Recht::BueroangabenAendern).await?;
        let raw = self.repos.lizenz.einstellung(KATALOG).await?;
        let alt: KategorienKatalog = raw
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(json)?
            .unwrap_or_default();
        let alt = alt.standard_ergaenzen();
        if alt.revision != neu.revision {
            return Err(CoreError::VersionConflict {
                expected: neu.revision,
                actual: alt.revision,
            });
        }
        neu.validieren(&alt)?;
        neu.revision = neu
            .revision
            .checked_add(1)
            .ok_or_else(|| CoreError::InvalidInput("Katalogrevision zu groß".into()))?;
        if !self
            .repos
            .lizenz
            .einstellung_vergleichen(
                KATALOG,
                raw.as_deref(),
                &serde_json::to_string(&neu).map_err(json)?,
            )
            .await?
        {
            return Err(CoreError::VersionConflict {
                expected: alt.revision,
                actual: self.kategorienkatalog().await?.revision,
            });
        }
        Ok(neu)
    }
    pub async fn vorlagenkategorie(&self, hash: &ContentHash) -> CoreResult<KategorieZuordnung> {
        let key = format!("vorlagenkategorie/{}", hash.as_str());
        self.repos
            .lizenz
            .einstellung(&key)
            .await?
            .map(|s| serde_json::from_str(&s).map_err(json))
            .unwrap_or_else(|| {
                Ok(KategorieZuordnung {
                    revision: 0,
                    kategorie: None,
                })
            })
    }
    pub async fn vorlagenkategorie_speichern(
        &self,
        hash: &ContentHash,
        mut neu: KategorieZuordnung,
        wer: &Benutzer,
    ) -> CoreResult<KategorieZuordnung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.kategorienkatalog()
            .await?
            .zuweisbar(neu.kategorie.as_deref())?;
        let key = format!("vorlagenkategorie/{}", hash.as_str());
        let raw = self.repos.lizenz.einstellung(&key).await?;
        let alt: KategorieZuordnung = raw
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(json)?
            .unwrap_or(KategorieZuordnung {
                revision: 0,
                kategorie: None,
            });
        if alt.revision != neu.revision {
            return Err(CoreError::VersionConflict {
                expected: neu.revision,
                actual: alt.revision,
            });
        }
        neu.revision = neu
            .revision
            .checked_add(1)
            .ok_or_else(|| CoreError::InvalidInput("Vorlagenrevision zu groß".into()))?;
        if !self
            .repos
            .lizenz
            .einstellung_vergleichen(
                &key,
                raw.as_deref(),
                &serde_json::to_string(&neu).map_err(json)?,
            )
            .await?
        {
            return Err(CoreError::VersionConflict {
                expected: alt.revision,
                actual: self.vorlagenkategorie(hash).await?.revision,
            });
        }
        Ok(neu)
    }
    pub async fn dokumentkategorien(
        &self,
        projekt: ProjectId,
    ) -> CoreResult<Vec<DokumentKategorie>> {
        self.repos.projects.get(projekt).await?;
        let staende = self.repos.commit.fachstaende(projekt, MODUL).await?;
        let docs = crate::projektablage::versionen(self.list_documents(projekt).await?, false);
        let mut result = Vec::new();
        for d in docs {
            let zuordnung = if let Some(f) = staende
                .iter()
                .find(|f| f.objekt == d.document_id.0.to_string())
            {
                if f.schema_version != 1 {
                    return Err(CoreError::InvalidInput(
                        "Unbekanntes Kategorieformat".into(),
                    ));
                }
                KategorieZuordnung {
                    revision: f.revision,
                    kategorie: serde_json::from_value::<Option<String>>(f.daten.clone())
                        .map_err(json)?,
                }
            } else {
                let zusatz = self
                    .repos
                    .commit
                    .dokumentzusatz(d.document_id, 1, MODUL)
                    .await?;
                let kategorie = if let Some(z) = zusatz {
                    if z.schema_version != 1 {
                        return Err(CoreError::InvalidInput(
                            "Unbekanntes Kategorieformat".into(),
                        ));
                    }
                    serde_json::from_value(z.daten).map_err(json)?
                } else {
                    None
                };
                KategorieZuordnung {
                    revision: 0,
                    kategorie,
                }
            };
            result.push(DokumentKategorie {
                dokument_id: d.document_id,
                zuordnung,
            });
        }
        Ok(result)
    }
    pub async fn dokumentkategorie_speichern(
        &self,
        id: DocumentId,
        neu: KategorieZuordnung,
        wer: &Benutzer,
    ) -> CoreResult<DokumentKategorie> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        if neu.revision < 0 {
            return Err(CoreError::InvalidInput("Ungültige Kategorieversion".into()));
        }
        let d = self.repos.documents.latest(id).await?;
        if let Some(s) = self.repos.documents.get_lock(id).await?
            && s.benutzer_id != wer.id
        {
            return Err(CoreError::Gesperrt {
                von: s.benutzer_name,
                seit: s.seit,
            });
        }
        self.kategorienkatalog()
            .await?
            .zuweisbar(neu.kategorie.as_deref())?;
        let f = Fachstand {
            projekt_id: d.project_id,
            modul: MODUL.into(),
            objekt: id.0.to_string(),
            schema_version: 1,
            revision: neu.revision,
            daten: serde_json::to_value(&neu.kategorie).map_err(json)?,
        };
        let f = self
            .repos
            .commit
            .fachstand(
                f,
                neu.revision,
                Self::audit(
                    wer,
                    Aktion::DokumentKategorieGeaendert,
                    Some(d.project_id),
                    Some(id),
                    format!(
                        "{}: Kategorie {}",
                        d.file_name,
                        neu.kategorie.as_deref().unwrap_or("Ohne Kategorie")
                    ),
                ),
            )
            .await?;
        Ok(DokumentKategorie {
            dokument_id: id,
            zuordnung: KategorieZuordnung {
                revision: f.revision,
                kategorie: neu.kategorie,
            },
        })
    }
    /// Bestehende Zuordnung beim Erstellen einer bearbeiteten Vorlagenkopie erhalten.
    /// Eine bereits konfigurierte Zielvorlage wird niemals überschrieben.
    pub async fn vorlagenkategorie_kopieren(
        &self,
        quelle: &ContentHash,
        ziel: &ContentHash,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        let kategorie = self.vorlagenkategorie(quelle).await?.kategorie;
        self.vorlagenkategorie_erben(ziel, kategorie, wer).await
    }
    pub async fn dokumentkategorie_als_vorlage(
        &self,
        id: DocumentId,
        ziel: &ContentHash,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        let d = self.repos.documents.latest(id).await?;
        let kategorie = self
            .dokumentkategorien(d.project_id)
            .await?
            .into_iter()
            .find(|k| k.dokument_id == id)
            .and_then(|k| k.zuordnung.kategorie);
        self.vorlagenkategorie_erben(ziel, kategorie, wer).await
    }
    async fn vorlagenkategorie_erben(
        &self,
        ziel: &ContentHash,
        kategorie: Option<String>,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        if let Some(id) = &kategorie {
            self.kategorienkatalog().await?.kategorie(id)?;
            // Auch eine archivierte, bereits zugeordnete Kategorie bleibt erhalten.
            let key = format!("vorlagenkategorie/{}", ziel.as_str());
            let neu = KategorieZuordnung {
                revision: 1,
                kategorie,
            };
            self.repos
                .lizenz
                .einstellung_vergleichen(&key, None, &serde_json::to_string(&neu).map_err(json)?)
                .await?;
        }
        Ok(())
    }
    pub(crate) async fn kategorie_zusatz(&self, hash: &ContentHash) -> CoreResult<DokumentZusatz> {
        let zuordnung = self.vorlagenkategorie(hash).await?;
        Ok(DokumentZusatz {
            modul: MODUL.into(),
            schema_version: 1,
            daten: serde_json::to_value(zuordnung.kategorie).map_err(json)?,
        })
    }
    pub async fn kategoriesnapshots(
        &self,
        projekt: ProjectId,
    ) -> CoreResult<BTreeMap<String, Option<KategorieStand>>> {
        let katalog = self.kategorienkatalog().await?;
        self.dokumentkategorien(projekt)
            .await?
            .into_iter()
            .map(|d| {
                Ok((
                    d.dokument_id.0.to_string(),
                    d.zuordnung
                        .kategorie
                        .as_deref()
                        .map(|id| katalog.snapshot(id))
                        .transpose()?,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
