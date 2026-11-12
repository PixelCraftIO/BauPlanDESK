//! Dateien übernommener Projekte aus einer entpackten Sicherung in die
//! laufende Ablage kopieren (siehe `projekte_uebernehmen` in `db-sqlite`).
//!
//! Ablauf in der Anwendung: Sicherung mit [`crate::sicherung::wiederherstellen`]
//! in ein Arbeitsverzeichnis entpacken, die Datenbank dort öffnen (Migration),
//! erst die Dateien hierüber kopieren und dann die Datenbankzeilen übernehmen.
//! So verweist die Datenbank nie auf eine fehlende Datei.
use core_lib::{
    CoreError, CoreResult,
    storage::{ContentHash, Storage, StorageKey},
};
use std::{collections::BTreeMap, path::Path};

/// Kopiert aus `dateien` (Ablage-Layout `data/<schlüssel>`, wie in einer
/// Sicherung unter `dateien/`) alle Dateien der Projekte `projekte` sowie alle
/// Schlüssel aus `erwartet` (Dokumentversionen, Formularquellen,
/// Einreichungspakete) nach `ziel`. Vorhandene Dateien mit gleichem Inhalt
/// bleiben stehen; ein abweichender Inhalt bricht ab, bevor etwas
/// überschrieben wird. Gibt die Zahl neu abgelegter Dateien zurück.
pub async fn dateien_uebernehmen(
    dateien: &Path,
    ziel: &dyn Storage,
    projekte: &[uuid::Uuid],
    erwartet: &BTreeMap<String, ContentHash>,
) -> CoreResult<usize> {
    let data = dateien.join("data");
    let quellen = if data.is_dir() {
        let d = data.clone();
        tokio::task::spawn_blocking(move || crate::sicherung::verzeichnis(&d, "data"))
            .await
            .map_err(|e| CoreError::Storage(e.to_string()))??
    } else {
        Vec::new()
    };
    let praefixe: Vec<String> = projekte.iter().map(|p| format!("projekte/{p}/")).collect();
    let mut offen: BTreeMap<&str, &ContentHash> =
        erwartet.iter().map(|(k, h)| (k.as_str(), h)).collect();
    let mut neu = 0;
    for q in &quellen {
        let Some(schluessel) = q.name.strip_prefix("data/") else {
            continue;
        };
        let hash = offen.remove(schluessel);
        if hash.is_none() && !praefixe.iter().any(|p| schluessel.starts_with(p)) {
            continue;
        }
        if crate::atomare_datei::ist_temporaer(schluessel.rsplit('/').next().unwrap_or(schluessel))
        {
            continue;
        }
        let inhalt = tokio::fs::read(&q.pfad)
            .await
            .map_err(|e| CoreError::Storage(format!("{}: {e}", q.pfad.display())))?;
        let ist = ContentHash::of(&inhalt);
        if let Some(h) = hash
            && *h != ist
        {
            return Err(CoreError::Storage(format!(
                "Datei in der Sicherung beschädigt: {schluessel}"
            )));
        }
        let key = StorageKey::new(schluessel)?;
        match ziel.head(&key).await {
            Ok(m) if m.sha256 == ist => continue,
            Ok(_) => {
                return Err(CoreError::InvalidInput(format!(
                    "In der Ablage liegt schon eine andere Datei unter {schluessel}; es wurde nichts überschrieben."
                )));
            }
            Err(CoreError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
        ziel.save(&key, inhalt.into()).await?;
        neu += 1;
    }
    if let Some(k) = offen.keys().next() {
        return Err(CoreError::Storage(format!(
            "{} Dateien fehlen in der Sicherung (z. B. {k}).",
            offen.len()
        )));
    }
    Ok(neu)
}
