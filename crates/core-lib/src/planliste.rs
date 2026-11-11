//! Planliste: Plannummern, Büroindex mit Änderungstext und Planverteiler.
//!
//! Vertrag:
//! - Je Planunterlage (Dokument im DMS) höchstens ein [`Planeintrag`]
//!   (`ObjektArt::Planliste`, Objektkennung = Dokument-ID).
//! - Die Plannummer setzt der Core beim Speichern aus dem Büroschema
//!   ([`Plannummernschema`]) und den Angaben Projektkennzeichen, Planart,
//!   Geschoss und laufender Nummer; sie ist im Projekt eindeutig.
//! - Der **Büroindex** (Revisionen mit Änderungstext und Dokumentversion) ist die
//!   Planfortschreibung des Büros. Der **Behördenindex** A, B, C … bleibt die
//!   Ableitung aus versendeten Abgaben ([`crate::verfahren::planindex`]); die
//!   Planliste zeigt beide nebeneinander und rechnet keinen davon um.
//! - Der Planverteiler besteht aus erfassten Versendungen und den aus der
//!   Verfahrensakte abgeleiteten Abgaben an die Behörde (nur lesend).
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    domain::{DocumentId, Project, ProjectId},
    plattform::{
        ObjektArt,
        objekte::{Dokumentbezug, Projektorganisation},
    },
    verfahren::{
        abgabe::Versandstatus,
        planindex::{Einreichungsstatus, ableiten},
    },
    zugriff::ProjektRecht,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

const SCHEMA: &str = "planliste.schema";
/// Vorgabe: Projektkennzeichen, Planart, Geschoss, zweistellige Nummer.
pub const STANDARDMUSTER: &str = "{projekt}-{planart}-{geschoss}-{nr:2}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Planart {
    Lageplan,
    Grundriss,
    Schnitt,
    Ansicht,
    Detail,
    Entwaesserung,
    Freiflaechen,
    Brandschutz,
    Sonstiges,
}
impl Planart {
    pub const ALLE: [Planart; 9] = [
        Self::Lageplan,
        Self::Grundriss,
        Self::Schnitt,
        Self::Ansicht,
        Self::Detail,
        Self::Entwaesserung,
        Self::Freiflaechen,
        Self::Brandschutz,
        Self::Sonstiges,
    ];
    pub fn kuerzel(self) -> &'static str {
        match self {
            Self::Lageplan => "LP",
            Self::Grundriss => "GR",
            Self::Schnitt => "SN",
            Self::Ansicht => "AN",
            Self::Detail => "DT",
            Self::Entwaesserung => "EW",
            Self::Freiflaechen => "FF",
            Self::Brandschutz => "BS",
            Self::Sonstiges => "SO",
        }
    }
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Lageplan => "Lageplan",
            Self::Grundriss => "Grundriss",
            Self::Schnitt => "Schnitt",
            Self::Ansicht => "Ansicht",
            Self::Detail => "Detail",
            Self::Entwaesserung => "Entwässerungsplan",
            Self::Freiflaechen => "Freiflächenplan",
            Self::Brandschutz => "Brandschutzplan",
            Self::Sonstiges => "Sonstiger Plan",
        }
    }
}

