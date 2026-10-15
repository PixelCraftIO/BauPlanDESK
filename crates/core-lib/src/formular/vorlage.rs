//! Zuordnung von Formularfeldern zu Projektdaten.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};

use super::pdf::{Feldart, Feldwert, Formularfeld};
use crate::domain::{Bundesland, Project};
use crate::error::{CoreError, CoreResult};
use crate::kunden::{Kunde, Kundentyp};

/// Flache Sicht auf alle Daten, die in Formulare einfließen können, z. B.
/// `projekt.name` oder `grundstueck.flaeche_m2`. Werte sind bereits für
/// deutsche Formulare formatiert (Dezimalkomma, TT.MM.JJJJ).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Projektkontext(pub BTreeMap<String, String>);

impl Projektkontext {
    pub fn aus_projekt(p: &Project) -> Self {
        let mut k = Self::default();
        k.setze("projekt.name", &p.name);
        k.setze("projekt.bundesland", p.bundesland.code());
        k.setze("projekt.bundesland_name", p.bundesland.name());
        if let Ok(einheiten) = crate::grundstueckskontext::ermitteln(p)
            && einheiten.len() == 1
            && let Some(f) = einheiten[0].bezugsflaeche_m2
        {
            k.setze("grundstueck.flaeche_m2", zahl(f, 2));
        }
        k.setze("datum.heute", Local::now().format("%d.%m.%Y").to_string());
        k
    }

    /// Angaben des (ersten) Bauherrn aus der Kundendatenbank:
    /// `bauherr.name`, `bauherr.vorname`, `bauherr.familienname`,
    /// `bauherr.firma`, `bauherr.strasse`, `bauherr.hausnummer`,
    /// `bauherr.strasse_hausnummer`, `bauherr.plz`, `bauherr.ort`,
    /// `bauherr.plz_ort`, `bauherr.telefon`, `bauherr.email`.
    pub fn bauherr(&mut self, k: &Kunde) {
        let d = &k.daten;
        let a = &d.anschrift;
        self.setze("bauherr.name", k.anzeigename());
        match d.typ {
            Kundentyp::Person => {
                self.setze("bauherr.vorname", &d.vorname);
                self.setze("bauherr.familienname", &d.familienname);
            }
            Kundentyp::Organisation => self.setze("bauherr.firma", &d.firma),
        }
        self.setze("bauherr.strasse", &a.strasse);
        self.setze("bauherr.hausnummer", &a.hausnummer);
        self.setze(
            "bauherr.strasse_hausnummer",
            format!("{} {}", a.strasse, a.hausnummer).trim(),
        );
        self.setze("bauherr.plz", &a.postleitzahl);
        self.setze("bauherr.ort", &a.ort);
        self.setze(
            "bauherr.plz_ort",
            format!("{} {}", a.postleitzahl, a.ort).trim(),
        );
        self.setze("bauherr.telefon", &d.telefon);
        self.setze("bauherr.email", &d.email);
        // Leere Angaben nicht vorschlagen.
        self.0.retain(|_, v| !v.is_empty());
    }

    pub fn setze(&mut self, schluessel: &str, wert: impl Into<String>) {
        self.0.insert(schluessel.to_owned(), wert.into());
    }

    pub fn wert(&self, schluessel: &str) -> Option<&str> {
        self.0.get(schluessel).map(String::as_str)
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
    pub fn passt_zu(&self, felder: &[Formularfeld]) -> bool {
        !self.erkennung.is_empty()
            && self
                .erkennung
                .iter()
                .all(|e| felder.iter().any(|f| &f.name == e))
    }
}

/// Erzeugt Vorschläge für Feldwerte. Ohne Vorlage werden nur Felder befüllt,
/// deren Name exakt einem Schlüssel des Kontexts entspricht (für eigene
/// Formulare). Bereits ausgefüllte Felder bleiben unangetastet.
pub fn vorschlag(
    felder: &[Formularfeld],
    vorlage: Option<&Formularvorlage>,
    kontext: &Projektkontext,
) -> BTreeMap<String, Feldwert> {
    let mut werte = BTreeMap::new();
    for feld in felder {
        let ausgefuellt = match &feld.wert {
            Some(Feldwert::Text(t)) => !t.trim().is_empty(),
            Some(Feldwert::Ja(an)) => *an,
            None => false,
        };
        if feld.schreibgeschuetzt || ausgefuellt {
            continue;
        }
        let quelle = vorlage
            .and_then(|v| v.zuordnung.get(&feld.name))
            .map(String::as_str)
            .unwrap_or(&feld.name);
        let Some(text) = quelle.strip_prefix('\'').or_else(|| kontext.wert(quelle)) else {
            continue;
        };
        let wert = match feld.art {
            Feldart::Text => Feldwert::Text(text.to_owned()),
            Feldart::Auswahl | Feldart::Optionsfeld if feld.optionen.iter().any(|o| o == text) => {
                Feldwert::Text(text.to_owned())
            }
            Feldart::Ankreuzfeld => Feldwert::Ja(!matches!(
                text.trim().to_lowercase().as_str(),
                "" | "0" | "nein" | "false"
            )),
            _ => continue,
        };
        werte.insert(feld.name.clone(), wert);
    }
    werte
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

    /// Die Vorlage mit den meisten passenden Erkennungsfeldern.
    pub fn erkenne(&self, felder: &[Formularfeld]) -> Option<&Formularvorlage> {
        self.vorlagen
            .iter()
            .filter(|v| v.passt_zu(felder))
            .max_by_key(|v| v.erkennung.len())
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
