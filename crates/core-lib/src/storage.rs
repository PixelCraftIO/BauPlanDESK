//! Storage-agnostisches Dateisystem-Interface für das DMS.
//!
//! `storage.save(...)` legt Dateien je nach Konfiguration lokal, auf einem
//! NAS-Laufwerk oder in einem S3-kompatiblen Speicher ab. Die konkreten
//! Backends liegen im Crate `storage`, der Client-Cache im Crate
//! `client-cache` (als Decorator um ein beliebiges `Storage`).

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, CoreResult};

/// SHA-256 eines Dateiinhalts als Kleinbuchstaben-Hex (64 Zeichen).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);

impl ContentHash {
    pub fn of(data: &[u8]) -> Self {
        Self(hex::encode(Sha256::digest(data)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Prüft `data` gegen diesen Hash.
    pub fn verify(&self, data: &[u8]) -> CoreResult<()> {
        let actual = Self::of(data);
        if &actual == self {
            Ok(())
        } else {
            Err(CoreError::HashMismatch {
                expected: self.0.clone(),
                actual: actual.0,
            })
        }
    }
}

impl TryFrom<String> for ContentHash {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.to_ascii_lowercase();
        if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(CoreError::InvalidInput(format!(
                "kein SHA-256-Hex: {value}"
            )))
        }
    }
}

impl From<ContentHash> for String {
    fn from(value: ContentHash) -> Self {
        value.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Backend-unabhängiger Schlüssel, z. B. `projekte/<uuid>/plaene/eg.dxf`.
/// Immer relativ, mit `/` getrennt, ohne `..` – damit kann kein Backend aus
/// seinem Wurzelverzeichnis bzw. Bucket-Präfix ausbrechen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StorageKey(String);

impl StorageKey {
    pub fn new(key: impl Into<String>) -> CoreResult<Self> {
        let key: String = key.into();
        let valid = !key.is_empty()
            && !key.starts_with('/')
            && !key.contains('\\')
            && !key.contains('\0')
            && key
                .split('/')
                .all(|segment| !segment.is_empty() && segment != "." && segment != "..");
        if valid {
            Ok(Self(key))
        } else {
            Err(CoreError::InvalidInput(format!(
                "ungültiger Storage-Schlüssel: {key:?}"
            )))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

impl TryFrom<String> for StorageKey {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<StorageKey> for String {
    fn from(value: StorageKey) -> Self {
        value.0
    }
}

impl std::fmt::Display for StorageKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub key: StorageKey,
    pub size_bytes: u64,
    /// Jedes Backend speichert den SHA-256 beim `save` mit ab, damit Clients
    /// ohne Download validieren können.
    pub sha256: ContentHash,
}

/// Einheitliches Interface für Dateiablage. Implementierungen müssen `save`
/// atomar gestalten (kein halb geschriebenes Objekt sichtbar).
#[async_trait]
pub trait Storage: Send + Sync {
    /// Kurzname für Logs/Diagnose, z. B. `local`, `nas`, `s3`, `cached(s3)`.
    fn backend_name(&self) -> String;

    async fn save(&self, key: &StorageKey, data: Bytes) -> CoreResult<ObjectMeta>;

    async fn load(&self, key: &StorageKey) -> CoreResult<Bytes>;

    /// Metadaten ohne Inhalt; muss günstig sein (kein Download).
    async fn head(&self, key: &StorageKey) -> CoreResult<ObjectMeta>;

    async fn delete(&self, key: &StorageKey) -> CoreResult<()>;

    async fn list(&self, prefix: Option<&StorageKey>) -> CoreResult<Vec<ObjectMeta>>;

    /// Lädt und prüft gegen einen bekannten Hash (z. B. aus der
    /// Dokumenthistorie in der Datenbank). Caches überschreiben dies, um bei
    /// einem Treffer ganz ohne Netzwerkzugriff auszukommen.
    async fn load_verified(&self, key: &StorageKey, expected: &ContentHash) -> CoreResult<Bytes> {
        let data = self.load(key).await?;
        expected.verify(&data)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_key_rejects_traversal() {
        assert!(StorageKey::new("projekte/a/plan.dxf").is_ok());
        for bad in ["", "/abs", "a/../b", "a//b", "a/./b", "a\\b", "a/"] {
            assert!(
                StorageKey::new(bad).is_err(),
                "{bad:?} sollte ungültig sein"
            );
        }
    }

    #[test]
    fn content_hash_roundtrip() {
        let hash = ContentHash::of(b"Bauantrag");
        assert!(hash.verify(b"Bauantrag").is_ok());
        assert!(hash.verify(b"Bauantrag!").is_err());
        assert_eq!(ContentHash::try_from(hash.to_string()).unwrap(), hash);
    }
}
