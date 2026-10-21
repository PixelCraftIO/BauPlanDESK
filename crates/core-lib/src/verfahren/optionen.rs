//! Handlungsoptionen in der jeweiligen Verfahrenslage: offene Anhörung,
//! Ablehnung, ausbleibende Entscheidung, erteilte Genehmigung mit Nachbarn.
//!
//! Die Optionen sind Hinweise mit Frist und Fundstelle; welche gewählt wird,
//! entscheidet der Mensch. Ein Folgeverfahren (Änderungsantrag, neuer
//! Antrag, Widerspruch) übernimmt Behörde, Abweichungen und den Bezug auf
//! die Ablehnungsgründe.
use super::*;
use crate::domain::Bundesland;

/// Was eine Option in der Oberfläche anbietet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Optionsaktion {
    /// Ein Schreiben aus dem Register „Schreiben“.
    Schreiben { schreiben: String },
    /// Folgeverfahren dieser Art anlegen.
    Folgeverfahren { verfahrensart: Verfahrensart },
    /// Register der Akte öffnen.
    Register { register: String },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Handlungsoption {
    pub titel: String,
    pub beschreibung: String,
    pub frist: Option<NaiveDate>,
    pub rechtsgrundlage: String,
    pub aktion: Option<Optionsaktion>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Verfahrenslage {
    /// Kurze Beschreibung der Lage, z. B. „Anhörung offen“.
    pub lage: String,
    pub optionen: Vec<Handlungsoption>,
}
fn option(
    titel: &str,
    beschreibung: impl Into<String>,
    frist: Option<NaiveDate>,
    rechtsgrundlage: &str,
    aktion: Option<Optionsaktion>,
) -> Handlungsoption {
    Handlungsoption {
        titel: titel.into(),
        beschreibung: beschreibung.into(),
        frist,
        rechtsgrundlage: rechtsgrundlage.into(),
        aktion,
    }
}
fn schreiben(s: &str) -> Option<Optionsaktion> {
    Some(Optionsaktion::Schreiben {
        schreiben: s.into(),
    })
}
fn register(r: &str) -> Option<Optionsaktion> {
    Some(Optionsaktion::Register { register: r.into() })
}
/// Genehmigungsfiktion nach Landesrecht: voraussichtlicher Eintritt aus
/// Vollständigkeit, Prüfverfahren und Fassung am Eingangstag.
pub fn fiktion(
    a: &Verfahrensakte,
    land: Bundesland,
    paket: &landesrecht::Regelpaket,
) -> Option<(NaiveDate, String)> {
    let v = a.vollstaendigkeit.as_ref()?;
    let eingang = a
        .abgaben
        .iter()
        .flat_map(|b| &b.versand)
        .map(|e| e.datum)
        .chain(a.externe_einreichungen.iter().map(|e| e.datum))
        .min()?;
    let f = paket.fassung(land, eingang)?.fiktion.as_ref()?;
    let ende = match a.pruefverfahren? {
        Pruefverfahren::Vereinfacht => {
            v.datum + chrono::Duration::weeks(i64::from(f.vereinfacht_wochen))
        }
        Pruefverfahren::Regulaer => v
            .datum
            .checked_add_months(chrono::Months::new(f.regulaer_monate))?,
    };
    Some((ende, format!("{}. {}", f.rechtsgrundlage, f.hinweis)))
}
/// Optionen aus der Lage der Akte am Stichtag.
pub fn optionen(
    a: &Verfahrensakte,
    land: Bundesland,
    heute: NaiveDate,
    paket: &landesrecht::Regelpaket,
) -> Verfahrenslage {
    let mut o = Vec::new();
    let mut lage = Vec::new();
    let nrw = land == Bundesland::Nw;
    // Offene Anhörung.
    for s in a
        .schritte
        .iter()
        .filter(|s| s.art == Schrittart::Anhoerung && s.stellungnahme.is_none())
    {
        if a.status.abgeschlossen() {
            continue;
        }
        let frist = s.frist.as_ref().map(|f| f.datum);
        lage.push("Anhörung offen".to_string());
        o.push(option(
            "Stellungnahme abgeben",
            "Zu jeder Beanstandung Stellung nehmen (Register Behördliche Schritte); Schreiben oder XBau-Stellungnahme 0204 erzeugen.",
            frist,
            "§ 28 VwVfG",
            schreiben("stellungnahme"),
        ));
        o.push(option(
            "Planung ändern und nachreichen",
            "Beanstandete Unterlagen überarbeiten und als Nachreichung zum Antrag einreichen; geänderte Nachweise neu erstellen.",
            frist,
            "",
            register("abgaben"),
        ));
        o.push(option(
            "Abweichung oder Befreiung beantragen",
            "Wenn die Beanstandung eine Unterschreitung betrifft, die bestehen bleiben soll: begründeten Antrag stellen.",
            frist,
            if nrw { "§ 69 BauO NRW; § 31 BauGB" } else { "§ 31 BauGB; Landesbauordnung" },
            register("abweichungen"),
        ));
        o.push(option(
            "Fristverlängerung beantragen",
            "Vor Ablauf der Frist mit Begründung beantragen.",
            frist,
            "",
            schreiben("fristverlaengerung"),
        ));
        o.push(option(
            "Antrag zurücknehmen",
            "Eine Ablehnung vermeiden; Gebührenfolgen der Rücknahme prüfen.",
            None,
            "",
            schreiben("ruecknahme"),
        ));
    }
    // Ablehnung.
    let fristen = fristen::alle(land, &a.art, &a.bescheide);
    for b in termine::wirksame_bescheide(a).filter(|b| b.entscheidung == Entscheidung::Abgelehnt) {
        lage.push(format!("Abgelehnt am {}", b.datum.format("%d.%m.%Y")));
        let rb = fristen
            .iter()
            .find(|f| f.bescheid == b.id)
            .and_then(|f| f.rechtsbehelfsfrist.as_ref())
            .map(|f| f.ende);
        let gruende = a
            .ablehnungsgruende
            .iter()
            .filter(|g| g.bescheid == b.id)
            .count();
        o.push(option(
            "Rechtsbehelf prüfen",
            format!(
                "Klage oder Widerspruch nach der Rechtsbehelfsbelehrung des Bescheids{}. Ohne ordnungsgemäße Belehrung gilt ein Jahr (§ 58 Abs. 2 VwGO).",
                if rb.is_none() { "; Zustelldatum erfassen, um die Frist zu berechnen" } else { "" }
            ),
            rb,
            "§§ 70, 74 VwGO",
            Some(Optionsaktion::Folgeverfahren {
                verfahrensart: Verfahrensart::Widerspruch,
            }),
        ));
        o.push(option(
            "Geänderten Antrag stellen",
            format!(
                "Folgeverfahren mit Bezug auf die Ablehnung anlegen; {} Ablehnungsgründe mit Reaktion erfassen und im neuen Antrag ausräumen.",
                gruende
            ),
            None,
            "",
            Some(Optionsaktion::Folgeverfahren {
                verfahrensart: Verfahrensart::Aenderungsantrag,
            }),
        ));
        o.push(option(
            "Vorbescheid zu den strittigen Fragen",
            "Strittige planungsrechtliche oder bauordnungsrechtliche Fragen vorab verbindlich klären lassen.",
            None,
            if nrw { "§ 77 BauO NRW (prüfen)" } else { "Landesbauordnung (Vorbescheid)" },
            Some(Optionsaktion::Folgeverfahren {
                verfahrensart: Verfahrensart::Vorbescheid,
            }),
        ));
        if gruende == 0 {
            o.push(option(
                "Ablehnungsgründe erfassen",
                "Die Gründe des Bescheids einzeln erfassen, damit der Folgeantrag sie nachweislich ausräumt.",
                None,
                "",
                register("bescheide"),
            ));
        }
    }
    // Ausbleibende Entscheidung.
    let eingereicht = a
        .abgaben
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
        .min();
    if let Some(ein) = eingereicht
        && !a.status.abgeschlossen()
        && !matches!(a.status, Verfahrensstatus::Unvollstaendig)
        && !regeln::entscheidungsbescheid(a)
    {
        let drei_monate = ein.checked_add_months(chrono::Months::new(3));
        let bearbeitung = a.bearbeitungsfrist.as_ref().map(|f| f.datum);
        if let Some((ende, grundlage)) = fiktion(a, land, paket) {
            lage.push("Entscheidung ausstehend".into());
            o.push(option(
                "Genehmigungsfiktion",
                format!(
                    "Ohne Entscheidung bis zum {} gilt die Genehmigung voraussichtlich als erteilt. Danach eine Bescheinigung über den Eintritt verlangen (§ 42a Abs. 3 VwVfG).",
                    ende.format("%d.%m.%Y")
                ),
                Some(ende),
                &grundlage,
                None,
            ));
        }
        if bearbeitung.is_some_and(|d| d < heute) || drei_monate.is_some_and(|d| d <= heute) {
            lage.push("Bearbeitung dauert an".into());
            o.push(option(
                "Sachstand erfragen",
                "Stand der Bearbeitung und voraussichtlichen Entscheidungszeitpunkt erfragen.",
                None,
                "",
                schreiben("sachstandsanfrage"),
            ));
            if drei_monate.is_some_and(|d| d <= heute) {
                o.push(option(
                    "Untätigkeitsklage prüfen",
                    "Ohne zureichenden Grund nicht entschieden: Klage ist frühestens drei Monate nach Antragstellung zulässig.",
                    drei_monate,
                    "§ 75 VwGO",
                    None,
                ));
            }
        }
    }
    // Genehmigt: Nachbarn und Baubeginn.
    if let Some(b) = termine::wirksame_bescheide(a).find(|b| {
        matches!(
            b.entscheidung,
            Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
        ) && !b.teilbaugenehmigung
            && b.verlaengerung.is_none()
    }) {
        lage.push("Genehmigt".into());
        if !a.nachbarbekanntgaben.iter().any(|n| n.bescheid == b.id) {
            o.push(option(
                "Bekanntgabe an Nachbarn",
                "Die Frist für Rechtsbehelfe von Nachbarn läuft erst ab Bekanntgabe an sie; ohne Bekanntgabe bleibt die Genehmigung länger anfechtbar. Bekanntgabe veranlassen oder erfassen.",
                None,
                "§§ 41, 43 VwVfG; § 70 VwGO",
                register("bescheide"),
            ));
        }
        for n in a
            .nachbarbekanntgaben
            .iter()
            .filter(|n| n.bescheid == b.id && n.angefochten.is_some())
        {
            o.push(option(
                "Anfechtung durch Nachbarn",
                format!(
                    "{} hat die Genehmigung angefochten. Baubeginn trotz Rechtsbehelf prüfen (keine aufschiebende Wirkung, § 212a BauGB; Antrag nach § 80a VwGO möglich).",
                    n.name
                ),
                None,
                "§ 212a BauGB; § 80a VwGO",
                None,
            ));
        }
        if a.ausfuehrung.is_none() {
            o.push(option(
                "Baubeginn vorbereiten",
                "Voraussetzungen des Baubeginns prüfen und die Baubeginnsanzeige erstellen.",
                None,
                if nrw {
                    "§ 74 BauO NRW"
                } else {
                    "Landesbauordnung"
                },
                register("stamm"),
            ));
        }
    }
    Verfahrenslage {
        lage: if lage.is_empty() {
            a.status.bezeichnung().to_string()
        } else {
            lage.join(" · ")
        },
        optionen: o,
    }
}
/// Neues Verfahren mit Bezug auf eine Akte.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Folgeverfahrenswunsch {
    pub id: Uuid,
    pub art: Verfahrensart,
    pub bezeichnung: String,
}
impl CoreServices {
    pub async fn verfahren_optionen(&self, p: ProjectId, vid: Uuid) -> CoreResult<Verfahrenslage> {
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let land = self.get_project(p).await?.bundesland;
        Ok(optionen(
            &a,
            land,
            crate::kalender::heute(),
            &landesrecht::Regelpaket::mitgeliefert()?,
        ))
    }
    /// Legt ein Folgeverfahren an: Behörde, Aktenzeichen-Bezug (Vorgänger),
    /// noch beantragte Abweichungen und Vorhabenprofil werden übernommen;
    /// Ablehnungsgründe des Vorgängers verweisen auf das neue Verfahren.
    pub async fn verfahren_folgeverfahren(
        &self,
        p: ProjectId,
        vorgaenger: Uuid,
        w: Folgeverfahrenswunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        crate::plattform::text(&w.bezeichnung, 500)?;
        let (stand, mut alt) = self.verfahrensakte(p, vorgaenger).await?;
        if self
            .repos
            .commit
            .fachstaende(p, "verfahren")
            .await?
            .iter()
            .any(|f| f.objekt == w.id.to_string())
        {
            return Err(fehler("Vorgangskennung bereits vergeben"));
        }
        let neu = Verfahrensakte {
            portal: None,
            bezeichnung: w.bezeichnung.trim().into(),
            art: w.art.clone(),
            antragsangaben: Default::default(),
            status: Verfahrensstatus::Vorbereitung,
            vollstaendigkeit: None,
            ruecknahme: None,
            ausfuehrung: None,
            behoerde: alt.behoerde.clone(),
            aktenzeichen: String::new(),
            verantwortlich: alt.verantwortlich,
            vorgaenger: Some(vorgaenger),
            pruefverfahren: alt.pruefverfahren,
            bearbeitungsfrist: None,
            profil: alt
                .profil
                .clone()
                .filter(|p| p.vorhaben.art == w.art)
                .map(|mut p| {
                    p.bestaetigt_von = None;
                    p.bestaetigt_am = None;
                    p
                }),
            kommunikation: vec![],
            nachforderungen: vec![],
            schritte: vec![],
            anforderungen: vec![],
            abweichungen: alt
                .abweichungen
                .iter()
                .filter(|e| e.aktiv())
                .map(|e| abweichungen::Abweichung {
                    id: Uuid::new_v5(&w.id, e.id.as_bytes()),
                    nachbarn: e
                        .nachbarn
                        .iter()
                        .map(|n| abweichungen::Nachbarzustimmung {
                            id: Uuid::new_v5(&w.id, n.id.as_bytes()),
                            ..n.clone()
                        })
                        .collect(),
                    ..e.clone()
                })
                .collect(),
            dokumentbeziehungen: vec![],
            abgaben: vec![],
            externe_einreichungen: vec![],
            bescheide: vec![],
            gebuehren: vec![],
            unterschriften: vec![],
            ablehnungsgruende: vec![],
            nachbarbekanntgaben: vec![],
            bautechnische_nachweise: vec![],
            bauvorbereitung: None,
            ereignisse: vec![],
        };
        let f = self.verfahren_commit(p, w.id, 0, neu, wer, false).await?;
        // Gründe der Ablehnung zeigen auf das Verfahren, das sie ausräumt.
        let offen = alt
            .ablehnungsgruende
            .iter()
            .any(|g| g.folgeverfahren.is_none());
        if offen && w.art != Verfahrensart::Widerspruch {
            for g in alt
                .ablehnungsgruende
                .iter_mut()
                .filter(|g| g.folgeverfahren.is_none())
            {
                g.folgeverfahren = Some(w.id);
            }
            self.verfahren_commit(p, vorgaenger, stand.revision, alt, wer, false)
                .await?;
        }
        Ok(f)
    }
}
/// Hinweise im Abgabestand eines Folgeverfahrens: Ablehnungsgründe des
/// Vorgängers ohne Reaktion.
pub(super) async fn vorgaenger_hinweise(
    core: &CoreServices,
    p: ProjectId,
    a: &Verfahrensakte,
) -> CoreResult<Vec<String>> {
    let Some(v) = a.vorgaenger else {
        return Ok(vec![]);
    };
    let (_, alt) = core.verfahrensakte(p, v).await?;
    Ok(alt
        .ablehnungsgruende
        .iter()
        .filter(|g| g.reaktion.trim().is_empty())
        .map(|g| {
            format!(
                "Ablehnungsgrund des Vorgängerverfahrens ohne Reaktion: „{}“. Darlegen, wie der Antrag ihn ausräumt.",
                g.text
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn akte() -> Verfahrensakte {
        serde_json::from_value(serde_json::json!({
            "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "eingereicht",
            "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
            "aktenzeichen": "BA-1", "verantwortlich": null, "vorgaenger": null, "profil": null,
            "kommunikation": [], "nachforderungen": [], "anforderungen": [],
            "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": [],
            "externe_einreichungen": [{"id": Uuid::new_v4(), "datum": "2026-09-10", "weg": "post",
                "beleg": null, "bemerkung": "", "dokumente": [], "berichtigt": null,
                "erfasst_am": "2026-09-10T08:00:00Z", "erfasst_von": Uuid::new_v4()}]
        }))
        .unwrap()
    }
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    #[test]
    fn untaetigkeit_und_fiktion_nach_fassung() {
        let paket = landesrecht::Regelpaket::mitgeliefert().unwrap();
        let mut a = akte();
        let l = optionen(&a, Bundesland::Nw, tag("2026-10-01"), &paket);
        assert!(l.optionen.is_empty(), "{l:?}");
        let l = optionen(&a, Bundesland::Nw, tag("2026-12-15"), &paket);
        assert!(l.optionen.iter().any(|o| o.rechtsgrundlage == "§ 75 VwGO"));
        assert!(l.optionen.iter().any(|o| o.titel == "Sachstand erfragen"));
        a.vollstaendigkeit = Some(Vollstaendigkeit {
            datum: tag("2026-09-20"),
            kommunikation: Uuid::new_v4(),
        });
        a.pruefverfahren = Some(Pruefverfahren::Vereinfacht);
        let l = optionen(&a, Bundesland::Nw, tag("2026-10-01"), &paket);
        let f = l
            .optionen
            .iter()
            .find(|o| o.titel == "Genehmigungsfiktion")
            .unwrap();
        assert_eq!(
            f.frist,
            Some(tag("2026-11-01")),
            "6 Wochen ab Vollständigkeit"
        );
        // Vor dem 01.09.2026 eingegangen: keine Fiktion nach neuer Fassung.
        a.externe_einreichungen[0].datum = tag("2026-08-20");
        let l = optionen(&a, Bundesland::Nw, tag("2026-10-01"), &paket);
        assert!(!l.optionen.iter().any(|o| o.titel == "Genehmigungsfiktion"));
    }
    #[test]
    fn anhoerung_bietet_stellungnahme_mit_frist() {
        let paket = landesrecht::Regelpaket::mitgeliefert().unwrap();
        let mut a = akte();
        a.schritte.push(Verfahrensschritt {
            id: Uuid::new_v4(),
            art: Schrittart::Anhoerung,
            kommunikation: Uuid::new_v4(),
            frist: Some(Frist {
                datum: tag("2026-10-20"),
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
        let l = optionen(&a, Bundesland::Nw, tag("2026-10-01"), &paket);
        assert!(l.lage.contains("Anhörung offen"));
        let s = &l.optionen[0];
        assert_eq!(s.titel, "Stellungnahme abgeben");
        assert_eq!(s.frist, Some(tag("2026-10-20")));
        assert!(
            l.optionen
                .iter()
                .any(|o| o.rechtsgrundlage.contains("§ 69 BauO NRW"))
        );
    }
}
