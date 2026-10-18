//! Portalunterstützung auf eingefrorenen Verfahrensabgaben. Kein automatischer
//! Versand. Portalereignis und Verfahrensprojektion werden gemeinsam bestätigt.
mod dienst;
mod paket;
pub mod profile;
pub(crate) mod pruefung;
use crate::{
    CoreError, CoreResult,
    domain::Bundesland,
    plattform::objekte::Dokumentbezug,
    verfahren::{
        Verfahrensart,
        abgabe::{Abgabedokument, Artefakt},
    },
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Profilfreigabe {
    Allgemein,
    Entwurf,
    FachlichGeprueft,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PortalUnterlagenart {
    pub code: String,
    pub name: String,
    pub erforderlich: bool,
    pub kategorien: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalprofil {
    pub id: String,
    pub version: String,
    pub titel: String,
    pub bundesland: Option<Bundesland>,
    pub verfahren: Vec<Verfahrensart>,
    pub freigabe: Profilfreigabe,
    pub quelle: String,
    pub stand: String,
    pub portal_url: Option<String>,
    #[ts(type = "number | null")]
    pub max_datei_bytes: Option<u64>,
    #[ts(type = "number | null")]
    pub max_gesamt_bytes: Option<u64>,
    pub erlaubte_endungen: Vec<String>,
    pub unterlagenarten: Vec<PortalUnterlagenart>,
    pub hinweise: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalziel {
    pub kommune: String,
    pub stelle: String,
    pub behoerdenkennung: String,
    pub url: String,
    pub zuordnung_bestaetigt: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalvorgang {
    pub id: Uuid,
    pub profil: Portalprofil,
    pub ziel: Portalziel,
    pub erstellt_am: DateTime<Utc>,
    pub erstellt_von: Uuid,
    pub abgaben: Vec<Portalabgabe>,
    pub rueckmeldungen: Vec<Portalrueckmeldung>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalakte {
    pub vorgaenge: Vec<Portalvorgang>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Portalabgabeart {
    Erstantrag,
    Nachreichung,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct PortalForderungsbezug {
    pub nachforderung: Uuid,
    pub punkt: Uuid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DateiZuordnung {
    pub dokument: Dokumentbezug,
    pub unterlagenart: Option<String>,
    pub uploadname: String,
    pub ersetzt: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Dateivergleich {
    Neu,
    Identisch,
    Geaendert,
    Aelter,
    AnderesDokument,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Portaldatei {
    pub original: Abgabedokument,
    pub uploadname: String,
    pub unterlagenart: Option<String>,
    pub vergleich: Dateivergleich,
    pub ersetzt: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Portalpruefbericht {
    pub fehler: Vec<String>,
    pub hinweise: Vec<String>,
    pub profil_fachlich_geprueft: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Portalabgabe {
    pub id: Uuid,
    pub abgabe: Uuid,
    pub auftrag_hash: String,
    pub art: Portalabgabeart,
    pub vorgaenger: Option<Uuid>,
    pub forderungen: Vec<PortalForderungsbezug>,
    pub dateien: Vec<Portaldatei>,
    pub pruefung: Portalpruefbericht,
    pub erstellt_am: DateTime<Utc>,
    pub bereitstellung: Artefakt,
    pub ereignisse: Vec<Portalereignis>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Portalereignisart {
    AbgabeBelegt,
    EingangBelegt,
    Zurueckgewiesen,
    AusgangUnklar,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Dateiabgleich {
    Ungeklaert,
    Vollstaendig,
    Abweichend,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalereignis {
    pub id: Uuid,
    pub art: Portalereignisart,
    pub datum: NaiveDate,
    pub referenz: String,
    pub notiz: String,
    pub beleg: Option<Dokumentbezug>,
    pub dateiabgleich: Dateiabgleich,
    pub bestaetigte_dateien: Vec<Dokumentbezug>,
    pub berichtigt: Option<Uuid>,
    pub erfasst_am: DateTime<Utc>,
    pub erfasst_von: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalereigniswunsch {
    pub id: Uuid,
    pub art: Portalereignisart,
    pub datum: NaiveDate,
    pub referenz: String,
    pub notiz: String,
    pub beleg: Option<Dokumentbezug>,
    pub dateiabgleich: Dateiabgleich,
    pub bestaetigte_dateien: Vec<Dokumentbezug>,
    pub berichtigt: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalrueckmeldung {
    pub id: Uuid,
    pub betreff: String,
    pub datum: NaiveDate,
    pub referenz: String,
    pub original: Dokumentbezug,
    pub inhalt: String,
    pub abgabe: Option<Uuid>,
    pub xbau_nachricht: Option<Uuid>,
    pub erfasst_am: DateTime<Utc>,
    pub erfasst_von: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalrueckmeldungswunsch {
    pub id: Uuid,
    pub betreff: String,
    pub datum: NaiveDate,
    pub referenz: String,
    pub original: Dokumentbezug,
    pub inhalt: String,
    pub abgabe: Option<Uuid>,
    pub xbau_nachricht: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalvorgangswunsch {
    pub id: Uuid,
    pub profil_id: String,
    pub profil_version: String,
    pub ziel: Portalziel,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Portalabgabewunsch {
    pub abgabe: Uuid,
    pub art: Portalabgabeart,
    pub vorgaenger: Option<Uuid>,
    pub dateien: Vec<DateiZuordnung>,
    pub forderungen: Vec<PortalForderungsbezug>,
}
pub(crate) fn fehler(t: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(format!("Portal: {}", t.into()))
}
pub fn letztes_ereignis(b: &Portalabgabe) -> Option<&Portalereignis> {
    b.ereignisse
        .iter()
        .enumerate()
        .filter(|(_, e)| e.berichtigt.is_none())
        .map(|(i, root)| {
            let mut e = root;
            while let Some(n) = b.ereignisse.iter().find(|n| n.berichtigt == Some(e.id)) {
                e = n;
            }
            (e.datum, i, e)
        })
        .max_by_key(|(d, i, _)| (*d, *i))
        .map(|(_, _, e)| e)
}
/// Nur explizit abgeglichene Abgaben bilden einen belastbaren Dateistand.
pub fn dateistand_belegt(b: &Portalabgabe) -> bool {
    letztes_ereignis(b).is_some_and(|e| {
        e.dateiabgleich == Dateiabgleich::Vollstaendig
            && matches!(
                e.art,
                Portalereignisart::AbgabeBelegt | Portalereignisart::EingangBelegt
            )
    })
}
pub fn browser_url(url: &str) -> CoreResult<String> {
    if url.len() > 2000 {
        return Err(fehler("Portaladresse zu lang"));
    }
    let u = url::Url::parse(url).map_err(|_| fehler("Gültige HTTPS-Portaladresse erforderlich"))?;
    let host = u
        .host_str()
        .ok_or_else(|| fehler("Portaladresse ohne Host"))?;
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port_or_known_default() != Some(443)
        || u.query().is_some()
        || u.fragment().is_some()
        || host.parse::<std::net::IpAddr>().is_ok()
        || !host.contains('.')
        || [".localhost", ".local", ".internal"]
            .iter()
            .any(|s| host.ends_with(s))
    {
        return Err(fehler(
            "Portaladresse benötigt HTTPS mit öffentlichem Host, ohne Zugangsdaten, Parameter oder abweichenden Port",
        ));
    }
    Ok(u.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Portalvorbereitung {
    pub referenzdateien: Vec<Portaldatei>,
    pub dateien: Vec<Portaldatei>,
    pub pruefung: Portalpruefbericht,
}
pub(crate) use pruefung::validieren;

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Portalbedarf {
    pub vorgang: Uuid,
    pub abgabe: Option<Uuid>,
    pub titel: String,
    pub hinweis: String,
}
pub fn arbeitsbedarf(a: &crate::verfahren::Verfahrensakte) -> Vec<Portalbedarf> {
    let mut r = vec![];
    for v in a.portal.iter().flat_map(|p| &p.vorgaenge) {
        if v.abgaben.is_empty() {
            r.push(Portalbedarf {
                vorgang: v.id,
                abgabe: None,
                titel: "Portalunterlagen zusammenstellen".into(),
                hinweis: format!(
                    "{}: Noch kein eingefrorener Abgabestand zugeordnet.",
                    v.ziel.stelle
                ),
            });
        }
        for b in &v.abgaben {
            if !dateistand_belegt(b) {
                r.push(Portalbedarf{vorgang:v.id,abgabe:Some(b.id),titel:"Portalabgabe und Dateistand prüfen".into(),hinweis:format!("{}: Für diesen Stand fehlen eine vollständig abgeglichene Abgabe oder deren gültiger Beleg. Originalereignisse im Portalbereich prüfen.",v.ziel.stelle)});
            }
        }
    }
    r
}
