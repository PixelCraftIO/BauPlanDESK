//! Schreiben an die Behörde aus der Verfahrensakte: Antwort auf eine
//! Nachforderung (Punkt für Punkt mit Anlagenverzeichnis), Antrag auf
//! Fristverlängerung, Stellungnahme zur Anhörung, begründeter Antrag auf
//! Abweichung, Sachstandsanfrage und Rücknahme.
//!
//! Der Inhalt entsteht hier aus den Akteneinträgen; Satz und Briefbogen
//! liefert [`crate::druckausgabe::brief`]. Das PDF kann als Projektunterlage
//! (Kategorie „Schriftverkehr Behörde“) abgelegt und einem Abgabestand
//! beigefügt werden.
use super::*;
use crate::druckausgabe::brief::{Brief, Briefblock};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schreibenart {
    /// Antwort auf eine Nachforderung mit allen Punkten.
    Nachforderungsantwort { nachforderung: Uuid },
    /// Antrag auf Verlängerung einer Frist (Nachforderung oder Anhörung).
    Fristverlaengerung {
        /// Forderungspunkt oder Verfahrensschritt mit Frist.
        bezug: Uuid,
        bis: NaiveDate,
        grund: String,
    },
    /// Stellungnahme zu den Beanstandungen einer Anhörung.
    Stellungnahme { schritt: Uuid },
    /// Begründeter Antrag auf Zulassung der Abweichungen der Akte.
    Abweichungsantrag,
    /// Anfrage zum Stand der Bearbeitung.
    Sachstandsanfrage,
    /// Erklärung der Rücknahme des Antrags.
    Ruecknahme,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Schreibenwunsch {
    pub art: Schreibenart,
    /// Datum des Schreibens; ohne Angabe heute.
    pub datum: Option<NaiveDate>,
    /// Zusätzlicher Text vor dem Gruß.
    #[serde(default)]
    pub zusatz: String,
    /// Als Projektunterlage ablegen.
    #[serde(default)]
    pub ablegen: bool,
}
/// Erzeugtes Schreiben.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Schreibenergebnis {
    pub dateiname: String,
    pub brief: Brief,
    /// Hinweise zur Vollständigkeit (fehlende Antworten, Unterlagen …).
    pub hinweise: Vec<String>,
    /// Abgelegte Dokumentversion, wenn gewünscht.
    pub dokument: Option<Dokumentbezug>,
}
/// Personen und Namen, die das Schreiben braucht.
#[derive(Debug, Clone, Default)]
pub struct Briefkontext {
    pub buero: String,
    pub buero_anschrift: String,
    pub unterzeichner: String,
    pub land: Option<crate::domain::Bundesland>,
    /// Dateiname je Dokument-ID.
    pub dateinamen: std::collections::HashMap<crate::domain::DocumentId, String>,
}
fn datum(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}
fn absatz(t: impl Into<String>) -> Briefblock {
    Briefblock::Absatz { text: t.into() }
}
/// Erstes Einreichungsdatum der Akte (Versand, Eingang oder externe Einreichung).
fn eingereicht_am(a: &Verfahrensakte) -> Option<NaiveDate> {
    a.abgaben
        .iter()
        .flat_map(|b| &b.versand)
        .filter(|e| {
            matches!(
                e.status,
                abgabe::Versandstatus::Versendet | abgabe::Versandstatus::Eingegangen
            )
        })
        .map(|e| e.datum)
        .chain(a.externe_einreichungen.iter().map(|e| e.datum))
        .min()
}
/// Stellt das Schreiben zusammen. Rein; Fehler nennen fehlende Akteneinträge.
pub fn entwerfen(
    a: &Verfahrensakte,
    art: &Schreibenart,
    tag: NaiveDate,
    zusatz: &str,
    k: &Briefkontext,
) -> CoreResult<(Brief, Vec<String>)> {
    let mut b = Vec::new();
    let mut hinweise = Vec::new();
    let schreiben = |id: Uuid| a.kommunikation.iter().find(|c| c.id == id);
    let name = |d: &Dokumentbezug| {
        format!(
            "{} (Version {})",
            k.dateinamen
                .get(&d.dokument_id)
                .cloned()
                .unwrap_or_else(|| d.dokument_id.0.to_string()),
            d.version
        )
    };
    let betreff = match art {
        Schreibenart::Nachforderungsantwort { nachforderung } => {
            let n = a
                .nachforderungen
                .iter()
                .find(|n| n.id == *nachforderung)
                .ok_or_else(|| fehler("Nachforderung nicht in dieser Akte"))?;
            let c = schreiben(n.kommunikation);
            b.push(absatz(format!(
                "zu Ihrem Schreiben{} nehmen wir wie folgt Stellung und reichen die Unterlagen nach.",
                c.map(|c| format!(" vom {} ({})", datum(c.datum), c.betreff))
                    .unwrap_or_default()
            )));
            let mut anlagen: Vec<Dokumentbezug> = Vec::new();
            for (i, p) in n.punkte.iter().enumerate() {
                // Der Satz nummeriert Abschnitte selbst.
                b.push(Briefblock::Abschnitt {
                    titel: p.titel.clone(),
                });
                if p.status == Punktstatus::Entfallen {
                    b.push(absatz(format!(
                        "Entfallen: {}",
                        if p.antwort.trim().is_empty() {
                            "nach Abstimmung mit der Behörde"
                        } else {
                            p.antwort.trim()
                        }
                    )));
                    continue;
                }
                if p.antwort.trim().is_empty() {
                    hinweise.push(format!("Punkt {}: Antwort fehlt noch.", i + 1));
                    b.push(absatz("Die Antwort folgt gesondert."));
                } else {
                    b.push(absatz(p.antwort.trim()));
                }
                if !p.dokumente.is_empty() {
                    let nummern = p
                        .dokumente
                        .iter()
                        .map(|d| {
                            if !anlagen.contains(d) {
                                anlagen.push(d.clone());
                            }
                            format!(
                                "Anlage {}",
                                anlagen.iter().position(|x| x == d).expect("ergänzt") + 1
                            )
                        })
                        .collect::<Vec<_>>();
                    b.push(absatz(format!("Beigefügt: {}.", nummern.join(", "))));
                }
            }
            if !anlagen.is_empty() {
                let index = planindex::index_bei_einreichung(a, &anlagen);
                b.push(Briefblock::Abschnitt {
                    titel: "Anlagenverzeichnis".into(),
                });
                b.push(Briefblock::Tabelle {
                    koepfe: vec!["Anlage".into(), "Unterlage".into(), "Planindex".into()],
                    zeilen: anlagen
                        .iter()
                        .zip(index)
                        .enumerate()
                        .map(|(i, (d, x))| vec![(i + 1).to_string(), name(d), x])
                        .collect(),
                });
            }
            let frist = n
                .punkte
                .iter()
                .filter_map(|p| p.frist.as_ref().map(|f| f.datum))
                .min();
            if let Some(f) = frist
                && f < tag
            {
                hinweise.push(format!(
                    "Die Frist ist am {} abgelaufen; Folgen der Fristversäumnis prüfen{}.",
                    datum(f),
                    if k.land == Some(crate::domain::Bundesland::Nw) && a.art.baugenehmigung() {
                        " (§ 71 BauO NRW: Antrag gilt als zurückgenommen)"
                    } else {
                        ""
                    }
                ));
            }
            format!(
                "Nachreichung zur Nachforderung{}",
                c.map(|c| format!(" vom {}", datum(c.datum)))
                    .unwrap_or_default()
            )
        }
        Schreibenart::Fristverlaengerung { bezug, bis, grund } => {
            let (was, frist) = a
                .nachforderungen
                .iter()
                .flat_map(|n| &n.punkte)
                .find(|p| p.id == *bezug)
                .map(|p| (format!("zur Nachforderung „{}“", p.titel), p.frist.as_ref()))
                .or_else(|| {
                    a.schritte
                        .iter()
                        .find(|s| s.id == *bezug)
                        .map(|s| (format!("zur {}", s.art.bezeichnung()), s.frist.as_ref()))
                })
                .ok_or_else(|| fehler("Frist nicht in dieser Akte"))?;
            let frist = frist.ok_or_else(|| fehler("Der Eintrag hat keine Frist"))?;
            if *bis <= frist.datum {
                return Err(fehler("Die neue Frist liegt nicht nach der bisherigen"));
            }
            if grund.trim().is_empty() {
                return Err(fehler(
                    "Antrag auf Fristverlängerung braucht eine Begründung",
                ));
            }
            b.push(absatz(format!(
                "die Frist {was} endet am {}. Wir beantragen, sie bis zum {} zu verlängern.",
                datum(frist.datum),
                datum(*bis)
            )));
            b.push(absatz(format!("Begründung: {}", grund.trim())));
            if frist.datum < tag {
                hinweise.push(format!(
                    "Die Frist ist bereits am {} abgelaufen; eine Verlängerung ist vor Ablauf zu beantragen.",
                    datum(frist.datum)
                ));
            }
            "Antrag auf Fristverlängerung".into()
        }
        Schreibenart::Stellungnahme { schritt } => {
            let s = a
                .schritte
                .iter()
                .find(|s| s.id == *schritt && s.art == Schrittart::Anhoerung)
                .ok_or_else(|| fehler("Anhörung nicht in dieser Akte"))?;
            let c = schreiben(s.kommunikation);
            b.push(absatz(format!(
                "zu Ihrer Anhörung{} nehmen wir wie folgt Stellung.",
                c.map(|c| format!(" vom {}", datum(c.datum)))
                    .unwrap_or_default()
            )));
            for (i, g) in s.gruende.iter().enumerate() {
                b.push(Briefblock::Abschnitt {
                    titel: g.text.clone(),
                });
                if g.reaktion.trim().is_empty() {
                    hinweise.push(format!("Beanstandung {}: Reaktion fehlt noch.", i + 1));
                } else {
                    b.push(absatz(g.reaktion.trim()));
                }
                if let Some(e) = g
                    .abweichung
                    .and_then(|id| a.abweichungen.iter().find(|e| e.id == id))
                {
                    b.push(absatz(format!(
                        "Hierzu beantragen wir die {} nach {} ({}).",
                        e.art.bezeichnung(),
                        e.rechtsgrundlage.trim(),
                        e.vorschrift.trim()
                    )));
                }
            }
            if s.gruende.is_empty() && zusatz.trim().is_empty() {
                hinweise.push(
                    "Keine Beanstandungen erfasst; die Stellungnahme im Zusatztext formulieren."
                        .into(),
                );
            }
            "Stellungnahme zur Anhörung".into()
        }
        Schreibenart::Abweichungsantrag => {
            let aktiv: Vec<_> = a.abweichungen.iter().filter(|e| e.aktiv()).collect();
            if aktiv.is_empty() {
                return Err(fehler("Keine beantragten Abweichungen in dieser Akte"));
            }
            b.push(absatz(
                "für das Bauvorhaben beantragen wir die Zulassung der folgenden Abweichungen, Ausnahmen und Befreiungen.",
            ));
            for e in &aktiv {
                b.push(Briefblock::Abschnitt {
                    titel: format!("{} nach {}", e.art.bezeichnung(), e.rechtsgrundlage.trim()),
                });
                b.push(absatz(format!(
                    "Gegenstand: {}\nBetroffene Vorschrift: {}\nSoll: {} · Ist: {}",
                    e.gegenstand.trim(),
                    e.vorschrift.trim(),
                    e.soll.trim(),
                    e.ist.trim()
                )));
                if e.begruendung.trim().is_empty() {
                    hinweise.push(format!("{}: Begründung fehlt.", e.gegenstand.trim()));
                } else {
                    b.push(absatz(format!("Begründung: {}", e.begruendung.trim())));
                }
                for n in &e.nachbarn {
                    b.push(absatz(format!(
                        "Angrenzer {}{}: {}",
                        n.name.trim(),
                        if n.flurstueck.trim().is_empty() {
                            String::new()
                        } else {
                            format!(" (Flurstück {})", n.flurstueck.trim())
                        },
                        match n.zugestimmt {
                            Some(true) =>
                                format!("zugestimmt am {}", n.datum.map(datum).unwrap_or_default()),
                            Some(false) => "nicht zugestimmt".into(),
                            None => "angefragt".into(),
                        }
                    )));
                }
            }
            "Antrag auf Abweichung, Ausnahme und Befreiung".into()
        }
        Schreibenart::Sachstandsanfrage => {
            let ein = eingereicht_am(a)
                .ok_or_else(|| fehler("Sachstandsanfrage setzt eine Einreichung voraus"))?;
            b.push(absatz(format!(
                "am {} haben wir den Antrag eingereicht{}. Bitte teilen Sie uns den Stand der Bearbeitung und den voraussichtlichen Zeitpunkt der Entscheidung mit.",
                datum(ein),
                a.vollstaendigkeit
                    .as_ref()
                    .map(|v| format!("; die Vollständigkeit haben Sie am {} bestätigt", datum(v.datum)))
                    .unwrap_or_default()
            )));
            if let Some(f) = &a.bearbeitungsfrist {
                b.push(absatz(format!(
                    "Die von Ihnen genannte Bearbeitungsfrist endete am {}.",
                    datum(f.datum)
                )));
            }
            "Anfrage zum Sachstand".into()
        }
        Schreibenart::Ruecknahme => {
            b.push(absatz(format!(
                "hiermit nehmen wir den Antrag „{}“{} zurück.",
                a.bezeichnung.trim(),
                eingereicht_am(a)
                    .map(|d| format!(" vom {}", datum(d)))
                    .unwrap_or_default()
            )));
            hinweise.push(
                "Die Rücknahme ist von der Bauherrschaft oder mit ihrer Vollmacht zu erklären; Gebührenfolgen prüfen."
                    .into(),
            );
            "Rücknahme des Antrags".into()
        }
    };
    let mut bloecke = vec![absatz("Sehr geehrte Damen und Herren,")];
    bloecke.extend(b);
    if !zusatz.trim().is_empty() {
        bloecke.push(absatz(zusatz.trim()));
    }
    bloecke.push(absatz("Mit freundlichen Grüßen"));
    bloecke.push(Briefblock::Unterschrift {
        felder: vec![(
            if matches!(art, Schreibenart::Ruecknahme) {
                "Bauherrschaft / Bevollmächtigte".into()
            } else {
                "Entwurfsverfasser/in".into()
            },
            k.unterzeichner.clone(),
        )],
    });
    Ok((
        Brief {
            absender: [k.buero.as_str(), k.buero_anschrift.as_str()]
                .iter()
                .filter(|t| !t.trim().is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join("\n"),
            empfaenger: a.behoerde.name.clone(),
            datum: tag,
            betreff: format!("{} – {}", a.bezeichnung.trim(), betreff),
            aktenzeichen: a.aktenzeichen.trim().to_string(),
            bloecke,
        },
        hinweise,
    ))
}
impl CoreServices {
    /// Erzeugt das Schreiben als PDF und legt es auf Wunsch als
    /// Projektunterlage (Schriftverkehr Behörde) ab.
    pub async fn verfahren_schreiben(
        &self,
        p: ProjectId,
        vid: Uuid,
        w: Schreibenwunsch,
        wer: &Benutzer,
    ) -> CoreResult<(Schreibenergebnis, Vec<u8>)> {
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Lesen)
            .await?;
        if w.zusatz.len() > 16000 {
            return Err(fehler("Zusatztext höchstens 16000 Zeichen"));
        }
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let projekt = self.get_project(p).await?;
        let buero = self.buero().await?;
        let antrag = self.bauantrag(p).await?.antrag;
        let anschrift = &buero.anschrift;
        let k = Briefkontext {
            buero: buero.name.clone(),
            buero_anschrift: format!(
                "{} {}, {} {}",
                anschrift.strasse, anschrift.hausnummer, anschrift.postleitzahl, anschrift.ort
            )
            .trim_matches([' ', ','])
            .to_string(),
            unterzeichner: antrag
                .entwurfsverfasser_von(&buero)
                .map(|e| format!("{} {}", e.vorname, e.familienname))
                .unwrap_or_default(),
            land: Some(projekt.bundesland),
            dateinamen: self
                .list_documents(p)
                .await?
                .into_iter()
                .map(|d| (d.document_id, d.file_name))
                .collect(),
        };
        let tag = w.datum.unwrap_or_else(crate::kalender::heute);
        let (brief, hinweise) = entwerfen(&a, &w.art, tag, &w.zusatz, &k)?;
        let pdf = self.brief_pdf(p, &brief).await?;
        let dateiname = format!(
            "{} {}.pdf",
            tag.format("%Y-%m-%d"),
            brief
                .betreff
                .rsplit(" – ")
                .next()
                .unwrap_or("Schreiben")
                .replace(['/', '\\', ':'], "-")
        );
        let dokument = if w.ablegen {
            self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
                .await?;
            let d = self
                .ablegen_vorgang(
                    p,
                    None,
                    None,
                    &dateiname,
                    bytes::Bytes::from(pdf.clone()),
                    wer,
                    crate::domain::Aktion::DokumentAbgelegt,
                    "Schreiben an die Behörde aus der Verfahrensakte",
                    Some(Uuid::new_v4()),
                    vec![crate::commit::DokumentZusatz {
                        modul: "dokumentkategorie".into(),
                        schema_version: 1,
                        daten: serde_json::json!(
                            self.kategorienkatalog()
                                .await?
                                .kategorien
                                .iter()
                                .any(|k| k.id == "schriftverkehr" && k.aktiv)
                                .then_some("schriftverkehr")
                        ),
                    }],
                )
                .await?;
            Some(Dokumentbezug {
                dokument_id: d.document_id,
                version: d.version,
            })
        } else {
            None
        };
        Ok((
            Schreibenergebnis {
                dateiname,
                brief,
                hinweise,
                dokument,
            },
            pdf,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn akte() -> Verfahrensakte {
        serde_json::from_value(serde_json::json!({
            "bezeichnung": "Bauantrag EFH", "art": "bauantrag", "status": "unvollstaendig",
            "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht Musterstadt", "kennung": "", "kontakt": ""},
            "aktenzeichen": "BA-42", "verantwortlich": null, "vorgaenger": null, "profil": null,
            "kommunikation": [], "nachforderungen": [], "anforderungen": [],
            "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
        }))
        .unwrap()
    }
    #[test]
    fn antwort_nennt_jeden_punkt_und_die_anlagen() {
        let mut a = akte();
        let c = Uuid::new_v4();
        a.kommunikation.push(Kommunikation {
            id: c,
            art: Kommunikationsart::Brief,
            eingehend: true,
            datum: "2026-04-01".parse().unwrap(),
            betreff: "Unvollständiger Bauantrag".into(),
            beteiligte: String::new(),
            inhalt: String::new(),
            original: None,
            bezuege: vec![],
        });
        let doc = Dokumentbezug {
            dokument_id: crate::domain::DocumentId(Uuid::new_v4()),
            version: 2,
        };
        let punkt = |titel: &str, antwort: &str, docs: Vec<Dokumentbezug>| Forderungspunkt {
            id: Uuid::new_v4(),
            titel: titel.into(),
            grundlage: String::new(),
            verantwortlich: None,
            frist: Some(Frist {
                datum: "2026-04-30".parse().unwrap(),
                ausloeser: String::new(),
                beleg: None,
                erinnerung_tage: 7,
                grund: String::new(),
            }),
            status: Punktstatus::Vorbereitet,
            antwort: antwort.into(),
            dokumente: docs,
            abgaben: vec![],
            bestaetigung: None,
            anforderung: None,
        };
        let nid = Uuid::new_v4();
        a.nachforderungen.push(Nachforderung {
            id: nid,
            kommunikation: c,
            punkte: vec![
                punkt("Standsicherheitsnachweis", "Liegt bei.", vec![doc.clone()]),
                punkt("Lageplan unterschreiben", "", vec![]),
            ],
        });
        let mut k = Briefkontext {
            buero: "Planungsbüro Muster".into(),
            unterzeichner: "Anna Architektin".into(),
            land: Some(crate::domain::Bundesland::Nw),
            ..Default::default()
        };
        k.dateinamen.insert(doc.dokument_id, "Statik.pdf".into());
        let (b, h) = entwerfen(
            &a,
            &Schreibenart::Nachforderungsantwort { nachforderung: nid },
            "2026-05-10".parse().unwrap(),
            "",
            &k,
        )
        .unwrap();
        assert!(
            b.betreff
                .contains("Nachreichung zur Nachforderung vom 01.04.2026")
        );
        assert!(b.bloecke.contains(&Briefblock::Abschnitt {
            titel: "Standsicherheitsnachweis".into()
        }));
        assert!(b.bloecke.contains(&absatz("Beigefügt: Anlage 1.")));
        assert!(b.bloecke.iter().any(|x| matches!(x, Briefblock::Tabelle { zeilen, .. } if zeilen[0][1] == "Statik.pdf (Version 2)")));
        assert!(h.iter().any(|h| h.contains("Punkt 2: Antwort fehlt")));
        assert!(h.iter().any(|h| h.contains("§ 71 BauO NRW")), "{h:?}");
        assert_eq!(b.aktenzeichen, "BA-42");
    }
    #[test]
    fn fristverlaengerung_braucht_spaetere_frist_und_grund() {
        let mut a = akte();
        let sid = Uuid::new_v4();
        a.schritte.push(Verfahrensschritt {
            id: sid,
            art: Schrittart::Anhoerung,
            kommunikation: Uuid::new_v4(),
            frist: Some(Frist {
                datum: "2026-06-01".parse().unwrap(),
                ausloeser: String::new(),
                beleg: None,
                erinnerung_tage: 7,
                grund: String::new(),
            }),
            stellungnahme: None,
            verantwortlich: None,
            folgeschreiben: None,
            gruende: vec![],
        });
        let k = Briefkontext::default();
        let art = |bis: &str, grund: &str| Schreibenart::Fristverlaengerung {
            bezug: sid,
            bis: bis.parse().unwrap(),
            grund: grund.into(),
        };
        let tag = "2026-05-20".parse().unwrap();
        assert!(entwerfen(&a, &art("2026-05-30", "x"), tag, "", &k).is_err());
        assert!(entwerfen(&a, &art("2026-07-01", " "), tag, "", &k).is_err());
        let (b, _) = entwerfen(&a, &art("2026-07-01", "Gutachten in Arbeit"), tag, "", &k).unwrap();
        assert!(b.bloecke.iter().any(
            |x| matches!(x, Briefblock::Absatz { text } if text.contains("bis zum 01.07.2026"))
        ));
    }
}
