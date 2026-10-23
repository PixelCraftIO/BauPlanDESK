//! Regression: vollständiger Versionsbestand und aktive Ablage sind verschiedene Sichten.
use async_trait::async_trait;
use bytes::Bytes;
use core_lib::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, BenutzerId},
    domain::{Bundesland, DocumentId, DocumentLock, DocumentVersion, NewProject, ProjectId},
    lbo::LboEngine,
    license::DevelopmentLicense,
    repository::DocumentRepository,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Ändert die Quelle genau nach dem Einlesen des vollständigen Bestands.
/// Dadurch ist die Konkurrenzprüfung deterministisch, ohne Zeitsteuerung.
struct Dokumentquelle {
    inner: Arc<dyn DocumentRepository>,
    aendern: AtomicBool,
    entfernen: bool,
}
#[async_trait]
impl DocumentRepository for Dokumentquelle {
    async fn latest(&self, id: DocumentId) -> CoreResult<DocumentVersion> {
        self.inner.latest(id).await
    }
    async fn history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>> {
        self.inner.history(id).await
    }
    async fn active_for_project(&self, p: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        self.inner.active_for_project(p).await
    }
    async fn list_for_project(&self, p: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        let docs = self.inner.list_for_project(p).await?;
        if self.aendern.swap(false, Ordering::SeqCst) {
            let d = docs[0].clone();
            if self.entfernen {
                self.inner
                    .remove_from_project(d.document_id, d.version)
                    .await?;
            } else {
                let basis = d.version;
                self.inner.add_version(d, Some(basis)).await?;
            }
        }
        Ok(docs)
    }
    async fn add_version(&self, d: DocumentVersion, b: Option<i64>) -> CoreResult<DocumentVersion> {
        self.inner.add_version(d, b).await
    }
    async fn lock(&self, id: DocumentId, p: ProjectId, wer: &Benutzer) -> CoreResult<DocumentLock> {
        self.inner.lock(id, p, wer).await
    }
    async fn unlock(
        &self,
        id: DocumentId,
        wer: Option<BenutzerId>,
    ) -> CoreResult<Option<DocumentLock>> {
        self.inner.unlock(id, wer).await
    }
    async fn get_lock(&self, id: DocumentId) -> CoreResult<Option<DocumentLock>> {
        self.inner.get_lock(id).await
    }
    async fn locks_for_project(&self, p: ProjectId) -> CoreResult<Vec<DocumentLock>> {
        self.inner.locks_for_project(p).await
    }
}

async fn umgebung() -> (
    CoreServices,
    ProjectId,
    DocumentVersion,
    Benutzer,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let db = db_sqlite::SqliteDatabase::in_memory().await.unwrap();
    let core = CoreServices::new(
        db.repositories(),
        Arc::new(storage::LocalStorage::new(dir.path().to_path_buf(), "test").unwrap()),
        Arc::new(LboEngine::new()),
        Arc::new(DevelopmentLicense),
    );
    let user = core.anmelden("planer", "Planer").await.unwrap();
    let p = core
        .create_project(
            NewProject {
                name: "Quellenprüfung".into(),
                bundesland: Bundesland::By,
                grundstuecksflaeche_m2: None,
                planungsrecht: Default::default(),
            },
            &user,
        )
        .await
        .unwrap();
    let d = core
        .store_document(
            p.id,
            None,
            None,
            "Plan.pdf",
            Bytes::from_static(b"test"),
            &user,
        )
        .await
        .unwrap();
    (core, p.id, d, user, dir)
}

#[tokio::test]
async fn entfernte_datei_verursacht_keinen_dauerhaften_quellenkonflikt() {
    let (core, p, d, user, _dir) = umgebung().await;
    // Mehrere Versionen müssen unabhängig von der Sortierung gleich verglichen werden.
    core.store_document(
        p,
        Some(d.document_id),
        Some(1),
        "Plan.pdf",
        Bytes::from_static(b"neu"),
        &user,
    )
    .await
    .unwrap();
    core.dokument_entfernen(d.document_id, 2, &user)
        .await
        .unwrap();
    assert!(core.list_documents(p).await.unwrap().is_empty());
    assert_eq!(core.document_history(d.document_id).await.unwrap().len(), 2);
    for _ in 0..3 {
        assert!(core.nachweise(p).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn echte_quellenaenderungen_waehrend_der_pruefung_bleiben_fehler() {
    for entfernen in [false, true] {
        let (mut core, p, _, _, _dir) = umgebung().await;
        core.repos.documents = Arc::new(Dokumentquelle {
            inner: core.repos.documents.clone(),
            aendern: AtomicBool::new(true),
            entfernen,
        });
        assert!(
            matches!(core.nachweise(p).await, Err(CoreError::InvalidInput(m)) if m.contains("Quellen während"))
        );
        // Nach Abschluss der Änderung ist eine neue Auswertung wieder möglich.
        assert!(core.nachweise(p).await.unwrap().is_empty());
    }
}
