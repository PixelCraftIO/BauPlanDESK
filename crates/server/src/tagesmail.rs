//! Versand der täglichen Fristen-E-Mail über den SMTP-Zugang des Büros.
//!
//! Aktiv nur, wenn `BAUPROJEKT_SMTP_URL` gesetzt ist, z. B.
//! `smtps://benutzer:kennwort@mail.example.de:465` oder
//! `smtp://benutzer:kennwort@mail.example.de:587?tls=required`.
//! `BAUPROJEKT_MAIL_ABSENDER` ist die Absenderadresse,
//! `BAUPROJEKT_MAIL_UHRZEIT` die Versandzeit in deutscher Ortszeit (Standard 06:30).
//! Der Versand eines Tages wird in der Datenbank beansprucht, damit mehrere
//! Serverinstanzen nicht doppelt senden. Fehler einzelner Empfänger werden
//! protokolliert und brechen den Lauf nicht ab.
use chrono::{NaiveTime, Utc};
use core_lib::CoreServices;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};
use std::time::Duration;

pub struct Mailversand {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    absender: Mailbox,
    uhrzeit: NaiveTime,
}

impl Mailversand {
    /// Liest die Umgebung; `Ok(None)`, wenn kein SMTP-Zugang eingerichtet ist.
    pub fn aus_umgebung() -> Result<Option<Self>, String> {
        let Ok(url) = std::env::var("BAUPROJEKT_SMTP_URL") else {
            return Ok(None);
        };
        let absender = std::env::var("BAUPROJEKT_MAIL_ABSENDER")
            .map_err(|_| "BAUPROJEKT_MAIL_ABSENDER fehlt".to_string())?
            .parse::<Mailbox>()
            .map_err(|e| format!("BAUPROJEKT_MAIL_ABSENDER: {e}"))?;
        let uhrzeit = std::env::var("BAUPROJEKT_MAIL_UHRZEIT").unwrap_or_else(|_| "06:30".into());
        let uhrzeit = NaiveTime::parse_from_str(&uhrzeit, "%H:%M")
            .map_err(|e| format!("BAUPROJEKT_MAIL_UHRZEIT (HH:MM): {e}"))?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(&url)
            .map_err(|e| format!("BAUPROJEKT_SMTP_URL: {e}"))?
            .build();
        Ok(Some(Self {
            transport,
            absender,
            uhrzeit,
        }))
    }

    /// Sendet die Nachrichten des Tages, falls die Versandzeit erreicht ist und
    /// keine andere Instanz den Tag übernommen hat. Liefert die Zahl der Nachrichten.
    pub async fn ausfuehren(&self, core: &CoreServices) -> Result<usize, String> {
        let jetzt = Utc::now().with_timezone(&chrono_tz::Europe::Berlin);
        if jetzt.time() < self.uhrzeit {
            return Ok(0);
        }
        let heute = jetzt.date_naive();
        if !core
            .tagesmail_beanspruchen(heute)
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(0);
        }
        let mails = core.tagesmails(heute).await.map_err(|e| e.to_string())?;
        let mut gesendet = 0;
        for m in mails {
            let an = match m.adresse.parse::<Mailbox>() {
                Ok(a) => a,
                Err(e) => {
                    tracing::warn!(benutzer = %m.empfaenger.0, fehler = %e, "Tagesmail: Adresse ungültig");
                    continue;
                }
            };
            let nachricht = Message::builder()
                .from(self.absender.clone())
                .to(an)
                .subject(m.betreff)
                .header(ContentType::TEXT_PLAIN)
                .body(m.text);
            let nachricht = match nachricht {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!(benutzer = %m.empfaenger.0, fehler = %e, "Tagesmail nicht erstellt");
                    continue;
                }
            };
            match self.transport.send(nachricht).await {
                Ok(_) => gesendet += 1,
                Err(e) => {
                    tracing::warn!(benutzer = %m.empfaenger.0, fehler = %e, "Tagesmail nicht versendet")
                }
            }
        }
        Ok(gesendet)
    }

    /// Prüft alle zehn Minuten, ob der Tagesversand fällig ist.
    pub fn starten(self, core: CoreServices) {
        tokio::spawn(async move {
            let mut takt = tokio::time::interval(Duration::from_secs(600));
            loop {
                takt.tick().await;
                match self.ausfuehren(&core).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(nachrichten = n, "Tägliche Fristen-E-Mail versendet"),
                    Err(e) => {
                        tracing::error!(fehler = %e, "Tägliche Fristen-E-Mail fehlgeschlagen")
                    }
                }
            }
        });
    }
}
