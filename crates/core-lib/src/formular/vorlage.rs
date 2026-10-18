//! Zuordnung von Formularfeldern zu Projektdaten.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::pdf::{Feldart, Feldwert, Formularfeld};
use crate::calc::Baugebiet;
use crate::domain::{BebauungsplanArt, Bundesland, Project};
use crate::error::{CoreError, CoreResult};
use crate::festsetzungen::{Festsetzungen, NutzungsEingabe, NutzungsPruefung};
use crate::grundstuecksinformationen::FlurstuecksInfo;
use crate::grundstueckskontext::GrundstueckKontext;
use crate::kunden::{Kunde, Kundentyp};

/// Flache Sicht auf alle Daten, die in Formulare einfließen können, z. B.
/// `projekt.name` oder `grundstueck.flaeche_m2`. Werte sind bereits für
/// deutsche Formulare formatiert (Dezimalkomma, TT.MM.JJJJ).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Projektkontext(pub BTreeMap<String, String>);

impl Projektkontext {
    /// Projektweite Angaben und, wenn das Projekt genau ein Baugrundstück hat,
    /// dessen Angaben.
    pub fn aus_projekt(p: &Project) -> Self {
        let mut k = Self::projekt(p);
        if let Ok(einheiten) = crate::grundstueckskontext::ermitteln(p)
            && let [g] = einheiten.as_slice()
        {
            k.grundstueck(p, g);
        }
        k
    }

    /// Projektweite Angaben ohne Bezug auf ein Baugrundstück:
    /// `projekt.*`, `planung.*` und `datum.heute`.
    pub fn projekt(p: &Project) -> Self {
        let mut k = Self::default();
        k.setze("projekt.name", &p.name);
        k.setze("projekt.bundesland", p.bundesland.code());
        k.setze("projekt.bundesland_name", p.bundesland.name());
        let r = &p.planungsrecht;
        k.setze("planung.gemeinde", &r.gemeinde);
        k.setze("planung.bebauungsplan", &r.plan_bezeichnung);
        k.setze("planung.bebauungsplan_nummer", &r.plan_nummer);
        if mit_plan(p)
            && let Some(g) = r.festgesetzte_werte.baugebiet
        {
            k.baugebiet(g);
        }
        k.setze(
            "datum.heute",
            crate::kalender::heute().format("%d.%m.%Y").to_string(),
        );
        k.leere_entfernen();
        k
    }

