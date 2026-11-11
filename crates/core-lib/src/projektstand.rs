//! Stand eines Projekts im Ablauf von der Vorbereitung bis zum Abschluss.
//!
//! Der Stand wird aus den Verfahrensakten abgeleitet (Abgabe, Bescheid,
//! Baubeginn, Fertigstellung). Nur der Abschluss ist eine Entscheidung des
//! Büros; er steht als Phase „abgeschlossen“ in der Projektorganisation.
use crate::{
    CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::ProjectId,
    error::CoreError,
    plattform::{
        ObjektArt,
        objekte::{Projektorganisation, Projektphase},
    },
    verfahren::{Entscheidung, Verfahrensakte, Verfahrensstatus, abgabe::Versandstatus},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Projektstufe {
    Vorbereitung,
    Eingereicht,
    Abgelehnt,
    Genehmigt,
    ImBau,
    Fertiggestellt,
    Abgeschlossen,
}
impl Projektstufe {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Vorbereitung => "In Vorbereitung",
            Self::Eingereicht => "Eingereicht",
            Self::Abgelehnt => "Abgelehnt",
            Self::Genehmigt => "Genehmigt",
            Self::ImBau => "Im Bau",
            Self::Fertiggestellt => "Fertiggestellt",
            Self::Abgeschlossen => "Abgeschlossen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Projektstand {
    pub projekt_id: ProjectId,
    pub stufe: Projektstufe,
    pub bezeichnung: String,
    /// Datum, an dem die Stufe erreicht wurde, soweit die Akte es nennt.
    pub seit: Option<NaiveDate>,
    /// Stufe aus den Verfahrensakten, ohne den Abschluss durch das Büro.
    pub verfahrensstufe: Projektstufe,
}

/// Stufe und Datum aus den Verfahrensakten: Maßgebend ist der am weitesten
/// fortgeschrittene Vorgang aller Akten des Projekts.
pub fn aus_akten(akten: &[Verfahrensakte]) -> (Projektstufe, Option<NaiveDate>) {
    let mut bester = (Projektstufe::Vorbereitung, None);
    let mut melden = |stufe: Projektstufe, datum: Option<NaiveDate>| {
        if stufe > bester.0 || (stufe == bester.0 && datum > bester.1) {
            bester = (stufe, datum);
        }
    };
    for a in akten {
        if a.status == Verfahrensstatus::Zurueckgenommen {
            continue;
        }
        let abgabe = a
            .abgaben
            .iter()
            .flat_map(|s| &s.versand)
            .filter(|v| {
                matches!(
                    v.status,
                    Versandstatus::Versendet | Versandstatus::Eingegangen
                )
            })
            .map(|v| v.datum)
            .chain(a.externe_einreichungen.iter().map(|e| e.datum))
            .min();
        if abgabe.is_some() || a.status != Verfahrensstatus::Vorbereitung {
            melden(Projektstufe::Eingereicht, abgabe);
        }
        let ersetzt: Vec<_> = a.bescheide.iter().filter_map(|b| b.ersetzt).collect();
        if let Some(b) = a
            .bescheide
            .iter()
            .filter(|b| !ersetzt.contains(&b.id) && b.verlaengerung.is_none())
            .max_by_key(|b| b.datum)
        {
            match b.entscheidung {
                Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt => {
                    melden(Projektstufe::Genehmigt, Some(b.datum))
                }
                Entscheidung::Abgelehnt => melden(Projektstufe::Abgelehnt, Some(b.datum)),
                _ => {}
            }
        }
        if let Some(x) = &a.ausfuehrung {
            melden(Projektstufe::ImBau, Some(x.baubeginn));
            if let Some(f) = x.fertigstellung {
                melden(Projektstufe::Fertiggestellt, Some(f));
            }
        }
    }
    bester
}

impl CoreServices {
    async fn akten_fuer_stand(&self, p: ProjectId) -> CoreResult<Vec<Verfahrensakte>> {
        Ok(self
            .repos
            .commit
            .fachstaende(p, ObjektArt::Verfahren.modul())
            .await?
            .into_iter()
            .filter_map(|f| serde_json::from_value(f.daten).ok())
            .collect())
    }
    /// Stand eines Projekts aus Verfahrensakten und Projektabschluss.
    pub async fn projektstand(&self, p: ProjectId) -> CoreResult<Projektstand> {
        let (verfahrensstufe, datum) = aus_akten(&self.akten_fuer_stand(p).await?);
        let abgeschlossen = self
            .organisation(p)
            .await?
            .is_some_and(|(o, _)| matches!(o.phase, Projektphase::Abgeschlossen));
        let (stufe, seit) = if abgeschlossen {
            (Projektstufe::Abgeschlossen, None)
        } else {
            (verfahrensstufe, datum)
        };
        Ok(Projektstand {
            projekt_id: p,
            stufe,
            bezeichnung: stufe.bezeichnung().into(),
            seit,
            verfahrensstufe,
        })
    }
    /// Stände aller Projekte, die der Benutzer lesen darf. Ein nicht
    /// auswertbares Projekt fehlt in der Liste.
    pub async fn projektstaende(&self, wer: &Benutzer) -> CoreResult<Vec<Projektstand>> {
        let mut alle = vec![];
        for p in self.projekte_fuer_benutzer(wer).await? {
            if let Ok(s) = self.projektstand(p.id).await {
                alle.push(s);
            }
        }
        Ok(alle)
    }
    /// Schließt ein Projekt ab oder öffnet es wieder. Der Abschluss steht als
    /// Phase in der Projektorganisation; Daten und Nachweise bleiben
    /// unverändert und bearbeitbar.
    pub async fn projekt_abschliessen(
        &self,
        p: ProjectId,
        abgeschlossen: bool,
        wer: &Benutzer,
    ) -> CoreResult<Projektstand> {
        let (verfahrensstufe, _) = aus_akten(&self.akten_fuer_stand(p).await?);
        let (mut org, basis) = match self.organisation(p).await? {
            Some((o, r)) => (o, r),
            None => (
                Projektorganisation {
                    kennzeichen: String::new(),
                    phase: Projektphase::Vorbereitung,
                    verantwortlich: None,
                    beschreibung: String::new(),
                    nicht_benoetigt: vec![],
                },
                0,
            ),
        };
        org.phase = if abgeschlossen {
            Projektphase::Abgeschlossen
        } else if verfahrensstufe >= Projektstufe::ImBau {
            Projektphase::Ausfuehrung
        } else if verfahrensstufe >= Projektstufe::Eingereicht {
            Projektphase::Genehmigung
        } else {
            Projektphase::Vorbereitung
        };
        let daten =
            serde_json::to_value(&org).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        self.fachobjekt_speichern(p, ObjektArt::Projektorganisation, p.0, basis, daten, wer)
            .await?;
        self.projektstand(p).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn akte(json: serde_json::Value) -> Verfahrensakte {
        let mut basis = serde_json::json!({
            "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
            "behoerde": {"id": uuid::Uuid::nil(), "name": "Bauamt", "kennung": "", "kontakt": ""}, "aktenzeichen": "",
            "verantwortlich": null, "vorgaenger": null, "profil": null, "kommunikation": [],
            "nachforderungen": [], "anforderungen": [], "dokumentbeziehungen": [],
            "abgaben": [], "bescheide": [], "ereignisse": []
        });
        for (k, v) in json.as_object().unwrap() {
            basis[k] = v.clone();
        }
        serde_json::from_value(basis).unwrap()
    }
    #[test]
    fn stufe_folgt_dem_am_weitesten_fortgeschrittenen_vorgang() {
        assert_eq!(aus_akten(&[]), (Projektstufe::Vorbereitung, None));
        assert_eq!(
            aus_akten(&[akte(serde_json::json!({}))]).0,
            Projektstufe::Vorbereitung
        );
        assert_eq!(
            aus_akten(&[akte(serde_json::json!({"status": "in_pruefung"}))]).0,
            Projektstufe::Eingereicht
        );
        let bau = akte(serde_json::json!({"status": "entschieden",
            "ausfuehrung": {"baubeginn": "2027-03-01"}}));
        assert_eq!(
            aus_akten(&[
                akte(serde_json::json!({"status": "eingereicht"})),
                bau.clone()
            ]),
            (Projektstufe::ImBau, "2027-03-01".parse().ok())
        );
        let fertig = akte(serde_json::json!({"status": "entschieden",
            "ausfuehrung": {"baubeginn": "2027-03-01", "fertigstellung": "2028-02-15"}}));
        assert_eq!(
            aus_akten(&[bau, fertig]),
            (Projektstufe::Fertiggestellt, "2028-02-15".parse().ok())
        );
        // Ein zurückgenommenes Verfahren zählt nicht.
        assert_eq!(
            aus_akten(&[akte(serde_json::json!({"status": "zurueckgenommen"}))]).0,
            Projektstufe::Vorbereitung
        );
    }
}
