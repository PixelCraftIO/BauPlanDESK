use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};

use super::ticket::{AbmeldeAnfrage, DienstFehler, Lizenzdienst, Ticket, TicketAnfrage};
use super::{
    AboStatus, Arbeitsplatzbelegung, LicenseProvider, Lizenz, Lizenzpruefer, Lizenzstatus,
    Lizenzzustand,
};
use crate::error::{CoreError, CoreResult};
use crate::repository::LizenzRepository;

const LIZENZ: &str = "lizenz";
const TESTBEGINN: &str = "testbeginn";
/// Abo-Lizenzschlüssel (gemeinsam für alle Arbeitsplätze).
const ABO_SCHLUESSEL: &str = "abo_schluessel";
/// Lizenz-ID, zu der der Schlüssel gehört. Tickets anderer Lizenzen zählen nicht.
const ABO_LIZENZ: &str = "abo_lizenz_id";

/// Verbindung dieses Rechners zum Lizenzserver (nur Desktop-App).
pub struct AboAnbindung {
    pub dienst: Arc<dyn Lizenzdienst>,
    /// Kennung dieses Rechners, an die der Server das Ticket bindet.
    pub machine_id: String,
    pub rechnername: String,
    pub app_version: String,
    /// Hier liegt das Ticket dieses Rechners (lokal, nicht in der
    /// gemeinsamen Datenbank, denn es gilt nur für diesen Rechner).
    pub ticket_datei: PathBuf,
}

#[derive(Default)]
struct AboLaufzeit {
    letzter_versuch: Option<DateTime<Utc>>,
    /// Letzte Ablehnung durch den Server, für die Meldung.
    ablehnung: Option<String>,
    /// In dieser Sitzung abgemeldet: nicht von selbst neu aktivieren.
    abgemeldet: bool,
}

/// Lizenz in der gemeinsamen Datenbank, siehe [`super`].
pub struct DatenbankLizenz {
    repo: Arc<dyn LizenzRepository>,
    pruefer: Lizenzpruefer,
    /// Desktop: ohne belegten Arbeitsplatz keine Änderungen.
    arbeitsplatzpflicht: bool,
    eigener: Mutex<Option<String>>,
    pub testtage: i64,
    pub testarbeitsplaetze: u32,
    /// Ohne Meldung gilt ein Arbeitsplatz nach dieser Zeit als frei
    /// (abgestürzter Rechner, gezogenes Netzwerkkabel).
    pub ablauf: Duration,
    abo: Option<AboAnbindung>,
    abo_laufzeit: Mutex<AboLaufzeit>,
    /// Ein Ticket, das älter ist, wird im Hintergrund erneuert.
    pub erneuern_nach: Duration,
    /// Mindestabstand zwischen zwei Versuchen, wenn der Server nicht
    /// erreichbar ist.
    pub erneut_versuchen_nach: Duration,
}

impl DatenbankLizenz {
    pub fn new(repo: Arc<dyn LizenzRepository>, pruefer: Lizenzpruefer) -> Self {
        Self {
            repo,
            pruefer,
            arbeitsplatzpflicht: false,
            eigener: Mutex::new(None),
            testtage: 30,
            testarbeitsplaetze: 3,
            ablauf: Duration::minutes(15),
            abo: None,
            abo_laufzeit: Mutex::default(),
            erneuern_nach: Duration::days(1),
            erneut_versuchen_nach: Duration::minutes(1),
        }
    }

    /// Abo-Lizenzen über den Lizenzserver ermöglichen (Desktop-App).
    pub fn mit_abo(mut self, abo: AboAnbindung) -> Self {
        self.abo = Some(abo);
        self
    }

    /// Änderungen nur, wenn dieser Prozess einen Arbeitsplatz belegt hat
    /// (Desktop-App).
    pub fn mit_arbeitsplatzpflicht(mut self) -> Self {
        self.arbeitsplatzpflicht = true;
        self
    }

    fn eigener(&self) -> Option<String> {
        self.eigener.lock().expect("Mutex").clone()
    }

