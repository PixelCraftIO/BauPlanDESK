//! Fachliche Auswertung eingehender Nachrichten der Bauaufsicht:
//! Eingangsbestätigung (1120), Aktenzeichen (1121), formelle Prüfung mit
//! Befundliste (1140), Gebührenbescheid (1160), Anhörung (0203) und
//! Bescheid (0205).
//!
//! Die Auswertung liest nur, sie ändert nichts. Was davon in die
//! Verfahrensakte übernommen wird, entscheidet der Mensch (siehe
//! [`crate::verfahren::eingang`]); XSD-Prüfung und Journal bleiben in
//! [`super::austausch`] und [`super::postfach`].
use super::austausch::{MAX_XML, Nachrichtenkopf, fehler};
use crate::CoreResult;
use crate::verfahren::Nebenbestimmungsart;
use base64::Engine as _;
use chrono::NaiveDate;
use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default)]
struct Knoten {
    name: String,
    text: String,
    kinder: Vec<Knoten>,
}
impl Knoten {
    fn kind(&self, name: &str) -> Option<&Knoten> {
        self.kinder.iter().find(|k| k.name == name)
    }
    fn kinder<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Knoten> {
        self.kinder.iter().filter(move |k| k.name == name)
    }
    fn pfad(&self, pfad: &[&str]) -> Option<&Knoten> {
        pfad.iter().try_fold(self, |k, n| k.kind(n))
    }
    fn wert(&self) -> String {
        self.text.trim().to_string()
    }
    /// Text eines `xbauk:Text` (Absätze) oder `xbauk:TextFormatiert` (HTML,
    /// Base64); leer, wenn keines von beiden.
    fn absaetze(&self) -> String {
        let absaetze: Vec<String> = self.kinder("textabsatz").map(Knoten::wert).collect();
        if !absaetze.is_empty() {
            return absaetze.join("\n");
        }
        match self.kind("text") {
            Some(t) => html_text(&t.wert()),
            None => self.wert(),
        }
    }
}
fn baum(xml: &[u8]) -> CoreResult<Knoten> {
    if xml.len() as u64 > MAX_XML {
        return Err(fehler("XML größer als 8 MiB"));
    }
    let raw = std::str::from_utf8(xml).map_err(|_| fehler("Nur UTF-8 XML wird unterstützt"))?;
    let mut r = Reader::from_str(raw);
    r.config_mut().expand_empty_elements = true;
    let mut stapel: Vec<Knoten> = vec![Knoten::default()];
    let mut knoten = 0usize;
    loop {
        match r
            .read_event()
            .map_err(|e| fehler(format!("XML nicht lesbar: {e}")))?
        {
            Event::Start(e) => {
                knoten += 1;
                if knoten > 100_000 || stapel.len() > 64 {
                    return Err(fehler(
                        "XML-Struktur überschreitet die zulässige Komplexität",
                    ));
                }
                stapel.push(Knoten {
                    name: e.local_name().as_ref().to_string(),
                    ..Default::default()
                });
            }
            Event::Text(t) => {
                if let Some(k) = stapel.last_mut() {
                    k.text.push_str(&t.xml10_content());
                }
            }
            Event::CData(t) => {
                if let Some(k) = stapel.last_mut() {
                    k.text.push_str(&t.xml10_content());
                }
            }
            Event::GeneralRef(g) => {
                let zeichen = match g.resolve_char_ref().ok().flatten() {
                    Some(c) => c,
                    None => match g.xml10_content().as_ref() {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "quot" => '"',
                        "apos" => '\'',
                        _ => '?',
                    },
                };
                if let Some(k) = stapel.last_mut() {
                    k.text.push(zeichen);
                }
            }
            Event::End(_) => {
                let k = stapel.pop().ok_or_else(|| fehler("XML unausgeglichen"))?;
                stapel
                    .last_mut()
                    .ok_or_else(|| fehler("XML unausgeglichen"))?
                    .kinder
                    .push(k);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let mut wurzel = stapel.pop().ok_or_else(|| fehler("XML leer"))?;
    if !stapel.is_empty() || wurzel.kinder.len() != 1 {
        return Err(fehler("XML benötigt genau ein Wurzelelement"));
    }
    Ok(wurzel.kinder.remove(0))
}
/// Lesbarer Text aus HTML in Base64 (`TextFormatiert`): Blockelemente werden
/// Zeilen, Auszeichnung entfällt.
fn html_text(b64: &str) -> String {
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(
        b64.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>(),
    ) else {
        return String::new();
    };
    let html = String::from_utf8_lossy(&bytes);
    let mut aus = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for c in html.chars() {
        match (in_tag, c) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let name = tag
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if matches!(
                    name.as_str(),
                    "p" | "br" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4"
                ) {
                    aus.push('\n');
                }
            }
            (true, c) => tag.push(c),
            (false, c) => aus.push(c),
        }
    }
    let aus = aus
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    aus.lines()
        .map(|z| z.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|z| !z.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
fn datum(k: Option<&Knoten>) -> Option<NaiveDate> {
    k.and_then(|k| k.wert().parse().ok())
}
fn code(k: Option<&Knoten>) -> String {
    k.and_then(|k| k.kind("code"))
        .map(Knoten::wert)
        .unwrap_or_default()
}

/// Befund der formellen Prüfung (Unterlage fehlt, unvollständig …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct FormellerBefund {
    /// Code der Befundart laut Nachricht.
    pub art: String,
    pub text: String,
}
/// Befund der materiellen Prüfung (Bescheidlage einer Anhörung).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct MateriellerBefund {
    pub rechtsbereich: String,
    pub regelverstoss: String,
    pub begruendung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct BescheidNebenbestimmung {
    pub art: Nebenbestimmungsart,
    /// Code der XBau-Liste „Nebenbestimmung Art“ (1 Bedingung … 6
    /// aufschiebende Bedingung).
    pub xbau_code: String,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct BescheidAbweichung {
    /// Kennung der beantragten Abweichung (aus dem eigenen Antrag).
    pub id: Uuid,
    pub zugelassen: bool,
    pub vorschrift: String,
    pub begruendung: String,
    pub nebenbestimmung: String,
}
/// Inhalt einer Behördennachricht, soweit er die Verfahrensakte betrifft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Auswertung {
    Eingangsbestaetigung {
        information: String,
    },
    Aktenzeichen {
        aktenzeichen: String,
    },
    Befundliste {
        vollstaendig: bool,
        befunde: Vec<FormellerBefund>,
        frist: Option<NaiveDate>,
        anschreiben: String,
    },
    Anhoerung {
        aufforderung: String,
        befunde: Vec<MateriellerBefund>,
        frist: Option<NaiveDate>,
    },
    Bescheid {
        genehmigung: bool,
        nebenbestimmungen: Vec<BescheidNebenbestimmung>,
        abweichungen: Vec<BescheidAbweichung>,
        begruendung: String,
        rechtsbehelfsbelehrung: String,
        hinweise: String,
    },
    Gebuehrenbescheid {
        vorauszahlung: bool,
        /// Summe in Eurocent, wenn die Kostenberechnung sie nennt.
        #[ts(type = "number | null")]
        summe_cent: Option<i64>,
        begruendung: String,
        verwendungszweck: String,
        frist: Option<NaiveDate>,
        rechtsbehelfsbelehrung: String,
    },
    /// Nachrichtentyp ohne fachliche Auswertung; nur als Kommunikation.
    Sonstige,
}
fn nebenbestimmungsart(code: &str) -> Nebenbestimmungsart {
    match code {
        "1" | "5" | "6" => Nebenbestimmungsart::Bedingung,
        "2" => Nebenbestimmungsart::Auflage,
        // Die Liste unterscheidet Widerrufs- und Auflagenvorbehalt nicht.
        "3" => Nebenbestimmungsart::Widerrufsvorbehalt,
        _ => Nebenbestimmungsart::Hinweis,
    }
}
/// Wertet die (schemageprüfte) Nachricht aus. Unbekannte Typen sind
/// [`Auswertung::Sonstige`].
pub fn auswerten(kopf: &Nachrichtenkopf, xml: &[u8]) -> CoreResult<Auswertung> {
    let w = baum(xml)?;
    if w.name != kopf.element {
        return Err(fehler(
            "Nachricht passt nicht zum erfassten Nachrichtenkopf",
        ));
    }
    let kurz = kopf.element.rsplit('.').next().unwrap_or_default();
    Ok(match kurz {
        "1120" => Auswertung::Eingangsbestaetigung {
            information: w
                .kind("information")
                .map(Knoten::absaetze)
                .unwrap_or_default(),
        },
        "1121" => Auswertung::Aktenzeichen {
            aktenzeichen: w
                .pfad(&["bezug", "vorgang"])
                .map(Knoten::wert)
                .unwrap_or_default(),
        },
        "1140" => {
            let befund = w.kind("befundFrist");
            Auswertung::Befundliste {
                vollstaendig: w
                    .kind("dokumentationVollstaendig")
                    .is_some_and(|k| k.wert() == "true" || k.wert() == "1"),
                befunde: befund
                    .and_then(|b| b.kind("befundliste"))
                    .map(|l| {
                        l.kinder("befund")
                            .map(|b| FormellerBefund {
                                art: code(b.kind("artDesBefundes")),
                                text: b
                                    .kind("beschreibung")
                                    .map(Knoten::absaetze)
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                frist: datum(befund.and_then(|b| b.kind("frist"))).or(datum(w.kind("frist"))),
                anschreiben: w
                    .kind("anschreiben")
                    .map(Knoten::absaetze)
                    .unwrap_or_default(),
            }
        }
        "1160" => {
            let g = w
                .kind("gebuehrenbescheid")
                .ok_or_else(|| fehler("Gebührenbescheid ohne Inhalt"))?;
            Auswertung::Gebuehrenbescheid {
                vorauszahlung: g
                    .kind("istVorauszahlungsbescheid")
                    .is_some_and(|k| k.wert() == "true" || k.wert() == "1"),
                summe_cent: g
                    .pfad(&["kostenberechnung", "summe"])
                    .and_then(|k| k.wert().parse().ok()),
                begruendung: g
                    .kind("begruendung")
                    .map(Knoten::absaetze)
                    .unwrap_or_default(),
                verwendungszweck: g
                    .pfad(&["zahlungsdaten", "verwendungszweck"])
                    .map(Knoten::wert)
                    .unwrap_or_default(),
                frist: datum(w.kind("frist")),
                rechtsbehelfsbelehrung: g
                    .kind("rechtshelfsbelehrung")
                    .map(Knoten::absaetze)
                    .unwrap_or_default(),
            }
        }
        "0203" | "0213" | "0233" | "0263" => Auswertung::Anhoerung {
            aufforderung: w
                .kind("aufforderungZurStellungnahme")
                .map(Knoten::absaetze)
                .unwrap_or_default(),
            befunde: w
                .kind("bescheidlage")
                .map(|l| {
                    l.kinder("befund")
                        .map(|b| MateriellerBefund {
                            rechtsbereich: code(b.kind("rechtsbereich")),
                            regelverstoss: b
                                .kind("bezeichnungRegelverstoss")
                                .map(Knoten::wert)
                                .unwrap_or_default(),
                            begruendung: b
                                .kind("begruendung")
                                .map(Knoten::wert)
                                .unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            frist: datum(w.kind("frist")),
        },
        "0205" | "0215" | "0235" | "0265" => {
            let b = w
                .kind("bescheid")
                .ok_or_else(|| fehler("Bescheid ohne Inhalt"))?;
            let tenor = b.kind("tenor");
            Auswertung::Bescheid {
                genehmigung: tenor
                    .and_then(|t| t.kind("istGenehmigung"))
                    .is_some_and(|k| k.wert() == "true" || k.wert() == "1"),
                nebenbestimmungen: b
                    .kinder("nebenbestimmung")
                    .map(|n| {
                        let c = code(n.kind("art"));
                        BescheidNebenbestimmung {
                            art: nebenbestimmungsart(&c),
                            xbau_code: c,
                            text: n
                                .kind("nebenbestimmung")
                                .map(Knoten::absaetze)
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
                abweichungen: b
                    .kinder("abweichung")
                    .filter_map(|a| {
                        Some(BescheidAbweichung {
                            id: a.kind("id")?.wert().parse().ok()?,
                            zugelassen: a
                                .kind("istZugelassen")
                                .is_some_and(|k| k.wert() == "true" || k.wert() == "1"),
                            vorschrift: a
                                .kind("vorschrift")
                                .map(Knoten::absaetze)
                                .unwrap_or_default(),
                            begruendung: a
                                .kind("begruendung")
                                .map(Knoten::absaetze)
                                .unwrap_or_default(),
                            nebenbestimmung: a
                                .kind("nebenbestimmung")
                                .map(Knoten::absaetze)
                                .unwrap_or_default(),
                        })
                    })
                    .collect(),
                begruendung: b
                    .kind("begruendung")
                    .map(Knoten::absaetze)
                    .unwrap_or_default(),
                rechtsbehelfsbelehrung: b
                    .kind("rechtshelfsbelehrung")
                    .map(Knoten::absaetze)
                    .unwrap_or_default(),
                hinweise: b.kind("hinweise").map(Knoten::absaetze).unwrap_or_default(),
            }
        }
        _ => Auswertung::Sonstige,
    })
}
/// Nachrichten-XML aus einer Journaldatei (XML oder ZIP-Paket).
pub fn nachricht_xml(daten: &[u8]) -> CoreResult<Vec<u8>> {
    use std::io::Read;
    if !daten.starts_with(b"PK") {
        return Ok(daten.to_vec());
    }
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(daten))
        .map_err(|e| fehler(format!("ZIP nicht lesbar: {e}")))?;
    let mut treffer = Vec::new();
    for i in 0..z.len().min(256) {
        let mut f = z.by_index(i).map_err(|e| fehler(e.to_string()))?;
        let name = f
            .name()
            .map_err(|e| fehler(e.to_string()))?
            .to_ascii_lowercase();
        if !name.ends_with(".xml") || f.size() > MAX_XML {
            continue;
        }
        let mut b = Vec::new();
        (&mut f)
            .take(MAX_XML + 1)
            .read_to_end(&mut b)
            .map_err(|e| fehler(e.to_string()))?;
        if super::austausch::vorpruefen(&b).is_ok() {
            treffer.push(b);
        }
    }
    match treffer.len() {
        1 => Ok(treffer.remove(0)),
        _ => Err(fehler(
            "Genau eine lesbare XBau-Nachricht pro Paket erforderlich",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xbau::austausch::vorpruefen;

    const KOPF: &str = r#"<nachrichtenkopf.g2g><identifikation.nachricht><nachrichtenUUID>8d1d6a3e-0d6b-4c8f-9d6c-7b7d1f1b2a01</nachrichtenUUID><nachrichtentyp listURI="urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten" listVersionID="3.0"><code>CODE</code></nachrichtentyp><erstellungszeitpunkt>2026-03-02T09:00:00Z</erstellungszeitpunkt></identifikation.nachricht><leser><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>planer:test</kennung><name>Büro</name></leser><autor><verzeichnisdienst listVersionID="1.0"><code>DVDV</code></verzeichnisdienst><kennung>bab:test</kennung><name>Bauaufsicht</name></autor></nachrichtenkopf.g2g>"#;

    pub(crate) fn befundliste(vollstaendig: bool) -> String {
        format!(
            r#"<k:prozessnachrichten.formellePruefungBefundliste.1140 xmlns:k="http://www.xleitstelle.de/xbau/kernmodul/1/4/0" produkt="Test" produkthersteller="Muster" standard="XBau-Kernmodul" version="1.4.0">{}<bezug><referenz>4a0c8f2e-3c1b-4d5e-8f6a-1b2c3d4e5f60</referenz><vorgang>BA-2026-42</vorgang></bezug><dokumentationVollstaendig>{vollstaendig}</dokumentationVollstaendig><befundFrist><befundliste><befund><artDesBefundes listURI="urn:test:befundart" listVersionID="1"><code>01</code></artDesBefundes><beschreibung><textabsatz>Standsicherheitsnachweis fehlt</textabsatz><textabsatz>Bitte mit Prüfbericht &amp; Bescheinigung</textabsatz></beschreibung></befund><befund><artDesBefundes listURI="urn:test:befundart" listVersionID="1"><code>02</code></artDesBefundes><beschreibung><textabsatz>Lageplan ohne Unterschrift</textabsatz></beschreibung></befund></befundliste><frist>2026-04-15</frist></befundFrist><anschreiben><textabsatz>Ihr Antrag ist unvollständig.</textabsatz></anschreiben></k:prozessnachrichten.formellePruefungBefundliste.1140>"#,
            KOPF.replace("CODE", "1140")
        )
    }

    #[test]
    fn befundliste_wird_zu_befunden_mit_frist() {
        let xml = befundliste(false);
        let kopf = vorpruefen(xml.as_bytes()).unwrap();
        match auswerten(&kopf, xml.as_bytes()).unwrap() {
            Auswertung::Befundliste {
                vollstaendig,
                befunde,
                frist,
                anschreiben,
            } => {
                assert!(!vollstaendig);
                assert_eq!(befunde.len(), 2);
                assert_eq!(befunde[0].art, "01");
                assert_eq!(
                    befunde[0].text,
                    "Standsicherheitsnachweis fehlt\nBitte mit Prüfbericht & Bescheinigung"
                );
                assert_eq!(frist, Some("2026-04-15".parse().unwrap()));
                assert_eq!(anschreiben, "Ihr Antrag ist unvollständig.");
            }
            a => panic!("{a:?}"),
        }
    }

    #[test]
    fn formatierter_text_wird_lesbar() {
        let b64 = base64::engine::general_purpose::STANDARD
            .encode("<p>Die <b>Auflage</b> gilt.</p><p>Zweiter&nbsp;Absatz &amp; mehr</p>");
        assert_eq!(html_text(&b64), "Die Auflage gilt.\nZweiter Absatz & mehr");
    }

    #[test]
    fn falscher_wurzelname_wird_abgewiesen() {
        let xml = befundliste(true);
        let mut kopf = vorpruefen(xml.as_bytes()).unwrap();
        kopf.element = "prozessnachrichten.aktenzeichen.1121".into();
        assert!(auswerten(&kopf, xml.as_bytes()).is_err());
    }
}
