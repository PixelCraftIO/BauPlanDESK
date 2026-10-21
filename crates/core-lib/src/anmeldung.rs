//! Anmeldung mit Anmeldename und Kennwort (Netzwerkbetrieb).
//!
//! Vertrag für die Adapter (Desktop):
//!
//! - [`CoreServices::anmelden_mit_kennwort`] prüft Anmeldename und Kennwort,
//!   zählt Fehlversuche und sperrt nach [`kennwort::FREIE_FEHLVERSUCHE`]
//!   Fehleingaben mit wachsender Wartezeit. Ergebnis ist eine [`Anmeldung`];
//!   mit `kennwort_aendern = true` darf der Adapter erst nach
//!   [`CoreServices::kennwort_aendern`] weiterarbeiten.
//! - Übergang für Bestandsdaten ohne Kennwörter: Solange kein aktiver
//!   Administrator ein Kennwort hat, darf sich ein Administrator einmalig über
//!   den Betriebssystem-Anmeldenamen anmelden
//!   ([`CoreServices::anmelden_uebergang`]) und muss sofort ein Kennwort setzen.
//!   Auf einer leeren Datenbank wird so der erste Administrator angelegt.
//! - Administratoren vergeben Erstkennwörter
//!   ([`CoreServices::erstkennwort_vergeben`]); das hebt auch eine Sperre auf.
//! - Mit [`CoreServices::mit_kennwortpflicht`] lehnt jede Rechteprüfung Konten
//!   ab, die noch kein eigenes Kennwort gesetzt haben.
//!
//! Die Einzelplatzversion meldet weiter über [`CoreServices::anmelden`]
//! (Betriebssystem-Benutzer) ohne Kennwort an.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::CoreServices;
use crate::benutzer::{self, Benutzer, BenutzerId, Kennwortstatus, Recht, Rolle};
use crate::domain::Aktion;
use crate::error::{CoreError, CoreResult};
use crate::kennwort;
use crate::service::SYSTEM;

/// Ergebnis einer erfolgreichen Anmeldung.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anmeldung {
    pub benutzer: Benutzer,
    /// Erstkennwort oder Übergang: Vor jeder weiteren Arbeit ein eigenes
    /// Kennwort setzen.
    pub kennwort_aendern: bool,
}

/// Was der Anmeldedialog neben Anmeldename/Kennwort anbieten darf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anmeldeoptionen {
    /// Einmalige Anmeldung als `rechnerkonto` ohne Kennwort ist möglich.
    pub uebergang_moeglich: bool,
    /// Normalisierter Betriebssystem-Anmeldename dieses Arbeitsplatzes.
    pub rechnerkonto: String,
    /// Leere Datenbank: Der Übergang legt den ersten Administrator an.
    pub erster_administrator: bool,
}

const FALSCH: &str = "Anmeldename oder Kennwort ist falsch.";

enum Uebergang {
    Vorhanden(Benutzer),
    ErsterAdministrator,
}

impl CoreServices {
    /// Im Netzwerkbetrieb: Rechte nur für Konten mit eigenem Kennwort.
    pub fn mit_kennwortpflicht(mut self) -> Self {
        self.kennwortpflicht = true;
        self
    }

    /// Aktuellen Stand des handelnden Kontos aus der Datenbank lesen und
    /// prüfen, dass es angemeldet ist und (mit Kennwortpflicht) ein eigenes
    /// Kennwort hat. Grundlage aller Rechteprüfungen.
    pub(crate) async fn angemeldeter_benutzer(&self, wer: &Benutzer) -> CoreResult<Benutzer> {
        if !wer.ist_angemeldet() {
            return Err(CoreError::Forbidden(benutzer::NICHT_ANGEMELDET.into()));
        }
        let aktuell = self.repos.benutzer.get(wer.id).await?;
        if self.kennwortpflicht
            && aktuell.anmeldename != SYSTEM
            && aktuell.kennwort != Kennwortstatus::Gesetzt
        {
            return Err(CoreError::Forbidden(
                "Bitte setzen Sie zuerst ein eigenes Kennwort.".into(),
            ));
        }
        Ok(aktuell)
    }

    /// Optionen für den Anmeldedialog des Arbeitsplatzes `rechnerkonto`.
    pub async fn anmeldeoptionen(&self, rechnerkonto: &str) -> CoreResult<Anmeldeoptionen> {
        let name = benutzer::anmeldename_normalisieren(rechnerkonto).unwrap_or_default();
        let uebergang = if name.is_empty() {
            None
        } else {
            self.uebergang_pruefen(&name).await.ok()
        };
        Ok(Anmeldeoptionen {
            uebergang_moeglich: uebergang.is_some(),
            erster_administrator: matches!(uebergang, Some(Uebergang::ErsterAdministrator)),
            rechnerkonto: name,
        })
    }

