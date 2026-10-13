//! Stripe-Webhooks: Signatur prüfen und Ereignisse auf Lizenzen abbilden.
//!
//! Der Server ruft die Stripe-API nie selbst auf; er braucht deshalb keinen
//! API-Schlüssel, nur das Signing-Secret des Webhook-Endpunkts (`whsec_…`).
//!
//! Verarbeitet werden:
//! * `checkout.session.completed` (Abo-Checkout bezahlt): Lizenz anlegen,
//!   Lizenznehmer und E-Mail übernehmen.
//! * `customer.subscription.created|updated|deleted`: Status (aktiv bei
//!   `active`, `trialing`, `past_due`), Anzahl Plätze (Menge der Positionen),
//!   Periodenende.
//!
//! Die Edition kommt aus den Metadaten `edition` (Checkout-Sitzung, Payment
//! Link oder Abo), sonst `BAUPROJEKT_STANDARD_EDITION`.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

use crate::db::{AboStand, Kauf, edition_aus_text};

/// So alt darf eine Signatur höchstens sein (Schutz vor Wiedereinspielen).
const TOLERANZ_SEKUNDEN: i64 = 300;

/// Prüft den Header `Stripe-Signature: t=…,v1=…[,v1=…]`.
pub fn signatur_pruefen(
    header: &str,
    body: &[u8],
    secret: &str,
    jetzt: DateTime<Utc>,
) -> Result<(), &'static str> {
    let mut zeit = None;
    let mut signaturen = Vec::new();
    for teil in header.split(',') {
        match teil.trim().split_once('=') {
            Some(("t", t)) => zeit = t.parse::<i64>().ok(),
            Some(("v1", s)) => signaturen.extend(hex::decode(s).ok()),
            _ => {}
        }
    }
    let zeit = zeit.ok_or("Zeitstempel fehlt")?;
    if (jetzt.timestamp() - zeit).abs() > TOLERANZ_SEKUNDEN {
        return Err("Zeitstempel außerhalb der Toleranz");
    }
    let passt = signaturen.iter().any(|sig| {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC nimmt jede Länge");
        mac.update(zeit.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        mac.verify_slice(sig).is_ok()
    });
    if passt {
        Ok(())
    } else {
        Err("Signatur passt nicht")
    }
}

/// Signatur-Header erzeugen (für Tests und `stripe trigger`-freie Versuche).
pub fn signatur_erzeugen(body: &[u8], secret: &str, zeit: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC");
    mac.update(zeit.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!("t={zeit},v1={}", hex::encode(mac.finalize().into_bytes()))
}

/// Was aus einem Ereignis folgt.
#[derive(Debug)]
pub enum Folge {
    Kauf(Kauf),
    Abo(AboStand),
    Nichts,
}

fn text(v: &Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

/// `customer` ist je nach Expand-Einstellung eine ID oder ein Objekt.
fn id_von(v: &Value) -> Option<String> {
    text(v).or_else(|| text(&v["id"]))
}

pub fn auswerten(ereignis: &Value) -> Folge {
    let objekt = &ereignis["data"]["object"];
    let edition = text(&objekt["metadata"]["edition"]).and_then(|e| edition_aus_text(&e));
    match ereignis["type"].as_str().unwrap_or_default() {
        "checkout.session.completed" => {
            if objekt["mode"].as_str() != Some("subscription") {
                return Folge::Nichts;
            }
            let (Some(checkout), Some(abo)) =
                (text(&objekt["id"]), id_von(&objekt["subscription"]))
            else {
                return Folge::Nichts;
            };
            let details = &objekt["customer_details"];
            // Firmenname aus einem Zusatzfeld „lizenznehmer“ des Checkouts,
            // sonst der Name der zahlenden Person.
            let feld = objekt["custom_fields"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|f| f["key"].as_str() == Some("lizenznehmer"))
                .and_then(|f| text(&f["text"]["value"]));
            let email = text(&details["email"]);
            let lizenznehmer = feld
                .or_else(|| text(&details["business_name"]))
                .or_else(|| text(&details["name"]))
                .or_else(|| email.clone())
                .unwrap_or_default();
            Folge::Kauf(Kauf {
                checkout,
                abo,
                kunde: id_von(&objekt["customer"]),
                lizenznehmer,
                email,
                edition,
            })
        }
        typ @ ("customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted") => {
            let Some(abo) = text(&objekt["id"]) else {
                return Folge::Nichts;
            };
            let positionen = objekt["items"]["data"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let menge: u64 = positionen
                .iter()
                .map(|p| p["quantity"].as_u64().unwrap_or(1))
                .sum();
            // Seit API-Version 2025-03-31 steht das Periodenende an den
            // Positionen, davor am Abo.
            let periode_bis = objekt["current_period_end"]
                .as_i64()
                .or_else(|| {
                    positionen
                        .iter()
                        .filter_map(|p| p["current_period_end"].as_i64())
                        .max()
                })
                .and_then(|t| DateTime::from_timestamp(t, 0));
            let status = objekt["status"].as_str().unwrap_or_default();
            Folge::Abo(AboStand {
                abo,
                kunde: id_von(&objekt["customer"]),
                aktiv: typ != "customer.subscription.deleted"
                    && matches!(status, "active" | "trialing" | "past_due"),
                arbeitsplaetze: u32::try_from(menge.max(1)).unwrap_or(u32::MAX),
                edition,
                periode_bis,
                stand: ereignis["created"].as_i64().unwrap_or_default(),
            })
        }
        _ => Folge::Nichts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatur() {
        let jetzt = Utc::now();
        let body = br#"{"id":"evt_1"}"#;
        let header = signatur_erzeugen(body, "whsec_test", jetzt.timestamp());
        assert!(signatur_pruefen(&header, body, "whsec_test", jetzt).is_ok());
        assert!(signatur_pruefen(&header, body, "whsec_anders", jetzt).is_err());
        assert!(signatur_pruefen(&header, b"{}", "whsec_test", jetzt).is_err());
        let spaeter = jetzt + chrono::Duration::minutes(10);
        assert!(signatur_pruefen(&header, body, "whsec_test", spaeter).is_err());
        // Mehrere v1-Signaturen (beim Wechsel des Secrets): eine reicht.
        let zwei = format!("{header},v1={}", "00".repeat(32));
        assert!(signatur_pruefen(&zwei, body, "whsec_test", jetzt).is_ok());
        assert!(signatur_pruefen("v1=abc", body, "whsec_test", jetzt).is_err());
    }

    #[test]
    fn abo_neue_api_version() {
        let e: Value = serde_json::json!({
            "type": "customer.subscription.updated",
            "created": 1_760_000_000,
            "data": {"object": {
                "id": "sub_1", "customer": "cus_1", "status": "past_due",
                "metadata": {"edition": "Einzelplatz"},
                "items": {"data": [
                    {"quantity": 3, "current_period_end": 1_762_000_000},
                    {"quantity": 2, "current_period_end": 1_762_000_000}
                ]}
            }}
        });
        let Folge::Abo(a) = auswerten(&e) else {
            panic!()
        };
        assert!(a.aktiv);
        assert_eq!(a.arbeitsplaetze, 5);
        assert_eq!(a.edition, Some(core_lib::license::Edition::Einzelplatz));
        assert_eq!(a.periode_bis.unwrap().timestamp(), 1_762_000_000);
        assert_eq!(a.stand, 1_760_000_000);
    }
}
