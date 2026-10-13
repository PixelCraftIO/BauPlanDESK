//! Verbindung zum Lizenzserver (Abo-Lizenzen) und Machine-ID dieses Rechners.

use std::time::Duration;

use async_trait::async_trait;
use core_lib::license::ticket::{
    Ablehnung, AbmeldeAnfrage, DienstFehler, Lizenzdienst, TicketAnfrage, TicketAntwort,
};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};

/// Adresse des Lizenzservers. Zur Laufzeit über die Umgebungsvariable
/// `BAUPROJEKT_LIZENZSERVER` (Entwicklung, eigener Server zum Testen),
/// sonst die beim Release-Build eingebaute.
pub fn server_url() -> Option<String> {
    std::env::var("BAUPROJEKT_LIZENZSERVER")
        .ok()
        .or_else(|| option_env!("BAUPROJEKT_LIZENZSERVER").map(str::to_string))
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty())
}

/// Kennung dieses Rechners, an die der Lizenzserver das Ticket bindet.
///
/// Grundlage ist die Kennung, die das Betriebssystem selbst für den Rechner
/// führt (Windows: `MachineGuid`, macOS: Hardware-UUID, Linux:
/// `/etc/machine-id`). Seriennummern von CPU und Mainboard sind ohne
/// Administratorrechte oft nicht lesbar und fehlen in vielen Rechnern ganz.
/// Gehasht mit einem festen Präfix, damit der Server die echte Kennung nie
/// sieht.
pub fn machine_id() -> Result<String, String> {
    let roh = machine_uid::get().map_err(|e| format!("Rechnerkennung nicht lesbar: {e}"))?;
    let roh = roh.trim();
    if roh.is_empty() {
        return Err("Rechnerkennung ist leer".into());
    }
    let hash = Sha256::digest(format!("BauProjektIO-Machine-v1:{roh}").as_bytes());
    Ok(hex::encode(&hash[..16]))
}

pub struct HttpLizenzdienst {
    client: reqwest::Client,
    url: String,
}

impl HttpLizenzdienst {
    pub fn new(url: String) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            // Kurz halten: Beim Start wartet die App auf die Bestätigung
            // (höchstens einmal am Tag), ohne Internet scheitert sie sofort.
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .user_agent(concat!("BauProjektIO/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client, url })
    }

    async fn senden<T: serde::Serialize>(
        &self,
        pfad: &str,
        body: &T,
    ) -> Result<reqwest::Response, DienstFehler> {
        let res = self
            .client
            .post(format!("{}{pfad}", self.url))
            .json(body)
            .send()
            .await
            .map_err(|e| DienstFehler::NichtErreichbar(e.to_string()))?;
        if res.status().is_success() {
            return Ok(res);
        }
        let status = res.status();
        // Nur eine echte Entscheidung des Servers (Ablehnung mit Grund) lässt
        // das Ticket verfallen; Proxyfehler, 5xx usw. gelten als Störung.
        if status.is_client_error()
            && status != StatusCode::TOO_MANY_REQUESTS
            && let Ok(a) = res.json::<Ablehnung>().await
        {
            return Err(DienstFehler::Abgelehnt(a.meldung));
        }
        Err(DienstFehler::NichtErreichbar(format!("HTTP {status}")))
    }
}

#[async_trait]
impl Lizenzdienst for HttpLizenzdienst {
    async fn ticket(&self, anfrage: &TicketAnfrage) -> Result<String, DienstFehler> {
        let antwort: TicketAntwort = self
            .senden("/v1/ticket", anfrage)
            .await?
            .json()
            .await
            .map_err(|e| DienstFehler::NichtErreichbar(e.to_string()))?;
        Ok(antwort.ticket)
    }

    async fn abmelden(&self, anfrage: &AbmeldeAnfrage) -> Result<(), DienstFehler> {
        self.senden("/v1/abmelden", anfrage).await.map(|_| ())
    }
}
