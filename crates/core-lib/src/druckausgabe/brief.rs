//! Schreiben an die Behörde im Briefbogen des Büros. Die Fachmodule liefern
//! einen typisierten [`Brief`]; Satz, Schrift und Briefbogen bleiben hier.
use super::berichte::{Zeile, abschnitt, hinweis, tabelle, text, titel};
use super::*;

/// Baustein eines Schreibens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Briefblock {
    Absatz {
        text: String,
    },
    /// Nummerierter Abschnitt (z. B. ein Forderungspunkt).
    Abschnitt {
        titel: String,
    },
    /// Hervorgehobener Hinweis.
    Hinweis {
        text: String,
    },
    Tabelle {
        koepfe: Vec<String>,
        zeilen: Vec<Vec<String>>,
    },
    /// Unterschriftsfelder: (Rolle, Name).
    Unterschrift {
        felder: Vec<(String, String)>,
    },
}
/// Ein Schreiben mit Briefkopf und Inhalt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Brief {
    pub absender: String,
    pub empfaenger: String,
    pub datum: chrono::NaiveDate,
    pub betreff: String,
    /// Aktenzeichen der Behörde; leer = ohne.
    pub aktenzeichen: String,
    pub bloecke: Vec<Briefblock>,
}
impl Brief {
    fn zeilen(&self, projekt: &str) -> Vec<Zeile> {
        let mut z = vec![
            text(&self.absender),
            text(""),
            text(&self.empfaenger),
            text(""),
            text(self.datum.format("%d.%m.%Y").to_string()),
            text(""),
            titel(&self.betreff),
            text(format!("Bauvorhaben: {projekt}")),
        ];
        if !self.aktenzeichen.trim().is_empty() {
            z.push(text(format!("Aktenzeichen: {}", self.aktenzeichen.trim())));
        }
        z.push(text(""));
        for b in &self.bloecke {
            z.push(match b {
                Briefblock::Absatz { text: t } => text(t),
                Briefblock::Abschnitt { titel } => abschnitt(titel),
                Briefblock::Hinweis { text } => hinweis(text),
                Briefblock::Tabelle { koepfe, zeilen } => {
                    let k: Vec<&str> = koepfe.iter().map(String::as_str).collect();
                    let breiten = vec![1.0; k.len()];
                    tabelle(&k, &breiten, zeilen.clone())
                }
                Briefblock::Unterschrift { felder } => Zeile::Unterschrift(felder.clone()),
            });
        }
        z
    }
}
impl CoreServices {
    /// Setzt das Schreiben im Briefbogen des Büros (gespeicherte Gestaltung).
    pub async fn brief_pdf(&self, p: ProjectId, b: &Brief) -> CoreResult<Vec<u8>> {
        if b.betreff.trim().is_empty()
            || b.betreff.len() > 1000
            || b.absender.len() > 4000
            || b.empfaenger.len() > 4000
            || b.bloecke.len() > 2000
        {
            return Err(fehler("Schreiben benötigt Betreff und begrenzte Angaben"));
        }
        let projekt = self.get_project(p).await?;
        let mut d = super::pdf::bericht("Schreiben", None, &b.zeilen(&projekt.name))?;
        let auftrag = Druckauftrag {
            gestaltung: Some(self.druckgestaltung().await?),
            aktenzeichen: b.aktenzeichen.clone(),
            datum: Some(b.datum),
            titel: b.betreff.clone(),
            absender: b.absender.clone(),
            empfaenger: b.empfaenger.clone(),
            anschreiben: String::new(),
            mit_anschreiben: false,
            inhaltsverzeichnis: false,
            deckblatt: false,
            trennblaetter: false,
            aufgestellt_von: None,
            projekt_version: projekt.version,
            quellen: vec![],
        };
        super::layout::briefbogen(&mut d, &projekt.name, &auftrag, b.datum)?;
        super::pdf::speichern(d)
    }
}
