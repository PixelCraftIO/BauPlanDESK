//! Bereichsstatus der Projektnavigation: je Bereich, ob dort ein Problem
//! besteht, etwas veraltet ist, Arbeit offen ist, er noch zu erledigen ist oder
//! fachlich geprüft ist (siehe [`Bereichszustand`]).
//!
//! Reines Lesemodell. Grundlage sind die Arbeitspunkte der
//! [`crate::projektuebersicht`] (Zielbereich und Dringlichkeit), die neuesten
//! Nachweisstände je Bereich (aktuell, vollständig, freigegeben, eingehalten)
//! sowie bereichseigene Prüfungen (Pflichtangaben, Aufmaßaufgaben, Planliste,
//! Gebäude, Raumbuch, Baukennwerte, Verfahrensstand). Grün gibt es nur, wenn
//! ein solches Prüfsignal erfüllt ist: Ein eingetragener Wert allein macht
//! einen Bereich nie „geprüft“. Ungespeicherte Änderungen kennt nur die
//! Oberfläche; sperrende Voraussetzungen bleiben dort wie bisher.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::ProjectId,
    planaufmass::Planaufmass,
    plattform::{
        ObjektArt,
        nachweise::{NachweisAnsicht, NachweisEingabe, NutzungsArt},
        objekte::{Gebaeude, Projektorganisation, Raumbuch},
    },
    projektuebersicht::{Dringlichkeit, Projektuebersicht},
    verfahren::Verfahrensstatus,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Zustand eines Bereichs, nach Dringlichkeit geordnet. Grün („geprüft“) gibt
/// es nur mit einem fachlichen Prüfsignal, nie allein, weil Daten erfasst sind.
/// „Nicht gespeichert“ kennt nur die Oberfläche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Bereichszustand {
    /// Nicht eingehalten, Prüffehler oder überfällig (rot).
    Problem,
    /// Eine Grundlage hat sich geändert; neu berechnen bzw. prüfen (orange).
    Veraltet,
    /// Begonnen, aber Angaben, Prüfung oder Freigabe fehlen (gelb).
    InArbeit,
    /// Benötigt, aber noch nichts erfasst (grau).
    ZuErledigen,
    /// Fachlich vollständig und geprüft (grün).
    Geprueft,
    /// Leer und nicht benötigt, oder Listenbereich ohne offene Einträge (kein Symbol).
    Leer,
    /// Vom Büro ausdrücklich als nicht benötigt gekennzeichnet (kein Symbol).
    NichtBenoetigt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bereichsstatus {
    /// Kennung des Bereichs wie in der Navigation (z. B. `flurstuecke`, `grz`).
    pub bereich: String,
    pub zustand: Bereichszustand,
    /// Kurze Gründe für den Zustand: Probleme, dann Veraltetes, dann Offenes.
    pub gruende: Vec<String>,
    /// Ob sich der Bereich als „nicht benötigt“ kennzeichnen lässt: nur
    /// Datenbereiche, in denen noch nichts erfasst ist.
    pub kennzeichenbar: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bereichsstaende {
    pub projekt_version: i64,
    pub stichtag: NaiveDate,
    pub bereiche: Vec<Bereichsstatus>,
}

/// Wie ein Bereich bewertet wird.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Art {
    /// Immer benötigt: ohne offene Punkte geprüft.
    Pflicht,
    /// Benötigt, sobald Daten vorhanden oder verlangt; geprüft nur mit Prüfsignal.
    Daten,
    /// Liste von Einträgen (Aufgaben, Lieferungen …): nie grün, nur offen,
    /// überfällig oder ohne Symbol.
    Liste,
}