    async fn uebergang_pruefen(&self, name: &str) -> CoreResult<Uebergang> {
        let alle = self.repos.benutzer.list().await?;
        let admins: Vec<&Benutzer> = alle
            .iter()
            .filter(|b| b.rolle == Rolle::Administrator && b.aktiv && b.anmeldename != SYSTEM)
            .collect();
        if admins.iter().any(|b| b.kennwort.vorhanden()) {
            return Err(CoreError::Forbidden(
                "Die Anmeldung über das Benutzerkonto des Rechners ist nicht mehr möglich: Ein Administrator hat bereits ein Kennwort. Bitte melden Sie sich mit Anmeldename und Kennwort an.".into(),
            ));
        }
        match alle.iter().find(|b| b.anmeldename == name) {
            Some(b) if b.aktiv && b.rolle == Rolle::Administrator => {
                Ok(Uebergang::Vorhanden(b.clone()))
            }
            _ if admins.is_empty() && !alle.iter().any(|b| b.anmeldename == name) => {
                Ok(Uebergang::ErsterAdministrator)
            }
            _ => Err(CoreError::Forbidden(format!(
                "Für „{name}“ ist kein Kennwort vergeben. Ohne Kennwort kann sich nur ein Administrator einmalig über das Benutzerkonto des Rechners anmelden. Bitte lassen Sie sich von einem Administrator ein Erstkennwort geben."
            ))),
        }
    }

    /// Einmalige Anmeldung eines Administrators über den
    /// Betriebssystem-Anmeldenamen, solange kein Administrator ein Kennwort
    /// hat. Das Ergebnis verlangt immer ein neues Kennwort.
    pub async fn anmelden_uebergang(
        &self,
        rechnerkonto: &str,
        anzeigename: &str,
    ) -> CoreResult<Anmeldung> {
        let name = benutzer::anmeldename_normalisieren(rechnerkonto)?;
        let b = match self.uebergang_pruefen(&name).await? {
            Uebergang::Vorhanden(b) => b,
            // Legt wie bisher den ersten Benutzer als Administrator an.
            Uebergang::ErsterAdministrator => self.anmelden(&name, anzeigename).await?,
        };
        self.repos
            .benutzer
            .anmeldung_vermerken(b.id, Utc::now())
            .await?;
        self.protokollieren(
            &b,
            Aktion::Angemeldet,
            None,
            None,
            format!(
                "{} ({}) über das Benutzerkonto des Rechners (einmaliger Übergang, Kennwort muss gesetzt werden)",
                b.anzeigename, b.anmeldename
            ),
        )
        .await?;
        Ok(Anmeldung {
            benutzer: self.repos.benutzer.get(b.id).await?,
            kennwort_aendern: true,
        })
    }