    /// Angaben eines Baugrundstücks: Größe laut Grundbuch
    /// (`grundstueck.flaeche_m2`), Bezugsfläche nach Abzügen, Flurstücke,
    /// Grundbuch sowie die aktuellen Kennzahlen aus der Nutzungsberechnung.
    /// Liefert Hinweise, wenn die Kennzahlen nicht berechnet werden können.
    pub fn grundstueck(&mut self, p: &Project, g: &GrundstueckKontext) -> Vec<String> {
        self.setze("grundstueck.bezeichnung", &g.bezeichnung);
        self.setze("grundstueck.flaeche_m2", zahl(g.eigentumsflaeche_m2, 2));
        if let Some(f) = g.bezugsflaeche_m2 {
            self.setze("grundstueck.bezugsflaeche_m2", zahl(f, 2));
        }
        let infos = crate::grundstuecksinformationen::informationen(&p.planungsrecht);
        let flurstuecke: Vec<_> = g
            .flurstueck_ids
            .iter()
            .filter_map(|id| infos.flurstuecke.iter().find(|f| &f.id == id))
            .collect();
        let liste = |wert: fn(&FlurstuecksInfo) -> &str| {
            let mut werte: Vec<&str> = Vec::new();
            for w in flurstuecke.iter().map(|f| wert(f).trim()) {
                if !w.is_empty() && !werte.contains(&w) {
                    werte.push(w);
                }
            }
            werte.join(", ")
        };
        self.setze("grundstueck.gemarkung", liste(|f| &f.gemarkung));
        self.setze("grundstueck.flur", liste(|f| &f.flur));
        self.setze("grundstueck.flurstuecke", liste(|f| &f.nummer));
        self.setze("grundstueck.grundbuchbezirk", liste(|f| &f.grundbuchbezirk));
        self.setze("grundstueck.grundbuchblatt", liste(|f| &f.grundbuchblatt));
        self.setze(
            "grundstueck.flurstuecksverzeichnis",
            flurstuecke
                .iter()
                .map(|f| {
                    [
                        ("Gemarkung", &f.gemarkung),
                        ("Flur", &f.flur),
                        ("Flurstück", &f.nummer),
                    ]
                    .iter()
                    .filter(|(_, w)| !w.trim().is_empty())
                    .map(|(t, w)| format!("{t} {}", w.trim()))
                    .collect::<Vec<_>>()
                    .join(", ")
                })
                .collect::<Vec<_>>()
                .join("; "),
        );
        let mut hinweise = Vec::new();
        match nutzung(p, g) {
            Ok(Some((pruefung, eingabe, eigene))) => {
                let k = &pruefung.kennzahlen;
                self.setze("kennzahlen.grz", zahl(k.grz, 2));
                self.setze("kennzahlen.grz_gesamt", zahl(k.grz_gesamt, 2));
                self.setze("kennzahlen.gfz", zahl(k.gfz, 2));
                if let Some(bmz) = k.bmz {
                    self.setze("kennzahlen.bmz", zahl(bmz, 2));
                }
                let (mass, vollgeschosse) = match &pruefung.aufstellung {
                    Some(a) => (&a.mass, Some(a.vollgeschosse)),
                    None => (&eingabe.mass, eingabe.vollgeschosse),
                };
                self.setze("flaechen.grundflaeche_m2", zahl(mass.grundflaeche_m2, 2));
                self.setze(
                    "flaechen.grundflaeche_nebenanlagen_m2",
                    zahl(mass.grundflaeche_nebenanlagen_m2, 2),
                );
                self.setze(
                    "flaechen.geschossflaeche_m2",
                    zahl(mass.geschossflaeche_m2, 2),
                );
                if let Some(b) = mass.baumasse_m3 {
                    self.setze("flaechen.baumasse_m3", zahl(b, 2));
                }
                if let Some(v) = vollgeschosse {
                    self.setze("gebaeude.vollgeschosse", v.to_string());
                }
                // Eigene Festsetzungen des Grundstücks gehen vor; ohne Plan gilt
                // das Gebiet der Berechnung.
                if let Some(b) = eigene.and_then(|f| f.baugebiet) {
                    self.baugebiet(b);
                } else if !mit_plan(p) {
                    self.baugebiet(eingabe.gebiet);
                }
            }
            Ok(None) => {}
            Err(e) => hinweise.push(format!("Kennzahlen sind nicht verfügbar: {e}")),
        }
        self.leere_entfernen();
        hinweise
    }

    fn baugebiet(&mut self, g: Baugebiet) {
        self.setze("planung.baugebiet", g.code());
        self.setze("planung.baugebiet_name", g.name());
    }

    /// Angaben des (ersten) Bauherrn aus der Kundendatenbank:
    /// `bauherr.name`, `bauherr.vorname`, `bauherr.familienname`,
    /// `bauherr.firma`, `bauherr.strasse`, `bauherr.hausnummer`,
    /// `bauherr.strasse_hausnummer`, `bauherr.plz`, `bauherr.ort`,
    /// `bauherr.plz_ort`, `bauherr.telefon`, `bauherr.email`.
    pub fn bauherr(&mut self, k: &Kunde) {
        self.person("bauherr", k);
    }

    /// Angaben einer Person aus der Kundendatenbank unter `präfix.*`, mit
    /// denselben Schlüsseln wie [`Self::bauherr`].
    pub fn person(&mut self, praefix: &str, k: &Kunde) {
        let d = &k.daten;
        self.setze(&format!("{praefix}.name"), k.anzeigename());
        match d.typ {
            Kundentyp::Person => {
                self.setze(&format!("{praefix}.vorname"), &d.vorname);
                self.setze(&format!("{praefix}.familienname"), &d.familienname);
            }
            Kundentyp::Organisation => self.setze(&format!("{praefix}.firma"), &d.firma),
        }
        let a = &d.anschrift;
        self.anschrift(praefix, &a.strasse, &a.hausnummer, &a.postleitzahl, &a.ort);
        self.setze(&format!("{praefix}.telefon"), &d.telefon);
        self.setze(&format!("{praefix}.email"), &d.email);
        self.leere_entfernen();
    }

