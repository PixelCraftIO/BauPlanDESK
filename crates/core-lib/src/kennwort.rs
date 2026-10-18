//! Kennwörter der Benutzeranmeldung: Regeln, Hash und Sperre nach Fehlversuchen.
//!
//! Reine Funktionen ohne Datenbank. Gespeichert wird ausschließlich der
//! Argon2id-Hash im PHC-Format (`$argon2id$v=19$m=…,t=…,p=…$salz$hash`); die
//! Parameter stehen im Hash selbst, sodass ältere Hashes nach einer späteren
//! Anhebung der Parameter weiter geprüft werden können.
//!
//! Abläufe (Anmeldung, Erstkennwort, Übergang) stehen in [`crate::anmeldung`].

use std::sync::OnceLock;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use chrono::{DateTime, Duration, Utc};

use crate::error::{CoreError, CoreResult};

/// Mindestlänge in Zeichen (nicht Bytes). Bewusst keine Zeichenklassen-Regeln:
/// Länge schützt besser, und erzwungene Sonderzeichen führen zu Notizzetteln.
pub const MINDESTLAENGE: usize = 10;

/// Höchstlänge in Zeichen; begrenzt den Aufwand einer einzelnen Prüfung.
pub const HOECHSTLAENGE: usize = 128;

/// So viele Fehlversuche in Folge bleiben ohne Wartezeit.
pub const FREIE_FEHLVERSUCHE: u32 = 4;

/// Wartezeit nach dem ersten Fehlversuch über [`FREIE_FEHLVERSUCHE`]; jede
/// weitere Fehleingabe verdoppelt sie bis höchstens [`HOECHSTE_SPERRE_MINUTEN`].
pub const ERSTE_SPERRE_SEKUNDEN: i64 = 30;

pub const HOECHSTE_SPERRE_MINUTEN: i64 = 15;

/// Prüft ein neues Kennwort gegen die Regeln. `anmeldename` ist der
/// normalisierte Anmeldename; das Kennwort darf ihn nicht nur wiederholen.
pub fn regeln_pruefen(kennwort: &str, anmeldename: &str) -> CoreResult<()> {
    let laenge = kennwort.chars().count();
    if laenge < MINDESTLAENGE {
        return Err(CoreError::InvalidInput(format!(
            "Das Kennwort muss mindestens {MINDESTLAENGE} Zeichen lang sein."
        )));
    }
    if laenge > HOECHSTLAENGE {
        return Err(CoreError::InvalidInput(format!(
            "Das Kennwort darf höchstens {HOECHSTLAENGE} Zeichen lang sein."
        )));
    }
    if kennwort.chars().any(char::is_control) {
        return Err(CoreError::InvalidInput(
            "Das Kennwort darf keine Steuerzeichen enthalten.".into(),
        ));
    }
    if kennwort.trim().to_lowercase() == anmeldename.trim().to_lowercase() {
        return Err(CoreError::InvalidInput(
            "Das Kennwort darf nicht dem Anmeldenamen entsprechen.".into(),
        ));
    }
    let mut zeichen: Vec<char> = kennwort.chars().collect();
    zeichen.sort_unstable();
    zeichen.dedup();
    if zeichen.len() < 3 {
        return Err(CoreError::InvalidInput(
            "Das Kennwort besteht aus zu wenigen verschiedenen Zeichen.".into(),
        ));
    }
    Ok(())
}

fn argon2() -> Argon2<'static> {
    // Empfohlene Mindestparameter für Argon2id: 19 MiB, 2 Durchläufe, 1 Spur.
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

/// Argon2id-Hash mit zufälligem Salz im PHC-Format. Prüft die Regeln nicht;
/// dafür ist [`regeln_pruefen`] zuständig.
pub fn hash_erzeugen(kennwort: &str) -> CoreResult<String> {
    let mut salz = [0u8; 16];
    getrandom::fill(&mut salz)
        .map_err(|e| CoreError::InvalidInput(format!("Zufallsquelle nicht verfügbar: {e}")))?;
    let salz = SaltString::encode_b64(&salz)
        .map_err(|e| CoreError::InvalidInput(format!("Salz nicht erzeugt: {e}")))?;
    argon2()
        .hash_password(kennwort.as_bytes(), &salz)
        .map(|h| h.to_string())
        .map_err(|e| CoreError::InvalidInput(format!("Kennwort-Hash nicht erzeugt: {e}")))
}