/// Bereiche mit Status in Navigationsreihenfolge. Reine Ansichten
/// (Projektübersicht, Chronik, Messwerte, Druck, Einreichen, Suche,
/// XBau-Nachrichten) bekommen nie ein Symbol und fehlen hier.
const BEREICHE: &[(&str, Art)] = &[
    ("aufgaben", Art::Liste),
    ("projekttagebuch", Art::Liste),
    ("fachplaner", Art::Liste),
    ("uebersicht", Art::Pflicht),
    ("rechtsgrundlagen", Art::Pflicht),
    ("festsetzungen", Art::Daten),
    ("flurstuecke", Art::Pflicht),
    ("dokumente", Art::Liste),
    ("planaufmass", Art::Daten),
    ("planliste", Art::Daten),
    ("unterlagen", Art::Daten),
    ("gebaeude", Art::Daten),
    ("raumbuch", Art::Daten),
    ("grundstueck", Art::Daten),
    ("grz", Art::Daten),
    ("gfz", Art::Daten),
    ("geschosse", Art::Daten),
    ("volumen", Art::Daten),
    ("ueberbaubarkeit", Art::Daten),
    ("wohnflaechen", Art::Daten),
    ("din277", Art::Daten),
    ("vollgeschosse", Art::Daten),
    ("hoehen", Art::Daten),
    ("stellplaetze", Art::Daten),
    ("abstand", Art::Daten),
    ("freiflaechen", Art::Daten),
    ("entwaesserung", Art::Daten),
    ("nachweise", Art::Daten),
    ("formulare", Art::Daten),
    ("baukennwerte", Art::Daten),
    ("verfahren", Art::Daten),
    ("bauantrag", Art::Daten),
    ("portale", Art::Liste),
    ("einreichungen", Art::Liste),
];

/// Bereiche, deren Prüfsignal ein Nachweisstand ist.
pub(crate) const NACHWEISBEREICHE: &[&str] = &[
    "grz",
    "gfz",
    "geschosse",
    "volumen",
    "ueberbaubarkeit",
    "wohnflaechen",
    "din277",
    "vollgeschosse",
    "hoehen",
    "stellplaetze",
    "abstand",
    "freiflaechen",
    "entwaesserung",
];

/// Bereich der Navigation, in dem Daten einer Objektart erfasst werden.
fn bereich_von(art: ObjektArt) -> Option<&'static str> {
    Some(match art {
        ObjektArt::Abstandsflaechen => "abstand",
        ObjektArt::Fachplanerlieferung => "fachplaner",
        ObjektArt::Planaufmass | ObjektArt::Cadaufmass | ObjektArt::Ifcmodell => "planaufmass",
        ObjektArt::Projektorganisation | ObjektArt::Dokumentdaten => return None,
        a => a.modul(),
    })
}
const DATENARTEN: [ObjektArt; 21] = [
    ObjektArt::Stellplaetze,
    ObjektArt::Wohnflaechen,
    ObjektArt::Din277,
    ObjektArt::Vollgeschosse,
    ObjektArt::Hoehen,
    ObjektArt::Freiflaechen,
    ObjektArt::Entwaesserung,
    ObjektArt::Ueberbaubarkeit,
    ObjektArt::Abstandsflaechen,
    ObjektArt::Verfahren,
    ObjektArt::Gebaeude,
    ObjektArt::Aufgaben,
    ObjektArt::Unterlagen,
    ObjektArt::Raumbuch,
    ObjektArt::Planaufmass,
    ObjektArt::Cadaufmass,
    ObjektArt::Ifcmodell,
    ObjektArt::Projekttagebuch,
    ObjektArt::Planliste,
    ObjektArt::Fachplanerlieferung,
    ObjektArt::Baukennwerte,
];

/// Gesammelte Befunde eines Bereichs.
#[derive(Default)]
struct Befund {
    problem: Vec<String>,
    veraltet: Vec<String>,
    offen: Vec<String>,
    /// Ein bereichseigenes Prüfsignal ist erfüllt (nur Datenbereiche).
    geprueft: bool,
    /// Der Bereich wird verlangt, auch wenn noch nichts erfasst ist.
    verlangt: bool,
}
fn dazu(liste: &mut Vec<String>, text: String) {
    if !liste.contains(&text) {
        liste.push(text);
    }
}
impl Befund {
    fn problem(&mut self, t: impl Into<String>) {
        dazu(&mut self.problem, t.into());
    }
    fn veraltet(&mut self, t: impl Into<String>) {
        dazu(&mut self.veraltet, t.into());
    }
    fn offen(&mut self, t: impl Into<String>) {
        dazu(&mut self.offen, t.into());
    }
}

