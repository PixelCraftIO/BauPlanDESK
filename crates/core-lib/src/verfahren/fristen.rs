//! Aus Bescheid, Verfahrensart und Bundesland berechnete Fristen. Die Werte
//! dienen der Wiedervorlage und ersetzen keine rechtliche Prüfung im Einzelfall.
use super::{Bescheid, Entscheidung, Verfahrensart, fehler};
use crate::domain::{Bundesland, ProjectId};
use crate::kalender::naechster_werktag;
use crate::{CoreResult, CoreServices};
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Bescheide einer Akte, auch ungespeicherte, für die Fristberechnung.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fristauftrag {
    pub art: Verfahrensart,
    pub bescheide: Vec<Bescheid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Fristberechnung {
    /// Tag des fristauslösenden Ereignisses; er wird nicht mitgezählt (§ 187 Abs. 1 BGB).
    pub beginn: NaiveDate,
    pub ende: NaiveDate,
    pub grundlage: String,
    pub hinweise: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bescheidfristen {
    pub bescheid: Uuid,
    pub rechtsbehelfsfrist: Option<Fristberechnung>,
    pub geltungsdauer: Option<Fristberechnung>,
    /// Warum eine Frist nicht berechnet wurde.
    pub hinweise: Vec<String>,
}

/// Geltungsdauer der Baugenehmigung in Jahren nach der Landesbauordnung.
const GELTUNGSDAUER: [(Bundesland, u32, &str); 16] = [
    (Bundesland::Bw, 3, "LBO Baden-Württemberg"),
    (Bundesland::By, 4, "BayBO"),
    (Bundesland::Be, 3, "BauO Bln"),
    (Bundesland::Bb, 6, "BbgBO"),
    (Bundesland::Hb, 3, "BremLBO"),
    (Bundesland::Hh, 3, "HBauO"),
    (Bundesland::He, 3, "HBO"),
    (Bundesland::Mv, 3, "LBauO M-V"),
    (Bundesland::Ni, 3, "NBauO"),
    (Bundesland::Nw, 3, "BauO NRW"),
    (Bundesland::Rp, 4, "LBauO Rheinland-Pfalz"),
    (Bundesland::Sl, 3, "LBO Saarland"),
    (Bundesland::Sn, 3, "SächsBO"),
    (Bundesland::St, 3, "BauO LSA"),
    (Bundesland::Sh, 3, "LBO Schleswig-Holstein"),
    (Bundesland::Th, 3, "ThürBO"),
];

/// Monatsfrist nach §§ 187 Abs. 1, 188 Abs. 2 und 3 BGB: Ende am Tag gleicher
/// Zahl, fehlt er, am Monatsletzten.
fn nach_monaten(beginn: NaiveDate, monate: u32) -> NaiveDate {
    beginn
        .checked_add_months(Months::new(monate))
        .unwrap_or(NaiveDate::MAX)
}

pub fn berechnen(land: Bundesland, art: &Verfahrensart, b: &Bescheid) -> Bescheidfristen {
    let mut r = Bescheidfristen {
        bescheid: b.id,
        rechtsbehelfsfrist: None,
        geltungsdauer: None,
        hinweise: Vec::new(),
    };
    let Some(zugestellt) = b.zugestellt else {
        r.hinweise.push(
            "Zustelldatum erfassen, um Rechtsbehelfsfrist und Geltungsdauer zu berechnen.".into(),
        );
        return r;
    };
    r.rechtsbehelfsfrist = Some(Fristberechnung {
        beginn: zugestellt,
        ende: naechster_werktag(land, nach_monaten(zugestellt, 1)),
        grundlage: "Ein Monat nach Bekanntgabe (§§ 70, 74 VwGO); Fristende an Wochenenden \
                    und Feiertagen am nächsten Werktag (§ 57 Abs. 2 VwGO, § 222 Abs. 2 ZPO)"
            .into(),
        hinweise: vec![
            "Gilt nur bei ordnungsgemäßer Rechtsbehelfsbelehrung, sonst ein Jahr (§ 58 Abs. 2 VwGO). \
             Ob Widerspruch oder Klage statthaft ist, steht in der Belehrung."
                .into(),
            "Bei Bekanntgabe mit einfachem Brief gilt der Bescheid am vierten Tag nach Aufgabe \
             zur Post als bekanntgegeben (§ 41 Abs. 2 VwVfG); diesen Tag als Zustelldatum erfassen."
                .into(),
            "Nachbarn laufen eigene Fristen ab ihrer Bekanntgabe. Örtliche Feiertage sind nicht \
             berücksichtigt."
                .into(),
        ],
    });
    let genehmigt = matches!(
        b.entscheidung,
        Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
    );
    let baugenehmigung = matches!(
        art,
        Verfahrensart::Bauantrag
            | Verfahrensart::Aenderungsantrag
            | Verfahrensart::Teilbaugenehmigung
    );
    if genehmigt && baugenehmigung {
        let (_, jahre, gesetz) = GELTUNGSDAUER
            .iter()
            .find(|(l, _, _)| *l == land)
            .expect("Tabelle enthält jedes Bundesland");
        r.geltungsdauer = Some(Fristberechnung {
            beginn: zugestellt,
            ende: nach_monaten(zugestellt, jahre * 12),
            grundlage: format!("{jahre} Jahre nach Erteilung ({gesetz})"),
            hinweise: vec![
                "Die Genehmigung erlischt, wenn bis dahin nicht mit der Ausführung begonnen wurde. \
                 Unterbrechung der Bauausführung und Verlängerung auf Antrag nach der \
                 Landesbauordnung prüfen."
                    .into(),
            ],
        });
    } else if genehmigt {
        r.hinweise.push(
            "Geltungsdauer wird nur für Baugenehmigungen berechnet; für diese Verfahrensart \
             nach Landesrecht prüfen."
                .into(),
        );
    }
    r
}

impl CoreServices {
    /// Fristen der übergebenen Bescheide nach dem Bundesland des Projekts.
    pub async fn bescheidfristen(
        &self,
        p: ProjectId,
        auftrag: &Fristauftrag,
    ) -> CoreResult<Vec<Bescheidfristen>> {
        if auftrag.bescheide.len() > 100 {
            return Err(fehler("Höchstens 100 Bescheide je Berechnung"));
        }
        let land = self.get_project(p).await?.bundesland;
        Ok(auftrag
            .bescheide
            .iter()
            .map(|b| berechnen(land, &auftrag.art, b))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plattform::objekte::Dokumentbezug;

    fn bescheid(entscheidung: Entscheidung, zugestellt: Option<&str>) -> Bescheid {
        Bescheid {
            id: Uuid::new_v4(),
            entscheidung,
            datum: "2026-01-01".parse().unwrap(),
            zugestellt: zugestellt.map(|d| d.parse().unwrap()),
            original: Dokumentbezug {
                dokument_id: crate::domain::DocumentId(Uuid::new_v4()),
                version: 1,
            },
            genehmigte_unterlagen: Vec::new(),
            auflagen: Vec::new(),
            ersetzt: None,
        }
    }
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn rechtsbehelfsfrist_monatsende_und_werktag() {
        let ende = |land, z| {
            berechnen(
                land,
                &Verfahrensart::Bauantrag,
                &bescheid(Entscheidung::Abgelehnt, Some(z)),
            )
            .rechtsbehelfsfrist
            .unwrap()
            .ende
        };
        // Gleicher Tag im Folgemonat.
        assert_eq!(ende(Bundesland::Nw, "2026-03-10"), tag("2026-04-10"));
        // 31.01. → kein 31.02., daher Monatsletzter 28.02.2026 (Samstag) → Montag.
        assert_eq!(ende(Bundesland::Nw, "2026-01-31"), tag("2026-03-02"));
        // Kein Zurückrechnen auf den Monatsletzten, wenn der Tag gleicher Zahl existiert.
        assert_eq!(ende(Bundesland::Nw, "2025-09-30"), tag("2025-10-30"));
        // Fristende an Allerheiligen (Mittwoch): in Bayern Feiertag, in Niedersachsen nicht.
        assert_eq!(ende(Bundesland::By, "2023-10-01"), tag("2023-11-02"));
        assert_eq!(ende(Bundesland::Ni, "2023-10-01"), tag("2023-11-01"));
    }

    #[test]
    fn geltungsdauer_nach_land_und_nur_fuer_genehmigungen() {
        let b = bescheid(Entscheidung::Genehmigt, Some("2028-02-29"));
        let by = berechnen(Bundesland::By, &Verfahrensart::Bauantrag, &b);
        assert_eq!(by.geltungsdauer.unwrap().ende, tag("2032-02-29"));
        let nw = berechnen(Bundesland::Nw, &Verfahrensart::Bauantrag, &b);
        assert_eq!(nw.geltungsdauer.unwrap().ende, tag("2031-02-28"));
        let vorbescheid = berechnen(Bundesland::Nw, &Verfahrensart::Vorbescheid, &b);
        assert!(vorbescheid.geltungsdauer.is_none());
        assert!(!vorbescheid.hinweise.is_empty());
        let abgelehnt = berechnen(
            Bundesland::Nw,
            &Verfahrensart::Bauantrag,
            &bescheid(Entscheidung::Abgelehnt, Some("2026-03-10")),
        );
        assert!(abgelehnt.geltungsdauer.is_none() && abgelehnt.rechtsbehelfsfrist.is_some());
        let offen = berechnen(
            Bundesland::Nw,
            &Verfahrensart::Bauantrag,
            &bescheid(Entscheidung::Genehmigt, None),
        );
        assert!(offen.rechtsbehelfsfrist.is_none() && offen.geltungsdauer.is_none());
        assert!(offen.hinweise[0].contains("Zustelldatum"));
        assert!(
            GELTUNGSDAUER
                .iter()
                .all(|(l, _, _)| Bundesland::ALL.contains(l))
        );
        assert_eq!(GELTUNGSDAUER.len(), Bundesland::ALL.len());
    }
}
