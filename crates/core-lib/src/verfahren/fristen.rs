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
    /// Frist für den Baubeginn (Erlöschen der Genehmigung).
    pub geltungsdauer: Option<Fristberechnung>,
    /// Frist für die Fertigstellung, wo die Landesbauordnung eine eigene
    /// Frist kennt (Berlin).
    pub fertigstellung: Option<Fristberechnung>,
    /// Warum eine Frist nicht berechnet wurde, und Hinweise nach Verfahrensart.
    pub hinweise: Vec<String>,
    /// Stand der Fristregeln, nach dem gerechnet wurde.
    pub regelstand: String,
}

/// Stand der Fristregeln. Bei einer Änderung der Tabelle oder der Regeln
/// anheben; die Fristen werden nicht gespeichert und gelten sofort neu.
pub const REGELSTAND: &str = "2026-10-18";

/// Erlöschen der Baugenehmigung nach Landesbauordnung.
struct Geltungsregel {
    land: Bundesland,
    /// Jahre bis zum Baubeginn.
    jahre: u32,
    /// Jahre bis zur Fertigstellung, falls das Land sie eigens regelt.
    fertigstellung_jahre: Option<u32>,
    quelle: &'static str,
    hinweis: Option<&'static str>,
}
const fn regel(
    land: Bundesland,
    jahre: u32,
    fertigstellung_jahre: Option<u32>,
    quelle: &'static str,
    hinweis: Option<&'static str>,
) -> Geltungsregel {
    Geltungsregel {
        land,
        jahre,
        fertigstellung_jahre,
        quelle,
        hinweis,
    }
}

/// Geltungsdauer der Baugenehmigung je Land (Regelstand [`REGELSTAND`]).
/// Geprüft gegen die geltenden Fassungen am 18.10.2026:
/// BW § 62 Abs. 1 LBO (Stand 16.3.2026), NW § 75 Abs. 1 BauO NRW (Abs. 1
/// unverändert 2024), NI § 71 NBauO (Unterbrechung 3 Jahre), HH § 73 HBauO
/// (Unterbrechung 1 Jahr, Hemmung bei Nachbarrechtsbehelf), BE § 73 Abs. 1
/// BauO Bln (2 Jahre bis Baubeginn, 6 Jahre bis Fertigstellung; bis zum
/// Regelstand 2026-10-18 fälschlich 3 Jahre), BB § 73 BbgBO (6 Jahre, keine
/// Verlängerung), BY Art. 69 BayBO und RP § 74 LBauO (4 Jahre). SN nur über
/// eine Sekundärquelle bestätigt, amtlicher Text nicht eingesehen; die übrigen
/// Länder nach den bekannten Fassungen (3 Jahre).
const GELTUNGSDAUER: [Geltungsregel; 16] = [
    regel(
        Bundesland::Bw,
        3,
        None,
        "§ 62 Abs. 1 LBO Baden-Württemberg",
        None,
    ),
    regel(Bundesland::By, 4, None, "Art. 69 BayBO", None),
    regel(Bundesland::Be, 2, Some(6), "§ 73 Abs. 1 BauO Bln", None),
    regel(
        Bundesland::Bb,
        6,
        None,
        "§ 73 BbgBO",
        Some("Die BbgBO sieht keine Verlängerung der Geltungsdauer vor."),
    ),
    regel(Bundesland::Hb, 3, None, "BremLBO", None),
    regel(
        Bundesland::Hh,
        3,
        None,
        "§ 73 HBauO",
        Some(
            "Die Genehmigung erlischt auch, wenn die Bauausführung ein Jahr unterbrochen \
             wird. Rechtsbehelfe Dritter hemmen den Fristlauf (§ 73 HBauO).",
        ),
    ),
    regel(Bundesland::He, 3, None, "HBO", None),
    regel(Bundesland::Mv, 3, None, "LBauO M-V", None),
    regel(
        Bundesland::Ni,
        3,
        None,
        "§ 71 NBauO",
        Some("Die Genehmigung erlischt auch, wenn die Bauausführung drei Jahre unterbrochen wird."),
    ),
    regel(Bundesland::Nw, 3, None, "§ 75 Abs. 1 BauO NRW", None),
    regel(Bundesland::Rp, 4, None, "§ 74 LBauO Rheinland-Pfalz", None),
    regel(Bundesland::Sl, 3, None, "LBO Saarland", None),
    regel(Bundesland::Sn, 3, None, "SächsBO", None),
    regel(Bundesland::St, 3, None, "BauO LSA", None),
    regel(Bundesland::Sh, 3, None, "LBO Schleswig-Holstein", None),
    regel(Bundesland::Th, 3, None, "ThürBO", None),
];

