//! SQLite-Backend für die Einzelplatz-Version.

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::repository::{DocumentRepository, ProjectRepository, Repositories};
use core_lib::storage::ContentHash;
use core_lib::{CoreError, CoreResult};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    /// Öffnet (oder erzeugt) die Datenbankdatei und führt Migrationen aus.
    pub async fn open(path: &Path) -> CoreResult<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        Self::connect(options, 4).await
    }

    /// In-Memory-Datenbank für Tests.
    pub async fn in_memory() -> CoreResult<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(db_err)?
            .foreign_keys(true);
        // Eine Verbindung, sonst sieht jede Verbindung eine eigene leere DB.
        Self::connect(options, 1).await
    }

    async fn connect(options: SqliteConnectOptions, max: u32) -> CoreResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max)
            .connect_with(options)
            .await
            .map_err(db_err)?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Repositories zur Injektion in [`core_lib::CoreServices`].
    pub fn repositories(&self) -> Repositories {
        let this = Arc::new(self.clone());
        Repositories {
            projects: this.clone(),
            documents: this,
        }
    }
}

fn db_err(e: sqlx::Error) -> CoreError {
    CoreError::Database(e.to_string())
}

fn parse_uuid(s: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| CoreError::Database(format!("UUID {s:?}: {e}")))
}

fn project_from_row(row: &SqliteRow) -> CoreResult<Project> {
    let id: String = row.try_get("id").map_err(db_err)?;
    let land: String = row.try_get("bundesland").map_err(db_err)?;
    Ok(Project {
        id: ProjectId(parse_uuid(&id)?),
        name: row.try_get("name").map_err(db_err)?,
        bundesland: Bundesland::from_str(&land)?,
        grundstuecksflaeche_m2: row.try_get("grundstuecksflaeche_m2").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        created_at: row
            .try_get::<DateTime<Utc>, _>("created_at")
            .map_err(db_err)?,
        updated_at: row
            .try_get::<DateTime<Utc>, _>("updated_at")
            .map_err(db_err)?,
    })
}

fn document_from_row(row: &SqliteRow) -> CoreResult<DocumentVersion> {
    let document_id: String = row.try_get("document_id").map_err(db_err)?;
    let project_id: String = row.try_get("project_id").map_err(db_err)?;
    let sha256: String = row.try_get("sha256").map_err(db_err)?;
    let size: i64 = row.try_get("size_bytes").map_err(db_err)?;
    Ok(DocumentVersion {
        document_id: DocumentId(parse_uuid(&document_id)?),
        project_id: ProjectId(parse_uuid(&project_id)?),
        version: row.try_get("version").map_err(db_err)?,
        file_name: row.try_get("file_name").map_err(db_err)?,
        storage_key: row.try_get("storage_key").map_err(db_err)?,
        sha256: ContentHash::try_from(sha256)?,
        size_bytes: u64::try_from(size).map_err(|e| CoreError::Database(e.to_string()))?,
        created_by: row.try_get("created_by").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
    })
}

