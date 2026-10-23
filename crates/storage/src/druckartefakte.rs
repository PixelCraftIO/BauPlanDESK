//! Kurzlebige Druckausgaben: Datei statt Base64-Rundtransport. Identität und
//! Eigentümer werden geprüft; höchstens acht offene Ausgaben (je max. 128 MiB).
use bytes::Bytes;
use core_lib::{CoreError, CoreResult, domain::ProjectId};
use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;
struct Eintrag {
    projekt: ProjectId,
    benutzer: Uuid,
    datei: Arc<tempfile::TempPath>,
    _platz: OwnedSemaphorePermit,
}
pub struct Druckartefakte {
    daten: Mutex<HashMap<Uuid, Eintrag>>,
    plaetze: Arc<Semaphore>,
}
impl Default for Druckartefakte {
    fn default() -> Self {
        Self {
            daten: Mutex::new(HashMap::new()),
            plaetze: Arc::new(Semaphore::new(8)),
        }
    }
}
impl Druckartefakte {
    pub async fn speichern(
        &self,
        projekt: ProjectId,
        benutzer: Uuid,
        daten: Vec<u8>,
    ) -> CoreResult<Uuid> {
        if daten.len() > core_lib::druckausgabe::MAX_DRUCKPDF {
            return Err(CoreError::InvalidInput("Druckausgabe zu groß".into()));
        }
        let platz = self.plaetze.clone().try_acquire_owned().map_err(|_| {
            CoreError::InvalidInput(
                "Zu viele offene Druckausgaben. Eine Druckvorschau schließen.".into(),
            )
        })?;
        let datei = tokio::task::spawn_blocking(move || {
            let mut f = tempfile::Builder::new()
                .prefix("bauplan-druck-")
                .tempfile()
                .map_err(err)?;
            f.write_all(&daten).map_err(err)?;
            f.flush().map_err(err)?;
            Ok::<_, CoreError>(Arc::new(f.into_temp_path()))
        })
        .await
        .map_err(err)??;
        let id = Uuid::new_v4();
        self.daten.lock().map_err(err)?.insert(
            id,
            Eintrag {
                projekt,
                benutzer,
                datei,
                _platz: platz,
            },
        );
        Ok(id)
    }
    fn datei(
        &self,
        id: Uuid,
        projekt: ProjectId,
        benutzer: Uuid,
    ) -> CoreResult<Arc<tempfile::TempPath>> {
        self.daten
            .lock()
            .map_err(err)?
            .get(&id)
            .filter(|d| d.projekt == projekt && d.benutzer == benutzer)
            .map(|d| d.datei.clone())
            .ok_or_else(|| {
                CoreError::NotFound("Druckvorschau abgelaufen oder nicht zugänglich".into())
            })
    }
    pub async fn lesen(&self, id: Uuid, projekt: ProjectId, benutzer: Uuid) -> CoreResult<Bytes> {
        let datei = self.datei(id, projekt, benutzer)?;
        Ok(tokio::fs::read(&**datei).await.map_err(err)?.into())
    }
    pub fn benutzer_schliessen(&self, benutzer: Uuid) -> CoreResult<()> {
        self.daten
            .lock()
            .map_err(err)?
            .retain(|_, d| d.benutzer != benutzer);
        Ok(())
    }
    pub fn entfernen(&self, id: Uuid, projekt: ProjectId, benutzer: Uuid) -> CoreResult<()> {
        let mut daten = self.daten.lock().map_err(err)?;
        if daten
            .get(&id)
            .is_some_and(|d| d.projekt == projekt && d.benutzer == benutzer)
        {
            daten.remove(&id);
        }
        Ok(())
    }
}
fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn isolation_freigabe_und_begrenzung() {
        let a = Druckartefakte::default();
        let p = ProjectId(Uuid::new_v4());
        let u = Uuid::new_v4();
        let id = a.speichern(p, u, b"%PDF-Test".to_vec()).await.unwrap();
        assert!(a.lesen(id, p, Uuid::new_v4()).await.is_err());
        assert!(a.lesen(id, ProjectId(Uuid::new_v4()), u).await.is_err());
        assert_eq!(a.lesen(id, p, u).await.unwrap(), b"%PDF-Test"[..]);
        for _ in 0..7 {
            a.speichern(p, u, vec![]).await.unwrap();
        }
        assert!(a.speichern(p, u, vec![]).await.is_err());
        a.entfernen(id, p, u).unwrap();
        assert!(a.lesen(id, p, u).await.is_err());
        a.speichern(p, u, vec![]).await.unwrap();
        a.benutzer_schliessen(u).unwrap();
        for _ in 0..8 {
            a.speichern(p, u, vec![]).await.unwrap();
        }
    }
}
