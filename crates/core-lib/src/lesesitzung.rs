//! Kurzlebige, ausschließlich lesende Sicht einer einzelnen Auswertung.
//!
//! Kein Cache über Requests hinweg: jede neue Auswertung liest neue Revisionen.
//! Wiederholte Zugriffe derselben Auswertung verwenden dieselben Quellenstände.
//! Dies ersetzt keine Datenbanktransaktion über mehrere Repository-Traits.
use crate::{CoreError, CoreResult, CoreServices, commit::*, domain::*, repository::*};
use async_trait::async_trait;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

pub(crate) struct Lesesitzung {
    projekt: Project,
    dokumente: Vec<DocumentVersion>,
    commit: Arc<dyn CommitRepository>,
    quelle: Option<Repositories>,
    fach: Mutex<BTreeMap<String, Vec<Fachstand>>>,
}
fn nur_lesen<T>() -> CoreResult<T> {
    Err(CoreError::Forbidden(
        "Lesesitzung erlaubt keine Schreiboperation".into(),
    ))
}
impl CoreServices {
    pub(crate) async fn lesesitzung(&self, pid: ProjectId) -> CoreResult<(Self, Arc<Lesesitzung>)> {
        let sicht = Arc::new(Lesesitzung {
            projekt: self.repos.projects.get(pid).await?,
            dokumente: self.repos.documents.list_for_project(pid).await?,
            commit: self.repos.commit.clone(),
            quelle: Some(self.repos.clone()),
            fach: Mutex::new(BTreeMap::new()),
        });
        let mut core = self.clone();
        core.repos.projects = sicht.clone();
        core.repos.documents = sicht.clone();
        core.repos.commit = sicht.clone();
        Ok((core, sicht))
    }
}
impl Lesesitzung {
    pub(crate) async fn unveraendert(&self) -> CoreResult<()> {
        let Some(repos) = &self.quelle else {
            return Ok(());
        };
        let geaendert = || {
            CoreError::InvalidInput(
                "Quellen während der Nachweisprüfung geändert. Bitte Ansicht aktualisieren.".into(),
            )
        };
        if repos.projects.get(self.projekt.id).await?.version != self.projekt.version {
            return Err(geaendert());
        }
        let aktuell = repos.documents.latest_for_project(self.projekt.id).await?;
        let mut bisher = BTreeMap::new();
        for d in &self.dokumente {
            bisher
                .entry(d.document_id.0)
                .and_modify(|v: &mut i64| *v = (*v).max(d.version))
                .or_insert(d.version);
        }
        let neu = aktuell
            .iter()
            .map(|d| (d.document_id.0, d.version))
            .collect::<BTreeMap<_, _>>();
        if neu != bisher {
            return Err(geaendert());
        }
        let stand = self.fach.lock().await;
        for (modul, daten) in stand.iter() {
            let mut alt = daten
                .iter()
                .map(|d| (d.objekt.clone(), d.revision))
                .collect::<Vec<_>>();
            alt.sort();
            let mut neu = repos.commit.fachrevisionen(self.projekt.id, modul).await?;
            neu.sort();
            if alt != neu {
                return Err(geaendert());
            }
        }
        Ok(())
    }
}
#[async_trait]
impl ProjectRepository for Lesesitzung {
    async fn get(&self, id: ProjectId) -> CoreResult<Project> {
        if self.projekt.id == id {
            Ok(self.projekt.clone())
        } else {
            Err(CoreError::NotFound(id.0.to_string()))
        }
    }
    async fn list(&self) -> CoreResult<Vec<Project>> {
        Ok(vec![self.projekt.clone()])
    }
    async fn create(&self, _: NewProject) -> CoreResult<Project> {
        nur_lesen()
    }
    async fn update(&self, _: ProjectUpdate) -> CoreResult<Project> {
        nur_lesen()
    }
    async fn delete(&self, _: ProjectId, _: i64) -> CoreResult<()> {
        nur_lesen()
    }
}
#[async_trait]
impl DocumentRepository for Lesesitzung {
    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion> {
        self.dokumente
            .iter()
            .filter(|d| d.document_id == id)
            .max_by_key(|d| d.version)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(id.0.to_string()))
    }
    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>> {
        let mut d = self
            .dokumente
            .iter()
            .filter(|d| d.document_id == id)
            .cloned()
            .collect::<Vec<_>>();
        d.sort_by_key(|d| d.version);
        Ok(d)
    }
    async fn list_for_project(&self, p: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        if p != self.projekt.id {
            return Err(CoreError::NotFound(p.to_string()));
        }
        Ok(self.dokumente.clone())
    }
    async fn add_version(&self, _: DocumentVersion, _: Option<i64>) -> CoreResult<DocumentVersion> {
        nur_lesen()
    }
    async fn lock(
        &self,
        _: DocumentId,
        _: ProjectId,
        _: &crate::benutzer::Benutzer,
    ) -> CoreResult<DocumentLock> {
        nur_lesen()
    }
    async fn unlock(
        &self,
        _: DocumentId,
        _: Option<crate::benutzer::BenutzerId>,
    ) -> CoreResult<Option<DocumentLock>> {
        nur_lesen()
    }
    async fn get_lock(&self, _: DocumentId) -> CoreResult<Option<DocumentLock>> {
        nur_lesen()
    }
    async fn locks_for_project(&self, _: ProjectId) -> CoreResult<Vec<DocumentLock>> {
        nur_lesen()
    }
}
#[async_trait]
impl CommitRepository for Lesesitzung {
    async fn fachstaende(&self, p: ProjectId, modul: &str) -> CoreResult<Vec<Fachstand>> {
        if p != self.projekt.id {
            return Err(CoreError::NotFound(p.to_string()));
        }
        let mut daten = self.fach.lock().await;
        if let Some(d) = daten.get(modul) {
            return Ok(d.clone());
        }
        let neu = self.commit.fachstaende(p, modul).await?;
        daten.insert(modul.into(), neu.clone());
        Ok(neu)
    }
    async fn fachobjekt(
        &self,
        p: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Option<Fachstand>> {
        if p != self.projekt.id {
            return Err(CoreError::NotFound(p.to_string()));
        }
        let mut daten = self.fach.lock().await;
        if !daten.contains_key(modul) {
            let neu = self.commit.fachstaende(p, modul).await?;
            tracing::debug!(
                vorgang = "nachweis-quellen",
                anzahl = neu.len(),
                abfragen = 1u32
            );
            daten.insert(modul.into(), neu);
        }
        Ok(daten
            .get(modul)
            .and_then(|d| d.iter().find(|f| f.objekt == objekt))
            .cloned())
    }
    async fn fachverlauf(
        &self,
        p: ProjectId,
        modul: &str,
        objekt: &str,
    ) -> CoreResult<Vec<Fachstand>> {
        if p != self.projekt.id {
            return Err(CoreError::NotFound(p.to_string()));
        }
        self.commit.fachverlauf(p, modul, objekt).await
    }
    async fn dokumentzusatz(
        &self,
        id: DocumentId,
        version: i64,
        modul: &str,
    ) -> CoreResult<Option<DokumentZusatz>> {
        if !self
            .dokumente
            .iter()
            .any(|d| d.document_id == id && d.version == version)
        {
            return Err(CoreError::NotFound(id.0.to_string()));
        }
        self.commit.dokumentzusatz(id, version, modul).await
    }
    async fn bauantrag(
        &self,
        _: ProjectId,
        _: String,
        _: i64,
        _: NeuerProtokolleintrag,
    ) -> CoreResult<i64> {
        nur_lesen()
    }
    async fn projekt(
        &self,
        _: Project,
        _: Option<i64>,
        _: NeuerProtokolleintrag,
    ) -> CoreResult<Project> {
        nur_lesen()
    }
    async fn dokument(
        &self,
        _: DokumentCommit,
        _: NeuerProtokolleintrag,
    ) -> CoreResult<DocumentVersion> {
        nur_lesen()
    }
    async fn einreichung(
        &self,
        _: crate::einreichung::Einreichung,
        _: Option<i64>,
        _: NeuerProtokolleintrag,
    ) -> CoreResult<crate::einreichung::Einreichung> {
        nur_lesen()
    }
    async fn fachstand(
        &self,
        _: Fachstand,
        _: i64,
        _: NeuerProtokolleintrag,
    ) -> CoreResult<Fachstand> {
        nur_lesen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Quelle(AtomicUsize);
    #[async_trait]
    impl CommitRepository for Quelle {
        async fn fachstaende(&self, p: ProjectId, modul: &str) -> CoreResult<Vec<Fachstand>> {
            let revision = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(vec![Fachstand {
                projekt_id: p,
                modul: modul.into(),
                objekt: "quelle".into(),
                revision: revision as i64,
                schema_version: 1,
                daten: serde_json::json!({"wert":revision}),
            }])
        }
        async fn fachverlauf(&self, _: ProjectId, _: &str, _: &str) -> CoreResult<Vec<Fachstand>> {
            nur_lesen()
        }
        async fn dokumentzusatz(
            &self,
            _: DocumentId,
            _: i64,
            _: &str,
        ) -> CoreResult<Option<DokumentZusatz>> {
            nur_lesen()
        }
        async fn bauantrag(
            &self,
            _: ProjectId,
            _: String,
            _: i64,
            _: NeuerProtokolleintrag,
        ) -> CoreResult<i64> {
            nur_lesen()
        }
        async fn projekt(
            &self,
            _: Project,
            _: Option<i64>,
            _: NeuerProtokolleintrag,
        ) -> CoreResult<Project> {
            nur_lesen()
        }
        async fn dokument(
            &self,
            _: DokumentCommit,
            _: NeuerProtokolleintrag,
        ) -> CoreResult<DocumentVersion> {
            nur_lesen()
        }
        async fn einreichung(
            &self,
            _: crate::einreichung::Einreichung,
            _: Option<i64>,
            _: NeuerProtokolleintrag,
        ) -> CoreResult<crate::einreichung::Einreichung> {
            nur_lesen()
        }
        async fn fachstand(
            &self,
            _: Fachstand,
            _: i64,
            _: NeuerProtokolleintrag,
        ) -> CoreResult<Fachstand> {
            nur_lesen()
        }
    }
    fn sicht(quelle: Arc<Quelle>, p: ProjectId) -> Lesesitzung {
        Lesesitzung {
            projekt: Project {
                id: p,
                name: "Test".into(),
                bundesland: crate::domain::Bundesland::Nw,
                grundstuecksflaeche_m2: Some(500.),
                version: 1,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                planungsrecht: Default::default(),
            },
            dokumente: vec![],
            commit: quelle,
            quelle: None,
            fach: Mutex::new(BTreeMap::new()),
        }
    }
    #[tokio::test]
    async fn eine_quellenabfrage_pro_modul_und_auswertung_auch_parallel() {
        let quelle = Arc::new(Quelle(AtomicUsize::new(0)));
        let p = ProjectId::new();
        let a = sicht(quelle.clone(), p);
        let (x, y) = tokio::join!(
            a.fachobjekt(p, "planaufmass", "quelle"),
            a.fachstaende(p, "planaufmass")
        );
        assert_eq!(x.unwrap().unwrap().revision, y.unwrap()[0].revision);
        for _ in 0..100 {
            a.fachobjekt(p, "planaufmass", "quelle").await.unwrap();
        }
        assert_eq!(quelle.0.load(Ordering::SeqCst), 1);
        let b = sicht(quelle.clone(), p);
        assert_eq!(
            b.fachobjekt(p, "planaufmass", "quelle")
                .await
                .unwrap()
                .unwrap()
                .revision,
            2
        );
        assert!(
            a.fachstaende(ProjectId::new(), "planaufmass")
                .await
                .is_err()
        );
        assert_eq!(quelle.0.load(Ordering::SeqCst), 2);
    }
}