#[async_trait]
impl ProjectRepository for SqliteDatabase {
    async fn create(&self, p: NewProject) -> CoreResult<Project> {
        let now = Utc::now();
        let project = Project {
            id: ProjectId::new(),
            name: p.name,
            bundesland: p.bundesland,
            grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
            version: 1,
            created_at: now,
            updated_at: now,
        };
        sqlx::query(
            "INSERT INTO projects (id, name, bundesland, grundstuecksflaeche_m2, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(project.id.to_string())
        .bind(&project.name)
        .bind(project.bundesland.code())
        .bind(project.grundstuecksflaeche_m2)
        .bind(project.version)
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(project)
    }

    async fn get(&self, id: ProjectId) -> CoreResult<Project> {
        let row = sqlx::query("SELECT * FROM projects WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| CoreError::NotFound(format!("Projekt {id}")))?;
        project_from_row(&row)
    }

    async fn list(&self) -> CoreResult<Vec<Project>> {
        sqlx::query("SELECT * FROM projects ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(project_from_row)
            .collect()
    }

    async fn update(&self, u: ProjectUpdate) -> CoreResult<Project> {
        let row = sqlx::query(
            "UPDATE projects
                SET name = ?, bundesland = ?, grundstuecksflaeche_m2 = ?,
                    version = version + 1, updated_at = ?
              WHERE id = ? AND version = ?
          RETURNING *",
        )
        .bind(&u.name)
        .bind(u.bundesland.code())
        .bind(u.grundstuecksflaeche_m2)
        .bind(Utc::now())
        .bind(u.id.to_string())
        .bind(u.expected_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        match row {
            Some(row) => project_from_row(&row),
            None => Err(self.conflict_or_missing(u.id, u.expected_version).await),
        }
    }

    async fn delete(&self, id: ProjectId, expected_version: i64) -> CoreResult<()> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ? AND version = ?")
            .bind(id.to_string())
            .bind(expected_version)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(self.conflict_or_missing(id, expected_version).await)
        }
    }
}

impl SqliteDatabase {
    async fn conflict_or_missing(&self, id: ProjectId, expected: i64) -> CoreError {
        match self.get(id).await {
            Ok(current) => CoreError::VersionConflict {
                expected,
                actual: current.version,
            },
            Err(e) => e,
        }
    }
}

#[async_trait]
impl DocumentRepository for SqliteDatabase {
    async fn add_version(&self, mut v: DocumentVersion) -> CoreResult<DocumentVersion> {
        // Versionsnummer und Insert in einem Statement: kein Rennen zwischen
        // zwei gleichzeitigen Uploads desselben Dokuments.
        let size =
            i64::try_from(v.size_bytes).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        let version: i64 = sqlx::query_scalar(
            "INSERT INTO document_versions
                (document_id, project_id, version, file_name, storage_key, sha256, size_bytes, created_by, created_at)
             SELECT ?1, ?2, COALESCE(MAX(version), 0) + 1, ?3, ?4, ?5, ?6, ?7, ?8
               FROM document_versions WHERE document_id = ?1
          RETURNING version",
        )
        .bind(v.document_id.0.to_string())
        .bind(v.project_id.to_string())
        .bind(&v.file_name)
        .bind(&v.storage_key)
        .bind(v.sha256.as_str())
        .bind(size)
        .bind(&v.created_by)
        .bind(v.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        v.version = version;
        Ok(v)
    }

    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion> {
        let row = sqlx::query(
            "SELECT * FROM document_versions WHERE document_id = ? ORDER BY version DESC LIMIT 1",
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| CoreError::NotFound(format!("Dokument {}", id.0)))?;
        document_from_row(&row)
    }

    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>> {
        sqlx::query("SELECT * FROM document_versions WHERE document_id = ? ORDER BY version")
            .bind(id.0.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(document_from_row)
            .collect()
    }

    async fn list_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        sqlx::query(
            "SELECT * FROM document_versions WHERE project_id = ? ORDER BY document_id, version",
        )
        .bind(project.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(document_from_row)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn optimistic_locking() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        let repos = db.repositories();
        let p = repos
            .projects
            .create(NewProject {
                name: "EFH Muster".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: Some(600.0),
            })
            .await
            .unwrap();

        let update = ProjectUpdate {
            id: p.id,
            expected_version: 1,
            name: "EFH Muster, 2. Entwurf".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: Some(600.0),
        };
        let p2 = repos.projects.update(update.clone()).await.unwrap();
        assert_eq!(p2.version, 2);

        // Zweiter Client mit veralteter Version
        let err = repos.projects.update(update).await.unwrap_err();
        assert!(matches!(
            err,
            CoreError::VersionConflict {
                expected: 1,
                actual: 2
            }
        ));
    }

    #[tokio::test]
    async fn dokumenthistorie() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        let repos = db.repositories();
        let p = repos
            .projects
            .create(NewProject {
                name: "MFH".into(),
                bundesland: Bundesland::Nw,
                grundstuecksflaeche_m2: None,
            })
            .await
            .unwrap();
        let doc = DocumentId::new();
        for inhalt in [b"v1".as_slice(), b"v2"] {
            repos
                .documents
                .add_version(DocumentVersion {
                    document_id: doc,
                    project_id: p.id,
                    version: 0,
                    file_name: "lageplan.pdf".into(),
                    storage_key: format!("k/{}", inhalt.len()),
                    sha256: ContentHash::of(inhalt),
                    size_bytes: inhalt.len() as u64,
                    created_by: "test".into(),
                    created_at: Utc::now(),
                })
                .await
                .unwrap();
        }
        let history = repos.documents.history(doc).await.unwrap();
        assert_eq!(
            history.iter().map(|v| v.version).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            repos.documents.latest(doc).await.unwrap().sha256,
            ContentHash::of(b"v2")
        );
    }
}
