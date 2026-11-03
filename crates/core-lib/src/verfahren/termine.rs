//! Termine der Verfahrensakte: erfasste Fristen (Forderungspunkte,
//! Nebenbestimmungen, Anhörung, Zurückstellung, Beteiligungen,
//! Bearbeitungsfrist der Behörde) und berechnete Bescheidfristen
//! (Rechtsbehelfsfrist, Geltungsdauer, Fertigstellung). Wiedervorlagen,
//! Projektübersicht und Kalenderexport (ICS nach RFC 5545) lesen dieselbe
//! Ableitung; wann ein Termin endet, entscheidet allein dieses Modul.
use super::{
    Auflage, Bescheid, Entscheidung, Erinnerung, Forderungspunkt, Punktstatus, Schrittart,
    Verfahrensakte, Verfahrensstatus, fehler, fristen,
};
use crate::{CoreResult, CoreServices, domain::Bundesland, domain::ProjectId};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use uuid::Uuid;

/// Art eines Termins; sie bestimmt, wann er endet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Terminart {
    /// Frist eines offenen Forderungspunkts; endet mit Bestätigung, Entfallen
    /// oder der Entscheidung über das Verfahren.
    Forderungspunkt,
    /// Frist einer Auflage oder Bedingung; endet mit der Erfüllung.
    Nebenbestimmung,
    /// Ablauf einer Befristung oder eines befristeten Vorbehalts; endet mit
    /// dem Ablaufdatum.
    Ablauf,
    /// Frist zur Stellungnahme; endet mit der Stellungnahme oder Entscheidung.
    Anhoerung,
    /// Ende der Zurückstellung; endet mit dem Fristende, einem Folgeschreiben
    /// oder der Entscheidung.
    Zurueckstellung,
    /// Ende der Rechtsbehelfsfrist; endet mit ihrem Ablauf.
    Rechtsbehelfsfrist,
    /// Ablauf der Geltungsdauer; endet mit erfasstem Baubeginn oder einer
    /// Verlängerung.
    Geltungsdauer,
    /// Frist zur Fertigstellung; endet mit erfasster Fertigstellung.
    Fertigstellung,
    /// Frist einer Beteiligung (Einvernehmen, Fachbehörden, Nachbarn); endet
    /// mit ihrem Ablauf oder der Entscheidung.
    Beteiligung,
    /// Bearbeitungsfrist oder Genehmigungsfiktion laut Behörde; endet mit der
    /// Entscheidung und wird danach überfällig.
    Bearbeitungsfrist,
    /// Zahlungsfrist eines Gebührenbescheids; endet mit der erfassten Zahlung.
    Zahlung,
}
impl Terminart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Forderungspunkt => "Forderungspunkt",
            Self::Nebenbestimmung => "Nebenbestimmung",
            Self::Ablauf => "Ablauf einer Befristung",
            Self::Anhoerung => "Anhörung",
            Self::Zurueckstellung => "Zurückstellung",
            Self::Rechtsbehelfsfrist => "Rechtsbehelfsfrist",
            Self::Geltungsdauer => "Geltungsdauer",
            Self::Fertigstellung => "Fertigstellungsfrist",
            Self::Beteiligung => "Beteiligung",
            Self::Bearbeitungsfrist => "Bearbeitungsfrist",
            Self::Zahlung => "Zahlungsfrist",
        }
    }
}

/// Vorlauf berechneter Fristen in Tagen.
pub const VORLAUF_RECHTSBEHELF: u32 = 14;
pub const VORLAUF_GELTUNGSDAUER: u32 = 90;

