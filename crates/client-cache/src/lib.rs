//! Intelligenter Client-Cache für große Dateien (CAD-Pläne, PDFs).
//!
//! [`CachedStorage`] ist ein Decorator um ein beliebiges entferntes
//! [`Storage`] (NAS, S3, später der axum-Server). Er implementiert selbst
//! `Storage`, sodass core-lib nichts vom Cache weiß.
//!
//! * **Inhaltsadressiert:** Dateien liegen als `blobs/<sha[0..2]>/<sha256>`.
//!   Gleiche Inhalte werden nur einmal gespeichert, und ein Blob kann nie
//!   "veralten" – ändert sich eine Datei, ändert sich ihr Hash.
//! * **Hash-Abgleich:** Jeder Download wird gegen den erwarteten SHA-256
//!   geprüft, bevor er in den Cache kommt. Bereits vorhandene Blobs werden
//!   beim ersten Zugriff nach App-Start erneut gehasht, sodass beschädigte
//!   Dateien erkannt und neu geladen werden.
//! * **Kein Netzwerk bei Treffern:** `load_verified` (Hash aus der
//!   Dokumenthistorie) beantwortet Treffer komplett lokal, auch offline.
//! * **LRU-Begrenzung:** Überschreitet der Cache `max_bytes`, werden die am
//!   längsten nicht genutzten Blobs gelöscht.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use core_lib::storage::{ContentHash, ObjectMeta, Storage, StorageKey};
use core_lib::{CoreError, CoreResult};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub dir: PathBuf,
    /// Obergrenze für den belegten Plattenplatz.
    pub max_bytes: u64,
    /// Kleinere Dateien werden nicht gecacht – dort überwiegt der Overhead.
    pub min_file_bytes: u64,
}

impl CacheConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_bytes: 10 * 1024 * 1024 * 1024,
            min_file_bytes: 256 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    size: u64,
    last_access: u64,
    /// In dieser Sitzung bereits gegen den Hash geprüft?
    verified: bool,
}

#[derive(Default)]
struct Index {
    entries: HashMap<ContentHash, Entry>,
    total_bytes: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub total_bytes: u64,
}

pub struct CachedStorage {
    inner: Arc<dyn Storage>,
    config: CacheConfig,
    index: Mutex<Index>,
    clock: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
}

fn io_err(path: &Path, e: std::io::Error) -> CoreError {
    CoreError::Storage(format!("Cache {}: {e}", path.display()))
}

impl CachedStorage {
    /// Öffnet den Cache und baut den Index aus den vorhandenen Blobs auf.
    pub async fn open(inner: Arc<dyn Storage>, config: CacheConfig) -> CoreResult<Self> {
        let blobs = config.dir.join("blobs");
        let tmp = config.dir.join("tmp");
        for dir in [&blobs, &tmp] {
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|e| io_err(dir, e))?;
        }
        // Reste abgebrochener Downloads entfernen.
        if let Ok(mut rd) = tokio::fs::read_dir(&tmp).await {
            while let Ok(Some(entry)) = rd.next_entry().await {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }

        let scan_dir = blobs.clone();
        let found = tokio::task::spawn_blocking(move || scan_blobs(&scan_dir))
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .map_err(|e| io_err(&blobs, e))?;

        let mut index = Index::default();
        for (hash, size, mtime) in found {
            index.total_bytes += size;
            index.entries.insert(
                hash,
                Entry {
                    size,
                    last_access: mtime,
                    verified: false,
                },
            );
        }
        let clock = index
            .entries
            .values()
            .map(|e| e.last_access)
            .max()
            .unwrap_or(0);
        tracing::info!(
            entries = index.entries.len(),
            bytes = index.total_bytes,
            backend = %inner.backend_name(),
            "Client-Cache geöffnet"
        );
        Ok(Self {
            inner,
            config,
            index: Mutex::new(index),
            clock: AtomicU64::new(clock.max(unix_now())),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        })
    }

