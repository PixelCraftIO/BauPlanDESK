//! Abo-Lizenzen über den Lizenzserver.
//!
//! Der Administrator gibt einmal einen Lizenzschlüssel ein; er liegt danach in
//! der gemeinsamen Datenbank. Jeder Rechner holt sich damit beim Lizenzserver
//! ein **Ticket**: eine signierte Bestätigung mit Ablaufzeitpunkt (Standard
//! 30 Tage), die an die Machine-ID dieses Rechners gebunden ist. Die App prüft
//! das Ticket lokal gegen den eingebauten öffentlichen Schlüssel des Servers,
//! also auch ohne Internet. Im Hintergrund erneuert sie es, sobald der Server
//! erreichbar ist. Läuft es ab, bleibt alles lesbar, nur Änderungen sind
//! gesperrt.
//!
//! Die Anfrage- und Antworttypen hier nutzen App und Lizenzserver gemeinsam.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

use super::Lizenz;
use super::datei::{Dateiart, Lizenzpruefer, signatur_pruefen, signieren};
use crate::error::{CoreError, CoreResult};

/// Inhalt eines Tickets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    /// Lizenzdaten wie bei einer Lizenzdatei. `gueltig_bis` ist hier das
    /// Ende einer fest befristeten Lizenz; bei Abos `None`, dort begrenzt
    /// das Ticket selbst.
    pub lizenz: Lizenz,
    /// Rechner, für den das Ticket gilt (siehe [`Ticket::machine_id`]).
    pub machine_id: String,
    pub ausgestellt_um: DateTime<Utc>,
    /// Bis dahin darf der Rechner ohne Kontakt zum Server arbeiten.
    pub gueltig_bis: DateTime<Utc>,
}

impl Ticket {
    pub fn ist_gueltig(&self, jetzt: DateTime<Utc>) -> bool {
        jetzt <= self.gueltig_bis
    }
}

/// Signiert ein Ticket (nur auf dem Lizenzserver).
pub fn ticket_ausstellen(ticket: &Ticket, schluessel: &SigningKey) -> String {
    signieren(Dateiart::Ticket, ticket, schluessel)
}

impl Lizenzpruefer {
    /// Prüft Signatur und Machine-ID eines Tickets. Ob es abgelaufen ist,
    /// prüft der Aufrufer ([`Ticket::ist_gueltig`]): Ein abgelaufenes Ticket
    /// sagt immer noch, für welche Lizenz der Rechner zuletzt bestätigt war.
    pub fn ticket_pruefen(&self, inhalt: &[u8], machine_id: &str) -> CoreResult<Ticket> {
        let daten = signatur_pruefen(inhalt, Dateiart::Ticket, &self.ticket_schluessel)?;
        let ticket: Ticket = serde_json::from_slice(&daten)
            .map_err(|_| CoreError::License("Die Lizenzbestätigung ist beschädigt.".into()))?;
        if ticket.machine_id != machine_id {
            return Err(CoreError::License(
                "Die Lizenzbestätigung gehört zu einem anderen Rechner.".into(),
            ));
        }
        Ok(ticket)
    }
}

// --- Protokoll zwischen App und Lizenzserver ------------------------------

/// `POST /v1/ticket`: Rechner aktivieren bzw. Ticket erneuern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAnfrage {
    pub lizenzschluessel: String,
    pub machine_id: String,
    /// Nur zur Anzeige in der Verwaltung des Herstellers.
    pub rechnername: String,
    #[serde(default)]
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketAntwort {
    /// Signiertes Ticket, so wie es der Rechner speichert.
    pub ticket: String,
}

/// `POST /v1/abmelden`: Rechner gibt seinen Platz frei (Rechnerwechsel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbmeldeAnfrage {
    pub lizenzschluessel: String,
    pub machine_id: String,
}

/// Warum der Server kein Ticket ausstellt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ablehnungsgrund {
    /// Schlüssel nicht bekannt (Tippfehler?).
    Unbekannt,
    /// Abo gekündigt, Zahlung offen oder befristete Lizenz abgelaufen.
    Inaktiv,
    /// Vom Hersteller gesperrt.
    Gesperrt,
    /// Alle Plätze sind von anderen Rechnern belegt.
    KeinePlaetze,
}

