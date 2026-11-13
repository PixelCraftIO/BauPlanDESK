//! Signierte Lizenzdateien (`.bplizenz`).
//!
//! Eine Lizenzdatei ist JSON mit den Lizenzdaten (Base64) und einer
//! Ed25519-Signatur darüber. Signiert wird beim Hersteller mit dem geheimen
//! Schlüssel (Werkzeug `tools/lizenz-werkzeug`); die App kennt nur den
//! öffentlichen Schlüssel und kann Lizenzen prüfen, aber nicht erzeugen.
//!
//! Die Daten liegen als Base64 vor, damit die signierten Bytes beim Lesen
//! exakt dieselben sind wie beim Signieren (keine JSON-Normalisierung nötig).

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chrono::NaiveDate;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Edition;
use crate::error::{CoreError, CoreResult};

/// Aktuelles Format der Lizenzdatei.
const FORMAT: u32 = 1;

/// Öffentliche Schlüssel des Herstellers, beim Release-Build über die
/// Umgebungsvariable `BAUPROJEKT_LIZENZ_SCHLUESSEL` eingebaut (Base64,
/// mehrere durch Komma getrennt, z. B. beim Schlüsselwechsel).
const HERSTELLER_SCHLUESSEL: Option<&str> = option_env!("BAUPROJEKT_LIZENZ_SCHLUESSEL");

/// Öffentliche Schlüssel des Lizenzservers für Tickets (siehe
/// [`super::ticket`]), beim Release-Build über `BAUPROJEKT_TICKET_SCHLUESSEL`
/// eingebaut. Getrennt vom Herstellerschlüssel: Ein kompromittierter Server
/// kann damit nur kurzlebige, rechnergebundene Tickets ausstellen, aber keine
/// unbefristeten Lizenzdateien.
const TICKET_SCHLUESSEL: Option<&str> = option_env!("BAUPROJEKT_TICKET_SCHLUESSEL");

/// Was eine signierte Datei enthält. Lizenzdateien haben das Feld nicht (sie
/// sind älter), Tickets tragen `ticket`. So lässt sich keine Datei als die
/// jeweils andere Art ausgeben.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Dateiart {
    #[default]
    Lizenz,
    Ticket,
}

/// Inhalt einer Lizenz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lizenz {
    pub lizenz_id: Uuid,
    pub lizenznehmer: String,
    pub edition: Edition,
    /// Gleichzeitig nutzbare Arbeitsplätze.
    pub arbeitsplaetze: u32,
    pub ausgestellt_am: NaiveDate,
    /// Letzter Tag der Nutzung; `None` = unbefristet (Kauflizenz).
    pub gueltig_bis: Option<NaiveDate>,
    /// Letzter Tag, bis zu dem Updates enthalten sind.
    pub wartung_bis: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize)]
struct Lizenzdatei {
    format: u32,
    #[serde(default, skip_serializing_if = "ist_lizenz")]
    art: Dateiart,
    daten: String,
    signatur: String,
}

fn ist_lizenz(art: &Dateiart) -> bool {
    *art == Dateiart::Lizenz
}

/// Signiert eine Lizenz und liefert den Inhalt der Lizenzdatei.
pub fn ausstellen(lizenz: &Lizenz, schluessel: &SigningKey) -> String {
    signieren(Dateiart::Lizenz, lizenz, schluessel)
}

pub(super) fn signieren<T: Serialize>(
    art: Dateiart,
    inhalt: &T,
    schluessel: &SigningKey,
) -> String {
    let daten = serde_json::to_vec(inhalt).expect("Inhalt ist serialisierbar");
    let signatur = schluessel.sign(&daten);
    let datei = Lizenzdatei {
        format: FORMAT,
        art,
        daten: B64.encode(&daten),
        signatur: B64.encode(signatur.to_bytes()),
    };
    serde_json::to_string_pretty(&datei).expect("Lizenzdatei ist serialisierbar")
}