/// Offener Termin einer Verfahrensakte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Termin {
    pub verfahren: Uuid,
    pub verfahrensbezeichnung: String,
    /// Akteneintrag, zu dem der Termin gehört (Punkt, Nebenbestimmung, Schritt
    /// oder Bescheid).
    pub objekt: Uuid,
    pub art: Terminart,
    pub titel: String,
    pub datum: NaiveDate,
    /// Erinnerung so viele Tage vor dem Datum.
    pub vorlauf_tage: u32,
    pub verantwortlich: Option<Uuid>,
    /// Der Termin endet mit seinem Datum und wird danach nicht überfällig.
    pub endet_mit_ablauf: bool,
    /// Kurze Erläuterung, z. B. die Rechtsgrundlage einer berechneten Frist.
    pub hinweis: String,
}
impl Termin {
    pub fn faellig_ab(&self) -> NaiveDate {
        self.datum
            .checked_sub_signed(Duration::days(i64::from(self.vorlauf_tage)))
            .unwrap_or(self.datum)
    }
    /// Läuft der Termin am Stichtag noch?
    pub fn aktiv(&self, heute: NaiveDate) -> bool {
        !(self.endet_mit_ablauf && heute > self.datum)
    }
    pub fn ueberfaellig(&self, heute: NaiveDate) -> bool {
        !self.endet_mit_ablauf && self.datum < heute
    }
    /// Wiedervorlage am Stichtag: ab dem Vorlauf, solange der Termin läuft.
    pub fn erinnerung(&self, heute: NaiveDate) -> Option<Erinnerung> {
        (self.aktiv(heute) && heute >= self.faellig_ab()).then(|| Erinnerung {
            verfahren: self.verfahren,
            objekt: self.objekt,
            titel: format!("{} · {}", self.verfahrensbezeichnung, self.titel),
            datum: self.datum,
            ueberfaellig: self.ueberfaellig(heute),
            verantwortlich: self.verantwortlich,
            art: self.art,
        })
    }
}

/// Bescheide, die nicht durch einen Folgebescheid ersetzt sind.
pub fn wirksame_bescheide(a: &Verfahrensakte) -> impl Iterator<Item = &Bescheid> {
    a.bescheide
        .iter()
        .filter(|b| !a.bescheide.iter().any(|n| n.ersetzt == Some(b.id)))
}

/// Forderungspunkte, die noch zu bearbeiten sind. Mit der Entscheidung oder
/// Rücknahme endet die Nachforderung; offene Punkte sind dann gegenstandslos.
pub fn offene_forderungspunkte(a: &Verfahrensakte) -> impl Iterator<Item = &Forderungspunkt> {
    let aktiv = !a.status.abgeschlossen();
    a.nachforderungen
        .iter()
        .flat_map(|n| &n.punkte)
        .filter(move |p| {
            aktiv && !matches!(p.status, Punktstatus::Bestaetigt | Punktstatus::Entfallen)
        })
}

/// Offene Auflagen und Bedingungen wirksamer, nicht ablehnender Bescheide.
/// Sie bleiben nach der Entscheidung bis zur Erfüllung bestehen (Bauausführung)
/// und enden mit der Rücknahme.
pub fn offene_pflichten(a: &Verfahrensakte) -> impl Iterator<Item = (&Bescheid, &Auflage)> {
    let aktiv = a.status != Verfahrensstatus::Zurueckgenommen;
    wirksame_bescheide(a)
        .filter(move |b| aktiv && b.entscheidung != Entscheidung::Abgelehnt)
        .flat_map(|b| b.auflagen.iter().map(move |n| (b, n)))
        .filter(|(_, n)| n.art.pflicht() && !n.erledigt)
}