    /// `präfix.strasse`, `.hausnummer`, `.strasse_hausnummer`, `.plz`,
    /// `.ort` und `.plz_ort`.
    pub fn anschrift(
        &mut self,
        praefix: &str,
        strasse: &str,
        hausnummer: &str,
        plz: &str,
        ort: &str,
    ) {
        self.setze(&format!("{praefix}.strasse"), strasse);
        self.setze(&format!("{praefix}.hausnummer"), hausnummer);
        self.setze(
            &format!("{praefix}.strasse_hausnummer"),
            format!("{strasse} {hausnummer}").trim(),
        );
        self.setze(&format!("{praefix}.plz"), plz);
        self.setze(&format!("{praefix}.ort"), ort);
        self.setze(&format!("{praefix}.plz_ort"), format!("{plz} {ort}").trim());
    }

    /// Leere Angaben nicht vorschlagen.
    pub fn leere_entfernen(&mut self) {
        self.0.retain(|_, v| !v.trim().is_empty());
    }

    pub fn setze(&mut self, schluessel: &str, wert: impl Into<String>) {
        self.0.insert(schluessel.to_owned(), wert.into());
    }

    pub fn wert(&self, schluessel: &str) -> Option<&str> {
        self.0.get(schluessel).map(String::as_str)
    }

    /// Werte, die den Datenstand eines Formulars bestimmen. Das Tagesdatum
    /// gehört nicht dazu, sonst gälte jeder Stand nach Mitternacht als
    /// geändert.
    pub fn datenstand(&self) -> BTreeMap<&str, &str> {
        self.0
            .iter()
            .filter(|(k, _)| k.as_str() != "datum.heute")
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

fn mit_plan(p: &Project) -> bool {
    matches!(
        p.planungsrecht.bebauungsplan_art,
        BebauungsplanArt::Qualifiziert
            | BebauungsplanArt::Einfach
            | BebauungsplanArt::Vorhabenbezogen
    )
}

/// Aktuelle Nutzungsprüfung eines Baugrundstücks aus den gespeicherten
/// Eingaben, mit Eingabe und eigenen Festsetzungen. `None`, solange keine
/// Berechnung erfasst ist.
fn nutzung(
    p: &Project,
    g: &GrundstueckKontext,
) -> CoreResult<Option<(NutzungsPruefung, NutzungsEingabe, Option<Festsetzungen>)>> {
    match crate::grundstuecksinformationen::plan(&p.planungsrecht)? {
        Some(plan) => {
            let Some(b) = plan
                .berechnungen
                .iter()
                .find(|b| b.schluessel == g.schluessel)
            else {
                return Ok(None);
            };
            let pruefung = plan
                .pruefe(&p.planungsrecht, p.version)?
                .baugrundstuecke
                .into_iter()
                .find(|n| n.grundstueck.schluessel == g.schluessel)
                .ok_or_else(|| {
                    CoreError::InvalidInput("Berechnung des Baugrundstücks fehlt".into())
                })?
                .pruefung;
            Ok(Some((
                pruefung,
                b.eingabe.clone(),
                b.eigene_festsetzungen.clone(),
            )))
        }
        None => p
            .planungsrecht
            .berechnung
            .as_ref()
            .map(|e| {
                crate::festsetzungen::pruefe(e, &p.planungsrecht, p.version)
                    .map(|r| (r, e.clone(), None))
            })
            .transpose(),
    }
}

/// Deutsche Zahlendarstellung ohne Tausenderpunkte, z. B. `640,00`.
pub fn zahl(wert: f64, stellen: usize) -> String {
    format!("{wert:.stellen$}").replace('.', ",")
}

/// Beschreibt ein amtliches Formular und wie seine Felder befüllt werden.
///
/// ```json
/// {
///   "id": "by-bauantrag",
///   "titel": "Bauantrag (Bayern)",
///   "land": "BY",
///   "version": "2026.1",
///   "erkennung": ["Bauherr_Name", "Grundstueck.Flaeche"],
///   "zuordnung": {
///     "Bauherr_Name": "projekt.name",
///     "Grundstueck.Flaeche": "grundstueck.flaeche_m2",
///     "Vorhaben": "'Neubau"
///   }
/// }
/// ```
///
/// Eine Quelle ist ein Schlüssel aus dem [`Projektkontext`] oder, mit
/// führendem `'`, ein fester Text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Formularvorlage {
    pub id: String,
    pub titel: String,
    #[serde(default)]
    pub land: Option<Bundesland>,
    pub version: String,
    /// Feldnamen, an denen das Formular erkannt wird.
    pub erkennung: Vec<String>,
    pub zuordnung: BTreeMap<String, String>,
}

impl Formularvorlage {
    /// Gilt die Zuordnung für ein Projekt in `land`? Zuordnungen ohne Land
    /// gelten bundesweit.
    pub fn gilt_fuer(&self, land: Bundesland) -> bool {
        self.land.is_none_or(|l| l == land)
    }

