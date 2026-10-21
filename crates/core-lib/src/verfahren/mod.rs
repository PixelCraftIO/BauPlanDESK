//! Fachliche Verfahrensakte. Eine Akte wird mit Ereignis und Revision atomar
//! geschrieben; DMS-Versionen, Abgabestände und Entscheidungen bleiben getrennt.
pub mod abgabe;
pub mod abgleich;
pub mod abweichungen;
pub mod chronik;
pub mod eingang;
pub mod externe_einreichung;
pub mod fristen;
pub mod landesrecht;
pub mod planindex;
pub mod profile;
mod pruefung;
mod regeln;
pub mod suche;
pub mod termine;
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
    /// Eigenständige Akte für eine Teilbaugenehmigung (ältere Arbeitsweise).
    /// Im laufenden Bauantrag wird sie als Bescheid mit
    /// [`Bescheid::teilbaugenehmigung`] erfasst.
    Teilbaugenehmigung,
    Freistellung,
    /// Genehmigungsbedürftige Nutzungsänderung ohne bauliche Errichtung.
    Nutzungsaenderung,
    /// Beseitigung (Abbruch) einer baulichen Anlage, je nach Land Anzeige
    /// oder Genehmigung.
    Beseitigung,
    /// Widerspruch gegen einen Bescheid des Vorgängerverfahrens.
    Widerspruch,
    Sonstiges,
}
impl Verfahrensart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Bauantrag => "Bauantrag",
            Self::Vorbescheid => "Vorbescheid",
            Self::Aenderungsantrag => "Änderungsantrag",
            Self::Teilbaugenehmigung => "Teilbaugenehmigung",
            Self::Freistellung => "Genehmigungsfreistellung",
            Self::Nutzungsaenderung => "Nutzungsänderung",
            Self::Beseitigung => "Beseitigung",
            Self::Widerspruch => "Widerspruch",
            Self::Sonstiges => "Sonstiges",
        }
    }
    /// Verfahren, die mit einer Baugenehmigung enden; für sie gelten die
    /// Geltungsdauer der Landesbauordnung und Teilbaugenehmigungen.
    pub fn baugenehmigung(&self) -> bool {
        matches!(
            self,
            Self::Bauantrag
                | Self::Aenderungsantrag
                | Self::Teilbaugenehmigung
                | Self::Nutzungsaenderung
        )
    }
    /// XBau-Antragsnachricht dieser Verfahrensart. Arten ohne eigene
    /// Antragsnachricht werden abgewiesen statt als Bauantrag gesendet.
    pub fn xbau_antrag<'a>(
        &self,
        angaben: &Antragsangaben,
        fragen: &'a [String],
    ) -> CoreResult<crate::xbau::Antragsart<'a>> {
        use crate::xbau::Antragsart;
        match self {
            // Ein Änderungsantrag ist ein neuer Bauantrag zum geänderten Vorhaben;
            // eine Nutzungsänderung ist ein Bauantrag mit geänderter Nutzung.
            Self::Bauantrag | Self::Aenderungsantrag | Self::Nutzungsaenderung => {
                Ok(Antragsart::Baugenehmigung)
            }
            Self::Vorbescheid => Ok(Antragsart::Vorbescheid { fragen }),
            Self::Freistellung => Ok(Antragsart::Genehmigungsfreistellung {
                als_bauantrag_weiterfuehren: angaben.als_bauantrag_weiterfuehren,
            }),
            Self::Teilbaugenehmigung => Err(fehler(
                "XBau-Ausgabe für Teilbaugenehmigungen wird noch nicht unterstützt: \
                 Die Nachricht 0230 braucht den Bezug auf das laufende Baugenehmigungsverfahren. \
                 Abgabe ohne XBau festhalten.",
            )),
            Self::Beseitigung => Err(fehler(
                "XBau-Ausgabe für Beseitigungen wird noch nicht unterstützt (Anzeige 0950). \
                 Abgabe ohne XBau festhalten.",
            )),
            Self::Widerspruch => Err(fehler(
                "Für einen Widerspruch gibt es keine XBau-Antragsnachricht. \
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
    /// Ein Abgabestand ist versendet; die Behörde hat die Vollständigkeit noch
    /// nicht beurteilt.
    Eingereicht,
    /// Die Behörde hat Unterlagen nachgefordert.
    Unvollstaendig,
    /// Die Behörde hat die Vollständigkeit bestätigt (siehe
    /// [`Verfahrensakte::vollstaendigkeit`]).
    Vollstaendig,
    InPruefung,
    Ruhend,
    Entschieden,
    Zurueckgenommen,
}
impl Verfahrensstatus {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Vorbereitung => "In Vorbereitung",
            Self::Eingereicht => "Eingereicht",
            Self::Unvollstaendig => "Unvollständig",
            Self::Vollstaendig => "Vollständig",
            Self::InPruefung => "In behördlicher Prüfung",
            Self::Ruhend => "Ruhend",
            Self::Entschieden => "Entschieden",
            Self::Zurueckgenommen => "Zurückgenommen",
        }
    }
    /// Entschiedene und zurückgenommene Verfahren werden nicht wieder geöffnet;
    /// eine neue Befassung ist ein Folgeverfahren.
    pub fn abgeschlossen(&self) -> bool {
        matches!(self, Self::Entschieden | Self::Zurueckgenommen)
    }
}
/// Behördliche Bestätigung der Vollständigkeit. Ihr Datum ist der Beginn von
/// Bearbeitungsfrist und Genehmigungsfiktion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Vollstaendigkeit {
    pub datum: NaiveDate,
    /// Eingegangenes Schreiben der Behörde in dieser Akte.
    pub kommunikation: Uuid,
}
/// Rücknahme des Antrags mit Datum und Beleg.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Ruecknahme {
    pub datum: NaiveDate,
    /// Rücknahmeschreiben oder Bestätigung der Behörde in dieser Akte.
    pub kommunikation: Uuid,
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
/// Art einer Nebenbestimmung (§ 36 VwVfG). Ein Hinweis begründet keine Pflicht.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Nebenbestimmungsart {
    #[default]
    Auflage,
    Bedingung,
    Befristung,
    Widerrufsvorbehalt,
    Auflagenvorbehalt,
    Hinweis,
}
impl Nebenbestimmungsart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Auflage => "Auflage",
            Self::Bedingung => "Bedingung",
            Self::Befristung => "Befristung",
            Self::Widerrufsvorbehalt => "Widerrufsvorbehalt",
            Self::Auflagenvorbehalt => "Auflagenvorbehalt",
            Self::Hinweis => "Hinweis",
        }
    }
    /// Auflage und Bedingung verlangen ein Tun und werden mit Beleg erfüllt.
    /// Befristung und Vorbehalte sind Ablauf- bzw. Kenntnishinweise; ein
    /// Hinweis begründet nichts.
    pub fn pflicht(&self) -> bool {
        matches!(self, Self::Auflage | Self::Bedingung)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Auflage {
    pub id: Uuid,
    /// Ältere Akten ohne diese Angabe lesen sie als Auflage.
    #[serde(default)]
    pub art: Nebenbestimmungsart,
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
    /// Verlängerungsbescheid: verlängert die Geltungsdauer eines früheren
    /// Bescheids dieser Akte. Ältere Bescheide lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub verlaengerung: Option<Verlaengerung>,
    /// Teilbaugenehmigung im laufenden Bauantrag: genehmigt einen Teil der
    /// Bauarbeiten vorab und beendet das Verfahren nicht. Ältere Bescheide
    /// lesen die Angabe als `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub teilbaugenehmigung: bool,
    /// Entscheidungen über beantragte Abweichungen, Ausnahmen und Befreiungen.
    /// Ältere Bescheide lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<abweichungen::Abweichungsentscheidung>>", optional)]
    pub abweichungsentscheidungen: Vec<abweichungen::Abweichungsentscheidung>,
}
/// Verlängerung der Geltungsdauer durch einen Bescheid der Behörde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Verlaengerung {
    /// Bescheid, dessen Geltungsdauer verlängert wird (Genehmigung oder
    /// frühere Verlängerung).
    pub bescheid: Uuid,
    /// Neues Ende der Geltungsdauer laut Verlängerungsbescheid.
    pub bis: NaiveDate,
}
/// Tatsächliche Bauausführung. Der Baubeginn beendet die Wiedervorlage der
/// Geltungsdauer, die Fertigstellung die einer Fertigstellungsfrist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Bauausfuehrung {
    pub baubeginn: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fertigstellung: Option<NaiveDate>,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Schrittart {
    /// Anhörung vor einer belastenden Entscheidung (§ 28 VwVfG).
    Anhoerung,
    /// Zurückstellung des Baugesuchs durch die Gemeinde (§ 15 BauGB).
    Zurueckstellung,
    /// Gemeindliches Einvernehmen (§ 36 BauGB). Die Frist ist das Ende der
    /// Zwei-Monats-Frist; danach gilt das Einvernehmen als erteilt.
    Einvernehmen,
    /// Beteiligung von Fachbehörden und Stellen.
    Fachbeteiligung,
    /// Beteiligung der Angrenzer nach der Landesbauordnung.
    Nachbarbeteiligung,
}
impl Schrittart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Anhoerung => "Anhörung",
            Self::Zurueckstellung => "Zurückstellung",
            Self::Einvernehmen => "Gemeindliches Einvernehmen",
            Self::Fachbeteiligung => "Beteiligung der Fachbehörden",
            Self::Nachbarbeteiligung => "Nachbarbeteiligung",
        }
    }
    /// Beteiligungsschritte: Ihre Frist endet mit Ablauf oder Entscheidung.
    pub fn beteiligung(&self) -> bool {
        matches!(
            self,
            Self::Einvernehmen | Self::Fachbeteiligung | Self::Nachbarbeteiligung
        )
    }
}
/// Prüfumfang des Genehmigungsverfahrens nach der Landesbauordnung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Pruefverfahren {
    /// Umfassende Prüfung (Baugenehmigungsverfahren).
    Regulaer,
    /// Vereinfachtes Baugenehmigungsverfahren mit eingeschränkter Prüfung.
    Vereinfacht,
}
/// Behördlicher Zwischenschritt vor der Entscheidung, eingeleitet durch ein
/// eingegangenes Schreiben der Akte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Verfahrensschritt {
    pub id: Uuid,
    pub art: Schrittart,
    pub kommunikation: Uuid,
    /// Anhörung: Frist zur Stellungnahme. Zurückstellung: Ende der Zurückstellung.
    pub frist: Option<Frist>,
    /// Ausgehende Stellungnahme dieser Akte; beendet die Wiedervorlage einer Anhörung.
    pub stellungnahme: Option<Uuid>,
    pub verantwortlich: Option<Uuid>,
    /// Zurückstellung: eingegangenes Folgeschreiben der Behörde, das sie vor
    /// ihrem Fristende beendet. Ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub folgeschreiben: Option<Uuid>,
    /// Anhörung: Beanstandungen der Behörde, auf die die Stellungnahme
    /// eingeht. Ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Beanstandung>>", optional)]
    pub gruende: Vec<Beanstandung>,
}
/// Beanstandung der Behörde (Anhörung, Ablehnung) und die Reaktion darauf.
/// Wortlaut und Grundlage bleiben nach dem Speichern unverändert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Beanstandung {
    pub id: Uuid,
    pub text: String,
    #[serde(default)]
    pub grundlage: String,
    /// Wie der Antrag reagiert: Stellungnahme, Planänderung, Abweichungsantrag.
    #[serde(default)]
    pub reaktion: String,
    /// Abweichung dieser Akte, mit der die Beanstandung ausgeräumt werden soll.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub abweichung: Option<Uuid>,
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
    /// Vom Core abgeleitete Art der Verfahrensänderung (Zeitleiste). Manuell
    /// erfasste Ereignisse und ältere Einträge haben keine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<chronik::Ereignisart>>", optional)]
    pub arten: Vec<chronik::Ereignisart>,
    /// Begründete Berichtigung eines erfassten Werts; alter und neuer Wert
    /// werden vom Core eingetragen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub berichtigung: Option<chronik::Wertberichtigung>,
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
    /// Ältere Akten ohne diese Angabe lesen sie leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub vollstaendigkeit: Option<Vollstaendigkeit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ruecknahme: Option<Ruecknahme>,
    /// Baubeginn und Fertigstellung; ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ausfuehrung: Option<Bauausfuehrung>,
    pub behoerde: Behoerde,
    pub aktenzeichen: String,
    pub verantwortlich: Option<Uuid>,
    pub vorgaenger: Option<Uuid>,
    /// Regulär oder vereinfacht; ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pruefverfahren: Option<Pruefverfahren>,
    /// Bearbeitungsfrist oder Eintritt einer Genehmigungsfiktion, wie sie die
    /// Behörde, etwa in der Eingangs- oder Vollständigkeitsbestätigung, nennt.
    /// Ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bearbeitungsfrist: Option<Frist>,
    pub profil: Option<profile::Profilstand>,
    pub kommunikation: Vec<Kommunikation>,
    pub nachforderungen: Vec<Nachforderung>,
    /// Ältere Akten ohne diese Angabe lesen sie leer.
    #[serde(default)]
    pub schritte: Vec<Verfahrensschritt>,
    pub anforderungen: Vec<Anforderung>,
    /// Abweichungen, Ausnahmen, Befreiungen und Erleichterungen. Ältere Akten
    /// lesen die Angabe leer; leer wird sie nicht geschrieben.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<abweichungen::Abweichung>>", optional)]
    pub abweichungen: Vec<abweichungen::Abweichung>,
    pub dokumentbeziehungen: Vec<Dokumentbeziehung>,
    pub abgaben: Vec<abgabe::Abgabestand>,
    /// Außerhalb der App erfolgte Einreichungen. Ältere Akten ohne diese
    /// Angabe lesen sie leer; leer wird sie nicht geschrieben.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<externe_einreichung::ExterneEinreichung>>", optional)]
    pub externe_einreichungen: Vec<externe_einreichung::ExterneEinreichung>,
    pub bescheide: Vec<Bescheid>,
    /// Gebührenbescheide mit Zahlungsfrist; ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<eingang::Gebuehrenbescheid>>", optional)]
    pub gebuehren: Vec<eingang::Gebuehrenbescheid>,
    /// Erfasste Unterschriften auf Dokumentversionen (§ 70 Abs. 3 BauO NRW
    /// u. ä.); ältere Akten lesen die Angabe leer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<landesrecht::Unterschrift>>", optional)]
    pub unterschriften: Vec<landesrecht::Unterschrift>,
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
    /// Art der Wiedervorlage; bestimmt, wann sie endet.
    pub art: termine::Terminart,
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
        chronik::berichtigungen_ausfuellen(original.as_ref(), &mut a, bisher);
        pruefung::pruefen(self, p, id, &a, original.as_ref(), wer, kontext).await?;
        if original.as_ref() == Some(&a) {
            return Ok(alt.expect("vorhandener Stand"));
        }
        let mut details = Vec::new();
        if let Some(alt) = &original {
            if alt.status != a.status {
                details.push(format!(
                    "Verfahrensstatus: {} → {}",
                    alt.status.bezeichnung(),
                    a.status.bezeichnung()
                ));
            }
            if alt.vollstaendigkeit != a.vollstaendigkeit {
                details.push(match &a.vollstaendigkeit {
                    Some(v) => format!("Vollständigkeit bestätigt am {}", v.datum),
                    None => "Vollständigkeitsbestätigung entfernt".into(),
                });
            }
            if alt.ruecknahme != a.ruecknahme {
                details.push(match &a.ruecknahme {
                    Some(r) => format!("Antrag zurückgenommen am {}", r.datum),
                    None => "Rücknahme entfernt".into(),
                });
            }
            if alt.ausfuehrung != a.ausfuehrung {
                details.push(match &a.ausfuehrung {
                    Some(x) => format!(
                        "Baubeginn {}{}",
                        x.baubeginn,
                        x.fertigstellung
                            .map(|f| format!(", Fertigstellung {f}"))
                            .unwrap_or_default()
                    ),
                    None => "Angaben zur Bauausführung entfernt".into(),
                });
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
            for v in &a.schritte {
                let vorher = alt.schritte.iter().find(|n| n.id == v.id);
                if vorher.is_none() {
                    details.push(format!("{} erfasst", v.art.bezeichnung()));
                }
                if vorher.is_some_and(|n| n.frist != v.frist)
                    || (vorher.is_none() && v.frist.is_some())
                {
                    details.push(format!(
                        "Frist {}: {}",
                        v.art.bezeichnung(),
                        v.frist
                            .as_ref()
                            .map(|f| f.datum.to_string())
                            .unwrap_or_else(|| "entfernt".into())
                    ));
                }
                if v.stellungnahme.is_some()
                    && vorher.is_none_or(|n| n.stellungnahme != v.stellungnahme)
                {
                    details.push(format!(
                        "Stellungnahme zur {} zugeordnet",
                        v.art.bezeichnung()
                    ));
                }
                if v.folgeschreiben.is_some()
                    && vorher.is_none_or(|n| n.folgeschreiben != v.folgeschreiben)
                {
                    details.push(format!(
                        "{} durch Folgeschreiben beendet",
                        v.art.bezeichnung()
                    ));
                }
            }
            if alt.aktenzeichen != a.aktenzeichen {
                details.push(format!(
                    "Aktenzeichen: {} → {}",
                    if alt.aktenzeichen.is_empty() {
                        "–"
                    } else {
                        &alt.aktenzeichen
                    },
                    if a.aktenzeichen.is_empty() {
                        "–"
                    } else {
                        &a.aktenzeichen
                    }
                ));
            }
            for e in a
                .externe_einreichungen
                .iter()
                .skip(alt.externe_einreichungen.len())
            {
                details.push(format!(
                    "Externe Einreichung nachgetragen: {} am {}{}",
                    e.weg.bezeichnung(),
                    e.datum,
                    if e.berichtigt.is_some() {
                        " (Berichtigung)"
                    } else {
                        ""
                    }
                ));
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
                match alt.bescheide.iter().find(|n| n.id == b.id) {
                    None => details.push(format!(
                        "Bescheid erfasst: {:?} am {}{}",
                        b.entscheidung,
                        b.datum,
                        b.verlaengerung
                            .as_ref()
                            .map(|v| format!(", verlängert die Geltungsdauer bis {}", v.bis))
                            .unwrap_or_default()
                    )),
                    Some(v) if v.zugestellt != b.zugestellt => {
                        if let Some(z) = b.zugestellt {
                            details.push(match v.zugestellt {
                                None => format!(
                                    "Zustellung des Bescheids vom {} nachgetragen: {z}",
                                    b.datum
                                ),
                                Some(alt) => format!(
                                    "Zustellung des Bescheids vom {} berichtigt: {alt} → {z}",
                                    b.datum
                                ),
                            });
                        }
                    }
                    _ => {}
                }
                for p in &b.auflagen {
                    let v = alt
                        .bescheide
                        .iter()
                        .flat_map(|b| &b.auflagen)
                        .find(|v| v.id == p.id);
                    if v.is_none_or(|v| {
                        v.erledigt != p.erledigt || v.frist != p.frist || v.art != p.art
                    }) {
                        details.push(format!(
                            "{} {} · {} · Frist {}",
                            p.art.bezeichnung(),
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
        let arten = chronik::arten(original.as_ref(), &a);
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
            datum: crate::kalender::heute(),
            erfasst_am: Utc::now(),
            erfasst_von: wer.id.0,
            text: text.chars().take(4000).collect(),
            beleg: None,
            berichtigt: None,
            arten: arten.clone(),
            berichtigung: None,
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
        // Das Projektprotokoll führt die Verfahrensänderung mit eigener Aktion,
        // abgeleitet aus dem Aktenereignis dieser Revision.
        let audit = Self::audit(
            wer,
            chronik::aktion(&arten),
            Some(p),
            None,
            format!("Verfahren {}: {}", a.bezeichnung, text)
                .chars()
                .take(2000)
                .collect(),
        );
        self.repos.commit.fachstand(f, basis, audit).await
    }
}
