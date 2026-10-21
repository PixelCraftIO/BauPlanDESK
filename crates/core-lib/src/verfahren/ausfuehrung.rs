//! Phase nach der Genehmigung: Bauleitung, bautechnische Nachweise mit
//! Prüfung, Fälligkeit von Nebenbestimmungen und die Voraussetzungen von
//! Baubeginn und Nutzungsaufnahme.
//!
//! Die Checklisten werten aus, was die Akte belegt; Unbelegtes bleibt offen.
//! Rechtsgrundlagen sind Hinweise (NRW mit Fundstelle, sonst allgemein).
use super::*;
use crate::domain::Bundesland;

/// Zeitpunkt, zu dem eine Pflicht erfüllt sein muss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Faelligkeit {
    MitAntrag,
    VorBaubeginn,
    WaehrendBauausfuehrung,
    VorNutzungsaufnahme,
    Dauerhaft,
}
impl Faelligkeit {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::MitAntrag => "Mit dem Antrag",
            Self::VorBaubeginn => "Vor Baubeginn",
            Self::WaehrendBauausfuehrung => "Während der Bauausführung",
            Self::VorNutzungsaufnahme => "Vor Nutzungsaufnahme",
            Self::Dauerhaft => "Dauerhaft",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Nachweisart {
    Standsicherheit,
    Brandschutz,
    Waermeschutz,
    Schallschutz,
    Sonstiges,
}
impl Nachweisart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Standsicherheit => "Standsicherheit",
            Self::Brandschutz => "Brandschutz",
            Self::Waermeschutz => "Wärmeschutz / GEG",
            Self::Schallschutz => "Schallschutz",
            Self::Sonstiges => "Sonstiger Nachweis",
        }
    }
}
/// Wer den Nachweis prüft oder bescheinigt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Pruefart {
    /// Keine Prüfung vorgeschrieben (Erklärung der Fachplanung genügt).
    Keine,
    Bauaufsicht,
    /// Prüfingenieurin oder Prüfingenieur.
    Pruefingenieur,
    /// Staatlich anerkannte Sachverständige.
    Sachverstaendiger,
}
/// Bautechnischer Nachweis mit Vorlage und Prüfung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BautechnischerNachweis {
    pub id: Uuid,
    pub art: Nachweisart,
    #[serde(default)]
    pub bezeichnung: String,
    pub pruefung: Pruefart,
    /// Prüfende Person oder Stelle.
    #[serde(default)]
    pub pruefer: String,
    pub faellig: Faelligkeit,
    /// Bei der Behörde vorgelegt am.
    pub vorgelegt: Option<NaiveDate>,
    /// Geprüft bzw. bescheinigt am (Prüfbericht, Bescheinigung).
    pub bescheinigt: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Dokumentbezug>>", optional)]
    pub dokumente: Vec<Dokumentbezug>,
}
impl BautechnischerNachweis {
    /// Erfüllt: vorgelegt und – falls geprüft wird – bescheinigt.
    pub fn erfuellt(&self) -> bool {
        self.vorgelegt.is_some() && (self.pruefung == Pruefart::Keine || self.bescheinigt.is_some())
    }
}
/// Bauleitung und Vorbereitung des Baubeginns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Bauvorbereitung {
    /// Bauleiterin oder Bauleiter.
    #[serde(default)]
    pub bauleitung: String,
    /// Der Behörde benannt am.
    pub bauleitung_benannt: Option<NaiveDate>,
    /// Grundrissfläche und Höhenlage abgesteckt am.
    pub absteckung: Option<NaiveDate>,
    /// Baubeginn angezeigt am.
    pub baubeginnanzeige: Option<NaiveDate>,
    /// Fertigstellung bzw. Nutzungsaufnahme angezeigt am.
    pub nutzungsanzeige: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Dokumentbezug>>", optional)]
    pub belege: Vec<Dokumentbezug>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Baubeginn,
    Nutzungsaufnahme,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Voraussetzung {
    pub titel: String,
    pub erfuellt: Option<bool>,
    pub hinweis: String,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Phasencheckliste {
    pub phase: Phase,
    pub voraussetzungen: Vec<Voraussetzung>,
    /// Alle belegt.
    pub bereit: bool,
    pub hinweise: Vec<String>,
}
/// Ist eine Folgenachricht dieser Art versendet worden?
fn folge_versendet(a: &Verfahrensakte, art: impl Fn(&abgabe::Folgewunsch) -> bool) -> bool {
    a.abgaben.iter().any(|b| {
        b.grundlagen.folge.as_ref().is_some_and(&art)
            && matches!(
                abgabe::versandstand(b),
                Some(abgabe::Versandstatus::Versendet | abgabe::Versandstatus::Eingegangen)
            )
    })
}
fn v(
    titel: impl Into<String>,
    erfuellt: Option<bool>,
    hinweis: impl Into<String>,
    quelle: &str,
) -> Voraussetzung {
    Voraussetzung {
        titel: titel.into(),
        erfuellt,
        hinweis: hinweis.into(),
        quelle: quelle.into(),
    }
}
/// Phasencheckliste für Baubeginn oder Nutzungsaufnahme aus der Akte. `planunterlagen`
/// liefert der Planindex (Tektur-Hinweis), `heute` ist der Stichtag.
pub fn checkliste(
    a: &Verfahrensakte,
    land: Bundesland,
    heute: NaiveDate,
    phase: Phase,
    planunterlagen: &[planindex::Planunterlage],
) -> Phasencheckliste {
    let nrw = land == Bundesland::Nw;
    let mut r = Vec::new();
    let mut hinweise = Vec::new();
    let genehmigung = termine::wirksame_bescheide(a).find(|b| {
        matches!(
            b.entscheidung,
            Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
        ) && b.verlaengerung.is_none()
    });
    let vb = a.bauvorbereitung.clone().unwrap_or_default();
    let faellig = match phase {
        Phase::Baubeginn => Faelligkeit::VorBaubeginn,
        Phase::Nutzungsaufnahme => Faelligkeit::VorNutzungsaufnahme,
    };
    if phase == Phase::Baubeginn {
        if a.art == Verfahrensart::Freistellung {
            let eingang = a
                .abgaben
                .iter()
                .flat_map(|b| &b.versand)
                .map(|e| e.datum)
                .chain(a.externe_einreichungen.iter().map(|e| e.datum))
                .min();
            let ab = eingang.and_then(|d| d.checked_add_months(chrono::Months::new(1)));
            r.push(v(
                "Wartefrist der Genehmigungsfreistellung abgelaufen",
                ab.map(|d| heute >= d),
                ab.map(|d| {
                    format!(
                        "Baubeginn frühestens am {}, wenn die Gemeinde kein Verfahren verlangt",
                        d.format("%d.%m.%Y")
                    )
                })
                .unwrap_or_else(|| "Eingang der Unterlagen erfassen".into()),
                if nrw {
                    "§ 63 BauO NRW"
                } else {
                    "Landesbauordnung (Genehmigungsfreistellung)"
                },
            ));
        } else {
            r.push(v(
                "Baugenehmigung zugegangen",
                Some(genehmigung.is_some_and(|b| b.zugestellt.is_some())),
                genehmigung
                    .and_then(|b| b.zugestellt)
                    .map(|d| format!("Zugegangen am {}", d.format("%d.%m.%Y")))
                    .unwrap_or_else(|| "Genehmigung mit Zustelldatum erfassen".into()),
                if nrw {
                    "§ 74 BauO NRW"
                } else {
                    "Landesbauordnung"
                },
            ));
            if let Some(b) = genehmigung {
                let ende = fristen::alle(land, &a.art, &a.bescheide)
                    .into_iter()
                    .find(|f| f.bescheid == b.id)
                    .and_then(|f| f.geltungsdauer.map(|g| g.ende));
                let verlaengert = fristen::verlaengert_durch(&a.bescheide, b.id)
                    .map(|v| v.verlaengerung.as_ref().map(|x| x.bis));
                let ende = verlaengert.flatten().or(ende);
                r.push(v(
                    "Geltungsdauer nicht abgelaufen",
                    ende.map(|e| heute <= e),
                    ende.map(|e| format!("Baubeginn bis {}", e.format("%d.%m.%Y")))
                        .unwrap_or_default(),
                    if nrw {
                        "§ 75 BauO NRW"
                    } else {
                        "Landesbauordnung"
                    },
                ));
            }
        }
        r.push(v(
            "Bauleitung benannt",
            Some(!vb.bauleitung.trim().is_empty() && vb.bauleitung_benannt.is_some()),
            if vb.bauleitung.trim().is_empty() {
                "Bauleiterin oder Bauleiter erfassen und der Behörde benennen".to_string()
            } else {
                vb.bauleitung.trim().to_string()
            },
            "Landesbauordnung (Bauleitung)",
        ));
        if nrw {
            r.push(v(
                "Grundrissfläche und Höhenlage abgesteckt",
                Some(vb.absteckung.is_some()),
                vb.absteckung
                    .map(|d| format!("Am {}", d.format("%d.%m.%Y")))
                    .unwrap_or_default(),
                "§ 74 BauO NRW",
            ));
        }
        let angezeigt = vb.baubeginnanzeige.is_some()
            || folge_versendet(a, |f| matches!(f, abgabe::Folgewunsch::Baubeginn { .. }));
        r.push(v(
            "Baubeginn angezeigt",
            Some(angezeigt),
            "Baubeginnsanzeige (XBau 0900 oder Formular); Frist der Landesbauordnung prüfen",
            "Landesbauordnung (Baubeginnsanzeige)",
        ));
        for n in a
            .nachbarbekanntgaben
            .iter()
            .filter(|n| n.angefochten.is_some())
        {
            hinweise.push(format!(
                "{} hat die Genehmigung angefochten; ein Baubeginn erfolgt auf eigenes Risiko (keine aufschiebende Wirkung, § 212a BauGB).",
                n.name
            ));
        }
    } else {
        r.push(v(
            "Baubeginn erfasst",
            Some(a.ausfuehrung.is_some()),
            "",
            "",
        ));
        r.push(v(
            "Fertigstellung bzw. Nutzungsaufnahme angezeigt",
            Some(
                vb.nutzungsanzeige.is_some()
                    || folge_versendet(a, |f| {
                        matches!(f, abgabe::Folgewunsch::Nutzungsaufnahme { .. })
                    }),
            ),
            "Anzeige nach der Landesbauordnung (XBau 0910 oder Formular); Frist prüfen",
            "Landesbauordnung",
        ));
    }
    for n in a
        .bautechnische_nachweise
        .iter()
        .filter(|n| n.faellig == faellig)
    {
        r.push(v(
            format!(
                "{}{}",
                n.art.bezeichnung(),
                if n.bezeichnung.trim().is_empty() {
                    String::new()
                } else {
                    format!(" – {}", n.bezeichnung.trim())
                }
            ),
            Some(n.erfuellt()),
            match (n.vorgelegt, n.bescheinigt, n.pruefung) {
                (None, _, _) => "Noch nicht vorgelegt".to_string(),
                (Some(_), None, p) if p != Pruefart::Keine => {
                    format!("Vorgelegt; Prüfung durch {} ausstehend", n.pruefer.trim())
                }
                (Some(d), _, _) => format!("Vorgelegt am {}", d.format("%d.%m.%Y")),
            },
            "Bautechnische Nachweise nach Landesbauordnung",
        ));
    }
    for (b, n) in termine::offene_pflichten(a) {
        if n.faelligkeit == Some(faellig) {
            r.push(v(
                format!("{}: {}", n.art.bezeichnung(), n.titel),
                Some(false),
                format!("Aus dem Bescheid vom {}", b.datum.format("%d.%m.%Y")),
                "Nebenbestimmung (§ 36 VwVfG)",
            ));
        }
    }
    let ohne = termine::offene_pflichten(a)
        .filter(|(_, n)| n.faelligkeit.is_none())
        .count();
    if ohne > 0 {
        hinweise.push(format!(
            "{ohne} offene Auflagen oder Bedingungen ohne Fälligkeit; im Register Bescheide zuordnen."
        ));
    }
    for u in planunterlagen.iter().filter(|u| {
        u.status == planindex::Einreichungsstatus::Genehmigt && u.offene_version.is_some()
    }) {
        hinweise.push(format!(
            "{}: Nach der Genehmigung gibt es eine neue Version. Wird abweichend vom genehmigten Stand gebaut, ist die Änderung zu beantragen (Änderungsantrag / Tektur).",
            u.name
        ));
    }
    let bereit = r.iter().all(|x| x.erfuellt == Some(true));
    Phasencheckliste {
        phase,
        voraussetzungen: r,
        bereit,
        hinweise,
    }
}
pub(super) fn pruefen(a: &Verfahrensakte, dokumente: &mut Vec<Dokumentbezug>) -> CoreResult<()> {
    use crate::plattform::text;
    let heute = crate::kalender::heute();
    if a.bautechnische_nachweise.len() > 200 {
        return Err(fehler("Höchstens 200 bautechnische Nachweise je Akte"));
    }
    for n in &a.bautechnische_nachweise {
        if n.bezeichnung.chars().count() > 500 || n.pruefer.chars().count() > 500 {
            return Err(fehler("Angaben zum Nachweis zu lang"));
        }
        if n.vorgelegt.is_some_and(|d| d > heute) || n.bescheinigt.is_some_and(|d| d > heute) {
            return Err(fehler("Vorlage oder Prüfung liegt in der Zukunft"));
        }
        if n.bescheinigt.is_some() && n.pruefung == Pruefart::Keine {
            return Err(fehler(
                "Eine Bescheinigung setzt eine prüfende Stelle voraus",
            ));
        }
        dokumente.extend(n.dokumente.iter().cloned());
    }
    if let Some(b) = &a.bauvorbereitung {
        if !b.bauleitung.trim().is_empty() {
            text(&b.bauleitung, 500)?;
        }
        if [
            b.bauleitung_benannt,
            b.absteckung,
            b.baubeginnanzeige,
            b.nutzungsanzeige,
        ]
        .iter()
        .flatten()
        .any(|d| *d > heute)
        {
            return Err(fehler("Angaben zur Bauvorbereitung liegen in der Zukunft"));
        }
        if b.bauleitung_benannt.is_some() && b.bauleitung.trim().is_empty() {
            return Err(fehler("Benannte Bauleitung braucht einen Namen"));
        }
        dokumente.extend(b.belege.iter().cloned());
    }
    Ok(())
}
impl CoreServices {
    pub async fn verfahren_checkliste(
        &self,
        p: ProjectId,
        vid: Uuid,
        phase: Phase,
    ) -> CoreResult<Phasencheckliste> {
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let land = self.get_project(p).await?.bundesland;
        let unterlagen = self.verfahren_planunterlagen(p, vid).await?;
        Ok(checkliste(
            &a,
            land,
            crate::kalender::heute(),
            phase,
            &unterlagen,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn akte() -> Verfahrensakte {
        let doc = Dokumentbezug {
            dokument_id: crate::domain::DocumentId(Uuid::new_v4()),
            version: 1,
        };
        let mut a: Verfahrensakte = serde_json::from_value(serde_json::json!({
            "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "entschieden",
            "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
            "aktenzeichen": "BA-1", "verantwortlich": null, "vorgaenger": null, "profil": null,
            "kommunikation": [], "nachforderungen": [], "anforderungen": [],
            "dokumentbeziehungen": [], "abgaben": [], "bescheide": [], "ereignisse": []
        }))
        .unwrap();
        a.bescheide.push(Bescheid {
            id: Uuid::new_v4(),
            entscheidung: Entscheidung::Genehmigt,
            datum: tag("2026-05-01"),
            zugestellt: Some(tag("2026-05-04")),
            original: doc.clone(),
            genehmigte_unterlagen: vec![doc],
            auflagen: vec![Auflage {
                id: Uuid::new_v4(),
                art: Nebenbestimmungsart::Auflage,
                titel: "Geprüfte Statik vorlegen".into(),
                ausloeser: String::new(),
                verantwortlich: None,
                frist: None,
                erledigt: false,
                nachweise: vec![],
                faelligkeit: Some(Faelligkeit::VorBaubeginn),
            }],
            ersetzt: None,
            verlaengerung: None,
            teilbaugenehmigung: false,
            abweichungsentscheidungen: vec![],
        });
        a
    }
    #[test]
    fn baubeginn_verlangt_bauleitung_nachweise_und_faellige_auflagen() {
        let mut a = akte();
        let c = checkliste(&a, Bundesland::Nw, tag("2026-06-01"), Phase::Baubeginn, &[]);
        assert!(!c.bereit);
        let titel: Vec<_> = c.voraussetzungen.iter().map(|v| v.titel.as_str()).collect();
        assert!(titel.contains(&"Baugenehmigung zugegangen"));
        assert!(titel.contains(&"Auflage: Geprüfte Statik vorlegen"));
        let geltung = c
            .voraussetzungen
            .iter()
            .find(|v| v.titel == "Geltungsdauer nicht abgelaufen")
            .unwrap();
        assert_eq!(geltung.erfuellt, Some(true));
        assert!(geltung.hinweis.contains("04.05.2029"));
        // Alles belegen.
        a.bescheide[0].auflagen[0].erledigt = true;
        a.bautechnische_nachweise.push(BautechnischerNachweis {
            id: Uuid::new_v4(),
            art: Nachweisart::Standsicherheit,
            bezeichnung: String::new(),
            pruefung: Pruefart::Sachverstaendiger,
            pruefer: "Dipl.-Ing. Prüf".into(),
            faellig: Faelligkeit::VorBaubeginn,
            vorgelegt: Some(tag("2026-05-20")),
            bescheinigt: None,
            dokumente: vec![],
        });
        a.bauvorbereitung = Some(Bauvorbereitung {
            bauleitung: "B. Leiter".into(),
            bauleitung_benannt: Some(tag("2026-05-25")),
            absteckung: Some(tag("2026-05-28")),
            baubeginnanzeige: Some(tag("2026-05-26")),
            ..Default::default()
        });
        let c = checkliste(&a, Bundesland::Nw, tag("2026-06-01"), Phase::Baubeginn, &[]);
        let statik = c
            .voraussetzungen
            .iter()
            .find(|v| v.titel == "Standsicherheit")
            .unwrap();
        assert_eq!(statik.erfuellt, Some(false), "Prüfung ausstehend");
        a.bautechnische_nachweise[0].bescheinigt = Some(tag("2026-05-30"));
        let c = checkliste(&a, Bundesland::Nw, tag("2026-06-01"), Phase::Baubeginn, &[]);
        assert!(c.bereit, "{:?}", c.voraussetzungen);
        // Nach Ablauf der Geltungsdauer nicht mehr.
        let c = checkliste(&a, Bundesland::Nw, tag("2029-06-01"), Phase::Baubeginn, &[]);
        assert!(!c.bereit);
    }
}