    fn laufzeit(&self) -> std::sync::MutexGuard<'_, AboLaufzeit> {
        self.abo_laufzeit.lock().expect("Mutex")
    }

    /// Wert einer Einstellung; leer gilt als nicht gesetzt.
    async fn einstellung(&self, name: &str) -> CoreResult<Option<String>> {
        Ok(self
            .repo
            .einstellung(name)
            .await?
            .filter(|w| !w.trim().is_empty()))
    }

    /// Lizenz bzw. Testversion, noch ohne Arbeitsplätze.
    async fn grundzustand(&self) -> CoreResult<(Lizenzzustand, Option<Lizenz>, Option<AboStatus>)> {
        let heute = Utc::now().date_naive();
        if let Some(text) = self.einstellung(LIZENZ).await? {
            return Ok(match self.pruefer.pruefen(text.as_bytes()) {
                Ok(l) => match l.gueltig_bis {
                    Some(bis) if bis < heute => {
                        (Lizenzzustand::Abgelaufen { seit: bis }, Some(l), None)
                    }
                    _ => (Lizenzzustand::Lizenziert, Some(l), None),
                },
                Err(e) => (
                    Lizenzzustand::Ungueltig {
                        grund: e.to_string(),
                    },
                    None,
                    None,
                ),
            });
        }
        if let Some(schluessel) = self.einstellung(ABO_SCHLUESSEL).await? {
            return self.abo_zustand(&schluessel).await;
        }
        let beginn = self
            .repo
            .einstellung_vorbelegen(TESTBEGINN, &heute.to_string())
            .await?;
        let beginn: NaiveDate = beginn.parse().unwrap_or(heute);
        let uebrig = self.testtage - (heute - beginn).num_days();
        Ok((
            if uebrig > 0 {
                Lizenzzustand::Testversion {
                    tage_uebrig: uebrig,
                }
            } else {
                Lizenzzustand::TestversionAbgelaufen
            },
            None,
            None,
        ))
    }

    async fn abo_zustand(
        &self,
        schluessel: &str,
    ) -> CoreResult<(Lizenzzustand, Option<Lizenz>, Option<AboStatus>)> {
        let mut abo_status = AboStatus {
            lizenzschluessel: maskieren(schluessel),
            bestaetigt_um: None,
            offline_gueltig_bis: None,
        };
        let Some(abo) = &self.abo else {
            return Ok((
                Lizenzzustand::Ungueltig {
                    grund: "Abo-Lizenzen bestätigt nur die Desktop-App beim Lizenzserver.".into(),
                },
                None,
                Some(abo_status),
            ));
        };
        let jetzt = Utc::now();
        let Some(ticket) = self.ticket_lesen(abo).await? else {
            let grund = {
                let l = self.laufzeit();
                if l.abgemeldet {
                    "Dieser Rechner wurde beim Lizenzserver abgemeldet.".to_string()
                } else {
                    l.ablehnung.clone().unwrap_or_else(|| {
                        "Dieser Rechner ist noch nicht beim Lizenzserver freigeschaltet. BauProjektIO versucht es automatisch, sobald eine Internetverbindung besteht.".into()
                    })
                }
            };
            return Ok((
                Lizenzzustand::NichtBestaetigt { grund },
                None,
                Some(abo_status),
            ));
        };
        abo_status.bestaetigt_um = Some(ticket.ausgestellt_um);
        abo_status.offline_gueltig_bis = Some(ticket.gueltig_bis);
        let zustand = if !ticket.ist_gueltig(jetzt) {
            Lizenzzustand::BestaetigungAbgelaufen {
                seit: ticket.gueltig_bis,
            }
        } else {
            match ticket.lizenz.gueltig_bis {
                Some(bis) if bis < jetzt.date_naive() => Lizenzzustand::Abgelaufen { seit: bis },
                _ => Lizenzzustand::Lizenziert,
            }
        };
        Ok((zustand, Some(ticket.lizenz), Some(abo_status)))
    }

    /// Das gespeicherte Ticket dieses Rechners, wenn es echt ist, zu diesem
    /// Rechner und zur eingetragenen Lizenz gehört (abgelaufen oder nicht).
    async fn ticket_lesen(&self, abo: &AboAnbindung) -> CoreResult<Option<Ticket>> {
        let inhalt = match std::fs::read(&abo.ticket_datei) {
            Ok(i) => i,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                tracing::warn!(datei = %abo.ticket_datei.display(), fehler = %e, "Lizenzbestätigung nicht lesbar");
                return Ok(None);
            }
        };
        let ticket = match self.pruefer.ticket_pruefen(&inhalt, &abo.machine_id) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(fehler = %e, "Lizenzbestätigung verworfen");
                return Ok(None);
            }
        };
        let lizenz_id = self.einstellung(ABO_LIZENZ).await?;
        Ok(
            (lizenz_id.as_deref() == Some(ticket.lizenz.lizenz_id.to_string().as_str()))
                .then_some(ticket),
        )
    }

    fn anfrage(abo: &AboAnbindung, schluessel: &str) -> TicketAnfrage {
        TicketAnfrage {
            lizenzschluessel: schluessel.to_string(),
            machine_id: abo.machine_id.clone(),
            rechnername: abo.rechnername.clone(),
            app_version: abo.app_version.clone(),
        }
    }

    /// Holt beim Server ein Ticket und prüft es; speichert noch nichts.
    async fn ticket_holen(
        &self,
        abo: &AboAnbindung,
        schluessel: &str,
    ) -> Result<(String, Ticket), DienstFehler> {
        let text = abo.dienst.ticket(&Self::anfrage(abo, schluessel)).await?;
        let ticket = self
            .pruefer
            .ticket_pruefen(text.as_bytes(), &abo.machine_id)
            .map_err(|e| DienstFehler::Abgelehnt(e.to_string()))?;
        Ok((text, ticket))
    }

    fn ticket_speichern(abo: &AboAnbindung, text: &str) -> CoreResult<()> {
        let schreiben = || -> std::io::Result<()> {
            if let Some(dir) = abo.ticket_datei.parent() {
                std::fs::create_dir_all(dir)?;
            }
            // Erst vollständig schreiben, dann umbenennen: Ein Absturz
            // mittendrin hinterlässt kein halbes Ticket.
            let tmp = abo.ticket_datei.with_extension("tmp");
            std::fs::write(&tmp, text)?;
            std::fs::rename(&tmp, &abo.ticket_datei)
        };
        schreiben().map_err(|e| {
            CoreError::License(format!(
                "Die Lizenzbestätigung konnte nicht gespeichert werden ({}): {e}",
                abo.ticket_datei.display()
            ))
        })
    }

    fn ticket_loeschen(abo: &AboAnbindung) {
        if let Err(e) = std::fs::remove_file(&abo.ticket_datei)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(fehler = %e, "Lizenzbestätigung nicht gelöscht");
        }
    }

    /// Ticket im Hintergrund erneuern, wenn es fehlt, abgelaufen oder älter
    /// als [`Self::erneuern_nach`] ist. Ist der Server nicht erreichbar, gilt das
    /// vorhandene Ticket weiter; lehnt er ab, wird es verworfen.
    /// `erzwingen`: sofort, und ein nicht erreichbarer Server ist ein Fehler.
    async fn erneuern(&self, erzwingen: bool) -> CoreResult<()> {
        let Some(abo) = &self.abo else {
            return Ok(());
        };
        if self.einstellung(LIZENZ).await?.is_some() {
            return Ok(());
        }
        let Some(schluessel) = self.einstellung(ABO_SCHLUESSEL).await? else {
            return Ok(());
        };
        let jetzt = Utc::now();
        if !erzwingen {
            let warten = {
                let l = self.laufzeit();
                l.abgemeldet
                    || l.letzter_versuch
                        .is_some_and(|t| jetzt - t < self.erneut_versuchen_nach)
            };
            if warten {
                return Ok(());
            }
            if let Some(t) = self.ticket_lesen(abo).await?
                && t.ist_gueltig(jetzt)
                && jetzt - t.ausgestellt_um < self.erneuern_nach
            {
                return Ok(());
            }
        }
        self.laufzeit().letzter_versuch = Some(jetzt);
        let lizenz_id = self.einstellung(ABO_LIZENZ).await?;
        let ergebnis = match self.ticket_holen(abo, &schluessel).await {
            Ok((text, t)) if lizenz_id.as_deref() == Some(t.lizenz.lizenz_id.to_string().as_str()) => {
                Self::ticket_speichern(abo, &text)?;
                let mut l = self.laufzeit();
                l.ablehnung = None;
                l.abgemeldet = false;
                tracing::info!(gueltig_bis = %t.gueltig_bis, "Lizenz beim Lizenzserver bestätigt");
                Ok(())
            }
            Ok(_) => Err(DienstFehler::Abgelehnt(
                "Der Lizenzserver hat eine Bestätigung für eine andere Lizenz geschickt. Bitte geben Sie den Lizenzschlüssel neu ein.".into(),
            )),
            Err(e) => Err(e),
        };
        match ergebnis {
            Ok(()) => Ok(()),
            Err(DienstFehler::Abgelehnt(m)) => {
                tracing::warn!(grund = %m, "Lizenzserver lehnt diesen Rechner ab");
                Self::ticket_loeschen(abo);
                self.laufzeit().ablehnung = Some(m.clone());
                if erzwingen {
                    Err(CoreError::License(m))
                } else {
                    Ok(())
                }
            }
            Err(e @ DienstFehler::NichtErreichbar(_)) => {
                tracing::info!(fehler = %e, "Lizenz nicht bestätigt, nächster Versuch später");
                if erzwingen {
                    Err(CoreError::License(e.to_string()))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn status_berechnen(&self) -> CoreResult<Lizenzstatus> {
        let (mut zustand, lizenz, abo) = self.grundzustand().await?;
        let max = lizenz
            .as_ref()
            .map_or(self.testarbeitsplaetze, |l| l.arbeitsplaetze);
        let arbeitsplaetze = self.repo.arbeitsplaetze(Utc::now() - self.ablauf).await?;
        let eigener = self.eigener();
        let gueltig = matches!(
            zustand,
            Lizenzzustand::Lizenziert | Lizenzzustand::Testversion { .. }
        );
        if gueltig && self.arbeitsplatzpflicht && eigener.is_none() {
            zustand = Lizenzzustand::KeinArbeitsplatz;
        }
        let schreiben_erlaubt = matches!(
            zustand,
            Lizenzzustand::Lizenziert | Lizenzzustand::Testversion { .. }
        );
        let meldung = meldung(
            &zustand,
            lizenz.as_ref(),
            max,
            &arbeitsplaetze,
            abo.as_ref(),
        );
        Ok(Lizenzstatus {
            zustand,
            schreiben_erlaubt,
            meldung,
            lizenz,
            arbeitsplaetze_max: max,
            arbeitsplaetze,
            eigener_arbeitsplatz: eigener,
            abo,
        })
    }
}

/// `BPIO-ABCDE-FGHJK-…-VWXYZ` → `BPIO-•••••-…-VWXYZ`: Wer nur die Anzeige
/// sieht, kann den Schlüssel nicht abschreiben.
fn maskieren(schluessel: &str) -> String {
    let teile: Vec<&str> = schluessel.split('-').collect();
    match teile.as_slice() {
        [erster, mitte @ .., letzter] if !mitte.is_empty() => {
            let mut v = vec![(*erster).to_string()];
            v.extend(mitte.iter().map(|t| "•".repeat(t.chars().count())));
            v.push((*letzter).to_string());
            v.join("-")
        }
        _ => {
            let n = schluessel.chars().count();
            let sichtbar: String = schluessel.chars().skip(n.saturating_sub(4)).collect();
            format!("{}{sichtbar}", "•".repeat(n.saturating_sub(4)))
        }
    }
}

/// Eingegebenen Schlüssel vereinheitlichen (Leerzeichen, Kleinbuchstaben).
fn schluessel_normalisieren(eingabe: &str) -> String {
    eingabe
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

fn plaetze(n: u32) -> String {
    if n == 1 {
        "1 Arbeitsplatz".into()
    } else {
        format!("{n} Arbeitsplätze")
    }
}

fn meldung(
    zustand: &Lizenzzustand,
    lizenz: Option<&Lizenz>,
    max: u32,
    belegt: &[Arbeitsplatzbelegung],
    abo: Option<&AboStatus>,
) -> String {
    const NUR_LESEN: &str =
        "Projekte und Dokumente lassen sich weiter ansehen und exportieren, aber nicht ändern.";
    let datum = |t: &DateTime<Utc>| t.format("%d.%m.%Y").to_string();
    match zustand {
        Lizenzzustand::Lizenziert => {
            let mut m = match lizenz {
                Some(l) => format!(
                    "Lizenziert für {} ({}, {}).",
                    l.lizenznehmer,
                    l.edition.name(),
                    plaetze(l.arbeitsplaetze)
                ),
                None => "Lizenziert.".into(),
            };
            if let Some(bis) = abo.and_then(|a| a.offline_gueltig_bis) {
                m.push_str(&format!(" Auch ohne Internet nutzbar bis {}.", datum(&bis)));
            }
            m
        }
        Lizenzzustand::NichtBestaetigt { grund } => format!("{grund} {NUR_LESEN}"),
        Lizenzzustand::BestaetigungAbgelaufen { seit } => format!(
            "Die Lizenz konnte seit dem {} nicht beim Lizenzserver bestätigt werden. Bitte verbinden Sie diesen Rechner mit dem Internet. {NUR_LESEN}",
            datum(seit)
        ),
        Lizenzzustand::Testversion { tage_uebrig: 1 } => "Testversion: noch heute nutzbar.".into(),
        Lizenzzustand::Testversion { tage_uebrig } => {
            format!("Testversion: noch {tage_uebrig} Tage nutzbar.")
        }
        Lizenzzustand::TestversionAbgelaufen => {
            format!(
                "Die Testversion ist abgelaufen. Bitte spielen Sie eine Lizenz ein. {NUR_LESEN}"
            )
        }
        Lizenzzustand::Abgelaufen { seit } => format!(
            "Die Lizenz ist am {} abgelaufen. {NUR_LESEN}",
            seit.format("%d.%m.%Y")
        ),
        Lizenzzustand::Ungueltig { grund } => {
            format!("Die eingespielte Lizenz ist ungültig: {grund} {NUR_LESEN}")
        }
        Lizenzzustand::KeinArbeitsplatz => {
            let namen: Vec<_> = belegt
                .iter()
                .map(|b| format!("{} ({})", b.arbeitsplatz, b.benutzer_name))
                .collect();
            let alle = if max == 1 {
                "Der Arbeitsplatz der Lizenz ist".to_string()
            } else {
                format!("Alle {max} Arbeitsplätze der Lizenz sind")
            };
            format!(
                "{alle} belegt: {}. {NUR_LESEN} Sobald ein Platz frei wird, schaltet BauProjektIO das Bearbeiten wieder frei.",
                namen.join(", ")
            )
        }
    }
}

#[async_trait]
impl LicenseProvider for DatenbankLizenz {
    async fn status(&self) -> CoreResult<Lizenzstatus> {
        self.status_berechnen().await
    }

    async fn einspielen(&self, datei: &[u8]) -> CoreResult<Lizenzstatus> {
        let lizenz = self.pruefer.pruefen(datei)?;
        // Eine Lizenzdatei ersetzt einen Abo-Schlüssel.
        self.repo.einstellung_setzen(ABO_SCHLUESSEL, "").await?;
        self.repo.einstellung_setzen(ABO_LIZENZ, "").await?;
        if let Some(bis) = lizenz.gueltig_bis
            && bis < Utc::now().date_naive()
        {
            return Err(CoreError::License(format!(
                "Diese Lizenz ist am {} abgelaufen.",
                bis.format("%d.%m.%Y")
            )));
        }
        let text = std::str::from_utf8(datei)
            .map_err(|_| CoreError::License("Die Lizenzdatei ist beschädigt.".into()))?;
        self.repo.einstellung_setzen(LIZENZ, text).await?;
        self.status_berechnen().await
    }

    async fn arbeitsplatz_belegen(
        &self,
        arbeitsplatz: &str,
        benutzer_name: &str,
    ) -> CoreResult<Lizenzstatus> {
        self.erneuern(false).await?;
        let (zustand, lizenz, abo) = self.grundzustand().await?;
        let gueltig = matches!(
            zustand,
            Lizenzzustand::Lizenziert | Lizenzzustand::Testversion { .. }
        );
        let erhalten = if gueltig {
            // Bei Abo-Lizenzen zählt der Lizenzserver die Plätze (je Rechner
            // ein Ticket); die Datenbank zeigt nur, wer gerade arbeitet.
            let max = match (&abo, lizenz) {
                (Some(_), _) => u32::MAX,
                (None, l) => l.map_or(self.testarbeitsplaetze, |l| l.arbeitsplaetze),
            };
            let jetzt = Utc::now();
            self.repo
                .arbeitsplatz_belegen(
                    Arbeitsplatzbelegung {
                        arbeitsplatz: arbeitsplatz.to_string(),
                        benutzer_name: benutzer_name.to_string(),
                        belegt_seit: jetzt,
                        letzte_meldung: jetzt,
                    },
                    max,
                    jetzt - self.ablauf,
                )
                .await?
        } else {
            false
        };
        *self.eigener.lock().expect("Mutex") = erhalten.then(|| arbeitsplatz.to_string());
        self.status_berechnen().await
    }

    async fn arbeitsplatz_freigeben(&self, arbeitsplatz: &str) -> CoreResult<()> {
        {
            let mut eigener = self.eigener.lock().expect("Mutex");
            if eigener.as_deref() == Some(arbeitsplatz) {
                *eigener = None;
            }
        }
        self.repo.arbeitsplatz_freigeben(arbeitsplatz).await
    }

    async fn abo_aktivieren(&self, lizenzschluessel: &str) -> CoreResult<Lizenzstatus> {
        let Some(abo) = self
            .abo
            .as_ref()
            .filter(|_| self.pruefer.hat_ticket_schluessel())
        else {
            return Err(CoreError::License(
                "Diese Version von BauProjektIO kennt keinen Lizenzserver. Bitte verwenden Sie eine Lizenzdatei.".into(),
            ));
        };
        let schluessel = schluessel_normalisieren(lizenzschluessel);
        if schluessel.is_empty() {
            return Err(CoreError::InvalidInput(
                "Bitte einen Lizenzschlüssel eingeben.".into(),
            ));
        }
        self.laufzeit().letzter_versuch = Some(Utc::now());
        let (text, ticket) = self
            .ticket_holen(abo, &schluessel)
            .await
            .map_err(|e| CoreError::License(e.to_string()))?;
        Self::ticket_speichern(abo, &text)?;
        self.repo
            .einstellung_setzen(ABO_SCHLUESSEL, &schluessel)
            .await?;
        self.repo
            .einstellung_setzen(ABO_LIZENZ, &ticket.lizenz.lizenz_id.to_string())
            .await?;
        // Ein Abo ersetzt eine eingespielte Lizenzdatei.
        self.repo.einstellung_setzen(LIZENZ, "").await?;
        {
            let mut l = self.laufzeit();
            l.ablehnung = None;
            l.abgemeldet = false;
        }
        self.status_berechnen().await
    }

    async fn abo_bestaetigen(&self) -> CoreResult<Lizenzstatus> {
        self.erneuern(true).await?;
        self.status_berechnen().await
    }

    async fn abo_abmelden(&self) -> CoreResult<Lizenzstatus> {
        let (Some(abo), Some(schluessel)) = (&self.abo, self.einstellung(ABO_SCHLUESSEL).await?)
        else {
            return Err(CoreError::License(
                "Hier ist keine Abo-Lizenz eingetragen.".into(),
            ));
        };
        abo.dienst
            .abmelden(&AbmeldeAnfrage {
                lizenzschluessel: schluessel,
                machine_id: abo.machine_id.clone(),
            })
            .await
            .map_err(|e| CoreError::License(e.to_string()))?;
        Self::ticket_loeschen(abo);
        self.laufzeit().abgemeldet = true;
        self.status_berechnen().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schluessel_anzeige() {
        assert_eq!(
            maskieren("BPIO-ABCDE-FGHJK-LMNPQ-RSTVW-XYZ23"),
            "BPIO-•••••-•••••-•••••-•••••-XYZ23"
        );
        assert_eq!(maskieren("ABCDEFGH"), "••••EFGH");
        assert_eq!(
            schluessel_normalisieren(" bpio-abcde -fghjk\n"),
            "BPIO-ABCDE-FGHJK"
        );
    }
}
