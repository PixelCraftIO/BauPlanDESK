//! Persistenz eigener Paketstände; Dokumentversionen bleiben im DMS.
use super::*;
use core_lib::{
    einreichung::{Einreichung, EinreichungsBestaetigung},
    repository::EinreichungRepository,
};
use uuid::Uuid;
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::Database(e.to_string())
}
#[async_trait]
impl EinreichungRepository for SqliteDatabase {
    async fn anlegen(&self, paket: &Einreichung) -> CoreResult<()> {
        sqlx::query("INSERT INTO einreichungen (id,project_id,version,created_at,payload) VALUES (?,?,?,?,?)")
            .bind(paket.stand.id.to_string()).bind(paket.stand.projekt_id.0.to_string()).bind(paket.version).bind(paket.stand.erstellt_am.to_rfc3339()).bind(serde_json::to_string(paket).map_err(json_err)?)
            .execute(&self.pool).await.map_err(db_err)?;
        Ok(())
    }
    async fn laden(&self, id: Uuid) -> CoreResult<Einreichung> {
        let payload: Option<String> =
            sqlx::query_scalar("SELECT payload FROM einreichungen WHERE id=?")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        serde_json::from_str(
            &payload.ok_or_else(|| CoreError::NotFound(format!("Einreichung {id}")))?,
        )
        .map_err(json_err)
    }
    async fn auflisten(&self, projekt: ProjectId) -> CoreResult<Vec<Einreichung>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT payload FROM einreichungen WHERE project_id=? ORDER BY created_at DESC,id DESC",
        )
        .bind(projekt.0.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|p| serde_json::from_str(p).map_err(json_err))
            .collect()
    }
    async fn bestaetigen(
        &self,
        id: Uuid,
        basis: i64,
        b: EinreichungsBestaetigung,
    ) -> CoreResult<Einreichung> {
        let mut paket = self.laden(id).await?;
        if paket.version != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: paket.version,
            });
        }
        if paket.einreichung.is_some() {
            return Err(CoreError::InvalidInput(
                "Einreichung ist bereits bestätigt; neue Unterlagen als eigenes Paket nachreichen"
                    .into(),
            ));
        }
        paket.einreichung = Some(b);
        paket.version += 1;
        let result =
            sqlx::query("UPDATE einreichungen SET payload=?,version=? WHERE id=? AND version=?")
                .bind(serde_json::to_string(&paket).map_err(json_err)?)
                .bind(paket.version)
                .bind(id.to_string())
                .bind(basis)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        if result.rows_affected() != 1 {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: self.laden(id).await?.version,
            });
        }
        Ok(paket)
    }
}
