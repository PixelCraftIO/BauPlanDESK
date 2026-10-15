//! Einzelne Anhänge werden bewusst als eigene Unterlage übernommen. Die Quelle
//! bleibt eine unveränderliche konkrete Nachrichtenversion.
use super::*;
use crate::{benutzer::Benutzer, commit::DokumentZusatz, domain::Aktion};
impl CoreServices {
    pub async fn email_anhang_ablegen(
        &self,
        id: DocumentId,
        version: i64,
        index: usize,
        wer: &Benutzer,
    ) -> CoreResult<DocumentVersion> {
        let (original, bytes) = self.load_version(id, version).await?;
        self.projekt_zugriff(
            original.project_id,
            wer,
            crate::zugriff::ProjektRecht::Bearbeiten,
        )
        .await?;
        let inhalt = lesen(&original.file_name, &bytes)?;
        let a = inhalt
            .anhaenge
            .get(index)
            .ok_or_else(|| fehler("Anhang nicht vorhanden"))?;
        if a.eingebettete_nachricht {
            return Err(fehler("Eingebettete Nachrichten im Mailprogramm öffnen"));
        }
        let daten = if original.file_name.to_lowercase().ends_with(".eml") {
            let m = MessageParser::default()
                .parse(&bytes)
                .ok_or_else(|| fehler("EML nicht lesbar"))?;
            let part = m
                .attachments()
                .nth(index)
                .ok_or_else(|| fehler("Anhang fehlt"))?;
            if part.is_encoding_problem {
                return Err(fehler("Anhang hat einen Kodierungsfehler"));
            }
            part.contents().to_vec()
        } else {
            msg::anhang(&bytes, index)?
        };
        if daten.is_empty() || daten.len() as u64 > MAX_BYTES {
            return Err(fehler("Anhang muss zwischen 1 Byte und 32 MB groß sein"));
        }
        let vorgang =
            uuid::Uuid::new_v5(&id.0, format!("email-anhang/{version}/{index}").as_bytes());
        self.ablegen_vorgang(original.project_id, None, None, &a.name, bytes::Bytes::from(daten), wer, Aktion::DokumentAbgelegt, "Anhang aus archivierter E-Mail übernommen", Some(vorgang), vec![DokumentZusatz {
            modul: "emailquelle".into(), schema_version: 1,
            daten: serde_json::json!({"dokument_id":id,"version":version,"sha256":original.sha256,"anhang":index}),
        }]).await
    }
}
