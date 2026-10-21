//! Weitere Inhalte der Antragsnachrichten neben Bauherr, Entwurfsverfasser und
//! Gebührenpflichtigem: weitere Beteiligte, Prozessmerkmale, Gebäudeklasse und
//! Flächen nach BauNVO.
//!
//! [`Vorhabenangaben`] ist der Vertrag zwischen den Liefermodulen (Kunden und
//! Projektbeteiligte, Verfahrensakte, Nutzungsberechnung) und dem XBau-Export.
//! Die Abgabe stellt die Angaben aus dem eingefrorenen Abgabestand zusammen;
//! der Export kennt weder Kundendatenbank noch Berechnungsschritte.

use super::Beteiligter;
use super::codeliste::{Codeliste, Codelisten, ERREICHBARKEIT};
use super::xml::XmlSchreiber;
use crate::domain::Project;
use crate::festsetzungen::{NutzungsPruefung, Quelle};

/// Rolle eines weiteren Beteiligten im Antrag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeitereRolle {
    /// Erster Bevollmächtigter: XBau `bevollmaechtigter`; weitere als
    /// `weitereBeteiligte` mit Rolle 01.
    Bevollmaechtigter,
    /// Eigentümer eines Vorhabenflurstücks (Rolle 02).
    Grundstueckseigentuemer,
    /// Erbbauberechtigter; die XBau-Codeliste kennt die Rolle nicht, sie geht
    /// als nicht gelisteter Wert.
    Erbbauberechtigter,
    /// Fachplaner (Rolle 06). Die Fachrichtung ist nicht erfasst.
    Fachplaner,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeitererBeteiligter {
    pub rolle: WeitereRolle,
    pub beteiligter: Beteiligter,
}

/// Flächen und Maßzahlen nach BauNVO (XBau `FlaechenBauNVO`) eines einzigen
/// Baugrundstücks. Fehlende Werte bleiben `None` und werden nicht geschrieben;
/// `None` bedeutet nie „0“.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlaechenBauNvo {
    pub anzahl_vollgeschosse: Option<u32>,
    /// Im Bebauungsplan festgesetzt.
    pub zulaessige_vollgeschosse: Option<u32>,
    /// Maßgebende Fläche des Baugrundstücks nach § 19 Abs. 3 BauNVO, m².
    pub flaeche_baugrundstueck_m2: Option<f64>,
    pub grundflaeche_m2: Option<f64>,
    /// Festsetzung des Bebauungsplans.
    pub grz_soll: Option<f64>,
    pub grz_geplant: Option<f64>,
    pub geschossflaeche_m2: Option<f64>,
    pub gfz_soll: Option<f64>,
    pub gfz_geplant: Option<f64>,
    pub baumasse_m3: Option<f64>,
    pub bmz_soll: Option<f64>,
    pub bmz_geplant: Option<f64>,
}

impl FlaechenBauNvo {
    /// Übernimmt die Ergebnisse einer aktuellen Nutzungsprüfung. Ist-Werte
    /// stammen aus den Nachweisen, die fehlende Bauteile oder Geschosse
    /// bereits als „nicht ermittelt“ führen; Soll-Werte nur, wenn sie ein
    /// Bebauungsplan festsetzt (Orientierungs- und Umgebungswerte sind keine
    /// Festsetzung).
    pub fn aus_pruefung(pruefung: &NutzungsPruefung, flaeche_m2: f64) -> Option<Self> {
        Self::aus_pruefung_mit_hinweis(pruefung, flaeche_m2).0
    }