/// Büroweites Muster der Plannummer. Platzhalter: `{projekt}` (Kennzeichen der
/// Projektorganisation), `{planart}` (Kürzel), `{geschoss}`, `{nr}`, `{nr:2}`,
/// `{nr:3}` (laufende Nummer, mit führenden Nullen).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Plannummernschema {
    pub muster: String,
}
impl Default for Plannummernschema {
    fn default() -> Self {
        Self {
            muster: STANDARDMUSTER.into(),
        }
    }
}
impl Plannummernschema {
    pub fn pruefen(&self) -> CoreResult<()> {
        let m = &self.muster;
        if m.trim().is_empty() || m.chars().count() > 100 {
            return Err(CoreError::InvalidInput(
                "Muster der Plannummer mit 1 bis 100 Zeichen erforderlich".into(),
            ));
        }
        if !["{nr}", "{nr:2}", "{nr:3}"].iter().any(|n| m.contains(n)) {
            return Err(CoreError::InvalidInput(
                "Das Muster braucht eine laufende Nummer ({nr}, {nr:2} oder {nr:3})".into(),
            ));
        }
        let mut rest = m.as_str();
        while let Some(a) = rest.find('{') {
            let Some(e) = rest[a..].find('}') else {
                return Err(CoreError::InvalidInput(
                    "Platzhalter nicht geschlossen".into(),
                ));
            };
            let name = &rest[a + 1..a + e];
            if !["projekt", "planart", "geschoss", "nr", "nr:2", "nr:3"].contains(&name) {
                return Err(CoreError::InvalidInput(format!(
                    "Unbekannter Platzhalter {{{name}}}"
                )));
            }
            rest = &rest[a + e + 1..];
        }
        Ok(())
    }
    /// Setzt die Plannummer zusammen. Leere Teile entfallen samt einem
    /// angrenzenden Trennzeichen (`-`, `_`, `.`, `/`, Leerzeichen).
    pub fn plannummer(&self, projekt: &str, planart: Planart, geschoss: &str, nr: u32) -> String {
        let s = self
            .muster
            .replace("{projekt}", projekt.trim())
            .replace("{planart}", planart.kuerzel())
            .replace("{geschoss}", geschoss.trim())
            .replace("{nr:3}", &format!("{nr:03}"))
            .replace("{nr:2}", &format!("{nr:02}"))
            .replace("{nr}", &nr.to_string());
        let trenner = |c: char| matches!(c, '-' | '_' | '.' | '/' | ' ');
        let mut r = String::new();
        for c in s.chars() {
            if trenner(c) && (r.is_empty() || r.ends_with(trenner)) {
                continue;
            }
            r.push(c);
        }
        r.trim_end_matches(trenner).to_string()
    }
}

/// Stand der Planfortschreibung des Büros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planrevision {
    /// Büroindex, z. B. „a“, „b“ oder „01“.
    pub index: String,
    pub datum: NaiveDate,
    /// Was sich gegenüber dem vorigen Index geändert hat.
    pub aenderung: String,
    /// Dokumentversion im DMS, die diesen Index trägt.
    #[ts(type = "number")]
    pub version: i64,
}

/// Erfasste Weitergabe eines Plans (Bauherr, Fachplaner, Prüfingenieur …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planversand {
    pub id: Uuid,
    pub empfaenger: String,
    pub datum: NaiveDate,
    /// Weitergegebener Büroindex.
    pub index: String,
    /// E-Mail, Post, Plattform, persönlich …
    pub weg: String,
    pub bemerkung: String,
}

/// Eintrag der Planliste (ein Plan = ein Dokument im DMS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planeintrag {
    pub dokument_id: DocumentId,
    pub planart: Planart,
    /// Geschoss oder Bereich, z. B. „EG“, „OG1“, „Nord“; darf leer sein.
    pub geschoss: String,
    /// Laufende Nummer, ab 1.
    pub nummer: u32,
    pub titel: String,
    /// Maßstab als Nenner (100 für 1:100); `None` ohne Maßstab.
    pub massstab: Option<u32>,
    /// Vom Core beim Speichern gesetzt.
    #[serde(default)]
    pub plannummer: String,
    /// Aufsteigend nach Datum.
    #[serde(default)]
    pub revisionen: Vec<Planrevision>,
    #[serde(default)]
    pub verteiler: Vec<Planversand>,
}

/// Behördenstand eines Plans je Verfahren (abgeleitet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Behoerdenstand {
    pub verfahren: Uuid,
    pub bezeichnung: String,
    pub planindex: Option<String>,
    pub status: Einreichungsstatus,
}

/// Zeile der Planliste mit abgeleiteten Angaben.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Planzeile {
    pub eintrag: Planeintrag,
    #[ts(type = "number")]
    pub revision: i64,
    pub dateiname: String,
    /// Neueste Version im DMS; `None`, wenn das Dokument entfernt wurde.
    #[ts(type = "number | null")]
    pub aktuelle_version: Option<i64>,
    /// Neueste Planrevision des Büros.
    pub index: Option<Planrevision>,
    /// Im DMS liegt eine neuere Version als die des letzten Index.
    pub ohne_index: bool,
    pub behoerde: Vec<Behoerdenstand>,
    /// Erfasste Versendungen und Abgaben an die Behörde, neueste zuerst.
    pub versand: Vec<Planversand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Planliste {
    pub schema: Plannummernschema,
    pub kennzeichen: String,
    pub zeilen: Vec<Planzeile>,
    /// Bei der Behörde eingereichte Pläne, die die Planliste noch nicht führt.
    #[serde(default)]
    pub ohne_eintrag: Vec<PlanOhneEintrag>,
}

/// Eingereichter Plan ohne Eintrag in der Planliste (abgeleitet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct PlanOhneEintrag {
    pub dokument_id: DocumentId,
    pub dateiname: String,
    pub behoerde: Vec<Behoerdenstand>,
}