/// Fehlerantwort des Servers (HTTP 4xx).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ablehnung {
    pub grund: Ablehnungsgrund,
    pub meldung: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DienstFehler {
    /// Der Server hat entschieden: kein Ticket für diesen Rechner. Ein
    /// vorhandenes Ticket wird verworfen.
    Abgelehnt(String),
    /// Kein Internet, Server nicht erreichbar oder gestört: später erneut
    /// versuchen, das vorhandene Ticket gilt weiter.
    NichtErreichbar(String),
}

impl std::fmt::Display for DienstFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Abgelehnt(m) => f.write_str(m),
            Self::NichtErreichbar(m) => write!(f, "Der Lizenzserver ist nicht erreichbar: {m}"),
        }
    }
}

/// Zugang zum Lizenzserver. Die Desktop-App implementiert ihn über HTTP;
/// core-lib selbst hat keinen HTTP-Client.
#[async_trait]
pub trait Lizenzdienst: Send + Sync {
    /// Liefert den Inhalt eines signierten Tickets.
    async fn ticket(&self, anfrage: &TicketAnfrage) -> Result<String, DienstFehler>;

    async fn abmelden(&self, anfrage: &AbmeldeAnfrage) -> Result<(), DienstFehler>;
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate};
    use uuid::Uuid;

    use super::*;
    use crate::license::Edition;
    use crate::license::datei::{ausstellen, testschluessel};

    fn ticket(machine_id: &str) -> Ticket {
        let jetzt = Utc::now();
        Ticket {
            lizenz: Lizenz {
                lizenz_id: Uuid::new_v4(),
                lizenznehmer: "Architekturbüro Muster".into(),
                edition: Edition::Buero,
                arbeitsplaetze: 3,
                ausgestellt_am: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
                gueltig_bis: None,
                wartung_bis: None,
            },
            machine_id: machine_id.into(),
            ausgestellt_um: jetzt,
            gueltig_bis: jetzt + Duration::days(30),
        }
    }

    fn pruefer() -> Lizenzpruefer {
        Lizenzpruefer::new(vec![testschluessel::oeffentlich()])
            .mit_ticket_schluesseln(vec![testschluessel::ticket_oeffentlich()])
    }

    #[test]
    fn ticket_gilt_nur_auf_seinem_rechner() {
        let t = ticket("rechner-a");
        let datei = ticket_ausstellen(&t, &testschluessel::ticket_geheim());
        assert_eq!(
            pruefer()
                .ticket_pruefen(datei.as_bytes(), "rechner-a")
                .unwrap(),
            t
        );
        let err = pruefer()
            .ticket_pruefen(datei.as_bytes(), "rechner-b")
            .unwrap_err();
        assert!(err.to_string().contains("anderen Rechner"), "{err}");
    }

    #[test]
    fn verlaengertes_ticket_wird_abgelehnt() {
        let datei = ticket_ausstellen(&ticket("a"), &testschluessel::ticket_geheim());
        let mut json: serde_json::Value = serde_json::from_str(&datei).unwrap();
        let mut t: Ticket = serde_json::from_slice(
            &base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                json["daten"].as_str().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        t.gueltig_bis += Duration::days(365);
        json["daten"] = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            serde_json::to_vec(&t).unwrap(),
        )
        .into();
        let err = pruefer()
            .ticket_pruefen(json.to_string().as_bytes(), "a")
            .unwrap_err();
        assert!(err.to_string().contains("Signatur"), "{err}");
    }

    #[test]
    fn ticket_und_lizenzdatei_sind_nicht_austauschbar() {
        // Ticket mit dem Herstellerschlüssel signiert: zählt nicht als Ticket.
        let t = ticket("a");
        let mit_hersteller = ticket_ausstellen(&t, &testschluessel::geheim());
        assert!(
            pruefer()
                .ticket_pruefen(mit_hersteller.as_bytes(), "a")
                .is_err()
        );
        // Lizenzdatei wird nicht als Ticket angenommen und umgekehrt.
        let lizenz = ausstellen(&t.lizenz, &testschluessel::ticket_geheim());
        assert!(pruefer().pruefen(lizenz.as_bytes()).is_err());
        let echtes = ticket_ausstellen(&t, &testschluessel::ticket_geheim());
        assert!(pruefer().pruefen(echtes.as_bytes()).is_err());
    }

    #[test]
    fn ablauf() {
        let t = ticket("a");
        assert!(t.ist_gueltig(t.gueltig_bis));
        assert!(!t.ist_gueltig(t.gueltig_bis + Duration::seconds(1)));
    }
}