fn bf<'a>(m: &'a mut BTreeMap<String, Befund>, id: &str) -> &'a mut Befund {
    m.entry(id.to_string()).or_default()
}

/// Anzeigename eines Nachweisbereichs für zusammengefasste Gründe.
pub(crate) fn bereichstitel(id: &str) -> &str {
    match id {
        "grz" => "GRZ",
        "gfz" => "GFZ",
        "geschosse" => "Geschossigkeit",
        "volumen" => "BMZ",
        "ueberbaubarkeit" => "Überbaubarkeit",
        "wohnflaechen" => "Wohnflächen",
        "din277" => "DIN 277",
        "vollgeschosse" => "Vollgeschosse",
        "hoehen" => "Höhen",
        "stellplaetze" => "Stellplätze",
        "abstand" => "Abstandsflächen",
        "freiflaechen" => "Freiflächen",
        "entwaesserung" => "Entwässerung",
        x => x,
    }
}

/// Bereich, zu dem ein Nachweisstand gehört.
pub(crate) fn nachweisbereich(e: &NachweisEingabe) -> &'static str {
    match e {
        NachweisEingabe::Fach { plan, .. } => plan.modul.objekt().modul(),
        NachweisEingabe::AbstandProjekt { .. } | NachweisEingabe::Abstand { .. } => "abstand",
        NachweisEingabe::Nutzung { fach, .. } => match fach {
            NutzungsArt::Grz => "grz",
            NutzungsArt::Gfz => "gfz",
            NutzungsArt::Geschossigkeit => "geschosse",
            NutzungsArt::Bmz => "volumen",
        },
    }
}

/// Ob das Ergebnis irgendwo ausdrücklich „nicht eingehalten“ bzw. „nicht
/// erfüllt“ meldet (Felder `eingehalten`/`erfuellt` = `false`).
fn nicht_eingehalten(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(m) => m.iter().any(|(k, w)| {
            ((k == "eingehalten" || k == "erfuellt") && w == &serde_json::Value::Bool(false))
                || nicht_eingehalten(w)
        }),
        serde_json::Value::Array(a) => a.iter().any(nicht_eingehalten),
        _ => false,
    }
}

/// Arbeitspunkte der Übersicht als Befunde je Zielbereich. Punkte zu
/// Nachweisständen bewertet der Bereichsstatus selbst (je Fachbereich).
fn befunde_aus(u: &Projektuebersicht, b: &mut BTreeMap<String, Befund>) {
    for p in &u.punkte {
        if p.id.starts_with("nachweis:") {
            continue;
        }
        let zusatz = match (p.dringlichkeit, p.faellig) {
            (Dringlichkeit::Ueberfaellig, Some(d)) => {
                format!(" (überfällig seit {})", d.format("%d.%m.%Y"))
            }
            (Dringlichkeit::Heute, _) => " (heute fällig)".into(),
            (_, Some(d)) => format!(" (fällig {})", d.format("%d.%m.%Y")),
            _ => String::new(),
        };
        let bereich = match p.ziel.bereich.as_str() {
            // Die Maßansicht zählt zum Baugrundstück.
            "mass" => "grundstueck",
            b => b,
        };
        let f = b.entry(bereich.to_string()).or_default();
        let text = format!("{}{zusatz}", p.titel);
        if p.dringlichkeit == Dringlichkeit::Ueberfaellig {
            f.problem(text);
        } else {
            f.offen(text);
        }
    }
}

