//! Zuordnung von Formularfeldern zu Projektdaten.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::pdf::{Feldart, Feldwert, Formularfeld};
use crate::calc::Baugebiet;
use crate::domain::{BebauungsplanArt, Bundesland, Project};
use crate::error::{CoreError, CoreResult};
use crate::grundstuecksinformationen::FlurstuecksInfo;
use crate::grundstueckskontext::GrundstueckKontext;
use crate::kunden::{Kunde, Kundentyp};
use crate::xbau::{Buero, Entwurfsverfasser};

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
        match crate::grundstueckskontext::nutzung(p, g) {
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
    /// `bauherr.firma`, `bauherr.name_vorname` („Familienname, Vorname“ bzw.
    /// Firma), `bauherr.strasse`, `bauherr.hausnummer`,
    /// `bauherr.strasse_hausnummer`, `bauherr.plz`, `bauherr.ort`,
    /// `bauherr.plz_ort`, `bauherr.anschrift` (eine Zeile),
    /// `bauherr.name_anschrift` (Name und Anschrift in zwei Zeilen),
    /// `bauherr.name_anschrift_zeile` (dasselbe in einer Zeile),
    /// `bauherr.telefon`, `bauherr.email`.
    pub fn bauherr(&mut self, k: &Kunde) {
        self.person("bauherr", k);
    }

    /// Angaben einer Person aus der Kundendatenbank unter `präfix.*`, mit
    /// denselben Schlüsseln wie [`Self::bauherr`].
    pub fn person(&mut self, praefix: &str, k: &Kunde) {
        let d = &k.daten;
        self.setze(&format!("{praefix}.name"), k.anzeigename());
        let name_vorname = match d.typ {
            Kundentyp::Person => {
                self.setze(&format!("{praefix}.vorname"), &d.vorname);
                self.setze(&format!("{praefix}.familienname"), &d.familienname);
                crate::xbau::name_vorname(&d.familienname, &d.vorname)
            }
            Kundentyp::Organisation => {
                self.setze(&format!("{praefix}.firma"), &d.firma);
                d.firma.trim().to_string()
            }
        };
        self.setze(&format!("{praefix}.name_vorname"), &name_vorname);
        let a = &d.anschrift;
        self.anschrift(praefix, &a.strasse, &a.hausnummer, &a.postleitzahl, &a.ort);
        self.setze(&format!("{praefix}.telefon"), &d.telefon);
        self.setze(&format!("{praefix}.email"), &d.email);
        self.namensblock(praefix, &name_vorname);
        self.leere_entfernen();
    }

    /// Angaben der entwurfsverfassenden Person; die Büroanschrift ist ihre
    /// Geschäftsanschrift: `entwurfsverfasser.name`, `.vorname`,
    /// `.familienname`, `.name_vorname`, `.name_buero` („Familienname,
    /// Vorname, Büro“), `.berufsbezeichnung`, `.mitgliedsnummer`,
    /// `.kammerland`, `.telefon` und `.email` (Person, sonst Büro), `.mobil`,
    /// `.strasse` … `.plz_ort`, `.anschrift`, `.name_anschrift` (Name mit
    /// Büro und Anschrift in zwei Zeilen) und `.name_anschrift_zeile`.
    pub fn entwurfsverfasser(&mut self, e: &Entwurfsverfasser, buero: &Buero) {
        let p = "entwurfsverfasser";
        self.setze("entwurfsverfasser.name", e.anzeigename());
        self.setze("entwurfsverfasser.vorname", &e.vorname);
        self.setze("entwurfsverfasser.familienname", &e.familienname);
        self.setze("entwurfsverfasser.berufsbezeichnung", &e.berufsbezeichnung);
        self.setze("entwurfsverfasser.mitgliedsnummer", &e.mitgliedsnummer);
        if let Some(l) = e.kammerland {
            self.setze("entwurfsverfasser.kammerland", l.name());
        }
        let name_vorname = e.name_vorname();
        self.setze("entwurfsverfasser.name_vorname", &name_vorname);
        let name_buero = [name_vorname.as_str(), buero.name.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        self.setze("entwurfsverfasser.name_buero", &name_buero);
        self.setze("entwurfsverfasser.telefon", e.telefon_oder(buero));
        self.setze("entwurfsverfasser.mobil", &e.mobil);
        self.setze("entwurfsverfasser.email", e.email_oder(buero));
        let a = &buero.anschrift;
        self.anschrift(p, &a.strasse, &a.hausnummer, &a.postleitzahl, &a.ort);
        self.namensblock(p, &name_buero);
        self.leere_entfernen();
    }

    /// `präfix.name_anschrift` (Name und Anschrift in zwei Zeilen, für
    /// mehrzeilige Felder wie „Bauherrschaft“) und
    /// `präfix.name_anschrift_zeile` (dasselbe in einer Zeile) aus `name`
    /// und der zuvor gesetzten Anschrift `präfix.anschrift`.
    pub fn namensblock(&mut self, praefix: &str, name: &str) {
        let anschrift = self
            .wert(&format!("{praefix}.anschrift"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let teile: Vec<&str> = [name.trim(), anschrift.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        self.setze(&format!("{praefix}.name_anschrift"), teile.join("\n"));
        self.setze(&format!("{praefix}.name_anschrift_zeile"), teile.join(", "));
    }

    /// `präfix.strasse`, `.hausnummer`, `.strasse_hausnummer`, `.plz`,
    /// `.ort`, `.plz_ort` und `.anschrift` („Straße Hausnummer, PLZ Ort“).
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
        let zeile = [
            format!("{strasse} {hausnummer}").trim().to_string(),
            format!("{plz} {ort}").trim().to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        self.setze(&format!("{praefix}.anschrift"), zeile);
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

/// Quelle eines Felds: die je Formular gewählte Feldquelle, sonst die
/// Zuordnung der Vorlage, sonst der Feldname selbst.
fn quelle<'a>(
    feld: &'a Formularfeld,
    vorlage: Option<&'a Formularvorlage>,
    feldquellen: &'a BTreeMap<String, String>,
) -> &'a str {
    feldquellen
        .get(&feld.name)
        .or_else(|| vorlage.and_then(|v| v.zuordnung.get(&feld.name)))
        .map(String::as_str)
        .unwrap_or(&feld.name)
}

/// Wirksame Projektdatenquelle je befüllbarem Feld: die gewählte Feldquelle
/// oder die Zuordnung der Vorlage, sonst der Feldname, wenn er ein Schlüssel
/// des Kontexts ist. Feste Texte (`'…`) und nicht befüllbare Felder fehlen.
pub fn feldquellen(
    felder: &[Formularfeld],
    vorlage: Option<&Formularvorlage>,
    gewaehlt: &BTreeMap<String, String>,
    kontext: &Projektkontext,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for feld in felder {
        if feld.schreibgeschuetzt || matches!(feld.art, Feldart::Sonstiges) {
            continue;
        }
        let q = quelle(feld, vorlage, gewaehlt);
        if q.starts_with('\'') {
            continue;
        }
        let zugeordnet = gewaehlt.contains_key(&feld.name)
            || vorlage.is_some_and(|v| v.zuordnung.contains_key(&feld.name));
        if zugeordnet || kontext.wert(q).is_some() {
            out.insert(feld.name.clone(), q.to_owned());
        }
    }
    out
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
    vorschlag_mit_quellen(felder, vorlage, &BTreeMap::new(), kontext)
}

/// Wie [`vorschlag`]; `feldquellen` wählt je Feld eine abweichende Quelle
/// (Schlüssel des Kontexts), etwa `bauherr.name_anschrift` statt der
/// Zuordnung der Vorlage.
pub fn vorschlag_mit_quellen(
    felder: &[Formularfeld],
    vorlage: Option<&Formularvorlage>,
    feldquellen: &BTreeMap<String, String>,
    kontext: &Projektkontext,
) -> BTreeMap<String, Feldwert> {
    let mut werte = BTreeMap::new();
    for feld in felder {
        if feld.schreibgeschuetzt || ausgefuellt(feld.wert.as_ref()) {
            continue;
        }
        let quelle = quelle(feld, vorlage, feldquellen);
        let Some(text) = quelle.strip_prefix('\'').or_else(|| kontext.wert(quelle)) else {
            continue;
        };
        if let Some(wert) = feldwert(feld, text) {
            // Ein Zeilenumbruch passt nur in mehrzeilige Felder; sonst würde
            // die Befüllung scheitern.
            if !feld.mehrzeilig && matches!(&wert, Feldwert::Text(t) if t.contains('\n')) {
                continue;
            }
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
    veraltete_werte_mit_quellen(felder, werte, vorlage, &BTreeMap::new(), kontext)
}

/// Wie [`veraltete_werte`], mit je Feld gewählten Quellen wie bei
/// [`vorschlag_mit_quellen`].
pub fn veraltete_werte_mit_quellen(
    felder: &[Formularfeld],
    werte: &BTreeMap<String, Feldwert>,
    vorlage: Option<&Formularvorlage>,
    feldquellen: &BTreeMap<String, String>,
    kontext: &Projektkontext,
) -> Vec<VeralteterFeldwert> {
    let mut out = Vec::new();
    for feld in felder {
        let wert = werte.get(&feld.name).or(feld.wert.as_ref());
        if feld.schreibgeschuetzt || !ausgefuellt(wert) {
            continue;
        }
        let quelle = quelle(feld, vorlage, feldquellen);
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
    #[test]
    fn sammelschluessel_fuer_personen_und_entwurfsverfasser() {
        use crate::kunden::{Kunde, KundeId, Kundendaten};
        use crate::xbau::{Anschrift, Buero, Entwurfsverfasser};
        let kunde = |daten: Kundendaten| Kunde {
            id: KundeId::new(),
            daten,
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let mut k = Projektkontext::default();
        k.bauherr(&kunde(Kundendaten {
            vorname: "Erika".into(),
            familienname: "Mustermann".into(),
            anschrift: Anschrift {
                strasse: "Musterweg".into(),
                hausnummer: "3".into(),
                postleitzahl: "80331".into(),
                ort: "München".into(),
            },
            ..Default::default()
        }));
        assert_eq!(k.wert("bauherr.name"), Some("Erika Mustermann"));
        assert_eq!(k.wert("bauherr.name_vorname"), Some("Mustermann, Erika"));
        assert_eq!(
            k.wert("bauherr.anschrift"),
            Some("Musterweg 3, 80331 München")
        );
        assert_eq!(
            k.wert("bauherr.name_anschrift"),
            Some("Mustermann, Erika\nMusterweg 3, 80331 München")
        );
        assert_eq!(
            k.wert("bauherr.name_anschrift_zeile"),
            Some("Mustermann, Erika, Musterweg 3, 80331 München")
        );
        // Organisation: Firma statt „Familienname, Vorname“; ohne Anschrift
        // nur der Name, keine leeren Zeilen.
        k.person(
            "eigentuemer",
            &kunde(Kundendaten {
                typ: Kundentyp::Organisation,
                firma: "Bau GmbH".into(),
                ..Default::default()
            }),
        );
        assert_eq!(k.wert("eigentuemer.name_vorname"), Some("Bau GmbH"));
        assert_eq!(k.wert("eigentuemer.name_anschrift"), Some("Bau GmbH"));
        assert!(k.wert("eigentuemer.anschrift").is_none());
        // Entwurfsverfasser: Büroanschrift als Geschäftsanschrift, eigene
        // E-Mail, Telefon des Büros als Ersatz.
        let buero = Buero {
            name: "Planbüro".into(),
            anschrift: Anschrift {
                strasse: "Domstraße".into(),
                hausnummer: "1".into(),
                postleitzahl: "50667".into(),
                ort: "Köln".into(),
            },
            telefon: "0221 5".into(),
            email: "info@planbuero.de".into(),
            ..Default::default()
        };
        let e = Entwurfsverfasser {
            id: uuid::Uuid::new_v4(),
            vorname: "Max".into(),
            familienname: "Plan".into(),
            berufsbezeichnung: "Architekt".into(),
            mitgliedsnummer: "4711".into(),
            kammerland: Some(Bundesland::Nw),
            nachweis: String::new(),
            telefon: String::new(),
            mobil: "0170 1".into(),
            email: "max@planbuero.de".into(),
        };
        k.entwurfsverfasser(&e, &buero);
        assert_eq!(k.wert("entwurfsverfasser.name"), Some("Max Plan"));
        assert_eq!(k.wert("entwurfsverfasser.name_vorname"), Some("Plan, Max"));
        assert_eq!(
            k.wert("entwurfsverfasser.name_buero"),
            Some("Plan, Max, Planbüro")
        );
        assert_eq!(k.wert("entwurfsverfasser.telefon"), Some("0221 5"));
        assert_eq!(k.wert("entwurfsverfasser.mobil"), Some("0170 1"));
        assert_eq!(k.wert("entwurfsverfasser.email"), Some("max@planbuero.de"));
        assert_eq!(
            k.wert("entwurfsverfasser.strasse_hausnummer"),
            Some("Domstraße 1")
        );
        assert_eq!(k.wert("entwurfsverfasser.plz_ort"), Some("50667 Köln"));
        assert_eq!(
            k.wert("entwurfsverfasser.name_anschrift"),
            Some("Plan, Max, Planbüro\nDomstraße 1, 50667 Köln")
        );
        assert_eq!(
            k.wert("entwurfsverfasser.kammerland"),
            Some("Nordrhein-Westfalen")
        );
    }

    #[test]
    fn gewaehlte_feldquelle_geht_der_vorlage_vor() {
        let felder = felder_lesen(&musterformular()).unwrap();
        let mut kontext = Projektkontext::aus_projekt(&projekt());
        kontext.setze("bauherr.name", "Erika Mustermann");
        kontext.setze(
            "bauherr.name_anschrift_zeile",
            "Mustermann, Erika, Musterweg 3, 80331 München",
        );
        kontext.setze("bauherr.name_anschrift", "Mustermann, Erika\nMusterweg 3");
        // Die Vorlage füllt Bauherr_Name aus projekt.name.
        let v = vorlage();
        let gewaehlt = BTreeMap::from([(
            "Bauherr_Name".to_owned(),
            "bauherr.name_anschrift_zeile".to_owned(),
        )]);
        let werte = vorschlag_mit_quellen(&felder, Some(&v), &gewaehlt, &kontext);
        assert_eq!(
            werte["Bauherr_Name"],
            Feldwert::Text("Mustermann, Erika, Musterweg 3, 80331 München".into())
        );
        // Unverändert ohne gewählte Quellen.
        assert_eq!(
            vorschlag(&felder, Some(&v), &kontext)["Bauherr_Name"],
            Feldwert::Text("EFH Muster".into())
        );
        // Ein zweizeiliger Wert passt nicht in ein einzeiliges Feld: kein
        // Vorschlag statt einer scheiternden Befüllung.
        let zweizeilig = BTreeMap::from([(
            "Bauherr_Name".to_owned(),
            "bauherr.name_anschrift".to_owned(),
        )]);
        assert!(
            !vorschlag_mit_quellen(&felder, Some(&v), &zweizeilig, &kontext)
                .contains_key("Bauherr_Name")
        );
        // Der Abgleich gilt für die gewählte Quelle, nicht für die Vorlage.
        let eingabe = BTreeMap::from([(
            "Bauherr_Name".to_owned(),
            Feldwert::Text("Mustermann, Erika, Musterweg 3, 80331 München".into()),
        )]);
        assert!(
            veraltete_werte_mit_quellen(&felder, &eingabe, Some(&v), &gewaehlt, &kontext)
                .is_empty()
        );
        let ohne = veraltete_werte(&felder, &eingabe, Some(&v), &kontext);
        assert_eq!(ohne.len(), 1);
        assert_eq!(ohne[0].quelle, "projekt.name");
        // Wirksame Quellen je Feld: Wahl, Vorlage, keine festen Texte.
        let quellen = feldquellen(&felder, Some(&v), &gewaehlt, &kontext);
        assert_eq!(quellen["Bauherr_Name"], "bauherr.name_anschrift_zeile");
        assert_eq!(quellen["Grundstueck.Flaeche"], "grundstueck.flaeche_m2");
        assert!(!quellen.contains_key("Vorhaben"), "fester Text");
        assert!(!quellen.contains_key("Sonderbau"), "fester Text");
    }

    /// Die mitgelieferten NRW-Vorlagen erkennen die amtlichen Formulare
    /// (Beispiel-PDFs unter `formulare/beispiele`) und belegen Bauherrschaft,
    /// Entwurfsverfasser und Grundstück aus den Sammelschlüsseln vor.
    #[test]
    fn nrw_formulare_werden_erkannt_und_vorbelegt() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formulare");
        let mut katalog = Formularkatalog::default();
        assert!(katalog.laden(&dir).unwrap().is_empty());
        let mut kontext = Projektkontext::default();
        kontext.setze("bauherr.name_vorname", "Mustermann, Erika");
        kontext.setze(
            "bauherr.name_anschrift",
            "Mustermann, Erika\nMusterweg 3, 50667 Köln",
        );
        kontext.setze("bauherr.strasse_hausnummer", "Musterweg 3");
        kontext.setze("entwurfsverfasser.name_buero", "Plan, Max, Planbüro");
        kontext.setze("entwurfsverfasser.plz_ort", "50667 Köln");
        kontext.setze("entwurfsverfasser.email", "max@planbuero.de");
        kontext.setze("grundstueck.anschrift", "Baustraße 7, 50667 Köln");
        for (datei, id, bauherr, bauherr_wert, entwurf) in [
            (
                "mb2026-220-anlage1.2.pdf",
                "nw-bauantrag-anlage-i-2",
                "bauherr_name",
                "Mustermann, Erika",
                "entwurf_name",
            ),
            (
                "Anlage_I_7_Baubeschreibung_250821.pdf",
                "nw-baubeschreibung-anlage-i-7",
                "Bauherrschaft",
                "Mustermann, Erika\nMusterweg 3, 50667 Köln",
                "Name_Vorname_Büro",
            ),
            (
                "anlage_i_8_betriebsbeschreibung_fuer_gewebliche_anlagen.pdf",
                "nw-betriebsbeschreibung-anlage-i-8",
                "Bauherrschaft",
                "Mustermann, Erika\nMusterweg 3, 50667 Köln",
                "Name_Vorname_Büro",
            ),
        ] {
            let pdf = std::fs::read(dir.join("beispiele").join(datei)).unwrap();
            let felder = felder_lesen(&pdf).unwrap();
            let v = katalog
                .erkenne_fuer(&felder, Bundesland::Nw)
                .unwrap_or_else(|| panic!("{datei} nicht erkannt"));
            assert_eq!(v.id, id, "{datei}");
            assert!(
                katalog.erkenne_fuer(&felder, Bundesland::By).is_none(),
                "{datei}: nur für NRW"
            );
            let werte = vorschlag(&felder, Some(v), &kontext);
            assert_eq!(
                werte[bauherr],
                Feldwert::Text(bauherr_wert.into()),
                "{datei}"
            );
            assert_eq!(
                werte[entwurf],
                Feldwert::Text("Plan, Max, Planbüro".into()),
                "{datei}"
            );
            crate::formular::pdf::befuellen(&pdf, &werte)
                .unwrap_or_else(|e| panic!("{datei}: {e}"));
        }
    }
}
