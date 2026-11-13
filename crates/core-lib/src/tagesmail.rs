//! Tägliche E-Mail-Zusammenfassung der Fristen (Netzwerkbetrieb).
//!
//! Vertrag:
//! - Jeder Benutzer stellt seine Zusammenfassung selbst ein (Adresse, Umfang,
//!   Abwesenheit mit Vertretung); gespeichert unter `tagesmail.<benutzer>`.
//! - [`CoreServices::tagesmails`] stellt die Nachrichten eines Tages zusammen:
//!   überfällige Termine und Termine der nächsten sieben Tage aus dem
//!   Büro-Cockpit ([`crate::cockpit`]); Projektrechte gelten wie dort.
//! - Umfang `meine`: nur Termine, für die der Empfänger zuständig ist. Ist ein
//!   Kollege abwesend und hat ihn als Vertretung benannt, erhält der Empfänger
//!   auch dessen Termine; der Abwesende erhält bis dahin keine Nachricht.
//! - Ohne fällige Termine wird keine Nachricht erzeugt.
//! - Versand und Zeitplan liegen beim Server (SMTP-Zugang des Büros); der
//!   Core versendet nichts. [`CoreServices::tagesmail_beanspruchen`] sorgt dafür,
//!   dass bei mehreren Serverinstanzen nur eine den Tag versendet.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, BenutzerId},
    cockpit::{CockpitTermin, Friststufe},
    kalenderabo::Aboumfang,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Tagesmaileinstellung {
    pub aktiv: bool,
    pub adresse: String,
    pub umfang: Aboumfang,
    /// Abwesend bis einschließlich; während dessen gehen eigene Termine an die Vertretung.
    #[serde(default)]
    pub abwesend_bis: Option<NaiveDate>,
    #[serde(default)]
    pub vertretung: Option<BenutzerId>,
}
impl Default for Tagesmaileinstellung {
    fn default() -> Self {
        Self {
            aktiv: false,
            adresse: String::new(),
            umfang: Aboumfang::Meine,
            abwesend_bis: None,
            vertretung: None,
        }
    }
}
impl Tagesmaileinstellung {
    fn abwesend(&self, heute: NaiveDate) -> bool {
        self.abwesend_bis.is_some_and(|b| heute <= b)
    }
}

/// Eine zu versendende Zusammenfassung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tagesmail {
    pub empfaenger: BenutzerId,
    pub adresse: String,
    pub betreff: String,
    pub text: String,
}

/// Einfache Adressprüfung: genau ein @, Punkt im Domänenteil, keine Leer- oder Steuerzeichen.
pub fn adresse_gueltig(a: &str) -> bool {
    let Some((lokal, domain)) = a.split_once('@') else {
        return false;
    };
    !lokal.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && a.len() <= 254
        && !a
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == ',' || c == ';')
}

fn schluessel(id: BenutzerId) -> String {
    format!("tagesmail.{}", id.0)
}
const VERSANDTAG: &str = "tagesmail.versandtag";

