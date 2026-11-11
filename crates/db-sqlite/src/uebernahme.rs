//! Projekte aus einer anderen Profildatenbank (z. B. einer entpackten
//! Sicherung oder dem Beispielprojekt) in die laufende Datenbank übernehmen.
//!
//! Anders als die Wiederherstellung in ein neues Profil braucht das keinen
//! Neustart: Die Zeilen werden in einer Transaktion in die geöffnete
//! Datenbank kopiert und sind sofort für alle Verbindungen sichtbar. Bestehende
//! Projekte, Kunden, Benutzer und Büroangaben werden nie überschrieben.
use core_lib::{CoreError, CoreResult};
use sqlx::{Connection, Row, SqliteConnection};
use std::path::Path;

use crate::{SqliteDatabase, db_err};

/// Ein übernommenes Projekt (ID und Name), für die Rückmeldung an die Oberfläche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UebernommenesProjekt {
    pub id: uuid::Uuid,
    pub name: String,
}

/// Projektbezogene Tabellen in Einfügereihenfolge (Fremdschlüssel). Beide
/// Datenbanken stehen auf demselben Migrationsstand (geprüft), die Spalten
/// liegen daher in derselben Reihenfolge und `SELECT *` passt.
const PROJEKTTABELLEN: [&str; 10] = [
    "projects",
    "document_versions",
    "dokumentkontexte",
    "bauantraege",
    "projekt_bauherren",
    "projekt_beteiligte",
    "einreichungen",
    "fachstaende",
    "fachhistorie",
    "entfernte_dokumente",
];

const BUERO: &str = "xbau.buero";

async fn migrationsstand(conn: &mut SqliteConnection, schema: &str) -> CoreResult<Option<i64>> {
    sqlx::query_scalar(&format!(
        "SELECT MAX(version) FROM {schema}._sqlx_migrations WHERE success=1"
    ))
    .fetch_one(&mut *conn)
    .await
    .map_err(db_err)
}

impl SqliteDatabase {
    /// Kopiert alle Projekte aus der Datenbankdatei `quelle` in diese
    /// Datenbank. `quelle` muss zuvor mit [`SqliteDatabase::open`] auf den
    /// aktuellen Stand gebracht worden sein.
    ///
    /// - Gibt es eines der Projekte hier schon, wird nichts übernommen.
    /// - Kunden und Benutzer kommen nur hinzu, wenn es sie (ID bzw.
    ///   Anmeldename) hier noch nicht gibt.
    /// - Büroangaben: Fehlen sie hier, gelten die der Quelle. Sonst kommen nur
    ///   fehlende Entwurfsverfasser hinzu, damit Bauanträge der übernommenen
    ///   Projekte vollständig bleiben.
    /// - Das Protokoll der Projekte wird mit neuen laufenden Nummern angehängt.
    pub async fn projekte_uebernehmen(
        &self,
        quelle: &Path,
    ) -> CoreResult<Vec<UebernommenesProjekt>> {
        let pfad = quelle
            .to_str()
            .ok_or_else(|| CoreError::InvalidInput("Pfad ist nicht UTF-8".into()))?;
        let mut conn = self.pool.acquire().await.map_err(db_err)?;
        sqlx::query("ATTACH DATABASE ? AS quelle")
            .bind(pfad)
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;
        let ergebnis = uebernehmen(&mut conn).await;
        // Auch nach einem Fehler lösen, damit die Verbindung im Pool sauber bleibt.
        let geloest = sqlx::query("DETACH DATABASE quelle")
            .execute(&mut *conn)
            .await
            .map_err(db_err);
        let projekte = ergebnis?;
        geloest?;
        Ok(projekte)
    }
}

