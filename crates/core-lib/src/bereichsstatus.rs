//! Bereichsstatus der Projektnavigation: je Bereich, ob für dieses Projekt noch
//! Arbeit offen ist, ob er erledigt ist oder (leer und nicht benötigt) kein
//! Symbol bekommt.
//!
//! Reines Lesemodell auf Grundlage der [`crate::projektuebersicht`]: Jeder
//! Arbeitspunkt der Übersicht nennt seinen Zielbereich und macht diesen Bereich
//! „offen“. Hinzu kommen wenige strukturelle Pflichten (Stammdaten,
//! Rechtsgrundlagen, Flurstücke, Planfestsetzungen bei Bebauungsplan,
//! Raumbuch/Gebäude als Grundlage begonnener Nachweise). Ungespeicherte
//! Änderungen kennt nur die Oberfläche; sperrende Voraussetzungen bleiben dort
//! wie bisher.
use crate::{
    CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::ProjectId,
    plattform::ObjektArt,
    projektuebersicht::{Dringlichkeit, Projektuebersicht},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Bereichszustand {
    /// Benötigt und noch Arbeit offen (gelb).
    Offen,
    /// Benötigt, vollständig und gespeichert (grün).
    Erledigt,
    /// Leer und nicht benötigt, oder Listenbereich ohne offene Einträge (kein Symbol).
    Leer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bereichsstatus {
    /// Kennung des Bereichs wie in der Navigation (z. B. `flurstuecke`, `grz`).
    pub bereich: String,
    pub zustand: Bereichszustand,
    /// Kurze Gründe für „offen“ in der Reihenfolge der Projektübersicht.
    pub gruende: Vec<String>,
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
    /// Immer benötigt: ohne offene Punkte erledigt.
    Pflicht,
    /// Benötigt, sobald Daten vorhanden sind: dann ohne offene Punkte erledigt.
    Daten,
    /// Liste von Einträgen (Aufgaben, Lieferungen …): offen oder ohne Symbol.
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
    ("baukennwerte", Art::Daten),
    ("verfahren", Art::Daten),
    ("portale", Art::Liste),
    ("einreichungen", Art::Liste),
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

/// Arbeitspunkte der Übersicht je Zielbereich, als kurze Gründe.
fn gruende_aus(u: &Projektuebersicht) -> BTreeMap<String, Vec<String>> {
    let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in &u.punkte {
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
        let liste = m.entry(bereich.to_string()).or_default();
        let text = format!("{}{zusatz}", p.titel);
        if !liste.contains(&text) {
            liste.push(text);
        }
    }
    m
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
        let mut gruende = gruende_aus(&uebersicht);
        let mut daten: BTreeSet<&str> = BTreeSet::new();
        for art in DATENARTEN {
            if let Some(b) = bereich_von(art)
                && !self.fachobjektseite(p, art, None, 1).await?.is_empty()
            {
                daten.insert(b);
            }
        }
        // Baugrundstück mit GRZ, GFZ, Geschossigkeit und BMZ.
        if crate::grundstuecksinformationen::ermittlung(recht)?
            .is_some_and(|e| !e.berechnungen.is_empty())
        {
            daten.extend(["grundstueck", "grz", "gfz", "geschosse", "volumen"]);
        }
        let berechnung_begonnen = crate::berechnungen::BerechnungsArt::ALLE
            .iter()
            .any(|a| daten.contains(a.objekt().modul()))
            || daten.contains("abstand");
        if berechnung_begonnen {
            daten.insert("nachweise");
        }
        let mut offen = |b: &str, g: &str| {
            gruende.entry(b.into()).or_default().push(g.into());
        };

        // Stammdaten: Vorhaben und Standort gehören auf jeden Bauantrag.
        if recht.gemeinde.trim().is_empty() || recht.standort.trim().is_empty() {
            offen("uebersicht", "Gemeinde und Standort des Vorhabens angeben");
        }
        if recht.vorhaben.trim().is_empty() {
            offen("uebersicht", "Vorhaben beschreiben");
        }
        // Flurstücke sind Grundlage aller grundstücksbezogenen Nachweise.
        if crate::grundstuecksinformationen::informationen(recht)
            .flurstuecke
            .is_empty()
        {
            offen("flurstuecke", "Mindestens ein Vorhabenflurstück erfassen");
        }
        // Mit Bebauungsplan sind dessen Festsetzungen zu erfassen.
        if recht.mit_bebauungsplan() {
            daten.insert("festsetzungen");
            let w = &recht.festgesetzte_werte;
            let mass = [w.grz, w.gfz, w.bmz, w.grundflaeche_m2, w.geschossflaeche_m2]
                .iter()
                .any(Option::is_some)
                || w.vollgeschosse.is_some();
            if !mass {
                offen(
                    "festsetzungen",
                    "Bebauungsplan angegeben: Maß der baulichen Nutzung (GRZ, GFZ oder Grundfläche, Vollgeschosse) eintragen",
                );
            }
        }
        // Begonnene Nachweise brauchen ihre Grundlagen.
        let raeume_noetig = ["wohnflaechen", "din277"].iter().any(|b| daten.contains(b));
        if raeume_noetig && !daten.contains("raumbuch") {
            offen(
                "raumbuch",
                "Wohnflächen- bzw. DIN-277-Nachweis begonnen: Räume im Raumbuch erfassen",
            );
        }
        let gebaeude_noetig = ["vollgeschosse", "hoehen", "abstand", "grz"]
            .iter()
            .any(|b| daten.contains(b));
        if gebaeude_noetig && !daten.contains("gebaeude") {
            offen(
                "gebaeude",
                "Nachweise brauchen Gebäudewerte: Gebäude und Geschosse erfassen",
            );
        }

        let bereiche = BEREICHE
            .iter()
            .map(|(b, art)| {
                let g = gruende.remove(*b).unwrap_or_default();
                let zustand = if !g.is_empty() {
                    Bereichszustand::Offen
                } else {
                    match art {
                        Art::Pflicht => Bereichszustand::Erledigt,
                        Art::Daten if daten.contains(b) => Bereichszustand::Erledigt,
                        _ => Bereichszustand::Leer,
                    }
                };
                Bereichsstatus {
                    bereich: (*b).into(),
                    zustand,
                    gruende: g,
                }
            })
            .collect();
        Ok(Bereichsstaende {
            projekt_version: uebersicht.projekt_version,
            stichtag: heute,
            bereiche,
        })
    }
}