/// Geltungsdauer des Vorbescheids im Regelfall der Musterbauordnung (§ 75
/// MBO). Die Landesfassungen sind nicht einzeln hinterlegt; der Hinweis der
/// Berechnung verweist darauf.
pub const VORBESCHEID_JAHRE: u32 = 3;

/// Monatsfrist nach §§ 187 Abs. 1, 188 Abs. 2 und 3 BGB: Ende am Tag gleicher
/// Zahl, fehlt er, am Monatsletzten.
fn nach_monaten(beginn: NaiveDate, monate: u32) -> NaiveDate {
    beginn
        .checked_add_months(Months::new(monate))
        .unwrap_or(NaiveDate::MAX)
}

const HINWEIS_NACHBAR: &str = "Während eines Widerspruchs oder einer Klage Dritter, etwa \
    von Nachbarn, kann der Lauf der Geltungsdauer ruhen (z. B. OVG Hamburg, Beschluss \
    2 Bs 179/14); das ist nicht berechnet.";
const HINWEIS_WERKTAG: &str = "Das Ende wird nicht auf den nächsten Werktag verschoben; ob \
    § 193 BGB für Erlöschensfristen gilt, ist umstritten.";

fn genehmigt(b: &Bescheid) -> bool {
    matches!(
        b.entscheidung,
        Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
    )
}