/// Text der Nachricht; `None`, wenn nichts fällig ist (rein).
pub fn zusammenfassen(
    name: &str,
    heute: NaiveDate,
    termine: &[(CockpitTermin, Option<String>)],
) -> Option<(String, String)> {
    let mut faellig: Vec<&(CockpitTermin, Option<String>)> = termine
        .iter()
        .filter(|(t, _)| matches!(t.stufe, Friststufe::Ueberfaellig | Friststufe::DieseWoche))
        .collect();
    if faellig.is_empty() {
        return None;
    }
    faellig.sort_by(|a, b| a.0.stufe.cmp(&b.0.stufe).then(a.0.datum.cmp(&b.0.datum)));
    let ueber = faellig
        .iter()
        .filter(|(t, _)| t.stufe == Friststufe::Ueberfaellig)
        .count();
    let betreff = if ueber > 0 {
        format!(
            "BauPlanDESK: {ueber} überfällig, {} diese Woche ({})",
            faellig.len() - ueber,
            heute.format("%d.%m.%Y")
        )
    } else {
        format!(
            "BauPlanDESK: {} Termine diese Woche ({})",
            faellig.len(),
            heute.format("%d.%m.%Y")
        )
    };
    let mut text = format!("Guten Morgen {name},\n\nhier sind Ihre Fristen und Wiedervorlagen:\n");
    for (stufe, titel) in [
        (Friststufe::Ueberfaellig, "Überfällig"),
        (Friststufe::DieseWoche, "Fällig in den nächsten 7 Tagen"),
    ] {
        let teil: Vec<_> = faellig.iter().filter(|(t, _)| t.stufe == stufe).collect();
        if teil.is_empty() {
            continue;
        }
        text.push_str(&format!(
            "\n{titel}\n{}\n",
            "-".repeat(titel.chars().count())
        ));
        for (t, vertreten) in teil {
            text.push_str(&format!(
                "{}  {} · {} · {}{}\n",
                t.datum.format("%d.%m.%Y"),
                t.projekt,
                t.bezug,
                t.titel,
                vertreten
                    .as_ref()
                    .map(|v| format!(" (Vertretung für {v})"))
                    .unwrap_or_default()
            ));
        }
    }
    text.push_str(
        "\nDie Termine bleiben bis zur Erledigung im Büro-Cockpit. Diese Nachricht abbestellen: BauPlanDESK → Startseite → Tägliche E-Mail.\n",
    );
    Some((betreff, text))
}

