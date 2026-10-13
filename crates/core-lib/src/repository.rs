//! Abstraktion der Datenbankschicht.
//!
//! core-lib weiß nicht, welche Datenbank genutzt wird. Die Implementierungen
//! liegen in eigenen Crates (`db-sqlite` für den Einzelplatz, `db-postgres`
//! für Server/Cloud) und werden zur Laufzeit als `Arc<dyn …>` injiziert.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::benutzer::{Benutzer, BenutzerAenderung, BenutzerId, NeuerBenutzer};
use crate::domain::{
    DocumentId, DocumentLock, DocumentVersion, NeuerProtokolleintrag, NewProject, Project,
    ProjectId, ProjectUpdate, Protokolleintrag,
};
use crate::error::CoreResult;
use crate::license::Arbeitsplatzbelegung;

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
    ///
    /// Optimistic Locking: Mit `basis = Some(n)` entsteht genau Version
    /// `n + 1`. Hat inzwischen jemand anderes Version `n + 1` angelegt, muss
    /// die Implementierung [`crate::CoreError::VersionConflict`] liefern
    /// (eindeutiger Schlüssel auf `(document_id, version)`).
    async fn add_version(
        &self,
        version: DocumentVersion,
        basis: Option<i64>,
    ) -> CoreResult<DocumentVersion>;

    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion>;

    /// Vollständige Änderungshistorie, älteste zuerst.
    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>>;

    async fn list_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentVersion>>;

    /// Sperrt das Dokument für `wer`. Atomar: Hält bereits ein anderer
    /// Benutzer die Sperre, [`crate::CoreError::Gesperrt`]. Sperrt `wer`
    /// erneut, bleibt die bestehende Sperre unverändert.
    async fn lock(
        &self,
        id: DocumentId,
        project: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<DocumentLock>;

    /// Hebt die Sperre auf. Mit `nur_von = Some(..)` nur, wenn sie diesem
    /// Benutzer gehört. Liefert die aufgehobene Sperre, `None` wenn keine bestand.
    async fn unlock(
        &self,
        id: DocumentId,
        nur_von: Option<BenutzerId>,
    ) -> CoreResult<Option<DocumentLock>>;

    async fn get_lock(&self, id: DocumentId) -> CoreResult<Option<DocumentLock>>;

    async fn locks_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentLock>>;
}

#[async_trait]
pub trait BenutzerRepository: Send + Sync {
    /// `anmeldename` ist bereits normalisiert. Doppelte Anmeldenamen:
    /// [`crate::CoreError::InvalidInput`].
    async fn create(&self, neu: NeuerBenutzer) -> CoreResult<Benutzer>;

    async fn get(&self, id: BenutzerId) -> CoreResult<Benutzer>;

    async fn by_anmeldename(&self, anmeldename: &str) -> CoreResult<Option<Benutzer>>;

    async fn list(&self) -> CoreResult<Vec<Benutzer>>;

    async fn update(&self, aenderung: BenutzerAenderung) -> CoreResult<Benutzer>;
}

#[async_trait]
pub trait ProtokollRepository: Send + Sync {
    async fn append(&self, eintrag: NeuerProtokolleintrag) -> CoreResult<()>;

    /// Neueste zuerst.
    async fn list_for_project(
        &self,
        project: ProjectId,
        limit: i64,
    ) -> CoreResult<Vec<Protokolleintrag>>;
}

/// Einstellungen der Installation (u. a. die Lizenzdatei) und belegte
/// Arbeitsplätze, gemeinsam für alle Arbeitsplätze an einer Datenbank.
#[async_trait]
pub trait LizenzRepository: Send + Sync {
    async fn einstellung(&self, schluessel: &str) -> CoreResult<Option<String>>;

    async fn einstellung_setzen(&self, schluessel: &str, wert: &str) -> CoreResult<()>;

    /// Setzt den Wert nur, wenn noch keiner gespeichert ist, und liefert den
    /// gespeicherten Wert (z. B. Beginn der Testversion).
    async fn einstellung_vorbelegen(&self, schluessel: &str, wert: &str) -> CoreResult<String>;

    /// Belegt einen Arbeitsplatz oder aktualisiert dessen letzte Meldung.
    /// Plätze, die sich vor `abgelaufen_vor` zuletzt gemeldet haben, gelten
    /// als frei. Atomar: Zwei Rechner können nicht gleichzeitig den letzten
    /// freien Platz bekommen. `false`, wenn alle `max` Plätze belegt sind.
    async fn arbeitsplatz_belegen(
        &self,
        belegung: Arbeitsplatzbelegung,
        max: u32,
        abgelaufen_vor: DateTime<Utc>,
    ) -> CoreResult<bool>;

    async fn arbeitsplatz_freigeben(&self, arbeitsplatz: &str) -> CoreResult<()>;

    /// Aktive Belegungen, älteste zuerst.
    async fn arbeitsplaetze(
        &self,
        abgelaufen_vor: DateTime<Utc>,
    ) -> CoreResult<Vec<Arbeitsplatzbelegung>>;
}

/// Gespeicherte Angaben zum Bauantrag eines Projekts (JSON von
/// [`crate::xbau::Bauantrag`]).
#[derive(Debug, Clone, PartialEq)]
pub struct BauantragDatensatz {
    pub daten: String,
    pub version: i64,
    pub geaendert_von: String,
    pub geaendert_am: DateTime<Utc>,
}

#[async_trait]
pub trait BauantragRepository: Send + Sync {
    async fn bauantrag(&self, project: ProjectId) -> CoreResult<Option<BauantragDatensatz>>;

    /// Speichert mit Optimistic Locking: `expected_version` ist die zuletzt
    /// gelesene Version (0 = noch keiner gespeichert). Liefert die neue
    /// Version oder [`crate::CoreError::VersionConflict`].
    async fn bauantrag_speichern(
        &self,
        project: ProjectId,
        daten: &str,
        expected_version: i64,
        geaendert_von: &str,
    ) -> CoreResult<i64>;
}

/// Bündelt alle Repositories einer Datenbank-Implementierung, damit die
/// Laufzeitumgebung genau ein Objekt injizieren muss.
#[derive(Clone)]
pub struct Repositories {
    pub projects: Arc<dyn ProjectRepository>,
    pub documents: Arc<dyn DocumentRepository>,
    pub benutzer: Arc<dyn BenutzerRepository>,
    pub protokoll: Arc<dyn ProtokollRepository>,
    pub lizenz: Arc<dyn LizenzRepository>,
    pub bauantraege: Arc<dyn BauantragRepository>,
}
