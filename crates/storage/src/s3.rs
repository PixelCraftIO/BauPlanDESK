use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use core_lib::storage::{ContentHash, ObjectMeta, Storage, StorageKey};
use core_lib::{CoreError, CoreResult};
use futures::{StreamExt, TryStreamExt};
use object_store::aws::AmazonS3Builder;
use object_store::path::Path;
use object_store::{
    Attribute, Attributes, GetOptions, ObjectStore, ObjectStoreExt, PutOptions, PutPayload,
};
use serde::{Deserialize, Serialize};

const SHA256_ATTRIBUTE: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    /// Für S3-kompatible Anbieter (MinIO, Hetzner, …); leer = AWS.
    pub endpoint: Option<String>,
    /// Leer = Zugangsdaten aus der Umgebung (`AWS_ACCESS_KEY_ID` …).
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    /// Optionales Präfix, z. B. pro Mandant in der Cloud-Variante.
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub allow_http: bool,
}

pub struct S3Storage {
    store: Arc<dyn ObjectStore>,
    prefix: Option<String>,
}

fn s3_err(e: object_store::Error) -> CoreError {
    match e {
        object_store::Error::NotFound { path, .. } => CoreError::NotFound(path),
        other => CoreError::Storage(other.to_string()),
    }
}

impl S3Storage {
    pub fn new(cfg: &S3Config) -> CoreResult<Self> {
        let mut builder = AmazonS3Builder::from_env()
            .with_bucket_name(&cfg.bucket)
            .with_region(&cfg.region)
            .with_allow_http(cfg.allow_http);
        if let Some(endpoint) = &cfg.endpoint {
            builder = builder.with_endpoint(endpoint);
        }
        if let (Some(id), Some(secret)) = (&cfg.access_key_id, &cfg.secret_access_key) {
            builder = builder
                .with_access_key_id(id)
                .with_secret_access_key(secret);
        }
        let store = builder.build().map_err(s3_err)?;
        Ok(Self::from_store(Arc::new(store), cfg.prefix.clone()))
    }

    /// Für Tests oder andere `object_store`-Backends (Azure, GCS, In-Memory).
    pub fn from_store(store: Arc<dyn ObjectStore>, prefix: Option<String>) -> Self {
        let prefix = prefix
            .map(|p| p.trim_matches('/').to_owned())
            .filter(|p| !p.is_empty());
        Self { store, prefix }
    }

    fn path(&self, key: &StorageKey) -> Path {
        match &self.prefix {
            Some(prefix) => Path::from(format!("{prefix}/{key}")),
            None => Path::from(key.as_str()),
        }
    }

    fn key_of(&self, path: &Path) -> CoreResult<StorageKey> {
        let raw = path.as_ref();
        let raw = match &self.prefix {
            Some(prefix) => raw
                .strip_prefix(prefix.as_str())
                .and_then(|r| r.strip_prefix('/'))
                .unwrap_or(raw),
            None => raw,
        };
        StorageKey::new(raw)
    }
}

#[async_trait]
impl Storage for S3Storage {
    fn backend_name(&self) -> String {
        "s3".into()
    }

    async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta> {
        let sha256 = ContentHash::of(&data);
        let size_bytes = data.len() as u64;
        let mut attributes = Attributes::new();
        attributes.insert(
            Attribute::Metadata(SHA256_ATTRIBUTE.into()),
            sha256.to_string().into(),
        );
        let options = PutOptions {
            attributes,
            ..Default::default()
        };
        // S3-PUT ist atomar: das Objekt ist erst nach vollständigem Upload sichtbar.
        self.store
            .put_opts(&self.path(key), PutPayload::from_bytes(data), options)
            .await
            .map_err(s3_err)?;
        Ok(ObjectMeta {
            key: key.clone(),
            size_bytes,
            sha256,
        })
    }

    async fn load(&self, key: &StorageKey) -> CoreResult<Bytes> {
        self.store
            .get(&self.path(key))
            .await
            .map_err(s3_err)?
            .bytes()
            .await
            .map_err(s3_err)
    }

    async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta> {
        let options = GetOptions {
            head: true,
            ..Default::default()
        };
        let result = self
            .store
            .get_opts(&self.path(key), options)
            .await
            .map_err(s3_err)?;
        let sha256 = result
            .attributes
            .get(&Attribute::Metadata(SHA256_ATTRIBUTE.into()))
            .ok_or_else(|| CoreError::Storage(format!("{key}: SHA-256-Metadaten fehlen")))?;
        Ok(ObjectMeta {
            key: key.clone(),
            size_bytes: result.meta.size,
            sha256: ContentHash::try_from(sha256.to_string())?,
        })
    }

    async fn delete(&self, key: &StorageKey) -> CoreResult<()> {
        match self.store.delete(&self.path(key)).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(s3_err(e)),
        }
    }

    async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>> {
        let list_prefix = match (prefix, &self.prefix) {
            (Some(p), _) => Some(self.path(p)),
            (None, Some(base)) => Some(Path::from(base.as_str())),
            (None, None) => None,
        };
        let keys: Vec<StorageKey> = self
            .store
            .list(list_prefix.as_ref())
            .map_err(s3_err)
            .and_then(|meta| async move { self.key_of(&meta.location) })
            .try_collect()
            .await?;
        // S3-LIST liefert keine Benutzer-Metadaten; daher HEAD je Objekt (parallel).
        let mut result: Vec<ObjectMeta> = futures::stream::iter(keys)
            .map(|key| async move { self.head(&key).await })
            .buffer_unordered(16)
            .try_collect()
            .await?;
        result.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::memory::InMemory;

    #[tokio::test]
    async fn roundtrip_mit_praefix() {
        let storage = S3Storage::from_store(Arc::new(InMemory::new()), Some("mandant-1/".into()));
        let key = StorageKey::new("projekte/p1/antrag.pdf").unwrap();
        let data = Bytes::from_static(b"%PDF-1.7");
        let meta = storage.save(&key, data.clone()).await.unwrap();
        assert_eq!(storage.head(&key).await.unwrap(), meta);
        assert_eq!(
            storage.load_verified(&key, &meta.sha256).await.unwrap(),
            data
        );
        assert_eq!(storage.list(None).await.unwrap(), vec![meta]);
    }
}
