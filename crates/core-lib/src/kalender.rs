//! Kalendertage fachlicher Vorgänge gelten in deutscher Ortszeit.
//! UTC bleibt das Format für Zeitstempel und Auditdaten.
use crate::domain::Bundesland;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};

pub fn datum_deutsch(zeitpunkt: DateTime<Utc>) -> NaiveDate {
    zeitpunkt
        .with_timezone(&chrono_tz::Europe::Berlin)
        .date_naive()
}

pub fn heute() -> NaiveDate {
    datum_deutsch(Utc::now())
}

/// Ostersonntag im gregorianischen Kalender (anonymer Algorithmus nach Meeus).
pub fn ostersonntag(jahr: i32) -> NaiveDate {
    let (a, b, c) = (jahr % 19, jahr / 100, jahr % 100);
    let (d, e) = (b / 4, b % 4);
    let g = (b - (b + 8) / 25 + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let n = h + l - 7 * m + 114;
    NaiveDate::from_ymd_opt(jahr, (n / 31) as u32, (n % 31 + 1) as u32)
        .expect("Ostersonntag liegt im März oder April")
}

/// Landesweit geltende gesetzliche Feiertage. Feiertage einzelner Gemeinden
/// oder Landesteile (Fronleichnam in Teilen Sachsens und Thüringens, Mariä
/// Himmelfahrt in Bayern, Augsburger Friedensfest) sind nicht enthalten.
pub fn gesetzlicher_feiertag(land: Bundesland, d: NaiveDate) -> bool {
    use Bundesland::*;
    let j = d.year();
    let ostern = ostersonntag(j);
    let am = |m: u32, t: u32| NaiveDate::from_ymd_opt(j, m, t) == Some(d);
    let nach_ostern = |tage: i64| ostern + Duration::days(tage) == d;
    let in_ = |laender: &[Bundesland]| laender.contains(&land);
    let buss_und_bettag = {
        let grenze = NaiveDate::from_ymd_opt(j, 11, 22).expect("gültiges Datum");
        let zurueck =
            (grenze.weekday().num_days_from_monday() + 7 - Weekday::Wed.num_days_from_monday()) % 7;
        grenze - Duration::days(i64::from(zurueck))
    };
    am(1, 1)
        || nach_ostern(-2)
        || nach_ostern(1)
        || am(5, 1)
        || nach_ostern(39)
        || nach_ostern(50)
        || am(10, 3)
        || am(12, 25)
        || am(12, 26)
        || (am(1, 6) && in_(&[Bw, By, St]))
        || (am(3, 8) && ((land == Be && j >= 2019) || (land == Mv && j >= 2023)))
        || (am(5, 8) && land == Be && (j == 2020 || j == 2025))
        || (nach_ostern(60) && in_(&[Bw, By, He, Nw, Rp, Sl]))
        || (am(8, 15) && land == Sl)
        || (am(9, 20) && land == Th && j >= 2019)
        || (am(10, 31)
            && (j == 2017 || in_(&[Bb, Mv, Sn, St, Th]) || (j >= 2018 && in_(&[Hb, Hh, Ni, Sh]))))
        || (am(11, 1) && in_(&[Bw, By, Nw, Rp, Sl]))
        || (d == buss_und_bettag && land == Sn)
}

/// Verschiebt ein Fristende von Samstag, Sonntag oder einem gesetzlichen
/// Feiertag auf den nächsten Werktag (§ 222 Abs. 2 ZPO, § 193 BGB).
pub fn naechster_werktag(land: Bundesland, mut d: NaiveDate) -> NaiveDate {
    while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) || gesetzlicher_feiertag(land, d) {
        d += Duration::days(1);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn ostern_und_bewegliche_feiertage() {
        for (jahr, ostern) in [
            (2024, "2024-03-31"),
            (2025, "2025-04-20"),
            (2026, "2026-04-05"),
            (2027, "2027-03-28"),
            (2038, "2038-04-25"),
        ] {
            assert_eq!(ostersonntag(jahr), tag(ostern));
        }
        assert!(gesetzlicher_feiertag(Bundesland::Nw, tag("2026-06-04")));
        assert!(!gesetzlicher_feiertag(Bundesland::Ni, tag("2026-06-04")));
        assert!(gesetzlicher_feiertag(Bundesland::Sn, tag("2026-11-18")));
        assert!(!gesetzlicher_feiertag(Bundesland::By, tag("2026-11-18")));
        assert!(gesetzlicher_feiertag(Bundesland::Ni, tag("2026-10-31")));
        assert!(!gesetzlicher_feiertag(Bundesland::Ni, tag("2016-10-31")));
        assert!(gesetzlicher_feiertag(Bundesland::Mv, tag("2026-03-08")));
        assert!(!gesetzlicher_feiertag(Bundesland::Mv, tag("2022-03-08")));
        assert!(gesetzlicher_feiertag(Bundesland::Th, tag("2026-09-20")));
    }

    #[test]
    fn fristende_rueckt_auf_den_naechsten_werktag() {
        // Samstag → Montag; Karfreitag bis Ostermontag → Dienstag.
        assert_eq!(
            naechster_werktag(Bundesland::By, tag("2026-10-17")),
            tag("2026-10-19")
        );
        assert_eq!(
            naechster_werktag(Bundesland::He, tag("2026-04-03")),
            tag("2026-04-07")
        );
        assert_eq!(
            naechster_werktag(Bundesland::Bw, tag("2026-10-20")),
            tag("2026-10-20")
        );
    }

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
