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
}