impl CoreServices {
    async fn tagesmail_lesen(&self, id: BenutzerId) -> CoreResult<Tagesmaileinstellung> {
        match self.repos.lizenz.einstellung(&schluessel(id)).await? {
            Some(t) if !t.is_empty() => {
                serde_json::from_str(&t).map_err(|e| CoreError::InvalidInput(e.to_string()))
            }
            _ => Ok(Tagesmaileinstellung::default()),
        }
    }
    pub async fn tagesmail_einstellung(&self, wer: &Benutzer) -> CoreResult<Tagesmaileinstellung> {
        let wer = self.angemeldeter_benutzer(wer).await?;
        self.tagesmail_lesen(wer.id).await
    }
    pub async fn tagesmail_setzen(
        &self,
        wer: &Benutzer,
        e: Tagesmaileinstellung,
    ) -> CoreResult<Tagesmaileinstellung> {
        let wer = self.angemeldeter_benutzer(wer).await?;
        let e = Tagesmaileinstellung {
            adresse: e.adresse.trim().to_string(),
            ..e
        };
        if e.aktiv && !adresse_gueltig(&e.adresse) {
            return Err(CoreError::InvalidInput(
                "Bitte eine gültige E-Mail-Adresse angeben".into(),
            ));
        }
        if let Some(v) = e.vertretung {
            if v == wer.id {
                return Err(CoreError::InvalidInput(
                    "Die Vertretung muss eine andere Person sein".into(),
                ));
            }
            if !self.repos.benutzer.get(v).await?.aktiv {
                return Err(CoreError::InvalidInput("Die Vertretung ist inaktiv".into()));
            }
        }
        self.repos
            .lizenz
            .einstellung_setzen(
                &schluessel(wer.id),
                &serde_json::to_string(&e).map_err(|x| CoreError::InvalidInput(x.to_string()))?,
            )
            .await?;
        Ok(e)
    }
    /// Beansprucht den Versand für `heute`; `false`, wenn er schon erfolgt ist
    /// oder eine andere Instanz ihn übernommen hat.
    pub async fn tagesmail_beanspruchen(&self, heute: NaiveDate) -> CoreResult<bool> {
        let bisher = self.repos.lizenz.einstellung(VERSANDTAG).await?;
        let tag = heute.to_string();
        if bisher.as_deref() == Some(tag.as_str()) {
            return Ok(false);
        }
        self.repos
            .lizenz
            .einstellung_vergleichen(VERSANDTAG, bisher.as_deref(), &tag)
            .await
    }
    /// Nachrichten des Tages für alle aktiven Empfänger.
    pub async fn tagesmails(&self, heute: NaiveDate) -> CoreResult<Vec<Tagesmail>> {
        let benutzer: Vec<Benutzer> = self
            .repos
            .benutzer
            .list()
            .await?
            .into_iter()
            .filter(|b| b.aktiv)
            .collect();
        let mut einstellungen = Vec::new();
        for b in &benutzer {
            einstellungen.push((b.clone(), self.tagesmail_lesen(b.id).await?));
        }
        let mut r = Vec::new();
        for (b, e) in &einstellungen {
            if !e.aktiv || e.abwesend(heute) || !adresse_gueltig(&e.adresse) {
                continue;
            }
            let mut termine: Vec<(CockpitTermin, Option<String>)> = self
                .buero_cockpit(b, heute)
                .await?
                .termine
                .into_iter()
                .filter(|t| e.umfang == Aboumfang::Alle || t.verantwortlich == Some(b.id.0))
                .map(|t| (t, None))
                .collect();
            // Vertretung abwesender Kollegen.
            for (v, ve) in &einstellungen {
                if v.id == b.id || ve.vertretung != Some(b.id) || !ve.abwesend(heute) {
                    continue;
                }
                for t in self.buero_cockpit(v, heute).await?.termine {
                    let doppelt = termine.iter().any(|(x, _)| x == &t);
                    if t.verantwortlich == Some(v.id.0) && !doppelt {
                        termine.push((t, Some(v.anzeigename.clone())));
                    }
                }
            }
            if let Some((betreff, text)) = zusammenfassen(&b.anzeigename, heute, &termine) {
                r.push(Tagesmail {
                    empfaenger: b.id,
                    adresse: e.adresse.clone(),
                    betreff,
                    text,
                });
            }
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::ProjectId, projektuebersicht::UebersichtsZiel};
    use uuid::Uuid;

    fn termin(datum: &str, stufe: Friststufe) -> CockpitTermin {
        let d: NaiveDate = datum.parse().unwrap();
        CockpitTermin {
            projekt_id: ProjectId(Uuid::nil()),
            projekt: "EFH Muster".into(),
            bezug: "Bauantrag".into(),
            art: None,
            art_bezeichnung: "Aufgabe".into(),
            titel: "Statik nachreichen".into(),
            datum: d,
            faellig_ab: d,
            verantwortlich: None,
            hinweis: String::new(),
            stufe,
            ziel: UebersichtsZiel {
                bereich: "aufgaben".into(),
                objekt: None,
                register: None,
                eintrag: None,
            },
        }
    }

    #[test]
    fn nur_faellige_termine_und_vertretung_markiert() {
        let heute: NaiveDate = "2026-11-02".parse().unwrap();
        assert!(
            zusammenfassen(
                "A",
                heute,
                &[(termin("2026-12-20", Friststufe::Spaeter), None)]
            )
            .is_none()
        );
        let (betreff, text) = zusammenfassen(
            "Anna",
            heute,
            &[
                (termin("2026-11-05", Friststufe::DieseWoche), None),
                (
                    termin("2026-10-30", Friststufe::Ueberfaellig),
                    Some("Bernd".into()),
                ),
                (termin("2026-11-20", Friststufe::Monat), None),
            ],
        )
        .unwrap();
        assert_eq!(
            betreff,
            "BauPlanDESK: 1 überfällig, 1 diese Woche (02.11.2026)"
        );
        assert!(text.starts_with("Guten Morgen Anna"));
        let u = text.find("Überfällig").unwrap();
        let w = text.find("Fällig in den nächsten 7 Tagen").unwrap();
        assert!(u < w);
        assert!(text.contains(
            "30.10.2026  EFH Muster · Bauantrag · Statik nachreichen (Vertretung für Bernd)"
        ));
        assert!(!text.contains("20.11.2026"));
    }

    #[test]
    fn adressen() {
        assert!(adresse_gueltig("planung@buero-muster.de"));
        for a in [
            "",
            "ohne-at.de",
            "a@b",
            "a b@c.de",
            "a@b.de,c@d.de",
            "a@.de",
        ] {
            assert!(!adresse_gueltig(a), "{a}");
        }
    }
}
