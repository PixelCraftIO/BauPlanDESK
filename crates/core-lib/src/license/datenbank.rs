use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, NaiveDate, Utc};

use super::{
    Arbeitsplatzbelegung, LicenseProvider, Lizenz, Lizenzpruefer, Lizenzstatus, Lizenzzustand,
};
use crate::error::{CoreError, CoreResult};
use crate::repository::LizenzRepository;

const LIZENZ: &str = "lizenz";
const TESTBEGINN: &str = "testbeginn";

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
        }
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

    /// Lizenz bzw. Testversion, noch ohne Arbeitsplätze.
    async fn grundzustand(&self) -> CoreResult<(Lizenzzustand, Option<Lizenz>)> {
        let heute = Utc::now().date_naive();
        if let Some(text) = self.repo.einstellung(LIZENZ).await? {
            return Ok(match self.pruefer.pruefen(text.as_bytes()) {
                Ok(l) => match l.gueltig_bis {
                    Some(bis) if bis < heute => (Lizenzzustand::Abgelaufen { seit: bis }, Some(l)),
                    _ => (Lizenzzustand::Lizenziert, Some(l)),
                },
                Err(e) => (
                    Lizenzzustand::Ungueltig {
                        grund: e.to_string(),
                    },
                    None,
                ),
            });
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
        ))
    }

    async fn status_berechnen(&self) -> CoreResult<Lizenzstatus> {
        let (mut zustand, lizenz) = self.grundzustand().await?;
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
        let meldung = meldung(&zustand, lizenz.as_ref(), max, &arbeitsplaetze);
        Ok(Lizenzstatus {
            zustand,
            schreiben_erlaubt,
            meldung,
            lizenz,
            arbeitsplaetze_max: max,
            arbeitsplaetze,
            eigener_arbeitsplatz: eigener,
        })
    }
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
) -> String {
    const NUR_LESEN: &str =
        "Projekte und Dokumente lassen sich weiter ansehen und exportieren, aber nicht ändern.";
    match zustand {
        Lizenzzustand::Lizenziert => match lizenz {
            Some(l) => format!(
                "Lizenziert für {} ({}, {}).",
                l.lizenznehmer,
                l.edition.name(),
                plaetze(l.arbeitsplaetze)
            ),
            None => "Lizenziert.".into(),
        },
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
        let (zustand, lizenz) = self.grundzustand().await?;
        let gueltig = matches!(
            zustand,
            Lizenzzustand::Lizenziert | Lizenzzustand::Testversion { .. }
        );
        let erhalten = if gueltig {
            let max = lizenz.map_or(self.testarbeitsplaetze, |l| l.arbeitsplaetze);
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
}
