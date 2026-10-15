//! Nur lesender Vertrag für archivierte Nachrichten. Parserdetails bleiben intern;
//! HTML wird als Text ausgegeben, Anhänge werden beschrieben, nicht ausgeführt.
mod anhaenge;
mod msg;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::{DocumentId, DocumentVersion},
};
use mail_parser::{MessageParser, MimeHeaders};
use serde::{Deserialize, Serialize};

pub const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TEXT: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct MailAdresse {
    pub name: String,
    pub adresse: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct MailAnhang {
    pub name: String,
    pub medientyp: Option<String>,
    #[ts(type = "number | null")]
    pub groesse_bytes: Option<u64>,
    pub eingebettete_nachricht: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct EmailInhalt {
    pub format: String,
    pub betreff: String,
    pub absender: Vec<MailAdresse>,
    pub empfaenger: Vec<MailAdresse>,
    pub cc: Vec<MailAdresse>,
    pub bcc: Vec<MailAdresse>,
    /// RFC3339, soweit ein Datum in der Originaldatei vorhanden und lesbar ist.
    pub datum: Option<String>,
    pub nachrichten_id: Option<String>,
    pub text: String,
    pub anhaenge: Vec<MailAnhang>,
    pub hinweise: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ArchivierteEmail {
    pub dokument: DocumentVersion,
    pub inhalt: EmailInhalt,
}
fn fehler(text: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(text.into())
}
pub fn ist_email(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|e| e.eq_ignore_ascii_case("eml") || e.eq_ignore_ascii_case("msg"))
}
pub fn lesen(name: &str, daten: &[u8]) -> CoreResult<EmailInhalt> {
    if daten.len() as u64 > MAX_BYTES {
        return Err(fehler(
            "Die E-Mail-Vorschau unterstützt Dateien bis 32 MB. Das Original kann weiterhin extern geöffnet werden.",
        ));
    }
    let inhalt = match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "eml" => eml(daten)?,
        "msg" => msg::lesen(daten)?,
        _ => {
            return Err(fehler(
                "Erwartet wird eine archivierte E-Mail (.eml oder .msg)",
            ));
        }
    };
    if inhalt.text.len() > MAX_TEXT
        || inhalt.anhaenge.len() > 500
        || [
            &inhalt.absender,
            &inhalt.empfaenger,
            &inhalt.cc,
            &inhalt.bcc,
        ]
        .iter()
        .any(|l| l.len() > 500)
    {
        return Err(fehler(
            "Die E-Mail enthält zu viele Angaben für die Vorschau. Bitte das Original extern öffnen.",
        ));
    }
    Ok(inhalt)
}
fn adressen(liste: Option<&mail_parser::Address<'_>>) -> Vec<MailAdresse> {
    liste
        .map(|l| {
            l.iter()
                .map(|a| MailAdresse {
                    name: a.name.as_deref().unwrap_or("").into(),
                    adresse: a.address.as_deref().unwrap_or("").into(),
                })
                .collect()
        })
        .unwrap_or_default()
}
fn eml(daten: &[u8]) -> CoreResult<EmailInhalt> {
    let mail = MessageParser::default()
        .parse(daten)
        .ok_or_else(|| fehler("Die EML-Datei konnte nicht gelesen werden"))?;
    if !mail.headers().iter().any(|h| {
        matches!(
            h.name,
            mail_parser::HeaderName::From
                | mail_parser::HeaderName::To
                | mail_parser::HeaderName::Cc
                | mail_parser::HeaderName::Bcc
                | mail_parser::HeaderName::Subject
                | mail_parser::HeaderName::Date
                | mail_parser::HeaderName::MessageId
                | mail_parser::HeaderName::MimeVersion
                | mail_parser::HeaderName::ContentType
        )
    }) {
        return Err(fehler("Die Datei enthält keine lesbaren E-Mail-Kopfdaten"));
    }
    if mail.parts.len() > 5000 {
        return Err(fehler("Die E-Mail enthält zu viele MIME-Bestandteile"));
    }
    let mut hinweise = vec![];
    if mail.parts.iter().any(|p| p.is_encoding_problem) {
        hinweise.push("Mindestens ein Nachrichtenteil hat einen Kodierungsfehler; die Vorschau kann unvollständig sein.".into());
    }
    let text = mail
        .body_text(0)
        .map(|s| s.into_owned())
        .unwrap_or_default();
    if text.trim().is_empty() {
        hinweise.push("Kein lesbarer Nachrichtentext vorhanden. Bei verschlüsselten Nachrichten bitte das Original im Mailprogramm öffnen.".into());
    }
    let anhaenge = mail
        .attachments()
        .map(|p| MailAnhang {
            name: p
                .attachment_name()
                .unwrap_or("Anhang ohne Dateinamen")
                .into(),
            medientyp: p.content_type().map(|t| {
                format!(
                    "{}/{}",
                    t.c_type,
                    t.c_subtype.as_deref().unwrap_or("octet-stream")
                )
            }),
            groesse_bytes: Some(p.contents().len() as u64),
            eingebettete_nachricht: matches!(p.body, mail_parser::PartType::Message(_)),
        })
        .collect();
    Ok(EmailInhalt {
        format: "EML".into(),
        betreff: mail.subject().unwrap_or("").into(),
        absender: adressen(mail.from()),
        empfaenger: adressen(mail.to()),
        cc: adressen(mail.cc()),
        bcc: adressen(mail.bcc()),
        datum: mail.date().map(|d| d.to_rfc3339()),
        nachrichten_id: mail.message_id().map(str::to_owned),
        text,
        anhaenge,
        hinweise,
    })
}
impl CoreServices {
    pub async fn archivierte_email(
        &self,
        id: DocumentId,
        version: i64,
    ) -> CoreResult<ArchivierteEmail> {
        let d = self
            .repos
            .documents
            .history(id)
            .await?
            .into_iter()
            .find(|d| d.version == version)
            .ok_or_else(|| CoreError::NotFound(format!("E-Mail-Version {version}")))?;
        if !ist_email(&d.file_name) || d.size_bytes > MAX_BYTES {
            return Err(fehler(
                "E-Mail-Vorschau: .eml oder .msg bis 32 MB erforderlich",
            ));
        }
        let (d, bytes) = self.load_version(id, version).await?;
        let inhalt = lesen(&d.file_name, &bytes)?;
        Ok(ArchivierteEmail {
            dokument: d,
            inhalt,
        })
    }
}
#[cfg(test)]
mod tests;
