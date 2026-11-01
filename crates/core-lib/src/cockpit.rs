//! Büro-Cockpit: büroweites Lesemodell über alle Projekte eines Benutzers.
//!
//! Vertrag:
//! - Eingabe: die für den Benutzer lesbaren Projekte mit ihren Verfahrensakten
//!   und ein Stichtag (deutsches Ortsdatum, [`crate::kalender::heute`]).
//! - Ergebnis: laufende Termine aller Projekte mit Friststufe und Zielregister
//!   sowie eine Projektzeile je Projekt mit Verfahrensstand, letzter Abgabe,
//!   offenen Forderungspunkten, letztem Bescheid und nächster Frist.
//! - Termine stammen ausschließlich aus [`crate::verfahren::termine::termine`];
//!   das Cockpit rechnet keine Fristen selbst. Es speichert nichts.
//! - Ein Projekt, dessen Akten nicht gelesen werden können, verschwindet nicht
//!   still: es erscheint unter [`Buerocockpit::fehler`].
use crate::{
    CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::{Bundesland, Project, ProjectId},
    verfahren::{
        Entscheidung, Punktstatus, Verfahrensakte, Verfahrensart, Verfahrensstatus,
        abgabe::Versandstatus,
        termine::{Termin, Terminart, termine},
    },
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Friststufe eines Termins am Stichtag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Friststufe {
    /// Pflichtfrist vor dem Stichtag und nicht erledigt.
    Ueberfaellig,
    /// Fällig am Stichtag oder in den nächsten 7 Tagen.
    DieseWoche,
    /// Fällig in 8 bis 30 Tagen.
    Monat,
    /// Später als 30 Tage.
    Spaeter,
}
impl Friststufe {
    pub fn von(t: &Termin, heute: NaiveDate) -> Self {
        if t.ueberfaellig(heute) {
            return Self::Ueberfaellig;
        }
        match (t.datum - heute).num_days() {
            ..=7 => Self::DieseWoche,
            8..=30 => Self::Monat,
            _ => Self::Spaeter,
        }
    }
}

/// Register der Verfahrensakte, in dem ein Termin bearbeitet wird.
pub fn register(art: Terminart) -> &'static str {
    match art {
        Terminart::Forderungspunkt => "nachforderungen",
        Terminart::Nebenbestimmung
        | Terminart::Ablauf
        | Terminart::Rechtsbehelfsfrist
        | Terminart::Zahlung => "bescheide",
        Terminart::Anhoerung | Terminart::Zurueckstellung | Terminart::Beteiligung => "schritte",
        Terminart::Geltungsdauer | Terminart::Fertigstellung => "ausfuehrung",
        Terminart::Bearbeitungsfrist => "stamm",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CockpitTermin {
    pub projekt_id: ProjectId,
    pub projekt: String,
    pub verfahren: Uuid,
    pub verfahrensbezeichnung: String,
    pub objekt: Uuid,
    pub art: Terminart,
    pub art_bezeichnung: String,
    pub titel: String,
    pub datum: NaiveDate,
    /// Beginn der Wiedervorlage (Datum abzüglich Vorlauf).
    pub faellig_ab: NaiveDate,
    pub verantwortlich: Option<Uuid>,
    pub hinweis: String,
    pub dringlichkeit: Friststufe,
    /// Register der Verfahrensakte für den Sprung.
    pub register: String,
}

/// Kurzstand eines Verfahrens im Cockpit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CockpitVerfahren {
    pub id: Uuid,
    pub bezeichnung: String,
    pub art: Verfahrensart,
    pub status: Verfahrensstatus,
    pub status_bezeichnung: String,
    pub aktenzeichen: String,
    pub behoerde: String,
    pub verantwortlich: Option<Uuid>,
    /// Datum des jüngsten Versands oder Eingangs eines Abgabestands bzw. einer
    /// externen Einreichung.
    pub letzte_abgabe: Option<NaiveDate>,
    /// Forderungspunkte, die weder bestätigt noch entfallen sind.
    pub offene_punkte: u32,
    /// Jüngster nicht ersetzter Bescheid.
    pub letzter_bescheid: Option<CockpitBescheid>,
    /// Abgelehnt und noch kein Folgeverfahren angelegt: Handlungsoptionen im
    /// Register „Lage“ der Akte prüfen.
    pub ablehnung_offen: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CockpitBescheid {
    pub entscheidung: Entscheidung,
    pub datum: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CockpitProjekt {
    pub projekt_id: ProjectId,
    pub name: String,
    pub bundesland: Bundesland,
    pub verfahren: Vec<CockpitVerfahren>,
    pub naechste_frist: Option<NaiveDate>,
    #[ts(type = "number")]
    pub ueberfaellig: u32,
    #[ts(type = "number")]
    pub offene_punkte: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct CockpitFehler {
    pub projekt_id: ProjectId,
    pub projekt: String,
    pub meldung: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Buerocockpit {
    pub stichtag: NaiveDate,
    /// Laufende Termine aller Projekte, überfällige zuerst, dann nach Datum.
    pub termine: Vec<CockpitTermin>,
    /// Projekte nach Friststufe (überfällig, nächste Frist, Name).
    pub projekte: Vec<CockpitProjekt>,
    pub fehler: Vec<CockpitFehler>,
}

fn letzte_abgabe(a: &Verfahrensakte) -> Option<NaiveDate> {
    let abgaben = a.abgaben.iter().flat_map(|s| {
        s.versand
            .iter()
            .filter(|v| {
                matches!(
                    v.status,
                    Versandstatus::Versendet | Versandstatus::Eingegangen
                )
            })
            .map(|v| v.datum)
    });
    let extern_ = a.externe_einreichungen.iter().map(|e| e.datum);
    abgaben.chain(extern_).max()
}

fn verfahren_auswerten(
    id: Uuid,
    a: &Verfahrensakte,
    akten: &[(Uuid, Verfahrensakte)],
) -> CockpitVerfahren {
    let offene_punkte = a
        .nachforderungen
        .iter()
        .flat_map(|n| &n.punkte)
        .filter(|p| !matches!(p.status, Punktstatus::Bestaetigt | Punktstatus::Entfallen))
        .count() as u32;
    let ersetzt: Vec<Uuid> = a.bescheide.iter().filter_map(|b| b.ersetzt).collect();
    let letzter = a
        .bescheide
        .iter()
        .filter(|b| !ersetzt.contains(&b.id) && b.verlaengerung.is_none())
        .max_by_key(|b| b.datum);
    let folgeverfahren = akten.iter().any(|(_, f)| f.vorgaenger == Some(id));
    CockpitVerfahren {
        id,
        bezeichnung: a.bezeichnung.clone(),
        art: a.art.clone(),
        status: a.status.clone(),
        status_bezeichnung: a.status.bezeichnung().into(),
        aktenzeichen: a.aktenzeichen.clone(),
        behoerde: a.behoerde.name.clone(),
        verantwortlich: a.verantwortlich,
        letzte_abgabe: letzte_abgabe(a),
        offene_punkte,
        ablehnung_offen: letzter.is_some_and(|b| b.entscheidung == Entscheidung::Abgelehnt)
            && !folgeverfahren,
        letzter_bescheid: letzter.map(|b| CockpitBescheid {
            entscheidung: b.entscheidung.clone(),
            datum: b.datum,
        }),
    }
}

/// Wertet ein Projekt mit seinen Akten am Stichtag aus (reine Funktion).
pub fn projekt_auswerten(
    p: &Project,
    akten: &[(Uuid, Verfahrensakte)],
    heute: NaiveDate,
) -> (CockpitProjekt, Vec<CockpitTermin>) {
    let mut ts: Vec<CockpitTermin> = akten
        .iter()
        .flat_map(|(id, a)| termine(a, *id, p.bundesland))
        .filter(|t| t.aktiv(heute))
        .map(|t| CockpitTermin {
            projekt_id: p.id,
            projekt: p.name.clone(),
            verfahren: t.verfahren,
            verfahrensbezeichnung: t.verfahrensbezeichnung.clone(),
            objekt: t.objekt,
            art: t.art,
            art_bezeichnung: t.art.bezeichnung().into(),
            titel: t.titel.clone(),
            datum: t.datum,
            faellig_ab: t.faellig_ab(),
            verantwortlich: t.verantwortlich,
            hinweis: t.hinweis.clone(),
            dringlichkeit: Friststufe::von(&t, heute),
            register: register(t.art).into(),
        })
        .collect();
    ts.sort_by(|a, b| {
        a.dringlichkeit
            .cmp(&b.dringlichkeit)
            .then(a.datum.cmp(&b.datum))
    });
    let verfahren: Vec<CockpitVerfahren> = akten
        .iter()
        .map(|(id, a)| verfahren_auswerten(*id, a, akten))
        .collect();
    let projekt = CockpitProjekt {
        projekt_id: p.id,
        name: p.name.clone(),
        bundesland: p.bundesland,
        naechste_frist: ts.iter().map(|t| t.datum).filter(|d| *d >= heute).min(),
        ueberfaellig: ts
            .iter()
            .filter(|t| t.dringlichkeit == Friststufe::Ueberfaellig)
            .count() as u32,
        offene_punkte: verfahren.iter().map(|v| v.offene_punkte).sum(),
        verfahren,
    };
    (projekt, ts)
}

/// Fügt die Projektauswertungen zum Cockpit zusammen und sortiert.
pub fn zusammenfassen(
    heute: NaiveDate,
    teile: Vec<(CockpitProjekt, Vec<CockpitTermin>)>,
    fehler: Vec<CockpitFehler>,
) -> Buerocockpit {
    let mut termine = Vec::new();
    let mut projekte = Vec::new();
    for (p, t) in teile {
        projekte.push(p);
        termine.extend(t);
    }
    termine.sort_by(|a, b| {
        a.dringlichkeit
            .cmp(&b.dringlichkeit)
            .then(a.datum.cmp(&b.datum))
            .then(a.projekt.cmp(&b.projekt))
    });
    projekte.sort_by(|a, b| {
        b.ueberfaellig
            .cmp(&a.ueberfaellig)
            .then(match (a.naechste_frist, b.naechste_frist) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Buerocockpit {
        stichtag: heute,
        termine,
        projekte,
        fehler,
    }
}

impl CoreServices {
    /// Büro-Cockpit über alle Projekte, die `wer` lesen darf.
    pub async fn buero_cockpit(
        &self,
        wer: &Benutzer,
        heute: NaiveDate,
    ) -> CoreResult<Buerocockpit> {
        let mut teile = Vec::new();
        let mut fehler = Vec::new();
        for p in self.projekte_fuer_benutzer(wer).await? {
            match self.verfahrensakten_lesen(p.id).await {
                Ok(akten) => teile.push(projekt_auswerten(&p, &akten, heute)),
                Err(e) => fehler.push(CockpitFehler {
                    projekt_id: p.id,
                    projekt: p.name.clone(),
                    meldung: e.to_string(),
                }),
            }
        }
        Ok(zusammenfassen(heute, teile, fehler))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verfahren::{Forderungspunkt, Frist, Nachforderung};

    fn akte(json: serde_json::Value) -> Verfahrensakte {
        let mut basis = serde_json::json!({
            "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "eingereicht",
            "behoerde": {"id": Uuid::nil(), "name": "Bauamt", "kennung": "", "kontakt": ""}, "aktenzeichen": "63/1",
            "verantwortlich": null, "vorgaenger": null, "profil": null, "kommunikation": [],
            "nachforderungen": [], "anforderungen": [], "dokumentbeziehungen": [],
            "abgaben": [], "bescheide": [], "ereignisse": []
        });
        for (k, v) in json.as_object().unwrap() {
            basis[k] = v.clone();
        }
        serde_json::from_value(basis).unwrap()
    }
    fn projekt(name: &str) -> Project {
        serde_json::from_value(
            serde_json::json!({"id": Uuid::new_v4(), "name": name, "bundesland": "NW",
            "grundstuecksflaeche_m2": null, "version": 1,
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"}),
        )
        .unwrap()
    }
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn punkt(titel: &str, frist: Option<&str>, status: Punktstatus) -> Forderungspunkt {
        Forderungspunkt {
            id: Uuid::new_v4(),
            titel: titel.into(),
            grundlage: String::new(),
            verantwortlich: None,
            frist: frist.map(|d| Frist {
                datum: tag(d),
                ausloeser: String::new(),
                beleg: None,
                erinnerung_tage: 7,
                grund: String::new(),
            }),
            status,
            antwort: String::new(),
            dokumente: vec![],
            abgaben: vec![],
            bestaetigung: None,
        }
    }
    fn mit_punkten(punkte: Vec<Forderungspunkt>) -> Verfahrensakte {
        let mut a = akte(serde_json::json!({"status": "unvollstaendig"}));
        a.nachforderungen.push(Nachforderung {
            id: Uuid::new_v4(),
            kommunikation: Uuid::new_v4(),
            punkte,
        });
        a
    }

    #[test]
    fn dringlichkeit_nach_tagen() {
        let a = mit_punkten(vec![
            punkt("Statik", Some("2026-11-01"), Punktstatus::Offen),
            punkt("Lageplan", Some("2026-11-14"), Punktstatus::Offen),
            punkt("Stellplätze", Some("2026-11-25"), Punktstatus::InArbeit),
            punkt("Brandschutz", Some("2027-01-20"), Punktstatus::Offen),
            punkt("Erledigt", Some("2026-10-01"), Punktstatus::Bestaetigt),
        ]);
        let (p, t) = projekt_auswerten(
            &projekt("Haus A"),
            &[(Uuid::new_v4(), a)],
            tag("2026-11-05"),
        );
        let d: Vec<_> = t
            .iter()
            .map(|t| (t.titel.as_str(), t.dringlichkeit))
            .collect();
        assert_eq!(d.len(), 4, "{d:?}");
        assert_eq!(d[0].1, Friststufe::Ueberfaellig);
        assert!(d[0].0.contains("Statik"));
        assert_eq!(d[1].1, Friststufe::Monat);
        assert_eq!(d[2].1, Friststufe::Monat);
        assert_eq!(d[3].1, Friststufe::Spaeter);
        assert!(t.iter().all(|t| t.register == "nachforderungen"));
        assert_eq!(p.ueberfaellig, 1);
        assert_eq!(p.offene_punkte, 4);
        assert_eq!(p.naechste_frist, Some(tag("2026-11-14")));
    }

    #[test]
    fn diese_woche_bis_sieben_tage() {
        let a = mit_punkten(vec![punkt("A", Some("2026-11-12"), Punktstatus::Offen)]);
        let (_, t) = projekt_auswerten(&projekt("P"), &[(Uuid::new_v4(), a)], tag("2026-11-05"));
        assert_eq!(t[0].dringlichkeit, Friststufe::DieseWoche);
    }

    #[test]
    fn ablehnung_ohne_folgeverfahren_wird_gemeldet() {
        let id = Uuid::new_v4();
        let a = akte(serde_json::json!({"status": "entschieden", "bescheide": [{
            "id": Uuid::new_v4(), "entscheidung": "abgelehnt", "datum": "2026-10-01", "zugestellt": null,
            "original": {"dokument_id": Uuid::new_v4(), "version": 1}, "genehmigte_unterlagen": [],
            "auflagen": [], "ersetzt": null}]}));
        let (p, _) = projekt_auswerten(&projekt("P"), &[(id, a.clone())], tag("2026-10-05"));
        assert!(p.verfahren[0].ablehnung_offen);
        assert_eq!(
            p.verfahren[0]
                .letzter_bescheid
                .as_ref()
                .unwrap()
                .entscheidung,
            Entscheidung::Abgelehnt
        );
        let mut w = akte(serde_json::json!({"art": "widerspruch", "status": "vorbereitung"}));
        w.vorgaenger = Some(id);
        let (p, _) = projekt_auswerten(
            &projekt("P"),
            &[(id, a), (Uuid::new_v4(), w)],
            tag("2026-10-05"),
        );
        assert!(!p.verfahren[0].ablehnung_offen);
    }

    #[test]
    fn projekte_mit_ueberfaelligen_zuerst() {
        let ruhig = projekt_auswerten(&projekt("Aaa"), &[], tag("2026-11-05"));
        let a = mit_punkten(vec![punkt("X", Some("2026-11-01"), Punktstatus::Offen)]);
        let eilig = projekt_auswerten(&projekt("Zzz"), &[(Uuid::new_v4(), a)], tag("2026-11-05"));
        let c = zusammenfassen(tag("2026-11-05"), vec![ruhig, eilig], vec![]);
        assert_eq!(c.projekte[0].name, "Zzz");
        assert_eq!(c.termine.len(), 1);
    }

    #[test]
    fn letzte_abgabe_aus_versand_und_externer_einreichung() {
        let a = akte(serde_json::json!({"externe_einreichungen": [{
            "id": Uuid::new_v4(), "datum": "2026-09-03", "weg": "post", "bemerkung": "",
            "beleg": null, "dokumente": [], "erfasst_am": "2026-09-03T08:00:00Z", "erfasst_von": Uuid::nil(), "berichtigt": null}]}));
        assert_eq!(letzte_abgabe(&a), Some(tag("2026-09-03")));
    }
}