async fn uebernehmen(conn: &mut SqliteConnection) -> CoreResult<Vec<UebernommenesProjekt>> {
    let hier = migrationsstand(conn, "main").await?;
    let dort = migrationsstand(conn, "quelle").await?;
    if hier != dort {
        return Err(CoreError::InvalidInput(format!(
            "Die Sicherung hat einen anderen Datenbankstand ({}) als dieses Profil ({}).",
            dort.unwrap_or_default(),
            hier.unwrap_or_default()
        )));
    }
    let projekte: Vec<UebernommenesProjekt> =
        sqlx::query("SELECT id, name FROM quelle.projects ORDER BY name")
            .fetch_all(&mut *conn)
            .await
            .map_err(db_err)?
            .iter()
            .map(|r| {
                let id: String = r.try_get("id").map_err(db_err)?;
                Ok(UebernommenesProjekt {
                    id: crate::parse_uuid(&id)?,
                    name: r.try_get("name").map_err(db_err)?,
                })
            })
            .collect::<CoreResult<_>>()?;
    if projekte.is_empty() {
        return Err(CoreError::InvalidInput(
            "Die Sicherung enthält keine Projekte.".into(),
        ));
    }
    let vorhanden: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM quelle.projects WHERE id IN (SELECT id FROM main.projects) ORDER BY name",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?;
    if !vorhanden.is_empty() {
        return Err(CoreError::InvalidInput(format!(
            "Bereits vorhanden: {}. Zum erneuten Übernehmen das Projekt zuerst löschen.",
            vorhanden
                .iter()
                .map(|n| format!("„{n}“"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let mut tx = conn.begin().await.map_err(db_err)?;
    sqlx::query(
        "INSERT INTO main.benutzer SELECT * FROM quelle.benutzer
         WHERE id NOT IN (SELECT id FROM main.benutzer)
           AND anmeldename NOT IN (SELECT anmeldename FROM main.benutzer)",
    )
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    sqlx::query("INSERT OR IGNORE INTO main.kunden SELECT * FROM quelle.kunden")
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    for tabelle in PROJEKTTABELLEN {
        sqlx::query(&format!(
            "INSERT INTO main.{tabelle} SELECT * FROM quelle.{tabelle}"
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| CoreError::Database(format!("{tabelle}: {e}")))?;
    }
    sqlx::query(
        "INSERT INTO main.protokoll (zeit, benutzer_id, benutzer_name, aktion, project_id, document_id, details)
         SELECT zeit, benutzer_id, benutzer_name, aktion, project_id, document_id, details
         FROM quelle.protokoll WHERE project_id IN (SELECT id FROM quelle.projects) ORDER BY id",
    )
    .execute(&mut *tx)
    .await
    .map_err(db_err)?;
    buero_ergaenzen(&mut tx).await?;
    tx.commit().await.map_err(db_err)?;
    Ok(projekte)
}

async fn buero_ergaenzen(conn: &mut SqliteConnection) -> CoreResult<()> {
    let lesen = |schema: &'static str| {
        sqlx::query_scalar::<_, String>(match schema {
            "quelle" => "SELECT wert FROM quelle.einstellungen WHERE schluessel = ?",
            _ => "SELECT wert FROM main.einstellungen WHERE schluessel = ?",
        })
        .bind(BUERO)
    };
    let Some(dort) = lesen("quelle")
        .fetch_optional(&mut *conn)
        .await
        .map_err(db_err)?
    else {
        return Ok(());
    };
    let Some(hier) = lesen("main")
        .fetch_optional(&mut *conn)
        .await
        .map_err(db_err)?
    else {
        sqlx::query("INSERT INTO main.einstellungen (schluessel, wert) VALUES (?, ?)")
            .bind(BUERO)
            .bind(&dort)
            .execute(&mut *conn)
            .await
            .map_err(db_err)?;
        return Ok(());
    };
    let json = |s: &str| {
        serde_json::from_str::<serde_json::Value>(s)
            .map_err(|e| CoreError::Database(format!("Büroangaben: {e}")))
    };
    let (mut hier_json, dort_json) = (json(&hier)?, json(&dort)?);
    let fehlende: Vec<serde_json::Value> = {
        let bekannt: Vec<&serde_json::Value> = hier_json["entwurfsverfasser"]
            .as_array()
            .map(|l| l.iter().map(|e| &e["id"]).collect())
            .unwrap_or_default();
        dort_json["entwurfsverfasser"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter(|e| !bekannt.contains(&&e["id"]))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    };
    if fehlende.is_empty() {
        return Ok(());
    }
    match hier_json.get_mut("entwurfsverfasser") {
        Some(serde_json::Value::Array(l)) => l.extend(fehlende),
        _ => hier_json["entwurfsverfasser"] = serde_json::Value::Array(fehlende),
    }
    sqlx::query("UPDATE main.einstellungen SET wert = ? WHERE schluessel = ?")
        .bind(hier_json.to_string())
        .bind(BUERO)
        .execute(&mut *conn)
        .await
        .map_err(db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_lib::domain::NewProject;

    async fn profil(dir: &Path, name: &str) -> SqliteDatabase {
        SqliteDatabase::open(&dir.join(name)).await.unwrap()
    }

    #[tokio::test]
    async fn uebernimmt_projekte_einmal_und_laesst_bestehende_stehen() {
        let dir = tempfile::tempdir().unwrap();
        let quelle = profil(dir.path(), "quelle.db").await;
        let neu = |name: &str| NewProject {
            name: name.into(),
            bundesland: core_lib::domain::Bundesland::Nw,
            grundstuecksflaeche_m2: None,
            planungsrecht: Default::default(),
        };
        let p = quelle
            .repositories()
            .projects
            .create(neu("Beispiel"))
            .await
            .unwrap();
        quelle.schliessen().await;

        let ziel = profil(dir.path(), "ziel.db").await;
        let eigenes = ziel
            .repositories()
            .projects
            .create(neu("Eigenes"))
            .await
            .unwrap();
        let uebernommen = ziel
            .projekte_uebernehmen(&dir.path().join("quelle.db"))
            .await
            .unwrap();
        assert_eq!(
            uebernommen,
            vec![UebernommenesProjekt {
                id: p.id.0,
                name: "Beispiel".into()
            }]
        );
        let liste = ziel.repositories().projects.list().await.unwrap();
        assert_eq!(liste.len(), 2);
        assert!(liste.iter().any(|x| x.id == eigenes.id));
        assert_eq!(
            ziel.repositories().projects.get(p.id).await.unwrap().name,
            "Beispiel"
        );

        // Zweites Mal: verständlicher Fehler, nichts doppelt.
        let e = ziel
            .projekte_uebernehmen(&dir.path().join("quelle.db"))
            .await
            .unwrap_err();
        assert!(
            e.to_string().contains("Bereits vorhanden: „Beispiel“"),
            "{e}"
        );
        assert_eq!(ziel.repositories().projects.list().await.unwrap().len(), 2);
    }
}