/// Signatur einer Datei der Art `art` gegen `schluessel` prüfen; liefert die
/// signierten Daten.
pub(super) fn signatur_pruefen(
    inhalt: &[u8],
    art: Dateiart,
    schluessel: &[VerifyingKey],
) -> CoreResult<Vec<u8>> {
    let fehler = |m: &str| CoreError::License(m.to_string());
    let name = match art {
        Dateiart::Lizenz => "Lizenzdatei",
        Dateiart::Ticket => "Lizenzbestätigung",
    };
    let datei: Lizenzdatei = serde_json::from_slice(inhalt)
        .map_err(|_| fehler(&format!("Die Datei ist keine BauPlanDESK-{name}.")))?;
    if datei.format != FORMAT {
        return Err(fehler(&format!(
            "Die {name} hat ein neueres Format. Bitte aktualisieren Sie BauPlanDESK."
        )));
    }
    if datei.art != art {
        return Err(fehler(&format!("Die Datei ist keine BauPlanDESK-{name}.")));
    }
    let beschaedigt = || fehler(&format!("Die {name} ist beschädigt."));
    let daten = B64.decode(&datei.daten).map_err(|_| beschaedigt())?;
    let signatur = B64
        .decode(&datei.signatur)
        .ok()
        .and_then(|b| Signature::from_slice(&b).ok())
        .ok_or_else(beschaedigt)?;
    if !schluessel
        .iter()
        .any(|k| k.verify(&daten, &signatur).is_ok())
    {
        return Err(fehler(&format!(
            "Die Signatur der {name} ist ungültig. Die Datei wurde verändert oder stammt nicht vom Hersteller."
        )));
    }
    Ok(daten)
}

/// Öffentlicher Schlüssel als Base64 (für `BAUPROJEKT_LIZENZ_SCHLUESSEL`).
pub fn oeffentlich_base64(schluessel: &SigningKey) -> String {
    B64.encode(schluessel.verifying_key().to_bytes())
}

/// Prüft Lizenzdateien gegen die vertrauenswürdigen öffentlichen Schlüssel.
#[derive(Clone)]
pub struct Lizenzpruefer {
    schluessel: Vec<VerifyingKey>,
    pub(super) ticket_schluessel: Vec<VerifyingKey>,
}

impl Lizenzpruefer {
    pub fn new(schluessel: Vec<VerifyingKey>) -> Self {
        Self {
            schluessel,
            ticket_schluessel: Vec::new(),
        }
    }

    /// Zusätzlich Tickets des Lizenzservers mit diesen Schlüsseln anerkennen.
    pub fn mit_ticket_schluesseln(mut self, schluessel: Vec<VerifyingKey>) -> Self {
        self.ticket_schluessel = schluessel;
        self
    }

    /// Schlüssel aus Base64-Texten (je 32 Byte).
    pub fn aus_base64<'a>(texte: impl IntoIterator<Item = &'a str>) -> CoreResult<Self> {
        Ok(Self::new(schluessel_aus_base64(texte)?))
    }
}

/// Öffentliche Schlüssel aus Base64-Texten (je 32 Byte); leere Einträge
/// werden übersprungen.
pub fn schluessel_aus_base64<'a>(
    texte: impl IntoIterator<Item = &'a str>,
) -> CoreResult<Vec<VerifyingKey>> {
    let mut schluessel = Vec::new();
    for t in texte {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        let bytes: [u8; 32] = B64
            .decode(t)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| CoreError::License(format!("ungültiger öffentlicher Schlüssel: {t}")))?;
        schluessel.push(
            VerifyingKey::from_bytes(&bytes)
                .map_err(|e| CoreError::License(format!("ungültiger Schlüssel: {e}")))?,
        );
    }
    Ok(schluessel)
}

impl Lizenzpruefer {
    /// Die im Programm eingebauten Schlüssel: der des Herstellers und der des
    /// Lizenzservers (falls beim Build gesetzt) und in Debug-Builds zusätzlich
    /// die Testschlüssel.
    pub fn standard() -> Self {
        let eingebaut = |wert: Option<&str>, name: &str| {
            wert.map(|s| schluessel_aus_base64(s.split(',')))
                .transpose()
                .unwrap_or_else(|e| panic!("{name} ist ungültig: {e}"))
                .unwrap_or_default()
        };
        let mut p = Self::new(eingebaut(
            HERSTELLER_SCHLUESSEL,
            "BAUPROJEKT_LIZENZ_SCHLUESSEL",
        ))
        .mit_ticket_schluesseln(eingebaut(TICKET_SCHLUESSEL, "BAUPROJEKT_TICKET_SCHLUESSEL"));
        if cfg!(debug_assertions) {
            p.schluessel.push(testschluessel::oeffentlich());
            p.ticket_schluessel
                .push(testschluessel::ticket_oeffentlich());
        }
        p
    }

    /// Ob Tickets eines Lizenzservers geprüft werden können (sonst sind nur
    /// Lizenzdateien möglich).
    pub fn hat_ticket_schluessel(&self) -> bool {
        !self.ticket_schluessel.is_empty()
    }

    /// Ob überhaupt ein Schlüssel bekannt ist (sonst ist nur die
    /// Testversion möglich).
    pub fn hat_schluessel(&self) -> bool {
        !self.schluessel.is_empty()
    }

