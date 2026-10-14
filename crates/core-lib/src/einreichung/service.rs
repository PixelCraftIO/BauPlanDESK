use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
    domain::Aktion,
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
        Ok(vergleichen(projekt, empfaenger, docs, &chronik))
    }
    pub async fn einreichung_erstellen(
        &self,
        projekt: ProjectId,
        input: NeuesPaket,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.repos.projects.get(projekt).await?;
        pruefen(&input)?;
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
        let stand = PaketStand {
            id: Uuid::new_v4(),
            projekt_id: projekt,
            titel: input.titel.trim().into(),
            empfaenger: input.empfaenger.trim().into(),
            erstellt_am: Utc::now(),
            erstellt_von: wer.anzeigename.clone(),
            dokumente,
        };
        let zip = packen(&stand, &inhalte)?;
        let key = StorageKey::new(format!("projekte/{projekt}/einreichungen/{}.zip", stand.id))?;
        let meta = self.storage.save(&key, Bytes::from(zip)).await?;
        let paket = Einreichung {
            stand,
            zip_key: key.to_string(),
            zip_hash: meta.sha256,
            size: meta.size_bytes,
            version: 1,
            einreichung: None,
        };
        if let Err(e) = self.repos.einreichungen.anlegen(&paket).await {
            let _ = self.storage.delete(&key).await;
            return Err(e);
        }
        self.protokollieren(
            wer,
            Aktion::EinreichungErstellt,
            Some(projekt),
            None,
            format!(
                "{}: {} Unterlagen, Empfänger {}",
                paket.stand.titel,
                paket.stand.dokumente.len(),
                paket.stand.empfaenger
            ),
        )
        .await?;
        Ok(paket)
    }
    pub async fn einreichung_bestaetigen(
        &self,
        id: Uuid,
        basis: i64,
        datum: NaiveDate,
        notiz: String,
        wer: &Benutzer,
    ) -> CoreResult<Einreichung> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        if datum > Utc::now().date_naive() || notiz.len() > 4000 {
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
        let e = self.repos.einreichungen.bestaetigen(id, basis, b).await?;
        self.protokollieren(
            wer,
            Aktion::EinreichungBestaetigt,
            Some(e.stand.projekt_id),
            None,
            format!(
                "{} an {}, eingereicht am {datum}",
                e.stand.titel, e.stand.empfaenger
            ),
        )
        .await?;
        Ok(e)
    }
    pub async fn einreichung_zip(&self, id: Uuid) -> CoreResult<(Einreichung, Bytes)> {
        let e = self.repos.einreichungen.laden(id).await?;
        let data = self
            .storage
            .load_verified(&StorageKey::new(e.zip_key.clone())?, &e.zip_hash)
            .await?;
        Ok((e, data))
    }
}