    /// Wie [`Self::aus_pruefung`], mit Hinweis, wenn eine Angabe nicht
    /// eindeutig übermittelt werden kann.
    ///
    /// Zahl der Vollgeschosse bei mehreren Gebäuden (je Gebäude ein Nachweis):
    /// XBau kennt nur eine Angabe je Bauvorhaben. Übermittelt wird die größte
    /// Zahl der Vollgeschosse der Gebäude (wie
    /// [`crate::flaechen::FlaechenErgebnis::vollgeschosse`]) und die für alle
    /// Gebäude gemeinsame Festsetzung. Fehlt die Zahl für einzelne Gebäude
    /// oder weichen die Festsetzungen ab, entfällt die Angabe mit Hinweis.
    pub fn aus_pruefung_mit_hinweis(
        pruefung: &NutzungsPruefung,
        flaeche_m2: f64,
    ) -> (Option<Self>, Option<String>) {
        let nachweis = |art: &str| {
            let mut n = pruefung.nachweise.iter().filter(|n| n.art == art);
            match (n.next(), n.next()) {
                (Some(a), None) => Some(a),
                _ => None,
            }
        };
        let ist = |art: &str| nachweis(art).and_then(|n| n.ist).filter(|v| v.is_finite());
        let soll_von = |n: &crate::festsetzungen::Nachweis| {
            Some(n)
                .filter(|n| n.quelle == Quelle::Festsetzung)
                .and_then(|n| n.soll)
                .filter(|v| v.is_finite())
        };
        let soll = |art: &str| nachweis(art).and_then(soll_von);
        let ganz = |v: Option<f64>| {
            v.filter(|v| *v >= 1.0 && v.fract() == 0.0 && *v <= f64::from(u32::MAX))
                .map(|v| v as u32)
        };
        let (vollgeschosse, vollgeschosse_soll, hinweis) = vollgeschosse(pruefung, soll_von);
        let baumasse = pruefung
            .aufstellung
            .as_ref()
            .and_then(|a| a.mass.baumasse_m3)
            .filter(|_| ist("bmz").is_some());
        let f = Self {
            anzahl_vollgeschosse: ganz(vollgeschosse),
            zulaessige_vollgeschosse: ganz(vollgeschosse_soll),
            flaeche_baugrundstueck_m2: Some(flaeche_m2).filter(|v| v.is_finite() && *v > 0.0),
            grundflaeche_m2: ist("grundflaeche"),
            grz_soll: soll("grz"),
            grz_geplant: ist("grz"),
            geschossflaeche_m2: ist("geschossflaeche"),
            gfz_soll: soll("gfz"),
            gfz_geplant: ist("gfz"),
            baumasse_m3: baumasse.filter(|v| v.is_finite()),
            bmz_soll: soll("bmz"),
            bmz_geplant: ist("bmz"),
        };
        ((f != Self::default()).then_some(f), hinweis)
    }

    /// Flächen aus dem aktuellen Berechnungsstand des Projekts. Liefert einen
    /// Hinweis, wenn die Flächen nicht übermittelt werden können (veraltete
    /// Ermittlung, mehrere Baugrundstücke, Berechnungsfehler). Ohne
    /// Berechnung gibt es weder Flächen noch Hinweis.
    pub fn aus_projekt(p: &Project) -> (Option<Self>, Option<String>) {
        let nicht = |grund: String| {
            (
                None,
                Some(format!(
                    "Flächen nach BauNVO nicht in der XBau-Nachricht: {grund}"
                )),
            )
        };
        // Ohne jede Berechnung gibt es nichts zu übermitteln und nichts zu melden.
        if p.planungsrecht.berechnung.is_none()
            && matches!(
                crate::grundstuecksinformationen::ermittlung(&p.planungsrecht),
                Ok(None)
            )
        {
            return (None, None);
        }
        let grundstuecke = match crate::grundstueckskontext::ermitteln(p) {
            Ok(g) => g,
            Err(e) => return nicht(e.to_string()),
        };
        let g = match grundstuecke.as_slice() {
            [] => return (None, None),
            [g] => g,
            mehrere => {
                return nicht(format!(
                    "Das Vorhaben hat {} Baugrundstücke, XBau sieht nur eine Flächenangabe je Bauvorhaben vor. Flächennachweise je Baugrundstück als Anlage beifügen.",
                    mehrere.len()
                ));
            }
        };
        match crate::grundstueckskontext::nutzung(p, g) {
            Ok(Some((pruefung, eingabe, _))) => {
                let flaeche = pruefung
                    .aufstellung
                    .as_ref()
                    .map(|a| a.mass.grundstuecksflaeche_m2)
                    .unwrap_or(eingabe.mass.grundstuecksflaeche_m2);
                Self::aus_pruefung_mit_hinweis(&pruefung, flaeche)
            }
            Ok(None) => (None, None),
            Err(e) => nicht(format!("Berechnung nicht verwendbar: {e}")),
        }
    }

