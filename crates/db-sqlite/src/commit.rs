//! Fachliche Commits und Audit in derselben Datenbanktransaktion.
use super::*;
use core_lib::{
    commit::{CommitRepository, DokumentCommit, DokumentZusatz, Fachstand},
    einreichung::Einreichung,
};
use sqlx::SqliteConnection;
fn uid(id: Uuid) -> String {
    id.to_string()
}
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::Database(e.to_string())
}
async fn audit(c: &mut SqliteConnection, e: NeuerProtokolleintrag) -> CoreResult<()> {
    sqlx::query("INSERT INTO protokoll (zeit,benutzer_id,benutzer_name,aktion,project_id,document_id,details) VALUES (?,?,?,?,?,?,?)").bind(e.zeit).bind(uid(e.benutzer_id.0)).bind(&e.benutzer_name).bind(e.aktion.code()).bind(e.project_id.map(|p|uid(p.0))).bind(e.document_id.map(|d|uid(d.0))).bind(&e.details).execute(c).await.map_err(db_err)?;
    Ok(())
}
#[async_trait]
impl CommitRepository for SqliteDatabase {
    async fn bauantrag(
        &self,
        p: ProjectId,
        daten: String,
        basis: i64,
        e: NeuerProtokolleintrag,
    ) -> CoreResult<i64> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        let neu: Option<i64> = if basis == 0 {
            sqlx::query_scalar("INSERT INTO bauantraege (project_id,daten,version,geaendert_von,geaendert_am) VALUES (?,?,1,?,?) ON CONFLICT(project_id) DO NOTHING RETURNING version")
            .bind(p.to_string()).bind(&daten).bind(&e.benutzer_name).bind(e.zeit).fetch_optional(&mut *tx).await.map_err(db_err)?
        } else {
            sqlx::query_scalar("UPDATE bauantraege SET daten=?,version=version+1,geaendert_von=?,geaendert_am=? WHERE project_id=? AND version=? RETURNING version")
            .bind(&daten).bind(&e.benutzer_name).bind(e.zeit).bind(p.to_string()).bind(basis).fetch_optional(&mut *tx).await.map_err(db_err)?
        };
        let Some(neu) = neu else {
            let actual: Option<i64> =
                sqlx::query_scalar("SELECT version FROM bauantraege WHERE project_id=?")
                    .bind(p.to_string())
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: actual.unwrap_or(0),
            });
        };
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(neu)
    }

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
            sqlx::query("UPDATE projects SET name=?,bundesland=?,grundstuecksflaeche_m2=?,version=?,updated_at=?,planungsrecht=? WHERE id=? AND version=?").bind(&p.name).bind(p.bundesland.code()).bind(p.grundstuecksflaeche_m2).bind(p.version).bind(p.updated_at).bind(&payload).bind(uid(p.id.0)).bind(b).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        } else {
            sqlx::query("INSERT INTO projects (id,name,bundesland,grundstuecksflaeche_m2,version,created_at,updated_at,planungsrecht) VALUES (?,?,?,?,?,?,?,?)").bind(uid(p.id.0)).bind(&p.name).bind(p.bundesland.code()).bind(p.grundstuecksflaeche_m2).bind(p.version).bind(p.created_at).bind(p.updated_at).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count != 1 {
            let actual: Option<i64> = sqlx::query_scalar("SELECT version FROM projects WHERE id=?")
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
            sqlx::query("INSERT INTO fachstaende (project_id,modul,objekt,schema_version,revision,payload) VALUES (?,'mitgliedschaften',?,1,1,?)").bind(uid(p.id.0)).bind(e.benutzer_id.to_string()).bind(mitglied).execute(&mut *tx).await.map_err(db_err)?;
            sqlx::query("INSERT INTO fachhistorie SELECT project_id,modul,objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=? AND modul='mitgliedschaften'").bind(uid(p.id.0)).execute(&mut *tx).await.map_err(db_err)?;
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
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        let latest: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM document_versions WHERE document_id=?")
                .bind(uid(d.document_id.0))
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;
        if neuanlage && latest.is_some() {
            let r = sqlx::query(
                "SELECT * FROM document_versions WHERE document_id=? ORDER BY version DESC LIMIT 1",
            )
            .bind(uid(d.document_id.0))
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
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
        sqlx::query("INSERT INTO document_versions (document_id,project_id,version,file_name,storage_key,sha256,size_bytes,created_by,created_at) VALUES (?,?,?,?,?,?,?,?,?)").bind(uid(d.document_id.0)).bind(uid(d.project_id.0)).bind(d.version).bind(&d.file_name).bind(&d.storage_key).bind(d.sha256.as_str()).bind(i64::try_from(d.size_bytes).map_err(|e|CoreError::InvalidInput(e.to_string()))?).bind(&d.created_by).bind(d.created_at).execute(&mut *tx).await.map_err(db_err)?;
        for z in zusaetze {
            sqlx::query("INSERT INTO dokumentkontexte (document_id,version,modul,schema_version,payload) VALUES (?,?,?,?,?)").bind(uid(d.document_id.0)).bind(d.version).bind(z.modul).bind(i64::from(z.schema_version)).bind(serde_json::to_string(&z.daten).map_err(json_err)?).execute(&mut *tx).await.map_err(db_err)?;
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
        let r=sqlx::query("SELECT schema_version,payload FROM dokumentkontexte WHERE document_id=? AND version=? AND modul=?").bind(uid(id.0)).bind(version).bind(modul).fetch_optional(&self.pool).await.map_err(db_err)?;
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
            sqlx::query("UPDATE einreichungen SET version=?,payload=? WHERE id=? AND version=?")
                .bind(p.version)
                .bind(&payload)
                .bind(uid(p.stand.id))
                .bind(b)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?
                .rows_affected()
        } else {
            sqlx::query("INSERT INTO einreichungen (id,project_id,version,created_at,payload) VALUES (?,?,?,?,?) ON CONFLICT (id) DO NOTHING").bind(uid(p.stand.id)).bind(uid(p.stand.projekt_id.0)).bind(p.version).bind(p.stand.erstellt_am.to_rfc3339()).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count == 0 && basis.is_none() {
            let raw: String = sqlx::query_scalar("SELECT payload FROM einreichungen WHERE id=?")
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
                sqlx::query_scalar("SELECT version FROM einreichungen WHERE id=?")
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
            sqlx::query("INSERT INTO fachstaende (project_id,modul,objekt,schema_version,revision,payload) VALUES (?,?,?,?,?,?) ON CONFLICT (project_id,modul,objekt) DO NOTHING").bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).bind(i64::from(p.schema_version)).bind(p.revision).bind(&payload).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        } else {
            sqlx::query("UPDATE fachstaende SET schema_version=?,revision=?,payload=? WHERE project_id=? AND modul=? AND objekt=? AND revision=?").bind(i64::from(p.schema_version)).bind(p.revision).bind(&payload).bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).bind(basis).execute(&mut *tx).await.map_err(db_err)?.rows_affected()
        };
        if count != 1 {
            let actual: Option<i64> = sqlx::query_scalar(
                "SELECT revision FROM fachstaende WHERE project_id=? AND modul=? AND objekt=?",
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
        sqlx::query("INSERT INTO fachhistorie (project_id,modul,objekt,schema_version,revision,payload) SELECT project_id,modul,objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=? AND modul=? AND objekt=?").bind(uid(p.projekt_id.0)).bind(&p.modul).bind(&p.objekt).execute(&mut *tx).await.map_err(db_err)?;
        audit(&mut tx, e).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(p)
    }
    async fn suchtextseite(
        &self,
        projekt: ProjectId,
        nach: Option<&str>,
        suche: &str,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let modul = "suchtexte";
        let rows=sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachstaende f WHERE project_id=? AND modul='suchtexte' AND (? IS NULL OR objekt>?) AND (json_type(payload,'$._suchtext_norm') IS NULL OR (?<3 AND instr(json_extract(payload,'$._suchtext_norm'),?)>0) OR (?>=3 AND objekt IN (SELECT objekt FROM suchtext_fts WHERE text MATCH ? AND project_id=?))) ORDER BY objekt LIMIT ?").bind(uid(projekt.0)).bind(nach).bind(nach).bind(suche.chars().count() as i64).bind(suche).bind(suche.chars().count() as i64).bind(format!("\"{}\"", suche.replace('"', "\"\""))).bind(uid(projekt.0)).bind(i64::from(limit.clamp(1,100))).fetch_all(&self.pool).await.map_err(db_err)?;
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
    async fn fachseite(
        &self,
        projekt: ProjectId,
        modul: &str,
        nach: Option<&str>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let rows = sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=? AND modul=? AND (? IS NULL OR objekt > ?) ORDER BY objekt LIMIT ?").bind(uid(projekt.0)).bind(modul).bind(nach).bind(nach).bind(i64::from(limit.clamp(1,100))).fetch_all(&self.pool).await.map_err(db_err)?;
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
    async fn fachverlaufsseite(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
        vor: Option<i64>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        let rows = sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachhistorie WHERE project_id=? AND modul=? AND objekt=? AND (? IS NULL OR revision < ?) ORDER BY revision DESC LIMIT ?").bind(uid(projekt.0)).bind(modul).bind(objekt).bind(vor).bind(vor).bind(i64::from(limit.clamp(1,100))).fetch_all(&self.pool).await.map_err(db_err)?;
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
    async fn fachobjekt(
        &self,
        projekt: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Option<Fachstand>> {
        let rows = sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=? AND modul=? AND objekt=?").bind(uid(projekt.0)).bind(modul).bind(objekt).fetch_all(&self.pool).await.map_err(db_err)?;
        let mut daten = rows
            .into_iter()
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
            .collect::<CoreResult<Vec<_>>>()?;
        Ok(daten.pop())
    }
    async fn fachrevisionen(
        &self,
        projekt: ProjectId,
        modul: &str,
    ) -> CoreResult<Vec<(String, i64)>> {
        sqlx::query("SELECT objekt,revision FROM fachstaende WHERE project_id=? AND modul=? ORDER BY objekt").bind(uid(projekt.0)).bind(modul).fetch_all(&self.pool).await.map_err(db_err)?.into_iter().map(|r|Ok((r.try_get("objekt").map_err(db_err)?,r.try_get("revision").map_err(db_err)?))).collect()
    }
    async fn fachstaende(&self, projekt: ProjectId, modul: &str) -> CoreResult<Vec<Fachstand>> {
        let rows=sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachstaende WHERE project_id=? AND modul=? ORDER BY objekt").bind(uid(projekt.0)).bind(modul).fetch_all(&self.pool).await.map_err(db_err)?;
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
        let rows=sqlx::query("SELECT objekt,schema_version,revision,payload FROM fachhistorie WHERE project_id=? AND modul=? AND objekt=? ORDER BY revision").bind(uid(projekt.0)).bind(modul).bind(objekt).fetch_all(&self.pool).await.map_err(db_err)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use core_lib::{benutzer::BenutzerId, domain::Aktion, storage::ContentHash};
    fn eintrag(p: ProjectId) -> NeuerProtokolleintrag {
        NeuerProtokolleintrag {
            zeit: Utc::now(),
            benutzer_id: BenutzerId::new(),
            benutzer_name: "Test".into(),
            aktion: Aktion::ProjektGeaendert,
            project_id: Some(p),
            document_id: None,
            details: "Test".into(),
        }
    }
    #[tokio::test]
    async fn auditfehler_rollt_fachliche_schreibvorgaenge_zurueck() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        let p = ProjectRepository::create(
            &db,
            NewProject {
                name: "Original".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
        )
        .await
        .unwrap();
        sqlx::query("CREATE TRIGGER audit_ausfall BEFORE INSERT ON protokoll BEGIN SELECT RAISE(ABORT, 'Simulierter Audit-Ausfall'); END").execute(&db.pool).await.unwrap();
        assert!(
            CommitRepository::bauantrag(&db, p.id, "{}".into(), 0, eintrag(p.id))
                .await
                .is_err()
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bauantraege WHERE project_id=?")
            .bind(p.id.to_string())
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "Bauantrag und Audit bilden eine Transaktion");
        let mut neu = p.clone();
        neu.name = "Nicht bestätigt".into();
        assert!(
            CommitRepository::projekt(&db, neu, Some(1), eintrag(p.id))
                .await
                .is_err()
        );
        assert_eq!(
            ProjectRepository::get(&db, p.id).await.unwrap().name,
            "Original"
        );
        let d = DocumentVersion {
            document_id: DocumentId::new(),
            project_id: p.id,
            version: 0,
            file_name: "Test.pdf".into(),
            storage_key: "Test.pdf".into(),
            sha256: ContentHash::of(b"Test"),
            size_bytes: 4,
            created_by: "Test".into(),
            created_at: Utc::now(),
        };
        assert!(
            CommitRepository::dokument(
                &db,
                DokumentCommit {
                    dokument: d.clone(),
                    basis: None,
                    neuanlage: true,
                    zusaetze: vec![]
                },
                eintrag(p.id)
            )
            .await
            .is_err()
        );
        assert!(
            DocumentRepository::history(&db, d.document_id)
                .await
                .unwrap()
                .is_empty()
        );
        let f = Fachstand {
            projekt_id: p.id,
            modul: "aufgaben".into(),
            objekt: "1".into(),
            schema_version: 1,
            revision: 0,
            daten: serde_json::json!({"titel":"Test"}),
        };
        assert!(
            CommitRepository::fachstand(&db, f, 0, eintrag(p.id))
                .await
                .is_err()
        );
        assert!(
            CommitRepository::fachstaende(&db, p.id, "aufgaben")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(ProjectRepository::get(&db, p.id).await.unwrap().version, 1);
    }
    #[tokio::test]
    async fn fachrevisionen_sind_unabhaengig_und_schuetzen_konkurrierende_entwuerfe() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        let p = ProjectRepository::create(
            &db,
            NewProject {
                name: "Test".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
        )
        .await
        .unwrap();
        let mut f = Fachstand {
            projekt_id: p.id,
            modul: "aufgaben".into(),
            objekt: "1".into(),
            schema_version: 1,
            revision: 0,
            daten: serde_json::json!({"titel":"Test"}),
        };
        f = CommitRepository::fachstand(&db, f, 0, eintrag(p.id))
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            CommitRepository::fachstand(&db, f.clone(), 1, eintrag(p.id)),
            CommitRepository::fachstand(&db, f.clone(), 1, eintrag(p.id))
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert!(matches!(
            a.err().or(b.err()).unwrap(),
            CoreError::VersionConflict { .. }
        ));
        assert_eq!(
            CommitRepository::fachstaende(&db, p.id, "aufgaben")
                .await
                .unwrap()[0]
                .revision,
            2
        );
        assert_eq!(ProjectRepository::get(&db, p.id).await.unwrap().version, 1);
    }
}
