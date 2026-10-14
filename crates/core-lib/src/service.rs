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
use crate::kunden::{self, Kunde, KundeId, Kundendaten};
use crate::lbo::{AbstandsflaecheErgebnis, AbstandsflaecheInput, Berechnung, LboEngine};
use crate::license::{LicenseProvider, Lizenzstatus};
use crate::repository::Repositories;
use crate::storage::{Storage, StorageKey};
use crate::xbau::{
    self, Anlagendatei, Bauantrag, Beteiligter, Buero, Codelisten, GespeicherterBauantrag,
    Nachrichtendaten,
};

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
    pub codelisten: Arc<Codelisten>,
}

/// Einstellungsschlüssel der Büroangaben.
const BUERO: &str = "xbau.buero";

/// Ergebnis des XBau-Exports.
#[derive(Debug, Clone, PartialEq)]
pub struct XbauExport {
    /// Vorgeschlagener Dateiname.
    pub dateiname: String,
    pub zip: Vec<u8>,
}

/// Entfernt Zeichen, die in Dateinamen unter Windows nicht erlaubt sind.
fn dateiname_sicher(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    s.trim().trim_end_matches('.').to_string()
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
            codelisten: Arc::new(Codelisten::eingebaut()),
        }
    }

    /// Formularvorlagen injizieren (siehe [`crate::formular::vorlage`]).
    pub fn mit_formularen(mut self, katalog: Formularkatalog) -> Self {
        self.formulare = Arc::new(katalog);
        self
    }

    /// XBau-Codelisten injizieren (siehe [`crate::xbau::codeliste`]).
    pub fn mit_codelisten(mut self, codelisten: Codelisten) -> Self {
        self.codelisten = Arc::new(codelisten);
        self
    }

    // --- Rechte und Lizenz ---------------------------------------------

    /// Rolle prüfen und, für Änderungen an Projekten und Dokumenten, ob die
    /// Lizenz Schreiben erlaubt. Benutzerverwaltung und Lizenz einspielen
    /// bleiben auch ohne gültige Lizenz möglich, sonst käme man nie heraus.
    pub(crate) async fn pruefe(&self, wer: &Benutzer, recht: Recht) -> CoreResult<()> {
        wer.pruefe(recht)?;
        let status = self.license.status().await?;
        if !status.schreiben_erlaubt {
            return Err(CoreError::License(status.meldung));
        }
        Ok(())
    }

    pub async fn lizenz(&self) -> CoreResult<Lizenzstatus> {
        self.license.status().await
    }

    /// Spielt eine Lizenzdatei ein (nur Administratoren). Sie gilt danach für
    /// alle Arbeitsplätze an dieser Datenbank.
    pub async fn lizenz_einspielen(
        &self,
        datei: &[u8],
        wer: &Benutzer,
    ) -> CoreResult<Lizenzstatus> {
        wer.pruefe(Recht::LizenzVerwalten)?;
        let status = self.license.einspielen(datei).await?;
        self.lizenz_protokollieren(wer, &status, "").await?;
        Ok(status)
    }

    /// Trägt einen Abo-Lizenzschlüssel ein (nur Administratoren) und
    /// aktiviert diesen Rechner beim Lizenzserver. Die übrigen Arbeitsplätze
    /// an dieser Datenbank aktivieren sich beim nächsten Kontakt selbst.
    pub async fn abo_aktivieren(
        &self,
        lizenzschluessel: &str,
        wer: &Benutzer,
    ) -> CoreResult<Lizenzstatus> {
        wer.pruefe(Recht::LizenzVerwalten)?;
        let status = self.license.abo_aktivieren(lizenzschluessel).await?;
        self.lizenz_protokollieren(wer, &status, "Abo, ").await?;
        Ok(status)
    }

    /// Bestätigung sofort beim Lizenzserver erneuern (jeder Benutzer, z. B.
    /// nach längerer Zeit offline).
    pub async fn abo_bestaetigen(&self) -> CoreResult<Lizenzstatus> {
        self.license.abo_bestaetigen().await
    }

    /// Gibt den Platz dieses Rechners beim Lizenzserver frei (nur
    /// Administratoren), z. B. vor einem Rechnerwechsel.
    pub async fn abo_abmelden(&self, wer: &Benutzer) -> CoreResult<Lizenzstatus> {
        wer.pruefe(Recht::LizenzVerwalten)?;
        self.license.abo_abmelden().await
    }

    async fn lizenz_protokollieren(
        &self,
        wer: &Benutzer,
        status: &Lizenzstatus,
        art: &str,
    ) -> CoreResult<()> {
        let Some(l) = &status.lizenz else {
            return Ok(());
        };
        self.protokollieren(
            wer,
            Aktion::LizenzEingespielt,
            None,
            None,
            format!(
                "{} ({art}{}, {} Arbeitsplätze, Lizenz {})",
                l.lizenznehmer,
                l.edition.name(),
                l.arbeitsplaetze,
                l.lizenz_id
            ),
        )
        .await
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
        self.pruefe(wer, Recht::ProjektAnlegen).await?;
        validate_project(&project.name, project.grundstuecksflaeche_m2)?;
        project.planungsrecht.validate()?;
        if project.planungsrecht.baugrenzen.is_some() {
            return Err(CoreError::InvalidInput(
                "Baugrenzen erst einem angelegten Projekt zuordnen".into(),
            ));
        }
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
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        validate_project(&update.name, update.grundstuecksflaeche_m2)?;
        if let Some(p) = &update.planungsrecht {
            p.validate()?;
            if let Some(g) = &p.baugrenzen {
                let vorher = self.repos.projects.get(update.id).await?;
                if vorher.planungsrecht.baugrenzen.as_ref() != Some(g) {
                    self.pruefe_planbezug(update.id, g).await?;
                }
            }
        }
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

    pub(crate) async fn protokollieren(
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

    // --- Kunden ---------------------------------------------------------

    /// Alle Kunden, nach Namen sortiert.
    pub async fn kunden_auflisten(&self) -> CoreResult<Vec<Kunde>> {
        let mut k = self.repos.kunden.kunden().await?;
        kunden::sortieren(&mut k);
        Ok(k)
    }

    pub async fn kunde(&self, id: KundeId) -> CoreResult<Kunde> {
        self.repos.kunden.kunde(id).await
    }

    pub async fn kunde_anlegen(&self, daten: Kundendaten, wer: &Benutzer) -> CoreResult<Kunde> {
        self.pruefe(wer, Recht::KundenBearbeiten).await?;
        let daten = daten.bereinigt();
        daten.pruefen()?;
        let k = self.repos.kunden.kunde_anlegen(daten).await?;
        self.protokollieren(wer, Aktion::KundeAngelegt, None, None, kunde_details(&k))
            .await?;
        Ok(k)
    }

    pub async fn kunde_aendern(
        &self,
        id: KundeId,
        daten: Kundendaten,
        expected_version: i64,
        wer: &Benutzer,
    ) -> CoreResult<Kunde> {
        self.pruefe(wer, Recht::KundenBearbeiten).await?;
        let daten = daten.bereinigt();
        daten.pruefen()?;
        let k = self
            .repos
            .kunden
            .kunde_aendern(id, daten, expected_version)
            .await?;
        // Im Protokoll jedes Projekts vermerken, dessen Bauherr sich ändert.
        let projekte = self.repos.kunden.projekte_des_kunden(id).await?;
        if projekte.is_empty() {
            self.protokollieren(wer, Aktion::KundeGeaendert, None, None, kunde_details(&k))
                .await?;
        }
        for p in projekte {
            self.protokollieren(
                wer,
                Aktion::KundeGeaendert,
                Some(p.id),
                None,
                format!("Bauherr {}", kunde_details(&k)),
            )
            .await?;
        }
        Ok(k)
    }

    /// Löscht einen Kunden, der keinem Projekt mehr zugeordnet ist.
    pub async fn kunde_loeschen(
        &self,
        id: KundeId,
        expected_version: i64,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        self.pruefe(wer, Recht::KundenLoeschen).await?;
        let k = self.repos.kunden.kunde(id).await?;
        let projekte = self.repos.kunden.projekte_des_kunden(id).await?;
        if !projekte.is_empty() {
            let namen: Vec<_> = projekte.iter().map(|p| format!("„{}“", p.name)).collect();
            return Err(CoreError::InvalidInput(format!(
                "{} ist Bauherr in {} und kann erst gelöscht werden, wenn er dort nicht mehr eingetragen ist.",
                k.anzeigename(),
                namen.join(", ")
            )));
        }
        self.repos
            .kunden
            .kunde_loeschen(id, expected_version)
            .await?;
        self.protokollieren(wer, Aktion::KundeGeloescht, None, None, k.anzeigename())
            .await
    }

    /// Projekte, in denen der Kunde Bauherr ist.
    pub async fn kunde_projekte(&self, id: KundeId) -> CoreResult<Vec<Project>> {
        self.repos.kunden.projekte_des_kunden(id).await
    }

    /// Bauherren eines Projekts in der gespeicherten Reihenfolge.
    pub async fn bauherren(&self, project: ProjectId) -> CoreResult<Vec<Kunde>> {
        self.repos.kunden.bauherren(project).await
    }

    /// Ordnet dem Projekt die Kunden als Bauherren zu. `expected_version`
    /// ist die Projektversion (Optimistic Locking); sie steigt um 1.
    pub async fn bauherren_setzen(
        &self,
        project: ProjectId,
        kunden: Vec<KundeId>,
        expected_version: i64,
        wer: &Benutzer,
    ) -> CoreResult<Project> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        let mut ids: Vec<KundeId> = Vec::with_capacity(kunden.len());
        let mut namen = Vec::with_capacity(kunden.len());
        for id in kunden {
            if !ids.contains(&id) {
                namen.push(self.repos.kunden.kunde(id).await?.anzeigename());
                ids.push(id);
            }
        }
        let p = self
            .repos
            .kunden
            .bauherren_setzen(project, &ids, expected_version)
            .await?;
        self.protokollieren(
            wer,
            Aktion::BauherrenGeaendert,
            Some(project),
            None,
            if namen.is_empty() {
                "keine Bauherren".into()
            } else {
                namen.join(", ")
            },
        )
        .await?;
        Ok(p)
    }

    /// Bauherren für Bauantrag und Export. Hat ein alter Bauantrag noch
    /// eigene Bauherren und das Projekt keine, gelten bis zur Übernahme
    /// ([`Self::bauherren_uebernehmen`]) die alten.
    async fn bauherren_fuer_antrag(
        &self,
        project: ProjectId,
        antrag: &Bauantrag,
    ) -> CoreResult<Vec<Beteiligter>> {
        let kunden = self.repos.kunden.bauherren(project).await?;
        Ok(if kunden.is_empty() {
            antrag.bauherren.clone()
        } else {
            kunden.iter().map(Kunde::als_beteiligter).collect()
        })
    }

    /// Übernimmt die Bauherren, die in Bauanträgen älterer Versionen direkt
    /// erfasst wurden, in die Kundendatenbank und ordnet sie den Projekten
    /// zu. Gleiche Personen bzw. Firmen werden nur einmal angelegt. Läuft
    /// beim Start (`wer` steht im Protokoll) und ist beliebig oft
    /// wiederholbar. Liefert die Zahl der Projekte, deren Bauherren
    /// übernommen wurden.
    pub async fn bauherren_uebernehmen(&self, wer: &Benutzer) -> CoreResult<usize> {
        let mut anzahl = 0;
        for p in self.repos.projects.list().await? {
            if self.bauherren_uebernehmen_fuer(p.id, wer).await?.is_some() {
                anzahl += 1;
            }
        }
        Ok(anzahl)
    }

    /// Übernahme für ein Projekt. Liefert die Version des Bauantrags vor und
    /// nach der Übernahme, `None` wenn nichts zu übernehmen war.
    async fn bauherren_uebernehmen_fuer(
        &self,
        project: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Option<(i64, i64)>> {
        let Some(d) = self.repos.bauantraege.bauantrag(project).await? else {
            return Ok(None);
        };
        let mut antrag: Bauantrag = match serde_json::from_str(&d.daten) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(%project, error = %e, "Bauantrag nicht lesbar, Bauherren nicht übernommen");
                return Ok(None);
            }
        };
        if antrag.bauherren.is_empty() {
            return Ok(None);
        }
        let mut kunden = self.repos.kunden.kunden().await?;
        let mut ids: Vec<KundeId> = self
            .repos
            .kunden
            .bauherren(project)
            .await?
            .iter()
            .map(|k| k.id)
            .collect();
        for b in &antrag.bauherren {
            let daten = Kundendaten::aus_beteiligter(b);
            if daten.ist_leer() {
                continue;
            }
            let id = match kunden.iter().find(|k| k.daten.gleiche_identitaet(&daten)) {
                Some(k) => k.id,
                None => {
                    let k = self.repos.kunden.kunde_anlegen(daten).await?;
                    self.protokollieren(
                        wer,
                        Aktion::KundeAngelegt,
                        Some(project),
                        None,
                        format!("{} (aus dem Bauantrag übernommen)", kunde_details(&k)),
                    )
                    .await?;
                    let id = k.id;
                    kunden.push(k);
                    id
                }
            };
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let p = self.repos.projects.get(project).await?;
        self.repos
            .kunden
            .bauherren_setzen(project, &ids, p.version)
            .await?;
        antrag.bauherren.clear();
        let daten =
            serde_json::to_string(&antrag).map_err(|e| CoreError::Database(e.to_string()))?;
        let neu = match self
            .repos
            .bauantraege
            .bauantrag_speichern(project, &daten, d.version, &wer.anzeigename)
            .await
        {
            Ok(v) => v,
            // Inzwischen anders gespeichert: Neue Fassungen enthalten keine
            // Bauherren mehr, es bleibt nichts zu tun.
            Err(CoreError::VersionConflict { actual, .. }) => actual,
            Err(e) => return Err(e),
        };
        Ok(Some((d.version, neu)))
    }

    // --- XBau -----------------------------------------------------------

    /// Angaben zum Bauantrag; noch nie gespeichert = Vorgabewerte mit
    /// Version 0.
    pub async fn bauantrag(&self, project: ProjectId) -> CoreResult<GespeicherterBauantrag> {
        self.repos.projects.get(project).await?;
        let (antrag, version, von, am) = match self.repos.bauantraege.bauantrag(project).await? {
            Some(d) => (
                serde_json::from_str(&d.daten).map_err(|e| {
                    CoreError::Database(format!("gespeicherter Bauantrag ist ungültig: {e}"))
                })?,
                d.version,
                Some(d.geaendert_von),
                Some(d.geaendert_am),
            ),
            None => (Bauantrag::default(), 0, None, None),
        };
        let bauherren = self.bauherren_fuer_antrag(project, &antrag).await?;
        let mut fehlende_angaben = antrag.fehlende_angaben(&self.buero().await?, &bauherren);
        fehlende_angaben.extend(self.codelisten.fehlende());
        fehlende_angaben.extend(self.anlagen_pruefen(&antrag).await?);
        Ok(GespeicherterBauantrag {
            antrag,
            version,
            geaendert_von: von,
            geaendert_am: am,
            fehlende_angaben,
        })
    }

    /// Anlagen, die beim Export abgelehnt würden: Dokument gelöscht,
    /// Dateityp oder Anlagenart nicht in der jeweiligen Codeliste.
    async fn anlagen_pruefen(&self, antrag: &Bauantrag) -> CoreResult<Vec<String>> {
        let mimes = self.codelisten.liste(crate::xbau::codeliste::MIMETYPES);
        let arten = self.codelisten.liste(crate::xbau::codeliste::ANLAGEN);
        let mut fehlt = Vec::new();
        for a in &antrag.anlagen {
            let datei = match self.repos.documents.latest(a.dokument).await {
                Ok(v) => v.file_name,
                Err(CoreError::NotFound(_)) => {
                    fehlt.push("Anlage: Das Dokument wurde gelöscht".into());
                    continue;
                }
                Err(e) => return Err(e),
            };
            if let Some(Err(CoreError::InvalidInput(m))) =
                mimes.map(|l| crate::xbau::mime_pruefen(&datei, l))
            {
                fehlt.push(m);
            }
            if let Some(l) = arten
                && !a.anlageart.trim().is_empty()
                && !l.enthaelt(&a.anlageart)
            {
                fehlt.push(format!(
                    "{datei}: Art der Anlage {} steht nicht in der Codeliste",
                    a.anlageart
                ));
            }
        }
        Ok(fehlt)
    }

    pub async fn bauantrag_speichern(
        &self,
        project: ProjectId,
        antrag: Bauantrag,
        expected_version: i64,
        wer: &Benutzer,
    ) -> CoreResult<GespeicherterBauantrag> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        self.repos.projects.get(project).await?;
        // Alte Bauherren erst in die Kundendatenbank holen, sonst gingen sie
        // mit dem Speichern verloren.
        let expected_version = match self.bauherren_uebernehmen_fuer(project, wer).await? {
            Some((alt, neu)) if alt == expected_version => neu,
            _ => expected_version,
        };
        for a in &antrag.anlagen {
            let v = self.repos.documents.latest(a.dokument).await?;
            if v.project_id != project {
                return Err(CoreError::InvalidInput(format!(
                    "{} gehört zu einem anderen Projekt",
                    v.file_name
                )));
            }
        }
        let daten =
            serde_json::to_string(&antrag).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        self.repos
            .bauantraege
            .bauantrag_speichern(project, &daten, expected_version, &wer.anzeigename)
            .await?;
        self.protokollieren(
            wer,
            Aktion::BauantragGeaendert,
            Some(project),
            None,
            "Angaben zum Bauantrag".into(),
        )
        .await?;
        self.bauantrag(project).await
    }

    /// Angaben zum eigenen Büro (Absender und Entwurfsverfasser).
    pub async fn buero(&self) -> CoreResult<Buero> {
        match self.repos.lizenz.einstellung(BUERO).await? {
            Some(t) => serde_json::from_str(&t)
                .map_err(|e| CoreError::Database(format!("Büroangaben sind ungültig: {e}"))),
            None => Ok(Buero::default()),
        }
    }

    pub async fn buero_speichern(&self, buero: Buero, wer: &Benutzer) -> CoreResult<Buero> {
        self.pruefe(wer, Recht::BueroangabenAendern).await?;
        let t =
            serde_json::to_string(&buero).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        self.repos.lizenz.einstellung_setzen(BUERO, &t).await?;
        Ok(buero)
    }

    /// Erzeugt das XBau-Paket (ZIP mit Antragsnachricht und Anlagen) aus dem
    /// gespeicherten Bauantrag. Exportieren ist Lesen: Es geht mit jeder
    /// Rolle und auch, wenn die Lizenz nur noch Lesen erlaubt.
    pub async fn xbau_exportieren(
        &self,
        project: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<XbauExport> {
        let projekt = self.repos.projects.get(project).await?;
        let antrag = self.bauantrag(project).await?.antrag;
        let bauherren = self.bauherren_fuer_antrag(project, &antrag).await?;
        let buero = self.buero().await?;
        let mut versionen = Vec::new();
        for a in &antrag.anlagen {
            let (v, inhalt) = self.load_document(a.dokument).await?;
            if v.project_id != project {
                return Err(CoreError::InvalidInput(format!(
                    "{} gehört zu einem anderen Projekt",
                    v.file_name
                )));
            }
            versionen.push((a, v, inhalt));
        }
        let namen =
            xbau::paket::eindeutige_namen(versionen.iter().map(|(_, v, _)| v.file_name.as_str()));
        let anlagen: Vec<Anlagendatei> = versionen
            .iter()
            .zip(&namen)
            .map(|((a, v, _), name)| Anlagendatei {
                dokumentid: a.dokument.0,
                bezeichnung: match a.bezeichnung.trim() {
                    "" => v.file_name.clone(),
                    b => b.to_string(),
                },
                versionsnummer: v.version,
                datum: v.created_at.date_naive(),
                anlageart: a.anlageart.trim().to_string(),
                dateiname: name.clone(),
            })
            .collect();
        let jetzt = Utc::now();
        let xml = xbau::baugenehmigung_antrag(&Nachrichtendaten {
            projekt: &projekt,
            antrag: &antrag,
            bauherren: &bauherren,
            buero: &buero,
            anlagen: &anlagen,
            codelisten: &self.codelisten,
            nachricht_id: uuid::Uuid::new_v4(),
            zeitpunkt: jetzt,
        })?;
        let dateien: Vec<(String, &[u8])> = namen
            .iter()
            .zip(&versionen)
            .map(|(n, (_, _, inhalt))| (n.clone(), inhalt.as_ref()))
            .collect();
        let zip = xbau::paket::packen(&xml, &dateien)?;
        self.protokollieren(
            wer,
            Aktion::XbauExportiert,
            Some(project),
            None,
            format!(
                "Bauantrag nach XBau {} mit {} Anlagen",
                xbau::XBAU_VERSION,
                anlagen.len()
            ),
        )
        .await?;
        Ok(XbauExport {
            dateiname: format!(
                "Bauantrag {} {}.zip",
                dateiname_sicher(&projekt.name),
                jetzt.with_timezone(&chrono::Local).format("%Y-%m-%d")
            ),
            zip,
        })
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
        self.pruefe(wer, Recht::DokumentAblegen).await?;
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
        self.pruefe(wer, Recht::DokumentSperren).await?;
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
            self.pruefe(wer, Recht::FremdeSperreAufheben).await?;
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

    /// Liefert einen backendunabhängigen Stand für lesbare Projektordner.
    pub async fn projektordner_plan(
        &self,
        id: ProjectId,
        historie: bool,
    ) -> CoreResult<crate::projektablage::ProjektordnerPlan> {
        let project = self.get_project(id).await?;
        let docs = crate::projektablage::versionen(self.list_documents(id).await?, historie);
        crate::projektablage::planen(&project, &docs)
    }

    /// Fügt benannte Felder einer wiederverwendbaren PDF-Vorlage hinzu.
    /// Der PDF-Hintergrund und das importierte Original bleiben erhalten.
    pub async fn formular_vorlage_erstellen(
        &self,
        id: DocumentId,
        basis: i64,
        felder: &[formular::editor::EigenesPdfFeld],
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        let (original, bytes) = self.load_document(id).await?;
        if original.version != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: original.version,
            });
        }
        let pdf = formular::editor::felder_hinzufuegen(&bytes, felder)?;
        let stamm = original
            .file_name
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(&original.file_name);
        self.ablegen(
            original.project_id,
            None,
            None,
            &format!("{stamm} (Vorlage).pdf"),
            Bytes::from(pdf),
            wer,
            Aktion::DokumentAbgelegt,
            &format!(
                "; PDF-Vorlage aus {}, Version {}",
                original.file_name, original.version
            ),
        )
        .await
    }

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
        let mut kontext = Projektkontext::aus_projekt(&projekt);
        let bauherren = self.repos.kunden.bauherren(projekt.id).await?;
        if let Some(b) = bauherren.first() {
            kontext.bauherr(b);
        }
        let vorschlag = formular::vorlage::vorschlag(&felder, vorlage, &kontext);
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
        self.formular_ausfuellen_version(id, werte, None, wer).await
    }

    pub async fn formular_ausfuellen_version(
        &self,
        id: DocumentId,
        werte: &BTreeMap<String, Feldwert>,
        basis: Option<i64>,
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        let (original, pdf) = self.load_document(id).await?;
        if let Some(b) = basis
            && b != original.version
        {
            return Err(CoreError::VersionConflict {
                expected: b,
                actual: original.version,
            });
        }
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

    async fn pruefe_planbezug(
        &self,
        projekt: ProjectId,
        g: &crate::cad::baugrenzen::BaugrenzenGeometrie,
    ) -> CoreResult<()> {
        let d = self.repos.documents.latest(g.dokument_id).await?;
        if d.project_id != projekt || d.version != g.dokument_version {
            return Err(CoreError::InvalidInput("Der zugeordnete Plan gehört nicht zu diesem Projekt oder wurde geändert. Baugrenzen erneut zuordnen.".into()));
        }
        Ok(())
    }

    pub async fn projekt_nutzung_pruefen(
        &self,
        id: ProjectId,
        eingabe: &crate::festsetzungen::NutzungsEingabe,
    ) -> CoreResult<crate::festsetzungen::NutzungsPruefung> {
        let p = self.repos.projects.get(id).await?;
        if p.planungsrecht.baugrundstuecke.is_some() {
            return Err(CoreError::InvalidInput(
                "Für dieses Projekt die getrennten Baugrundstücksnachweise verwenden".into(),
            ));
        }
        crate::festsetzungen::pruefe(eingabe, &p.planungsrecht, p.version)
    }

    pub async fn projekt_baugrundstuecke_pruefen(
        &self,
        projekt_id: ProjectId,
        eingabe: &crate::baugrundstuecke::BaugrundstuecksPlan,
    ) -> CoreResult<crate::baugrundstuecke::BaugrundstuecksPruefung> {
        let p = self.get_project(projekt_id).await?;
        eingabe.pruefe(&p.planungsrecht, p.version)
    }

    pub fn baugrenzen_pruefen(
        &self,
        g: &crate::cad::baugrenzen::BaugrenzenGeometrie,
    ) -> CoreResult<crate::cad::baugrenzen::BaugrenzenPruefung> {
        crate::cad::baugrenzen::pruefe(g)
    }

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

fn kunde_details(k: &Kunde) -> String {
    let mut d = k.anzeigename();
    if !k.daten.kundennummer.is_empty() {
        d.push_str(&format!(" (Kundennummer {})", k.daten.kundennummer));
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