/// Dokumentkategorien, deren Unterlagen Pläne sind.
const PLANKATEGORIEN: &[&str] = &["lageplan", "grundriss", "schnitt", "ansicht"];

/// Nächste freie laufende Nummer je Planart (für Vorschläge).
pub fn naechste_nummer(eintraege: &[Planeintrag], planart: Planart) -> u32 {
    eintraege
        .iter()
        .filter(|e| e.planart == planart)
        .map(|e| e.nummer)
        .max()
        .unwrap_or(0)
        + 1
}

fn fehler(t: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(t.into())
}

/// Prüft einen Eintrag ohne Speicherzugriff und setzt die Plannummer.
pub fn eintrag_pruefen(
    mut e: Planeintrag,
    id: Uuid,
    schema: &Plannummernschema,
    kennzeichen: &str,
    andere: &[Planeintrag],
    versionen: &[i64],
) -> CoreResult<Planeintrag> {
    if e.dokument_id.0 != id {
        return Err(fehler(
            "Planeintrag benötigt die Identität seines Dokuments",
        ));
    }
    crate::plattform::text(&e.titel, 300)?;
    if e.geschoss.chars().count() > 30 {
        return Err(fehler("Geschossangabe zu lang"));
    }
    if e.nummer == 0 || e.nummer > 9999 {
        return Err(fehler("Laufende Nummer von 1 bis 9999 erforderlich"));
    }
    if e.massstab.is_some_and(|m| m == 0 || m > 100_000) {
        return Err(fehler("Maßstab als Nenner zwischen 1 und 100000 angeben"));
    }
    if e.revisionen.len() > 200 || e.verteiler.len() > 1000 {
        return Err(fehler("Zu viele Revisionen oder Versendungen"));
    }
    let heute = crate::kalender::heute();
    let mut indizes = std::collections::HashSet::new();
    let mut vorher: Option<&Planrevision> = None;
    for r in &e.revisionen {
        crate::plattform::text(&r.index, 10)?;
        if r.aenderung.chars().count() > 2000 {
            return Err(fehler("Änderungstext zu lang"));
        }
        if !indizes.insert(r.index.trim().to_lowercase()) {
            return Err(fehler(format!("Index {} doppelt vergeben", r.index)));
        }
        if r.datum > heute {
            return Err(fehler("Das Datum eines Index liegt nicht in der Zukunft"));
        }
        if !versionen.contains(&r.version) {
            return Err(fehler(format!(
                "Index {}: Dokumentversion {} gibt es nicht",
                r.index, r.version
            )));
        }
        if let Some(v) = vorher
            && (r.datum < v.datum || r.version < v.version)
        {
            return Err(fehler(
                "Revisionen aufsteigend nach Datum und Dokumentversion erfassen",
            ));
        }
        vorher = Some(r);
    }
    let mut ids = std::collections::HashSet::new();
    for v in &e.verteiler {
        crate::plattform::text(&v.empfaenger, 300)?;
        if !ids.insert(v.id) {
            return Err(fehler("Versendung doppelt erfasst"));
        }
        if v.datum > heute {
            return Err(fehler(
                "Das Datum einer Versendung liegt nicht in der Zukunft",
            ));
        }
        if !v.index.is_empty() && !indizes.contains(&v.index.trim().to_lowercase()) {
            return Err(fehler(format!(
                "Versendung an {}: Index {} ist nicht erfasst",
                v.empfaenger, v.index
            )));
        }
        if v.weg.chars().count() > 100 || v.bemerkung.chars().count() > 2000 {
            return Err(fehler("Angaben zur Versendung zu lang"));
        }
    }
    e.plannummer = schema.plannummer(kennzeichen, e.planart, &e.geschoss, e.nummer);
    if let Some(a) = andere
        .iter()
        .find(|a| a.dokument_id != e.dokument_id && a.plannummer == e.plannummer)
    {
        return Err(fehler(format!(
            "Plannummer {} ist schon an „{}“ vergeben",
            e.plannummer, a.titel
        )));
    }
    Ok(e)
}