/// Alle laufenden Termine einer Akte, ohne Stichtagsfilter.
pub fn termine(a: &Verfahrensakte, verfahren: Uuid, land: Bundesland) -> Vec<Termin> {
    let mut r = Vec::new();
    if a.status == Verfahrensstatus::Zurueckgenommen {
        return r;
    }
    let mut neu = |objekt, art, titel: String, datum, vorlauf_tage, verantwortlich, hinweis| {
        r.push(Termin {
            verfahren,
            verfahrensbezeichnung: a.bezeichnung.clone(),
            objekt,
            art,
            titel,
            datum,
            vorlauf_tage,
            verantwortlich,
            endet_mit_ablauf: matches!(
                art,
                Terminart::Ablauf
                    | Terminart::Zurueckstellung
                    | Terminart::Rechtsbehelfsfrist
                    | Terminart::Beteiligung
            ),
            hinweis,
        })
    };
    // NRW: Werden Mängel nicht fristgerecht behoben, gilt der Bauantrag als
    // zurückgenommen (§ 71 BauO NRW); die Wiedervorlage nennt die Folge.
    let ruecknahmefiktion = land == Bundesland::Nw && a.art.baugenehmigung();
    for p in offene_forderungspunkte(a) {
        if let Some(f) = &p.frist {
            let hinweis = if ruecknahmefiktion && !f.grund.contains("§ 71 BauO NRW") {
                format!(
                    "{}{}Fristversäumnis: Der Antrag gilt als zurückgenommen (§ 71 BauO NRW). Rechtzeitig nachreichen oder Fristverlängerung beantragen.",
                    f.grund.trim(),
                    if f.grund.trim().is_empty() {
                        ""
                    } else {
                        " · "
                    }
                )
            } else {
                f.grund.clone()
            };
            neu(
                p.id,
                Terminart::Forderungspunkt,
                p.titel.clone(),
                f.datum,
                f.erinnerung_tage,
                p.verantwortlich,
                hinweis,
            );
        }
    }
    for (_, n) in offene_pflichten(a) {
        if let Some(f) = &n.frist {
            neu(
                n.id,
                Terminart::Nebenbestimmung,
                n.titel.clone(),
                f.datum,
                f.erinnerung_tage,
                n.verantwortlich,
                format!("{}: {}", n.art.bezeichnung(), n.ausloeser),
            );
        }
    }
    // Befristung und Vorbehalte sind keine Pflicht: Mit Frist erinnern sie vor
    // dem Ablauf und enden danach, ohne Frist erinnern sie nicht.
    for b in wirksame_bescheide(a).filter(|b| b.entscheidung != Entscheidung::Abgelehnt) {
        for n in &b.auflagen {
            if n.art.pflicht() || n.art == super::Nebenbestimmungsart::Hinweis || n.erledigt {
                continue;
            }
            if let Some(f) = &n.frist {
                neu(
                    n.id,
                    Terminart::Ablauf,
                    format!("{} läuft ab: {}", n.art.bezeichnung(), n.titel),
                    f.datum,
                    f.erinnerung_tage,
                    n.verantwortlich,
                    n.ausloeser.clone(),
                );
            }
        }
    }
    if !a.status.abgeschlossen() {
        for v in &a.schritte {
            let Some(f) = &v.frist else { continue };
            match v.art {
                Schrittart::Anhoerung if v.stellungnahme.is_none() => neu(
                    v.id,
                    Terminart::Anhoerung,
                    "Stellungnahme zur Anhörung".into(),
                    f.datum,
                    f.erinnerung_tage,
                    v.verantwortlich,
                    f.grund.clone(),
                ),
                Schrittart::Zurueckstellung if v.folgeschreiben.is_none() => neu(
                    v.id,
                    Terminart::Zurueckstellung,
                    "Ende der Zurückstellung".into(),
                    f.datum,
                    f.erinnerung_tage,
                    v.verantwortlich,
                    "Danach Entscheidung der Behörde erwarten".into(),
                ),
                art if art.beteiligung() => neu(
                    v.id,
                    Terminart::Beteiligung,
                    format!("Frist: {}", art.bezeichnung()),
                    f.datum,
                    f.erinnerung_tage,
                    v.verantwortlich,
                    match art {
                        Schrittart::Einvernehmen => {
                            "Ohne Verweigerung bis dahin gilt das Einvernehmen als erteilt \
                             (§ 36 Abs. 2 Satz 2 BauGB)."
                                .into()
                        }
                        _ => f.grund.clone(),
                    },
                ),
                _ => {}
            }
        }
        if let Some(f) = &a.bearbeitungsfrist
            && super::pruefung::ist_eingereicht(a)
        {
            neu(
                verfahren,
                Terminart::Bearbeitungsfrist,
                "Bearbeitungsfrist der Behörde".into(),
                f.datum,
                f.erinnerung_tage,
                a.verantwortlich,
                format!(
                    "{} Danach Sachstand erfragen; eine Genehmigungsfiktion nach der \
                     Landesbauordnung prüfen und bescheinigen lassen.",
                    f.grund
                ),
            );
        }
    }
    // Nachbarn: eigene Rechtsbehelfsfrist ab ihrer Bekanntgabe.
    for n in a
        .nachbarbekanntgaben
        .iter()
        .filter(|n| n.angefochten.is_none())
    {
        if !wirksame_bescheide(a).any(|b| b.id == n.bescheid) {
            continue;
        }
        neu(
            n.id,
            Terminart::Rechtsbehelfsfrist,
            format!("Rechtsbehelfsfrist {} (Bekanntgabe am {})", n.name, n.datum.format("%d.%m.%Y")),
            fristen::rechtsbehelf_ende(land, n.datum),
            VORLAUF_RECHTSBEHELF,
            a.verantwortlich,
            "Ein Monat ab Bekanntgabe an den Nachbarn bei ordnungsgemäßer Rechtsbehelfsbelehrung (§§ 70, 74 VwGO); danach ist die Genehmigung ihm gegenüber bestandskräftig.".into(),
        );
    }
    for g in a.gebuehren.iter().filter(|g| g.bezahlt.is_none()) {
        if let Some(f) = &g.frist {
            neu(
                g.id,
                Terminart::Zahlung,
                format!(
                    "{} bezahlen",
                    if g.vorauszahlung {
                        "Gebührenvorauszahlung"
                    } else {
                        "Gebühren"
                    }
                ),
                f.datum,
                f.erinnerung_tage,
                g.verantwortlich,
                f.grund.clone(),
            );
        }
    }
    let fristen = fristen::alle(land, &a.art, &a.bescheide);
    for b in wirksame_bescheide(a) {
        let Some(f) = fristen.iter().find(|f| f.bescheid == b.id) else {
            continue;
        };
        let vom = b.datum.format("%d.%m.%Y");
        if let Some(r) = &f.rechtsbehelfsfrist {
            neu(
                b.id,
                Terminart::Rechtsbehelfsfrist,
                format!("Ende der Rechtsbehelfsfrist (Bescheid vom {vom})"),
                r.ende,
                VORLAUF_RECHTSBEHELF,
                a.verantwortlich,
                r.grundlage.clone(),
            );
        }
        if let Some(g) = &f.geltungsdauer
            && a.ausfuehrung.is_none()
            && fristen::verlaengert_durch(&a.bescheide, b.id).is_none()
        {
            neu(
                b.id,
                Terminart::Geltungsdauer,
                if a.art == super::Verfahrensart::Vorbescheid {
                    format!(
                        "Vorbescheid gilt bis {} (Bescheid vom {vom}): Bauantrag stellen oder Verlängerung beantragen",
                        g.ende.format("%d.%m.%Y")
                    )
                } else {
                    format!(
                        "Geltungsdauer läuft ab (Bescheid vom {vom}): Baubeginn erfassen oder Verlängerung beantragen"
                    )
                },
                g.ende,
                VORLAUF_GELTUNGSDAUER,
                a.verantwortlich,
                g.grundlage.clone(),
            );
        }
        if let Some(g) = &f.fertigstellung
            && a.ausfuehrung
                .as_ref()
                .is_none_or(|x| x.fertigstellung.is_none())
        {
            neu(
                b.id,
                Terminart::Fertigstellung,
                format!("Fertigstellungsfrist läuft ab (Bescheid vom {vom})"),
                g.ende,
                VORLAUF_GELTUNGSDAUER,
                a.verantwortlich,
                g.grundlage.clone(),
            );
        }
    }
    r
}

