use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
    storage::StorageKey,
};
use bytes::Bytes;
impl CoreServices {
    pub async fn einreichungen(&self, projekt: ProjectId) -> CoreResult<Vec<Einreichung>> {
        self.repos.projects.get(projekt).await?;
        self.repos.einreichungen.auflisten(projekt).await
    }
    pub async fn einreichungs_vergleich(
        &self,
        projekt: ProjectId,
        empfaenger: &str,
    ) -> CoreResult<Vec<DokumentVergleich>> {
        let chronik = self.einreichungen(projekt).await?;
        let docs = crate::projektablage::versionen(self.list_documents(projekt).await?, false);
        let kategorien = self.kategoriesnapshots(projekt).await?;
        let mut vergleich = vergleichen(projekt, empfaenger, docs, &chronik);
        for v in &mut vergleich {
            v.kategorie = kategorien
                .get(&v.aktuell.document_id.0.to_string())
                .cloned()
                .flatten();
        }
        Ok(vergleich)
    }
    /// Neue Einreichungspakete entstehen nur noch für Testdaten. Einreichen
    /// und Nachreichen laufen über die Abgabestände der Verfahrensakte;
    /// bestehende Pakete und ihre Bestätigungen bleiben lesbar und exportierbar.
    #[cfg(feature = "testdaten")]
    pub async fn einreichung_erstellen(
        &self,
        projekt: ProjectId,
        input: NeuesPaket,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.einreichung_erstellen_vorgang(projekt, input, None, wer)
            .await
    }
    #[cfg(feature = "testdaten")]
    pub async fn einreichung_erstellen_vorgang(
        &self,
        projekt: ProjectId,
        input: NeuesPaket,
        vorgang: Option<Uuid>,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.repos.projects.get(projekt).await?;
        pruefen(&input)?;
        let id = vorgang
            .map(|v| Uuid::new_v5(&projekt.0, format!("einreichung/{v}").as_bytes()))
            .unwrap_or_else(Uuid::new_v4);
        match self.repos.einreichungen.laden(id).await {
            Ok(e) => {
                if e.stand.projekt_id == projekt
                    && e.stand.titel == input.titel.trim()
                    && e.stand.empfaenger == input.empfaenger.trim()
                    && e.stand.erstellt_von == wer.anzeigename
                    && e.stand.dokumente.len() == input.dokumente.len()
                    && e.stand
                        .dokumente
                        .iter()
                        .zip(&input.dokumente)
                        .all(|(a, b)| a.document_id == b.dokument_id && a.version == b.version)
                {
                    return Ok(e);
                }
                return Err(CoreError::InvalidInput(
                    "Vorgangskennung gehört bereits zu einem anderen Paket".into(),
                ));
            }
            Err(CoreError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
        let mut dokumente = Vec::new();
        let mut inhalte = Vec::new();
        let mut bytes = 0u64;
        for d in &input.dokumente {
            let meta = self
                .repos
                .documents
                .history(d.dokument_id)
                .await?
                .into_iter()
                .find(|v| v.version == d.version)
                .ok_or_else(|| CoreError::NotFound(format!("Dokumentversion {}", d.version)))?;
            if meta.project_id != projekt {
                return Err(CoreError::InvalidInput(
                    "Unterlage gehört zu einem anderen Projekt".into(),
                ));
            }
            bytes = bytes.saturating_add(meta.size_bytes);
            if bytes > 512 * 1024 * 1024 {
                return Err(CoreError::InvalidInput(
                    "Einreichungspaket darf unkomprimiert höchstens 512 MB umfassen".into(),
                ));
            }
            let (_, data) = self.load_version(d.dokument_id, d.version).await?;
            dokumente.push(meta);
            inhalte.push(data.to_vec());
        }
        let mut kategorien = self.kategoriesnapshots(projekt).await?;
        kategorien.retain(|id, _| {
            input
                .dokumente
                .iter()
                .any(|d| d.dokument_id.0.to_string() == *id)
        });
        let stand = PaketStand {
            kategorien,
            id,
            pruefstand: Some(self.paket_pruefstand(projekt, &input.dokumente).await?),
            projekt_id: projekt,
            titel: input.titel.trim().into(),
            empfaenger: input.empfaenger.trim().into(),
            erstellt_am: Utc::now(),
            erstellt_von: wer.anzeigename.clone(),
            dokumente,
        };
        let zip = packen(&stand, &inhalte)?;
        let key = StorageKey::new(format!(
            "projekte/{projekt}/einreichungen/{}/{}.zip",
            stand.id,
            Uuid::new_v4()
        ))?;
        let meta = self.storage.save(&key, Bytes::from(zip)).await?;
        let paket = Einreichung {
            stand,
            zip_key: key.to_string(),
            zip_hash: meta.sha256,
            size: meta.size_bytes,
            version: 1,
            einreichung: None,
        };
        let audit = Self::audit(
            wer,
            crate::domain::Aktion::EinreichungErstellt,
            Some(projekt),
            None,
            format!(
                "{}: {} Unterlagen, Empfänger {}",
                paket.stand.titel,
                paket.stand.dokumente.len(),
                paket.stand.empfaenger
            ),
        );
        match self.repos.commit.einreichung(paket, None, audit).await {
            Ok(p) => {
                if p.zip_key != key.to_string() {
                    let _ = self.storage.delete(&key).await;
                }
                Ok(p)
            }
            Err(e) => {
                let _ = self.storage.delete(&key).await;
                Err(e)
            }
        }
    }

    #[cfg(feature = "testdaten")]
    pub async fn paket_pruefstand(
        &self,
        p: ProjectId,
        auswahl: &[DokumentAuswahl],
    ) -> CoreResult<PaketPruefstand> {
        use crate::plattform::{
            ObjektArt,
            objekte::{Unterlage, UnterlagenStatus},
        };
        let unterlagen = self.fachobjekte(p, ObjektArt::Unterlagen).await?;
        let mut s = PaketPruefstand {
            schema_version: 1,
            ermittelt_am: Utc::now(),
            unterlagen: unterlagen.clone(),
            nachweise: vec![],
            nicht_enthaltene_pflichtunterlagen: vec![],
            ungepruefte_unterlagen: vec![],
            hinweise: vec![],
        };
        if unterlagen.is_empty() {
            s.hinweise.push("Keine Unterlagenanforderungen erfasst. Die Vollständigkeit ist damit nicht geprüft.".into());
        }
        for f in &unterlagen {
            match serde_json::from_value::<Unterlage>(f.daten.clone()) {
                Ok(u) => {
                    let enthalten = u.dokument.as_ref().is_some_and(|d| {
                        auswahl
                            .iter()
                            .any(|a| a.dokument_id == d.dokument_id && a.version == d.version)
                    });
                    if u.erforderlich && !enthalten {
                        s.nicht_enthaltene_pflichtunterlagen
                            .push(u.bezeichnung.clone());
                    }
                    if enthalten && u.status != UnterlagenStatus::Freigegeben {
                        s.ungepruefte_unterlagen.push(u.bezeichnung);
                    }
                }
                Err(e) => s.hinweise.push(format!(
                    "Unterlagenstand {} konnte nicht bewertet werden: {e}",
                    f.objekt
                )),
            }
        }
        match self.nachweise(p).await {
            Ok(n) => s.nachweise = n,
            Err(e) => s.hinweise.push(format!(
                "Nachweisaktualität konnte nicht ermittelt werden: {e}"
            )),
        }
        Ok(s)
    }

    /// Einreichungspakete werden nicht mehr bestätigt: „Eingereicht“ hat eine
    /// einzige Quelle, die Verfahrensakte (versendeter Abgabestand oder
    /// externe Einreichung). Bereits bestätigte Pakete bleiben lesbar.
    pub async fn einreichung_bestaetigen(
        &self,
        id: Uuid,
        _basis: i64,
        _datum: NaiveDate,
        _notiz: String,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.repos.einreichungen.laden(id).await?;
        Err(CoreError::InvalidInput(
            "Frühere Einreichungspakete werden nicht mehr bestätigt. Die Einreichung in der \
             Verfahrensakte als Versand eines Abgabestands oder mit „Externe Einreichung \
             nachtragen“ erfassen."
                .into(),
        ))
    }
    /// Bestätigung im früheren Verfahren, nur zum Aufbau von Altbeständen in Tests.
    #[cfg(feature = "testdaten")]
    pub async fn einreichung_bestaetigen_altbestand(
        &self,
        id: Uuid,
        basis: i64,
        datum: NaiveDate,
        notiz: String,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        if datum > crate::kalender::heute() || notiz.len() > 4000 {
            return Err(CoreError::InvalidInput(
                "Einreichungsdatum darf nicht in der Zukunft liegen; Notiz maximal 4000 Zeichen"
                    .into(),
            ));
        }
        let b = EinreichungsBestaetigung {
            datum,
            notiz: notiz.trim().into(),
            bestaetigt_am: Utc::now(),
            bestaetigt_von: wer.anzeigename.clone(),
        };
        let mut e = self.repos.einreichungen.laden(id).await?;
        if e.version != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: e.version,
            });
        }
        if e.einreichung.is_some() {
            return Err(CoreError::InvalidInput(
                "Einreichung bereits bestätigt; neue Unterlagen als eigenes Paket nachreichen"
                    .into(),
            ));
        }
        e.einreichung = Some(b);
        e.version += 1;
        let audit = Self::audit(
            wer,
            crate::domain::Aktion::EinreichungBestaetigt,
            Some(e.stand.projekt_id),
            None,
            format!(
                "{} an {}, eingereicht am {datum}",
                e.stand.titel, e.stand.empfaenger
            ),
        );
        self.repos.commit.einreichung(e, Some(basis), audit).await
    }

    pub async fn einreichung_zip(&self, id: Uuid) -> CoreResult<(Einreichung, Bytes)> {
        let e = self.repos.einreichungen.laden(id).await?;
        let data = self
            .storage
            .load_verified(&StorageKey::new(e.zip_key.clone())?, &e.zip_hash)
            .await?;
        Ok((e, data))
    }
    /// Schreibt das gespeicherte Einreichungs-ZIP nach `ziel` (Download, Datei).
    /// Gleichzeitige Exporte sind je Prozess begrenzt; das Paket liegt dabei nur
    /// einmal im Speicher und wird nach dem Schreiben freigegeben.
    pub async fn einreichung_zip_schreiben<W: std::io::Write + Send>(
        &self,
        id: Uuid,
        mut ziel: W,
    ) -> CoreResult<(Einreichung, W)> {
        let _platz = crate::exportgrenze::platz().await?;
        let (e, data) = self.einreichung_zip(id).await?;
        ziel.write_all(&data)
            .and_then(|_| ziel.flush())
            .map_err(|f| CoreError::Storage(format!("Einreichungspaket nicht geschrieben: {f}")))?;
        Ok((e, ziel))
    }
}