impl CoreServices {
    pub async fn bereichsstatus(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Bereichsstaende> {
        self.bereichsstatus_am(p, crate::kalender::heute(), wer)
            .await
    }

    /// Interner Stichtag für deterministische Prüfungen.
    pub async fn bereichsstatus_am(
        &self,
        p: ProjectId,
        heute: NaiveDate,
        wer: &Benutzer,
    ) -> CoreResult<Bereichsstaende> {
        let uebersicht = self.projektuebersicht_am(p, heute, wer).await?;
        let projekt = self.get_project(p).await?;
        let recht = &projekt.planungsrecht;
        let mut befunde: BTreeMap<String, Befund> = BTreeMap::new();
        befunde_aus(&uebersicht, &mut befunde);
        // Je Objektart genügt ein Datensatz; das Projekt ist oben schon gelesen
        // (Zugriff geprüft), daher direkt und ohne erneutes Projektlesen.
        let mut daten: BTreeSet<&str> = BTreeSet::new();
        for art in DATENARTEN {
            let Some(b) = bereich_von(art) else { continue };
            if daten.contains(b) {
                continue;
            }
            if !self
                .repos
                .commit
                .fachseite(p, art.modul(), None, 1)
                .await?
                .is_empty()
            {
                daten.insert(b);
            }
        }
        // Baugrundstück mit GRZ, GFZ, Geschossigkeit und BMZ.
        let ermittlung = crate::grundstuecksinformationen::ermittlung(recht)?;
        if ermittlung
            .as_ref()
            .is_some_and(|e| !e.berechnungen.is_empty())
        {
            daten.extend(["grundstueck", "grz", "gfz", "geschosse", "volumen"]);
        }

        // Stammdaten: Vorhaben und Standort gehören auf jeden Bauantrag.
        if recht.gemeinde.trim().is_empty() || recht.standortzeile().is_empty() {
            bf(&mut befunde, "uebersicht").offen("Gemeinde und Standort des Vorhabens angeben");
        }
        if recht.vorhaben.trim().is_empty() {
            bf(&mut befunde, "uebersicht").offen("Vorhaben beschreiben");
        }
        if [&recht.gemeinde, &recht.standortzeile(), &recht.vorhaben]
            .iter()
            .any(|t| !t.trim().is_empty())
        {
            daten.insert("uebersicht");
        }
        if recht.rechtsgrundlage.is_some() {
            daten.insert("rechtsgrundlagen");
        }
        // Flurstücke sind Grundlage aller grundstücksbezogenen Nachweise.
        let info = crate::grundstuecksinformationen::informationen(recht);
        if info.flurstuecke.is_empty() {
            bf(&mut befunde, "flurstuecke").offen("Mindestens ein Vorhabenflurstück erfassen");
        } else {
            daten.insert("flurstuecke");
        }
        for f in info.flurstuecke.iter().filter(|f| f.flaeche_offen) {
            bf(&mut befunde, "flurstuecke").offen(format!(
                "{}: Fläche wird noch im Aufmaß ermittelt",
                f.bezeichnung()
            ));
        }
        // Mit Bebauungsplan sind dessen Festsetzungen zu erfassen.
        if recht.mit_bebauungsplan() {
            bf(&mut befunde, "festsetzungen").verlangt = true;
            let w = &recht.festgesetzte_werte;
            let mass = [w.grz, w.gfz, w.bmz, w.grundflaeche_m2, w.geschossflaeche_m2]
                .iter()
                .any(Option::is_some)
                || w.vollgeschosse.is_some();
            if mass {
                daten.insert("festsetzungen");
                bf(&mut befunde, "festsetzungen").geprueft = true;
            } else {
                bf(&mut befunde, "festsetzungen").offen(
                    "Bebauungsplan angegeben: Maß der baulichen Nutzung (GRZ, GFZ oder Grundfläche, Vollgeschosse) eintragen",
                );
            }
        }
        // Baugrundstück: ermittelt und ohne offene Punkte der Übersicht.
        if daten.contains("grundstueck") {
            bf(&mut befunde, "grundstueck").geprueft = true;
        }

        // Nachweise: neuester Stand je Produzent und Bezugsumfang.
        let mut neueste: BTreeMap<String, NachweisAnsicht> = BTreeMap::new();
        for n in self.nachweise(p).await? {
            let key = crate::projektuebersicht::nachweisschluessel(&n.stand)?;
            let ersetzen = neueste.get(&key).is_none_or(|alt| {
                (n.stand.erstellt_am, n.stand.id) > (alt.stand.erstellt_am, alt.stand.id)
            });
            if ersetzen {
                neueste.insert(key, n);
            }
        }
        let mut mit_nachweis: BTreeSet<&str> = BTreeSet::new();
        let mut nachweis_ok: BTreeMap<&str, bool> = BTreeMap::new();
        for n in neueste.values() {
            let bereich = nachweisbereich(&n.stand.eingabe);
            mit_nachweis.insert(bereich);
            daten.insert(bereich);
            let name = &n.stand.bezeichnung;
            let mut ok = false;
            if let Some(e) = &n.prueffehler {
                bf(&mut befunde, bereich)
                    .problem(format!("{name}: Prüfung technisch nicht möglich ({e})"));
            } else if !n.aktuell {
                bf(&mut befunde, bereich).veraltet(format!(
                    "{name}: Grundlagen oder Eingaben geändert, Nachweis neu erstellen"
                ));
            } else if nicht_eingehalten(&n.stand.ergebnis) {
                bf(&mut befunde, bereich).problem(format!("{name}: Anforderung nicht eingehalten"));
            } else if !n.stand.vollstaendig {
                bf(&mut befunde, bereich)
                    .offen(format!("{name}: Berechnung enthält offene Prüfpunkte"));
            } else if n.freigabe.is_none() {
                bf(&mut befunde, bereich).offen(format!("{name}: fachliche Bürofreigabe fehlt"));
            } else {
                ok = true;
            }
            let e = nachweis_ok.entry(bereich).or_insert(true);
            *e = *e && ok;
        }
        for bereich in NACHWEISBEREICHE {
            if !daten.contains(bereich) {
                continue;
            }
            if mit_nachweis.contains(bereich) {
                bf(&mut befunde, bereich).geprueft =
                    nachweis_ok.get(bereich).copied().unwrap_or(false);
            } else {
                bf(&mut befunde, bereich)
                    .offen("Noch kein Nachweis erstellt: Berechnung prüfen und Nachweis anlegen");
            }
        }
        // Übersicht „Nachweise“: Zusammenfassung der Fachbereiche.
        let berechnung_begonnen = NACHWEISBEREICHE.iter().any(|x| daten.contains(x));
        if berechnung_begonnen {
            daten.insert("nachweise");
            let mut summe = Befund::default();
            let mut alle_ok = true;
            for x in NACHWEISBEREICHE.iter().filter(|x| daten.contains(*x)) {
                let f = bf(&mut befunde, x);
                let titel = bereichstitel(x);
                for t in &f.problem {
                    summe.problem(format!("{titel}: {t}"));
                }
                for t in &f.veraltet {
                    summe.veraltet(format!("{titel}: {t}"));
                }
                if !f.offen.is_empty() {
                    summe.offen(format!("{titel}: {}", f.offen[0]));
                }
                alle_ok &= f.geprueft
                    && f.problem.is_empty()
                    && f.veraltet.is_empty()
                    && f.offen.is_empty();
            }
            summe.geprueft = alle_ok;
            let ziel = bf(&mut befunde, "nachweise");
            ziel.problem.extend(summe.problem);
            ziel.veraltet.extend(summe.veraltet);
            ziel.offen.extend(summe.offen);
            ziel.geprueft = summe.geprueft;
        }

        // Begonnene Nachweise brauchen ihre Grundlagen.
        let raeume_noetig = ["wohnflaechen", "din277"].iter().any(|x| daten.contains(x));
        if raeume_noetig && !daten.contains("raumbuch") {
            bf(&mut befunde, "raumbuch").verlangt = true;
            bf(&mut befunde, "raumbuch")
                .offen("Wohnflächen- bzw. DIN-277-Nachweis begonnen: Räume im Raumbuch erfassen");
        }
        let gebaeude_noetig = ["vollgeschosse", "hoehen", "abstand", "grz"]
            .iter()
            .any(|x| daten.contains(x));
        if gebaeude_noetig && !daten.contains("gebaeude") {
            bf(&mut befunde, "gebaeude").verlangt = true;
            bf(&mut befunde, "gebaeude")
                .offen("Nachweise brauchen Gebäudewerte: Gebäude und Geschosse erfassen");
        }

        // Aufmaß: Maßstab gesetzt und jede Aufmaßaufgabe mit Messwert.
        if daten.contains("planaufmass") {
            let mut erfuellt: BTreeSet<(String, String, String)> = BTreeSet::new();
            let mut alle: BTreeSet<(String, String, String)> = BTreeSet::new();
            let mut ohne_massstab = Vec::new();
            for f in self.fachobjekte(p, ObjektArt::Planaufmass).await? {
                let Ok(plan) = serde_json::from_value::<Planaufmass>(f.daten) else {
                    continue;
                };
                if plan.dokument.is_some()
                    && plan.kalibrierung.is_none()
                    && plan.ausschnitte.iter().all(|a| a.kalibrierung.is_none())
                {
                    ohne_massstab.push(plan.bezeichnung.clone());
                }
                for a in self.planaufmass_aufgaben(p, &plan).await? {
                    let k = (a.verbraucher.clone(), a.objekt.clone(), a.ziel.clone());
                    if a.gebunden.is_some() || a.wert_vorhanden {
                        erfuellt.insert(k.clone());
                    }
                    alle.insert(k);
                }
            }
            for name in ohne_massstab {
                bf(&mut befunde, "planaufmass").offen(format!("{name}: Maßstab festlegen"));
            }
            let offen = alle.difference(&erfuellt).count();
            if offen > 0 {
                bf(&mut befunde, "planaufmass").offen(format!(
                    "{offen} Aufmaßaufgabe(n) ohne Messwert: zeichnen und übernehmen oder Wert erfassen"
                ));
            }
            let f = bf(&mut befunde, "planaufmass");
            f.geprueft = f.offen.is_empty();
        }
        // Planliste: jeder Plan mit Nummer, Index und vorhandenem Dokument.
        if daten.contains("planliste") {
            let liste = self.planliste(p, wer).await?;
            let f = bf(&mut befunde, "planliste");
            for z in &liste.zeilen {
                let name = if z.eintrag.plannummer.is_empty() {
                    z.eintrag.titel.clone()
                } else {
                    z.eintrag.plannummer.clone()
                };
                if z.aktuelle_version.is_none() {
                    f.problem(format!("{name}: Dokument wurde entfernt"));
                } else if z.index.is_none() {
                    f.offen(format!("{name}: Planindex fehlt"));
                } else if z.ohne_index {
                    f.offen(format!("{name}: neuere Planversion ohne Index"));
                }
            }
            f.geprueft = !liste.zeilen.is_empty();
        }
        // Unterlagen: die Übersicht meldet jede fehlende oder ungeprüfte Unterlage.
        if daten.contains("unterlagen") {
            bf(&mut befunde, "unterlagen").geprueft = true;
        }
        // Gebäude: Geschosse eingestuft, Flurstücke zugeordnet.
        if daten.contains("gebaeude") {
            let mut fehler = Vec::new();
            for f in self.fachobjekte(p, ObjektArt::Gebaeude).await? {
                let Ok(g) = serde_json::from_value::<Gebaeude>(f.daten) else {
                    continue;
                };
                let n = &g.bezeichnung;
                if g.geschosse.is_empty() {
                    fehler.push(format!("{n}: Geschosse erfassen"));
                } else if g.geschosse.iter().any(|s| s.vollgeschoss.is_none())
                    && mit_nachweis.contains("vollgeschosse")
                {
                    // Ohne Vollgeschossprüfung ist das kein offener Punkt des
                    // Gebäudes: Der Nachweis wird in Bereich 2 verlangt.
                    fehler.push(format!(
                        "{n}: Einstufung der Geschosse aus der Vollgeschossprüfung übernehmen"
                    ));
                }
                if g.flurstueck_ids.is_empty() && g.grundstueck_ids.is_empty() {
                    fehler.push(format!("{n}: Flurstücke zuordnen"));
                } else if g.flurstuecke_zu_pruefen == Some(true) {
                    fehler.push(format!("{n}: Flurstückszuordnung prüfen"));
                }
            }
            let f = bf(&mut befunde, "gebaeude");
            for t in fehler {
                f.offen(t);
            }
            f.geprueft = f.offen.is_empty();
        }
        // Raumbuch: jeder Raum mit Fläche und Nutzung.
        if daten.contains("raumbuch") {
            let mut fehler = Vec::new();
            for f in self.fachobjekte(p, ObjektArt::Raumbuch).await? {
                let Ok(r) = serde_json::from_value::<Raumbuch>(f.daten) else {
                    continue;
                };
                let n = &r.bezeichnung;
                if r.raeume.is_empty() {
                    fehler.push(format!("{n}: Räume erfassen"));
                }
                let ohne_flaeche = r.raeume.iter().filter(|x| x.flaeche_m2 <= 0.0).count();
                if ohne_flaeche > 0 {
                    fehler.push(format!("{n}: {ohne_flaeche} Raum/Räume ohne Fläche"));
                }
                let ohne_nutzung = r
                    .raeume
                    .iter()
                    .filter(|x| x.nutzung.trim().is_empty())
                    .count();
                if ohne_nutzung > 0 {
                    fehler.push(format!("{n}: {ohne_nutzung} Raum/Räume ohne Nutzung"));
                }
            }
            let f = bf(&mut befunde, "raumbuch");
            for t in fehler {
                f.offen(t);
            }
            f.geprueft = f.offen.is_empty();
        }
        // Baukennwerte: Gebührenschätzung ohne offene Angaben.
        if daten.contains("baukennwerte") {
            match self.gebuehrenschaetzung(p, wer).await {
                Ok(s) => {
                    let f = bf(&mut befunde, "baukennwerte");
                    for t in s.offen {
                        f.offen(t);
                    }
                    if s.gebuehr_euro.is_none() && f.offen.is_empty() {
                        f.offen("Gebühr lässt sich noch nicht ermitteln");
                    }
                    f.geprueft = f.offen.is_empty();
                }
                Err(e) => bf(&mut befunde, "baukennwerte").problem(e.to_string()),
            }
        }
        // Bauantrag und Formulare gehören zu jedem Verfahren vor der Einreichung.
        let akten = self.verfahrensakten_lesen(p).await?;
        let in_vorbereitung = akten
            .iter()
            .any(|(_, a)| a.status == Verfahrensstatus::Vorbereitung);
        {
            let antrag = self.bauantrag(p).await?;
            if antrag.version > 0 {
                daten.insert("bauantrag");
            }
            let f = bf(&mut befunde, "bauantrag");
            f.verlangt = in_vorbereitung;
            if antrag.version > 0 || in_vorbereitung {
                let n = antrag.fehlende_angaben.len();
                for t in antrag.fehlende_angaben.iter().take(5) {
                    f.offen(format!("Fehlt noch: {t}"));
                }
                if n > 5 {
                    f.offen(format!("… und {} weitere Angaben", n - 5));
                }
                f.geprueft = n == 0;
            }
            let mut formulare = false;
            for d in self.repos.documents.latest_for_project(p).await? {
                if self
                    .formular_instanz(d.document_id, d.version)
                    .await?
                    .is_some()
                {
                    formulare = true;
                    break;
                }
            }
            let f = bf(&mut befunde, "formulare");
            if formulare {
                daten.insert("formulare");
                f.geprueft = true;
            } else if in_vorbereitung {
                f.verlangt = true;
                f.offen("Noch keine Antragsformulare erstellt");
            }
        }
        // Verfahren: geprüft erst mit Entscheidung der Behörde.
        if daten.contains("verfahren") {
            let laufend: Vec<_> = akten
                .iter()
                .filter(|(_, a)| {
                    !matches!(
                        a.status,
                        Verfahrensstatus::Entschieden | Verfahrensstatus::Zurueckgenommen
                    )
                })
                .collect();
            let f = bf(&mut befunde, "verfahren");
            for (_, a) in &laufend {
                f.offen(if a.status == Verfahrensstatus::Vorbereitung {
                    format!("{}: in Vorbereitung, noch nicht eingereicht", a.bezeichnung)
                } else {
                    format!(
                        "{}: Verfahren läuft, noch keine Entscheidung",
                        a.bezeichnung
                    )
                });
            }
            f.geprueft = laufend.is_empty()
                && akten
                    .iter()
                    .any(|(_, a)| a.status == Verfahrensstatus::Entschieden);
        }

        let nicht_benoetigt = self
            .organisation(p)
            .await?
            .map(|(o, _)| o.nicht_benoetigt)
            .unwrap_or_default();
        let bereiche = BEREICHE
            .iter()
            .map(|(id, art)| {
                let kennzeichenbar = *art == Art::Daten && !daten.contains(id);
                let f = befunde.remove(*id).unwrap_or_default();
                let mut gruende: Vec<String> = Vec::new();
                for t in f.problem.iter().chain(&f.veraltet).chain(&f.offen) {
                    dazu(&mut gruende, t.clone());
                }
                let hat_daten = daten.contains(id);
                let zustand = if kennzeichenbar && nicht_benoetigt.iter().any(|n| n == id) {
                    gruende.clear();
                    Bereichszustand::NichtBenoetigt
                } else if !f.problem.is_empty() {
                    Bereichszustand::Problem
                } else if !f.veraltet.is_empty() {
                    Bereichszustand::Veraltet
                } else {
                    match art {
                        Art::Liste if f.offen.is_empty() => Bereichszustand::Leer,
                        Art::Liste => Bereichszustand::InArbeit,
                        _ if !hat_daten
                            && (f.verlangt || *art == Art::Pflicht || !f.offen.is_empty()) =>
                        {
                            Bereichszustand::ZuErledigen
                        }
                        _ if !hat_daten => Bereichszustand::Leer,
                        _ if !f.offen.is_empty() => Bereichszustand::InArbeit,
                        Art::Pflicht => Bereichszustand::Geprueft,
                        _ if f.geprueft => Bereichszustand::Geprueft,
                        _ => {
                            gruende.push("Erfasst, aber noch nicht fachlich geprüft".into());
                            Bereichszustand::InArbeit
                        }
                    }
                };
                Bereichsstatus {
                    bereich: (*id).into(),
                    zustand,
                    gruende,
                    kennzeichenbar,
                }
            })
            .collect();
        Ok(Bereichsstaende {
            projekt_version: uebersicht.projekt_version,
            stichtag: heute,
            bereiche,
        })
    }

    pub(crate) async fn organisation(
        &self,
        p: ProjectId,
    ) -> CoreResult<Option<(Projektorganisation, i64)>> {
        let Some(f) = self
            .fachobjekt_lesen(p, ObjektArt::Projektorganisation, p.0)
            .await?
        else {
            return Ok(None);
        };
        let o = serde_json::from_value(f.daten)
            .map_err(|e| CoreError::InvalidInput(format!("Projektorganisation: {e}")))?;
        Ok(Some((o, f.revision)))
    }

    /// Kennzeichnet einen leeren Datenbereich als „nicht benötigt“ oder nimmt
    /// die Kennzeichnung zurück. Gespeichert in der Projektorganisation (gilt
    /// für alle Mitarbeiter, mit Revision und Verlauf).
    pub async fn bereich_benoetigt_setzen(
        &self,
        p: ProjectId,
        bereich: &str,
        benoetigt: bool,
        wer: &Benutzer,
    ) -> CoreResult<Bereichsstaende> {
        let stand = self.bereichsstatus(p, wer).await?;
        let Some(b) = stand.bereiche.iter().find(|b| b.bereich == bereich) else {
            return Err(CoreError::InvalidInput(format!(
                "Unbekannter Bereich „{bereich}“"
            )));
        };
        if !benoetigt && !b.kennzeichenbar {
            return Err(CoreError::InvalidInput(
                "Nur Bereiche ohne erfasste Daten lassen sich als nicht benötigt kennzeichnen"
                    .into(),
            ));
        }
        let (mut org, basis) = match self.organisation(p).await? {
            Some((o, r)) => (o, r),
            None => (
                Projektorganisation {
                    kennzeichen: String::new(),
                    phase: crate::plattform::objekte::Projektphase::Vorbereitung,
                    verantwortlich: None,
                    beschreibung: String::new(),
                    nicht_benoetigt: vec![],
                },
                0,
            ),
        };
        org.nicht_benoetigt.retain(|n| n != bereich);
        if !benoetigt {
            org.nicht_benoetigt.push(bereich.to_string());
        }
        let daten =
            serde_json::to_value(&org).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        self.fachobjekt_speichern(p, ObjektArt::Projektorganisation, p.0, basis, daten, wer)
            .await?;
        self.bereichsstatus(p, wer).await
    }
}