/// Fügt Planeintrag, DMS-Stand und Behördenstände zu einer Zeile zusammen.
pub fn zeile(
    eintrag: Planeintrag,
    revision: i64,
    dateiname: String,
    aktuelle_version: Option<i64>,
    behoerde: Vec<Behoerdenstand>,
    abgaben: Vec<Planversand>,
) -> Planzeile {
    let index = eintrag.revisionen.last().cloned();
    let ohne_index = match (aktuelle_version, &index) {
        (Some(a), Some(i)) => a > i.version,
        (Some(_), None) => true,
        _ => false,
    };
    let mut versand: Vec<Planversand> = eintrag.verteiler.iter().cloned().chain(abgaben).collect();
    versand.sort_by(|a, b| b.datum.cmp(&a.datum).then(a.empfaenger.cmp(&b.empfaenger)));
    Planzeile {
        eintrag,
        revision,
        dateiname,
        aktuelle_version,
        index,
        ohne_index,
        behoerde,
        versand,
    }
}

impl CoreServices {
    pub async fn plannummernschema(&self) -> CoreResult<Plannummernschema> {
        match self.repos.lizenz.einstellung(SCHEMA).await? {
            Some(t) if !t.is_empty() => {
                serde_json::from_str(&t).map_err(|e| CoreError::Database(e.to_string()))
            }
            _ => Ok(Plannummernschema::default()),
        }
    }
    /// Ändert das Büroschema. Bestehende Plannummern bleiben, bis ein Eintrag
    /// erneut gespeichert wird.
    pub async fn plannummernschema_speichern(
        &self,
        s: Plannummernschema,
        wer: &Benutzer,
    ) -> CoreResult<Plannummernschema> {
        self.pruefe(wer, Recht::BueroangabenAendern).await?;
        s.pruefen()?;
        let t = serde_json::to_string(&s).map_err(|e| fehler(e.to_string()))?;
        self.repos.lizenz.einstellung_setzen(SCHEMA, &t).await?;
        Ok(s)
    }
    async fn projektkennzeichen(&self, p: &Project) -> CoreResult<String> {
        Ok(self
            .fachobjekt_lesen(p.id, ObjektArt::Projektorganisation, p.id.0)
            .await?
            .and_then(|f| serde_json::from_value::<Projektorganisation>(f.daten).ok())
            .map(|o| o.kennzeichen)
            .filter(|k| !k.trim().is_empty())
            .unwrap_or_else(|| p.name.clone()))
    }
    pub(crate) async fn planeintraege(&self, p: ProjectId) -> CoreResult<Vec<(i64, Planeintrag)>> {
        self.repos
            .commit
            .fachstaende(p, ObjektArt::Planliste.modul())
            .await?
            .into_iter()
            .map(|f| {
                serde_json::from_value(f.daten)
                    .map(|e| (f.revision, e))
                    .map_err(|e| fehler(e.to_string()))
            })
            .collect()
    }
    /// Prüfung beim Speichern über die Fachobjekt-Schnittstelle.
    pub(crate) async fn planeintrag_pruefen(
        &self,
        p: &Project,
        id: Uuid,
        daten: serde_json::Value,
    ) -> CoreResult<serde_json::Value> {
        let e: Planeintrag = serde_json::from_value(daten).map_err(|e| fehler(e.to_string()))?;
        let versionen: Vec<i64> = self
            .repos
            .documents
            .history(e.dokument_id)
            .await?
            .into_iter()
            .filter(|v| v.project_id == p.id)
            .map(|v| v.version)
            .collect();
        if versionen.is_empty() {
            return Err(fehler("Dokument gehört nicht zu diesem Projekt"));
        }
        let schema = self.plannummernschema().await?;
        let kennzeichen = self.projektkennzeichen(p).await?;
        let andere: Vec<Planeintrag> = self
            .planeintraege(p.id)
            .await?
            .into_iter()
            .map(|(_, e)| e)
            .collect();
        let e = eintrag_pruefen(e, id, &schema, &kennzeichen, &andere, &versionen)?;
        serde_json::to_value(e).map_err(|e| fehler(e.to_string()))
    }
    /// Planliste des Projekts mit DMS-Stand, Behördenindex und Verteiler.
    pub async fn planliste(&self, p: ProjectId, wer: &Benutzer) -> CoreResult<Planliste> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let projekt = self.repos.projects.get(p).await?;
        let akten = self.verfahrensakten_lesen(p).await?;
        let mut dms: HashMap<DocumentId, (i64, String)> = HashMap::new();
        let eintraege = self.planeintraege(p).await?;
        for (_, e) in &eintraege {
            if let Ok(d) = self.repos.documents.latest(e.dokument_id).await {
                dms.insert(e.dokument_id, (d.version, d.file_name));
            }
        }
        let versionen: HashMap<DocumentId, i64> = dms.iter().map(|(k, (v, _))| (*k, *v)).collect();
        let abgeleitet: Vec<(Uuid, String, Vec<_>)> = akten
            .iter()
            .map(|(id, a)| (*id, a.bezeichnung.clone(), ableiten(a, &versionen)))
            .collect();
        let mut zeilen = Vec::new();
        for (rev, e) in eintraege {
            let behoerde = abgeleitet
                .iter()
                .filter_map(|(vid, name, us)| {
                    us.iter()
                        .find(|u| u.dokument_id == e.dokument_id)
                        .map(|u| Behoerdenstand {
                            verfahren: *vid,
                            bezeichnung: name.clone(),
                            planindex: u.planindex.clone(),
                            status: u.status,
                        })
                })
                .collect();
            let mut abgaben = Vec::new();
            for (_, a) in &akten {
                for s in &a.abgaben {
                    let Some(d) = s
                        .dokumente
                        .iter()
                        .find(|d| d.dokument.dokument_id == e.dokument_id)
                    else {
                        continue;
                    };
                    for v in s.versand.iter().filter(|v| {
                        matches!(
                            v.status,
                            Versandstatus::Versendet | Versandstatus::Eingegangen
                        ) && v.berichtigt.is_none()
                    }) {
                        let buero = e
                            .revisionen
                            .iter()
                            .rev()
                            .find(|r| r.version <= d.dokument.version)
                            .map(|r| r.index.clone())
                            .unwrap_or_default();
                        abgaben.push(Planversand {
                            id: v.id,
                            empfaenger: if a.behoerde.name.is_empty() {
                                "Behörde".into()
                            } else {
                                a.behoerde.name.clone()
                            },
                            datum: v.datum,
                            index: buero,
                            weg: v.weg.clone(),
                            bemerkung: format!(
                                "{} · Abgabe „{}“ · Version {}{}",
                                a.bezeichnung,
                                s.titel,
                                d.dokument.version,
                                d.planindex
                                    .as_ref()
                                    .map(|i| format!(" · Behördenindex {i}"))
                                    .unwrap_or_default()
                            ),
                        });
                    }
                }
            }
            let (aktuell, name) = dms
                .get(&e.dokument_id)
                .map(|(v, n)| (Some(*v), n.clone()))
                .unwrap_or((None, String::new()));
            zeilen.push(zeile(e, rev, name, aktuell, behoerde, abgaben));
        }
        zeilen.sort_by(|a, b| a.eintrag.plannummer.cmp(&b.eintrag.plannummer));
        // Eine Planliste: Was als Plan bei der Behörde liegt, gehört hinein.
        let kategorien = self.kategoriesnapshots(p).await?;
        let mut ohne_eintrag: Vec<PlanOhneEintrag> = Vec::new();
        for (vid, name, us) in &abgeleitet {
            for u in us {
                if u.status == Einreichungsstatus::Entwurf
                    || zeilen
                        .iter()
                        .any(|z| z.eintrag.dokument_id == u.dokument_id)
                {
                    continue;
                }
                let plan = kategorien
                    .get(&u.dokument_id.0.to_string())
                    .cloned()
                    .flatten()
                    .is_some_and(|k| {
                        PLANKATEGORIEN.contains(&k.id.as_str())
                            || k.hauptkategorie
                                .is_some_and(|h| PLANKATEGORIEN.contains(&h.as_str()))
                    });
                if !plan {
                    continue;
                }
                let stand = Behoerdenstand {
                    verfahren: *vid,
                    bezeichnung: name.clone(),
                    planindex: u.planindex.clone(),
                    status: u.status,
                };
                match ohne_eintrag
                    .iter_mut()
                    .find(|o| o.dokument_id == u.dokument_id)
                {
                    Some(o) => o.behoerde.push(stand),
                    None => ohne_eintrag.push(PlanOhneEintrag {
                        dokument_id: u.dokument_id,
                        dateiname: u.name.clone(),
                        behoerde: vec![stand],
                    }),
                }
            }
        }
        ohne_eintrag.sort_by(|a, b| a.dateiname.cmp(&b.dateiname));
        Ok(Planliste {
            ohne_eintrag,
            schema: self.plannummernschema().await?,
            kennzeichen: self.projektkennzeichen(&projekt).await?,
            zeilen,
        })
    }
}