    pub async fn stats(&self) -> CacheStats {
        let index = self.index.lock().await;
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            entries: index.entries.len(),
            total_bytes: index.total_bytes,
        }
    }

    fn blob_path(&self, hash: &ContentHash) -> PathBuf {
        let h = hash.as_str();
        self.config.dir.join("blobs").join(&h[..2]).join(h)
    }

    fn tick(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Liefert den Blob, falls vorhanden und (ggf. nach erneuter Prüfung) gültig.
    async fn lookup(&self, hash: &ContentHash) -> Option<Bytes> {
        let needs_verify = {
            let mut index = self.index.lock().await;
            let now = self.tick();
            let entry = index.entries.get_mut(hash)?;
            entry.last_access = now;
            !entry.verified
        };
        let path = self.blob_path(hash);
        let data = match tokio::fs::read(&path).await {
            Ok(data) => Bytes::from(data),
            Err(_) => {
                self.forget(hash).await;
                return None;
            }
        };
        if needs_verify {
            let (data, ok) = {
                let expected = hash.clone();
                tokio::task::spawn_blocking(move || {
                    let ok = expected.verify(&data).is_ok();
                    (data, ok)
                })
                .await
                .ok()?
            };
            if !ok {
                tracing::warn!(%hash, "beschädigter Cache-Eintrag verworfen");
                let _ = tokio::fs::remove_file(&path).await;
                self.forget(hash).await;
                return None;
            }
            if let Some(entry) = self.index.lock().await.entries.get_mut(hash) {
                entry.verified = true;
            }
            return Some(data);
        }
        Some(data)
    }

    async fn forget(&self, hash: &ContentHash) {
        let mut index = self.index.lock().await;
        if let Some(entry) = index.entries.remove(hash) {
            index.total_bytes -= entry.size;
        }
    }

    /// Legt bereits verifizierte Daten im Cache ab. Fehler beim Cachen sind
    /// nie fatal – die Daten werden trotzdem an den Aufrufer geliefert.
    async fn insert(&self, hash: &ContentHash, data: &Bytes) {
        let size = data.len() as u64;
        if size < self.config.min_file_bytes || size > self.config.max_bytes {
            return;
        }
        if self.index.lock().await.entries.contains_key(hash) {
            return;
        }
        if let Err(e) = self.write_blob(hash, data).await {
            tracing::warn!(%hash, error = %e, "Datei konnte nicht gecacht werden");
            return;
        }
        let mut index = self.index.lock().await;
        let now = self.tick();
        if index
            .entries
            .insert(
                hash.clone(),
                Entry {
                    size,
                    last_access: now,
                    verified: true,
                },
            )
            .is_none()
        {
            index.total_bytes += size;
        }
        let evicted = evict(&mut index, self.config.max_bytes, hash);
        drop(index);
        for victim in evicted {
            let _ = tokio::fs::remove_file(self.blob_path(&victim)).await;
        }
    }

    async fn write_blob(&self, hash: &ContentHash, data: &[u8]) -> CoreResult<()> {
        let target = self.blob_path(hash);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| io_err(parent, e))?;
        }
        let tmp = self
            .config
            .dir
            .join("tmp")
            .join(uuid::Uuid::new_v4().to_string());
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| io_err(&tmp, e))?;
        file.write_all(data).await.map_err(|e| io_err(&tmp, e))?;
        file.flush().await.map_err(|e| io_err(&tmp, e))?;
        drop(file);
        tokio::fs::rename(&tmp, &target).await.map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            io_err(&target, e)
        })
    }

    async fn fetch_and_cache(&self, key: &StorageKey, expected: &ContentHash) -> CoreResult<Bytes> {
        self.misses.fetch_add(1, Ordering::Relaxed);
        let data = self.inner.load(key).await?;
        let (data, check) = {
            let expected = expected.clone();
            tokio::task::spawn_blocking(move || {
                let check = expected.verify(&data);
                (data, check)
            })
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))?
        };
        check?;
        self.insert(expected, &data).await;
        Ok(data)
    }
}

