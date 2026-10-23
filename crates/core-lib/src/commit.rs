//! Atomare Schreibgrenzen: Fachdatensatz und Audit werden gemeinsam bestätigt.
use crate::{
    CoreResult,
    domain::{DocumentVersion, NeuerProtokolleintrag, Project, ProjectId},
    einreichung::Einreichung,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Fachmodule besitzen das JSON-Schema; der Adapter kennt nur Revision und Identität.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
pub struct Fachstand {
    pub projekt_id: ProjectId,
    pub modul: String,
    pub objekt: String,
    pub schema_version: u32,
    #[ts(type = "number")]
    pub revision: i64,
    pub daten: serde_json::Value,
}
/// Vom DMS atomar gespeicherte, modulgebundene Metadaten einer konkreten Version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DokumentZusatz {
    pub modul: String,
    pub schema_version: u32,
    pub daten: serde_json::Value,
}
pub struct DokumentCommit {
    pub dokument: DocumentVersion,
    pub basis: Option<i64>,
    pub neuanlage: bool,
    pub zusaetze: Vec<DokumentZusatz>,
}
#[async_trait]
pub trait CommitRepository: Send + Sync {
    async fn bauantrag(
        &self,
        projekt: ProjectId,
        daten: String,
        basis: i64,
        audit: NeuerProtokolleintrag,
    ) -> CoreResult<i64>;
    async fn projekt(
        &self,
        p: Project,
        basis: Option<i64>,
        audit: NeuerProtokolleintrag,
    ) -> CoreResult<Project>;
    async fn dokument(
        &self,
        auftrag: DokumentCommit,
        audit: NeuerProtokolleintrag,
    ) -> CoreResult<DocumentVersion>;
    async fn dokumentzusatz(
        &self,
        id: crate::domain::DocumentId,
        version: i64,
        modul: &str,
    ) -> CoreResult<Option<DokumentZusatz>>;
    async fn einreichung(
        &self,
        p: Einreichung,
        basis: Option<i64>,
        audit: NeuerProtokolleintrag,
    ) -> CoreResult<Einreichung>;
    async fn fachstand(
        &self,
        p: Fachstand,
        basis: i64,
        audit: NeuerProtokolleintrag,
    ) -> CoreResult<Fachstand>;
    async fn fachverlauf(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Vec<Fachstand>>;
    async fn fachstaende(&self, projekt: ProjectId, modul: &str) -> CoreResult<Vec<Fachstand>>;
    /// Deterministische Seite nach Objektkennung; höchstens 100 Nutzdaten pro Abruf.
    async fn fachseite(
        &self,
        projekt: ProjectId,
        modul: &str,
        nach: Option<&str>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let mut daten = self.fachstaende(projekt, modul).await?;
        daten.sort_by(|a, b| a.objekt.cmp(&b.objekt));
        Ok(daten
            .into_iter()
            .filter(|f| nach.is_none_or(|n| f.objekt.as_str() > n))
            .take(limit.clamp(1, 100) as usize)
            .collect())
    }
    /// Neueste Revisionen zuerst, exklusiver Revisionscursor.
    async fn fachverlaufsseite(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
        vor: Option<i64>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let mut daten = self.fachverlauf(projekt, modul, objekt).await?;
        daten.sort_by_key(|f| std::cmp::Reverse(f.revision));
        Ok(daten
            .into_iter()
            .filter(|f| vor.is_none_or(|v| f.revision < v))
            .take(limit.clamp(1, 100) as usize)
            .collect())
    }
    async fn fachobjekt(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Option<Fachstand>> {
        Ok(self
            .fachstaende(projekt, modul)
            .await?
            .into_iter()
            .find(|f| f.objekt == objekt))
    }
    /// Suchkandidaten mit normalisiertem Text; ältere Datensätze ohne Projektion
    /// bleiben Kandidaten und werden im Core vollständig geprüft.
    async fn suchtextseite(
        &self,
        projekt: ProjectId,
        nach: Option<&str>,
        suche: &str,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let mut daten = self.fachstaende(projekt, "suchtexte").await?;
        daten.sort_by(|a, b| a.objekt.cmp(&b.objekt));
        Ok(daten
            .into_iter()
            .filter(|f| nach.is_none_or(|n| f.objekt.as_str() > n))
            .filter(|f| {
                f.daten
                    .get("_suchtext_norm")
                    .and_then(|v| v.as_str())
                    .is_none_or(|s| s.contains(suche))
            })
            .take(limit.clamp(1, 100) as usize)
            .collect())
    }
    /// Schlanke Änderungsprüfung ohne erneute Übertragung der Nutzdaten.
    async fn fachrevisionen(
        &self,
        projekt: ProjectId,
        modul: &str,
    ) -> CoreResult<Vec<(String, i64)>> {
        Ok(self
            .fachstaende(projekt, modul)
            .await?
            .into_iter()
            .map(|f| (f.objekt, f.revision))
            .collect())
    }
}