/// Dokumentbezug des aktuellen Index (für Plankopf und Ausgabe).
pub fn indexbezug(e: &Planeintrag) -> Option<Dokumentbezug> {
    e.revisionen.last().map(|r| Dokumentbezug {
        dokument_id: e.dokument_id,
        version: r.version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn eintrag(nr: u32) -> Planeintrag {
        Planeintrag {
            dokument_id: DocumentId(Uuid::from_u128(1)),
            planart: Planart::Grundriss,
            geschoss: "EG".into(),
            nummer: nr,
            titel: "Grundriss Erdgeschoss".into(),
            massstab: Some(100),
            plannummer: String::new(),
            revisionen: vec![],
            verteiler: vec![],
        }
    }

    #[test]
    fn plannummer_nach_muster() {
        let s = Plannummernschema::default();
        assert_eq!(
            s.plannummer("2026-014", Planart::Grundriss, "EG", 3),
            "2026-014-GR-EG-03"
        );
        assert_eq!(
            s.plannummer("2026-014", Planart::Lageplan, "", 1),
            "2026-014-LP-01"
        );
        let s = Plannummernschema {
            muster: "{planart}{nr:3} / {geschoss}".into(),
        };
        assert_eq!(s.plannummer("x", Planart::Schnitt, "", 7), "SN007");
        assert!(s.pruefen().is_ok());
        assert!(
            Plannummernschema {
                muster: "{projekt}-{planart}".into()
            }
            .pruefen()
            .is_err()
        );
        assert!(
            Plannummernschema {
                muster: "{nr}-{datum}".into()
            }
            .pruefen()
            .is_err()
        );
        assert!(
            Plannummernschema {
                muster: "{nr".into()
            }
            .pruefen()
            .is_err()
        );
    }

    #[test]
    fn eintrag_setzt_plannummer_und_prueft_dubletten() {
        let s = Plannummernschema::default();
        let id = Uuid::from_u128(1);
        let e = eintrag_pruefen(eintrag(1), id, &s, "P1", &[], &[1]).unwrap();
        assert_eq!(e.plannummer, "P1-GR-EG-01");
        let mut anderer = e.clone();
        anderer.dokument_id = DocumentId(Uuid::from_u128(2));
        let err = eintrag_pruefen(eintrag(1), id, &s, "P1", &[anderer], &[1]).unwrap_err();
        assert!(err.to_string().contains("schon an"));
        assert!(eintrag_pruefen(eintrag(1), Uuid::from_u128(9), &s, "P1", &[], &[1]).is_err());
    }

    #[test]
    fn revisionen_aufsteigend_eindeutig_und_mit_version() {
        let s = Plannummernschema::default();
        let id = Uuid::from_u128(1);
        let rev = |i: &str, d: &str, v: i64| Planrevision {
            index: i.into(),
            datum: tag(d),
            aenderung: "Fenster verschoben".into(),
            version: v,
        };
        let mut e = eintrag(1);
        e.revisionen = vec![rev("a", "2026-09-01", 1), rev("b", "2026-10-01", 2)];
        assert!(eintrag_pruefen(e.clone(), id, &s, "P", &[], &[1, 2]).is_ok());
        assert!(
            eintrag_pruefen(e.clone(), id, &s, "P", &[], &[1]).is_err(),
            "Version 2 fehlt"
        );
        e.revisionen[1].index = "A".into();
        assert!(
            eintrag_pruefen(e.clone(), id, &s, "P", &[], &[1, 2]).is_err(),
            "Index doppelt"
        );
        e.revisionen[1].index = "b".into();
        e.revisionen[1].datum = tag("2026-08-01");
        assert!(
            eintrag_pruefen(e.clone(), id, &s, "P", &[], &[1, 2]).is_err(),
            "absteigend"
        );
        e.revisionen[1].datum = tag("2026-10-01");
        e.verteiler.push(Planversand {
            id: Uuid::new_v4(),
            empfaenger: "Statikbüro".into(),
            datum: tag("2026-10-02"),
            index: "c".into(),
            weg: "E-Mail".into(),
            bemerkung: String::new(),
        });
        assert!(
            eintrag_pruefen(e.clone(), id, &s, "P", &[], &[1, 2]).is_err(),
            "Index c fehlt"
        );
        e.verteiler[0].index = "b".into();
        let e = eintrag_pruefen(e, id, &s, "P", &[], &[1, 2]).unwrap();
        let z = zeile(e, 3, "EG.pdf".into(), Some(3), vec![], vec![]);
        assert!(z.ohne_index, "Version 3 ohne Index");
        assert_eq!(z.index.unwrap().index, "b");
        assert_eq!(naechste_nummer(&[z.eintrag], Planart::Grundriss), 2);
    }
}
