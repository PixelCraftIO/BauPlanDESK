//! Übernahme ausgewerteter Behördennachrichten (siehe [`crate::xbau::eingang`])
//! in die Verfahrensakte: Eingangsbestätigung als Eingang des Abgabestands,
//! Befundliste als Nachforderung oder Vollständigkeit, Anhörung als
//! Verfahrensschritt, Bescheid mit Nebenbestimmungen und Entscheidungen über
//! Abweichungen, Gebührenbescheid mit Zahlungsfrist.
//!
//! Die Übernahme ist ausdrücklich und wiederholbar: Kennungen werden aus der
//! Nachricht abgeleitet, eine zweite Übernahme ändert nichts. Vorher zeigt
//! [`Nachrichtenauswertung::wirkung`], was in der Akte entsteht.
use super::*;
use crate::xbau::eingang::{Auswertung, auswerten, nachricht_xml};
use crate::xbau::postfach::{Richtung, XbauNachricht, kommunikation_id};

/// Ergebnis der Auswertung einer Nachricht für eine Akte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachrichtenauswertung {
    pub nachricht: Uuid,
    pub element: String,
    pub titel: String,
    pub auswertung: Auswertung,
    /// Was die Übernahme in der gewählten Akte bewirkt; leer ohne Akte.
    pub wirkung: Vec<String>,
    /// Bescheid: vorgeschlagene genehmigte Unterlagen aus den versendeten
    /// Abgabeständen des Antrags.
    pub genehmigte_unterlagen: Vec<Dokumentbezug>,
    /// Bereits fachlich in die gewählte Akte übernommen.
    pub uebernommen: bool,
}
/// Bestätigte Übernahme einer ausgewerteten Nachricht.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Uebernahmewunsch {
    pub verfahren: Uuid,
    #[ts(type = "number")]
    pub basis: i64,
    /// Bestätigtes Zugangsdatum; ohne Angabe der Tag der Erfassung im Journal.
    pub zugang: Option<NaiveDate>,
    /// Bescheid: ausdrücklich genehmigte Unterlagen.
    #[serde(default)]
    pub genehmigte_unterlagen: Vec<Dokumentbezug>,
}
/// Gebührenbescheid der Behörde mit Zahlungsfrist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Gebuehrenbescheid {
    pub id: Uuid,
    /// Eingegangenes Schreiben dieser Akte.
    pub kommunikation: Uuid,
    pub datum: NaiveDate,
    /// Betrag in Eurocent, wenn bekannt.
    #[ts(type = "number | null")]
    pub betrag_cent: Option<i64>,
    pub vorauszahlung: bool,
    #[serde(default)]
    pub verwendungszweck: String,
    pub frist: Option<Frist>,
    /// Tag der Zahlung; beendet die Wiedervorlage.
    pub bezahlt: Option<NaiveDate>,
    pub verantwortlich: Option<Uuid>,
}
fn id(nachricht: Uuid, was: &str) -> Uuid {
    Uuid::new_v5(&nachricht, format!("bauprojektio-xbau-{was}").as_bytes())
}
fn kurz(t: &str, max: usize) -> String {
    let erste = t
        .lines()
        .map(str::trim)
        .find(|z| !z.is_empty())
        .unwrap_or("");
    let mut s: String = erste.chars().take(max).collect();
    if erste.chars().count() > max {
        s.pop();
        s.push('…');
    }
    s
}
fn lang(t: &str, max: usize) -> String {
    t.chars().take(max).collect()
}
/// Abgabestand, auf den sich die Nachricht bezieht: Bezugsnachricht oder
/// Antragsreferenz (Vorgang des Bauantrags), jüngster zuerst.
fn bezugsabgabe<'a>(
    a: &'a Verfahrensakte,
    kopf: &crate::xbau::austausch::Nachrichtenkopf,
) -> Option<&'a abgabe::Abgabestand> {
    a.abgaben
        .iter()
        .rev()
        .find(|b| Some(b.id) == kopf.bezugsnachricht)
        .or_else(|| {
            a.abgaben
                .iter()
                .rev()
                .filter(|b| b.artefakte.iter().any(|f| f.format == "xbau"))
                .find(|b| Some(b.grundlagen.antrag.vorgang) == kopf.antragsreferenz)
        })
}
/// Unterlagen der versendeten Abgabestände: die XBau-Abgaben desselben
/// Antrags (Antragsreferenz), sonst alle versendeten; je Dokument die
/// jüngste Version.
fn vorgelegte_unterlagen(
    a: &Verfahrensakte,
    kopf: &crate::xbau::austausch::Nachrichtenkopf,
) -> Vec<Dokumentbezug> {
    let versendet: Vec<_> = a
        .abgaben
        .iter()
        .filter(|b| {
            matches!(
                abgabe::versandstand(b),
                Some(abgabe::Versandstatus::Versendet | abgabe::Versandstatus::Eingegangen)
            )
        })
        .collect();
    let zum_antrag: Vec<_> = versendet
        .iter()
        .copied()
        .filter(|b| {
            b.artefakte.iter().any(|f| f.format == "xbau")
                && Some(b.grundlagen.antrag.vorgang) == kopf.antragsreferenz
        })
        .collect();
    let quelle = if zum_antrag.is_empty() {
        versendet
    } else {
        zum_antrag
    };
    let mut r: Vec<Dokumentbezug> = Vec::new();
    for d in quelle.iter().flat_map(|b| &b.dokumente) {
        match r
            .iter_mut()
            .find(|x| x.dokument_id == d.dokument.dokument_id)
        {
            Some(x) if x.version < d.dokument.version => *x = d.dokument.clone(),
            Some(_) => {}
            None => r.push(d.dokument.clone()),
        }
    }
    r
}
/// Wendet die Auswertung auf die Akte an. Ohne Änderung (bereits übernommen
/// oder nichts zu übernehmen) bleibt die Akte gleich.
#[allow(clippy::too_many_arguments)]
pub(crate) fn anwenden(
    a: &mut Verfahrensakte,
    nid: Uuid,
    n: &XbauNachricht,
    aus: &Auswertung,
    kommunikation: Uuid,
    zugang: NaiveDate,
    land: crate::domain::Bundesland,
    genehmigte: &[Dokumentbezug],
) -> CoreResult<Vec<String>> {
    let k = &n.pruefung.kopf;
    let mut w = Vec::new();
    match aus {
        Auswertung::Eingangsbestaetigung { .. } => {
            let Some(b) = bezugsabgabe(a, k).map(|b| b.id) else {
                w.push("Kein Abgabestand dieser Akte passt zur Antragsreferenz; nur als Kommunikation erfasst.".into());
                return Ok(w);
            };
            let b = a.abgaben.iter_mut().find(|x| x.id == b).expect("gefunden");
            let eid = id(nid, "eingang");
            if b.versand.iter().any(|e| e.id == eid) {
                return Ok(w);
            }
            if abgabe::versandstand(b) == Some(abgabe::Versandstatus::Eingegangen) {
                w.push(format!("Eingang von „{}“ ist bereits bestätigt.", b.titel));
                return Ok(w);
            }
            if zugang < crate::kalender::datum_deutsch(b.erstellt_am) {
                return Err(fehler(
                    "Die Eingangsbestätigung liegt vor dem Festhalten des Abgabestands",
                ));
            }
            b.versand.push(abgabe::Versandereignis {
                id: eid,
                status: abgabe::Versandstatus::Eingegangen,
                datum: zugang,
                weg: "XBau".into(),
                referenz: nid.to_string(),
                notiz: "Eingangsbestätigung der Behörde (XBau 1120)".into(),
                beleg: Some(n.original.clone()),
                berichtigt: None,
            });
            w.push(format!(
                "Eingang von „{}“ am {} mit der Nachricht als Beleg.",
                b.titel,
                zugang.format("%d.%m.%Y")
            ));
            if a.status == Verfahrensstatus::Vorbereitung {
                a.status = Verfahrensstatus::Eingereicht;
                w.push("Bearbeitungsstand „Eingereicht“.".into());
            }
        }
        Auswertung::Aktenzeichen { aktenzeichen } => {
            let az = aktenzeichen.trim();
            if !az.is_empty() && a.aktenzeichen.trim().is_empty() {
                a.aktenzeichen = az.into();
                w.push(format!("Aktenzeichen {az} in die Akte übernommen."));
            } else if !az.is_empty() && a.aktenzeichen.trim() != az {
                w.push(format!(
                    "Aktenzeichen der Nachricht ({az}) weicht von der Akte ({}) ab; maßgebend bleibt die Akte.",
                    a.aktenzeichen.trim()
                ));
            }
        }
        Auswertung::Befundliste {
            vollstaendig,
            befunde,
            frist,
            ..
        } => {
            if *vollstaendig {
                if a.vollstaendigkeit.is_none() {
                    a.vollstaendigkeit = Some(Vollstaendigkeit {
                        datum: zugang,
                        kommunikation,
                    });
                    w.push(format!(
                        "Vollständigkeit am {} bestätigt.",
                        zugang.format("%d.%m.%Y")
                    ));
                }
                if matches!(
                    a.status,
                    Verfahrensstatus::Eingereicht | Verfahrensstatus::Unvollstaendig
                ) && !terminiert_offen(a)
                {
                    a.status = Verfahrensstatus::Vollstaendig;
                    w.push("Bearbeitungsstand „Vollständig“.".into());
                }
                return Ok(w);
            }
            let nfid = id(nid, "nachforderung");
            if befunde.is_empty() || a.nachforderungen.iter().any(|x| x.id == nfid) {
                return Ok(w);
            }
            let grund = match land {
                crate::domain::Bundesland::Nw if a.art.baugenehmigung() => {
                    "Frist der Behörde zur Behebung der Mängel. Werden sie nicht fristgerecht behoben, gilt der Bauantrag als zurückgenommen (§ 71 BauO NRW).".to_string()
                }
                _ => "Frist der Behörde zur Behebung der Mängel. Folgen einer Fristversäumnis im Schreiben und in der Landesbauordnung prüfen.".to_string(),
            };
            let punkte = befunde
                .iter()
                .enumerate()
                .map(|(i, b)| Forderungspunkt {
                    id: id(nid, &format!("befund-{i}")),
                    titel: match kurz(&b.text, 500) {
                        t if t.is_empty() => format!("Befund {}", i + 1),
                        t => t,
                    },
                    grundlage: lang(
                        &format!(
                            "Formelle Prüfung, Befundart {}:\n{}",
                            if b.art.is_empty() { "–" } else { &b.art },
                            b.text
                        ),
                        4000,
                    ),
                    verantwortlich: a.verantwortlich,
                    frist: frist.map(|d| Frist {
                        datum: d,
                        ausloeser: format!(
                            "Befundliste der Behörde, zugegangen am {}",
                            zugang.format("%d.%m.%Y")
                        ),
                        beleg: Some(n.original.clone()),
                        erinnerung_tage: 7,
                        grund: grund.clone(),
                    }),
                    status: Punktstatus::Offen,
                    antwort: String::new(),
                    dokumente: vec![],
                    abgaben: vec![],
                    bestaetigung: None,
                })
                .collect::<Vec<_>>();
            w.push(format!(
                "Nachforderung mit {} Punkt{}{} angelegt.",
                punkte.len(),
                if punkte.len() == 1 { "" } else { "en" },
                frist
                    .map(|d| format!(", Frist {}", d.format("%d.%m.%Y")))
                    .unwrap_or_default()
            ));
            a.nachforderungen.push(Nachforderung {
                id: nfid,
                kommunikation,
                punkte,
            });
            if matches!(
                a.status,
                Verfahrensstatus::Eingereicht
                    | Verfahrensstatus::Vollstaendig
                    | Verfahrensstatus::InPruefung
            ) {
                a.status = Verfahrensstatus::Unvollstaendig;
                w.push("Bearbeitungsstand „Unvollständig“.".into());
            }
        }
        Auswertung::Anhoerung { befunde, frist, .. } => {
            let sid = id(nid, "anhoerung");
            if a.schritte.iter().any(|s| s.id == sid) {
                return Ok(w);
            }
            a.schritte.push(Verfahrensschritt {
                id: sid,
                art: Schrittart::Anhoerung,
                kommunikation,
                frist: frist.map(|d| Frist {
                    datum: d,
                    ausloeser: format!(
                        "Anhörung der Behörde, zugegangen am {}",
                        zugang.format("%d.%m.%Y")
                    ),
                    beleg: Some(n.original.clone()),
                    erinnerung_tage: 7,
                    grund: "Frist zur Stellungnahme (§ 28 VwVfG)".into(),
                }),
                stellungnahme: None,
                verantwortlich: a.verantwortlich,
                folgeschreiben: None,
                gruende: befunde
                    .iter()
                    .enumerate()
                    .map(|(i, b)| Beanstandung {
                        id: id(nid, &format!("grund-{i}")),
                        text: lang(
                            &if b.regelverstoss.trim().is_empty() {
                                b.begruendung.clone()
                            } else {
                                b.regelverstoss.clone()
                            },
                            4000,
                        ),
                        grundlage: lang(&b.begruendung, 4000),
                        reaktion: String::new(),
                        abweichung: None,
                    })
                    .collect(),
            });
            w.push(format!(
                "Anhörung{}{} erfasst.",
                frist
                    .map(|d| format!(" mit Frist zur Stellungnahme bis {}", d.format("%d.%m.%Y")))
                    .unwrap_or_default(),
                if befunde.is_empty() {
                    String::new()
                } else {
                    format!(" und {} Beanstandungen", befunde.len())
                }
            ));
        }
        Auswertung::Bescheid {
            genehmigung,
            nebenbestimmungen,
            abweichungen,
            ..
        } => {
            let bid = id(nid, "bescheid");
            if a.bescheide.iter().any(|b| b.id == bid) {
                return Ok(w);
            }
            let teil = k.element.ends_with(".0235");
            if *genehmigung && genehmigte.is_empty() {
                return Err(fehler(
                    "Für die Genehmigung die ausdrücklich genehmigten Unterlagen auswählen",
                ));
            }
            let auflagen = nebenbestimmungen
                .iter()
                .enumerate()
                .filter(|(_, n)| !n.text.trim().is_empty())
                .map(|(i, nb)| Auflage {
                    id: id(nid, &format!("nebenbestimmung-{i}")),
                    // Ein ablehnender Bescheid trägt nur Hinweise.
                    art: if *genehmigung {
                        nb.art
                    } else {
                        Nebenbestimmungsart::Hinweis
                    },
                    titel: kurz(&nb.text, 500),
                    ausloeser: lang(&nb.text, 4000),
                    verantwortlich: None,
                    frist: None,
                    erledigt: false,
                    nachweise: vec![],
                    faelligkeit: None,
                })
                .collect::<Vec<_>>();
            let entscheidungen = abweichungen
                .iter()
                .filter(|x| a.abweichungen.iter().any(|e| e.id == x.id))
                .map(|x| abweichungen::Abweichungsentscheidung {
                    abweichung: x.id,
                    zugelassen: x.zugelassen,
                    begruendung: lang(&x.begruendung, 16000),
                    nebenbestimmung: lang(&x.nebenbestimmung, 16000),
                })
                .collect::<Vec<_>>();
            let fremd = abweichungen.len() - entscheidungen.len();
            w.push(format!(
                "{} vom {} mit {} Nebenbestimmung{}{} erfasst.",
                if *genehmigung {
                    if teil {
                        "Teilbaugenehmigung"
                    } else {
                        "Genehmigung"
                    }
                } else {
                    "Ablehnung"
                },
                crate::kalender::datum_deutsch(k.erstellt).format("%d.%m.%Y"),
                auflagen.len(),
                if auflagen.len() == 1 { "" } else { "en" },
                if entscheidungen.is_empty() {
                    String::new()
                } else {
                    format!(
                        " und {} Entscheidungen über Abweichungen",
                        entscheidungen.len()
                    )
                }
            ));
            if fremd > 0 {
                w.push(format!(
                    "{fremd} Abweichungsentscheidungen beziehen sich auf keine Abweichung dieser Akte; im Bescheid nachlesen."
                ));
            }
            a.bescheide.push(Bescheid {
                id: bid,
                entscheidung: if *genehmigung {
                    Entscheidung::Genehmigt
                } else {
                    Entscheidung::Abgelehnt
                },
                datum: crate::kalender::datum_deutsch(k.erstellt),
                zugestellt: Some(zugang),
                original: n.original.clone(),
                genehmigte_unterlagen: if *genehmigung {
                    genehmigte.to_vec()
                } else {
                    vec![]
                },
                auflagen,
                ersetzt: None,
                verlaengerung: None,
                teilbaugenehmigung: teil,
                abweichungsentscheidungen: entscheidungen,
            });
            if !teil && !a.status.abgeschlossen() {
                a.status = Verfahrensstatus::Entschieden;
                w.push("Bearbeitungsstand „Entschieden“.".into());
            }
        }
        Auswertung::Gebuehrenbescheid {
            vorauszahlung,
            summe_cent,
            verwendungszweck,
            frist,
            ..
        } => {
            let gid = id(nid, "gebuehr");
            if a.gebuehren.iter().any(|g| g.id == gid) {
                return Ok(w);
            }
            a.gebuehren.push(Gebuehrenbescheid {
                id: gid,
                kommunikation,
                datum: zugang,
                betrag_cent: *summe_cent,
                vorauszahlung: *vorauszahlung,
                verwendungszweck: lang(verwendungszweck, 500),
                frist: frist.map(|d| Frist {
                    datum: d,
                    ausloeser: "Gebührenbescheid der Behörde".into(),
                    beleg: Some(n.original.clone()),
                    erinnerung_tage: 7,
                    grund: "Zahlungsfrist laut Gebührenbescheid".into(),
                }),
                bezahlt: None,
                verantwortlich: a.verantwortlich,
            });
            w.push(format!(
                "{}{}{} erfasst.",
                if *vorauszahlung {
                    "Vorauszahlungsbescheid"
                } else {
                    "Gebührenbescheid"
                },
                summe_cent
                    .map(|c| format!(
                        " über {} €",
                        crate::formular::vorlage::zahl(c as f64 / 100.0, 2)
                    ))
                    .unwrap_or_default(),
                frist
                    .map(|d| format!(", zahlbar bis {}", d.format("%d.%m.%Y")))
                    .unwrap_or_default()
            ));
        }
        Auswertung::Sonstige => {}
    }
    Ok(w)
}
/// Offene Forderungspunkte, die einer Vollständigkeit entgegenstehen.
fn terminiert_offen(a: &Verfahrensakte) -> bool {
    termine::offene_forderungspunkte(a).next().is_some()
}
impl CoreServices {
    async fn eingang_lesen(
        &self,
        p: ProjectId,
        nachricht: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<(XbauNachricht, Auswertung)> {
        let n = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == nachricht)
            .ok_or_else(|| fehler("Nachricht nicht gefunden"))?
            .nachricht;
        let (_, daten) = self
            .load_version(n.original.dokument_id, n.original.version)
            .await?;
        let xml = nachricht_xml(&daten)?;
        let aus = if n.richtung == Richtung::Eingang {
            auswerten(&n.pruefung.kopf, &xml)?
        } else {
            Auswertung::Sonstige
        };
        Ok((n, aus))
    }
    /// Liest den fachlichen Inhalt einer Nachricht und zeigt, was die
    /// Übernahme in der Akte bewirkt.
    pub async fn xbau_auswertung(
        &self,
        p: ProjectId,
        nachricht: Uuid,
        verfahren: Option<Uuid>,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenauswertung> {
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Lesen)
            .await?;
        let (n, aus) = self.eingang_lesen(p, nachricht, wer).await?;
        let mut r = Nachrichtenauswertung {
            nachricht,
            element: n.pruefung.kopf.element.clone(),
            titel: n.pruefung.kopf.titel.clone(),
            auswertung: aus.clone(),
            wirkung: vec![],
            genehmigte_unterlagen: vec![],
            uebernommen: false,
        };
        if let Some(v) = verfahren {
            let (_, a) = self.verfahrensakte(p, v).await?;
            let land = self.get_project(p).await?.bundesland;
            r.genehmigte_unterlagen = vorgelegte_unterlagen(&a, &n.pruefung.kopf);
            let mut probe = a.clone();
            // Ist die Nachricht schon als Kommunikation erfasst, gilt deren Zugang.
            let zugang = a
                .kommunikation
                .iter()
                .find(|c| c.id == kommunikation_id(nachricht))
                .map(|c| c.datum)
                .unwrap_or_else(|| crate::kalender::datum_deutsch(n.erfasst_am));
            r.wirkung = match anwenden(
                &mut probe,
                nachricht,
                &n,
                &aus,
                kommunikation_id(nachricht),
                zugang,
                land,
                &r.genehmigte_unterlagen,
            ) {
                Ok(w) => w,
                Err(e) => vec![e.to_string()],
            };
            r.uebernommen = probe == a
                && a.kommunikation
                    .iter()
                    .any(|c| c.id == kommunikation_id(nachricht));
            if !a
                .kommunikation
                .iter()
                .any(|c| c.id == kommunikation_id(nachricht))
            {
                r.wirkung
                    .insert(0, "Nachricht als Kommunikation erfassen.".into());
            }
        }
        Ok(r)
    }
    /// Übernimmt eine Behördennachricht fachlich in die Akte. Ist sie noch
    /// nicht als Kommunikation erfasst, geschieht das zuerst.
    pub async fn xbau_auswertung_uebernehmen(
        &self,
        p: ProjectId,
        nachricht: Uuid,
        w: Uebernahmewunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (n, aus) = self.eingang_lesen(p, nachricht, wer).await?;
        if n.richtung != Richtung::Eingang {
            return Err(fehler(
                "Nur eingehende Nachrichten der Behörde werden fachlich übernommen",
            ));
        }
        let (stand, akte) = self.verfahrensakte(p, w.verfahren).await?;
        if stand.revision != w.basis {
            return Err(CoreError::VersionConflict {
                expected: w.basis,
                actual: stand.revision,
            });
        }
        let c = kommunikation_id(nachricht);
        let land = self.get_project(p).await?.bundesland;
        // Erst vollständig prüfen, dann schreiben: Ein Fehler darf keine
        // halbe Übernahme (nur Kommunikation) hinterlassen.
        anwenden(
            &mut akte.clone(),
            nachricht,
            &n,
            &aus,
            c,
            akte.kommunikation
                .iter()
                .find(|k| k.id == c)
                .map(|k| k.datum)
                .or(w.zugang)
                .unwrap_or_else(|| crate::kalender::datum_deutsch(n.erfasst_am)),
            land,
            &w.genehmigte_unterlagen,
        )?;
        let basis = if akte.kommunikation.iter().any(|k| k.id == c) {
            stand.revision
        } else {
            self.xbau_in_verfahren(p, nachricht, w.verfahren, w.basis, w.zugang, wer)
                .await?
                .revision
        };
        let (stand, mut akte) = self.verfahrensakte(p, w.verfahren).await?;
        let zugang = akte
            .kommunikation
            .iter()
            .find(|k| k.id == c)
            .map(|k| k.datum)
            .ok_or_else(|| fehler("Nachricht ist in einer anderen Akte erfasst"))?;
        let vorher = akte.clone();
        anwenden(
            &mut akte,
            nachricht,
            &n,
            &aus,
            c,
            zugang,
            land,
            &w.genehmigte_unterlagen,
        )?;
        if akte == vorher {
            return Ok(stand);
        }
        self.verfahren_commit(p, w.verfahren, basis, akte, wer, false)
            .await
    }
}