    pub fn passt_zu(&self, felder: &[Formularfeld]) -> bool {
        !self.erkennung.is_empty()
            && self
                .erkennung
                .iter()
                .all(|e| felder.iter().any(|f| &f.name == e))
    }
}

/// Quelle eines Felds: Zuordnung der Vorlage, sonst der Feldname selbst.
fn quelle<'a>(feld: &'a Formularfeld, vorlage: Option<&'a Formularvorlage>) -> &'a str {
    vorlage
        .and_then(|v| v.zuordnung.get(&feld.name))
        .map(String::as_str)
        .unwrap_or(&feld.name)
}

/// Feldwert für `text` passend zur Feldart; `None`, wenn das Feld den Wert
/// nicht aufnehmen kann (z. B. unbekannte Option).
fn feldwert(feld: &Formularfeld, text: &str) -> Option<Feldwert> {
    match feld.art {
        Feldart::Text => Some(Feldwert::Text(text.to_owned())),
        Feldart::Auswahl | Feldart::Optionsfeld if feld.optionen.iter().any(|o| o == text) => {
            Some(Feldwert::Text(text.to_owned()))
        }
        Feldart::Ankreuzfeld => Some(Feldwert::Ja(!matches!(
            text.trim().to_lowercase().as_str(),
            "" | "0" | "nein" | "false"
        ))),
        _ => None,
    }
}

fn ausgefuellt(wert: Option<&Feldwert>) -> bool {
    match wert {
        Some(Feldwert::Text(t)) => !t.trim().is_empty(),
        Some(Feldwert::Ja(an)) => *an,
        None => false,
    }
}

/// Erzeugt Vorschläge für Feldwerte. Ohne Vorlage werden nur Felder befüllt,
/// deren Name exakt einem Schlüssel des Kontexts entspricht (für eigene
/// Formulare). Bereits ausgefüllte Felder bleiben unangetastet; weichen sie
/// von den Projektdaten ab, meldet [`veraltete_werte`] sie.
pub fn vorschlag(
    felder: &[Formularfeld],
    vorlage: Option<&Formularvorlage>,
    kontext: &Projektkontext,
) -> BTreeMap<String, Feldwert> {
    let mut werte = BTreeMap::new();
    for feld in felder {
        if feld.schreibgeschuetzt || ausgefuellt(feld.wert.as_ref()) {
            continue;
        }
        let quelle = quelle(feld, vorlage);
        let Some(text) = quelle.strip_prefix('\'').or_else(|| kontext.wert(quelle)) else {
            continue;
        };
        if let Some(wert) = feldwert(feld, text) {
            werte.insert(feld.name.clone(), wert);
        }
    }
    werte
}

/// Ein ausgefülltes Feld, dessen Wert nicht (mehr) dem Wert aus den aktuellen
/// Projektdaten entspricht, etwa nach Änderung der Bauherrenanschrift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VeralteterFeldwert {
    /// Vollqualifizierter Feldname.
    pub feld: String,
    /// Schlüssel im [`Projektkontext`], aus dem das Feld befüllt wird.
    pub quelle: String,
    /// Eingetragener Wert.
    pub wert: Feldwert,
    /// Wert aus den aktuellen Projektdaten.
    pub aktuell: Feldwert,
}