    /// Anmeldung mit Anmeldename (Groß-/Kleinschreibung egal) und Kennwort.
    pub async fn anmelden_mit_kennwort(
        &self,
        anmeldename: &str,
        kennwort: &str,
    ) -> CoreResult<Anmeldung> {
        let name = benutzer::anmeldename_normalisieren(anmeldename)
            .map_err(|_| CoreError::Forbidden(FALSCH.into()))?;
        let jetzt = Utc::now();
        let b = match self.repos.benutzer.by_anmeldename(&name).await? {
            Some(b) if name != SYSTEM => b,
            _ => {
                kennwort::blindpruefung(kennwort);
                let system = self.systembenutzer().await?;
                self.protokollieren(
                    &system,
                    Aktion::AnmeldungFehlgeschlagen,
                    None,
                    None,
                    format!("Unbekannter Anmeldename „{name}“"),
                )
                .await?;
                return Err(CoreError::Forbidden(FALSCH.into()));
            }
        };
        let daten = self.repos.benutzer.kennwortdaten(b.id).await?;
        if let Some(bis) = daten.gesperrt_bis
            && bis > jetzt
        {
            return Err(CoreError::Forbidden(format!(
                "Nach mehreren falschen Kennwörtern ist die Anmeldung für „{name}“ bis {} Uhr gesperrt. Ein Administrator kann ein neues Erstkennwort vergeben.",
                bis.with_timezone(&chrono::Local).format("%H:%M:%S")
            )));
        }
        let Some(hash) = daten.hash else {
            kennwort::blindpruefung(kennwort);
            self.protokollieren(
                &b,
                Aktion::AnmeldungFehlgeschlagen,
                None,
                None,
                format!(
                    "{} ({}): kein Kennwort vergeben",
                    b.anzeigename, b.anmeldename
                ),
            )
            .await?;
            return Err(CoreError::Forbidden(format!(
                "Für „{name}“ ist noch kein Kennwort vergeben. Bitte lassen Sie sich von einem Administrator ein Erstkennwort geben."
            )));
        };
        if !kennwort::pruefen(kennwort, &hash)? {
            let n = self.repos.benutzer.fehlversuch_zaehlen(b.id).await?;
            let bis = kennwort::gesperrt_bis(n, jetzt);
            if bis.is_some() {
                self.repos.benutzer.anmeldesperre_setzen(b.id, bis).await?;
            }
            self.protokollieren(
                &b,
                Aktion::AnmeldungFehlgeschlagen,
                None,
                None,
                format!(
                    "{} ({}): falsches Kennwort, {n}. Fehlversuch in Folge{}",
                    b.anzeigename,
                    b.anmeldename,
                    if bis.is_some() {
                        ", Anmeldung vorübergehend gesperrt"
                    } else {
                        ""
                    }
                ),
            )
            .await?;
            return Err(CoreError::Forbidden(match bis {
                Some(bis) => format!(
                    "{FALSCH} Nach {n} Fehlversuchen ist die nächste Anmeldung erst ab {} Uhr möglich.",
                    bis.with_timezone(&chrono::Local).format("%H:%M:%S")
                ),
                None => FALSCH.into(),
            }));
        }
        // Erst nach richtigem Kennwort verraten, dass das Konto deaktiviert ist.
        if !b.aktiv {
            self.protokollieren(
                &b,
                Aktion::AnmeldungFehlgeschlagen,
                None,
                None,
                format!("{} ({}): Konto deaktiviert", b.anzeigename, b.anmeldename),
            )
            .await?;
            return Err(CoreError::Forbidden(format!(
                "Das Benutzerkonto „{name}“ ist deaktiviert. Bitte wenden Sie sich an einen Administrator."
            )));
        }
        self.repos.benutzer.anmeldung_vermerken(b.id, jetzt).await?;
        let kennwort_aendern = b.kennwort == Kennwortstatus::Erstkennwort;
        self.protokollieren(
            &b,
            Aktion::Angemeldet,
            None,
            None,
            format!(
                "{} ({}) mit {}",
                b.anzeigename,
                b.anmeldename,
                if kennwort_aendern {
                    "Erstkennwort"
                } else {
                    "Kennwort"
                }
            ),
        )
        .await?;
        Ok(Anmeldung {
            benutzer: self.repos.benutzer.get(b.id).await?,
            kennwort_aendern,
        })
    }

    /// Eigenes Kennwort ändern. Ist ein Kennwort gespeichert (auch ein
    /// Erstkennwort), muss `bisher` stimmen: Setzt ein Administrator das
    /// Kennwort zurück, kann eine ältere Sitzung so kein eigenes mehr setzen.
    /// Ohne gespeichertes Kennwort (Übergang) genügt die Anmeldung.
    pub async fn kennwort_aendern(
        &self,
        wer: &Benutzer,
        bisher: Option<&str>,
        neu: &str,
    ) -> CoreResult<Benutzer> {
        if !wer.ist_angemeldet() {
            return Err(CoreError::Forbidden(benutzer::NICHT_ANGEMELDET.into()));
        }
        let b = self.repos.benutzer.get(wer.id).await?;
        if !b.aktiv || b.anmeldename == SYSTEM {
            return Err(CoreError::Forbidden(format!(
                "Für das Konto „{}“ kann kein Kennwort gesetzt werden.",
                b.anmeldename
            )));
        }
        if b.kennwort.vorhanden() {
            let hash = self
                .repos
                .benutzer
                .kennwortdaten(b.id)
                .await?
                .hash
                .ok_or_else(|| CoreError::Database("Kennwort-Hash fehlt".into()))?;
            if !kennwort::pruefen(bisher.unwrap_or_default(), &hash)? {
                return Err(CoreError::Forbidden(
                    "Das bisherige Kennwort (bzw. Erstkennwort) ist falsch.".into(),
                ));
            }
            if bisher == Some(neu) {
                return Err(CoreError::InvalidInput(
                    "Das neue Kennwort muss sich vom bisherigen unterscheiden.".into(),
                ));
            }
        }
        kennwort::regeln_pruefen(neu, &b.anmeldename)?;
        let hash = kennwort::hash_erzeugen(neu)?;
        self.repos
            .benutzer
            .kennwort_speichern(b.id, &hash, false, Utc::now())
            .await?;
        self.protokollieren(
            &b,
            Aktion::KennwortGeaendert,
            None,
            None,
            format!(
                "{} ({}) hat das eigene Kennwort geändert",
                b.anzeigename, b.anmeldename
            ),
        )
        .await?;
        self.repos.benutzer.get(b.id).await
    }

