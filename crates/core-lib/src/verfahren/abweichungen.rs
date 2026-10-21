//! Abweichungen, Ausnahmen, Befreiungen und Erleichterungen der Verfahrensakte.
//!
//! Der **Bedarf** wird aus dem aktuellen Planungsstand des Projekts abgeleitet:
//! Maß der baulichen Nutzung je Baugrundstück (Festsetzungen des
//! Bebauungsplans, § 19 Abs. 4 BauNVO, Lage im Baufenster) und Abstandsflächen
//! je Abstandsplan. Jeder Bedarf braucht vor der Abgabe einen Eintrag in der
//! Akte: beantragt (mit Vorschrift, Rechtsgrundlage und Begründung) oder
//! begründet als nicht erforderlich. So enthält kein Antrag eine
//! stillschweigende Abweichung; nach § 74 BauO NRW erfasst etwa die
//! Genehmigungsfiktion Abweichungen nur, soweit sie beantragt sind.
//!
//! Die Anwendung beurteilt nicht, ob eine Abweichung zugelassen werden kann.
//! Rechtsgrundlagen sind Vorschläge und fachlich zu prüfen.
use super::*;
use crate::domain::{Bundesland, Project};

/// Art der Zulassung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Abweichungsart {
    /// Abweichung von bauordnungsrechtlichen Anforderungen (z. B. § 69 BauO NRW).
    Abweichung,
    /// Ausnahme, die der Bebauungsplan vorsieht (§ 31 Abs. 1 BauGB).
    Ausnahme,
    /// Befreiung von Festsetzungen des Bebauungsplans (§ 31 Abs. 2 BauGB).
    Befreiung,
    /// Erleichterung für Sonderbauten (vgl. § 51 MBO).
    Erleichterung,
}
impl Abweichungsart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Abweichung => "Abweichung",
            Self::Ausnahme => "Ausnahme",
            Self::Befreiung => "Befreiung",
            Self::Erleichterung => "Erleichterung",
        }
    }
    /// Vorgeschlagene Rechtsgrundlage. Landesrecht wird nur für Länder mit
    /// geprüfter Fundstelle konkret benannt, sonst mit Verweis auf die
    /// Musterbauordnung.
    pub fn rechtsgrundlage(&self, land: Bundesland) -> String {
        match (self, land) {
            (Self::Abweichung, Bundesland::Nw) => "§ 69 BauO NRW".into(),
            (Self::Abweichung, _) => format!(
                "Abweichung nach der Landesbauordnung {} (vgl. § 67 MBO)",
                land.name()
            ),
            (Self::Ausnahme, _) => "§ 31 Abs. 1 BauGB".into(),
            (Self::Befreiung, _) => "§ 31 Abs. 2 BauGB".into(),
            (Self::Erleichterung, _) => format!(
                "Erleichterung für Sonderbauten nach der Landesbauordnung {} (vgl. § 51 MBO)",
                land.name()
            ),
        }
    }
    /// Ausnahmen und Befreiungen betreffen das Planungsrecht; sie schließen
    /// eine Genehmigungsfreistellung aus.
    pub fn planungsrechtlich(&self) -> bool {
        matches!(self, Self::Ausnahme | Self::Befreiung)
    }
    /// Kennung der XBau-Codeliste, aus der der Code der Art stammt.
    pub fn codeliste(&self) -> &'static str {
        match self {
            Self::Erleichterung => ERLEICHTERUNG_GEGENSTAND,
            _ => ABWEICHUNGART,
        }
    }
}
/// XBau-Codeliste der Abweichungsarten (Typ 3, XRepository).
pub const ABWEICHUNGART: &str = "urn:xoev-de:xbau:codeliste:abweichungart";
/// XBau-Codeliste der Erleichterungstatbestände (Typ 3, XRepository).
pub const ERLEICHTERUNG_GEGENSTAND: &str = "urn:xoev-de:xbau:codeliste:erleichterung.gegenstand";