/// Ersetzt Sonderzeichen in TEXT-Werten nach RFC 5545, Abschnitt 3.3.11.
fn ics_text(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => r.push_str("\\\\"),
            ';' => r.push_str("\\;"),
            ',' => r.push_str("\\,"),
            '\n' => r.push_str("\\n"),
            '\r' => {}
            c if c.is_control() => {}
            c => r.push(c),
        }
    }
    r
}

/// Faltet eine Inhaltszeile nach 75 Oktetten (RFC 5545, Abschnitt 3.1), ohne
/// ein UTF-8-Zeichen zu teilen, und schließt sie mit CRLF ab.
fn ics_zeile(ziel: &mut String, zeile: &str) {
    let mut laenge = 0;
    for c in zeile.chars() {
        if laenge + c.len_utf8() > 75 {
            ziel.push_str("\r\n ");
            laenge = 1;
        }
        ziel.push(c);
        laenge += c.len_utf8();
    }
    ziel.push_str("\r\n");
}

/// Kalender mit ganztägigen Terminen (RFC 5545). Jeder Termin erhält eine
/// stabile UID, damit ein erneuter Import ihn aktualisiert statt verdoppelt,
/// und eine Erinnerung mit dem Vorlauf.
pub fn ics(kalendername: &str, termine: &[Termin], erzeugt: DateTime<Utc>) -> String {
    let eintraege: Vec<Kalendereintrag> = termine
        .iter()
        .map(|t| {
            let art = serde_json::to_value(t.art)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let mut beschreibung = format!("{}\n{}", t.art.bezeichnung(), t.hinweis);
            if !t.endet_mit_ablauf {
                beschreibung.push_str("\nBleibt bis zur Erledigung in den Wiedervorlagen.");
            }
            Kalendereintrag {
                uid: format!("{}-{art}-{}", t.objekt, t.verfahren),
                datum: t.datum,
                zusammenfassung: format!("{} · {}", t.verfahrensbezeichnung, t.titel),
                beschreibung,
                kategorie: t.art.bezeichnung().into(),
                erinnerung: t.titel.clone(),
                vorlauf_tage: t.vorlauf_tage,
            }
        })
        .collect();
    kalender(kalendername, &eintraege, erzeugt)
}