    /// Erstkennwort vergeben oder zurücksetzen (Administrator). Der Benutzer
    /// muss es bei der nächsten Anmeldung ändern; eine Sperre nach
    /// Fehlversuchen ist damit aufgehoben.
    pub async fn erstkennwort_vergeben(
        &self,
        ziel: BenutzerId,
        erstkennwort: &str,
        wer: &Benutzer,
    ) -> CoreResult<Benutzer> {
        self.berechtigung_pruefen(wer, Recht::BenutzerVerwalten)
            .await?;
        let b = self.repos.benutzer.get(ziel).await?;
        if b.anmeldename == SYSTEM {
            return Err(CoreError::InvalidInput(
                "Das Systemkonto meldet sich nicht an und bekommt kein Kennwort.".into(),
            ));
        }
        kennwort::regeln_pruefen(erstkennwort, &b.anmeldename)?;
        let hash = kennwort::hash_erzeugen(erstkennwort)?;
        self.repos
            .benutzer
            .kennwort_speichern(b.id, &hash, true, Utc::now())
            .await?;
        self.protokollieren(
            wer,
            Aktion::KennwortGeaendert,
            None,
            None,
            format!(
                "Erstkennwort für {} ({}) vergeben",
                b.anzeigename, b.anmeldename
            ),
        )
        .await?;
        self.repos.benutzer.get(b.id).await
    }
}

/// Einzelplatz: Wer eine Sicherung als Administrator wiederherstellt, verwaltet
/// auch das neue Profil – selbst wenn das Rechnerkonto dort unbekannt ist oder
/// nur eine geringere Rolle hat (anderer Rechner, anderer Benutzername). Ohne
/// Kennwortanmeldung gäbe es sonst keinen Weg zurück zur Benutzerverwaltung.
/// Der Vorgang wird im wiederhergestellten Profil protokolliert.
pub async fn verwaltung_nach_wiederherstellung(
    repos: &crate::repository::Repositories,
    rechnerkonto: &str,
    anzeigename: &str,
) -> CoreResult<Benutzer> {
    use crate::benutzer::{BenutzerAenderung, NeuerBenutzer};
    let name = benutzer::anmeldename_normalisieren(rechnerkonto)?;
    if name == SYSTEM {
        return Err(CoreError::InvalidInput(
            "Systemkonto ist nicht zulässig".into(),
        ));
    }
    let (b, aktion, details) = match repos.benutzer.by_anmeldename(&name).await? {
        Some(b) if b.rolle == Rolle::Administrator && b.aktiv => return Ok(b),
        Some(b) => {
            let neu = repos
                .benutzer
                .update(BenutzerAenderung {
                    id: b.id,
                    anzeigename: b.anzeigename.clone(),
                    rolle: Rolle::Administrator,
                    aktiv: true,
                })
                .await?;
            let details = format!(
                "{} ({}): bei der Wiederherstellung der Sicherung von {} zum Administrator",
                neu.anzeigename,
                neu.anmeldename,
                b.rolle.name()
            );
            (neu, Aktion::BenutzerGeaendert, details)
        }
        None => {
            let anzeige = match anzeigename.trim() {
                "" => name.clone(),
                n => n.to_string(),
            };
            let neu = repos
                .benutzer
                .create(NeuerBenutzer {
                    anmeldename: name,
                    anzeigename: anzeige,
                    rolle: Rolle::Administrator,
                })
                .await?;
            let details = format!(
                "{} ({}): bei der Wiederherstellung der Sicherung als Administrator angelegt",
                neu.anzeigename, neu.anmeldename
            );
            (neu, Aktion::BenutzerAngelegt, details)
        }
    };
    repos
        .protokoll
        .append(crate::domain::NeuerProtokolleintrag {
            zeit: Utc::now(),
            benutzer_id: b.id,
            benutzer_name: b.anzeigename.clone(),
            aktion,
            project_id: None,
            document_id: None,
            details,
        })
        .await?;
    Ok(b)
}