    pub(super) fn schreiben(&self, x: &mut XmlSchreiber) {
        let zahl = |v: f64| format!("{v:.2}");
        let opt = |x: &mut XmlSchreiber, name: &str, v: Option<f64>| {
            if let Some(v) = v {
                x.text(name, &zahl(v));
            }
        };
        x.auf("xbau:bauvorhabenDatenblatt", &[])
            .auf("xbau:flaechen", &[])
            .auf("xbau:flaechen.BauNVO", &[]);
        if let Some(n) = self.anzahl_vollgeschosse {
            x.text("xbau:anzahlVollgeschosse", &n.to_string());
        }
        if let Some(n) = self.zulaessige_vollgeschosse {
            x.text("xbau:zulaessigeAnzahlVollgeschosse", &n.to_string());
        }
        opt(
            x,
            "xbau:flaecheDesBaugrundstuecks",
            self.flaeche_baugrundstueck_m2,
        );
        for (gruppe, werte) in [
            (
                "xbau:grundflaechen",
                [
                    ("xbau:grundflaeche", self.grundflaeche_m2),
                    ("xbau:grundflaechenzahlSoll", self.grz_soll),
                    ("xbau:grundflaechenzahlGeplant", self.grz_geplant),
                ],
            ),
            (
                "xbau:geschossflaechen",
                [
                    ("xbau:geschossflaeche", self.geschossflaeche_m2),
                    ("xbau:geschossflaechenzahlSoll", self.gfz_soll),
                    ("xbau:geschossflaechenzahlGeplant", self.gfz_geplant),
                ],
            ),
            (
                "xbau:baumassen",
                [
                    ("xbau:baumasse", self.baumasse_m3),
                    ("xbau:baumassenzahlSoll", self.bmz_soll),
                    ("xbau:baumassenzahlGeplant", self.bmz_geplant),
                ],
            ),
        ] {
            if werte.iter().any(|(_, v)| v.is_some()) {
                x.auf(gruppe, &[]);
                for (name, v) in werte {
                    opt(x, name, v);
                }
                x.zu();
            }
        }
        x.zu().zu().zu();
    }
}

/// Angaben zum Vorhaben, die nicht im Bauantrag stehen, sondern aus Projekt
/// und Verfahrensakte kommen. Alle Angaben sind optional; leer = Nachricht
/// wie bisher.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vorhabenangaben {
    /// Bevollmächtigte, Eigentümer, Erbbauberechtigte und Fachplaner.
    pub beteiligte: Vec<WeitererBeteiligter>,
    /// Gebäudeklasse 1 bis 5 nach MBO, wenn im Vorhabenprofil erfasst.
    pub gebaeudeklasse: Option<u32>,
    /// Aktenzeichen der erteilten Baugenehmigung, zu der ein Änderungsantrag
    /// einen Nachtrag beantragt (`prozessmerkmale/nachtragsbaugenehmigung`).
    pub hauptgenehmigung: Option<String>,
    pub flaechen: Option<FlaechenBauNvo>,
}

