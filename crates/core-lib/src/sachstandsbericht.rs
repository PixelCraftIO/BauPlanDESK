//! Sachstandsbericht für den Bauherrn: Verfahrensstand, eingereichte
//! Unterlagen, offene Nachforderungen, nächste Fristen und Bescheide.
//!
//! Vertrag:
//! - Nur Behördenvorgänge aus den Verfahrensakten. Aufgaben, Projekttagebuch,
//!   Fachplanerlieferungen und Bearbeiter des Büros stehen nicht im Bericht
//!   (bürointern).
//! - Fristen sind dieselben wie im Büro-Cockpit ([`crate::verfahren::termine`]),
//!   ab dem Stichtag, höchstens zwölf.
//! - Ergebnis ist ein [`Brief`] an den ersten Bauherrn; gesetzt wird er im
//!   Briefbogen des Büros ([`CoreServices::brief_pdf`]).
use crate::{
    CoreResult, CoreServices,
    benutzer::Benutzer,
    cockpit::projekt_auswerten,
    domain::ProjectId,
    druckausgabe::brief::{Brief, Briefblock},
    verfahren::{Entscheidung, Punktstatus, termine::offene_forderungspunkte},
    zugriff::ProjektRecht,
};
use chrono::NaiveDate;

fn datum(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}
fn entscheidung(e: &Entscheidung) -> &'static str {
    match e {
        Entscheidung::Genehmigt => "Genehmigt",
        Entscheidung::TeilweiseGenehmigt => "Teilweise genehmigt",
        Entscheidung::Abgelehnt => "Abgelehnt",
        Entscheidung::Sonstiges => "Entschieden",
    }
}

impl CoreServices {
    pub async fn sachstandsbericht(&self, p: ProjectId, wer: &Benutzer) -> CoreResult<Brief> {
        self.sachstandsbericht_am(p, crate::kalender::heute(), wer)
            .await
    }
    pub async fn sachstandsbericht_am(
        &self,
        p: ProjectId,
        heute: NaiveDate,
        wer: &Benutzer,
    ) -> CoreResult<Brief> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let projekt = self.get_project(p).await?;
        let bauherren = self.bauherren(p).await?;
        let buero = self.buero().await.ok();
        let (akten, _) = self.cockpit_quellen(&projekt, heute).await?;
        let (stand, termine) = projekt_auswerten(&projekt, &akten, vec![], heute);
        let uebersicht = self.projektuebersicht_am(p, heute, wer).await?;

        let absender = buero
            .as_ref()
            .map(|b| {
                let a = &b.anschrift;
                format!(
                    "{}\n{} {}\n{} {}",
                    b.name, a.strasse, a.hausnummer, a.postleitzahl, a.ort
                )
            })
            .unwrap_or_default();
        let empfaenger = bauherren
            .first()
            .map(|k| {
                let a = &k.daten.anschrift;
                format!(
                    "{}\n{} {}\n{} {}",
                    k.anzeigename(),
                    a.strasse,
                    a.hausnummer,
                    a.postleitzahl,
                    a.ort
                )
                .trim()
                .to_string()
            })
            .unwrap_or_default();
        let mut b = vec![
            Briefblock::Absatz {
                text: "Sehr geehrte Damen und Herren,".into(),
            },
            Briefblock::Absatz {
                text: format!(
                    "wir informieren Sie über den Stand Ihres Bauvorhabens „{}“ zum {}.",
                    projekt.name,
                    datum(heute)
                ),
            },
        ];

        b.push(Briefblock::Abschnitt {
            titel: "Verfahrensstand".into(),
        });
        if stand.verfahren.is_empty() {
            b.push(Briefblock::Absatz {
                text: "Ein Verfahren bei der Bauaufsicht ist noch nicht eröffnet; die Bauvorlagen werden vorbereitet.".into(),
            });
        } else {
            b.push(Briefblock::Tabelle {
                koepfe: vec![
                    "Verfahren".into(),
                    "Behörde / Aktenzeichen".into(),
                    "Stand".into(),
                    "Letzte Abgabe".into(),
                    "Bescheid".into(),
                ],
                zeilen: stand
                    .verfahren
                    .iter()
                    .map(|v| {
                        vec![
                            v.bezeichnung.clone(),
                            [v.behoerde.as_str(), v.aktenzeichen.as_str()]
                                .into_iter()
                                .filter(|s| !s.trim().is_empty())
                                .collect::<Vec<_>>()
                                .join(" · "),
                            v.status_bezeichnung.clone(),
                            v.letzte_abgabe.map(datum).unwrap_or_else(|| "–".into()),
                            v.letzter_bescheid
                                .as_ref()
                                .map(|x| {
                                    format!(
                                        "{} am {}",
                                        entscheidung(&x.entscheidung),
                                        datum(x.datum)
                                    )
                                })
                                .unwrap_or_else(|| "noch keiner".into()),
                        ]
                    })
                    .collect(),
            });
        }