/// Vergleicht ausgefüllte Felder mit dem aktuellen Kontext. Gemeldet wird
/// jedes Feld mit Projektdatenquelle, dessen Wert vom aktuellen Projektwert
/// abweicht. Feste Texte der Vorlage (`'…`), das Tagesdatum und Felder ohne
/// aktuellen Projektwert werden nicht verglichen. `werte` überlagert die im
/// PDF stehenden Feldwerte (z. B. ungespeicherte Eingaben).
pub fn veraltete_werte(
    felder: &[Formularfeld],
    werte: &BTreeMap<String, Feldwert>,
    vorlage: Option<&Formularvorlage>,
    kontext: &Projektkontext,
) -> Vec<VeralteterFeldwert> {
    let mut out = Vec::new();
    for feld in felder {
        let wert = werte.get(&feld.name).or(feld.wert.as_ref());
        if feld.schreibgeschuetzt || !ausgefuellt(wert) {
            continue;
        }
        let quelle = quelle(feld, vorlage);
        if quelle.starts_with('\'') || quelle == "datum.heute" {
            continue;
        }
        let (Some(wert), Some(aktuell)) =
            (wert, kontext.wert(quelle).and_then(|t| feldwert(feld, t)))
        else {
            continue;
        };
        let gleich = match (wert, &aktuell) {
            (Feldwert::Text(a), Feldwert::Text(b)) => a.trim() == b.trim(),
            (a, b) => a == b,
        };
        if !gleich {
            out.push(VeralteterFeldwert {
                feld: feld.name.clone(),
                quelle: quelle.to_owned(),
                wert: wert.clone(),
                aktuell,
            });
        }
    }
    out
}

/// Alle bekannten Formularvorlagen, geladen aus `formulare/*.json`.
#[derive(Debug, Default)]
pub struct Formularkatalog {
    vorlagen: Vec<Formularvorlage>,
}

impl Formularkatalog {
    pub fn new(vorlagen: Vec<Formularvorlage>) -> Self {
        Self { vorlagen }
    }

    /// Lädt alle `*.json`-Dateien eines Verzeichnisses. Fehlerhafte Dateien
    /// werden gesammelt zurückgegeben, statt das Laden abzubrechen. Spätere
    /// Verzeichnisse überschreiben Vorlagen mit gleicher `id` (Updates).
    pub fn laden(&mut self, dir: &Path) -> CoreResult<Vec<CoreError>> {
        let mut fehler = Vec::new();
        let eintraege = std::fs::read_dir(dir)
            .map_err(|e| CoreError::InvalidInput(format!("{}: {e}", dir.display())))?;
        let mut pfade: Vec<_> = eintraege
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        pfade.sort();
        for pfad in pfade {
            let ergebnis = std::fs::read(&pfad)
                .map_err(|e| e.to_string())
                .and_then(|raw| {
                    serde_json::from_slice::<Formularvorlage>(&raw).map_err(|e| e.to_string())
                });
            match ergebnis {
                Ok(v) => {
                    self.vorlagen.retain(|alt| alt.id != v.id);
                    self.vorlagen.push(v);
                }
                Err(e) => fehler.push(CoreError::InvalidInput(format!("{}: {e}", pfad.display()))),
            }
        }
        Ok(fehler)
    }

    pub fn vorlagen(&self) -> &[Formularvorlage] {
        &self.vorlagen
    }

    /// Die Vorlage mit den meisten passenden Erkennungsfeldern, ohne Rücksicht
    /// auf das Land. Für Projekte [`Self::erkenne_fuer`] verwenden.
    pub fn erkenne(&self, felder: &[Formularfeld]) -> Option<&Formularvorlage> {
        self.vorlagen
            .iter()
            .filter(|v| v.passt_zu(felder))
            .max_by_key(|v| v.erkennung.len())
    }

