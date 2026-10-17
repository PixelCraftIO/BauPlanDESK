//! Fachliche Verfahrensakte. Eine Akte wird mit Ereignis und Revision atomar
//! geschrieben; DMS-Versionen, Abgabestände und Entscheidungen bleiben getrennt.
pub mod abgabe;
pub mod profile;
mod pruefung;
pub mod suche;
use crate::plattform::objekte::Dokumentbezug;
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::ProjectId,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Verfahrensart {
    Bauantrag,
    Vorbescheid,
    Aenderungsantrag,
    Teilbaugenehmigung,
    Freistellung,
    Sonstiges,
}
impl Verfahrensart {
    /// XBau-Antragsnachricht dieser Verfahrensart. Arten ohne eigene
    /// Antragsnachricht werden abgewiesen statt als Bauantrag gesendet.
    pub fn xbau_antrag<'a>(
        &self,
        angaben: &Antragsangaben,
        fragen: &'a [String],
    ) -> CoreResult<crate::xbau::Antragsart<'a>> {
        use crate::xbau::Antragsart;
        match self {
            // Ein Änderungsantrag ist ein neuer Bauantrag zum geänderten Vorhaben.
            Self::Bauantrag | Self::Aenderungsantrag => Ok(Antragsart::Baugenehmigung),
            Self::Vorbescheid => Ok(Antragsart::Vorbescheid { fragen }),
            Self::Freistellung => Ok(Antragsart::Genehmigungsfreistellung {
                als_bauantrag_weiterfuehren: angaben.als_bauantrag_weiterfuehren,
            }),
            Self::Teilbaugenehmigung => Err(fehler(
                "XBau-Ausgabe für Teilbaugenehmigungen wird noch nicht unterstützt: \
                 Die Nachricht 0230 braucht den Bezug auf das laufende Baugenehmigungsverfahren. \
                 Abgabe ohne XBau festhalten.",
            )),
            Self::Sonstiges => Err(fehler(
                "Für die Verfahrensart „Sonstiges“ gibt es keine XBau-Antragsnachricht. \
                 Verfahrensart wählen oder Abgabe ohne XBau festhalten.",
            )),
        }
    }
}
/// Angaben, die nur bestimmte Verfahrensarten in ihrer Antragsnachricht tragen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Antragsangaben {
    /// Fragen des Vorbescheidsantrags; jede Frage wird ein `beantragterVorbescheid`.
    pub vorbescheidfragen: Vec<Vorbescheidfrage>,
    /// Genehmigungsfreistellung: bei abschlägiger Entscheidung als Bauantrag weiterführen.
    pub als_bauantrag_weiterfuehren: bool,
}
impl Antragsangaben {
    pub fn fragen(&self) -> Vec<String> {
        self.vorbescheidfragen
            .iter()
            .map(|f| f.frage.clone())
            .collect()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Vorbescheidfrage {
    pub id: Uuid,
    pub frage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Verfahrensstatus {
    Vorbereitung,
    InPruefung,
    Ruhend,
    Entschieden,
    Zurueckgenommen,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Behoerde {
    pub id: Uuid,
    pub name: String,
    pub kennung: String,
    pub kontakt: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Bezug {
    Dokument { dokument: Dokumentbezug },
    Nachweis { id: Uuid },
    Kommunikation { id: Uuid },
    Nachforderung { id: Uuid, punkt: Uuid },
    Anforderung { id: Uuid },
    Abgabe { id: Uuid },
    Bescheid { id: Uuid },
    Auflage { bescheid: Uuid, id: Uuid },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Kommunikationsart {
    Email,
    Brief,
    Portal,
    Telefon,
    Besprechung,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Kommunikation {
    pub id: Uuid,
    pub art: Kommunikationsart,
    pub eingehend: bool,
    pub datum: NaiveDate,
    pub betreff: String,
    pub beteiligte: String,
    pub inhalt: String,
    pub original: Option<Dokumentbezug>,
    pub bezuege: Vec<Bezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Punktstatus {
    Offen,
    InArbeit,
    Vorbereitet,
    Eingereicht,
    Bestaetigt,
    Entfallen,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Frist {
    pub datum: NaiveDate,
    pub ausloeser: String,
    pub beleg: Option<Dokumentbezug>,
    pub erinnerung_tage: u32,
    pub grund: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Forderungspunkt {
    pub id: Uuid,
    pub titel: String,
    pub grundlage: String,
    pub verantwortlich: Option<Uuid>,
    pub frist: Option<Frist>,
    pub status: Punktstatus,
    pub antwort: String,
    pub dokumente: Vec<Dokumentbezug>,
    pub abgaben: Vec<Uuid>,
    pub bestaetigung: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Nachforderung {
    pub id: Uuid,
    pub kommunikation: Uuid,
    pub punkte: Vec<Forderungspunkt>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Entscheidung {
    Genehmigt,
    TeilweiseGenehmigt,
    Abgelehnt,
    Sonstiges,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Auflage {
    pub id: Uuid,
    pub titel: String,
    pub ausloeser: String,
    pub verantwortlich: Option<Uuid>,
    pub frist: Option<Frist>,
    pub erledigt: bool,
    pub nachweise: Vec<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Bescheid {
    pub id: Uuid,
    pub entscheidung: Entscheidung,
    pub datum: NaiveDate,
    pub zugestellt: Option<NaiveDate>,
    pub original: Dokumentbezug,
    pub genehmigte_unterlagen: Vec<Dokumentbezug>,
    pub auflagen: Vec<Auflage>,
    pub ersetzt: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Erforderlichkeit {
    Ungeklaert,
    Erforderlich,
    NichtErforderlich,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Anforderung {
    pub id: Uuid,
    pub titel: String,
    pub erforderlich: Erforderlichkeit,
    pub grund: String,
    pub quelle: String,
    pub dokumente: Vec<Dokumentbezug>,
    pub freigegeben: bool,
    pub verantwortlich: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dokumentbeziehung {
    pub id: Uuid,
    pub vorher: Dokumentbezug,
    pub nachher: Dokumentbezug,
    pub grund: String,
    pub bezug: Option<Bezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Aktenereignis {
    pub id: Uuid,
    pub datum: NaiveDate,
    pub erfasst_am: DateTime<Utc>,
    pub erfasst_von: Uuid,
    pub text: String,
    pub beleg: Option<Dokumentbezug>,
    pub berichtigt: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Verfahrensakte {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub portal: Option<crate::portale::Portalakte>,
    pub bezeichnung: String,
    pub art: Verfahrensart,
    /// Ältere Akten ohne diese Angaben lesen sie leer.
    #[serde(default)]
    pub antragsangaben: Antragsangaben,
    pub status: Verfahrensstatus,
    pub behoerde: Behoerde,
    pub aktenzeichen: String,
    pub verantwortlich: Option<Uuid>,
    pub vorgaenger: Option<Uuid>,
    pub profil: Option<profile::Profilstand>,
    pub kommunikation: Vec<Kommunikation>,
    pub nachforderungen: Vec<Nachforderung>,
    pub anforderungen: Vec<Anforderung>,
    pub dokumentbeziehungen: Vec<Dokumentbeziehung>,
    pub abgaben: Vec<abgabe::Abgabestand>,
    pub bescheide: Vec<Bescheid>,
    pub ereignisse: Vec<Aktenereignis>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Erinnerung {
    pub verfahren: Uuid,
    pub objekt: Uuid,
    pub titel: String,
    pub datum: NaiveDate,
    pub ueberfaellig: bool,
    pub verantwortlich: Option<Uuid>,
}
fn fehler(t: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(t.into())
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AktenCommit {
    Bearbeitung,
    Abgabe,
    Portal,
}
impl CoreServices {
    pub async fn verfahrensakte(
        &self,
        p: ProjectId,
        id: Uuid,
    ) -> CoreResult<(Fachstand, Verfahrensakte)> {
        let f = self
            .repos
            .commit
            .fachstaende(p, "verfahren")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| CoreError::NotFound("Verfahren".into()))?;
        if f.schema_version != 1 {
            return Err(fehler(
                "Unbekannte Version der Verfahrensakte; Anwendung aktualisieren",
            ));
        }
        let a = serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
        Ok((f, a))
    }
    pub(crate) async fn verfahren_commit(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        a: Verfahrensakte,
        wer: &Benutzer,
        server_abgabe: bool,
    ) -> CoreResult<Fachstand> {
        self.verfahren_speichern_intern(
            p,
            id,
            basis,
            a,
            wer,
            if server_abgabe {
                AktenCommit::Abgabe
            } else {
                AktenCommit::Bearbeitung
            },
        )
        .await
    }
    pub(crate) async fn verfahren_speichern_intern(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        mut a: Verfahrensakte,
        wer: &Benutzer,
        kontext: AktenCommit,
    ) -> CoreResult<Fachstand> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        let alt = self
            .repos
            .commit
            .fachstaende(p, "verfahren")
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string());
        let revision = alt.as_ref().map(|f| f.revision).unwrap_or(0);
        if alt.as_ref().is_some_and(|f| f.schema_version != 1) {
            return Err(fehler(
                "Unbekannte Version der Verfahrensakte; Anwendung aktualisieren",
            ));
        }
        if revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: revision,
            });
        }
        let original: Option<Verfahrensakte> = alt
            .as_ref()
            .map(|f| serde_json::from_value(f.daten.clone()))
            .transpose()
            .map_err(|e| fehler(e.to_string()))?;
        if let Some(profil) = &mut a.profil {
            profil.katalog.pruefen()?;
            profil.katalog.vorschlaege(id, &profil.vorhaben)?;
            if original
                .as_ref()
                .and_then(|a| a.profil.as_ref())
                .is_some_and(|alt| {
                    alt.katalog.id == profil.katalog.id
                        && alt.katalog.version == profil.katalog.version
                        && alt.katalog != profil.katalog
                })
            {
                return Err(fehler("Geänderter Katalog benötigt eine neue Version"));
            }
            if profil.vorhaben.art != a.art
                || profil.vorhaben.bundesland != self.get_project(p).await?.bundesland
            {
                return Err(fehler("Vorhabenprofil passt nicht zum Verfahren"));
            }
            if profil.bestaetigt_von.is_some()
                && !original
                    .as_ref()
                    .is_some_and(|alt| alt.profil.as_ref() == Some(profil))
            {
                self.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
                profil.bestaetigt_von = Some(wer.id.0);
                profil.bestaetigt_am = Some(Utc::now());
            }
        }
        let profil_geaendert =
            original
                .as_ref()
                .is_some_and(|alt| match (&alt.profil, &a.profil) {
                    (Some(vorher), Some(nachher)) => {
                        vorher.katalog != nachher.katalog || vorher.vorhaben != nachher.vorhaben
                    }
                    (Some(_), None) => true,
                    _ => false,
                });
        if profil_geaendert {
            for r in &mut a.anforderungen {
                r.freigegeben = false;
            }
        }
        if kontext == AktenCommit::Bearbeitung {
            self.anforderungen_abgleichen(p, id, &mut a).await?;
        }
        let bisher = original.as_ref().map(|a| a.ereignisse.len()).unwrap_or(0);
        for b in &mut a.abgaben {
            b.versandstatus = abgabe::versandstand(b);
        }
        for e in a.ereignisse.iter_mut().skip(bisher) {
            e.erfasst_von = wer.id.0;
            e.erfasst_am = Utc::now();
        }
        pruefung::pruefen(self, p, id, &a, original.as_ref(), wer, kontext).await?;
        if original.as_ref() == Some(&a) {
            return Ok(alt.expect("vorhandener Stand"));
        }
        let mut details = Vec::new();
        if let Some(alt) = &original {
            if alt.status != a.status {
                details.push(format!(
                    "Verfahrensstatus: {:?} → {:?}",
                    alt.status, a.status
                ));
            }
            for n in &a.nachforderungen {
                for p in &n.punkte {
                    let vorher = alt
                        .nachforderungen
                        .iter()
                        .flat_map(|n| &n.punkte)
                        .find(|v| v.id == p.id);
                    match vorher {
                        None => details.push(format!("Forderungspunkt ergänzt: {}", p.titel)),
                        Some(v) => {
                            if v.status != p.status {
                                details
                                    .push(format!("{}: {:?} → {:?}", p.titel, v.status, p.status));
                            }
                            if v.frist != p.frist {
                                details.push(format!(
                                    "Frist {}: {} → {} · {}",
                                    p.titel,
                                    v.frist
                                        .as_ref()
                                        .map(|f| f.datum.to_string())
                                        .unwrap_or_default(),
                                    p.frist
                                        .as_ref()
                                        .map(|f| f.datum.to_string())
                                        .unwrap_or_default(),
                                    p.frist
                                        .as_ref()
                                        .map(|f| f.grund.as_str())
                                        .unwrap_or("entfernt")
                                ));
                            }
                        }
                    }
                }
            }
            for c in &a.kommunikation {
                if !alt.kommunikation.iter().any(|n| n.id == c.id) {
                    details.push(format!("Kommunikation erfasst: {}", c.betreff));
                }
            }
            for b in &a.abgaben {
                if let Some(v) = alt.abgaben.iter().find(|v| v.id == b.id) {
                    for e in b.versand.iter().skip(v.versand.len()) {
                        details.push(format!(
                            "{}: {:?} am {} · {}",
                            b.titel, e.status, e.datum, e.notiz
                        ));
                    }
                } else {
                    details.push(format!("Abgabestand festgehalten: {}", b.titel));
                }
            }
            for b in &a.bescheide {
                if !alt.bescheide.iter().any(|n| n.id == b.id) {
                    details.push(format!(
                        "Bescheid erfasst: {:?} am {}",
                        b.entscheidung, b.datum
                    ));
                }
                for p in &b.auflagen {
                    let v = alt
                        .bescheide
                        .iter()
                        .flat_map(|b| &b.auflagen)
                        .find(|v| v.id == p.id);
                    if v.is_none_or(|v| v.erledigt != p.erledigt || v.frist != p.frist) {
                        details.push(format!(
                            "Auflage {} · {} · Frist {}",
                            p.titel,
                            if p.erledigt { "erfüllt" } else { "offen" },
                            p.frist
                                .as_ref()
                                .map(|f| f.datum.to_string())
                                .unwrap_or_default()
                        ));
                    }
                }
            }
        }
        let text = if basis == 0 {
            "Verfahren angelegt".into()
        } else {
            format!(
                "Revision {}: {}",
                basis + 1,
                if details.is_empty() {
                    "Verfahrensangaben aktualisiert".into()
                } else {
                    details.join("; ")
                }
            )
        };
        a.ereignisse.push(Aktenereignis {
            id: Uuid::new_v4(),
            datum: Utc::now().date_naive(),
            erfasst_am: Utc::now(),
            erfasst_von: wer.id.0,
            text: text.chars().take(4000).collect(),
            beleg: None,
            berichtigt: None,
        });

        let daten = serde_json::to_value(&a).map_err(|e| fehler(e.to_string()))?;
        if serde_json::to_vec(&daten)
            .map_err(|e| fehler(e.to_string()))?
            .len()
            > 4 * 1024 * 1024
        {
            return Err(fehler(
                "Verfahrensakte zu groß; weitere Verfahren getrennt führen",
            ));
        }
        let f = Fachstand {
            projekt_id: p,
            modul: "verfahren".into(),
            objekt: id.to_string(),
            schema_version: 1,
            revision: basis,
            daten,
        };
        let audit = Self::audit(
            wer,
            crate::domain::Aktion::ProjektGeaendert,
            Some(p),
            None,
            format!("Verfahren {}: Revision {}", a.bezeichnung, basis + 1),
        );
        self.repos.commit.fachstand(f, basis, audit).await
    }
    pub async fn verfahren_erinnerungen(
        &self,
        p: ProjectId,
        heute: NaiveDate,
    ) -> CoreResult<Vec<Erinnerung>> {
        self.get_project(p).await?;
        let mut r = Vec::new();
        for f in self.repos.commit.fachstaende(p, "verfahren").await? {
            let a: Verfahrensakte =
                serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
            let vid = Uuid::parse_str(&f.objekt).map_err(|e| fehler(e.to_string()))?;
            if a.status == Verfahrensstatus::Zurueckgenommen {
                continue;
            }
            let mut add =
                |objekt: Uuid, titel: &str, frist: &Option<Frist>, verantwortlich: Option<Uuid>| {
                    if let Some(f) = frist
                        && heute
                            >= f.datum
                                .checked_sub_signed(chrono::Duration::days(i64::from(
                                    f.erinnerung_tage,
                                )))
                                .unwrap_or(f.datum)
                    {
                        r.push(Erinnerung {
                            verfahren: vid,
                            objekt,
                            titel: format!("{} · {titel}", a.bezeichnung),
                            datum: f.datum,
                            ueberfaellig: f.datum < heute,
                            verantwortlich,
                        });
                    }
                };
            for n in &a.nachforderungen {
                for p in &n.punkte {
                    if !matches!(p.status, Punktstatus::Bestaetigt | Punktstatus::Entfallen) {
                        add(p.id, &p.titel, &p.frist, p.verantwortlich);
                    }
                }
            }
            for b in &a.bescheide {
                if a.bescheide.iter().any(|neu| neu.ersetzt == Some(b.id)) {
                    continue;
                }
                for p in &b.auflagen {
                    if !p.erledigt {
                        add(p.id, &p.titel, &p.frist, p.verantwortlich);
                    }
                }
            }
        }
        r.sort_by_key(|e| e.datum);
        Ok(r)
    }
}