/// Fehlende Angaben und unzulässige Zeichen der weiteren Beteiligten.
pub(super) fn fehlende(v: &Vorhabenangaben, fehlt: &mut Vec<String>) {
    for b in &v.beteiligte {
        let wer = format!("{} {}", rollenname(b.rolle), anzeigename(&b.beteiligter));
        b.beteiligter.namen_pruefen(wer.trim(), fehlt);
    }
    if let Some(g) = v.gebaeudeklasse
        && !(1..=5).contains(&g)
    {
        fehlt.push(format!("Gebäudeklasse {g}: zulässig sind 1 bis 5"));
    }
    if let Some(az) = &v.hauptgenehmigung {
        fehlt.extend(super::din91379::meldung(
            "Aktenzeichen der Baugenehmigung",
            az,
        ));
    }
}

fn rollenname(r: WeitereRolle) -> &'static str {
    match r {
        WeitereRolle::Bevollmaechtigter => "Bevollmächtigter",
        WeitereRolle::Grundstueckseigentuemer => "Grundstückseigentümer",
        WeitereRolle::Erbbauberechtigter => "Erbbauberechtigter",
        WeitereRolle::Fachplaner => "Fachplaner",
    }
}

fn anzeigename(b: &Beteiligter) -> String {
    match b {
        Beteiligter::Person {
            vorname,
            familienname,
            ..
        } => format!("{} {}", vorname.trim(), familienname.trim()),
        Beteiligter::Organisation { name, .. } => name.trim().to_string(),
    }
    .trim()
    .to_string()
}

/// `weitereBeteiligte` (alle außer dem ersten Bevollmächtigten).
pub(super) fn weitere_beteiligte(
    x: &mut XmlSchreiber,
    v: &Vorhabenangaben,
    kanaele: Option<&Codeliste>,
) {
    let rollen = [
        ("listURI", "urn:xoev-de:xbau:codeliste:beteiligterrolle"),
        ("listVersionID", "2.0"),
    ];
    let fachrichtungen = [
        (
            "listURI",
            "urn:xoev-de:xbau:codeliste:beteiligterfachrichtung",
        ),
        ("listVersionID", "3.0"),
    ];
    let erster = v
        .beteiligte
        .iter()
        .position(|b| b.rolle == WeitereRolle::Bevollmaechtigter);
    for (i, b) in v.beteiligte.iter().enumerate() {
        if Some(i) == erster {
            continue;
        }
        x.auf("xbau:weitereBeteiligte", &[])
            .auf("xbau:beteiligter", &[]);
        super::antrag::akteur(x, &b.beteiligter, kanaele);
        x.zu().auf("xbau:rolle", &[]);
        match b.rolle {
            WeitereRolle::Bevollmaechtigter => {
                x.auf("xbau:code", &rollen).text("code", "01").zu();
            }
            WeitereRolle::Grundstueckseigentuemer => {
                x.auf("xbau:code", &rollen).text("code", "02").zu();
            }
            WeitereRolle::Fachplaner => {
                x.auf("xbau:code", &rollen).text("code", "06").zu();
            }
            WeitereRolle::Erbbauberechtigter => {
                x.text("xbau:nichtGelisteterWert", "Erbbauberechtigter");
            }
        }
        x.zu().auf("xbau:fachrichtung", &[]);
        match b.rolle {
            // Die Fachrichtung des Fachplaners ist nicht erfasst.
            WeitereRolle::Fachplaner => {
                x.text("xbau:nichtGelisteterWert", "nicht angegeben");
            }
            _ => {
                x.auf("xbau:code", &fachrichtungen).text("code", "99").zu();
            }
        }
        x.zu().zu();
    }
}

/// Der erste Bevollmächtigte; XBau sieht genau einen `bevollmaechtigter` vor.
pub(super) fn bevollmaechtigter(v: &Vorhabenangaben) -> Option<&WeitererBeteiligter> {
    v.beteiligte
        .iter()
        .find(|b| b.rolle == WeitereRolle::Bevollmaechtigter)
}

