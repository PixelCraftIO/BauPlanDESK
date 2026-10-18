//! Außerhalb der App erfolgte Einreichung, nachgetragen vor allem für Altakten.
//! Sie zählt wie ein versendeter Abgabestand als eingereicht, friert aber keinen
//! Datenstand ein und erzeugt keine Ausgabe-Artefakte. Gespeicherte Einträge sind
//! unveränderlich; ein Fehler wird durch einen berichtigenden Folgeeintrag behoben.
use super::*;

/// Weg, auf dem die Unterlagen außerhalb der App eingereicht wurden.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Einreichungsweg {
    Post,
    Portal,
    Persoenlich,
    Email,
    Fax,
    /// Erfordert eine Bemerkung.
    Sonstiges,
}
impl Einreichungsweg {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Post => "Post",
            Self::Portal => "Portal",
            Self::Persoenlich => "Persönlich",
            Self::Email => "E-Mail",
            Self::Fax => "Fax",
            Self::Sonstiges => "Sonstiger Weg",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExterneEinreichung {
    pub id: Uuid,
    /// Tag der Einreichung bei der Behörde (Ortsdatum); nie nach dem Erfassungstag.
    pub datum: NaiveDate,
    pub weg: Einreichungsweg,
    /// Einlieferungs-, Eingangs- oder Portalbeleg, falls vorhanden.
    pub beleg: Option<Dokumentbezug>,
    pub bemerkung: String,
    /// Dokumentversionen der Projektablage, die nachweislich eingereicht wurden.
    /// Leer, wenn die eingereichten Unterlagen nicht in der Ablage liegen.
    pub dokumente: Vec<Dokumentbezug>,
    /// Frühere externe Einreichung dieser Akte, die dieser Eintrag berichtigt.
    pub berichtigt: Option<Uuid>,
    /// Vom Core gesetzt.
    pub erfasst_am: DateTime<Utc>,
    /// Vom Core gesetzt.
    pub erfasst_von: Uuid,
}
/// Eingabe für [`CoreServices::verfahren_externe_einreichung`]; Erfassungszeit
/// und -person setzt der Core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct ExterneEinreichungswunsch {
    pub id: Uuid,
    pub datum: NaiveDate,
    pub weg: Einreichungsweg,
    pub beleg: Option<Dokumentbezug>,
    pub bemerkung: String,
    pub dokumente: Vec<Dokumentbezug>,
    pub berichtigt: Option<Uuid>,
}
impl ExterneEinreichungswunsch {
    fn entspricht(&self, e: &ExterneEinreichung) -> bool {
        self.id == e.id
            && self.datum == e.datum
            && self.weg == e.weg
            && self.beleg == e.beleg
            && self.bemerkung.trim() == e.bemerkung
            && self.dokumente == e.dokumente
            && self.berichtigt == e.berichtigt
    }
}
/// Maßgebende externe Einreichungen: nicht durch einen Folgeeintrag berichtigt
/// und nicht nach `heute` datiert.
pub fn gueltige_externe(
    a: &Verfahrensakte,
    heute: NaiveDate,
) -> impl Iterator<Item = &ExterneEinreichung> {
    a.externe_einreichungen.iter().filter(move |e| {
        e.datum <= heute
            && !a
                .externe_einreichungen
                .iter()
                .any(|n| n.berichtigt == Some(e.id))
    })
}
/// Ein Abgabestand gilt als tatsächlich eingereicht, wenn sein maßgebendes
/// Versandereignis „Versendet“ oder „Eingegangen“ lautet und nicht nach `heute`
/// liegt. Ein künftig datierter Versand zählt nicht.
pub fn abgabe_eingereicht(b: &abgabe::Abgabestand, heute: NaiveDate) -> bool {
    abgabe::letztes_versandereignis(b).is_some_and(|e| {
        e.datum <= heute
            && matches!(
                e.status,
                abgabe::Versandstatus::Versendet | abgabe::Versandstatus::Eingegangen
            )
    })
}
/// Liegt der Behörde eine tatsächliche Einreichung vor: ein versendeter
/// Abgabestand oder eine nachgetragene externe Einreichung?
pub fn eingereicht(a: &Verfahrensakte, heute: NaiveDate) -> bool {
    a.abgaben.iter().any(|b| abgabe_eingereicht(b, heute))
        || gueltige_externe(a, heute).next().is_some()
}
/// Dokumentversionen, die der Behörde nachweislich vorlagen: aus versendeten
/// Abgabeständen und aus maßgebenden externen Einreichungen.
pub fn eingereichte_dokumente(a: &Verfahrensakte, heute: NaiveDate) -> Vec<&Dokumentbezug> {
    a.abgaben
        .iter()
        .filter(|b| abgabe_eingereicht(b, heute))
        .flat_map(|b| b.dokumente.iter().map(|d| &d.dokument))
        .chain(gueltige_externe(a, heute).flat_map(|e| &e.dokumente))
        .collect()
}
/// Prüfregeln der externen Einreichungen einer Akte; Dokumentbezüge sammelt
/// der Aufrufer zur Projektprüfung ein.
pub(super) fn pruefen(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    kontext: AktenCommit,
    heute: NaiveDate,
    dokumente: &mut Vec<Dokumentbezug>,
) -> CoreResult<()> {
    if a.externe_einreichungen.len() > 100 {
        return Err(fehler("Höchstens 100 externe Einreichungen je Akte"));
    }
    let bisher = alt
        .map(|a| a.externe_einreichungen.as_slice())
        .unwrap_or(&[]);
    if a.externe_einreichungen.len() < bisher.len()
        || a.externe_einreichungen[..bisher.len()] != *bisher
    {
        return Err(fehler(
            "Externe Einreichungen bleiben unverändert. Berichtigenden Folgeeintrag erfassen.",
        ));
    }
    for (i, e) in a.externe_einreichungen.iter().enumerate() {
        dokumente.extend(e.beleg.iter().cloned());
        dokumente.extend(e.dokumente.iter().cloned());
        if i < bisher.len() {
            continue;
        }
        if kontext != AktenCommit::Abgabe {
            return Err(fehler(
                "Externe Einreichung über „Externe Einreichung nachtragen“ erfassen",
            ));
        }
        if e.datum > heute {
            return Err(fehler(
                "Das Einreichungsdatum darf nicht in der Zukunft liegen",
            ));
        }
        if e.bemerkung.len() > 4000 {
            return Err(fehler("Bemerkung höchstens 4000 Zeichen"));
        }
        if e.weg == Einreichungsweg::Sonstiges && e.bemerkung.trim().is_empty() {
            return Err(fehler("Sonstiger Einreichungsweg benötigt eine Bemerkung"));
        }
        if e.dokumente.len() > 200 {
            return Err(fehler("Höchstens 200 eingereichte Unterlagen je Eintrag"));
        }
        let mut seen = std::collections::HashSet::new();
        if e.dokumente.iter().any(|d| !seen.insert(d.dokument_id)) {
            return Err(fehler("Je Dokument eine eingereichte Version wählen"));
        }
        if let Some(id) = e.berichtigt
            && (!a.externe_einreichungen[..i].iter().any(|v| v.id == id)
                || a.externe_einreichungen[..i]
                    .iter()
                    .any(|v| v.berichtigt == Some(id)))
        {
            return Err(fehler(
                "Berichtigung benötigt eine frühere, noch nicht berichtigte externe Einreichung",
            ));
        }
        if e.berichtigt.is_some() && e.bemerkung.trim().is_empty() {
            return Err(fehler(
                "Berichtigung benötigt eine Begründung als Bemerkung",
            ));
        }
    }
    Ok(())
}
impl CoreServices {
    /// Trägt eine außerhalb der App erfolgte Einreichung nach. Wiederholter
    /// Aufruf mit derselben Kennung und denselben Angaben ist wirkungslos.
    pub async fn verfahren_externe_einreichung(
        &self,
        p: ProjectId,
        vid: Uuid,
        basis: i64,
        w: ExterneEinreichungswunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        let (f, mut a) = self.verfahrensakte(p, vid).await?;
        if w.id.is_nil() {
            return Err(fehler("Externe Einreichung benötigt eine Kennung"));
        }
        if let Some(e) = a.externe_einreichungen.iter().find(|e| e.id == w.id) {
            if w.entspricht(e) {
                return Ok(f);
            }
            return Err(fehler(
                "Kennung bereits mit einer anderen externen Einreichung belegt",
            ));
        }
        if f.revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: f.revision,
            });
        }
        a.externe_einreichungen.push(ExterneEinreichung {
            id: w.id,
            datum: w.datum,
            weg: w.weg,
            beleg: w.beleg,
            bemerkung: w.bemerkung.trim().into(),
            dokumente: w.dokumente,
            berichtigt: w.berichtigt,
            erfasst_am: Utc::now(),
            erfasst_von: wer.id.0,
        });
        self.verfahren_commit(p, vid, basis, a, wer, true).await
    }
}
