//! Kalendertage fachlicher Vorgänge gelten in deutscher Ortszeit.
//! UTC bleibt das Format für Zeitstempel und Auditdaten.
use chrono::{DateTime, NaiveDate, Utc};

pub fn datum_deutsch(zeitpunkt: DateTime<Utc>) -> NaiveDate {
    zeitpunkt
        .with_timezone(&chrono_tz::Europe::Berlin)
        .date_naive()
}

pub fn heute() -> NaiveDate {
    datum_deutsch(Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mitternacht_sommer_winter_und_zeitumstellung() {
        for (utc, datum) in [
            ("2026-01-15T22:59:59Z", "2026-01-15"),
            ("2026-01-15T23:00:00Z", "2026-01-16"),
            ("2026-07-15T21:59:59Z", "2026-07-15"),
            ("2026-07-15T22:00:00Z", "2026-07-16"),
            ("2026-03-29T00:59:59Z", "2026-03-29"),
            ("2026-03-29T01:00:00Z", "2026-03-29"),
            ("2026-10-25T00:59:59Z", "2026-10-25"),
            ("2026-10-25T01:00:00Z", "2026-10-25"),
        ] {
            let zeitpunkt = utc.parse::<DateTime<Utc>>().unwrap();
            assert_eq!(datum_deutsch(zeitpunkt).to_string(), datum);
        }
    }
}