/// Zustimmung eines Angrenzers zu einer Abweichung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Nachbarzustimmung {
    pub id: Uuid,
    pub name: String,
    /// Flurstück des Angrenzers, frei formuliert.
    #[serde(default)]
    pub flurstueck: String,
    /// `None` angefragt, `Some(true)` zugestimmt, `Some(false)` verweigert.
    pub zugestimmt: Option<bool>,
    pub datum: Option<NaiveDate>,
    /// Unterschriebene Zustimmung oder Ablehnung.
    pub beleg: Option<Dokumentbezug>,
}
/// Beantragte (oder begründet nicht erforderliche) Abweichung der Akte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Abweichung {
    pub id: Uuid,
    pub art: Abweichungsart,
    /// Betroffene Vorschrift, z. B. „§ 6 Abs. 5 BauO NRW (Tiefe der Abstandsfläche)“.
    pub vorschrift: String,
    /// Rechtsgrundlage der Zulassung, z. B. „§ 31 Abs. 2 BauGB“.
    pub rechtsgrundlage: String,
    /// Was weicht ab (Bauteil, Wand, Festsetzung).
    pub gegenstand: String,
    pub soll: String,
    pub ist: String,
    /// Warum die Zulassung gerechtfertigt ist (Schutzziel, nachbarliche und
    /// öffentliche Belange).
    pub begruendung: String,
    /// Schlüssel des abgeleiteten Bedarfs, den dieser Eintrag abdeckt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub herkunft: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Nachbarzustimmung>>", optional)]
    pub nachbarn: Vec<Nachbarzustimmung>,
    /// Begründende Unterlagen (Lageplanausschnitt, Gutachten).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Dokumentbezug>>", optional)]
    pub dokumente: Vec<Dokumentbezug>,
    /// Code der eingespielten XBau-Codeliste (siehe [`Abweichungsart::codeliste`]);
    /// leer = Vorschlag nach der Art.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(as = "Option<String>", optional)]
    pub xbau_code: String,
    /// Nicht (mehr) erforderlich, mit Begründung (z. B. Planänderung,
    /// Baulast). Der Eintrag bleibt in der Akte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub entfallen: Option<String>,
}
impl Abweichung {
    pub fn aktiv(&self) -> bool {
        self.entfallen.is_none()
    }
}
/// Entscheidung des Bescheids über eine beantragte Abweichung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Abweichungsentscheidung {
    pub abweichung: Uuid,
    pub zugelassen: bool,
    #[serde(default)]
    pub begruendung: String,
    /// Nebenbestimmung zur Zulassung, wörtlich aus dem Bescheid.
    #[serde(default)]
    pub nebenbestimmung: String,
}
/// Aus dem Projekt abgeleiteter Bedarf einer Zulassung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Abweichungsbedarf {
    /// Stabiler Schlüssel aus Herkunft und Gegenstand.
    pub schluessel: String,
    pub art: Abweichungsart,
    pub vorschrift: String,
    pub rechtsgrundlage: String,
    pub gegenstand: String,
    pub soll: String,
    pub ist: String,
    /// Nachweis, aus dem der Bedarf stammt.
    pub quelle: String,
}
/// Abgleich von Bedarf und Einträgen der Akte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Abweichungsabgleich {
    pub bedarf: Vec<Abweichungsbedarf>,
    /// Schlüssel des Bedarfs ohne Eintrag in der Akte.
    pub offen: Vec<String>,
    /// Einträge, deren Bedarf im aktuellen Planungsstand nicht mehr besteht.
    pub ohne_bedarf: Vec<Uuid>,
    /// Einträge, deren Soll oder Ist vom aktuellen Bedarf abweicht.
    pub geaendert: Vec<Uuid>,
    /// Nicht ermittelbare Bereiche und weitere Hinweise.
    pub hinweise: Vec<String>,
}