/// Ganztägiger Kalendereintrag für [`kalender`].
#[derive(Debug, Clone, PartialEq)]
pub struct Kalendereintrag {
    /// Eindeutig und stabil über Abrufe; `@bauprojektio` wird angehängt.
    pub uid: String,
    pub datum: NaiveDate,
    pub zusammenfassung: String,
    pub beschreibung: String,
    pub kategorie: String,
    /// Text der Erinnerung.
    pub erinnerung: String,
    pub vorlauf_tage: u32,
}

/// Kalenderdatei (RFC 5545) aus ganztägigen Einträgen.
pub fn kalender(
    kalendername: &str,
    eintraege: &[Kalendereintrag],
    erzeugt: DateTime<Utc>,
) -> String {
    let mut s = String::new();
    let stempel = erzeugt.format("%Y%m%dT%H%M%SZ").to_string();
    for z in [
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//BauPlanIO//Verfahrenstermine//DE",
        "CALSCALE:GREGORIAN",
        "METHOD:PUBLISH",
    ] {
        ics_zeile(&mut s, z);
    }
    ics_zeile(&mut s, &format!("X-WR-CALNAME:{}", ics_text(kalendername)));
    for t in eintraege {
        ics_zeile(&mut s, "BEGIN:VEVENT");
        ics_zeile(&mut s, &format!("UID:{}@bauprojektio", t.uid));
        ics_zeile(&mut s, &format!("DTSTAMP:{stempel}"));
        ics_zeile(
            &mut s,
            &format!("DTSTART;VALUE=DATE:{}", t.datum.format("%Y%m%d")),
        );
        let ende = t.datum.succ_opt().unwrap_or(t.datum);
        ics_zeile(
            &mut s,
            &format!("DTEND;VALUE=DATE:{}", ende.format("%Y%m%d")),
        );
        ics_zeile(&mut s, &format!("SUMMARY:{}", ics_text(&t.zusammenfassung)));
        ics_zeile(
            &mut s,
            &format!("DESCRIPTION:{}", ics_text(&t.beschreibung)),
        );
        ics_zeile(&mut s, &format!("CATEGORIES:{}", ics_text(&t.kategorie)));
        ics_zeile(&mut s, "TRANSP:TRANSPARENT");
        ics_zeile(&mut s, "BEGIN:VALARM");
        ics_zeile(&mut s, "ACTION:DISPLAY");
        ics_zeile(&mut s, &format!("DESCRIPTION:{}", ics_text(&t.erinnerung)));
        ics_zeile(
            &mut s,
            &if t.vorlauf_tage == 0 {
                "TRIGGER:PT0S".to_string()
            } else {
                format!("TRIGGER:-P{}D", t.vorlauf_tage)
            },
        );
        ics_zeile(&mut s, "END:VALARM");
        ics_zeile(&mut s, "END:VEVENT");
    }
    ics_zeile(&mut s, "END:VCALENDAR");
    s
}

impl CoreServices {
    /// Verfahrensakten eines Projekts mit ihrer Kennung (für Lesemodelle
    /// anderer Module wie das Büro-Cockpit).
    pub(crate) async fn verfahrensakten_lesen(
        &self,
        p: ProjectId,
    ) -> CoreResult<Vec<(Uuid, Verfahrensakte)>> {
        self.verfahrensakten(p).await
    }
    /// Verfahrensakten eines Projekts mit ihrer Kennung.
    async fn verfahrensakten(&self, p: ProjectId) -> CoreResult<Vec<(Uuid, Verfahrensakte)>> {
        self.repos
            .commit
            .fachstaende(p, "verfahren")
            .await?
            .into_iter()
            .map(|f| {
                let a = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
                let id = Uuid::parse_str(&f.objekt).map_err(|e| fehler(e.to_string()))?;
                Ok((id, a))
            })
            .collect()
    }

