//! Abstandsflächen-Regelwerke je Bundesland und Fassung
//! (`regelwerke/abstandsflaechen/<land>/<gültig ab>.json`).
//!
//! Jede Datei beschreibt genau eine Fassung des Abstandsflächenrechts eines
//! Landes: Fundstelle, Gültigkeit, Quellen, Wortlaut je Absatz, nicht
//! automatisch geprüfte Tatbestände, Beispielrechnungen mit Sollwert und die
//! Rechenregeln selbst. Eine fachlich freigegebene Datei wird nie geändert
//! (Prüfsumme in `freigaben.json`); eine Gesetzesänderung bringt eine neue
//! Datei. Der Abstandsplan speichert eine Kopie der Regeln, alte Berechnungen
//! bleiben dadurch reproduzierbar.
use super::*;
use chrono::NaiveDate;
use sha2::{Digest, Sha256};

/// Ausgelieferte Regeldateien (Pfad relativ zu `regelwerke/abstandsflaechen`).
/// Ein Test stellt sicher, dass jede Datei des Ordners hier steht.
pub(crate) const DATEIEN: &[(&str, &str)] = &[
    (
        "nw/2024-01-01.json",
        include_str!("../../../../regelwerke/abstandsflaechen/nw/2024-01-01.json"),
    ),
    (
        "nw/2026-09-01.json",
        include_str!("../../../../regelwerke/abstandsflaechen/nw/2026-09-01.json"),
    ),
    (
        "by/pilot.json",
        include_str!("../../../../regelwerke/abstandsflaechen/by/pilot.json"),
    ),
    (
        "bw/pilot.json",
        include_str!("../../../../regelwerke/abstandsflaechen/bw/pilot.json"),
    ),
];
#[cfg(test)]
const FREIGABEN: &str = include_str!("../../../../regelwerke/abstandsflaechen/freigaben.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fassungsangabe {
    pub gesetz: String,
    pub paragraf: String,
    /// Tag des Inkrafttretens; fehlt bei Pilotansätzen ohne belegte Fassung.
    #[ts(type = "string | null")]
    pub gueltig_ab: Option<NaiveDate>,
    /// Letzter Geltungstag; fehlt, solange die Fassung gilt.
    #[ts(type = "string | null")]
    pub gueltig_bis: Option<NaiveDate>,
    pub fundstelle: String,
    pub uebergang: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Rechtsquelle {
    pub titel: String,
    pub url: String,
    /// Abrufdatum; fehlt, wenn die Quelle nicht online geprüft werden konnte.
    #[ts(type = "string | null")]
    pub abgerufen: Option<NaiveDate>,
    /// Beigelegte Datei relativ zum Länderordner.
    pub datei: String,
    /// SHA-256 der beigelegten Datei.
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Regelpruefung {
    /// Nur das Büro setzt `true` (fachliche Abnahme), zusammen mit dem
    /// Eintrag in `freigaben.json`.
    pub geprueft: bool,
    pub geprueft_von: String,
    #[ts(type = "string | null")]
    pub geprueft_am: Option<NaiveDate>,
    pub vermerk: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Absatztext {
    pub absatz: u32,
    pub text: String,
}
/// Tatbestand, den der Rechenkern nicht prüft; erscheint als Prüfpunkt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Regelpruefpunkt {
    pub absatz: u32,
    pub frage: String,
}
/// Einzelwand mit erwartetem Ergebnis; ein Test rechnet jede nach.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Beispielrechnung {
    pub titel: String,
    pub gebiet: String,
    pub laenge_m: f64,
    /// Geländehöhe an Wandanfang und Wandende.
    pub gelaende_m: [f64; 2],
    /// Wandoberkante an Wandanfang und Wandende.
    pub oberkante_m: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dachhoehe_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dachneigung_grad: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tiefenfall: Option<Tiefenfall>,
    pub erwartet_h_m: f64,
    pub erwartet_tiefe_m: f64,
}
/// Inhalt einer Regeldatei.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Regeldatei {
    pub schema: u32,
    pub fassung: Fassungsangabe,
    pub quellen: Vec<Rechtsquelle>,
    pub pruefung: Regelpruefung,
    pub wortlaut: Vec<Absatztext>,
    pub pruefpunkte: Vec<Regelpruefpunkt>,
    pub beispiele: Vec<Beispielrechnung>,
    pub regeln: AbstandsRegeln,
}
/// Geladene Regeldatei mit Herkunft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Regelwerk {
    /// Pfad relativ zu `regelwerke/abstandsflaechen`, z. B. `nw/2024-01-01.json`.
    pub datei: String,
    /// SHA-256 des Dateiinhalts.
    pub pruefsumme: String,
    #[serde(flatten)]
    #[ts(flatten)]
    pub inhalt: Regeldatei,
}
impl Regelwerk {
    /// Gilt die Fassung am Stichtag? Pilotansätze ohne Datum gelten nie.
    pub fn gilt_am(&self, tag: NaiveDate) -> bool {
        let f = &self.inhalt.fassung;
        f.gueltig_ab.is_some_and(|ab| ab <= tag) && f.gueltig_bis.is_none_or(|bis| tag <= bis)
    }
}
pub(crate) fn pruefsumme(inhalt: &str) -> String {
    format!("{:x}", Sha256::digest(inhalt.as_bytes()))
}
/// Alle ausgelieferten Regelwerke, je Land nach Gültigkeitsbeginn sortiert
/// (Pilotansätze ohne Datum zuerst).
pub fn regelwerke() -> CoreResult<Vec<Regelwerk>> {
    let mut alle = DATEIEN
        .iter()
        .map(|(datei, inhalt)| {
            let inhalt: Regeldatei = serde_json::from_str(inhalt)
                .map_err(|e| err(format!("Regeldatei {datei}: {e}")))?;
            Ok(Regelwerk {
                datei: datei.to_string(),
                pruefsumme: pruefsumme(DATEIEN.iter().find(|d| d.0 == *datei).unwrap().1),
                inhalt,
            })
        })
        .collect::<CoreResult<Vec<_>>>()?;
    alle.sort_by(|a, b| {
        (a.inhalt.regeln.land.code(), a.inhalt.fassung.gueltig_ab)
            .cmp(&(b.inhalt.regeln.land.code(), b.inhalt.fassung.gueltig_ab))
    });
    Ok(alle)
}
/// Fassung, die am Stichtag gilt. Gibt es für das Land keine datierte
/// Fassung, wird der Pilotansatz vorgeschlagen.
pub fn fassung_am(land: Bundesland, tag: NaiveDate) -> CoreResult<Option<Regelwerk>> {
    let des_landes: Vec<_> = regelwerke()?
        .into_iter()
        .filter(|r| r.inhalt.regeln.land == land)
        .collect();
    if des_landes
        .iter()
        .any(|r| r.inhalt.fassung.gueltig_ab.is_some())
    {
        Ok(des_landes.into_iter().find(|r| r.gilt_am(tag)))
    } else {
        Ok(des_landes.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn ordner() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../regelwerke/abstandsflaechen")
    }
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn jede_regeldatei_des_ordners_ist_eingebunden() {
        let mut gefunden = BTreeSet::new();
        for land in std::fs::read_dir(ordner()).unwrap() {
            let land = land.unwrap();
            if !land.file_type().unwrap().is_dir() {
                continue;
            }
            for datei in std::fs::read_dir(land.path()).unwrap() {
                let datei = datei.unwrap().path();
                if datei.extension().is_some_and(|e| e == "json") {
                    gefunden.insert(format!(
                        "{}/{}",
                        land.file_name().to_string_lossy(),
                        datei.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
        let eingebunden: BTreeSet<_> = DATEIEN.iter().map(|d| d.0.to_string()).collect();
        assert_eq!(gefunden, eingebunden);
        // Der eingebettete Inhalt ist der Inhalt auf der Platte.
        for (datei, inhalt) in DATEIEN {
            assert_eq!(
                std::fs::read_to_string(ordner().join(datei)).unwrap(),
                *inhalt
            );
        }
    }

    #[test]
    fn regeldateien_sind_vollstaendig_und_belegt() {
        for r in regelwerke().unwrap() {
            let d = &r.inhalt;
            let ort = &r.datei;
            assert_eq!(d.schema, 1, "{ort}");
            let (land, name) = ort.split_once('/').unwrap();
            assert_eq!(
                land,
                d.regeln.land.code().to_lowercase(),
                "{ort}: Ordner = Land"
            );
            match d.fassung.gueltig_ab {
                Some(ab) => {
                    assert_eq!(name, format!("{ab}.json"), "{ort}: Dateiname = gültig ab");
                    assert_eq!(
                        d.regeln.version,
                        ab.to_string(),
                        "{ort}: Version = gültig ab"
                    );
                    assert!(
                        !d.wortlaut.is_empty(),
                        "{ort}: datierte Fassung ohne Wortlaut"
                    );
                    assert!(!d.quellen.is_empty(), "{ort}: datierte Fassung ohne Quelle");
                    assert!(!d.beispiele.is_empty(), "{ort}: ohne Beispielrechnung");
                    assert!(!d.fassung.fundstelle.trim().is_empty(), "{ort}: Fundstelle");
                }
                None => assert_eq!(name, "pilot.json", "{ort}: undatiert nur als Pilot"),
            }
            if let (Some(ab), Some(bis)) = (d.fassung.gueltig_ab, d.fassung.gueltig_bis) {
                assert!(ab <= bis, "{ort}");
            }
            let mut plan = testdaten::plan();
            plan.regeln = d.regeln.clone();
            plan.validate().unwrap_or_else(|e| panic!("{ort}: {e}"));
            let absaetze: Vec<_> = d.wortlaut.iter().map(|a| a.absatz).collect();
            assert!(absaetze.windows(2).all(|w| w[0] < w[1]), "{ort}: Absätze");
            for a in &d.wortlaut {
                assert!(!a.text.trim().is_empty(), "{ort}: Abs. {} leer", a.absatz);
                assert!(
                    !a.text.contains("Herausgeber"),
                    "{ort}: Seitenfuß im Wortlaut"
                );
                assert!(
                    !a.text.contains('\u{2010}'),
                    "{ort}: Trennstrich im Wortlaut"
                );
                // Umlaute vorkomponiert (NFC), sonst scheitern Suche und Vergleich.
                assert!(
                    !a.text.chars().any(|c| ('\u{300}'..='\u{36f}').contains(&c)),
                    "{ort}: Abs. {} enthält zerlegte Umlaute",
                    a.absatz
                );
            }
            for p in &d.pruefpunkte {
                assert!(
                    absaetze.contains(&p.absatz),
                    "{ort}: Prüfpunkt Abs. {}",
                    p.absatz
                );
            }
            for q in &d.quellen {
                let pfad = ordner().join(land).join(&q.datei);
                let bytes = std::fs::read(&pfad)
                    .unwrap_or_else(|_| panic!("{ort}: Quelle {} fehlt", q.datei));
                assert_eq!(format!("{:x}", Sha256::digest(&bytes)), q.sha256, "{ort}");
            }
            assert_eq!(
                d.pruefung.geprueft,
                d.pruefung.geprueft_am.is_some() && !d.pruefung.geprueft_von.trim().is_empty(),
                "{ort}: Freigabe nur mit Prüfer und Datum"
            );
        }
    }

    #[test]
    fn fassungen_eines_landes_schliessen_lueckenlos_an() {
        let alle = regelwerke().unwrap();
        for land in Bundesland::ALL {
            let datiert: Vec<_> = alle
                .iter()
                .filter(|r| r.inhalt.regeln.land == land && r.inhalt.fassung.gueltig_ab.is_some())
                .map(|r| &r.inhalt.fassung)
                .collect();
            for w in datiert.windows(2) {
                let bis = w[0].gueltig_bis.expect("nur die letzte Fassung ist offen");
                assert_eq!(bis.succ_opt(), w[1].gueltig_ab, "{land:?}");
            }
        }
    }

    #[test]
    fn beispielrechnungen_ergeben_den_sollwert() {
        let mut anzahl = 0;
        for r in regelwerke().unwrap() {
            for b in &r.inhalt.beispiele {
                let mut plan = testdaten::plan();
                plan.regeln = r.inhalt.regeln.clone();
                plan.recht.gebiet = b.gebiet.clone();
                let w = &mut plan.waende[0];
                w.bis = testdaten::punkt(w.von.x + b.laenge_m, w.von.y);
                let profil = |h: [f64; 2]| {
                    vec![
                        Hoehenpunkt {
                            station_m: 0.,
                            hoehe_m: h[0],
                            quelle: String::new(),
                        },
                        Hoehenpunkt {
                            station_m: b.laenge_m,
                            hoehe_m: h[1],
                            quelle: String::new(),
                        },
                    ]
                };
                w.gelaende = profil(b.gelaende_m);
                w.oberkante = profil(b.oberkante_m);
                (
                    w.gelaende_a_m,
                    w.gelaende_b_m,
                    w.abschluss_a_m,
                    w.abschluss_b_m,
                ) = (None, None, None, None);
                if let Some(h) = b.dachhoehe_m {
                    w.dachart = Dachart::Dachflaeche;
                    w.dachhoehe_m = Some(h);
                    w.dachneigung_grad = b.dachneigung_grad;
                }
                if let Some(f) = b.tiefenfall {
                    w.tiefenfall = Some(f);
                    w.tiefenfall_grundlage = "Beispielrechnung".into();
                }
                let e = berechnen(&plan).unwrap();
                let w = &e.waende[0];
                let ort = format!("{}: {}", r.datei, b.titel);
                let nah = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
                assert!(nah(w.h_m, b.erwartet_h_m), "{ort}: H = {:?}", w.h_m);
                assert!(
                    nah(w.tiefe_m, b.erwartet_tiefe_m),
                    "{ort}: T = {:?}",
                    w.tiefe_m
                );
                anzahl += 1;
            }
        }
        assert!(anzahl >= 20);
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Freigaben {
        #[allow(dead_code)]
        hinweis: String,
        freigaben: Vec<Freigabe>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Freigabe {
        datei: String,
        sha256: String,
    }

    /// Eine fachlich freigegebene Datei darf sich nie mehr ändern.
    #[test]
    fn freigegebene_regeldateien_sind_unveraendert() {
        let f: Freigaben = serde_json::from_str(FREIGABEN).unwrap();
        let alle = regelwerke().unwrap();
        for e in &f.freigaben {
            let r = alle
                .iter()
                .find(|r| r.datei == e.datei)
                .unwrap_or_else(|| panic!("{}: freigegebene Datei fehlt", e.datei));
            assert_eq!(
                r.pruefsumme, e.sha256,
                "{}: freigegebene Regeldatei wurde geändert – neue Fassung als neue Datei anlegen",
                e.datei
            );
            assert!(r.inhalt.pruefung.geprueft, "{}", e.datei);
        }
        for r in alle.iter().filter(|r| r.inhalt.pruefung.geprueft) {
            assert!(
                f.freigaben.iter().any(|e| e.datei == r.datei),
                "{}: geprüft, aber ohne Prüfsumme in freigaben.json",
                r.datei
            );
        }
    }

    #[test]
    fn fassung_nach_stichtag() {
        let nw = |t: &str| fassung_am(Bundesland::Nw, tag(t)).unwrap().map(|r| r.datei);
        assert_eq!(nw("2023-12-31"), None);
        assert_eq!(nw("2024-01-01").as_deref(), Some("nw/2024-01-01.json"));
        assert_eq!(nw("2026-08-31").as_deref(), Some("nw/2024-01-01.json"));
        assert_eq!(nw("2026-09-01").as_deref(), Some("nw/2026-09-01.json"));
        assert_eq!(nw("2030-01-01").as_deref(), Some("nw/2026-09-01.json"));
        let by = fassung_am(Bundesland::By, tag("2026-11-05"))
            .unwrap()
            .unwrap();
        assert_eq!(by.datei, "by/pilot.json");
        assert!(
            fassung_am(Bundesland::He, tag("2026-11-05"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn katalog_liefert_je_land_die_geltende_fassung() {
        let nw = |t: &str| {
            katalog_am(tag(t))
                .unwrap()
                .into_iter()
                .find(|r| r.land == Bundesland::Nw)
                .unwrap()
                .version
        };
        assert_eq!(nw("2025-06-01"), "2024-01-01");
        assert_eq!(nw("2026-11-05"), "2026-09-01");
        // Vor der ältesten Fassung die jüngste statt keiner.
        assert_eq!(nw("2020-01-01"), "2026-09-01");
        assert_eq!(katalog_am(tag("2026-11-05")).unwrap().len(), 3);
    }

    /// Abgleich zweier unabhängig übertragener Texte: Zwischen den
    /// NRW-Fassungen ändern sich in § 6 nur Abs. 1 Satz 3 und der Verweis in
    /// Abs. 4 Satz 6; die Rechenregeln bleiben gleich.
    #[test]
    fn nrw_fassungen_unterscheiden_sich_nur_im_belegten_wortlaut() {
        let alle = regelwerke().unwrap();
        let datei = |n: &str| &alle.iter().find(|r| r.datei == n).unwrap().inhalt;
        let (alt, neu) = (datei("nw/2024-01-01.json"), datei("nw/2026-09-01.json"));
        for (a, n) in alt.wortlaut.iter().zip(&neu.wortlaut) {
            assert_eq!(a.absatz, n.absatz);
            match a.absatz {
                1 => assert_eq!(
                    n.text.replace(
                        "Nebenanlagen von verfahrensfreien Bauvorhaben nach § 62 Absatz 1 Satz 1 Nummer 4 Buchstabe b sowie für ",
                        ""
                    ),
                    a.text
                ),
                4 => assert_eq!(
                    n.text.replace("§ 8 Absatz 3", "§ 8 Absatz 5"),
                    a.text
                ),
                _ => assert_eq!(a.text, n.text, "Abs. {}", a.absatz),
            }
        }
        assert_eq!(alt.wortlaut.len(), 13);
        assert_eq!(neu.wortlaut.len(), 13);
        let rechenteil = |r: &AbstandsRegeln| {
            let mut r = r.clone();
            (r.version, r.rechtsstand, r.hinweis) = Default::default();
            r
        };
        assert_eq!(rechenteil(&alt.regeln), rechenteil(&neu.regeln));
    }
}
