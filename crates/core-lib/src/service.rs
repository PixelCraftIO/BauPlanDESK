//! Anwendungsfälle. Tauri-Commands und axum-Handler rufen ausschließlich
//! diese Methoden auf und enthalten selbst keine Geschäftslogik.

use std::collections::BTreeMap;
use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use serde::Serialize;

use crate::benutzer::{self, Benutzer, BenutzerAenderung, NeuerBenutzer, Recht, Rolle};
use crate::cad::{self, Laengeneinheit, Messung, Plan, Punkt};
use crate::calc::{self, Baugebiet, MassDerNutzung, Orientierungswerte, Pruefergebnis};
use crate::domain::{
    Aktion, Bundesland, DocumentId, DocumentLock, DocumentVersion, NeuerProtokolleintrag,
    NewProject, Project, ProjectId, ProjectUpdate, Protokolleintrag,
};
use crate::error::{CoreError, CoreResult};
use crate::formular::{self, Feldwert, Formularfeld, Formularkatalog, Projektkontext};
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
    pub formulare: Arc<Formularkatalog>,
}

/// Ein CAD-Plan aus dem DMS.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CadPlan {
    pub dokument: DocumentVersion,
    pub plan: Plan,
}

/// Ergebnis der Analyse eines PDF-Formulars aus dem DMS.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FormularAnalyse {
    pub dokument: DocumentVersion,
    pub felder: Vec<Formularfeld>,
    /// Erkannte Vorlage (id, Titel, Version), falls vorhanden.
    pub vorlage: Option<VorlageInfo>,
    /// Vorgeschlagene Werte aus den Projektdaten.
    pub vorschlag: BTreeMap<String, Feldwert>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VorlageInfo {
    pub id: String,
    pub titel: String,
    pub version: String,
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
            formulare: Arc::new(Formularkatalog::default()),
        }
    }

    /// Formularvorlagen injizieren (siehe [`crate::formular::vorlage`]).
    pub fn mit_formularen(mut self, katalog: Formularkatalog) -> Self {
        self.formulare = Arc::new(katalog);
        self
    }

    // --- Benutzer --------------------------------------------------------

    /// Anmeldung über den Betriebssystem-Benutzer (Desktop-App). Unbekannte
    /// Benutzer werden angelegt: Solange es keinen Administrator gibt, als
    /// Administrator, danach mit der Rolle „Lesen“; ein Administrator kann
    /// sie dann hochstufen.
    pub async fn anmelden(&self, anmeldename: &str, anzeigename: &str) -> CoreResult<Benutzer> {
        let name = benutzer::anmeldename_normalisieren(anmeldename)?;
        if let Some(b) = self.repos.benutzer.by_anmeldename(&name).await? {
            if !b.aktiv {
                return Err(CoreError::Forbidden(format!(
                    "Das Benutzerkonto „{name}“ ist deaktiviert. Bitte wenden Sie sich an einen Administrator."
                )));
            }
            return Ok(b);
        }
        let admin_vorhanden = self
            .repos
            .benutzer
            .list()
            .await?
            .iter()
            .any(|b| b.rolle == Rolle::Administrator && b.aktiv && b.anmeldename != SYSTEM);
        let anzeigename = match anzeigename.trim() {
            "" => name.clone(),
            n => n.to_string(),
        };
        let neu = self
            .repos
            .benutzer
            .create(NeuerBenutzer {
                anmeldename: name,
                anzeigename,
                rolle: if admin_vorhanden {
                    Rolle::Lesen
                } else {
                    Rolle::Administrator
                },
            })
            .await?;
        self.protokollieren(
            &neu,
            Aktion::BenutzerAngelegt,
            None,
            None,
            format!(
                "{} ({}), Rolle {}, bei der ersten Anmeldung",
                neu.anzeigename,
                neu.anmeldename,
                neu.rolle.name()
            ),
        )
        .await?;
        Ok(neu)
    }

    /// Technisches Konto für Server-Prozesse, bis der Server eine eigene
    /// Anmeldung hat. Zählt nicht als Administrator für [`Self::anmelden`].
    pub async fn systembenutzer(&self) -> CoreResult<Benutzer> {
        if let Some(b) = self.repos.benutzer.by_anmeldename(SYSTEM).await? {
            return Ok(b);
        }
        self.repos
            .benutzer
            .create(NeuerBenutzer {
                anmeldename: SYSTEM.into(),
                anzeigename: "System".into(),
                rolle: Rolle::Administrator,
            })
            .await
    }

    pub async fn benutzer_auflisten(&self) -> CoreResult<Vec<Benutzer>> {
        self.repos.benutzer.list().await
    }

    pub async fn benutzer_anlegen(
        &self,
        neu: NeuerBenutzer,
        wer: &Benutzer,
    ) -> CoreResult<Benutzer> {
        wer.pruefe(Recht::BenutzerVerwalten)?;
        let neu = NeuerBenutzer {
            anmeldename: benutzer::anmeldename_normalisieren(&neu.anmeldename)?,
            anzeigename: neu.anzeigename.trim().to_string(),
            rolle: neu.rolle,
        };
        if neu.anzeigename.is_empty() {
            return Err(CoreError::InvalidInput("Anzeigename fehlt".into()));
        }
        let b = self.repos.benutzer.create(neu).await?;
        self.protokollieren(
            wer,
            Aktion::BenutzerAngelegt,
            None,
            None,
            format!(
                "{} ({}), Rolle {}",
                b.anzeigename,
                b.anmeldename,
                b.rolle.name()
            ),
        )
        .await?;
        Ok(b)
    }

    pub async fn benutzer_aendern(
        &self,
        aenderung: BenutzerAenderung,
        wer: &Benutzer,
    ) -> CoreResult<Benutzer> {
        wer.pruefe(Recht::BenutzerVerwalten)?;
        if aenderung.anzeigename.trim().is_empty() {
            return Err(CoreError::InvalidInput("Anzeigename fehlt".into()));
        }
        let vorher = self.repos.benutzer.get(aenderung.id).await?;
        let verliert_admin = vorher.rolle == Rolle::Administrator
            && vorher.aktiv
            && (aenderung.rolle != Rolle::Administrator || !aenderung.aktiv);
        // Das Systemkonto zählt nicht: Es kann sich nicht anmelden.
        let admins = self
            .repos
            .benutzer
            .list()
            .await?
            .iter()
            .filter(|b| b.rolle == Rolle::Administrator && b.aktiv && b.anmeldename != SYSTEM)
            .count();
        if verliert_admin && admins <= 1 {
            return Err(CoreError::InvalidInput(
                "Der letzte aktive Administrator kann nicht herabgestuft oder deaktiviert werden."
                    .into(),
            ));
        }
        let b = self
            .repos
            .benutzer
            .update(BenutzerAenderung {
                anzeigename: aenderung.anzeigename.trim().to_string(),
                ..aenderung
            })
            .await?;
        self.protokollieren(
            wer,
            Aktion::BenutzerGeaendert,
            None,
            None,
            format!(
                "{} ({}): Rolle {}, {}",
                b.anzeigename,
                b.anmeldename,
                b.rolle.name(),
                if b.aktiv { "aktiv" } else { "deaktiviert" }
            ),
        )
        .await?;
        Ok(b)
    }

    // --- Projekte -------------------------------------------------------

    pub async fn create_project(&self, project: NewProject, wer: &Benutzer) -> CoreResult<Project> {
        wer.pruefe(Recht::ProjektAnlegen)?;
        validate_project(&project.name, project.grundstuecksflaeche_m2)?;
        let p = self.repos.projects.create(project).await?;
        self.protokollieren(
            wer,
            Aktion::ProjektAngelegt,
            Some(p.id),
            None,
            projekt_details(&p),
        )
        .await?;
        Ok(p)
    }

    pub async fn get_project(&self, id: ProjectId) -> CoreResult<Project> {
        self.repos.projects.get(id).await
    }

    pub async fn list_projects(&self) -> CoreResult<Vec<Project>> {
        self.repos.projects.list().await
    }

    pub async fn update_project(
        &self,
        update: ProjectUpdate,
        wer: &Benutzer,
    ) -> CoreResult<Project> {
        wer.pruefe(Recht::ProjektBearbeiten)?;
        validate_project(&update.name, update.grundstuecksflaeche_m2)?;
        let p = self.repos.projects.update(update).await?;
        self.protokollieren(
            wer,
            Aktion::ProjektGeaendert,
            Some(p.id),
            None,
            projekt_details(&p),
        )
        .await?;
        Ok(p)
    }

    /// Protokoll eines Projekts, neueste Einträge zuerst.
    pub async fn protokoll(
        &self,
        project: ProjectId,
        limit: i64,
    ) -> CoreResult<Vec<Protokolleintrag>> {
        self.repos
            .protokoll
            .list_for_project(project, limit.clamp(1, 10_000))
            .await
    }

    async fn protokollieren(
        &self,
        wer: &Benutzer,
        aktion: Aktion,
        project_id: Option<ProjectId>,
        document_id: Option<DocumentId>,
        details: String,
    ) -> CoreResult<()> {
        self.repos
            .protokoll
            .append(NeuerProtokolleintrag {
                zeit: Utc::now(),
                benutzer_id: wer.id,
                benutzer_name: wer.anzeigename.clone(),
                aktion,
                project_id,
                document_id,
                details,
            })
            .await
    }

    // --- DMS ------------------------------------------------------------

    /// Speichert eine neue Version eines Dokuments. `document_id = None`
    /// legt ein neues Dokument an.
    ///
    /// `basis` ist die Version, auf der die Änderung beruht (Optimistic
    /// Locking): Hat inzwischen jemand eine neuere Version abgelegt, schlägt
    /// das Speichern mit [`CoreError::VersionConflict`] fehl, statt dessen
    /// Arbeit stillschweigend zu überholen. Ein von anderen gesperrtes
    /// Dokument ergibt [`CoreError::Gesperrt`].
    pub async fn store_document(
        &self,
        project_id: ProjectId,
        document_id: Option<DocumentId>,
        basis: Option<i64>,
        file_name: &str,
        data: Bytes,
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        self.ablegen(
            project_id,
            document_id,
            basis,
            file_name,
            data,
            wer,
            Aktion::DokumentAbgelegt,
            "",
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn ablegen(
        &self,
        project_id: ProjectId,
        document_id: Option<DocumentId>,
        basis: Option<i64>,
        file_name: &str,
        data: Bytes,
        wer: &Benutzer,
        aktion: Aktion,
        zusatz: &str,
    ) -> CoreResult<DocumentVersion> {
        wer.pruefe(Recht::DokumentAblegen)?;
        let file_name = sanitize_file_name(file_name)?;
        let document_id = match document_id {
            Some(id) => {
                let aktuell = self.repos.documents.latest(id).await?;
                if aktuell.project_id != project_id {
                    return Err(CoreError::InvalidInput(
                        "Das Dokument gehört zu einem anderen Projekt".into(),
                    ));
                }
                if let Some(sperre) = self.repos.documents.get_lock(id).await?
                    && sperre.benutzer_id != wer.id
                {
                    return Err(CoreError::Gesperrt {
                        von: sperre.benutzer_name,
                        seit: sperre.seit,
                    });
                }
                if let Some(b) = basis
                    && b != aktuell.version
                {
                    return Err(CoreError::VersionConflict {
                        expected: b,
                        actual: aktuell.version,
                    });
                }
                id
            }
            None if basis.is_some() => {
                return Err(CoreError::InvalidInput(
                    "Eine Basisversion gibt es nur für bestehende Dokumente".into(),
                ));
            }
            None => DocumentId::new(),
        };
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
            created_by: wer.anzeigename.clone(),
            created_at: Utc::now(),
        };
        let gespeichert = match self.repos.documents.add_version(version, basis).await {
            Ok(v) => v,
            Err(e) => {
                // Die Datei wäre ohne Datenbankeintrag verwaist.
                if let Err(aufraeumen) = self.storage.delete(&key).await {
                    tracing::warn!(%key, error = %aufraeumen, "verwaiste Datei nicht gelöscht");
                }
                return Err(e);
            }
        };
        self.protokollieren(
            wer,
            aktion,
            Some(project_id),
            Some(document_id),
            format!(
                "{}, Version {}{zusatz}",
                gespeichert.file_name, gespeichert.version
            ),
        )
        .await?;
        Ok(gespeichert)
    }

    /// Lädt die aktuelle Version. Der Hash kommt aus der Datenbank, sodass
    /// ein Client-Cache ohne Rückfrage beim Server validieren kann.
    pub async fn load_document(&self, id: DocumentId) -> CoreResult<(DocumentVersion, Bytes)> {
        let version = self.repos.documents.latest(id).await?;
        self.inhalt(version).await
    }

    /// Lädt eine bestimmte, auch ältere Version.
    pub async fn load_version(
        &self,
        id: DocumentId,
        version: i64,
    ) -> CoreResult<(DocumentVersion, Bytes)> {
        let v = self
            .repos
            .documents
            .history(id)
            .await?
            .into_iter()
            .find(|v| v.version == version)
            .ok_or_else(|| {
                CoreError::NotFound(format!("Version {version} von Dokument {}", id.0))
            })?;
        self.inhalt(v).await
    }

    async fn inhalt(&self, version: DocumentVersion) -> CoreResult<(DocumentVersion, Bytes)> {
        let key = StorageKey::new(version.storage_key.clone())?;
        let data = self.storage.load_verified(&key, &version.sha256).await?;
        Ok((version, data))
    }

    /// Legt eine ältere Version als neue, aktuelle Version ab. Die Historie
    /// bleibt vollständig erhalten.
    pub async fn version_wiederherstellen(
        &self,
        id: DocumentId,
        version: i64,
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        let aktuell = self.repos.documents.latest(id).await?;
        let (alt, daten) = self.load_version(id, version).await?;
        self.ablegen(
            aktuell.project_id,
            Some(id),
            Some(aktuell.version),
            &alt.file_name,
            daten,
            wer,
            Aktion::DokumentAbgelegt,
            &format!(" (wiederhergestellt aus Version {version})"),
        )
        .await
    }

    /// Alle Versionen aller Dokumente eines Projekts.
    pub async fn list_documents(&self, project: ProjectId) -> CoreResult<Vec<DocumentVersion>> {
        self.repos.documents.list_for_project(project).await
    }

    /// Sperrt ein Dokument, damit andere keine neuen Versionen ablegen,
    /// während `wer` es außerhalb der App bearbeitet.
    pub async fn dokument_sperren(
        &self,
        id: DocumentId,
        wer: &Benutzer,
    ) -> CoreResult<DocumentLock> {
        wer.pruefe(Recht::DokumentSperren)?;
        let aktuell = self.repos.documents.latest(id).await?;
        if let Some(s) = self.repos.documents.get_lock(id).await?
            && s.benutzer_id == wer.id
        {
            return Ok(s);
        }
        let sperre = self
            .repos
            .documents
            .lock(id, aktuell.project_id, wer)
            .await?;
        self.protokollieren(
            wer,
            Aktion::DokumentGesperrt,
            Some(aktuell.project_id),
            Some(id),
            aktuell.file_name,
        )
        .await?;
        Ok(sperre)
    }

    /// Hebt die eigene Sperre auf; fremde Sperren nur mit dem Recht
    /// [`Recht::FremdeSperreAufheben`].
    pub async fn dokument_entsperren(&self, id: DocumentId, wer: &Benutzer) -> CoreResult<()> {
        let Some(sperre) = self.repos.documents.get_lock(id).await? else {
            return Ok(());
        };
        let fremd = sperre.benutzer_id != wer.id;
        if fremd {
            wer.pruefe(Recht::FremdeSperreAufheben)?;
        }
        let aufgehoben = self
            .repos
            .documents
            .unlock(id, (!fremd).then_some(wer.id))
            .await?;
        if aufgehoben.is_some() {
            let name = self.repos.documents.latest(id).await?.file_name;
            let (aktion, details) = if fremd {
                (
                    Aktion::SperreAufgehoben,
                    format!("{name} (gesperrt von {})", sperre.benutzer_name),
                )
            } else {
                (Aktion::DokumentEntsperrt, name)
            };
            self.protokollieren(wer, aktion, Some(sperre.project_id), Some(id), details)
                .await?;
        }
        Ok(())
    }

    pub async fn sperren_im_projekt(&self, project: ProjectId) -> CoreResult<Vec<DocumentLock>> {
        self.repos.documents.locks_for_project(project).await
    }

    // --- PDF-Formulare ---------------------------------------------------

    /// Liest die Felder eines PDF-Formulars aus dem DMS und schlägt Werte aus
    /// den Projektdaten vor.
    pub async fn formular_analysieren(&self, id: DocumentId) -> CoreResult<FormularAnalyse> {
        let (dokument, pdf) = self.load_document(id).await?;
        let projekt = self.repos.projects.get(dokument.project_id).await?;
        let felder = formular::pdf::felder_lesen(&pdf)?;
        if felder.is_empty() {
            return Err(CoreError::InvalidInput(format!(
                "{} enthält keine ausfüllbaren Formularfelder",
                dokument.file_name
            )));
        }
        let vorlage = self.formulare.erkenne(&felder);
        let vorschlag =
            formular::vorlage::vorschlag(&felder, vorlage, &Projektkontext::aus_projekt(&projekt));
        Ok(FormularAnalyse {
            vorlage: vorlage.map(|v| VorlageInfo {
                id: v.id.clone(),
                titel: v.titel.clone(),
                version: v.version.clone(),
            }),
            dokument,
            felder,
            vorschlag,
        })
    }

    /// Befüllt das Formular und legt das Ergebnis als neues Dokument im
    /// selben Projekt ab; das leere Original bleibt unverändert.
    pub async fn formular_ausfuellen(
        &self,
        id: DocumentId,
        werte: &BTreeMap<String, Feldwert>,
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        wer.pruefe(Recht::DokumentAblegen)?;
        let (original, pdf) = self.load_document(id).await?;
        let ausgefuellt = formular::pdf::befuellen(&pdf, werte)?;
        let stamm = original
            .file_name
            .strip_suffix(".pdf")
            .or_else(|| original.file_name.strip_suffix(".PDF"))
            .unwrap_or(&original.file_name);
        self.ablegen(
            original.project_id,
            None,
            None,
            &format!("{stamm} (ausgefüllt).pdf"),
            Bytes::from(ausgefuellt),
            wer,
            Aktion::FormularAusgefuellt,
            &format!(" aus {}, Version {}", original.file_name, original.version),
        )
        .await
    }

    /// Lädt die aktuelle Version eines DXF-Plans aus dem DMS.
    pub async fn cad_plan(&self, id: DocumentId) -> CoreResult<CadPlan> {
        let (dokument, daten) = self.load_document(id).await?;
        let endung = dokument
            .file_name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        if endung == "dwg" {
            return Err(CoreError::InvalidInput(
                "DWG-Dateien können nicht direkt geöffnet werden. Bitte den Plan im CAD-Programm als DXF exportieren.".into(),
            ));
        }
        let plan = cad::dxf::lesen(&daten)?;
        Ok(CadPlan { dokument, plan })
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

    pub fn messen(
        &self,
        punkte: &[Punkt],
        geschlossen: bool,
        einheit: Laengeneinheit,
    ) -> CoreResult<Messung> {
        cad::messen(punkte, geschlossen, einheit)
    }

    pub fn abstandsflaeche(
        &self,
        land: Bundesland,
        input: &AbstandsflaecheInput,
    ) -> CoreResult<Berechnung<AbstandsflaecheErgebnis>> {
        self.lbo.abstandsflaeche(land, input)
    }
}

/// Anmeldename des technischen Kontos, siehe [`CoreServices::systembenutzer`].
const SYSTEM: &str = "system";

fn projekt_details(p: &Project) -> String {
    let mut d = format!("{} ({})", p.name, p.bundesland.name());
    if let Some(f) = p.grundstuecksflaeche_m2 {
        d.push_str(&format!(
            ", Grundstück {} m²",
            f.to_string().replace('.', ",")
        ));
    }
    d
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
