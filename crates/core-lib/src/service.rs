//! Anwendungsfälle. Tauri-Commands und axum-Handler rufen ausschließlich
//! diese Methoden auf und enthalten selbst keine Geschäftslogik.

use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;

use crate::calc::{self, Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use crate::domain::{
    Bundesland, DocumentId, DocumentVersion, NewProject, Project, ProjectId, ProjectUpdate,
};
use crate::error::{CoreError, CoreResult};
use crate::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, LboEngine};
use crate::license::LicenseProvider;
use crate::repository::Repositories;
use crate::storage::{Storage, StorageKey};

/// Alle Abhängigkeiten der Geschäftslogik, von der Laufzeitumgebung injiziert:
///
/// | Umgebung     | Datenbank   | Storage                         |
/// |--------------|-------------|---------------------------------|
/// | Einzelplatz  | SQLite      | lokales Verzeichnis             |
/// | NAS/Büro     | PostgreSQL  | NAS-Freigabe (+ Client-Cache)   |
/// | Cloud        | PostgreSQL  | S3-kompatibel (+ Client-Cache)  |
#[derive(Clone)]
pub struct CoreServices {
    pub repos: Repositories,
    pub storage: Arc<dyn Storage>,
    pub lbo: Arc<LboEngine>,
    pub license: Arc<dyn LicenseProvider>,
}

impl CoreServices {
    pub fn new(
        repos: Repositories,
        storage: Arc<dyn Storage>,
        lbo: Arc<LboEngine>,
        license: Arc<dyn LicenseProvider>,
    ) -> Self {
        Self {
            repos,
            storage,
            lbo,
            license,
        }
    }

    // --- Projekte -------------------------------------------------------

    pub async fn create_project(&self, project: NewProject) -> CoreResult<Project> {
        validate_project(&project.name, project.grundstuecksflaeche_m2)?;
        self.repos.projects.create(project).await
    }

    pub async fn get_project(&self, id: ProjectId) -> CoreResult<Project> {
        self.repos.projects.get(id).await
    }

    pub async fn list_projects(&self) -> CoreResult<Vec<Project>> {
        self.repos.projects.list().await
    }

    pub async fn update_project(&self, update: ProjectUpdate) -> CoreResult<Project> {
        validate_project(&update.name, update.grundstuecksflaeche_m2)?;
        self.repos.projects.update(update).await
    }

    // --- DMS ------------------------------------------------------------

    /// Speichert eine neue Version eines Dokuments. `document_id = None`
    /// legt ein neues Dokument an.
    pub async fn store_document(
        &self,
        project_id: ProjectId,
        document_id: Option<DocumentId>,
        file_name: &str,
        data: Bytes,
        user: &str,
    ) -> CoreResult<DocumentVersion> {
        let file_name = sanitize_file_name(file_name)?;
        let document_id = document_id.unwrap_or_default();
        // Jede Version bekommt einen eigenen, unveränderlichen Schlüssel.
        let key = StorageKey::new(format!(
            "projekte/{project_id}/dokumente/{}/{}-{file_name}",
            document_id.0,
            uuid::Uuid::new_v4()
        ))?;
        let meta = self.storage.save(&key, data).await?;
        let version = DocumentVersion {
            document_id,
            project_id,
            version: 0, // vergibt das Repository
            file_name,
            storage_key: key.to_string(),
            sha256: meta.sha256,
            size_bytes: meta.size_bytes,
            created_by: user.to_owned(),
            created_at: Utc::now(),
        };
        self.repos.documents.add_version(version).await
    }

    /// Lädt die aktuelle Version. Der Hash kommt aus der Datenbank, sodass
    /// ein Client-Cache ohne Rückfrage beim Server validieren kann.
    pub async fn load_document(&self, id: DocumentId) -> CoreResult<(DocumentVersion, Bytes)> {
        let version = self.repos.documents.latest(id).await?;
        let key = StorageKey::new(version.storage_key.clone())?;
        let data = self.storage.load_verified(&key, &version.sha256).await?;
        Ok((version, data))
    }

    pub async fn document_history(&self, id: DocumentId) -> CoreResult<Vec<DocumentVersion>> {
        self.repos.documents.history(id).await
    }

    // --- Berechnungen ---------------------------------------------------

    pub fn mass_der_nutzung(
        &self,
        eingabe: &MassDerNutzung,
        gebiet: Baugebiet,
        festsetzungen: Option<Orientierungswerte>,
    ) -> CoreResult<Pruefergebnis> {
        calc::pruefe(eingabe, gebiet, festsetzungen)
    }

    pub fn abstandsflaeche(
        &self,
        land: Bundesland,
        input: &AbstandsflaecheInput,
    ) -> CoreResult<Berechnung<AbstandsflaecheErgebnis>> {
        self.lbo.abstandsflaeche(land, input)
    }
}

fn validate_project(name: &str, flaeche: Option<f64>) -> CoreResult<()> {
    if name.trim().is_empty() {
        return Err(CoreError::InvalidInput("Projektname fehlt".into()));
    }
    if let Some(f) = flaeche
        && !(f.is_finite() && f > 0.0)
    {
        return Err(CoreError::InvalidInput(
            "Grundstücksfläche muss größer als 0 sein".into(),
        ));
    }
    Ok(())
}

fn sanitize_file_name(name: &str) -> CoreResult<String> {
    let cleaned: String = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_owned();
    if cleaned.is_empty() {
        Err(CoreError::InvalidInput(format!(
            "ungültiger Dateiname: {name:?}"
        )))
    } else {
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_file_name;

    #[test]
    fn dateinamen() {
        assert_eq!(sanitize_file_name("C:\\plaene\\EG.dxf").unwrap(), "EG.dxf");
        assert_eq!(sanitize_file_name("../../etc/passwd").unwrap(), "passwd");
        assert!(sanitize_file_name("..").is_err());
    }
}
