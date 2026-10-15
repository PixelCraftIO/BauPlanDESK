//! Fachliche Commits und Audit in derselben Datenbanktransaktion.
use super::*;
use core_lib::{
    commit::{CommitRepository, DokumentCommit, DokumentZusatz, Fachstand},
    einreichung::Einreichung,
};
use sqlx::PgConnection;
use uuid::Uuid;
fn uid(id: Uuid) -> Uuid {
    id
}
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::Database(e.to_string())
}
async fn audit(c: &mut PgConnection, e: NeuerProtokolleintrag) -> CoreResult<()> {
    sqlx::query("INSERT INTO protokoll (zeit,benutzer_id,benutzer_name,aktion,project_id,document_id,details) VALUES ($1,$2,$3,$4,$5,$6,$7)").bind(e.zeit).bind(uid(e.benutzer_id.0)).bind(&e.benutzer_name).bind(e.aktion.code()).bind(e.project_id.map(|p|uid(p.0))).bind(e.document_id.map(|d|uid(d.0))).bind(&e.details).execute(c).await.map_err(db_err)?;
    Ok(())
}
#[async_trait]
impl CommitRepository for PostgresDatabase {
    async fn projekt(
        &self,
        mut p: Project,
        basis: Option<i64>,
        e: NeuerProtokolleintrag,
    ) -> CoreResult<Project> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        p.version = basis.map(|b| b + 1).unwrap_or(1);
        let payload = serde_json::to_string(&p.planungsrecht).map_err(json_err)?;
        let count = if let Some(b) = basis {
            sqlx::query("UPDATE projects SET name=$1,bundesland=$2,grundstuecksflaeche_m2=$3,version=$4,updated_at=$5,planungsrecht=$6 WHERE id=$7 AND version=$8").bind(&p.name).bind(p.bundesland.code()).bind(p.grundstuecksflaeche_m2).bind(p.version).bind(p.updated_at).bind(&payload).bind(uid(p.id.0)).bind(b).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        } else {
            sqlx::query("INSERT INTO projects (id,name,bundesland,grundstuecksflaeche_m2,version,created_at,updated_at,planungsrecht) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)").bind(uid(p.id.0)).bind(&p.name).bind(p.bundesland.code()).bind(p.grundstuecksflaeche_m2).bind(p.version).bind(p.created_at).bind(p.updated_at).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count != 1 {
            let actual: Option<i64> =
                sqlx::query_scalar("SELECT version FROM projects WHERE id=$1")
                    .bind(uid(p.id.0))
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
            return Err(match actual {
                Some(actual) => CoreError::VersionConflict {
                    expected: basis.unwrap_or(0),
                    actual,
                },
                None => CoreError::NotFound(format!("Projekt {}", p.id)),
            });
        }
        if basis.is_none() {
            let mitglied = serde_json::to_string(&core_lib::zugriff::ProjektMitglied {
                benutzer_id: e.benutzer_id,
                recht: core_lib::zugriff::ProjektRecht::Leitung,
            })
            .map_err(json_err)?;
            sqlx::query("INSERT INTO fachstaende (project_id,modul,objekt,schema_version,revision,payload) VALUES ($1,'mitgliedschaften',$2,1,1,$3)").bind(uid(p.id.0)).bind(e.benutzer_id.to_string()).bind(mitglied).execute(&mut *tx).await.map_err(db_err)?;
            sqlx::query("INSERT INTO fachhistorie SELECT project_id,modul,objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=$1 AND modul='mitgliedschaften'").bind(uid(p.id.0)).execute(&mut *tx).await.map_err(db_err)?;
        }
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(p)
    }
    async fn dokument(
        &self,
        auftrag: DokumentCommit,
        e: NeuerProtokolleintrag,
    ) -> CoreResult<DocumentVersion> {
        let DokumentCommit {
            dokument: mut d,
            basis,
            neuanlage,
            zusaetze,
        } = auftrag;
        // PostgreSQL timestamps have microsecond precision. Return the persisted representation also on first creation.
        d.created_at = chrono::DateTime::from_timestamp_micros(d.created_at.timestamp_micros())
            .ok_or_else(|| {
                CoreError::InvalidInput("Dokumentzeit außerhalb des Wertebereichs".into())
            })?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(d.document_id.0.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let latest: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM document_versions WHERE document_id=$1")
                .bind(uid(d.document_id.0))
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;
        if neuanlage && latest.is_some() {
            let r=sqlx::query("SELECT * FROM document_versions WHERE document_id=$1 ORDER BY version DESC LIMIT 1").bind(uid(d.document_id.0)).fetch_one(&mut *tx).await.map_err(db_err)?;
            let alt = document_from_row(&r)?;
            if alt.project_id == d.project_id
                && alt.sha256 == d.sha256
                && alt.file_name == d.file_name
            {
                return Ok(alt);
            }
            return Err(CoreError::InvalidInput(
                "Vorgang wurde bereits mit einem anderen Dateiinhalt gespeichert".into(),
            ));
        }
        if let Some(expected) = basis
            && latest != Some(expected)
        {
            return Err(CoreError::VersionConflict {
                expected,
                actual: latest.unwrap_or(0),
            });
        }
        d.version = latest.unwrap_or(0) + 1;
        sqlx::query("INSERT INTO document_versions (document_id,project_id,version,file_name,storage_key,sha256,size_bytes,created_by,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(uid(d.document_id.0)).bind(uid(d.project_id.0)).bind(d.version).bind(&d.file_name).bind(&d.storage_key).bind(d.sha256.as_str()).bind(i64::try_from(d.size_bytes).map_err(|e|CoreError::InvalidInput(e.to_string()))?).bind(&d.created_by).bind(d.created_at).execute(&mut *tx).await.map_err(db_err)?;
        for z in zusaetze {
            sqlx::query("INSERT INTO dokumentkontexte (document_id,version,modul,schema_version,payload) VALUES ($1,$2,$3,$4,$5)").bind(uid(d.document_id.0)).bind(d.version).bind(z.modul).bind(i64::from(z.schema_version)).bind(serde_json::to_string(&z.daten).map_err(json_err)?).execute(&mut *tx).await.map_err(db_err)?;
        }
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(d)
    }
    async fn dokumentzusatz(
        &self,
        id: DocumentId,
        version: i64,
        modul: &str,
    ) -> CoreResult<Option<DokumentZusatz>> {
        let r=sqlx::query("SELECT schema_version,payload FROM dokumentkontexte WHERE document_id=$1 AND version=$2 AND modul=$3").bind(uid(id.0)).bind(version).bind(modul).fetch_optional(&self.pool).await.map_err(db_err)?;
        r.map(|r| {
            Ok(DokumentZusatz {
                modul: modul.into(),
                schema_version: u32::try_from(
                    r.try_get::<i64, _>("schema_version").map_err(db_err)?,
                )
                .map_err(|e| CoreError::Database(e.to_string()))?,
                daten: serde_json::from_str(&r.try_get::<String, _>("payload").map_err(db_err)?)
                    .map_err(json_err)?,
            })
        })
        .transpose()
    }
    async fn einreichung(
        &self,
        p: Einreichung,
        basis: Option<i64>,
        e: NeuerProtokolleintrag,
    ) -> CoreResult<Einreichung> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let payload = serde_json::to_string(&p).map_err(json_err)?;
        let count = if let Some(b) = basis {
            sqlx::query("UPDATE einreichungen SET version=$1,payload=$2 WHERE id=$3 AND version=$4")
                .bind(p.version)
                .bind(&payload)
                .bind(uid(p.stand.id))
                .bind(b)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?
                .rows_affected()
        } else {
            sqlx::query("INSERT INTO einreichungen (id,project_id,version,created_at,payload) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (id) DO NOTHING").bind(uid(p.stand.id)).bind(uid(p.stand.projekt_id.0)).bind(p.version).bind(p.stand.erstellt_am.to_rfc3339()).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count == 0 && basis.is_none() {
            let raw: String = sqlx::query_scalar("SELECT payload FROM einreichungen WHERE id=$1")
                .bind(uid(p.stand.id))
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;
            let alt: Einreichung = serde_json::from_str(&raw).map_err(json_err)?;
            if core_lib::einreichung::gleicher_auftrag(&alt.stand, &p.stand) {
                tx.commit().await.map_err(db_err)?;
                return Ok(alt);
            }
            return Err(CoreError::InvalidInput(
                "Vorgangskennung wurde für ein anderes Paket verwendet".into(),
            ));
        }
        if count != 1 {
            let actual: Option<i64> =
                sqlx::query_scalar("SELECT version FROM einreichungen WHERE id=$1")
                    .bind(uid(p.stand.id))
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
            return Err(CoreError::VersionConflict {
                expected: basis.unwrap_or(0),
                actual: actual.unwrap_or(0),
            });
        }
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(p)
    }
    async fn fachstand(
        &self,
        mut p: Fachstand,
        basis: i64,
        e: NeuerProtokolleintrag,
    ) -> CoreResult<Fachstand> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        p.revision = basis + 1;
        let payload = serde_json::to_string(&p.daten).map_err(json_err)?;
        let count = if basis == 0 {
            sqlx::query("INSERT INTO fachstaende (project_id,modul,objekt,schema_version,revision,payload) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (project_id,modul,objekt) DO NOTHING").bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).bind(i64::from(p.schema_version)).bind(p.revision).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        } else {
            sqlx::query("UPDATE fachstaende SET schema_version=$1,revision=$2,payload=$3 WHERE project_id=$4 AND modul=$5 AND objekt=$6 AND revision=$7").bind(i64::from(p.schema_version)).bind(p.revision).bind(&payload).bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).bind(basis).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count != 1 {
            let actual: Option<i64> = sqlx::query_scalar(
                "SELECT revision FROM fachstaende WHERE project_id=$1 AND modul=$2 AND objekt=$3",
            )
            .bind(uid(p.projekt_id.0))
            .bind(&p.modul)
            .bind(&p.objekt)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: actual.unwrap_or(0),
            });
        }
        sqlx::query("INSERT INTO fachhistorie (project_id,modul,objekt,schema_version,revision,payload) SELECT project_id,modul,objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=$1 AND modul=$2 AND objekt=$3").bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).execute(&mut *tx).await.map_err(db_err)?;
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(p)
    }
    async fn fachstaende(&self, projekt: ProjectId, modul: &str) -> CoreResult<Vec<Fachstand>> {
        let rows=sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=$1 AND modul=$2 ORDER BY objekt").bind(uid(projekt.0)).bind(modul).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.into_iter()
            .map(|r| {
                Ok(Fachstand {
                    projekt_id: projekt,
                    modul: modul.into(),
                    objekt: r.try_get("objekt").map_err(db_err)?,
                    schema_version: u32::try_from(
                        r.try_get::<i64, _>("schema_version").map_err(db_err)?,
                    )
                    .map_err(|e| CoreError::Database(e.to_string()))?,
                    revision: r.try_get("revision").map_err(db_err)?,
                    daten: serde_json::from_str(
                        &r.try_get::<String, _>("payload").map_err(db_err)?,
                    )
                    .map_err(json_err)?,
                })
            })
            .collect()
    }
    async fn fachverlauf(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Vec<Fachstand>> {
        let rows=sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachhistorie WHERE project_id=$1 AND modul=$2 AND objekt=$3 ORDER BY revision").bind(uid(projekt.0)).bind(modul).bind(objekt).fetch_all(&self.pool).await.map_err(db_err)?;
        rows.into_iter()
            .map(|r| {
                Ok(Fachstand {
                    projekt_id: projekt,
                    modul: modul.into(),
                    objekt: r.try_get("objekt").map_err(db_err)?,
                    schema_version: u32::try_from(
                        r.try_get::<i64, _>("schema_version").map_err(db_err)?,
                    )
                    .map_err(|e| CoreError::Database(e.to_string()))?,
                    revision: r.try_get("revision").map_err(db_err)?,
                    daten: serde_json::from_str(
                        &r.try_get::<String, _>("payload").map_err(db_err)?,
                    )
                    .map_err(json_err)?,
                })
            })
            .collect()
    }
}