pub fn berechnen(land: Bundesland, art: &Verfahrensart, b: &Bescheid) -> Bescheidfristen {
    let mut r = Bescheidfristen {
        bescheid: b.id,
        rechtsbehelfsfrist: None,
        geltungsdauer: None,
        fertigstellung: None,
        hinweise: Vec::new(),
        regelstand: REGELSTAND.into(),
    };
    let genehmigt = genehmigt(b);
    // Ein Verlängerungsbescheid nennt das neue Ende selbst.
    if let Some(v) = &b.verlaengerung
        && genehmigt
    {
        r.geltungsdauer = Some(Fristberechnung {
            beginn: b.datum,
            ende: v.bis,
            grundlage: "Verlängerung der Geltungsdauer durch diesen Bescheid".into(),
            hinweise: vec![
                "Das Ende stammt aus dem Verlängerungsbescheid. Baubeginn in der Akte erfassen."
                    .into(),
                HINWEIS_NACHBAR.into(),
            ],
        });
    }
    match art {
        Verfahrensart::Freistellung => r.hinweise.push(
            "Genehmigungsfreistellung: Mit dem Vorhaben darf erst nach Ablauf der Wartefrist \
             der Landesbauordnung nach Eingang der Unterlagen bei der Gemeinde begonnen werden, \
             wenn sie kein Genehmigungsverfahren verlangt; die Wartefrist wird nicht berechnet. \
             Ein späterer Baubeginn kann nach Landesrecht eine erneute Vorlage erfordern."
                .into(),
        ),
        Verfahrensart::Widerspruch => r.hinweise.push(
            "Widerspruchsbescheid: Gegen eine Zurückweisung ist innerhalb eines Monats Klage \
             möglich. Hilft die Behörde ab, ist die Genehmigung im Ausgangsverfahren als \
             Bescheid zu erfassen."
                .into(),
        ),
        _ => {}
    }
    if b.teilbaugenehmigung {
        r.hinweise.push(
            "Teilbaugenehmigung: Sie erlaubt nur die genehmigten Teile der Bauarbeiten und \
             erlischt wie die Baugenehmigung. Die Entscheidung über den Bauantrag steht noch aus."
                .into(),
        );
    }
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
    let baugenehmigung = art.baugenehmigung();
    if genehmigt && *art == Verfahrensart::Vorbescheid && b.verlaengerung.is_none() {
        r.geltungsdauer = Some(Fristberechnung {
            beginn: zugestellt,
            ende: nach_monaten(zugestellt, VORBESCHEID_JAHRE * 12),
            grundlage: format!(
                "{VORBESCHEID_JAHRE} Jahre nach Erteilung (Regelfall nach § 75 MBO)"
            ),
            hinweise: vec![
                "Bis dahin den Bauantrag stellen oder die Verlängerung beantragen. Die Dauer \
                 folgt der Musterbauordnung; eine abweichende Landesregel oder ein im Bescheid \
                 genanntes Datum ist maßgebend und als Befristung zu erfassen."
                    .into(),
                HINWEIS_WERKTAG.into(),
            ],
        });
    }
    if genehmigt && baugenehmigung && b.verlaengerung.is_none() {
        let g = GELTUNGSDAUER
            .iter()
            .find(|g| g.land == land)
            .expect("Tabelle enthält jedes Bundesland");
        let mut hinweise = vec![
            "Die Genehmigung erlischt, wenn bis dahin nicht mit der Ausführung begonnen wurde. \
             Baubeginn in der Akte erfassen; Unterbrechung der Bauausführung und Verlängerung \
             auf Antrag nach der Landesbauordnung prüfen."
                .into(),
            HINWEIS_NACHBAR.into(),
            HINWEIS_WERKTAG.into(),
        ];
        hinweise.extend(g.hinweis.map(String::from));
        r.geltungsdauer = Some(Fristberechnung {
            beginn: zugestellt,
            ende: nach_monaten(zugestellt, g.jahre * 12),
            grundlage: format!(
                "{} Jahre nach Erteilung bis zum Baubeginn ({})",
                g.jahre, g.quelle
            ),
            hinweise,
        });
        if let Some(jahre) = g.fertigstellung_jahre {
            r.fertigstellung = Some(Fristberechnung {
                beginn: zugestellt,
                ende: nach_monaten(zugestellt, jahre * 12),
                grundlage: format!(
                    "{jahre} Jahre nach Erteilung bis zur Fertigstellung ({})",
                    g.quelle
                ),
                hinweise: vec![
                    "Die Genehmigung erlischt, wenn das Vorhaben bis dahin nicht fertiggestellt \
                     ist. Fertigstellung in der Akte erfassen."
                        .into(),
                    HINWEIS_NACHBAR.into(),
                    HINWEIS_WERKTAG.into(),
                ],
            });
        }
    } else if genehmigt
        && b.verlaengerung.is_none()
        && !matches!(
            art,
            Verfahrensart::Freistellung | Verfahrensart::Vorbescheid | Verfahrensart::Widerspruch
        )
    {
        r.hinweise.push(
            "Geltungsdauer wird nur für Baugenehmigungen berechnet; für diese Verfahrensart \
             nach Landesrecht prüfen."
                .into(),
        );
    }
    r
}

/// Fristen aller Bescheide einer Akte. Ein verlängerter Bescheid nennt seinen
/// Verlängerungsbescheid; dessen Ende ist maßgebend.
pub fn alle(land: Bundesland, art: &Verfahrensart, bescheide: &[Bescheid]) -> Vec<Bescheidfristen> {
    bescheide
        .iter()
        .map(|b| {
            let mut f = berechnen(land, art, b);
            if let Some(n) = verlaengert_durch(bescheide, b.id) {
                f.hinweise.push(format!(
                    "Geltungsdauer durch Bescheid vom {} bis {} verlängert; maßgebend ist der \
                     Verlängerungsbescheid.",
                    n.datum.format("%d.%m.%Y"),
                    n.verlaengerung
                        .as_ref()
                        .map(|v| v.bis.format("%d.%m.%Y").to_string())
                        .unwrap_or_default()
                ));
            }
            f
        })
        .collect()
}

