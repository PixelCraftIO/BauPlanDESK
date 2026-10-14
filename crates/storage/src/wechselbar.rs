//! Kontrollierter Wechsel der lokalen DMS-Ablage. Alle Storage-Aufrufe behalten
//! denselben Vertrag; während der Kopie werden neue Dateioperationen serialisiert.
use crate::LocalStorage;
use async_trait::async_trait;
use bytes::Bytes;
use core_lib::{
    CoreError, CoreResult,
    storage::{ContentHash, ObjectMeta, Storage, StorageKey},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
pub struct WechselbareAblage {
    aktiv: RwLock<(PathBuf, Arc<dyn Storage>)>,
}
impl WechselbareAblage {
    pub fn new(root: PathBuf) -> CoreResult<Self> {
        let storage = Arc::new(LocalStorage::new(root.clone(), "local")?);
        Ok(Self {
            aktiv: RwLock::new((root, storage)),
        })
    }
    pub async fn root(&self) -> PathBuf {
        self.aktiv.read().await.0.clone()
    }
    /// Übernimmt erst nach vollständiger Hash-Prüfung und erfolgreichem Speichern
    /// der Konfiguration. Die bisherige Ablage wird niemals gelöscht.
    pub async fn wechseln(
        &self,
        ziel: &Path,
        konfiguration: impl FnOnce(&Path) -> CoreResult<()>,
    ) -> CoreResult<usize> {
        if !ziel.is_absolute() {
            return Err(CoreError::InvalidInput(
                "Bitte einen absoluten Ablagepfad wählen".into(),
            ));
        }
        let mut aktiv = self.aktiv.write().await;
        let next = Arc::new(LocalStorage::new(ziel.to_path_buf(), "local")?);
        let old = std::fs::canonicalize(&aktiv.0).map_err(|e| CoreError::Storage(e.to_string()))?;
        let dest = std::fs::canonicalize(ziel).map_err(|e| CoreError::Storage(e.to_string()))?;
        if old == dest {
            return Ok(0);
        }
        if dest.starts_with(&old) || old.starts_with(&dest) {
            return Err(CoreError::InvalidInput(
                "Neue und bisherige DMS-Ablage dürfen nicht ineinander liegen".into(),
            ));
        }
        let source = aktiv.1.list(None).await?;
        let existing = next.list(None).await?;
        for item in existing {
            if !source
                .iter()
                .any(|s| s.key == item.key && s.sha256 == item.sha256)
            {
                return Err(CoreError::InvalidInput(
                    "Ziel enthält eine andere DMS-Ablage. Einen eigenen oder leeren Ordner wählen."
                        .into(),
                ));
            }
        }
        for item in &source {
            let data = aktiv.1.load_verified(&item.key, &item.sha256).await?;
            let result = next.save(&item.key, data).await?;
            if result.sha256 != item.sha256 {
                return Err(CoreError::Storage(
                    "Hash der kopierten Datei stimmt nicht überein".into(),
                ));
            }
            next.load_verified(&item.key, &item.sha256).await?;
        }
        konfiguration(&dest)?;
        *aktiv = (dest, next);
        Ok(source.len())
    }
}
#[async_trait]
impl Storage for WechselbareAblage {
    fn backend_name(&self) -> String {
        "local".into()
    }
    async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta> {
        self.aktiv.read().await.1.save(key, data).await
    }
    async fn load(&self, key: &StorageKey) -> CoreResult<Bytes> {
        self.aktiv.read().await.1.load(key).await
    }
    async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta> {
        self.aktiv.read().await.1.head(key).await
    }
    async fn delete(&self, key: &StorageKey) -> CoreResult<()> {
        self.aktiv.read().await.1.delete(key).await
    }
    async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>> {
        self.aktiv.read().await.1.list(prefix).await
    }
    async fn load_verified(&self, key: &StorageKey, expected: &ContentHash) -> CoreResult<Bytes> {
        self.aktiv.read().await.1.load_verified(key, expected).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn kopiert_versionen_und_erhaelt_alte_ablage() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("alt");
        let target = temp.path().join("neu");
        let a = WechselbareAblage::new(old.clone()).unwrap();
        let key = StorageKey::new("projekte/p/dokumente/v1.pdf").unwrap();
        let meta = a.save(&key, Bytes::from_static(b"Original")).await.unwrap();
        assert_eq!(a.wechseln(&target, |_| Ok(())).await.unwrap(), 1);
        assert_eq!(
            a.load_verified(&key, &meta.sha256).await.unwrap(),
            "Original"
        );
        let backup = LocalStorage::new(old, "local").unwrap();
        assert_eq!(backup.load(&key).await.unwrap(), "Original");
    }
    #[tokio::test]
    async fn fehlgeschlagene_konfiguration_schaltet_nicht_um() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("alt");
        let a = WechselbareAblage::new(old.clone()).unwrap();
        assert!(
            a.wechseln(&temp.path().join("neu"), |_| Err(CoreError::Storage(
                "Testfehler".into()
            )))
            .await
            .is_err()
        );
        assert_eq!(a.root().await, old);
        assert!(
            a.wechseln(&old.join("unterordner"), |_| Ok(()))
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn fremde_ablage_wird_nicht_vermischt() {
        let temp = tempfile::tempdir().unwrap();
        let a = WechselbareAblage::new(temp.path().join("alt")).unwrap();
        let target = temp.path().join("neu");
        LocalStorage::new(target.clone(), "local")
            .unwrap()
            .save(
                &StorageKey::new("fremd.pdf").unwrap(),
                Bytes::from_static(b"Fremd"),
            )
            .await
            .unwrap();
        assert!(a.wechseln(&target, |_| Ok(())).await.is_err());
    }
}