    /// Erkennung für ein Projekt in `land`: nur Zuordnungen dieses Landes oder
    /// bundesweite. Es gewinnt die Vorlage mit den meisten Erkennungsfeldern,
    /// bei Gleichstand die landesspezifische, danach die kleinere `id` –
    /// unabhängig von der Ladereihenfolge.
    pub fn erkenne_fuer(
        &self,
        felder: &[Formularfeld],
        land: Bundesland,
    ) -> Option<&Formularvorlage> {
        self.vorlagen
            .iter()
            .filter(|v| v.gilt_fuer(land) && v.passt_zu(felder))
            .max_by(|a, b| {
                (a.erkennung.len(), a.land.is_some())
                    .cmp(&(b.erkennung.len(), b.land.is_some()))
                    .then_with(|| b.id.cmp(&a.id))
            })
    }

    /// Zuordnung für ein Formular: die zuvor verwendete (`bisher`), solange sie
    /// noch geladen ist, zum Land passt und das Formular erkennt; sonst
    /// [`Self::erkenne_fuer`].
    pub fn zuordnung_fuer(
        &self,
        felder: &[Formularfeld],
        land: Bundesland,
        bisher: Option<&str>,
    ) -> Option<&Formularvorlage> {
        bisher
            .and_then(|id| self.vorlagen.iter().find(|v| v.id == id))
            .filter(|v| v.gilt_fuer(land) && v.passt_zu(felder))
            .or_else(|| self.erkenne_fuer(felder, land))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProjectId;
    use crate::formular::pdf::felder_lesen;
    use crate::formular::testdaten::musterformular;

    fn projekt() -> Project {
        Project {
            id: ProjectId::new(),
            name: "EFH Muster".into(),
            bundesland: Bundesland::By,
            grundstuecksflaeche_m2: Some(640.0),
            planungsrecht: Default::default(),
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn vorlage() -> Formularvorlage {
        serde_json::from_str(
            r#"{ "id": "test", "titel": "Test", "land": "BY", "version": "1",
                 "erkennung": ["Bauherr_Name", "Grundstueck.Flaeche"],
                 "zuordnung": { "Bauherr_Name": "projekt.name",
                                "Grundstueck.Flaeche": "grundstueck.flaeche_m2",
                                "Vorhaben": "'Neubau", "Sonderbau": "'nein",
                                "Gebaeudeklasse": "'7" } }"#,
        )
        .unwrap()
    }

    #[test]
    fn erkennt_und_schlaegt_vor() {
        let felder = felder_lesen(&musterformular()).unwrap();
        let katalog = Formularkatalog::new(vec![vorlage()]);
        let v = katalog.erkenne(&felder).expect("Vorlage erkannt");
        let werte = vorschlag(&felder, Some(v), &Projektkontext::aus_projekt(&projekt()));
        assert_eq!(werte["Bauherr_Name"], Feldwert::Text("EFH Muster".into()));
        assert_eq!(
            werte["Grundstueck.Flaeche"],
            Feldwert::Text("640,00".into())
        );
        assert_eq!(werte["Vorhaben"], Feldwert::Text("Neubau".into()));
        assert_eq!(werte["Sonderbau"], Feldwert::Ja(false));
        // "7" ist keine gültige Option und wird nicht vorgeschlagen.
        assert!(!werte.contains_key("Gebaeudeklasse"));
        // Vorschläge lassen sich direkt eintragen.
        crate::formular::pdf::befuellen(&musterformular(), &werte).unwrap();
    }

    #[test]
    fn erkennung_nach_bundesland() {
        let felder = felder_lesen(&musterformular()).unwrap();
        let mut bw = vorlage();
        bw.id = "bw".into();
        bw.land = Some(Bundesland::Bw);
        let mut bund = vorlage();
        bund.id = "bund".into();
        bund.land = None;
        // Ladereihenfolge entscheidet nicht mehr.
        for katalog in [
            Formularkatalog::new(vec![vorlage(), bw.clone(), bund.clone()]),
            Formularkatalog::new(vec![bund.clone(), bw.clone(), vorlage()]),
        ] {
            assert_eq!(
                katalog.erkenne_fuer(&felder, Bundesland::By).unwrap().id,
                "test"
            );
            assert_eq!(
                katalog.erkenne_fuer(&felder, Bundesland::Bw).unwrap().id,
                "bw"
            );
            assert_eq!(
                katalog.erkenne_fuer(&felder, Bundesland::He).unwrap().id,
                "bund"
            );
            // Eine gespeicherte Zuordnung bleibt maßgebend, wenn sie passt.
            assert_eq!(
                katalog
                    .zuordnung_fuer(&felder, Bundesland::By, Some("bund"))
                    .unwrap()
                    .id,
                "bund"
            );
            assert_eq!(
                katalog
                    .zuordnung_fuer(&felder, Bundesland::By, Some("bw"))
                    .unwrap()
                    .id,
                "test"
            );
        }
        let nur_bw = Formularkatalog::new(vec![bw]);
        assert!(nur_bw.erkenne_fuer(&felder, Bundesland::By).is_none());
    }

    #[test]
    fn abweichende_feldwerte_werden_als_veraltet_gemeldet() {
        let mut felder = felder_lesen(&musterformular()).unwrap();
        for f in &mut felder {
            match f.name.as_str() {
                "Bauherr_Name" => f.wert = Some(Feldwert::Text("Alter Name".into())),
                "Grundstueck.Flaeche" => f.wert = Some(Feldwert::Text(" 640,00 ".into())),
                "Vorhaben" => f.wert = Some(Feldwert::Text("Umbau".into())),
                _ => {}
            }
        }
        let v = vorlage();
        let kontext = Projektkontext::aus_projekt(&projekt());
        let veraltet = veraltete_werte(&felder, &BTreeMap::new(), Some(&v), &kontext);
        assert_eq!(veraltet.len(), 1, "{veraltet:?}");
        assert_eq!(veraltet[0].feld, "Bauherr_Name");
        assert_eq!(veraltet[0].quelle, "projekt.name");
        assert_eq!(veraltet[0].aktuell, Feldwert::Text("EFH Muster".into()));
        // Ungespeicherte Eingaben überlagern den PDF-Wert.
        let eingabe = BTreeMap::from([(
            "Bauherr_Name".to_owned(),
            Feldwert::Text("EFH Muster".into()),
        )]);
        assert!(veraltete_werte(&felder, &eingabe, Some(&v), &kontext).is_empty());
    }

    #[test]
    fn ohne_vorlage_nur_exakte_namen() {
        let felder = vec![Formularfeld {
            name: "projekt.name".into(),
            beschriftung: None,
            art: Feldart::Text,
            wert: None,
            optionen: vec![],
            schreibgeschuetzt: false,
            max_laenge: None,
            mehrzeilig: false,
        }];
        let werte = vorschlag(&felder, None, &Projektkontext::aus_projekt(&projekt()));
        assert_eq!(werte["projekt.name"], Feldwert::Text("EFH Muster".into()));
    }

    #[test]
    fn mitgelieferte_vorlagen_laden() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formulare");
        let mut katalog = Formularkatalog::default();
        let fehler = katalog.laden(&dir).unwrap();
        assert!(fehler.is_empty(), "{fehler:?}");
        assert!(!katalog.vorlagen().is_empty());
    }

    #[test]
    fn bauherr_im_kontext() {
        use crate::kunden::{Kunde, KundeId, Kundendaten};
        let mut k = Projektkontext::aus_projekt(&projekt());
        let mut daten = Kundendaten {
            vorname: "Erika".into(),
            familienname: "Mustermann".into(),
            ..Kundendaten::default()
        };
        daten.anschrift.postleitzahl = "80331".into();
        daten.anschrift.ort = "München".into();
        k.bauherr(&Kunde {
            id: KundeId::new(),
            daten,
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        assert_eq!(k.wert("bauherr.name"), Some("Erika Mustermann"));
        assert_eq!(k.wert("bauherr.plz_ort"), Some("80331 München"));
        assert_eq!(k.wert("bauherr.strasse_hausnummer"), None, "leer");
        assert_eq!(k.wert("projekt.name"), Some("EFH Muster"));
    }
}
