//! PostgreSQL-Backend für die NAS-/Büro- und Cloud-Variante.

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use core_lib::benutzer::{Benutzer, BenutzerAenderung, BenutzerId, NeuerBenutzer, Rolle};
use core_lib::domain::{
    Aktion, Bundesland, DocumentId, DocumentLock, DocumentVersion, NeuerProtokolleintrag,
    NewProject, Project, ProjectId, ProjectUpdate, Protokolleintrag,
};
use core_lib::repository::{
    BenutzerRepository, DocumentRepository, ProjectRepository, ProtokollRepository, Repositories,
};
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
            documents: this.clone(),
            benutzer: this.clone(),
            protokoll: this,
        }
    }

    async fn conflict_or_missing(&self, id: ProjectId, expected: i64) -> CoreError {
        match ProjectRepository::get(self, id).await {
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
    async fn add_version(
        &self,
        mut v: DocumentVersion,
        basis: Option<i64>,
    ) -> CoreResult<DocumentVersion> {
        let size =
            i64::try_from(v.size_bytes).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        // Zwei gleichzeitige Uploads können dieselbe Versionsnummer berechnen;
        // der Primärschlüssel verhindert das Duplikat. Ohne `basis` erneut
        // versuchen, mit `basis` ist das ein Versionskonflikt.
        for _ in 0..3 {
            let result = sqlx::query_scalar::<_, i64>(
                "INSERT INTO document_versions
                    (document_id, project_id, version, file_name, storage_key, sha256, size_bytes, created_by, created_at)
                 SELECT $1, $2, COALESCE(MAX(version), 0) + 1, $3, $4, $5, $6, $7, $8
                   FROM document_versions WHERE document_id = $1
                 HAVING $9::BIGINT IS NULL OR COALESCE(MAX(version), 0) = $9
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
            .bind(basis)
            .fetch_optional(&self.pool)
            .await;
            match (result, basis) {
                (Ok(Some(version)), _) => {
                    v.version = version;
                    return Ok(v);
                }
                (Ok(None), Some(expected)) => {
                    return Err(CoreError::VersionConflict {
                        expected,
                        actual: self.latest(v.document_id).await?.version,
                    });
                }
                (Err(sqlx::Error::Database(e)), Some(expected)) if e.is_unique_violation() => {
                    return Err(CoreError::VersionConflict {
                        expected,
                        actual: self.latest(v.document_id).await?.version,
                    });
                }
                (Err(sqlx::Error::Database(e)), None) if e.is_unique_violation() => continue,
                (Ok(None), None) => {
                    return Err(CoreError::Database(
                        "Dokumentversion wurde nicht angelegt".into(),
                    ));
                }
                (Err(e), _) => return Err(db_err(e)),
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

    async fn lock(
        &self,
        id: DocumentId,
        project: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<DocumentLock> {
        sqlx::query(
            "INSERT INTO document_locks (document_id, project_id, benutzer_id, benutzer_name, seit)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (document_id) DO NOTHING",
        )
        .bind(id.0)
        .bind(project.0)
        .bind(wer.id.0)
        .bind(&wer.anzeigename)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        let sperre = self
            .get_lock(id)
            .await?
            .ok_or_else(|| CoreError::Database("Sperre wurde nicht angelegt".into()))?;
        if sperre.benutzer_id != wer.id {
            return Err(CoreError::Gesperrt {
                von: sperre.benutzer_name,
                seit: sperre.seit,
            });
        }
        Ok(sperre)
    }

    async fn unlock(
        &self,
        id: DocumentId,
        nur_von: Option<BenutzerId>,
    ) -> CoreResult<Option<DocumentLock>> {
        sqlx::query(
            "DELETE FROM document_locks
              WHERE document_id = $1 AND ($2::UUID IS NULL OR benutzer_id = $2)
          RETURNING *",
        )
        .bind(id.0)
        .bind(nur_von.map(|b| b.0))
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .as_ref()
        .map(lock_from_row)
        .transpose()
    }

    async fn get_lock(&self, id: DocumentId) -> CoreResult<Option<DocumentLock>> {
        sqlx::query("SELECT * FROM document_locks WHERE document_id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(lock_from_row)
            .transpose()
    }

    async fn locks_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentLock>> {
        sqlx::query("SELECT * FROM document_locks WHERE project_id = $1 ORDER BY seit")
            .bind(project.0)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(lock_from_row)
            .collect()
    }
}

fn lock_from_row(row: &PgRow) -> CoreResult<DocumentLock> {
    Ok(DocumentLock {
        document_id: DocumentId(row.try_get("document_id").map_err(db_err)?),
        project_id: ProjectId(row.try_get("project_id").map_err(db_err)?),
        benutzer_id: BenutzerId(row.try_get("benutzer_id").map_err(db_err)?),
        benutzer_name: row.try_get("benutzer_name").map_err(db_err)?,
        seit: row.try_get("seit").map_err(db_err)?,
    })
}

fn benutzer_from_row(row: &PgRow) -> CoreResult<Benutzer> {
    let rolle: String = row.try_get("rolle").map_err(db_err)?;
    Ok(Benutzer {
        id: BenutzerId(row.try_get("id").map_err(db_err)?),
        anmeldename: row.try_get("anmeldename").map_err(db_err)?,
        anzeigename: row.try_get("anzeigename").map_err(db_err)?,
        rolle: Rolle::from_str(&rolle)?,
        aktiv: row.try_get("aktiv").map_err(db_err)?,
        erstellt_am: row.try_get("erstellt_am").map_err(db_err)?,
    })
}

fn protokoll_from_row(row: &PgRow) -> CoreResult<Protokolleintrag> {
    let aktion: String = row.try_get("aktion").map_err(db_err)?;
    Ok(Protokolleintrag {
        id: row.try_get("id").map_err(db_err)?,
        eintrag: NeuerProtokolleintrag {
            zeit: row.try_get("zeit").map_err(db_err)?,
            benutzer_id: BenutzerId(row.try_get("benutzer_id").map_err(db_err)?),
            benutzer_name: row.try_get("benutzer_name").map_err(db_err)?,
            aktion: Aktion::from_str(&aktion)?,
            project_id: row
                .try_get::<Option<uuid::Uuid>, _>("project_id")
                .map_err(db_err)?
                .map(ProjectId),
            document_id: row
                .try_get::<Option<uuid::Uuid>, _>("document_id")
                .map_err(db_err)?
                .map(DocumentId),
            details: row.try_get("details").map_err(db_err)?,
        },
    })
}

#[async_trait]
impl BenutzerRepository for PostgresDatabase {
    async fn create(&self, neu: NeuerBenutzer) -> CoreResult<Benutzer> {
        let b = Benutzer {
            id: BenutzerId::new(),
            anmeldename: neu.anmeldename,
            anzeigename: neu.anzeigename,
            rolle: neu.rolle,
            aktiv: true,
            // Postgres speichert Mikrosekunden; so ist der Rückgabewert
            // gleich dem, was später gelesen wird.
            erstellt_am: Utc::now().trunc_subsecs(6),
        };
        sqlx::query(
            "INSERT INTO benutzer (id, anmeldename, anzeigename, rolle, aktiv, erstellt_am)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(b.id.0)
        .bind(&b.anmeldename)
        .bind(&b.anzeigename)
        .bind(b.rolle.code())
        .bind(b.aktiv)
        .bind(b.erstellt_am)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => CoreError::InvalidInput(
                format!("Den Anmeldenamen „{}“ gibt es bereits", b.anmeldename),
            ),
            e => db_err(e),
        })?;
        Ok(b)
    }

    async fn get(&self, id: BenutzerId) -> CoreResult<Benutzer> {
        let row = sqlx::query("SELECT * FROM benutzer WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| CoreError::NotFound(format!("Benutzer {id}")))?;
        benutzer_from_row(&row)
    }

    async fn by_anmeldename(&self, anmeldename: &str) -> CoreResult<Option<Benutzer>> {
        sqlx::query("SELECT * FROM benutzer WHERE anmeldename = $1")
            .bind(anmeldename)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(benutzer_from_row)
            .transpose()
    }

    async fn list(&self) -> CoreResult<Vec<Benutzer>> {
        sqlx::query("SELECT * FROM benutzer ORDER BY lower(anzeigename)")
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(benutzer_from_row)
            .collect()
    }

    async fn update(&self, a: BenutzerAenderung) -> CoreResult<Benutzer> {
        let row = sqlx::query(
            "UPDATE benutzer SET anzeigename = $1, rolle = $2, aktiv = $3 WHERE id = $4 RETURNING *",
        )
        .bind(&a.anzeigename)
        .bind(a.rolle.code())
        .bind(a.aktiv)
        .bind(a.id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| CoreError::NotFound(format!("Benutzer {}", a.id)))?;
        benutzer_from_row(&row)
    }
}

#[async_trait]
impl ProtokollRepository for PostgresDatabase {
    async fn append(&self, e: NeuerProtokolleintrag) -> CoreResult<()> {
        sqlx::query(
            "INSERT INTO protokoll
                (zeit, benutzer_id, benutzer_name, aktion, project_id, document_id, details)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(e.zeit)
        .bind(e.benutzer_id.0)
        .bind(&e.benutzer_name)
        .bind(e.aktion.code())
        .bind(e.project_id.map(|p| p.0))
        .bind(e.document_id.map(|d| d.0))
        .bind(&e.details)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn list_for_project(
        &self,
        project: ProjectId,
        limit: i64,
    ) -> CoreResult<Vec<Protokolleintrag>> {
        sqlx::query("SELECT * FROM protokoll WHERE project_id = $1 ORDER BY id DESC LIMIT $2")
            .bind(project.0)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(protokoll_from_row)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Läuft nur mit `BAUPROJEKT_TEST_POSTGRES=postgres://…` (leere Datenbank).
    #[tokio::test]
    async fn konformitaet() {
        let Ok(url) = std::env::var("BAUPROJEKT_TEST_POSTGRES") else {
            eprintln!("BAUPROJEKT_TEST_POSTGRES nicht gesetzt, Test übersprungen");
            return;
        };
        let db = PostgresDatabase::connect(&url, 4).await.unwrap();
        core_lib::testsuite::repositories(db.repositories()).await;
    }
}