    /// Liest und prüft eine Lizenzdatei. Das Ablaufdatum prüft der Aufrufer.
    pub fn pruefen(&self, inhalt: &[u8]) -> CoreResult<Lizenz> {
        let fehler = |m: &str| CoreError::License(m.to_string());
        let daten = signatur_pruefen(inhalt, Dateiart::Lizenz, &self.schluessel)?;
        let lizenz: Lizenz = serde_json::from_slice(&daten)
            .map_err(|_| fehler("Die Lizenzdatei ist beschädigt."))?;
        if lizenz.arbeitsplaetze == 0 {
            return Err(fehler("Die Lizenz enthält keine Arbeitsplätze."));
        }
        Ok(lizenz)
    }
}

/// Fester Testschlüssel. Debug-Builds vertrauen ihm, Release-Builds nicht;
/// damit lassen sich Lizenzen in Tests und bei der Entwicklung ausstellen.
pub mod testschluessel {
    use ed25519_dalek::{SigningKey, VerifyingKey};

    const GEHEIM: [u8; 32] = *b"BauProjektIO-Testschluessel-v1!!";
    const TICKET_GEHEIM: [u8; 32] = *b"BauProjektIO-Testticket-v1!!!!!!";

    pub fn oeffentlich() -> VerifyingKey {
        SigningKey::from_bytes(&GEHEIM).verifying_key()
    }

    /// Testschlüssel für Tickets eines Lizenzservers.
    pub fn ticket_oeffentlich() -> VerifyingKey {
        SigningKey::from_bytes(&TICKET_GEHEIM).verifying_key()
    }

    /// Nur für Tests und Beispiele.
    #[cfg(any(test, feature = "testdaten"))]
    pub fn geheim() -> SigningKey {
        SigningKey::from_bytes(&GEHEIM)
    }

    /// Nur für Tests und einen lokalen Lizenzserver bei der Entwicklung.
    #[cfg(any(test, feature = "testdaten"))]
    pub fn ticket_geheim() -> SigningKey {
        SigningKey::from_bytes(&TICKET_GEHEIM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lizenz() -> Lizenz {
        Lizenz {
            lizenz_id: Uuid::new_v4(),
            lizenznehmer: "Architekturbüro Muster".into(),
            edition: Edition::Buero,
            arbeitsplaetze: 5,
            ausgestellt_am: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
            gueltig_bis: None,
            wartung_bis: NaiveDate::from_ymd_opt(2027, 10, 13),
        }
    }

    fn pruefer() -> Lizenzpruefer {
        Lizenzpruefer::new(vec![testschluessel::oeffentlich()])
    }

    #[test]
    fn signierte_lizenz_wird_erkannt() {
        let l = lizenz();
        let datei = ausstellen(&l, &testschluessel::geheim());
        assert_eq!(pruefer().pruefen(datei.as_bytes()).unwrap(), l);
    }

    #[test]
    fn veraenderte_lizenz_wird_abgelehnt() {
        let datei = ausstellen(&lizenz(), &testschluessel::geheim());
        // Arbeitsplätze von 5 auf 50 „erhöhen“.
        let mut json: serde_json::Value = serde_json::from_str(&datei).unwrap();
        let daten = B64.decode(json["daten"].as_str().unwrap()).unwrap();
        let manipuliert = String::from_utf8(daten)
            .unwrap()
            .replace("\"arbeitsplaetze\":5", "\"arbeitsplaetze\":50");
        json["daten"] = B64.encode(manipuliert).into();
        let err = pruefer().pruefen(json.to_string().as_bytes()).unwrap_err();
        assert!(err.to_string().contains("Signatur"), "{err}");
    }

    #[test]
    fn fremder_schluessel_wird_abgelehnt() {
        let fremd = SigningKey::from_bytes(&[3; 32]);
        let datei = ausstellen(&lizenz(), &fremd);
        assert!(pruefer().pruefen(datei.as_bytes()).is_err());
        // Mit dessen öffentlichem Schlüssel klappt es.
        let p = Lizenzpruefer::aus_base64([oeffentlich_base64(&fremd).as_str()]).unwrap();
        assert!(p.pruefen(datei.as_bytes()).is_ok());
    }

    #[test]
    fn unsinn_wird_abgelehnt() {
        for inhalt in [
            &b"kein json"[..],
            b"{}",
            br#"{"format":1,"daten":"!","signatur":""}"#,
        ] {
            assert!(matches!(
                pruefer().pruefen(inhalt),
                Err(CoreError::License(_))
            ));
        }
    }

    #[test]
    fn standard_vertraut_im_debug_build_dem_testschluessel() {
        let datei = ausstellen(&lizenz(), &testschluessel::geheim());
        assert_eq!(
            Lizenzpruefer::standard().pruefen(datei.as_bytes()).is_ok(),
            cfg!(debug_assertions)
        );
    }
}
