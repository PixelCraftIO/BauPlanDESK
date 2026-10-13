//! Abstraktion der Datenbankschicht.
//!
//! core-lib weiß nicht, welche Datenbank genutzt wird. Die Implementierungen
//! liegen in eigenen Crates (`db-sqlite` für den Einzelplatz, `db-postgres`
//! für Server/Cloud) und werden zur Laufzeit als `Arc<dyn …>` injiziert.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate};
use crate::error::CoreResult;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: NewProject) -> CoreResult<Project>;

    async fn get(&self, id: ProjectId) -> CoreResult<Project>;

    async fn list(&self) -> CoreResult<Vec<Project>>;

    /// Muss atomar prüfen, dass `update.expected_version` der gespeicherten
    /// Version entspricht, und die Version dann um 1 erhöhen
    /// (`UPDATE … WHERE id = ? AND version = ?`). Andernfalls
    /// [`crate::CoreError::VersionConflict`] bzw. `NotFound`.
    async fn update(&self, update: ProjectUpdate) -> CoreResult<Project>;

    async fn delete(&self, id: ProjectId, expected_version: i64) -> CoreResult<()>;
}

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    /// Legt eine neue Version an. `version` wird von der Implementierung
    /// vergeben (fortlaufend je Dokument, beginnend bei 1).
    async fn add_version(&self, version: DocumentVersion) -> CoreResult<DocumentVersion>;

    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion>;

    /// Vollständige Änderungshistorie, älteste zuerst.
    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>>;

    async fn list_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentVersion>>;
}

/// Bündelt alle Repositories einer Datenbank-Implementierung, damit die
/// Laufzeitumgebung genau ein Objekt injizieren muss.
#[derive(Clone)]
pub struct Repositories {
    pub projects: Arc<dyn ProjectRepository>,
    pub documents: Arc<dyn DocumentRepository>,
}