fn zahl(v: f64, stellen: usize) -> String {
    crate::formular::vorlage::zahl(v, stellen)
}
fn nutzung_vorschrift(art: &str) -> Option<(&'static str, usize, &'static str)> {
    // (Vorschrift, Nachkommastellen, Einheit)
    Some(match art {
        "grz" => (
            "Grundflächenzahl (§ 19 BauNVO), Festsetzung des Bebauungsplans",
            2,
            "",
        ),
        "grz_gesamt" => (
            "Zulässige Überschreitung der Grundfläche durch Garagen, Stellplätze und Nebenanlagen (§ 19 Abs. 4 BauNVO)",
            2,
            "",
        ),
        "gfz" => (
            "Geschossflächenzahl (§ 20 BauNVO), Festsetzung des Bebauungsplans",
            2,
            "",
        ),
        "bmz" => (
            "Baumassenzahl (§ 21 BauNVO), Festsetzung des Bebauungsplans",
            2,
            "",
        ),
        "grundflaeche" => ("Zulässige Grundfläche (§ 19 Abs. 2 BauNVO)", 2, " m²"),
        "geschossflaeche" => ("Zulässige Geschossfläche (§ 20 Abs. 2 BauNVO)", 2, " m²"),
        "vollgeschosse" => ("Zahl der Vollgeschosse (§ 16 Abs. 2 Nr. 3 BauNVO)", 0, ""),
        "baufenster_lage" => (
            "Überbaubare Grundstücksfläche, Baugrenzen und Baulinien (§ 23 BauNVO)",
            2,
            " m²",
        ),
        _ => return None,
    })
}
/// Bedarf aus der Nutzungsprüfung eines Baugrundstücks. Nur Festsetzungen
/// und § 19 Abs. 4 BauNVO sind Zulässigkeitsmaßstäbe; Orientierungs- und
/// Umgebungswerte (§ 34 BauGB) begründen keine Befreiung.
pub fn bedarf_aus_nutzung(
    grundstueck: &str,
    bezeichnung: &str,
    nachweise: &[crate::festsetzungen::Nachweis],
    land: Bundesland,
) -> Vec<Abweichungsbedarf> {
    use crate::festsetzungen::Quelle;
    nachweise
        .iter()
        .filter(|n| n.eingehalten == Some(false))
        .filter(|n| matches!(n.quelle, Quelle::Festsetzung | Quelle::BauNvo19))
        .filter_map(|n| {
            let (vorschrift, stellen, einheit) = nutzung_vorschrift(n.art)?;
            let gegenstand = n.gegenstand.clone().unwrap_or_default();
            let (soll, ist) = if n.art == "baufenster_lage" {
                (
                    "Vorhaben innerhalb der Baugrenzen".to_string(),
                    format!(
                        "{} außerhalb der überbaubaren Fläche",
                        n.ist
                            .map(|v| format!("{}{einheit}", zahl(v, stellen)))
                            .unwrap_or_else(|| "Fläche".into())
                    ),
                )
            } else {
                let f = |v: Option<f64>| {
                    v.map(|v| format!("{}{einheit}", zahl(v, stellen)))
                        .unwrap_or_else(|| "–".into())
                };
                (
                    format!(
                        "{}{}",
                        if n.zwingend {
                            "zwingend "
                        } else {
                            "höchstens "
                        },
                        f(n.soll)
                    ),
                    f(n.ist),
                )
            };
            let art = Abweichungsart::Befreiung;
            Some(Abweichungsbedarf {
                schluessel: format!("nutzung:{grundstueck}:{}:{gegenstand}", n.art),
                art,
                vorschrift: vorschrift.into(),
                rechtsgrundlage: art.rechtsgrundlage(land),
                gegenstand: [bezeichnung, gegenstand.as_str()]
                    .iter()
                    .filter(|t| !t.trim().is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · "),
                soll,
                ist,
                quelle: format!("Maß der baulichen Nutzung, {bezeichnung}"),
            })
        })
        .collect()
}
/// Bedarf aus dem Ergebnis eines Abstandsplans: Abstandsflächen jenseits der
/// maßgeblichen Grenze und unzulässige Überdeckungen.
pub fn bedarf_aus_abstand(
    plan_id: &str,
    plan: &crate::abstandsflaechen::AbstandsPlan,
    ergebnis: &crate::abstandsflaechen::AbstandsErgebnis,
    land: Bundesland,
) -> Vec<Abweichungsbedarf> {
    use crate::abstandsflaechen::AbstandsStatus;
    let art = Abweichungsart::Abweichung;
    let grundlage = if plan.regeln.rechtsstand.trim().is_empty() {
        format!("Landesbauordnung {}", land.name())
    } else {
        plan.regeln.rechtsstand.trim().to_string()
    };
    let mut r = Vec::new();
    for w in ergebnis
        .waende
        .iter()
        .filter(|w| w.status == AbstandsStatus::Konflikt)
    {
        r.push(Abweichungsbedarf {
            schluessel: format!("abstand:{plan_id}:{}", w.id),
            art,
            vorschrift: format!("Abstandsflächen, Lage auf dem Grundstück ({grundlage})"),
            rechtsgrundlage: art.rechtsgrundlage(land),
            gegenstand: format!("{} · {}", plan.bezeichnung.trim(), w.bezeichnung.trim()),
            soll: w
                .mindesttiefe_m
                .or(w.tiefe_m)
                .map(|t| format!("Tiefe {} m auf dem Baugrundstück", zahl(t, 2)))
                .unwrap_or_else(|| "Abstandsfläche auf dem Baugrundstück".into()),
            ist: w
                .ausserhalb_m2
                .map(|a| format!("{} m² jenseits der Grenze", zahl(a, 2)))
                .unwrap_or_else(|| "Abstandsfläche reicht über die Grenze".into()),
            quelle: format!("Abstandsflächen, {}", plan.bezeichnung.trim()),
        });
    }
    let name = |id: Uuid| {
        ergebnis
            .waende
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.bezeichnung.trim().to_string())
            .unwrap_or_else(|| id.to_string())
    };
    for u in ergebnis
        .ueberdeckungen
        .iter()
        .filter(|u| u.zulaessig.is_none())
    {
        r.push(Abweichungsbedarf {
            schluessel: format!("abstand:{plan_id}:ueberdeckung:{}:{}", u.wand_a, u.wand_b),
            art,
            vorschrift: format!("Abstandsflächen, Überdeckungsverbot ({grundlage})"),
            rechtsgrundlage: art.rechtsgrundlage(land),
            gegenstand: format!(
                "{} · {} / {}",
                plan.bezeichnung.trim(),
                name(u.wand_a),
                name(u.wand_b)
            ),
            soll: "keine Überdeckung".into(),
            ist: format!("{} m² Überdeckung", zahl(u.flaeche_m2, 2)),
            quelle: format!("Abstandsflächen, {}", plan.bezeichnung.trim()),
        });
    }
    r
}
/// Gleicht Bedarf und Akteneinträge ab.
pub fn abgleichen(
    a: &Verfahrensakte,
    bedarf: Vec<Abweichungsbedarf>,
    hinweise: Vec<String>,
) -> Abweichungsabgleich {
    let offen = bedarf
        .iter()
        .filter(|b| {
            !a.abweichungen
                .iter()
                .any(|e| e.herkunft.as_deref() == Some(b.schluessel.as_str()))
        })
        .map(|b| b.schluessel.clone())
        .collect();
    let mut ohne_bedarf = Vec::new();
    let mut geaendert = Vec::new();
    for e in a.abweichungen.iter().filter(|e| e.aktiv()) {
        let Some(h) = &e.herkunft else { continue };
        match bedarf.iter().find(|b| &b.schluessel == h) {
            None => ohne_bedarf.push(e.id),
            Some(b) if b.soll != e.soll || b.ist != e.ist => geaendert.push(e.id),
            Some(_) => {}
        }
    }
    Abweichungsabgleich {
        bedarf,
        offen,
        ohne_bedarf,
        geaendert,
        hinweise,
    }
}
/// Verfahrensarten, deren Antrag über Abweichungen entscheiden lässt.
pub fn betroffen(art: &Verfahrensart) -> bool {
    matches!(
        art,
        Verfahrensart::Bauantrag
            | Verfahrensart::Aenderungsantrag
            | Verfahrensart::Nutzungsaenderung
            | Verfahrensart::Teilbaugenehmigung
            | Verfahrensart::Freistellung
    )
}
/// Prüfung vor dem Festhalten eines Abgabestands. Fehler sperren die Abgabe;
/// zurück kommen Hinweise für den Stand.
pub fn abgabe_pruefen(a: &Verfahrensakte, g: &Abweichungsabgleich) -> CoreResult<Vec<String>> {
    let mut hinweise = g.hinweise.clone();
    if !betroffen(&a.art) {
        return Ok(vec![]);
    }
    if !g.offen.is_empty() {
        let liste = g
            .bedarf
            .iter()
            .filter(|b| g.offen.contains(&b.schluessel))
            .map(|b| {
                format!(
                    "{} ({}: Soll {}, Ist {})",
                    b.gegenstand, b.vorschrift, b.soll, b.ist
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(fehler(format!(
            "Abgabe gesperrt: Für folgende Unterschreitungen fehlt ein Eintrag unter Abweichungen (beantragen oder begründet als nicht erforderlich führen): {liste}"
        )));
    }
    for e in a.abweichungen.iter().filter(|e| e.aktiv()) {
        let mut fehlt = Vec::new();
        if e.vorschrift.trim().is_empty() {
            fehlt.push("betroffene Vorschrift");
        }
        if e.rechtsgrundlage.trim().is_empty() {
            fehlt.push("Rechtsgrundlage");
        }
        if e.begruendung.trim().is_empty() {
            fehlt.push("Begründung");
        }
        if !fehlt.is_empty() {
            return Err(fehler(format!(
                "Abgabe gesperrt: {} „{}“ braucht noch {}. Ein Antrag auf Zulassung ist zu begründen (z. B. § 69 Abs. 1 BauO NRW).",
                e.art.bezeichnung(),
                e.gegenstand.trim(),
                fehlt.join(", ")
            )));
        }
        if a.art == Verfahrensart::Freistellung && e.art.planungsrechtlich() {
            return Err(fehler(format!(
                "Abgabe gesperrt: Eine Genehmigungsfreistellung setzt voraus, dass das Vorhaben den Festsetzungen entspricht; die {} „{}“ ({}) ist dort nicht möglich. Bauantrag stellen oder die Planung anpassen.",
                e.art.bezeichnung(),
                e.gegenstand.trim(),
                e.rechtsgrundlage.trim()
            )));
        }
        if a.art == Verfahrensart::Freistellung {
            hinweise.push(format!(
                "{} „{}“: Bei einer Genehmigungsfreistellung gesondert beantragen (XBau-Nachricht abweichung.antrag.0220).",
                e.art.bezeichnung(),
                e.gegenstand.trim()
            ));
        }
        for n in e.nachbarn.iter().filter(|n| n.zugestimmt == Some(false)) {
            hinweise.push(format!(
                "{} „{}“: {} hat nicht zugestimmt; nachbarliche Belange in der Begründung würdigen.",
                e.art.bezeichnung(),
                e.gegenstand.trim(),
                n.name.trim()
            ));
        }
    }
    for id in &g.ohne_bedarf {
        if let Some(e) = a.abweichungen.iter().find(|e| e.id == *id) {
            hinweise.push(format!(
                "{} „{}“: Im aktuellen Planungsstand besteht keine Unterschreitung mehr. Prüfen, ob der Antrag entfallen kann.",
                e.art.bezeichnung(),
                e.gegenstand.trim()
            ));
        }
    }
    for id in &g.geaendert {
        if let Some(e) = a.abweichungen.iter().find(|e| e.id == *id) {
            hinweise.push(format!(
                "{} „{}“: Soll oder Ist hat sich seit der Erfassung geändert. Angaben und Begründung aktualisieren.",
                e.art.bezeichnung(),
                e.gegenstand.trim()
            ));
        }
    }
    Ok(hinweise)
}
/// Code der XBau-Codeliste für eine Abweichung: die ausdrückliche Wahl, sonst
/// der Eintrag, dessen Beschreibung die Art nennt. Ohne Liste kein Code.
pub fn xbau_code(
    e: &Abweichung,
    listen: &crate::xbau::codeliste::Codelisten,
) -> Option<(String, String, String)> {
    let liste = listen.liste(e.art.codeliste())?;
    let code = if !e.xbau_code.trim().is_empty() {
        liste
            .enthaelt(e.xbau_code.trim())
            .then(|| e.xbau_code.trim().to_string())?
    } else if e.art == Abweichungsart::Erleichterung {
        return None;
    } else {
        let wort = e.art.bezeichnung().to_lowercase();
        let treffer: Vec<_> = liste
            .eintraege
            .iter()
            .filter(|c| c.beschreibung.to_lowercase().contains(&wort))
            .collect();
        match treffer.as_slice() {
            [t] => t.code.clone(),
            [] => return None,
            _ => {
                // Mehrdeutig, z. B. „Ausnahme (Bebauungsplan)“ und „Ausnahme
                // (Veränderungssperre)“: nur die BauGB-Fundstelle entscheidet.
                // Landesrechtliche Abweichungen bleiben eine Wahl des Büros.
                let r = &e.rechtsgrundlage;
                let zusatz = if !r.contains("BauGB") {
                    return None;
                } else if nennt_paragraf(r, "31") {
                    "bebauungsplan"
                } else if nennt_paragraf(r, "14") {
                    "veränderungssperre"
                } else {
                    return None;
                };
                match treffer
                    .iter()
                    .filter(|c| c.beschreibung.to_lowercase().contains(zusatz))
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    [t] => t.code.clone(),
                    _ => return None,
                }
            }
        }
    };
    Some((liste.uri.clone(), liste.version.clone(), code))
}
/// `§ 31` in „§ 31 Abs. 1 BauGB“, nicht in „§ 310“.
fn nennt_paragraf(text: &str, nr: &str) -> bool {
    let muster = format!("§ {nr}");
    text.match_indices(&muster).any(|(i, _)| {
        !text[i + muster.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
    })
}
/// Codes, aus denen für eine Abweichung ohne eindeutigen Code zu wählen ist.
pub fn xbau_code_kandidaten(
    e: &Abweichung,
    listen: &crate::xbau::codeliste::Codelisten,
) -> Vec<crate::xbau::codeliste::Codeeintrag> {
    let Some(liste) = listen.liste(e.art.codeliste()) else {
        return vec![];
    };
    if e.art == Abweichungsart::Erleichterung {
        return liste.eintraege.clone();
    }
    let wort = e.art.bezeichnung().to_lowercase();
    liste
        .eintraege
        .iter()
        .filter(|c| c.beschreibung.to_lowercase().contains(&wort))
        .cloned()
        .collect()
}
impl CoreServices {
    /// Bedarf aus dem aktuellen Planungsstand. Nicht ermittelbare Bereiche
    /// werden als Hinweis genannt, nie als „kein Bedarf“ verschwiegen.
    pub async fn abweichungsbedarf(
        &self,
        p: ProjectId,
    ) -> CoreResult<(Vec<Abweichungsbedarf>, Vec<String>)> {
        let projekt = self.get_project(p).await?;
        let mut bedarf = Vec::new();
        let mut hinweise = Vec::new();
        self.bedarf_nutzung(&projekt, &mut bedarf, &mut hinweise)
            .await?;
        for f in self.repos.commit.fachstaende(p, "abstandsflaechen").await? {
            let Ok(plan) = serde_json::from_value::<crate::abstandsflaechen::AbstandsPlan>(f.daten)
            else {
                hinweise.push(format!(
                    "Abstandsplan {} nicht lesbar; Abweichungsbedarf nicht geprüft.",
                    f.objekt
                ));
                continue;
            };
            match crate::abstandsflaechen::berechnen(&plan) {
                Ok(e) => {
                    if !e.vollstaendig {
                        hinweise.push(format!(
                            "Abstandsflächen „{}“ sind nicht vollständig berechnet; weiterer Abweichungsbedarf ist möglich.",
                            plan.bezeichnung.trim()
                        ));
                    }
                    bedarf.extend(bedarf_aus_abstand(&f.objekt, &plan, &e, projekt.bundesland));
                }
                Err(e) => hinweise.push(format!(
                    "Abstandsflächen „{}“ nicht berechenbar ({e}); Abweichungsbedarf nicht geprüft.",
                    plan.bezeichnung.trim()
                )),
            }
        }
        Ok((bedarf, hinweise))
    }
    async fn bedarf_nutzung(
        &self,
        projekt: &Project,
        bedarf: &mut Vec<Abweichungsbedarf>,
        hinweise: &mut Vec<String>,
    ) -> CoreResult<()> {
        let (wirksam, gebaeude) = self.projekt_mit_gebaeuden(projekt).await?;
        hinweise.extend(gebaeude);
        if wirksam.planungsrecht.berechnung.is_none()
            && matches!(
                crate::grundstuecksinformationen::ermittlung(&wirksam.planungsrecht),
                Ok(None)
            )
        {
            hinweise.push(
                "Maß der baulichen Nutzung noch nicht berechnet; Befreiungsbedarf nicht geprüft."
                    .into(),
            );
            return Ok(());
        }
        let grundstuecke = match crate::grundstueckskontext::ermitteln(&wirksam) {
            Ok(g) => g,
            Err(e) => {
                hinweise.push(format!(
                    "Maß der baulichen Nutzung nicht prüfbar ({e}); Befreiungsbedarf nicht geprüft."
                ));
                return Ok(());
            }
        };
        for g in &grundstuecke {
            match crate::grundstueckskontext::nutzung(&wirksam, g) {
                Ok(Some((pruefung, _, _))) => bedarf.extend(bedarf_aus_nutzung(
                    &g.schluessel,
                    &g.bezeichnung,
                    &pruefung.nachweise,
                    wirksam.bundesland,
                )),
                Ok(None) => {}
                Err(e) => hinweise.push(format!(
                    "{}: Maß der baulichen Nutzung nicht prüfbar ({e}).",
                    g.bezeichnung
                )),
            }
        }
        Ok(())
    }
    /// Abgleich der Abweichungen einer Akte mit dem aktuellen Bedarf.
    pub async fn verfahren_abweichungen(
        &self,
        p: ProjectId,
        vid: Uuid,
    ) -> CoreResult<Abweichungsabgleich> {
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let (bedarf, hinweise) = self.abweichungsbedarf(p).await?;
        Ok(abgleichen(&a, bedarf, hinweise))
    }
}
/// Prüfung der Einträge beim Speichern der Akte.
pub(super) fn eintraege_pruefen(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    dokumente: &mut Vec<Dokumentbezug>,
) -> CoreResult<()> {
    use crate::plattform::text;
    let laenge = |s: &str, max: usize| {
        if s.chars().count() > max {
            Err(fehler(format!(
                "Angabe zur Abweichung höchstens {max} Zeichen"
            )))
        } else {
            Ok(())
        }
    };
    if a.abweichungen.len() > 200 {
        return Err(fehler("Höchstens 200 Abweichungen je Verfahrensakte"));
    }
    let mut herkunft = std::collections::HashSet::new();
    for e in &a.abweichungen {
        laenge(&e.vorschrift, 1000)?;
        laenge(&e.rechtsgrundlage, 1000)?;
        laenge(&e.gegenstand, 1000)?;
        laenge(&e.soll, 1000)?;
        laenge(&e.ist, 1000)?;
        laenge(&e.xbau_code, 100)?;
        if e.begruendung.len() > 32000 || e.nachbarn.len() > 100 || e.dokumente.len() > 100 {
            return Err(fehler("Abweichung zu umfangreich"));
        }
        if e.vorschrift.trim().is_empty() && e.gegenstand.trim().is_empty() {
            return Err(fehler(
                "Abweichung benötigt die betroffene Vorschrift oder den Gegenstand",
            ));
        }
        if let Some(h) = &e.herkunft {
            text(h, 500)?;
            if !herkunft.insert(h.clone()) {
                return Err(fehler(
                    "Ein abgeleiteter Bedarf wird von genau einem Eintrag abgedeckt",
                ));
            }
        }
        if let Some(g) = &e.entfallen
            && g.trim().is_empty()
        {
            return Err(fehler(
                "Nicht erforderliche Abweichung benötigt eine Begründung",
            ));
        }
        dokumente.extend(e.dokumente.iter().cloned());
        for n in &e.nachbarn {
            text(&n.name, 500)?;
            laenge(&n.flurstueck, 500)?;
            if n.name.trim().is_empty() {
                return Err(fehler("Angrenzer benötigt einen Namen"));
            }
            if n.zugestimmt.is_some() && n.datum.is_none() {
                return Err(fehler(
                    "Zustimmung oder Ablehnung eines Angrenzers benötigt ein Datum",
                ));
            }
            if n.datum.is_some_and(|d| d > crate::kalender::heute()) {
                return Err(fehler("Datum der Nachbarerklärung liegt in der Zukunft"));
            }
            dokumente.extend(n.beleg.iter().cloned());
        }
    }
    for b in &a.bescheide {
        let mut gesehen = std::collections::HashSet::new();
        for d in &b.abweichungsentscheidungen {
            if !a.abweichungen.iter().any(|e| e.id == d.abweichung) {
                return Err(fehler(
                    "Entscheidung bezieht sich auf eine unbekannte Abweichung",
                ));
            }
            if !gesehen.insert(d.abweichung) {
                return Err(fehler("Je Bescheid eine Entscheidung je Abweichung"));
            }
            laenge(&d.begruendung, 16000)?;
            laenge(&d.nebenbestimmung, 16000)?;
        }
    }
    if let Some(alt) = alt
        && alt
            .abweichungen
            .iter()
            .any(|v| !a.abweichungen.iter().any(|n| n.id == v.id))
    {
        return Err(fehler(
            "Abweichungen werden nicht gelöscht. Nicht mehr erforderliche begründet als entfallen führen.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::festsetzungen::{Nachweis, Quelle};
    fn nachweis(art: &'static str, ist: f64, soll: f64, quelle: Quelle) -> Nachweis {
        Nachweis {
            gegenstand: None,
            art,
            ist: Some(ist),
            soll: Some(soll),
            quelle,
            zwingend: false,
            eingehalten: Some(ist <= soll),
            abweichung: Some(ist - soll),
            ueberschreitung: Some((ist - soll).max(0.0)),
            ueberschreitung_prozent: None,
            flaechenueberschreitung_m2: None,
        }
    }
    fn akte(abweichungen: Vec<Abweichung>) -> Verfahrensakte {
        serde_json::from_value(serde_json::json!({
            "bezeichnung": "Bauantrag",
            "art": "bauantrag",
            "status": "vorbereitung",
            "behoerde": {"id": Uuid::new_v4(), "name": "Bauaufsicht", "kennung": "", "kontakt": ""},
            "aktenzeichen": "",
            "verantwortlich": null,
            "vorgaenger": null,
            "profil": null,
            "kommunikation": [],
            "nachforderungen": [],
            "anforderungen": [],
            "dokumentbeziehungen": [],
            "abgaben": [],
            "bescheide": [],
            "ereignisse": [],
            "abweichungen": abweichungen,
        }))
        .unwrap()
    }
    fn eintrag(herkunft: &str, begruendung: &str) -> Abweichung {
        Abweichung {
            id: Uuid::new_v4(),
            art: Abweichungsart::Befreiung,
            vorschrift: "GRZ".into(),
            rechtsgrundlage: "§ 31 Abs. 2 BauGB".into(),
            gegenstand: "Baugrundstück 1".into(),
            soll: "höchstens 0,40".into(),
            ist: "0,45".into(),
            begruendung: begruendung.into(),
            herkunft: Some(herkunft.into()),
            nachbarn: vec![],
            dokumente: vec![],
            xbau_code: String::new(),
            entfallen: None,
        }
    }
    #[test]
    fn nur_festsetzungen_begruenden_befreiungsbedarf() {
        let p = vec![
            nachweis("grz", 0.45, 0.4, Quelle::Festsetzung),
            nachweis("gfz", 1.5, 1.2, Quelle::Orientierungswert),
            nachweis("gfz", 0.5, 0.8, Quelle::Festsetzung),
            nachweis("grz_gesamt", 0.7, 0.6, Quelle::BauNvo19),
        ];
        let b = bedarf_aus_nutzung("g1", "Baugrundstück 1", &p, Bundesland::Nw);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].art, Abweichungsart::Befreiung);
        assert_eq!(b[0].rechtsgrundlage, "§ 31 Abs. 2 BauGB");
        assert_eq!(b[0].soll, "höchstens 0,40");
        assert_eq!(b[0].ist, "0,45");
        assert!(b[1].vorschrift.contains("§ 19 Abs. 4 BauNVO"));
        assert_ne!(b[0].schluessel, b[1].schluessel);
    }

    #[test]
    fn offener_bedarf_und_fehlende_begruendung_sperren_die_abgabe() {
        let p = vec![nachweis("grz", 0.45, 0.4, Quelle::Festsetzung)];
        let bedarf = bedarf_aus_nutzung("g1", "Baugrundstück 1", &p, Bundesland::Nw);
        let schluessel = bedarf[0].schluessel.clone();

        let leer = akte(vec![]);
        let g = abgleichen(&leer, bedarf.clone(), vec![]);
        assert_eq!(g.offen, vec![schluessel.clone()]);
        let e = abgabe_pruefen(&leer, &g).unwrap_err().to_string();
        assert!(e.contains("Abgabe gesperrt"), "{e}");

        let ohne = akte(vec![eintrag(&schluessel, " ")]);
        let g = abgleichen(&ohne, bedarf.clone(), vec![]);
        assert!(g.offen.is_empty());
        let e = abgabe_pruefen(&ohne, &g).unwrap_err().to_string();
        assert!(e.contains("Begründung"), "{e}");

        let ok = akte(vec![eintrag(&schluessel, "Schutzziel gewahrt")]);
        let g = abgleichen(&ok, bedarf.clone(), vec![]);
        assert!(abgabe_pruefen(&ok, &g).unwrap().is_empty());

        // Begründet entfallen deckt den Bedarf ebenfalls ab.
        let mut weg = eintrag(&schluessel, "");
        weg.entfallen = Some("Baulast übernommen".into());
        let entfallen = akte(vec![weg]);
        let g = abgleichen(&entfallen, bedarf.clone(), vec![]);
        assert!(abgabe_pruefen(&entfallen, &g).is_ok());

        // Freistellung schließt Befreiungen aus.
        let mut frei = ok.clone();
        frei.art = Verfahrensart::Freistellung;
        let g = abgleichen(&frei, bedarf, vec![]);
        let e = abgabe_pruefen(&frei, &g).unwrap_err().to_string();
        assert!(e.contains("Genehmigungsfreistellung"), "{e}");
    }

    #[test]
    fn geaenderter_und_entfallener_bedarf_wird_gemeldet() {
        let p = vec![nachweis("grz", 0.48, 0.4, Quelle::Festsetzung)];
        let bedarf = bedarf_aus_nutzung("g1", "Baugrundstück 1", &p, Bundesland::Nw);
        let e = eintrag(&bedarf[0].schluessel, "Begründung");
        let veraltet = eintrag("nutzung:g1:gfz:", "Begründung");
        let a = akte(vec![e.clone(), veraltet.clone()]);
        let g = abgleichen(&a, bedarf, vec![]);
        assert_eq!(g.geaendert, vec![e.id]);
        assert_eq!(g.ohne_bedarf, vec![veraltet.id]);
        let h = abgabe_pruefen(&a, &g).unwrap();
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn xbau_code_nur_aus_eingespielter_liste() {
        let mut listen = crate::xbau::codeliste::Codelisten::default();
        let e = eintrag("x", "b");
        assert!(xbau_code(&e, &listen).is_none());
        listen.hinzufuegen(crate::xbau::codeliste::Codeliste {
            uri: ABWEICHUNGART.into(),
            version: "2.0".into(),
            name: "Abweichung Art".into(),
            eintraege: vec![
                crate::xbau::codeliste::Codeeintrag {
                    code: "10".into(),
                    beschreibung: "Abweichung nach Landesrecht".into(),
                },
                crate::xbau::codeliste::Codeeintrag {
                    code: "20".into(),
                    beschreibung: "Befreiung von Festsetzungen".into(),
                },
            ],
        });
        assert_eq!(xbau_code(&e, &listen).unwrap().2, "20");
        let mut gewaehlt = e.clone();
        gewaehlt.xbau_code = "10".into();
        assert_eq!(xbau_code(&gewaehlt, &listen).unwrap().2, "10");
        gewaehlt.xbau_code = "99".into();
        assert!(xbau_code(&gewaehlt, &listen).is_none());
    }

    #[test]
    fn mitgelieferte_listen_liefern_codes_fuer_ausnahme_und_befreiung() {
        let mut listen = crate::xbau::codeliste::Codelisten::default();
        for xml in [
            include_str!("../../../../codelisten/xbau-abweichungart_2.1.xml"),
            include_str!("../../../../codelisten/xbau-erleichterung.gegenstand_1.0.xml"),
        ] {
            listen.hinzufuegen(crate::xbau::codeliste::Codeliste::genericode(xml).unwrap());
        }
        let mit = |art: Abweichungsart, rechtsgrundlage: &str| {
            let mut e = eintrag("x", "b");
            e.art = art;
            e.rechtsgrundlage = rechtsgrundlage.into();
            e
        };
        let befreiung = xbau_code(
            &mit(Abweichungsart::Befreiung, "§ 31 Abs. 2 BauGB"),
            &listen,
        );
        assert_eq!(
            befreiung,
            Some((ABWEICHUNGART.into(), "2.1".into(), "1".into()))
        );
        let code = |e: &Abweichung| xbau_code(e, &listen).map(|c| c.2);
        assert_eq!(
            code(&mit(Abweichungsart::Ausnahme, "§ 31 Abs. 1 BauGB")).as_deref(),
            Some("2")
        );
        assert_eq!(
            code(&mit(Abweichungsart::Ausnahme, "§ 14 Abs. 2 BauGB")).as_deref(),
            Some("5")
        );
        assert_eq!(code(&mit(Abweichungsart::Ausnahme, "§ 310 BauGB")), None);
        // Landesrechtliche Abweichung: Codes 3 und 4 (MBO) bleiben Wahl des Büros.
        let abw = mit(Abweichungsart::Abweichung, "§ 69 BauO NRW");
        assert_eq!(code(&abw), None);
        let k: Vec<_> = xbau_code_kandidaten(&abw, &listen)
            .into_iter()
            .map(|c| c.code)
            .collect();
        assert_eq!(k, ["3", "4"]);
        let mut erl = mit(Abweichungsart::Erleichterung, "Sonderbau");
        assert_eq!(code(&erl), None);
        assert_eq!(xbau_code_kandidaten(&erl, &listen).len(), 24);
        erl.xbau_code = "19".into();
        assert_eq!(
            xbau_code(&erl, &listen),
            Some((ERLEICHTERUNG_GEGENSTAND.into(), "1.0".into(), "19".into()))
        );
    }

    #[test]
    fn entscheidung_muss_auf_bekannte_abweichung_zeigen() {
        let e = eintrag("x", "b");
        let mut a = akte(vec![e.clone()]);
        let mut docs = vec![];
        assert!(eintraege_pruefen(&a, None, &mut docs).is_ok());
        let mut entfallen = a.clone();
        entfallen.abweichungen[0].entfallen = Some(" ".into());
        assert!(eintraege_pruefen(&entfallen, None, &mut docs).is_err());
        let geloescht = akte(vec![]);
        assert!(eintraege_pruefen(&geloescht, Some(&a), &mut docs).is_err());
        a.abweichungen.push(eintrag("x", "doppelt"));
        assert!(eintraege_pruefen(&a, None, &mut docs).is_err());
    }
}
