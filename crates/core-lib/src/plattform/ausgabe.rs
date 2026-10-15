//! Dauerhafte Ausgabeaufträge; das DMS bleibt die Quelle, Dateisystemzugriff liegt im Adapter.
use super::*;
use crate::{
    domain::{DocumentId, DocumentVersion},
    storage::{ContentHash, StorageKey},
};
use chrono::{DateTime, Duration, Utc};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AusgabeStatus {
    Laeuft,
    Fertig,
    Fehler,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Ausgabeauftrag {
    pub id: Uuid,
    pub dokument_id: DocumentId,
    #[ts(type = "number")]
    pub dokument_version: i64,
    pub ziel_root: String,
    pub relativer_pfad: String,
    #[ts(type = "string")]
    pub sha256: ContentHash,
    pub status: AusgabeStatus,
    pub versuche: u32,
    pub beansprucht_bis: DateTime<Utc>,
    pub versuch_id: Uuid,
    pub naechster_versuch: DateTime<Utc>,
    pub fehler: Option<String>,
    pub aktualisiert_am: DateTime<Utc>,
}
impl CoreServices {
    pub async fn ausgabeauftraege(
        &self,
        p: ProjectId,
    ) -> CoreResult<Vec<(Fachstand, Ausgabeauftrag)>> {
        self.repos.projects.get(p).await?;
        self.repos
            .commit
            .fachstaende(p, "ausgaben")
            .await?
            .into_iter()
            .map(|f| {
                if f.schema_version != 1 {
                    return Err(CoreError::InvalidInput(
                        "Unbekannte Ausgabeauftragsversion".into(),
                    ));
                }
                let a = serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                Ok((f, a))
            })
            .collect()
    }
    pub async fn ausgabe_beanspruchen(
        &self,
        d: &DocumentVersion,
        ziel_root: String,
        relativer_pfad: String,
        erzwingen: bool,
        wer: &Benutzer,
    ) -> CoreResult<Option<(Fachstand, Ausgabeauftrag)>> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        if ziel_root.is_empty() || ziel_root.len() > 4096 {
            return Err(CoreError::InvalidInput("Ungültiges Ausgabeziel".into()));
        }
        StorageKey::new(&relativer_pfad)?;
        // Prüft die konkrete, unveränderliche Quellversion und ihren Hash.
        let meta = self
            .repos
            .documents
            .history(d.document_id)
            .await?
            .into_iter()
            .find(|v| v.version == d.version && v.project_id == d.project_id)
            .ok_or_else(|| CoreError::NotFound("Auszugebende Dokumentversion".into()))?;
        if meta.sha256 != d.sha256 {
            return Err(CoreError::InvalidInput(
                "Dokumenthash hat sich geändert".into(),
            ));
        }
        let id = Uuid::new_v5(
            &d.project_id.0,
            format!(
                "ausgabe/{}/{}/{ziel_root}/{relativer_pfad}",
                d.document_id.0, d.version
            )
            .as_bytes(),
        );
        let alt = self
            .ausgabeauftraege(d.project_id)
            .await?
            .into_iter()
            .find(|(_, a)| a.id == id);
        let jetzt = Utc::now();
        let mut revision = 0;
        let mut versuche = 0;
        if let Some((f, a)) = alt {
            if a.status == AusgabeStatus::Laeuft && a.beansprucht_bis > jetzt {
                return Ok(None);
            }
            if !erzwingen && (a.status == AusgabeStatus::Fertig || a.naechster_versuch > jetzt) {
                return Ok(None);
            }
            revision = f.revision;
            versuche = a.versuche;
        }
        let a = Ausgabeauftrag {
            id,
            dokument_id: d.document_id,
            dokument_version: d.version,
            ziel_root,
            relativer_pfad,
            sha256: d.sha256.clone(),
            status: AusgabeStatus::Laeuft,
            versuche: versuche.saturating_add(1),
            beansprucht_bis: jetzt + Duration::minutes(5),
            versuch_id: Uuid::new_v4(),
            naechster_versuch: jetzt,
            fehler: None,
            aktualisiert_am: jetzt,
        };
        match self
            .ausgabe_stand(d.project_id, a.clone(), revision, wer)
            .await
        {
            Ok(f) => Ok(Some((f, a))),
            Err(CoreError::VersionConflict { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }
    async fn ausgabe_stand(
        &self,
        p: ProjectId,
        a: Ausgabeauftrag,
        basis: i64,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p,
                    modul: "ausgaben".into(),
                    objekt: a.id.to_string(),
                    schema_version: 1,
                    revision: basis,
                    daten: serde_json::to_value(&a).map_err(json_err)?,
                },
                basis,
                Self::audit(
                    wer,
                    Aktion::AusgabeauftragGeaendert,
                    Some(p),
                    Some(a.dokument_id),
                    format!(
                        "Ausgabeauftrag {}: {:?}, Versuch {}",
                        a.id, a.status, a.versuche
                    ),
                ),
            )
            .await
    }
    pub async fn ausgabe_abschliessen(
        &self,
        f: Fachstand,
        mut a: Ausgabeauftrag,
        fehler: Option<String>,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        let aktuell = self
            .ausgabeauftraege(f.projekt_id)
            .await?
            .into_iter()
            .find(|(_, b)| b.id == a.id)
            .ok_or_else(|| CoreError::NotFound("Ausgabeauftrag".into()))?;
        if aktuell.1.versuch_id != a.versuch_id || aktuell.0.revision != f.revision {
            return Err(CoreError::VersionConflict {
                expected: f.revision,
                actual: aktuell.0.revision,
            });
        }
        a.aktualisiert_am = Utc::now();
        a.status = if fehler.is_some() {
            AusgabeStatus::Fehler
        } else {
            AusgabeStatus::Fertig
        };
        a.naechster_versuch =
            a.aktualisiert_am + Duration::seconds(30 * (1i64 << a.versuche.min(6)));
        a.fehler = fehler.map(|s| s.chars().take(4000).collect());
        self.ausgabe_stand(f.projekt_id, a, f.revision, wer).await?;
        Ok(())
    }
}
