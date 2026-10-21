//! Folgenachrichten des Büros im laufenden Verfahren: Stellungnahme zur
//! Anhörung (0204), isolierter Abweichungsantrag (0220), Antrag auf
//! Verlängerung (0260), Baubeginnsanzeige (0900), Anzeige der
//! Nutzungsaufnahme (0910) und Rücknahme (1130).
//!
//! Sie verwenden dieselben Bausteine wie die Antragsnachrichten
//! ([`super::antrag`]); die Tests prüfen jede Art mit `xmllint` gegen die
//! mitgelieferten Schemas.
use base64::Engine as _;
use chrono::{NaiveDate, SecondsFormat};

use super::antrag::{
    Nachrichtendaten, akteur, anlagen_schreiben, bauvorhaben, beteiligte, kopf, mime_pruefen,
    wurzel_auf,
};
use super::austausch::Nachrichtenkopf;
use super::codeliste::{ANLAGEN, MIMETYPES, VERZEICHNISDIENST};
use super::inhalt::{self, BeantragteAbweichung};
use super::xml::XmlSchreiber;
use crate::error::{CoreError, CoreResult};

/// Stellungnahme zu einer Beanstandung der Anhörung.
#[derive(Debug, Clone, PartialEq)]
pub struct Stellungnahmepunkt {
    pub regelverstoss: String,
    pub stellungnahme: String,
}

