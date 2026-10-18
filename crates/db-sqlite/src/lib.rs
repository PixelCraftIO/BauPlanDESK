//! SQLite-Backend für die Einzelplatz-Version.

mod commit;
mod einreichungen;

use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SubsecRound, Utc};
use core_lib::benutzer::{
    Benutzer, BenutzerAenderung, BenutzerId, Kennwortdaten, Kennwortstatus, NeuerBenutzer, Rolle,
};
use core_lib::domain::{
    Aktion, Bundesland, DocumentId, DocumentLock, DocumentVersion, NeuerProtokolleintrag,
    NewProject, Project, ProjectId, ProjectUpdate, Protokolleintrag,
};
use core_lib::kunden::{
    Beteiligtenrolle, Beteiligtenzuordnung, Kunde, KundeId, Kundendaten, Projektbeteiligter,
};
use core_lib::license::Arbeitsplatzbelegung;
use core_lib::repository::{
    BauantragDatensatz, BauantragRepository, BenutzerRepository, DocumentRepository,
    KundenRepository, LizenzRepository, ProjectRepository, ProtokollRepository, Repositories,
};
use core_lib::storage::ContentHash;
use core_lib::xbau::Anschrift;
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

    /// Konsistente SQLite-Kopie einschließlich bestätigter WAL-Transaktionen.
    /// SQLite selbst erzeugt die Zieldatei; vorhandene Sicherungen werden nicht überschrieben.
    pub async fn sichern(&self, ziel: &Path) -> CoreResult<()> {
        if ziel.exists() {
            return Err(CoreError::InvalidInput(
                "Sicherungsdatenbank existiert bereits".into(),
            ));
        }
        let path = ziel
            .to_str()
            .ok_or_else(|| CoreError::InvalidInput("Sicherungspfad ist nicht UTF-8".into()))?;
        sqlx::query("VACUUM INTO ?")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
    pub async fn datei_pruefen(pfad: &Path) -> CoreResult<()> {
        if !pfad.is_file() {
            return Err(CoreError::NotFound("Profildatenbank".into()));
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(pfad).read_only(true))
            .await
            .map_err(db_err)?;
        let db = Self { pool };
        db.integritaet_pruefen().await?;
        let version: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success=1")
                .fetch_one(&db.pool)
                .await
                .map_err(db_err)?;
        let migrationen = sqlx::migrate!("./migrations");
        if version.is_none() || version.is_some_and(|v| migrationen.iter().all(|m| m.version < v)) {
            return Err(CoreError::InvalidInput(
                "Profil verwendet eine unbekannte Datenbankversion".into(),
            ));
        }
        Ok(())
    }
    pub async fn integritaet_pruefen(&self) -> CoreResult<()> {
        let result: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)?;
        if result != "ok" {
            return Err(CoreError::Database(format!(
                "Datenbankintegrität: {result}"
            )));
        }
        Ok(())
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

    pub async fn schliessen(&self) {
        self.pool.close().await;
    }
    /// Repositories zur Injektion in [`core_lib::CoreServices`].
    pub fn repositories(&self) -> Repositories {
        let this = Arc::new(self.clone());
        Repositories {
            commit: this.clone(),
            einreichungen: this.clone(),
            projects: this.clone(),
            documents: this.clone(),
            benutzer: this.clone(),
            protokoll: this.clone(),
            lizenz: this.clone(),
            bauantraege: this.clone(),
            kunden: this,
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
        planungsrecht: serde_json::from_str(
            &row.try_get::<String, _>("planungsrecht").map_err(db_err)?,
        )
        .map_err(|e| CoreError::Database(e.to_string()))?,
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
            planungsrecht: p.planungsrecht,
            version: 1,
            created_at: now,
            updated_at: now,
        };
        sqlx::query(
            "INSERT INTO projects (id, name, bundesland, grundstuecksflaeche_m2, version, created_at, updated_at, planungsrecht)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(project.id.to_string())
        .bind(&project.name)
        .bind(project.bundesland.code())
        .bind(project.grundstuecksflaeche_m2)
        .bind(project.version)
        .bind(project.created_at)
        .bind(project.updated_at)
        .bind(serde_json::to_string(&project.planungsrecht).map_err(|e| CoreError::Database(e.to_string()))?)
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
                    planungsrecht = COALESCE(?, planungsrecht),
                    version = version + 1, updated_at = ?
              WHERE id = ? AND version = ?
          RETURNING *",
        )
        .bind(&u.name)
        .bind(u.bundesland.code())
        .bind(u.grundstuecksflaeche_m2)
        .bind(
            u.planungsrecht
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| CoreError::Database(e.to_string()))?,
        )
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
        match ProjectRepository::get(self, id).await {
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
    async fn add_version(
        &self,
        mut v: DocumentVersion,
        basis: Option<i64>,
    ) -> CoreResult<DocumentVersion> {
        // Versionsnummer und Insert in einem Statement: kein Rennen zwischen
        // zwei gleichzeitigen Uploads desselben Dokuments. Mit `basis` wird
        // nur eingefügt, wenn `basis` noch die neueste Version ist.
        let size =
            i64::try_from(v.size_bytes).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        let version: Option<i64> = sqlx::query_scalar(
            "INSERT INTO document_versions
                (document_id, project_id, version, file_name, storage_key, sha256, size_bytes, created_by, created_at)
             SELECT ?1, ?2, COALESCE(MAX(version), 0) + 1, ?3, ?4, ?5, ?6, ?7, ?8
               FROM document_versions WHERE document_id = ?1
             HAVING ?9 IS NULL OR COALESCE(MAX(version), 0) = ?9
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
        .bind(basis)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        match (version, basis) {
            (Some(version), _) => {
                v.version = version;
                Ok(v)
            }
            (None, Some(expected)) => Err(CoreError::VersionConflict {
                expected,
                actual: self.latest(v.document_id).await?.version,
            }),
            (None, None) => Err(CoreError::Database(
                "Dokumentversion wurde nicht angelegt".into(),
            )),
        }
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

    async fn lock(
        &self,
        id: DocumentId,
        project: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<DocumentLock> {
        sqlx::query(
            "INSERT INTO document_locks (document_id, project_id, benutzer_id, benutzer_name, seit)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (document_id) DO NOTHING",
        )
        .bind(id.0.to_string())
        .bind(project.to_string())
        .bind(wer.id.to_string())
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
              WHERE document_id = ?1 AND (?2 IS NULL OR benutzer_id = ?2)
          RETURNING *",
        )
        .bind(id.0.to_string())
        .bind(nur_von.map(|b| b.to_string()))
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .as_ref()
        .map(lock_from_row)
        .transpose()
    }

    async fn get_lock(&self, id: DocumentId) -> CoreResult<Option<DocumentLock>> {
        sqlx::query("SELECT * FROM document_locks WHERE document_id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(lock_from_row)
            .transpose()
    }

    async fn locks_for_project(&self, project: ProjectId) -> CoreResult<Vec<DocumentLock>> {
        sqlx::query("SELECT * FROM document_locks WHERE project_id = ? ORDER BY seit")
            .bind(project.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(lock_from_row)
            .collect()
    }
}

fn lock_from_row(row: &SqliteRow) -> CoreResult<DocumentLock> {
    let document_id: String = row.try_get("document_id").map_err(db_err)?;
    let project_id: String = row.try_get("project_id").map_err(db_err)?;
    let benutzer_id: String = row.try_get("benutzer_id").map_err(db_err)?;
    Ok(DocumentLock {
        document_id: DocumentId(parse_uuid(&document_id)?),
        project_id: ProjectId(parse_uuid(&project_id)?),
        benutzer_id: BenutzerId(parse_uuid(&benutzer_id)?),
        benutzer_name: row.try_get("benutzer_name").map_err(db_err)?,
        seit: row.try_get("seit").map_err(db_err)?,
    })
}

fn benutzer_from_row(row: &SqliteRow) -> CoreResult<Benutzer> {
    let id: String = row.try_get("id").map_err(db_err)?;
    let rolle: String = row.try_get("rolle").map_err(db_err)?;
    Ok(Benutzer {
        id: BenutzerId(parse_uuid(&id)?),
        anmeldename: row.try_get("anmeldename").map_err(db_err)?,
        anzeigename: row.try_get("anzeigename").map_err(db_err)?,
        rolle: Rolle::from_str(&rolle)?,
        aktiv: row.try_get("aktiv").map_err(db_err)?,
        erstellt_am: row.try_get("erstellt_am").map_err(db_err)?,
        kennwort: kennwortstatus(
            row.try_get::<Option<String>, _>("kennwort_hash")
                .map_err(db_err)?
                .is_some(),
            row.try_get("kennwort_aendern").map_err(db_err)?,
        ),
        anmeldesperre_bis: row.try_get("gesperrt_bis").map_err(db_err)?,
        letzte_anmeldung: row.try_get("letzte_anmeldung").map_err(db_err)?,
    })
}

fn kennwortstatus(hash_vorhanden: bool, aendern: bool) -> Kennwortstatus {
    match (hash_vorhanden, aendern) {
        (false, _) => Kennwortstatus::Keines,
        (true, true) => Kennwortstatus::Erstkennwort,
        (true, false) => Kennwortstatus::Gesetzt,
    }
}

fn benutzer_fehlt(id: BenutzerId) -> CoreError {
    CoreError::NotFound(format!("Benutzer {id}"))
}

fn protokoll_from_row(row: &SqliteRow) -> CoreResult<Protokolleintrag> {
    let benutzer_id: String = row.try_get("benutzer_id").map_err(db_err)?;
    let aktion: String = row.try_get("aktion").map_err(db_err)?;
    let project_id: Option<String> = row.try_get("project_id").map_err(db_err)?;
    let document_id: Option<String> = row.try_get("document_id").map_err(db_err)?;
    Ok(Protokolleintrag {
        id: row.try_get("id").map_err(db_err)?,
        eintrag: NeuerProtokolleintrag {
            zeit: row.try_get("zeit").map_err(db_err)?,
            benutzer_id: BenutzerId(parse_uuid(&benutzer_id)?),
            benutzer_name: row.try_get("benutzer_name").map_err(db_err)?,
            aktion: Aktion::from_str(&aktion)?,
            project_id: project_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?
                .map(ProjectId),
            document_id: document_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?
                .map(DocumentId),
            details: row.try_get("details").map_err(db_err)?,
        },
    })
}

#[async_trait]
impl BenutzerRepository for SqliteDatabase {
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
            kennwort: Kennwortstatus::Keines,
            anmeldesperre_bis: None,
            letzte_anmeldung: None,
        };
        sqlx::query(
            "INSERT INTO benutzer (id, anmeldename, anzeigename, rolle, aktiv, erstellt_am)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(b.id.to_string())
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
        let row = sqlx::query("SELECT * FROM benutzer WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| CoreError::NotFound(format!("Benutzer {id}")))?;
        benutzer_from_row(&row)
    }

    async fn by_anmeldename(&self, anmeldename: &str) -> CoreResult<Option<Benutzer>> {
        sqlx::query("SELECT * FROM benutzer WHERE anmeldename = ?")
            .bind(anmeldename)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .as_ref()
            .map(benutzer_from_row)
            .transpose()
    }

    async fn list(&self) -> CoreResult<Vec<Benutzer>> {
        sqlx::query("SELECT * FROM benutzer ORDER BY anzeigename COLLATE NOCASE")
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(benutzer_from_row)
            .collect()
    }

    async fn update(&self, a: BenutzerAenderung) -> CoreResult<Benutzer> {
        let row = sqlx::query(
            "UPDATE benutzer SET anzeigename = ?, rolle = ?, aktiv = ? WHERE id = ? RETURNING *",
        )
        .bind(&a.anzeigename)
        .bind(a.rolle.code())
        .bind(a.aktiv)
        .bind(a.id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| CoreError::NotFound(format!("Benutzer {}", a.id)))?;
        benutzer_from_row(&row)
    }

    async fn kennwortdaten(&self, id: BenutzerId) -> CoreResult<Kennwortdaten> {
        let row = sqlx::query(
            "SELECT kennwort_hash, fehlversuche, gesperrt_bis FROM benutzer WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| benutzer_fehlt(id))?;
        let fehlversuche: i64 = row.try_get("fehlversuche").map_err(db_err)?;
        Ok(Kennwortdaten {
            hash: row.try_get("kennwort_hash").map_err(db_err)?,
            fehlversuche: u32::try_from(fehlversuche.max(0)).unwrap_or(u32::MAX),
            gesperrt_bis: row.try_get("gesperrt_bis").map_err(db_err)?,
        })
    }

    async fn kennwort_speichern(
        &self,
        id: BenutzerId,
        hash: &str,
        erstkennwort: bool,
        zeit: DateTime<Utc>,
    ) -> CoreResult<()> {
        let r = sqlx::query(
            "UPDATE benutzer SET kennwort_hash = ?, kennwort_aendern = ?,
                 kennwort_geaendert_am = ?, fehlversuche = 0, gesperrt_bis = NULL
             WHERE id = ?",
        )
        .bind(hash)
        .bind(erstkennwort)
        .bind(zeit.trunc_subsecs(6))
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if r.rows_affected() == 0 {
            return Err(benutzer_fehlt(id));
        }
        Ok(())
    }

    async fn fehlversuch_zaehlen(&self, id: BenutzerId) -> CoreResult<u32> {
        let n: i64 = sqlx::query_scalar(
            "UPDATE benutzer SET fehlversuche = MIN(fehlversuche, 1000000) + 1
             WHERE id = ? RETURNING fehlversuche",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| benutzer_fehlt(id))?;
        Ok(u32::try_from(n.max(0)).unwrap_or(u32::MAX))
    }

    async fn anmeldesperre_setzen(
        &self,
        id: BenutzerId,
        bis: Option<DateTime<Utc>>,
    ) -> CoreResult<()> {
        let r = sqlx::query("UPDATE benutzer SET gesperrt_bis = ? WHERE id = ?")
            .bind(bis.map(|b| b.trunc_subsecs(6)))
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if r.rows_affected() == 0 {
            return Err(benutzer_fehlt(id));
        }
        Ok(())
    }

    async fn anmeldung_vermerken(&self, id: BenutzerId, zeit: DateTime<Utc>) -> CoreResult<()> {
        let r = sqlx::query(
            "UPDATE benutzer SET fehlversuche = 0, gesperrt_bis = NULL, letzte_anmeldung = ?
             WHERE id = ?",
        )
        .bind(zeit.trunc_subsecs(6))
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if r.rows_affected() == 0 {
            return Err(benutzer_fehlt(id));
        }
        Ok(())
    }
}

#[async_trait]
impl ProtokollRepository for SqliteDatabase {
    async fn append(&self, e: NeuerProtokolleintrag) -> CoreResult<()> {
        sqlx::query(
            "INSERT INTO protokoll
                (zeit, benutzer_id, benutzer_name, aktion, project_id, document_id, details)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(e.zeit)
        .bind(e.benutzer_id.to_string())
        .bind(&e.benutzer_name)
        .bind(e.aktion.code())
        .bind(e.project_id.map(|p| p.to_string()))
        .bind(e.document_id.map(|d| d.0.to_string()))
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
        sqlx::query("SELECT * FROM protokoll WHERE project_id = ? ORDER BY id DESC LIMIT ?")
            .bind(project.to_string())
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(protokoll_from_row)
            .collect()
    }
}

fn ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

fn aus_ms(ms: i64) -> CoreResult<DateTime<Utc>> {
    DateTime::from_timestamp_millis(ms)
        .ok_or_else(|| CoreError::Database(format!("ungültiger Zeitstempel: {ms}")))
}

fn belegung_from_row(row: &SqliteRow) -> CoreResult<Arbeitsplatzbelegung> {
    Ok(Arbeitsplatzbelegung {
        arbeitsplatz: row.try_get("arbeitsplatz").map_err(db_err)?,
        benutzer_name: row.try_get("benutzer_name").map_err(db_err)?,
        belegt_seit: aus_ms(row.try_get("belegt_seit").map_err(db_err)?)?,
        letzte_meldung: aus_ms(row.try_get("letzte_meldung").map_err(db_err)?)?,
    })
}

#[async_trait]
impl LizenzRepository for SqliteDatabase {
    async fn einstellung(&self, schluessel: &str) -> CoreResult<Option<String>> {
        sqlx::query_scalar("SELECT wert FROM einstellungen WHERE schluessel = ?")
            .bind(schluessel)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)
    }

    async fn einstellung_setzen(&self, schluessel: &str, wert: &str) -> CoreResult<()> {
        sqlx::query(
            "INSERT INTO einstellungen (schluessel, wert) VALUES (?, ?)
             ON CONFLICT (schluessel) DO UPDATE SET wert = excluded.wert",
        )
        .bind(schluessel)
        .bind(wert)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn einstellung_vergleichen(
        &self,
        schluessel: &str,
        bisher: Option<&str>,
        wert: &str,
    ) -> CoreResult<bool> {
        let count = if let Some(alt) = bisher {
            sqlx::query("UPDATE einstellungen SET wert=? WHERE schluessel=? AND wert=?")
                .bind(wert)
                .bind(schluessel)
                .bind(alt)
                .execute(&self.pool)
                .await
                .map_err(db_err)?
                .rows_affected()
        } else {
            sqlx::query(
                "INSERT INTO einstellungen (schluessel,wert) VALUES (?,?) ON CONFLICT DO NOTHING",
            )
            .bind(schluessel)
            .bind(wert)
            .execute(&self.pool)
            .await
            .map_err(db_err)?
            .rows_affected()
        };
        Ok(count == 1)
    }

    async fn einstellung_vorbelegen(&self, schluessel: &str, wert: &str) -> CoreResult<String> {
        sqlx::query(
            "INSERT INTO einstellungen (schluessel, wert) VALUES (?, ?) ON CONFLICT DO NOTHING",
        )
        .bind(schluessel)
        .bind(wert)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        sqlx::query_scalar("SELECT wert FROM einstellungen WHERE schluessel = ?")
            .bind(schluessel)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    async fn arbeitsplatz_belegen(
        &self,
        b: Arbeitsplatzbelegung,
        max: u32,
        abgelaufen_vor: DateTime<Utc>,
    ) -> CoreResult<bool> {
        // Das erste DELETE nimmt die Schreibsperre der Datenbank; ab da
        // kann kein anderer Rechner dazwischen zählen und einfügen.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("DELETE FROM arbeitsplaetze WHERE letzte_meldung < ?")
            .bind(ms(abgelaufen_vor))
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let aktualisiert = sqlx::query(
            "UPDATE arbeitsplaetze SET letzte_meldung = ?, benutzer_name = ? WHERE arbeitsplatz = ?",
        )
        .bind(ms(b.letzte_meldung))
        .bind(&b.benutzer_name)
        .bind(&b.arbeitsplatz)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?
        .rows_affected();
        let erhalten = if aktualisiert > 0 {
            true
        } else {
            let belegt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM arbeitsplaetze")
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;
            if belegt < i64::from(max) {
                sqlx::query(
                    "INSERT INTO arbeitsplaetze (arbeitsplatz, benutzer_name, belegt_seit, letzte_meldung)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(&b.arbeitsplatz)
                .bind(&b.benutzer_name)
                .bind(ms(b.belegt_seit))
                .bind(ms(b.letzte_meldung))
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                true
            } else {
                false
            }
        };
        tx.commit().await.map_err(db_err)?;
        Ok(erhalten)
    }

    async fn arbeitsplatz_freigeben(&self, arbeitsplatz: &str) -> CoreResult<()> {
        sqlx::query("DELETE FROM arbeitsplaetze WHERE arbeitsplatz = ?")
            .bind(arbeitsplatz)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn arbeitsplaetze(
        &self,
        abgelaufen_vor: DateTime<Utc>,
    ) -> CoreResult<Vec<Arbeitsplatzbelegung>> {
        sqlx::query(
            "SELECT * FROM arbeitsplaetze WHERE letzte_meldung >= ? ORDER BY belegt_seit, arbeitsplatz",
        )
        .bind(ms(abgelaufen_vor))
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(belegung_from_row)
        .collect()
    }
}

#[async_trait]
impl BauantragRepository for SqliteDatabase {
    async fn bauantrag(&self, project: ProjectId) -> CoreResult<Option<BauantragDatensatz>> {
        let row = sqlx::query(
            "SELECT daten AS daten, version, geaendert_von, geaendert_am
               FROM bauantraege WHERE project_id = ?",
        )
        .bind(project.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.map(|row| {
            Ok(BauantragDatensatz {
                daten: row.try_get("daten").map_err(db_err)?,
                version: row.try_get("version").map_err(db_err)?,
                geaendert_von: row.try_get("geaendert_von").map_err(db_err)?,
                geaendert_am: row.try_get("geaendert_am").map_err(db_err)?,
            })
        })
        .transpose()
    }

    async fn bauantrag_speichern(
        &self,
        project: ProjectId,
        daten: &str,
        expected_version: i64,
        geaendert_von: &str,
    ) -> CoreResult<i64> {
        let neu: Option<i64> = if expected_version == 0 {
            sqlx::query_scalar(
                "INSERT INTO bauantraege (project_id, daten, version, geaendert_von, geaendert_am)
                 VALUES (?, ?, 1, ?, ?)
                 ON CONFLICT (project_id) DO NOTHING
                 RETURNING version",
            )
            .bind(project.to_string())
            .bind(daten)
            .bind(geaendert_von)
            .bind(Utc::now())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
        } else {
            sqlx::query_scalar(
                "UPDATE bauantraege
                    SET daten = ?, version = version + 1, geaendert_von = ?, geaendert_am = ?
                  WHERE project_id = ? AND version = ?
              RETURNING version",
            )
            .bind(daten)
            .bind(geaendert_von)
            .bind(Utc::now())
            .bind(project.to_string())
            .bind(expected_version)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
        };
        match neu {
            Some(v) => Ok(v),
            None => Err(CoreError::VersionConflict {
                expected: expected_version,
                actual: self.bauantrag(project).await?.map_or(0, |b| b.version),
            }),
        }
    }
}

fn kunde_from_row(row: &SqliteRow) -> CoreResult<Kunde> {
    let id: String = row.try_get("id").map_err(db_err)?;
    let typ: String = row.try_get("typ").map_err(db_err)?;
    let kategorie: String = row.try_get("kategorie").map_err(db_err)?;
    let text = |spalte: &str| row.try_get::<String, _>(spalte).map_err(db_err);
    Ok(Kunde {
        id: KundeId(parse_uuid(&id)?),
        daten: Kundendaten {
            typ: typ.parse()?,
            kategorie: kategorie.parse()?,
            kundennummer: text("kundennummer")?,
            vorname: text("vorname")?,
            familienname: text("familienname")?,
            firma: text("firma")?,
            ansprechpartner: text("ansprechpartner")?,
            anschrift: Anschrift {
                strasse: text("strasse")?,
                hausnummer: text("hausnummer")?,
                postleitzahl: text("postleitzahl")?,
                ort: text("ort")?,
            },
            telefon: text("telefon")?,
            email: text("email")?,
            notizen: text("notizen")?,
        },
        version: row.try_get("version").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
        updated_at: row.try_get("updated_at").map_err(db_err)?,
    })
}

fn ist_fremdschluessel(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.is_foreign_key_violation())
}

#[async_trait]
impl KundenRepository for SqliteDatabase {
    async fn kunde_anlegen(&self, d: Kundendaten) -> CoreResult<Kunde> {
        let jetzt = Utc::now();
        let row = sqlx::query(
            "INSERT INTO kunden (id, typ, kategorie, kundennummer, vorname, familienname, firma,
                                 ansprechpartner, strasse, hausnummer, postleitzahl, ort,
                                 telefon, email, notizen, version, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
          RETURNING *",
        )
        .bind(KundeId::new().to_string())
        .bind(d.typ.code())
        .bind(d.kategorie.code())
        .bind(&d.kundennummer)
        .bind(&d.vorname)
        .bind(&d.familienname)
        .bind(&d.firma)
        .bind(&d.ansprechpartner)
        .bind(&d.anschrift.strasse)
        .bind(&d.anschrift.hausnummer)
        .bind(&d.anschrift.postleitzahl)
        .bind(&d.anschrift.ort)
        .bind(&d.telefon)
        .bind(&d.email)
        .bind(&d.notizen)
        .bind(jetzt)
        .bind(jetzt)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        kunde_from_row(&row)
    }

    async fn kunde(&self, id: KundeId) -> CoreResult<Kunde> {
        let row = sqlx::query("SELECT * FROM kunden WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?
            .ok_or_else(|| CoreError::NotFound(format!("Kunde {id}")))?;
        kunde_from_row(&row)
    }

    async fn kunden(&self) -> CoreResult<Vec<Kunde>> {
        sqlx::query("SELECT * FROM kunden")
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?
            .iter()
            .map(kunde_from_row)
            .collect()
    }

    async fn kunde_aendern(
        &self,
        id: KundeId,
        d: Kundendaten,
        expected_version: i64,
    ) -> CoreResult<Kunde> {
        let row = sqlx::query(
            "UPDATE kunden
                SET typ = ?, kategorie = ?, kundennummer = ?, vorname = ?, familienname = ?,
                    firma = ?, ansprechpartner = ?, strasse = ?, hausnummer = ?,
                    postleitzahl = ?, ort = ?, telefon = ?, email = ?, notizen = ?,
                    version = version + 1, updated_at = ?
              WHERE id = ? AND version = ?
          RETURNING *",
        )
        .bind(d.typ.code())
        .bind(d.kategorie.code())
        .bind(&d.kundennummer)
        .bind(&d.vorname)
        .bind(&d.familienname)
        .bind(&d.firma)
        .bind(&d.ansprechpartner)
        .bind(&d.anschrift.strasse)
        .bind(&d.anschrift.hausnummer)
        .bind(&d.anschrift.postleitzahl)
        .bind(&d.anschrift.ort)
        .bind(&d.telefon)
        .bind(&d.email)
        .bind(&d.notizen)
        .bind(Utc::now())
        .bind(id.to_string())
        .bind(expected_version)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        match row {
            Some(row) => kunde_from_row(&row),
            None => Err(self.kunde_konflikt(id, expected_version).await),
        }
    }

    async fn kunde_loeschen(&self, id: KundeId, expected_version: i64) -> CoreResult<()> {
        let result = sqlx::query("DELETE FROM kunden WHERE id = ? AND version = ?")
            .bind(id.to_string())
            .bind(expected_version)
            .execute(&self.pool)
            .await;
        match result {
            Ok(r) if r.rows_affected() == 1 => Ok(()),
            Ok(_) => Err(self.kunde_konflikt(id, expected_version).await),
            Err(e) if ist_fremdschluessel(&e) => Err(CoreError::InvalidInput(
                "Der Kunde ist noch einem Projekt als Bauherr oder Beteiligter zugeordnet".into(),
            )),
            Err(e) => Err(db_err(e)),
        }
    }

    async fn bauherren(&self, project: ProjectId) -> CoreResult<Vec<Kunde>> {
        sqlx::query(
            "SELECT k.* FROM projekt_bauherren b JOIN kunden k ON k.id = b.kunde_id
              WHERE b.project_id = ? ORDER BY b.position",
        )
        .bind(project.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(kunde_from_row)
        .collect()
    }

    async fn bauherren_setzen(
        &self,
        project: ProjectId,
        kunden: &[KundeId],
        expected_version: i64,
    ) -> CoreResult<Project> {
        // Das UPDATE nimmt die Schreibsperre; die Projektversion schützt die
        // Zuordnung wie die übrigen Projektdaten.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = sqlx::query(
            "UPDATE projects SET version = version + 1, updated_at = ?
              WHERE id = ? AND version = ?
          RETURNING *",
        )
        .bind(Utc::now())
        .bind(project.to_string())
        .bind(expected_version)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let Some(row) = row else {
            tx.rollback().await.map_err(db_err)?;
            return Err(self.conflict_or_missing(project, expected_version).await);
        };
        sqlx::query("DELETE FROM projekt_bauherren WHERE project_id = ?")
            .bind(project.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for (position, kunde) in kunden.iter().enumerate() {
            sqlx::query(
                "INSERT INTO projekt_bauherren (project_id, kunde_id, position) VALUES (?, ?, ?)",
            )
            .bind(project.to_string())
            .bind(kunde.to_string())
            .bind(position as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if ist_fremdschluessel(&e) {
                    CoreError::NotFound(format!("Kunde {kunde}"))
                } else {
                    db_err(e)
                }
            })?;
        }
        tx.commit().await.map_err(db_err)?;
        project_from_row(&row)
    }

    async fn beteiligte(&self, project: ProjectId) -> CoreResult<Vec<Projektbeteiligter>> {
        let mut liste = sqlx::query(
            "SELECT b.rolle, b.position, k.* FROM projekt_beteiligte b JOIN kunden k ON k.id = b.kunde_id
              WHERE b.project_id = ?",
        )
        .bind(project.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            let rolle: String = row.try_get("rolle").map_err(db_err)?;
            let position: i64 = row.try_get("position").map_err(db_err)?;
            Ok((rolle.parse::<Beteiligtenrolle>()?, position, kunde_from_row(row)?))
        })
        .collect::<CoreResult<Vec<_>>>()?;
        liste.sort_by_key(|(rolle, position, _)| (*rolle, *position));
        Ok(liste
            .into_iter()
            .map(|(rolle, _, kunde)| Projektbeteiligter { rolle, kunde })
            .collect())
    }

    async fn beteiligte_setzen(
        &self,
        project: ProjectId,
        beteiligte: &[Beteiligtenzuordnung],
        expected_version: i64,
    ) -> CoreResult<Project> {
        // Wie bei den Bauherren schützt die Projektversion die Zuordnung.
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let row = sqlx::query(
            "UPDATE projects SET version = version + 1, updated_at = ?
              WHERE id = ? AND version = ?
          RETURNING *",
        )
        .bind(Utc::now())
        .bind(project.to_string())
        .bind(expected_version)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let Some(row) = row else {
            tx.rollback().await.map_err(db_err)?;
            return Err(self.conflict_or_missing(project, expected_version).await);
        };
        sqlx::query("DELETE FROM projekt_beteiligte WHERE project_id = ?")
            .bind(project.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for (position, z) in beteiligte.iter().enumerate() {
            sqlx::query(
                "INSERT INTO projekt_beteiligte (project_id, kunde_id, rolle, position) VALUES (?, ?, ?, ?)",
            )
            .bind(project.to_string())
            .bind(z.kunde.to_string())
            .bind(z.rolle.code())
            .bind(position as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if ist_fremdschluessel(&e) {
                    CoreError::NotFound(format!("Kunde {}", z.kunde))
                } else {
                    db_err(e)
                }
            })?;
        }
        tx.commit().await.map_err(db_err)?;
        project_from_row(&row)
    }

    async fn projekte_des_kunden(&self, id: KundeId) -> CoreResult<Vec<Project>> {
        sqlx::query(
            "SELECT p.* FROM projects p
              WHERE p.id IN (SELECT project_id FROM projekt_bauherren WHERE kunde_id = ?
                             UNION SELECT project_id FROM projekt_beteiligte WHERE kunde_id = ?)
              ORDER BY p.updated_at DESC",
        )
        .bind(id.to_string())
        .bind(id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(project_from_row)
        .collect()
    }
}

impl SqliteDatabase {
    async fn kunde_konflikt(&self, id: KundeId, expected: i64) -> CoreError {
        match KundenRepository::kunde(self, id).await {
            Ok(k) => CoreError::VersionConflict {
                expected,
                actual: k.version,
            },
            Err(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_bewahrt_bestehende_projekte() {
        use std::borrow::Cow;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let alle = sqlx::migrate!("./migrations");
        let alt = sqlx::migrate::Migrator {
            migrations: Cow::Owned(alle.iter().filter(|m| m.version < 6).cloned().collect()),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        alt.run(&pool).await.unwrap();
        let id = ProjectId::new();
        sqlx::query("INSERT INTO projects (id, name, bundesland, grundstuecksflaeche_m2, created_at, updated_at) VALUES (?, 'Bestandsprojekt', 'BY', 800, ?, ?)")
            .bind(id.to_string()).bind(Utc::now()).bind(Utc::now()).execute(&pool).await.unwrap();
        alle.run(&pool).await.unwrap();
        let db = SqliteDatabase { pool };
        let p = ProjectRepository::get(&db, id).await.unwrap();
        assert_eq!(p.name, "Bestandsprojekt");
        assert_eq!(p.grundstuecksflaeche_m2, Some(800.0));
        assert_eq!(p.version, 1);
        assert_eq!(p.planungsrecht, Default::default());
    }

    #[tokio::test]
    async fn konformitaet() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        core_lib::testsuite::repositories(db.repositories()).await;
    }
}