    /// Laufende Termine aller Verfahrensakten am Stichtag, nach Datum.
    pub async fn verfahren_termine(
        &self,
        p: ProjectId,
        heute: NaiveDate,
    ) -> CoreResult<Vec<Termin>> {
        let land = self.get_project(p).await?.bundesland;
        let mut r: Vec<Termin> = Vec::new();
        for (id, a) in self.verfahrensakten(p).await? {
            r.extend(termine(&a, id, land).into_iter().filter(|t| t.aktiv(heute)));
        }
        r.sort_by(|a, b| a.datum.cmp(&b.datum).then(a.titel.cmp(&b.titel)));
        Ok(r)
    }

    /// Wiedervorlagen am Stichtag: Termine ab ihrem Vorlauf, solange sie laufen.
    pub async fn verfahren_erinnerungen(
        &self,
        p: ProjectId,
        heute: NaiveDate,
    ) -> CoreResult<Vec<Erinnerung>> {
        let mut r: Vec<Erinnerung> = self
            .verfahren_termine(p, heute)
            .await?
            .iter()
            .filter_map(|t| t.erinnerung(heute))
            .collect();
        r.sort_by_key(|e| e.datum);
        Ok(r)
    }

    /// Laufende Termine aller Verfahren als Kalenderdatei (text/calendar).
    pub async fn verfahren_termine_ics(
        &self,
        p: ProjectId,
        heute: NaiveDate,
    ) -> CoreResult<String> {
        let projekt = self.get_project(p).await?;
        let termine = self.verfahren_termine(p, heute).await?;
        let mut name = String::new();
        let _ = write!(name, "{} · Verfahrenstermine", projekt.name);
        Ok(ics(&name, &termine, Utc::now()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn termin(art: Terminart, datum: &str, vorlauf: u32) -> Termin {
        Termin {
            verfahren: Uuid::nil(),
            verfahrensbezeichnung: "Bauantrag".into(),
            objekt: Uuid::from_u128(7),
            art,
            titel: "Frist".into(),
            datum: datum.parse().unwrap(),
            vorlauf_tage: vorlauf,
            verantwortlich: None,
            endet_mit_ablauf: matches!(art, Terminart::Ablauf | Terminart::Rechtsbehelfsfrist),
            hinweis: String::new(),
        }
    }
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn ablaufhinweis_endet_pflicht_wird_ueberfaellig() {
        let ablauf = termin(Terminart::Ablauf, "2026-05-10", 10);
        assert!(ablauf.erinnerung(tag("2026-04-29")).is_none());
        assert!(ablauf.erinnerung(tag("2026-04-30")).is_some());
        let am_tag = ablauf.erinnerung(tag("2026-05-10")).unwrap();
        assert!(!am_tag.ueberfaellig);
        assert!(ablauf.erinnerung(tag("2026-05-11")).is_none());
        let pflicht = termin(Terminart::Nebenbestimmung, "2026-05-10", 0);
        assert!(pflicht.erinnerung(tag("2027-01-01")).unwrap().ueberfaellig);
    }

    #[test]
    fn ics_faltet_maskiert_und_bleibt_ganztaegig() {
        let mut t = termin(Terminart::Geltungsdauer, "2026-12-31", 90);
        t.titel = "Geltungsdauer; Bescheid, Nr. 1 \\ mit sehr langem Titel über Ä, Ö und Ü für die Faltung".into();
        t.hinweis = "Zeile 1\nZeile 2".into();
        let s = ics("Musterhaus", &[t], "2026-10-18T08:00:00Z".parse().unwrap());
        assert!(s.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n"));
        assert!(s.ends_with("END:VCALENDAR\r\n"));
        assert!(s.contains("DTSTART;VALUE=DATE:20261231\r\n"));
        assert!(s.contains("DTEND;VALUE=DATE:20270101\r\n"));
        assert!(s.contains("TRIGGER:-P90D\r\n"));
        assert!(s.contains("DTSTAMP:20261018T080000Z\r\n"));
        // Jede physische Zeile höchstens 75 Oktette, Fortsetzung mit Leerzeichen.
        for z in s.split("\r\n") {
            assert!(z.len() <= 75, "{z}");
        }
        let entfaltet = s.replace("\r\n ", "");
        assert!(entfaltet.contains("Geltungsdauer\\; Bescheid\\, Nr. 1 \\\\ mit"));
        assert!(entfaltet.contains("Zeile 1\\nZeile 2"));
        assert!(entfaltet.contains("UID:00000000-0000-0000-0000-000000000007-geltungsdauer-"));
        assert!(!s.replace("\r\n", "").contains('\n'));
    }
}