/// Genehmigender Verlängerungsbescheid, der die Geltungsdauer von `id` verlängert.
pub fn verlaengert_durch(bescheide: &[Bescheid], id: Uuid) -> Option<&Bescheid> {
    bescheide
        .iter()
        .find(|n| genehmigt(n) && n.verlaengerung.as_ref().is_some_and(|v| v.bescheid == id))
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
        Ok(alle(land, &auftrag.art, &auftrag.bescheide))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plattform::objekte::Dokumentbezug;
    use crate::verfahren::Verlaengerung;

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
            verlaengerung: None,
            teilbaugenehmigung: false,
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
        let g = vorbescheid.geltungsdauer.unwrap();
        assert_eq!(g.ende, tag("2031-02-28"));
        assert!(g.grundlage.contains("§ 75 MBO"));
        let nutzung = berechnen(Bundesland::Nw, &Verfahrensart::Nutzungsaenderung, &b);
        assert_eq!(nutzung.geltungsdauer.unwrap().ende, tag("2031-02-28"));
        let widerspruch = berechnen(Bundesland::Nw, &Verfahrensart::Widerspruch, &b);
        assert!(widerspruch.geltungsdauer.is_none());
        assert!(widerspruch.hinweise.iter().any(|h| h.contains("Klage")));
        let mut teil = b.clone();
        teil.teilbaugenehmigung = true;
        let teil = berechnen(Bundesland::By, &Verfahrensart::Bauantrag, &teil);
        assert_eq!(teil.geltungsdauer.unwrap().ende, tag("2032-02-29"));
        assert!(
            teil.hinweise
                .iter()
                .any(|h| h.contains("Teilbaugenehmigung"))
        );
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
        // Jedes Land genau einmal.
        for land in Bundesland::ALL {
            assert_eq!(GELTUNGSDAUER.iter().filter(|g| g.land == land).count(), 1);
        }
        assert_eq!(offen.regelstand, REGELSTAND);
    }

    #[test]
    fn berlin_zwei_jahre_bis_baubeginn_und_sechs_bis_fertigstellung() {
        let b = bescheid(Entscheidung::Genehmigt, Some("2026-03-10"));
        let be = berechnen(Bundesland::Be, &Verfahrensart::Bauantrag, &b);
        let g = be.geltungsdauer.unwrap();
        assert_eq!(g.ende, tag("2028-03-10"));
        assert!(
            g.grundlage.contains("§ 73 Abs. 1 BauO Bln"),
            "{}",
            g.grundlage
        );
        assert!(g.hinweise.iter().any(|h| h.contains("2 Bs 179/14")));
        assert_eq!(be.fertigstellung.unwrap().ende, tag("2032-03-10"));
        // Andere Länder kennen keine eigene Fertigstellungsfrist.
        let bb = berechnen(Bundesland::Bb, &Verfahrensart::Bauantrag, &b);
        assert!(bb.fertigstellung.is_none());
        let g = bb.geltungsdauer.unwrap();
        assert_eq!(g.ende, tag("2032-03-10"));
        assert!(g.hinweise.iter().any(|h| h.contains("keine Verlängerung")));
    }

    #[test]
    fn verlaengerung_ersetzt_das_ende_und_freistellung_nennt_wartefrist() {
        let original = bescheid(Entscheidung::Genehmigt, Some("2026-03-10"));
        let mut verlaengerung = bescheid(Entscheidung::Genehmigt, None);
        verlaengerung.verlaengerung = Some(Verlaengerung {
            bescheid: original.id,
            bis: tag("2031-03-10"),
        });
        let f = alle(
            Bundesland::Nw,
            &Verfahrensart::Bauantrag,
            &[original.clone(), verlaengerung.clone()],
        );
        assert!(f[0].hinweise.iter().any(|h| h.contains("verlängert")));
        // Das neue Ende gilt auch ohne Zustelldatum des Verlängerungsbescheids.
        assert_eq!(f[1].geltungsdauer.as_ref().unwrap().ende, tag("2031-03-10"));
        assert!(f[1].rechtsbehelfsfrist.is_none());
        assert_eq!(
            verlaengert_durch(&[original.clone(), verlaengerung], original.id).map(|b| b.datum),
            Some(tag("2026-01-01"))
        );
        let frei = berechnen(Bundesland::By, &Verfahrensart::Freistellung, &original);
        assert!(frei.geltungsdauer.is_none());
        assert!(frei.hinweise.iter().any(|h| h.contains("Wartefrist")));
        let vor = berechnen(Bundesland::By, &Verfahrensart::Vorbescheid, &original);
        let g = vor.geltungsdauer.unwrap();
        assert!(g.hinweise.iter().any(|h| h.contains("Befristung")));
    }
}
