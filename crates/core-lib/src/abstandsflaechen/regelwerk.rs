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

/// Im Abstandsplan gespeicherte Wahl der Regeldatei.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Regelwerkbezug {
    /// Pfad relativ zu `regelwerke/abstandsflaechen`, z. B. `nw/2024-01-01.json`.
    pub datei: String,
    /// SHA-256 der Regeldatei bei der Übernahme.
    pub pruefsumme: String,
    /// Tag, nach dem sich die Fassung richtet (in der Regel Antragseingang).
    #[ts(type = "string")]
    pub stichtag: NaiveDate,
    /// Woher der Stichtag stammt, z. B. „Eingang Bauantrag“.
    pub stichtag_art: String,
    /// Begründung, wenn die Fassung nicht am Stichtag gilt (z. B. Übergangsrecht).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(optional, as = "Option<String>")]
    pub abweichung: String,
    /// Bearbeitungsstand der Prüfpunkte der Regeldatei.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Pruefpunktstand>>")]
    pub pruefpunkte: Vec<Pruefpunktstand>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Pruefpunktstand {
    pub absatz: u32,
    pub erledigt: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(optional, as = "Option<String>")]
    pub vermerk: String,
}
/// Abgleich des Plans mit seiner Regeldatei.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Regelwerkpruefung {
    /// Die gewählte Regeldatei, sofern in dieser Programmversion enthalten.
    pub regelwerk: Option<Regelwerk>,
    /// Datei der am Stichtag geltenden Fassung.
    pub geltend: Option<String>,
    /// Rechenregeln des Plans entsprechen der Regeldatei.
    pub regeln_aktuell: bool,
    /// Die Regeldatei hat sich seit der Übernahme geändert.
    pub datei_geaendert: bool,
    /// Absätze, die die Rechenregeln anwenden (aus den Quellenangaben).
    pub angewandte_absaetze: Vec<u32>,
    pub offen: Vec<String>,
}
/// Absätze, auf die sich die Rechenregeln berufen („Abs. 5 Satz 1“ → 5).
fn angewandte_absaetze(r: &AbstandsRegeln) -> Vec<u32> {
    let quellen = r
        .tiefen
        .iter()
        .map(|t| &t.quelle)
        .chain(r.dach.iter().map(|d| &d.quelle))
        .chain(r.grenzanlagen.iter().map(|g| &g.quelle))
        .chain(r.giebel.iter().map(|g| &g.quelle))
        .chain(r.gauben.iter().map(|g| &g.quelle))
        .chain(r.ueberdeckung.iter().map(|u| &u.quelle));
    let mut v: Vec<u32> = quellen
        .flat_map(|q| {
            q.match_indices("Abs. ")
                .filter_map(|(i, m)| {
                    let rest = &q[i + m.len()..];
                    let ziffern: String = rest.chars().take_while(char::is_ascii_digit).collect();
                    ziffern.parse().ok()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}
/// Prüft die Fassungswahl eines Plans; `None` für Pläne ohne Regeldatei.
pub fn regelwerk_pruefen(plan: &AbstandsPlan) -> Option<Regelwerkpruefung> {
    let b = plan.regelwerk.as_ref()?;
    let mut offen = Vec::new();
    let alle = match regelwerke() {
        Ok(a) => a,
        Err(e) => {
            offen.push(format!("Regeldateien nicht lesbar: {e}"));
            vec![]
        }
    };
    let regelwerk = alle.iter().find(|r| r.datei == b.datei).cloned();
    let geltend = alle
        .iter()
        .filter(|r| r.inhalt.regeln.land == plan.regeln.land)
        .find(|r| r.gilt_am(b.stichtag))
        .map(|r| r.datei.clone());
    let Some(r) = &regelwerk else {
        offen.push(format!(
            "Regeldatei {} ist in dieser Programmversion nicht enthalten",
            b.datei
        ));
        return Some(Regelwerkpruefung {
            regelwerk: None,
            geltend,
            regeln_aktuell: false,
            datei_geaendert: false,
            angewandte_absaetze: angewandte_absaetze(&plan.regeln),
            offen,
        });
    };
    let regeln_aktuell = r.inhalt.regeln == plan.regeln;
    if !regeln_aktuell {
        offen.push(format!(
            "Rechenregeln des Plans weichen von der Regeldatei {} ab; Fassung neu übernehmen",
            b.datei
        ));
    }
    if geltend.as_deref() != Some(b.datei.as_str()) && b.abweichung.trim().is_empty() {
        offen.push(format!(
            "Die gewählte Fassung gilt nicht am Stichtag {} ({}); Begründung angeben, z. B. Übergangsrecht oder Antrag nach altem Recht",
            b.stichtag.format("%d.%m.%Y"),
            match &geltend {
                Some(d) => format!("am Stichtag gilt {d}"),
                None => "für diesen Tag ist keine Fassung hinterlegt".into(),
            }
        ));
    }
    let nicht_erledigt: Vec<String> = r
        .inhalt
        .pruefpunkte
        .iter()
        .filter(|p| {
            !b.pruefpunkte
                .iter()
                .any(|s| s.absatz == p.absatz && s.erledigt)
        })
        .map(|p| p.absatz.to_string())
        .collect();
    if !nicht_erledigt.is_empty() {
        offen.push(format!(
            "Prüfpunkte der Regeldatei abhaken (nicht automatisch geprüft): Abs. {}",
            nicht_erledigt.join(", ")
        ));
    }
    Some(Regelwerkpruefung {
        datei_geaendert: r.pruefsumme != b.pruefsumme,
        angewandte_absaetze: angewandte_absaetze(&r.inhalt.regeln),
        regelwerk: regelwerk.clone(),
        geltend,
        regeln_aktuell,
        offen,
    })
}
/// Regelwerkbezug für eine Regeldatei mit offenen Prüfpunkten.
pub fn regelwerk_uebernehmen(
    plan: &mut AbstandsPlan,
    datei: &str,
    stichtag: NaiveDate,
    stichtag_art: String,
) -> CoreResult<()> {
    let r = regelwerke()?
        .into_iter()
        .find(|r| r.datei == datei)
        .ok_or_else(|| err("Unbekannte Regeldatei"))?;
    if r.inhalt.regeln.land != plan.regeln.land {
        return Err(err("Regeldatei gehört zu einem anderen Bundesland"));
    }
    // Begründung und abgehakte Prüfpunkte gelten nur für dieselbe Datei.
    let alt = plan.regelwerk.take().filter(|a| a.datei == r.datei);
    plan.regeln = r.inhalt.regeln.clone();
    plan.regelwerk = Some(Regelwerkbezug {
        datei: r.datei.clone(),
        pruefsumme: r.pruefsumme.clone(),
        stichtag,
        stichtag_art,
        abweichung: alt
            .as_ref()
            .map(|a| a.abweichung.clone())
            .unwrap_or_default(),
        pruefpunkte: alt.map(|a| a.pruefpunkte).unwrap_or_default(),
    });
    Ok(())
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

    fn nw_plan(datei: &str, stichtag: &str) -> AbstandsPlan {
        let mut p = testdaten::plan();
        regelwerk_uebernehmen(&mut p, datei, tag(stichtag), "Eingang Bauantrag".into()).unwrap();
        p
    }
    fn alle_abhaken(p: &mut AbstandsPlan) {
        let r = regelwerk_pruefen(p).unwrap().regelwerk.unwrap();
        p.regelwerk.as_mut().unwrap().pruefpunkte = r
            .inhalt
            .pruefpunkte
            .iter()
            .map(|q| Pruefpunktstand {
                absatz: q.absatz,
                erledigt: true,
                vermerk: "geprüft".into(),
            })
            .collect();
    }

    #[test]
    fn uebernahme_setzt_regeln_und_prueft_die_fassung_am_stichtag() {
        let mut p = nw_plan("nw/2024-01-01.json", "2026-05-04");
        let b = p.regelwerk.clone().unwrap();
        assert_eq!(p.regeln.version, "2024-01-01");
        assert_eq!(b.pruefsumme.len(), 64);
        let pr = regelwerk_pruefen(&p).unwrap();
        assert!(pr.regeln_aktuell && !pr.datei_geaendert);
        assert_eq!(pr.geltend.as_deref(), Some("nw/2024-01-01.json"));
        assert_eq!(pr.angewandte_absaetze, vec![3, 4, 5, 8]);
        // Nur die Prüfpunkte sind offen, in einer Meldung.
        assert_eq!(pr.offen.len(), 1);
        assert!(pr.offen[0].contains("Abs. 1, 2, 3"));
        alle_abhaken(&mut p);
        assert!(regelwerk_pruefen(&p).unwrap().offen.is_empty());

        // Fassung, die am Stichtag nicht gilt: nur mit Begründung.
        regelwerk_uebernehmen(
            &mut p,
            "nw/2026-09-01.json",
            tag("2026-05-04"),
            "Eingang Bauantrag".into(),
        )
        .unwrap();
        let b = p.regelwerk.as_ref().unwrap();
        assert!(
            b.pruefpunkte.is_empty(),
            "Prüfpunkte gelten nur für dieselbe Datei"
        );
        alle_abhaken(&mut p);
        let pr = regelwerk_pruefen(&p).unwrap();
        assert_eq!(pr.offen.len(), 1);
        assert!(pr.offen[0].contains("gilt nicht am Stichtag 04.05.2026"));
        p.regelwerk.as_mut().unwrap().abweichung =
            "Bauherrschaft beantragt neues Recht (§ 90 Abs. 4 Satz 2)".into();
        assert!(regelwerk_pruefen(&p).unwrap().offen.is_empty());
    }

    #[test]
    fn abweichende_regelkopie_und_fremdes_land_werden_erkannt() {
        let mut p = nw_plan("nw/2026-09-01.json", "2026-11-05");
        alle_abhaken(&mut p);
        p.regeln.tiefen[0].faktor = 0.5;
        let pr = regelwerk_pruefen(&p).unwrap();
        assert!(!pr.regeln_aktuell);
        assert!(pr.offen[0].contains("weichen von der Regeldatei"));
        p.regelwerk.as_mut().unwrap().pruefsumme = "0".repeat(64);
        assert!(regelwerk_pruefen(&p).unwrap().datei_geaendert);
        p.regelwerk.as_mut().unwrap().datei = "nw/1999-01-01.json".into();
        assert!(regelwerk_pruefen(&p).unwrap().offen[0].contains("nicht enthalten"));
        assert!(
            regelwerk_uebernehmen(&mut p, "by/pilot.json", tag("2026-11-05"), String::new())
                .is_err()
        );
        assert!(
            regelwerk_uebernehmen(&mut p, "xx.json", tag("2026-11-05"), String::new()).is_err()
        );
    }

    #[test]
    fn offene_fassungswahl_macht_den_nachweis_unvollstaendig() {
        let mut p = nw_plan("nw/2026-09-01.json", "2026-11-05");
        let mut r = berechnen(&p).unwrap();
        let recht = crate::domain::Planungsrecht::default();
        projektabgleich(&p, &recht, &mut r);
        assert!(!r.vollstaendig);
        assert!(
            r.offen
                .iter()
                .any(|o| o.contains("Prüfpunkte der Regeldatei"))
        );
        alle_abhaken(&mut p);
        let mut r = berechnen(&p).unwrap();
        let vorher = r.vollstaendig;
        projektabgleich(&p, &recht, &mut r);
        assert_eq!(r.vollstaendig, vorher);
        // Ohne Regeldatei (Altpläne) keine zusätzliche Meldung.
        p.regelwerk = None;
        assert!(regelwerk_pruefen(&p).is_none());
    }
}