/// Art der Folgenachricht mit ihren besonderen Angaben.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Folgeart<'a> {
    /// `baugenehmigung.stellungnahme.0204` zur Anhörung `anhoerung`.
    Stellungnahme {
        anhoerung: &'a Nachrichtenkopf,
        punkte: &'a [Stellungnahmepunkt],
    },
    /// `abweichung.antrag.0220`, etwa neben einer Genehmigungsfreistellung.
    Abweichungsantrag {
        abweichungen: &'a [BeantragteAbweichung],
    },
    /// `verlaengerung.antrag.0260` für die erteilte Genehmigung.
    Verlaengerung { bis: Option<NaiveDate> },
    /// `anzeige.baubeginn.0900`.
    Baubeginn {
        datum: NaiveDate,
        pruefung_standsicherheit: bool,
    },
    /// `anzeige.nutzungsaufnahme.0910`.
    Nutzungsaufnahme {
        datum: NaiveDate,
        standsicherheit_bescheinigt: bool,
        brandschutz_bescheinigt: bool,
        gebaeudeklasse4: bool,
        wiederkehrende_pruefungen: bool,
    },
    /// `prozessnachrichten.ruecknahme.1130`.
    Ruecknahme { information: &'a str },
}
impl Folgeart<'_> {
    fn wurzel(&self) -> &'static str {
        match self {
            Self::Stellungnahme { .. } => "xbau:baugenehmigung.stellungnahme.0204",
            Self::Abweichungsantrag { .. } => "xbau:abweichung.antrag.0220",
            Self::Verlaengerung { .. } => "xbau:verlaengerung.antrag.0260",
            Self::Baubeginn { .. } => "xbau:anzeige.baubeginn.0900",
            Self::Nutzungsaufnahme { .. } => "xbau:anzeige.nutzungsaufnahme.0910",
            Self::Ruecknahme { .. } => "k:prozessnachrichten.ruecknahme.1130",
        }
    }
    /// Wurzelelement ohne Präfix.
    pub fn element(&self) -> &'static str {
        self.wurzel()
            .split_once(':')
            .map(|(_, e)| e)
            .unwrap_or_default()
    }
    pub fn dateiname(&self) -> String {
        format!("{}.xml", self.element())
    }
    fn kern(&self) -> bool {
        matches!(self, Self::Ruecknahme { .. })
    }
}
fn absaetze(x: &mut XmlSchreiber, name: &'static str, text: &str) {
    let absaetze: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|z| !z.is_empty())
        .collect();
    if absaetze.is_empty() {
        return;
    }
    x.auf(name, &[]);
    for a in absaetze {
        x.text("textabsatz", a);
    }
    x.zu();
}
/// `xbauk:TextFormatiert`: Absätze als HTML in Base64.
fn formatiert(x: &mut XmlSchreiber, name: &'static str, text: &str) {
    let html: String = text
        .lines()
        .map(str::trim)
        .filter(|z| !z.is_empty())
        .map(|z| {
            format!(
                "<p>{}</p>",
                z.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            )
        })
        .collect();
    x.auf(name, &[])
        .text(
            "text",
            &base64::engine::general_purpose::STANDARD.encode(html),
        )
        .zu();
}
/// Bezug auf den Antrag (Referenz und Aktenzeichen).
fn bezug(
    x: &mut XmlSchreiber,
    name: &'static str,
    d: &Nachrichtendaten<'_>,
    aktenzeichen: &str,
    nachricht: Option<&Nachrichtenkopf>,
) {
    x.auf(name, &[])
        .text("referenz", &d.antrag.vorgang.to_string());
    if !aktenzeichen.trim().is_empty() {
        x.text("vorgang", aktenzeichen.trim());
    }
    if let Some(k) = nachricht {
        let (liste, version) = if k.namensraum == super::austausch::KERN {
            (
                "urn:xoev-de:xbau-kernmodul:codeliste:xbau-kernmodul-nachrichten",
                "3.0",
            )
        } else {
            ("urn:xoev-de:xbau:codeliste:xbau-nachrichten", "5.0")
        };
        x.auf("bezugNachricht", &[])
            .text("bn:nachrichtenUUID", &k.id.to_string())
            .auf(
                "bn:nachrichtentyp",
                &[("listURI", liste), ("listVersionID", version)],
            )
            .text("code", k.element.rsplit('.').next().unwrap_or_default())
            .zu()
            .text(
                "bn:erstellungszeitpunkt",
                &k.erstellt.to_rfc3339_opts(SecondsFormat::Secs, true),
            )
            .zu();
    }
    x.zu();
}
/// Erzeugt die Folgenachricht. `aktenzeichen` ist das der Verfahrensakte.
pub fn folgenachricht(
    d: &Nachrichtendaten<'_>,
    art: Folgeart<'_>,
    aktenzeichen: &str,
) -> CoreResult<String> {
    let fehlt = |m: &str| {
        Err(CoreError::InvalidInput(format!(
            "Für die XBau-Nachricht fehlt noch: {m}."
        )))
    };
    if d.antrag.bauaufsicht.kennung.trim().is_empty() {
        return fehlt("Behördenkennung (DVDV) der Bauaufsicht");
    }
    if d.buero.kennung.trim().is_empty() {
        return fehlt("Kennung des Büros (Absender)");
    }
    let mit_antragsteller = matches!(
        art,
        Folgeart::Abweichungsantrag { .. } | Folgeart::Verlaengerung { .. }
    );
    if (mit_antragsteller || matches!(art, Folgeart::Baubeginn { .. })) && d.bauherren.is_empty() {
        return fehlt("Bauherr (Projektbeteiligte)");
    }
    let anlagenpflicht = matches!(
        art,
        Folgeart::Abweichungsantrag { .. } | Folgeart::Baubeginn { .. }
    );
    if anlagenpflicht && d.anlagen.is_empty() {
        return fehlt("mindestens eine Anlage");
    }
    for a in d.anlagen {
        if a.anlageart.trim().is_empty() {
            return fehlt(&format!(
                "Art der Anlage für „{}“ (Dokumentkategorie oder Anlagenwahl im Bauantrag)",
                a.bezeichnung.trim()
            ));
        }
    }
    let verzeichnis = d
        .codelisten
        .benoetigt(VERZEICHNISDIENST, "Verzeichnisdienst")?;
    let kanaele = inhalt::kanaele(d.codelisten);
    let listen = if d.anlagen.is_empty() {
        None
    } else {
        let anlagen = d.codelisten.benoetigt(ANLAGEN, "XBau Anlage Art")?;
        let mimes = d.codelisten.benoetigt(MIMETYPES, "XBau MIME-Types")?;
        for a in d.anlagen {
            if !anlagen.enthaelt(&a.anlageart) {
                return Err(CoreError::InvalidInput(format!(
                    "Anlage „{}“: Die Art {} steht nicht in der Codeliste {} (Version {}).",
                    a.bezeichnung, a.anlageart, anlagen.name, anlagen.version
                )));
            }
            mime_pruefen(&a.dateiname, mimes)?;
        }
        Some((anlagen, mimes))
    };

    let mut x = XmlSchreiber::new();
    wurzel_auf(&mut x, art.wurzel(), art.kern());
    kopf(&mut x, d, art.element(), verzeichnis, art.kern());
    let referenz = |x: &mut XmlSchreiber, name: &'static str| {
        x.auf(name, &[])
            .text("referenzAntragsservice", &d.antrag.vorgang.to_string())
            .zu();
    };
    let antragsteller = |x: &mut XmlSchreiber| {
        for b in d.bauherren {
            x.auf("xbau:antragsteller", &[]);
            akteur(x, b, kanaele);
            x.zu();
        }
    };
    let anlagen = |x: &mut XmlSchreiber, element: &'static str| {
        if let Some((a, m)) = listen {
            anlagen_schreiben(x, d, a, m, element);
        }
    };
    match art {
        Folgeart::Stellungnahme { anhoerung, punkte } => {
            bezug(&mut x, "xbau:bezug", d, aktenzeichen, Some(anhoerung));
            if punkte.is_empty() {
                return fehlt("Stellungnahme zu mindestens einer Beanstandung");
            }
            for p in punkte {
                if p.stellungnahme.trim().is_empty() {
                    return fehlt(&format!("Stellungnahme zu „{}“", p.regelverstoss.trim()));
                }
                x.auf("xbau:stellungnahmeBelastenderVerwaltungsakt", &[])
                    .text("xbau:bezeichnungRegelverstoss", p.regelverstoss.trim());
                formatiert(&mut x, "xbau:stellungnahme", &p.stellungnahme);
                x.zu();
            }
            anlagen(&mut x, "xbau:anlagen");
        }
        Folgeart::Abweichungsantrag { abweichungen } => {
            if abweichungen.iter().all(|a| a.erleichterung) {
                return fehlt("beantragte Abweichung mit Code der Codeliste „Abweichung Art“");
            }
            referenz(&mut x, "xbau:referenzAntragsservice");
            antragsteller(&mut x);
            let ohne_erleichterung = inhalt::Vorhabenangaben {
                abweichungen: abweichungen
                    .iter()
                    .filter(|a| !a.erleichterung)
                    .cloned()
                    .collect(),
                ..Default::default()
            };
            inhalt::abweichungen(&mut x, &ohne_erleichterung);
            bauvorhaben(&mut x, d, kanaele, "xbau:bauvorhaben")?;
            anlagen(&mut x, "xbau:anlagen");
        }
        Folgeart::Verlaengerung { bis } => {
            referenz(&mut x, "xbau:referenzAntragsservice");
            antragsteller(&mut x);
            bezug(&mut x, "xbau:bezug", d, aktenzeichen, None);
            if let Some(b) = bis {
                x.text("xbau:begehrteVerlaengerung", &b.to_string());
            }
            anlagen(&mut x, "xbau:anlagen");
        }
        Folgeart::Baubeginn {
            datum,
            pruefung_standsicherheit,
        } => {
            referenz(&mut x, "xbau:referenzAnzeigeservice");
            x.auf("xbau:bezug", &[]);
            bezug(&mut x, "xbau:mitAktenzeichen", d, aktenzeichen, None);
            x.zu();
            x.auf("xbau:sachverhalt", &[])
                .auf("xbau:baubeginn", &[])
                .text("xbau:datum", &datum.to_string())
                .zu()
                .zu();
            x.auf("xbau:nachweise", &[])
                .text(
                    "xbau:pruefungStandsicherheitsnachweisErforderlich",
                    if pruefung_standsicherheit {
                        "true"
                    } else {
                        "false"
                    },
                )
                .zu();
            beteiligte(&mut x, d, kanaele)?;
            anlagen(&mut x, "xbau:anlagen");
        }
        Folgeart::Nutzungsaufnahme {
            datum,
            standsicherheit_bescheinigt,
            brandschutz_bescheinigt,
            gebaeudeklasse4,
            wiederkehrende_pruefungen,
        } => {
            let b = |v: bool| if v { "true" } else { "false" };
            referenz(&mut x, "xbau:referenzAnzeigeservice");
            x.auf("xbau:bezug", &[]);
            bezug(&mut x, "xbau:mitAktenzeichen", d, aktenzeichen, None);
            x.zu();
            x.auf("xbau:sachverhalt", &[])
                .auf("xbau:geplanteNutzungsaufnahme", &[])
                .text("xbau:datum", &datum.to_string())
                .zu()
                .zu();
            x.auf("xbau:nachweise", &[])
                .text(
                    "xbau:standsicherheitBescheinigt",
                    b(standsicherheit_bescheinigt),
                )
                .text("xbau:brandschutzBescheinigt", b(brandschutz_bescheinigt))
                .text("xbau:gebaeudeDerKlasse4", b(gebaeudeklasse4))
                .text(
                    "xbau:wiederkehrendePruefungenErforderlich",
                    b(wiederkehrende_pruefungen),
                )
                .zu();
            if !d.bauherren.is_empty() {
                beteiligte(&mut x, d, kanaele)?;
            }
            anlagen(&mut x, "xbau:anlagen");
        }
        Folgeart::Ruecknahme { information } => {
            bezug(&mut x, "bezug", d, aktenzeichen, None);
            absaetze(&mut x, "information", information);
            anlagen(&mut x, "anlagen");
        }
    }
    x.zu();
    x.fertig()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xbau::antrag::tests::{anlagen, beispiel, daten, schemafehler, test_codelisten};
    use uuid::Uuid;

    fn pruefen(art: Folgeart<'_>, mit_anlagen: bool) -> String {
        let (projekt, antrag, buero) = beispiel();
        let a = anlagen();
        let c = test_codelisten();
        let leer = [];
        let d = daten(
            &projekt,
            &antrag,
            &buero,
            if mit_anlagen { &a } else { &leer },
            &c,
        );
        let xml = folgenachricht(&d, art, "BA-2026-42").unwrap();
        if let Some(r) = schemafehler(&xml) {
            r.unwrap_or_else(|e| panic!("{}: {e}\n{xml}", art.element()));
        }
        let kopf = crate::xbau::austausch::vorpruefen(xml.as_bytes()).unwrap();
        assert_eq!(kopf.element, art.element());
        xml
    }

    #[test]
    fn alle_folgenachrichten_sind_schemakonform() {
        let anhoerung = Nachrichtenkopf {
            id: Uuid::new_v4(),
            element: "baugenehmigung.anhoerung.0203".into(),
            namensraum: crate::xbau::austausch::HOCHBAU.into(),
            titel: "Anhörung".into(),
            erstellt: "2026-05-04T10:00:00Z".parse().unwrap(),
            absender: "bab:test".into(),
            empfaenger: "planer:test".into(),
            antragsreferenz: None,
            aktenzeichen: Some("BA-2026-42".into()),
            bezugsnachricht: None,
        };
        let punkte = [Stellungnahmepunkt {
            regelverstoss: "Überschreitung der GRZ".into(),
            stellungnahme:
                "Die Überschreitung ist durch die Befreiung gedeckt.\nSiehe Anlage <1> & 2".into(),
        }];
        let xml = pruefen(
            Folgeart::Stellungnahme {
                anhoerung: &anhoerung,
                punkte: &punkte,
            },
            false,
        );
        assert!(xml.contains(&anhoerung.id.to_string()));
        let abw = [BeantragteAbweichung {
            id: Uuid::new_v4(),
            erleichterung: false,
            liste: crate::verfahren::abweichungen::ABWEICHUNGART.into(),
            version: "2.0".into(),
            code: "1".into(),
            vorschrift: "§ 6 BauO NRW".into(),
            beschreibung: "Garage".into(),
            begruendung: "Belichtung gewahrt".into(),
        }];
        pruefen(Folgeart::Abweichungsantrag { abweichungen: &abw }, true);
        pruefen(
            Folgeart::Verlaengerung {
                bis: Some("2029-05-01".parse().unwrap()),
            },
            false,
        );
        pruefen(
            Folgeart::Baubeginn {
                datum: "2026-06-01".parse().unwrap(),
                pruefung_standsicherheit: true,
            },
            true,
        );
        pruefen(
            Folgeart::Nutzungsaufnahme {
                datum: "2027-06-01".parse().unwrap(),
                standsicherheit_bescheinigt: true,
                brandschutz_bescheinigt: true,
                gebaeudeklasse4: false,
                wiederkehrende_pruefungen: false,
            },
            false,
        );
        pruefen(
            Folgeart::Ruecknahme {
                information: "Der Antrag wird zurückgenommen.",
            },
            true,
        );
    }

    #[test]
    fn pflichtangaben_werden_genannt() {
        let (projekt, antrag, buero) = beispiel();
        let c = test_codelisten();
        let d = daten(&projekt, &antrag, &buero, &[], &c);
        let e = folgenachricht(
            &d,
            Folgeart::Baubeginn {
                datum: "2026-06-01".parse().unwrap(),
                pruefung_standsicherheit: false,
            },
            "",
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("Anlage"), "{e}");
    }
}