/// Entfernt LRU-Einträge, bis die Obergrenze eingehalten ist. `keep` (der
/// gerade eingefügte Eintrag) wird nie entfernt.
fn evict(index: &mut Index, max_bytes: u64, keep: &ContentHash) -> Vec<ContentHash> {
    let mut victims = Vec::new();
    if index.total_bytes <= max_bytes {
        return victims;
    }
    let mut candidates: Vec<(u64, ContentHash, u64)> = index
        .entries
        .iter()
        .filter(|(h, _)| *h != keep)
        .map(|(h, e)| (e.last_access, h.clone(), e.size))
        .collect();
    candidates.sort_by_key(|(t, _, _)| *t);
    for (_, hash, size) in candidates {
        if index.total_bytes <= max_bytes {
            break;
        }
        index.entries.remove(&hash);
        index.total_bytes -= size;
        victims.push(hash);
    }
    victims
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn scan_blobs(dir: &Path) -> std::io::Result<Vec<(ContentHash, u64, u64)>> {
    let mut found = Vec::new();
    for shard in std::fs::read_dir(dir)? {
        let shard = shard?;
        if !shard.file_type()?.is_dir() {
            continue;
        }
        for blob in std::fs::read_dir(shard.path())? {
            let blob = blob?;
            let meta = blob.metadata()?;
            let Ok(hash) = ContentHash::try_from(blob.file_name().to_string_lossy().into_owned())
            else {
                let _ = std::fs::remove_file(blob.path());
                continue;
            };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            found.push((hash, meta.len(), mtime));
        }
    }
    Ok(found)
}

#[async_trait]
impl Storage for CachedStorage {
    fn backend_name(&self) -> String {
        format!("cached({})", self.inner.backend_name())
    }

    /// Write-through: erst entfernt speichern, dann lokal cachen, damit der
    /// eigene Upload sofort ohne Download wieder geöffnet werden kann.
    async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta> {
        let meta = self.inner.save(key, data.clone()).await?;
        self.insert(&meta.sha256, &data).await;
        Ok(meta)
    }

    /// Ohne bekannten Hash: kurze `head`-Anfrage, dann ggf. Treffer aus dem Cache.
    async fn load(&self, key: &StorageKey) -> CoreResult<Bytes> {
        let meta = self.inner.head(key).await?;
        self.load_verified(key, &meta.sha256).await
    }

    async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta> {
        self.inner.head(key).await
    }

    async fn delete(&self, key: &StorageKey) -> CoreResult<()> {
        // Blobs sind inhaltsadressiert und evtl. von anderen Schlüsseln
        // referenziert; sie verlassen den Cache über die LRU-Begrenzung.
        self.inner.delete(key).await
    }

    async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>> {
        self.inner.list(prefix).await
    }

    async fn load_verified(&self, key: &StorageKey, expected: &ContentHash) -> CoreResult<Bytes> {
        if let Some(data) = self.lookup(expected).await {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(data);
        }
        self.fetch_and_cache(key, expected).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use storage::LocalStorage;

    /// Zählt Downloads, um Cache-Treffer nachzuweisen.
    struct Counting {
        inner: LocalStorage,
        loads: AtomicUsize,
    }

    #[async_trait]
    impl Storage for Counting {
        fn backend_name(&self) -> String {
            "counting".into()
        }
        async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta> {
            self.inner.save(key, data).await
        }
        async fn load(&self, key: &StorageKey) -> CoreResult<Bytes> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            self.inner.load(key).await
        }
        async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta> {
            self.inner.head(key).await
        }
        async fn delete(&self, key: &StorageKey) -> CoreResult<()> {
            self.inner.delete(key).await
        }
        async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>> {
            self.inner.list(prefix).await
        }
    }

    struct Fixture {
        _remote_dir: tempfile::TempDir,
        cache_dir: tempfile::TempDir,
        remote: Arc<Counting>,
    }

    impl Fixture {
        fn new() -> Self {
            let remote_dir = tempfile::tempdir().unwrap();
            let remote = Arc::new(Counting {
                inner: LocalStorage::new(remote_dir.path().to_path_buf(), "nas").unwrap(),
                loads: AtomicUsize::new(0),
            });
            Self {
                _remote_dir: remote_dir,
                cache_dir: tempfile::tempdir().unwrap(),
                remote,
            }
        }

        async fn cache(&self, max_bytes: u64) -> CachedStorage {
            let config = CacheConfig {
                dir: self.cache_dir.path().to_path_buf(),
                max_bytes,
                min_file_bytes: 0,
            };
            CachedStorage::open(self.remote.clone(), config)
                .await
                .unwrap()
        }

        fn loads(&self) -> usize {
            self.remote.loads.load(Ordering::SeqCst)
        }
    }

    fn key(s: &str) -> StorageKey {
        StorageKey::new(s).unwrap()
    }

    #[tokio::test]
    async fn treffer_ohne_download() {
        let fx = Fixture::new();
        let meta = fx
            .remote
            .save(&key("a/plan.dxf"), Bytes::from(vec![7u8; 1000]))
            .await
            .unwrap();
        let cache = fx.cache(u64::MAX).await;

        cache.load_verified(&meta.key, &meta.sha256).await.unwrap();
        cache.load_verified(&meta.key, &meta.sha256).await.unwrap();
        cache.load(&meta.key).await.unwrap();
        assert_eq!(fx.loads(), 1);
        assert_eq!(cache.stats().await.hits, 2);

        // Cache überlebt einen Neustart.
        drop(cache);
        let cache = fx.cache(u64::MAX).await;
        cache.load_verified(&meta.key, &meta.sha256).await.unwrap();
        assert_eq!(fx.loads(), 1);
    }

    #[tokio::test]
    async fn beschaedigter_blob_wird_neu_geladen() {
        let fx = Fixture::new();
        let data = Bytes::from(vec![1u8; 500]);
        let meta = fx
            .remote
            .save(&key("b/antrag.pdf"), data.clone())
            .await
            .unwrap();
        let cache = fx.cache(u64::MAX).await;
        cache.load_verified(&meta.key, &meta.sha256).await.unwrap();
        let blob = cache.blob_path(&meta.sha256);
        drop(cache);

        std::fs::write(&blob, b"manipuliert").unwrap();
        let cache = fx.cache(u64::MAX).await;
        assert_eq!(
            cache.load_verified(&meta.key, &meta.sha256).await.unwrap(),
            data
        );
        assert_eq!(fx.loads(), 2);
    }

    #[tokio::test]
    async fn falscher_hash_vom_server_wird_abgelehnt() {
        let fx = Fixture::new();
        let meta = fx
            .remote
            .save(&key("c/x.pdf"), Bytes::from_static(b"echt"))
            .await
            .unwrap();
        let cache = fx.cache(u64::MAX).await;
        let falsch = ContentHash::of(b"etwas anderes");
        assert!(matches!(
            cache.load_verified(&meta.key, &falsch).await,
            Err(CoreError::HashMismatch { .. })
        ));
        assert_eq!(cache.stats().await.entries, 0);
    }

    #[tokio::test]
    async fn lru_verdraengung() {
        let fx = Fixture::new();
        let cache = fx.cache(2500).await;
        let mut metas = Vec::new();
        for i in 0..3u8 {
            let meta = cache
                .save(&key(&format!("d/{i}.dxf")), Bytes::from(vec![i; 1000]))
                .await
                .unwrap();
            metas.push(meta);
        }
        let stats = cache.stats().await;
        assert_eq!(stats.entries, 2);
        assert!(stats.total_bytes <= 2500);
        // Der älteste Eintrag (0) wurde verdrängt und muss neu geladen werden.
        cache
            .load_verified(&metas[0].key, &metas[0].sha256)
            .await
            .unwrap();
        assert_eq!(fx.loads(), 1);
        cache
            .load_verified(&metas[2].key, &metas[2].sha256)
            .await
            .unwrap();
        assert_eq!(fx.loads(), 1);
    }
}