/// `Kommunikation` eines Akteurs. Mit geladener Codeliste Erreichbarkeit
/// wird der Kanal codiert, sonst als Zusatz benannt.
pub(super) fn kommunikation(
    x: &mut XmlSchreiber,
    k: &super::Kommunikation,
    kanaele: Option<&Codeliste>,
) {
    for (kennung, merkmal, zusatz) in [
        (k.telefon.trim(), "telefon", "Telefon"),
        (k.email.trim(), "mail", "E-Mail"),
    ] {
        if kennung.is_empty() {
            continue;
        }
        x.auf("kommunikation", &[]);
        match kanaele.and_then(|l| kanal(l, merkmal).map(|c| (l, c))) {
            Some((l, code)) => {
                x.auf("kanal", &[("listVersionID", l.version.as_str())])
                    .text("code", code)
                    .zu()
                    .text("kennung", kennung);
            }
            None => {
                x.text("kennung", kennung).text("zusatz", zusatz);
            }
        }
        x.zu();
    }
}

/// Code des Kanals in der Codeliste Erreichbarkeit, erkannt an der
/// Beschreibung (Festnetz-Telefon bzw. E-Mail, nicht Mobil oder De-Mail).
fn kanal<'a>(l: &'a Codeliste, merkmal: &str) -> Option<&'a str> {
    l.eintraege
        .iter()
        .find(|e| {
            let b = e.beschreibung.to_lowercase();
            b.contains(merkmal) && !b.contains("mobil") && !b.contains("de-mail")
        })
        .map(|e| e.code.as_str())
}

/// Codeliste Erreichbarkeit, wenn geladen.
pub(super) fn kanaele(c: &Codelisten) -> Option<&Codeliste> {
    c.liste(ERREICHBARKEIT)
}

/// Zahl der Vollgeschosse (Ist, Festsetzung) des Bauvorhabens aus den
/// Nachweisen „vollgeschosse“ (einer je Gebäude bei mehreren Gebäuden), mit
/// Hinweis, wenn sie nicht eindeutig ist.
fn vollgeschosse(
    pruefung: &NutzungsPruefung,
    soll_von: impl Fn(&crate::festsetzungen::Nachweis) -> Option<f64>,
) -> (Option<f64>, Option<f64>, Option<String>) {
    let n: Vec<_> = pruefung
        .nachweise
        .iter()
        .filter(|n| n.art == "vollgeschosse")
        .collect();
    match n.as_slice() {
        [] => return (None, None, None),
        [a] => return (a.ist.filter(|v| v.is_finite()), soll_von(a), None),
        _ => {}
    }
    let mut offen = Vec::new();
    let ist: Vec<_> = n
        .iter()
        .filter_map(|n| n.ist.filter(|v| v.is_finite()))
        .collect();
    let ist = if ist.is_empty() {
        // Für kein Gebäude ermittelt: wie bei einem Gebäude ohne Angabe.
        None
    } else if ist.len() < n.len() {
        offen.push("anzahlVollgeschosse (nicht für alle Gebäude ermittelt)");
        None
    } else {
        ist.into_iter().reduce(f64::max)
    };
    let soll: Vec<_> = n.iter().map(|n| soll_von(n)).collect();
    let soll = match soll.first().copied().flatten() {
        Some(s) if soll.iter().all(|v| *v == Some(s)) => Some(s),
        None if soll.iter().all(Option::is_none) => None,
        _ => {
            offen.push("zulaessigeAnzahlVollgeschosse (je Gebäude unterschiedlich festgesetzt)");
            None
        }
    };
    let hinweis = (!offen.is_empty()).then(|| {
        format!(
            "Zahl der Vollgeschosse nicht in der XBau-Nachricht: {}. Die Angaben je Gebäude enthält der Nachweis der Zahl der Vollgeschosse.",
            offen.join("; ")
        )
    });
    (ist, soll, hinweis)
}
