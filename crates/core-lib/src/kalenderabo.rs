//! Kalender-Abo: persönliche, abonnierbare Kalenderadresse für die Fristen
//! aller lesbaren Projekte (Netzwerkbetrieb mit BauPlanIO-Server).
//!
//! Vertrag:
//! - Ein Benutzer besitzt höchstens ein Abo. Anlegen erzeugt einen neuen
//!   zufälligen Schlüssel (256 Bit) und ersetzt einen früheren; gespeichert wird
//!   nur der SHA-256-Wert des Schlüssels unter `kalenderabo.<benutzer>`.
//! - Der Schlüssel wird nur beim Anlegen zurückgegeben. Wer ihn kennt, kann die
//!   Termine lesen, die der Benutzer lesen darf; Projektrechte werden bei jedem
//!   Abruf neu geprüft (entzogene Mitgliedschaft = Termine verschwinden).
//! - Umfang `meine`: nur Termine, für die der Benutzer zuständig ist.
//! - Der Abruf liefert dieselben Termine wie [`crate::cockpit`] als RFC-5545-Kalender
//!   ([`crate::verfahren::termine::ics`]); Widerruf löscht den Schlüssel.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, BenutzerId},
    verfahren::termine::{Termin, ics, termine},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Welche Termine das Abo enthält.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aboumfang {
    /// Alle laufenden Termine der lesbaren Projekte.
    Alle,
    /// Nur Termine mit dem Benutzer als Zuständigem.
    Meine,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Abodaten {
    schluessel_sha256: String,
    umfang: Aboumfang,
    erstellt_am: DateTime<Utc>,
}

/// Stand des eigenen Abos für die Oberfläche (ohne Schlüssel).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Aboauskunft {
    pub aktiv: bool,
    pub umfang: Option<Aboumfang>,
    pub erstellt_am: Option<DateTime<Utc>>,
}

/// Ergebnis des Anlegens: einmalig sichtbarer Schlüssel und Pfad auf dem Server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Aboschluessel {
    pub umfang: Aboumfang,
    /// Pfad relativ zur Serveradresse, z. B. `/kalender/<benutzer>/<schluessel>.ics`.
    pub pfad: String,
}

fn einstellung(id: BenutzerId) -> String {
    format!("kalenderabo.{}", id.0)
}
fn hash(schluessel: &str) -> String {
    hex::encode(Sha256::digest(schluessel.as_bytes()))
}
/// Vergleich in konstanter Zeit (gleich lange Hex-Werte).
fn gleich(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}
fn neuer_schluessel() -> CoreResult<String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| CoreError::Storage(format!("Zufallsquelle: {e}")))?;
    Ok(hex::encode(b))
}
/// Nur Hex-Zeichen, 64 lang: alles andere wird ohne Datenbankzugriff abgewiesen.
fn schluessel_form(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
fn nicht_gefunden() -> CoreError {
    CoreError::NotFound("Kalender nicht gefunden".into())
}

impl CoreServices {
    async fn abodaten(&self, id: BenutzerId) -> CoreResult<Option<Abodaten>> {
        match self.repos.lizenz.einstellung(&einstellung(id)).await? {
            Some(t) if !t.is_empty() => serde_json::from_str(&t)
                .map(Some)
                .map_err(|e| CoreError::InvalidInput(e.to_string())),
            _ => Ok(None),
        }
    }
    pub async fn kalenderabo_auskunft(&self, wer: &Benutzer) -> CoreResult<Aboauskunft> {
        let wer = self.angemeldeter_benutzer(wer).await?;
        let d = self.abodaten(wer.id).await?;
        Ok(Aboauskunft {
            aktiv: d.is_some(),
            umfang: d.as_ref().map(|d| d.umfang),
            erstellt_am: d.map(|d| d.erstellt_am),
        })
    }
    /// Legt ein neues Abo an; ein bisheriger Schlüssel wird damit ungültig.
    pub async fn kalenderabo_anlegen(
        &self,
        wer: &Benutzer,
        umfang: Aboumfang,
    ) -> CoreResult<Aboschluessel> {
        let wer = self.angemeldeter_benutzer(wer).await?;
        let schluessel = neuer_schluessel()?;
        let d = Abodaten {
            schluessel_sha256: hash(&schluessel),
            umfang,
            erstellt_am: Utc::now(),
        };
        self.repos
            .lizenz
            .einstellung_setzen(
                &einstellung(wer.id),
                &serde_json::to_string(&d).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
            )
            .await?;
        Ok(Aboschluessel {
            umfang,
            pfad: format!("/kalender/{}/{schluessel}.ics", wer.id.0),
        })
    }
    pub async fn kalenderabo_widerrufen(&self, wer: &Benutzer) -> CoreResult<()> {
        let wer = self.angemeldeter_benutzer(wer).await?;
        self.repos
            .lizenz
            .einstellung_setzen(&einstellung(wer.id), "")
            .await
    }
    /// Kalenderabruf ohne Sitzung: prüft Schlüssel, Konto und Projektrechte.
    /// Jeder Fehler beim Schlüssel ergibt `NotFound`, damit gültige Konten nicht
    /// erraten werden können.
    pub async fn kalenderabo_ics(
        &self,
        benutzer: BenutzerId,
        schluessel: &str,
        heute: NaiveDate,
    ) -> CoreResult<String> {
        if !schluessel_form(schluessel) {
            return Err(nicht_gefunden());
        }
        let Some(d) = self.abodaten(benutzer).await? else {
            return Err(nicht_gefunden());
        };
        if !gleich(&d.schluessel_sha256, &hash(schluessel)) {
            return Err(nicht_gefunden());
        }
        let wer = self
            .repos
            .benutzer
            .get(benutzer)
            .await
            .map_err(|_| nicht_gefunden())?;
        if !wer.aktiv {
            return Err(nicht_gefunden());
        }
        let mut alle: Vec<Termin> = Vec::new();
        for p in self.projekte_fuer_benutzer(&wer).await? {
            for (id, a) in self.verfahrensakten_lesen(p.id).await? {
                alle.extend(
                    termine(&a, id, p.bundesland)
                        .into_iter()
                        .filter(|t| t.aktiv(heute))
                        .filter(|t| {
                            d.umfang == Aboumfang::Alle || t.verantwortlich == Some(wer.id.0)
                        })
                        .map(|mut t| {
                            t.verfahrensbezeichnung =
                                format!("{} · {}", p.name, t.verfahrensbezeichnung);
                            t
                        }),
                );
            }
        }
        alle.sort_by(|a, b| a.datum.cmp(&b.datum).then(a.titel.cmp(&b.titel)));
        let name = match d.umfang {
            Aboumfang::Alle => "BauPlanIO · Fristen".to_string(),
            Aboumfang::Meine => format!("BauPlanIO · Meine Fristen ({})", wer.anzeigename),
        };
        Ok(ics(&name, &alle, Utc::now()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schluessel_ist_hex_und_wird_gehasht_verglichen() {
        let s = neuer_schluessel().unwrap();
        assert!(schluessel_form(&s));
        assert_ne!(s, neuer_schluessel().unwrap());
        assert!(gleich(&hash(&s), &hash(&s)));
        assert!(!gleich(&hash(&s), &hash("x")));
        assert!(!schluessel_form("../etc/passwd"));
        assert!(!schluessel_form(&s[..63]));
    }
}
