//! PostgreSQL-Backend für die NAS-/Büro- und Cloud-Variante.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use core_lib::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use core_lib::repository::{DocumentRepository, ProjectRepository, Repositories};
use core_lib::storage::ContentHash;
use core_lib::{CoreError, CoreResult};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    /// Verbindet sich mit `postgres://…` und führt Migrationen aus.
    pub async fn connect(url: &str, max_connections: u32) -> CoreResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await
            .map_err(db_err)?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| CoreError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    pub fn repositories(&self) -> Repositories {
        let this = Arc::new(self.clone());
        Repositories {
            projects: this.clone(),
            documents: this,
        }
    }

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

fn db_err(e: sqlx::Error) -> CoreError {
    CoreError::Database(e.to_string())
}

fn project_from_row(row: &PgRow) -> CoreResult<Project> {
    let land: String = row.try_get("bundesland").map_err(db_err)?;
    Ok(Project {
        id: ProjectId(row.try_get("id").map_err(db_err)?),
        name: row.try_get("name").map_err(db_err)?,
        bundesland: Bundesland::from_str(&land)?,
        grundstuecksflaeche_m2: row.try_get("grundstuecksflaeche_m2").map_err(db_err)?,
        version: row.try_get("version").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
        updated_at: row.try_get("updated_at").map_err(db_err)?,
    })
}

fn document_from_row(row: &PgRow) -> CoreResult<DocumentVersion> {
    let sha256: String = row.try_get("sha256").map_err(db_err)?;
    let size: i64 = row.try_get("size_bytes").map_err(db_err)?;
    Ok(DocumentVersion {
        document_id: DocumentId(row.try_get("document_id").map_err(db_err)?),
        project_id: ProjectId(row.try_get("project_id").map_err(db_err)?),
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
impl ProjectRepository for PostgresDatabase {
    async fn create(&self, p: NewProject) -> CoreResult<Project> {
        let now = Utc::now();
        let row = sqlx::query(
            "INSERT INTO projects (id, name, bundesland, grundstuecksflaeche_m2, version, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 1, $5, $5)
          RETURNING *",
        )
        .bind(ProjectId::new().0)
        .bind(&p.name)
        .bind(p.bundesland.code())
        .bind(p.grundstuecksflaeche_m2)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        project_from_row(&row)
    }

    async fn get(&self, id: ProjectId) -> CoreResult<Project> {
        let row = sqlx::query("SELECT * FROM projects WHERE id = $1")
            .bind(id.0)
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
                SET name = $1, bundesland = $2, grundstuecksflaeche_m2 = $3,
                    version = version + 1, updated_at = now()
              WHERE id = $4 AND version = $5
          RETURNING *",
        )
        .bind(&u.name)
        .bind(u.bundesland.code())
        .bind(u.grundstuecksflaeche_m2)
        .bind(u.id.0)
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
        let result = sqlx::query("DELETE FROM projects WHERE id = $1 AND version = $2")
            .bind(id.0)
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

#[async_trait]
impl DocumentRepository for PostgresDatabase {
    async fn add_version(&self, mut v: DocumentVersion) -> CoreResult<DocumentVersion> {
        let size =
            i64::try_from(v.size_bytes).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        // Zwei gleichzeitige Uploads können dieselbe Versionsnummer berechnen;
        // der Primärschlüssel verhindert das Duplikat, dann erneut versuchen.
        for _ in 0..3 {
            let result = sqlx::query_scalar::<_, i64>(
                "INSERT INTO document_versions
                    (document_id, project_id, version, file_name, storage_key, sha256, size_bytes, created_by, created_at)
                 SELECT $1, $2, COALESCE(MAX(version), 0) + 1, $3, $4, $5, $6, $7, $8
                   FROM document_versions WHERE document_id = $1
              RETURNING version",
            )
            .bind(v.document_id.0)
            .bind(v.project_id.0)
            .bind(&v.file_name)
            .bind(&v.storage_key)
            .bind(v.sha256.as_str())
            .bind(size)
            .bind(&v.created_by)
            .bind(v.created_at)
            .fetch_one(&self.pool)
            .await;
            match result {
                Ok(version) => {
                    v.version = version;
                    return Ok(v);
                }
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => continue,
                Err(e) => return Err(db_err(e)),
            }
        }
        Err(CoreError::Database(
            "Dokumentversion konnte wegen gleichzeitiger Änderungen nicht angelegt werden".into(),
        ))
    }

    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion> {
        let row = sqlx::query(
            "SELECT * FROM document_versions WHERE document_id = $1 ORDER BY version DESC LIMIT 1",
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| CoreError::NotFound(format!("Dokument {}", id.0)))?;
        document_from_row(&row)
    }

    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>> {
        sqlx::query("SELECT * FROM document_versions WHERE document_id = $1 ORDER BY version")
            .bind(id.0)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(document_from_row)
            .collect()
    }

    async fn list_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        sqlx::query(
            "SELECT * FROM document_versions WHERE project_id = $1 ORDER BY document_id, version",
        )
        .bind(project.0)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(document_from_row)
        .collect()
    }
}
