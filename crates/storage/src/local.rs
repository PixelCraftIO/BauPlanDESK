use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use core_lib::storage::{ContentHash, ObjectMeta, Storage, StorageKey};
use core_lib::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

/// Dateisystem-Backend für lokale Verzeichnisse und NAS-Freigaben.
///
/// Layout unterhalb von `root`:
/// * `data/<key>`       – Inhalt
/// * `meta/<key>.json`  – Größe und SHA-256
/// * `tmp/`             – Zwischendateien für atomares Schreiben
pub struct LocalStorage {
    root: PathBuf,
    name: &'static str,
}

#[derive(Serialize, Deserialize)]
struct Sidecar {
    size_bytes: u64,
    sha256: ContentHash,
}

fn io_err(context: &Path, e: std::io::Error) -> CoreError {
    if e.kind() == std::io::ErrorKind::NotFound {
        CoreError::NotFound(context.display().to_string())
    } else {
        CoreError::Storage(format!("{}: {e}", context.display()))
    }
}

impl LocalStorage {
    pub fn new(root: PathBuf, name: &'static str) -> CoreResult<Self> {
        for sub in ["data", "meta", "tmp"] {
            let dir = root.join(sub);
            std::fs::create_dir_all(&dir).map_err(|e| io_err(&dir, e))?;
        }
        Ok(Self { root, name })
    }

    fn data_path(&self, key: &StorageKey) -> PathBuf {
        key.segments()
            .fold(self.root.join("data"), |p, s| p.join(s))
    }

    fn meta_path(&self, key: &StorageKey) -> PathBuf {
        let mut path = key
            .segments()
            .fold(self.root.join("meta"), |p, s| p.join(s));
        path.as_mut_os_string().push(".json");
        path
    }

    async fn write_atomic(&self, target: &Path, data: &[u8]) -> CoreResult<()> {
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| io_err(parent, e))?;
        }
        let tmp = self.root.join("tmp").join(uuid::Uuid::new_v4().to_string());
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| io_err(&tmp, e))?;
        file.write_all(data).await.map_err(|e| io_err(&tmp, e))?;
        file.sync_all().await.map_err(|e| io_err(&tmp, e))?;
        drop(file);
        if let Err(e) = tokio::fs::rename(&tmp, target).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(io_err(target, e));
        }
        Ok(())
    }

    fn collect_keys(dir: &Path, prefix: &str, out: &mut Vec<String>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let key = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type()?.is_dir() {
                Self::collect_keys(&entry.path(), &key, out)?;
            } else {
                out.push(key);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Storage for LocalStorage {
    fn backend_name(&self) -> String {
        self.name.to_owned()
    }

    async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta> {
        let (data, sha256) = tokio::task::spawn_blocking(move || {
            let hash = ContentHash::of(&data);
            (data, hash)
        })
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))?;
        let meta = ObjectMeta {
            key: key.clone(),
            size_bytes: data.len() as u64,
            sha256,
        };
        let sidecar = serde_json::to_vec(&Sidecar {
            size_bytes: meta.size_bytes,
            sha256: meta.sha256.clone(),
        })
        .map_err(|e| CoreError::Storage(e.to_string()))?;
        self.write_atomic(&self.data_path(key), &data).await?;
        self.write_atomic(&self.meta_path(key), &sidecar).await?;
        Ok(meta)
    }

    async fn load(&self, key: &StorageKey) -> CoreResult<Bytes> {
        let path = self.data_path(key);
        tokio::fs::read(&path)
            .await
            .map(Bytes::from)
            .map_err(|e| io_err(&path, e))
    }

    async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta> {
        let path = self.meta_path(key);
        let raw = tokio::fs::read(&path).await.map_err(|e| io_err(&path, e))?;
        let sidecar: Sidecar =
            serde_json::from_slice(&raw).map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(ObjectMeta {
            key: key.clone(),
            size_bytes: sidecar.size_bytes,
            sha256: sidecar.sha256,
        })
    }

    async fn delete(&self, key: &StorageKey) -> CoreResult<()> {
        for path in [self.data_path(key), self.meta_path(key)] {
            match tokio::fs::remove_file(&path).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_err(&path, e)),
            }
        }
        Ok(())
    }

    async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>> {
        let (dir, base) = match prefix {
            Some(p) => (self.data_path(p), p.to_string()),
            None => (self.root.join("data"), String::new()),
        };
        let keys = tokio::task::spawn_blocking(move || {
            let mut keys = Vec::new();
            match Self::collect_keys(&dir, &base, &mut keys) {
                Ok(()) => Ok(keys),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(keys),
                Err(e) => Err(io_err(&dir, e)),
            }
        })
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
        let mut result = Vec::with_capacity(keys.len());
        for key in keys {
            result.push(self.head(&StorageKey::new(key)?).await?);
        }
        result.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn save_load_head_list_delete() {
        let dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(dir.path().to_path_buf(), "local").unwrap();
        let key = StorageKey::new("projekte/p1/plan.dxf").unwrap();
        let data = Bytes::from_static(b"0\nSECTION\n");

        let meta = storage.save(&key, data.clone()).await.unwrap();
        assert_eq!(meta.sha256, ContentHash::of(&data));
        assert_eq!(storage.load(&key).await.unwrap(), data);
        assert_eq!(storage.head(&key).await.unwrap(), meta);
        assert_eq!(
            storage.load_verified(&key, &meta.sha256).await.unwrap(),
            data
        );

        let prefix = StorageKey::new("projekte").unwrap();
        assert_eq!(storage.list(Some(&prefix)).await.unwrap(), vec![meta]);

        storage.delete(&key).await.unwrap();
        assert!(matches!(
            storage.load(&key).await,
            Err(CoreError::NotFound(_))
        ));
    }
}