/// `true`, wenn `kennwort` zum gespeicherten Hash passt. Ein beschädigter
/// Hash ist ein Datenfehler und wird nicht als falsches Kennwort verdeckt.
pub fn pruefen(kennwort: &str, hash: &str) -> CoreResult<bool> {
    let hash = PasswordHash::new(hash)
        .map_err(|e| CoreError::Database(format!("gespeicherter Kennwort-Hash ungültig: {e}")))?;
    if hash.algorithm.as_str() != "argon2id" {
        return Err(CoreError::Database(format!(
            "gespeicherter Kennwort-Hash mit unerwartetem Verfahren: {}",
            hash.algorithm
        )));
    }
    match argon2().verify_password(kennwort.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(CoreError::Database(format!(
            "Kennwort-Hash nicht prüfbar: {e}"
        ))),
    }
}

/// Prüfung gegen einen festen Hash, damit ein unbekannter Anmeldename oder
/// ein Konto ohne Kennwort nicht an der Antwortzeit erkennbar ist.
pub fn blindpruefung(kennwort: &str) {
    static BLIND: OnceLock<Option<String>> = OnceLock::new();
    if let Some(hash) = BLIND.get_or_init(|| hash_erzeugen("blindpruefung-ohne-konto").ok()) {
        let _ = pruefen(kennwort, hash);
    }
}

/// Wartezeit nach `fehlversuche` aufeinanderfolgenden Fehleingaben,
/// `None` solange keine Wartezeit gilt.
pub fn sperrdauer(fehlversuche: u32) -> Option<Duration> {
    let ueber = fehlversuche.checked_sub(FREIE_FEHLVERSUCHE)?.checked_sub(1);
    let stufe = ueber?;
    let sekunden = ERSTE_SPERRE_SEKUNDEN
        .saturating_mul(1i64.checked_shl(stufe.min(30)).unwrap_or(i64::MAX))
        .min(HOECHSTE_SPERRE_MINUTEN * 60);
    Some(Duration::seconds(sekunden))
}

/// Ende der Sperre nach `fehlversuche` Fehleingaben ab `jetzt`.
pub fn gesperrt_bis(fehlversuche: u32, jetzt: DateTime<Utc>) -> Option<DateTime<Utc>> {
    sperrdauer(fehlversuche).map(|d| jetzt + d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_ist_argon2id_mit_salz_und_prueft() {
        let a = hash_erzeugen("Baugrube-2026!").unwrap();
        let b = hash_erzeugen("Baugrube-2026!").unwrap();
        assert!(a.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{a}");
        assert_ne!(a, b, "jedes Kennwort bekommt ein eigenes Salz");
        assert!(!a.contains("Baugrube"));
        assert!(pruefen("Baugrube-2026!", &a).unwrap());
        assert!(!pruefen("baugrube-2026!", &a).unwrap());
        assert!(!pruefen("", &a).unwrap());
    }

    #[test]
    fn beschaedigter_hash_ist_fehler_kein_falsches_kennwort() {
        assert!(matches!(
            pruefen("x", "kein-hash"),
            Err(CoreError::Database(_))
        ));
    }

    #[test]
    fn regeln() {
        assert!(regeln_pruefen("kurz", "anna").is_err());
        assert!(regeln_pruefen("123456789", "anna").is_err());
        assert!(regeln_pruefen("1234567890", "anna").is_ok());
        // Zeichen, nicht Bytes
        assert!(regeln_pruefen("äöüäöüäöüß", "anna").is_ok());
        assert!(regeln_pruefen("aaaaaaaaaaaa", "anna").is_err());
        assert!(regeln_pruefen("Mueller.Bau", "mueller.bau").is_err());
        assert!(regeln_pruefen(&"x1y".repeat(50), "anna").is_err());
        assert!(regeln_pruefen("Kennwort\n12345", "anna").is_err());
    }

    #[test]
    fn sperre_waechst_und_ist_begrenzt() {
        for n in 0..=FREIE_FEHLVERSUCHE {
            assert_eq!(sperrdauer(n), None, "{n}");
        }
        assert_eq!(
            sperrdauer(FREIE_FEHLVERSUCHE + 1),
            Some(Duration::seconds(30))
        );
        assert_eq!(
            sperrdauer(FREIE_FEHLVERSUCHE + 2),
            Some(Duration::seconds(60))
        );
        assert_eq!(
            sperrdauer(FREIE_FEHLVERSUCHE + 10),
            Some(Duration::minutes(15))
        );
        assert_eq!(sperrdauer(u32::MAX), Some(Duration::minutes(15)));
    }
}