        let abgaben: Vec<_> = uebersicht
            .abgaben
            .iter()
            .filter(|a| a.datum.is_some())
            .collect();
        if !abgaben.is_empty() {
            b.push(Briefblock::Abschnitt {
                titel: "Eingereichte Unterlagen".into(),
            });
            b.push(Briefblock::Tabelle {
                koepfe: vec![
                    "Datum".into(),
                    "Abgabe".into(),
                    "Empfänger".into(),
                    "Unterlagen".into(),
                ],
                zeilen: abgaben
                    .iter()
                    .map(|a| {
                        vec![
                            a.datum.map(datum).unwrap_or_default(),
                            a.titel.clone(),
                            a.empfaenger.clone(),
                            a.dokumente.to_string(),
                        ]
                    })
                    .collect(),
            });
        }

        let offen: Vec<Vec<String>> = akten
            .iter()
            .flat_map(|(_, a)| {
                offene_forderungspunkte(a).map(|r| {
                    vec![
                        a.bezeichnung.clone(),
                        r.titel.clone(),
                        r.frist
                            .as_ref()
                            .map(|f| datum(f.datum))
                            .unwrap_or_else(|| "–".into()),
                        match r.status {
                            Punktstatus::Eingereicht => {
                                "Beantwortet, Bestätigung der Behörde steht aus"
                            }
                            _ => "In Bearbeitung durch das Büro",
                        }
                        .into(),
                    ]
                })
            })
            .collect();
        b.push(Briefblock::Abschnitt {
            titel: "Nachforderungen der Behörde".into(),
        });
        if offen.is_empty() {
            b.push(Briefblock::Absatz {
                text: "Zurzeit sind keine Nachforderungen offen.".into(),
            });
        } else {
            b.push(Briefblock::Tabelle {
                koepfe: vec![
                    "Verfahren".into(),
                    "Nachforderung".into(),
                    "Frist".into(),
                    "Stand".into(),
                ],
                zeilen: offen,
            });
        }

        let mut naechste: Vec<_> = termine
            .iter()
            .filter(|t| t.art.is_some() && t.datum >= heute)
            .collect();
        naechste.sort_by_key(|t| t.datum);
        naechste.truncate(12);
        b.push(Briefblock::Abschnitt {
            titel: "Nächste Fristen und Termine".into(),
        });
        if naechste.is_empty() {
            b.push(Briefblock::Absatz {
                text: "Zurzeit laufen keine Fristen.".into(),
            });
        } else {
            b.push(Briefblock::Tabelle {
                koepfe: vec!["Datum".into(), "Termin".into(), "Verfahren".into()],
                zeilen: naechste
                    .iter()
                    .map(|t| vec![datum(t.datum), t.titel.clone(), t.bezug.clone()])
                    .collect(),
            });
        }
        b.push(Briefblock::Absatz {
            text: "Für Rückfragen stehen wir Ihnen gern zur Verfügung.".into(),
        });
        b.push(Briefblock::Hinweis {
            text: "Dieser Bericht gibt den Stand der Verfahrensakte wieder. Fristen der Behörde können sich durch Nachforderungen oder Beteiligungen ändern.".into(),
        });
        Ok(Brief {
            absender,
            empfaenger,
            datum: heute,
            betreff: format!("Sachstandsbericht {}", projekt.name),
            aktenzeichen: String::new(),
            bloecke: b,
        })
    }
    pub async fn sachstandsbericht_pdf(&self, p: ProjectId, wer: &Benutzer) -> CoreResult<Vec<u8>> {
        let brief = self.sachstandsbericht(p, wer).await?;
        self.brief_pdf(p, &brief).await
    }
}
