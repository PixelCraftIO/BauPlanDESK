//! Kataster-/Grundbuchfakten und davon getrennte Ansätze der Baugrundstücksermittlung.
//! `BaugrundstuecksPlan` bleibt der vollständige Eingabe-/Archivvertrag für Verbraucher.
use crate::kunden::KundeId;
use crate::{
    CoreError, CoreResult, baugrundstuecke::*, domain::Planungsrecht,
    plattform::objekte::Dokumentbezug, storage::ContentHash,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Grundstuecksinformationen {
    pub flurstuecke: Vec<FlurstuecksInfo>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FlurstuecksInfo {
    pub id: String,
    /// Name der Gemarkung.
    pub gemarkung: String,
    /// Gemarkungsnummer laut Liegenschaftskataster (höchstens vier Ziffern, ohne
    /// Landesschlüssel). XBau übermittelt sie statt des Namens.
    // Nicht geschrieben, solange leer, damit Bestandsdaten unverändert bleiben.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(optional, as = "Option<String>")]
    pub gemarkungsnummer: String,
    pub flur: String,
    /// Flurstücksnummer „Zähler/Nenner“ (Zähler bis fünf, Nenner bis vier Ziffern),
    /// siehe [`Self::zaehler_nenner`]. Bestandsangaben in anderer Form bleiben
    /// erhalten, neue und geänderte Nummern prüft [`kennzeichen_pruefen`].
    pub nummer: String,
    pub flaeche_m2: f64,
    #[serde(default)]
    pub grundbuchbezirk: String,
    #[serde(default)]
    pub grundbuchblatt: String,
    #[serde(default)]
    pub laufende_nummer: Option<u32>,
    #[serde(default)]
    pub grundbuchauszug: Option<Dokumentbezug>,
    /// Frühere Freitextangabe. Sie bleibt lesbar, kann aber nur noch entfernt
    /// werden; Eigentümer werden über [`Self::eigentuemer_kunden`] zugeordnet.
    #[serde(default)]
    pub eigentuemer: String,
    /// Eigentümer aus der Kundendatenbank. Bei Vorhabenflurstücken Bauherr oder
    /// Projektbeteiligter mit Verfügungsrecht, siehe [`eigentuemer_pruefen`].
    // Nicht geschrieben, solange leer, damit Bestandsdaten unverändert bleiben.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub eigentuemer_kunden: Vec<KundeId>,
    #[serde(default)]
    pub bemerkung: String,
    /// Bezug zum Vorhaben; fehlt bei Vorhabenflurstücken.
    // Nicht geschrieben, damit Bestandsdaten ihren Quellen-Fingerprint behalten.
    #[serde(default, skip_serializing_if = "FlurstuecksRolle::ist_vorhaben")]
    #[ts(as = "Option<FlurstuecksRolle>", optional)]
    pub rolle: FlurstuecksRolle,
    /// Im Aufmaß gezeichnete Flurstücksgrenze (Flurkarte oder Lageplan):
    /// Bindung an eine Markierung mit Bedeutung „Flurstück“, die diesem
    /// Flurstück zugeordnet ist (Ziel [`Self::planziel`]). Die Katasterfläche
    /// [`Self::flaeche_m2`] bleibt der Rechenwert, solange nicht
    /// [`Self::flaeche_aus_plan`] gesetzt ist; die Abweichung bleibt sichtbar.
    // Nicht geschrieben, solange leer, damit Bestandsdaten unverändert bleiben.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub planbindung: Option<crate::planaufmass::Planbindung>,
    /// Gemessene Fläche der gebundenen Markierung in m² (auf 0,01 m² gerundet).
    /// Setzt ausschließlich der Core bei der Übernahme (`planaufmass_flurstuecke`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub planflaeche_m2: Option<f64>,
    /// Die gezeichnete Fläche gilt als Flurstücksfläche, weil (noch) kein
    /// amtlicher Wert vorliegt; `flaeche_m2` wird dann aus `planflaeche_m2`
    /// übernommen und ist nicht von Hand änderbar.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(optional, as = "Option<bool>")]
    pub flaeche_aus_plan: bool,
}
/// Anlass der Baulastklärung für ein Flurstück mit Rolle „Sonstiges“.
pub const BAULAST_GRUND: &str = "Flurstück außerhalb des Vorhabengrundstücks (Zuwegung, Baulastfläche o. Ä.). Klären, ob eine Baulast nötig ist; die Baulasterklärung des Eigentümers ist dann Bauvorlage.";
/// Bezug eines erfassten Flurstücks zum Vorhaben. Nur Vorhabenflurstücke bilden
/// Baugrundstücke und stehen im Bauantrag; die übrigen bleiben als Kataster-
/// und Grundbuchfakten erfasst, etwa für Abstandsflächen oder Nachbarn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum FlurstuecksRolle {
    #[default]
    Vorhaben,
    /// Angrenzendes Flurstück, etwa für die Nachbarbeteiligung.
    Nachbar,
    /// Weiteres Flurstück, etwa Zuwegung oder Baulastfläche.
    Sonstiges,
}
impl FlurstuecksInfo {
    /// Zähler und Nenner der Flurstücksnummer; der Nenner ist leer, wenn die
    /// Nummer keinen Schrägstrich enthält.
    pub fn zaehler_nenner(&self) -> (&str, &str) {
        let n = self.nummer.trim();
        let (z, nenner) = n.split_once('/').unwrap_or((n, ""));
        (z.trim(), nenner.trim())
    }
    /// Die Nummer hat die Form „Zähler“ oder „Zähler/Nenner“ aus Ziffern.
    pub fn nummer_gueltig(&self) -> bool {
        let ziffern = |s: &str, max: usize| {
            !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_digit())
        };
        let n = self.nummer.trim();
        match n.split_once('/') {
            Some((z, nenner)) => ziffern(z, 5) && ziffern(nenner, 4),
            None => ziffern(n, 5),
        }
    }
    /// Bezeichnung für Hinweise, etwa „Schwabing 123/4“.
    pub fn bezeichnung(&self) -> String {
        format!("{} {}", self.gemarkung.trim(), self.nummer.trim())
    }
    /// Zielpfad der Flächenübernahme aus dem Aufmaß (`Tabelle.Zeilen-ID.Feld`).
    pub fn planziel(&self) -> String {
        format!("flurstuecke.{}.flaeche_m2", self.id)
    }
    /// Gezeichnete minus Katasterfläche in m², solange beide Werte getrennt
    /// vorliegen; `None` ohne Bindung oder wenn die Zeichnung die Fläche liefert.
    pub fn flaechenabweichung_m2(&self) -> Option<f64> {
        if self.flaeche_aus_plan {
            return None;
        }
        self.planflaeche_m2.map(|p| p - self.flaeche_m2)
    }
}
/// Einstufung der Abweichung zwischen gezeichneter und Katasterfläche
/// (Schwellen in `regelwerke/aufmass-toleranzen.json`): bis 1 m² oder 1 %
/// unauffällig (Mess- und Rundungsgenauigkeit), bis 5 % Warnung, darüber
/// Fehler, der die Einreichung aufhält, bis Kataster oder Zeichnung geklärt sind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Flaechenabweichung {
    Unauffaellig,
    Warnung,
    Fehler,
}
impl FlurstuecksInfo {
    pub fn flaechenabweichung(&self) -> Option<Flaechenabweichung> {
        let d = self.flaechenabweichung_m2()?.abs();
        let t = &crate::planaufmass::toleranzen().flurstueck;
        let basis = self.flaeche_m2.abs();
        Some(
            if d <= t.unauffaellig_m2.max(basis * t.unauffaellig_prozent / 100.) {
                Flaechenabweichung::Unauffaellig
            } else if d <= basis * t.fehler_prozent / 100. {
                Flaechenabweichung::Warnung
            } else {
                Flaechenabweichung::Fehler
            },
        )
    }
    /// Text zu einer auffälligen Abweichung (Warnung oder Fehler).
    pub fn flaechenabweichung_hinweis(&self) -> Option<(Flaechenabweichung, String)> {
        let stufe = self.flaechenabweichung()?;
        if stufe == Flaechenabweichung::Unauffaellig {
            return None;
        }
        let plan = self.planflaeche_m2?;
        let d = plan - self.flaeche_m2;
        let zahl = crate::formular::vorlage::zahl;
        Some((
            stufe,
            format!(
                "Flurstück {}: gezeichnete Fläche {} m² weicht um {} m² ({} %) von der Katasterfläche {} m² ab. Kataster oder Zeichnung prüfen; gerechnet wird mit der Katasterfläche.",
                self.bezeichnung(),
                zahl(plan, 2),
                zahl(d, 2),
                zahl(d / self.flaeche_m2 * 100., 1),
                zahl(self.flaeche_m2, 2)
            ),
        ))
    }
}
impl Grundstuecksinformationen {
    /// Auffällige Flächenabweichungen der Vorhabenflurstücke.
    pub fn flaechenabweichungen(&self) -> Vec<(Flaechenabweichung, String)> {
        self.flurstuecke
            .iter()
            .filter(|f| f.rolle.ist_vorhaben())
            .filter_map(|f| f.flaechenabweichung_hinweis())
            .collect()
    }
}
impl FlurstuecksRolle {
    pub fn ist_vorhaben(&self) -> bool {
        *self == Self::Vorhaben
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct FlurstuecksAnsatz {
    pub flurstueck_id: String,
    #[ts(type = "Array<FlaechenAbzug>")]
    pub abzuege: Vec<FlaechenAbzug>,
    #[ts(type = "{ kennung: string; grundlage: string } | null")]
    pub abweichende_zuordnung: Option<AbweichendeZuordnung>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct BaugrundstuecksErmittlung {
    pub ansaetze: Vec<FlurstuecksAnsatz>,
    #[ts(type = "Array<BaugrundstueckBerechnung>")]
    pub berechnungen: Vec<BaugrundstueckBerechnung>,
    /// Bei Speicherung durch den Core gesetzt; keine zweite Kopie der Katasterflächen.
    #[ts(type = "string | null")]
    pub quellen_hash: Option<ContentHash>,
}
impl Grundstuecksinformationen {
    pub fn validate(&self) -> CoreResult<()> {
        if self.flurstuecke.len() > 1000 {
            return Err(CoreError::InvalidInput(
                "Höchstens 1000 Flurstücke erfassen".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        let mut kataster = std::collections::HashSet::new();
        let mut vorhaben_buchgrundstuecke = std::collections::HashSet::new();
        for (i, f) in self.flurstuecke.iter().enumerate() {
            // Die Meldung nennt Flurstück und Feld, damit die Eingabe auffindbar ist.
            let name = match f.nummer.trim() {
                "" => format!("{}. Flurstück", i + 1),
                n => format!("Flurstück {n}"),
            };
            let fehler = |t: &str| Err(CoreError::InvalidInput(format!("{name}: {t}")));
            if f.id.trim().is_empty() || f.id.len() > 500 {
                return fehler("interne Kennung fehlt");
            }
            if f.gemarkung.trim().is_empty() || f.gemarkung.len() > 500 {
                return fehler("Gemarkung angeben (höchstens 500 Zeichen)");
            }
            if f.nummer.trim().is_empty() || f.nummer.len() > 500 {
                return fehler("Flurstücksnummer angeben (höchstens 500 Zeichen)");
            }
            if [
                &f.flur,
                &f.grundbuchbezirk,
                &f.grundbuchblatt,
                &f.eigentuemer,
            ]
            .iter()
            .any(|s| s.len() > 500)
                || f.bemerkung.len() > 4000
            {
                return fehler("Angaben zu lang (je Feld höchstens 500 Zeichen, Bemerkung 4000)");
            }
            if f.gemarkungsnummer.len() > 4
                || !f.gemarkungsnummer.bytes().all(|b| b.is_ascii_digit())
            {
                return fehler(
                    "Gemarkungsnummer besteht aus höchstens vier Ziffern, ohne Landesschlüssel (z. B. 1234 statt 051234)",
                );
            }
            if f.eigentuemer_kunden.len() > 20
                || (1..f.eigentuemer_kunden.len())
                    .any(|i| f.eigentuemer_kunden[..i].contains(&f.eigentuemer_kunden[i]))
            {
                return fehler("Eigentümer höchstens 20 und jeweils nur einmal zuordnen");
            }
            if f.laufende_nummer == Some(0) {
                return fehler(
                    "laufende Nummer im Bestandsverzeichnis muss positiv sein oder leer bleiben",
                );
            }
            match (&f.planbindung, f.planflaeche_m2) {
                (Some(b), Some(p)) => {
                    if b.ziel != f.planziel()
                        || b.groesse != crate::planaufmass::Planmessgroesse::Flaeche
                    {
                        return fehler("Aufmaßverknüpfung gehört zu einem anderen Ziel");
                    }
                    if !p.is_finite() || p <= 0. {
                        return fehler("gezeichnete Fläche muss größer als 0 sein");
                    }
                    if f.flaeche_aus_plan && (f.flaeche_m2 - p).abs() > 1e-6 {
                        return fehler(
                            "Flurstücksfläche weicht von der gezeichneten Fläche ab; Übernahme aus dem Aufmaß erneut ausführen",
                        );
                    }
                }
                (None, None) => {
                    if f.flaeche_aus_plan {
                        return fehler(
                            "Fläche aus dem Aufmaß benötigt eine verknüpfte Flurstücksmarkierung",
                        );
                    }
                }
                _ => {
                    return fehler(
                        "Aufmaßverknüpfung und gezeichnete Fläche gehören zusammen; Übernahme aus dem Aufmaß erneut ausführen",
                    );
                }
            }
            if !f.flaeche_m2.is_finite() || f.flaeche_m2 <= 0. {
                return fehler(if f.planbindung.is_some() {
                    "Fläche laut Kataster in m² angeben oder die gezeichnete Fläche als Flurstücksfläche verwenden"
                } else {
                    "Fläche laut Kataster in m² angeben (größer als 0)"
                });
            }
            if f.rolle.ist_vorhaben()
                && !f.grundbuchbezirk.trim().is_empty()
                && !f.grundbuchblatt.trim().is_empty()
                && let Some(n) = f.laufende_nummer
            {
                vorhaben_buchgrundstuecke.insert((
                    f.grundbuchbezirk.trim().to_lowercase(),
                    f.grundbuchblatt.trim().to_lowercase(),
                    n,
                ));
            }
            if !ids.insert(&f.id)
                || !kataster.insert((
                    f.gemarkung.trim().to_lowercase(),
                    f.flur.trim().to_lowercase(),
                    f.nummer.trim().to_lowercase(),
                ))
            {
                return fehler("wurde mehrfach erfasst (gleiche Gemarkung, Flur und Nummer)");
            }
        }
        if !self.flurstuecke.is_empty() && self.vorhaben().next().is_none() {
            return Err(CoreError::InvalidInput(
                "Mindestens ein Flurstück muss zum Vorhabengrundstück gehören".into(),
            ));
        }
        // Das Baugrundstück ist das Buchgrundstück; es gehört nur ganz zum Vorhaben.
        for f in self.flurstuecke.iter().filter(|f| !f.rolle.ist_vorhaben()) {
            if let Some(n) = f.laufende_nummer
                && vorhaben_buchgrundstuecke.contains(&(
                    f.grundbuchbezirk.trim().to_lowercase(),
                    f.grundbuchblatt.trim().to_lowercase(),
                    n,
                ))
            {
                return Err(CoreError::InvalidInput(format!(
                    "Flurstück {} liegt im selben Buchgrundstück wie ein Vorhabenflurstück und gehört deshalb zum Vorhaben",
                    f.nummer.trim()
                )));
            }
        }
        Ok(())
    }
    /// Flurstücke, die das Vorhabengrundstück bilden.
    pub fn vorhaben(&self) -> impl Iterator<Item = &FlurstuecksInfo> {
        self.flurstuecke.iter().filter(|f| f.rolle.ist_vorhaben())
    }
    /// Summe der Flurstücksflächen des Vorhabens (Eigentumsfläche, ohne Abzüge);
    /// `None`, solange keine Flurstücke erfasst sind.
    pub fn vorhabenflaeche_m2(&self) -> Option<f64> {
        let mut flaechen = self.vorhaben().map(|f| f.flaeche_m2).peekable();
        flaechen.peek()?;
        Some(flaechen.sum())
    }
    /// Quellenstand der Baugrundstücksermittlung: nur Angaben, die Bildung und
    /// Fläche der Baugrundstücke bestimmen (Vorhabenflurstücke mit Kataster-,
    /// Grundbuch- und Flächenangaben). Nachbar- und Baulastflurstücke,
    /// Bemerkungen, Grundbuchauszüge und Eigentümer lassen die Ermittlung aktuell.
    pub fn fingerprint(&self) -> CoreResult<ContentHash> {
        let quelle: Vec<_> = self
            .vorhaben()
            .map(|f| {
                (
                    "v2",
                    &f.id,
                    &f.gemarkung,
                    &f.flur,
                    &f.nummer,
                    f.flaeche_m2,
                    &f.grundbuchbezirk,
                    &f.grundbuchblatt,
                    f.laufende_nummer,
                )
            })
            .collect();
        Ok(ContentHash::of(
            &serde_json::to_vec(&quelle).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        ))
    }
    /// Quellenstand bis Oktober 2026 über alle Flurstücksangaben. Bereits
    /// gespeicherte Ermittlungen mit diesem Stand bleiben aktuell, solange sich
    /// die Angaben nicht ändern; die nächste Speicherung schreibt [`Self::fingerprint`].
    pub fn fingerprint_v1(&self) -> CoreResult<ContentHash> {
        let mut quelle = self.clone();
        for f in &mut quelle.flurstuecke {
            f.eigentuemer_kunden.clear();
            f.gemarkungsnummer.clear();
        }
        Ok(ContentHash::of(
            &serde_json::to_vec(&quelle).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        ))
    }
    /// Flurstücke außerhalb des Vorhabengrundstücks, die das Vorhaben nutzt,
    /// etwa Zuwegung oder Baulastfläche (Rolle „Sonstiges“).
    pub fn baulastflurstuecke(&self) -> impl Iterator<Item = &FlurstuecksInfo> {
        self.flurstuecke
            .iter()
            .filter(|f| f.rolle == FlurstuecksRolle::Sonstiges)
    }
    /// Hinweise für den Bauantrag zu [`Self::baulastflurstuecke`].
    pub fn baulasthinweise(&self) -> Vec<String> {
        self.baulastflurstuecke()
            .map(|f| format!("Flurstück {}: {BAULAST_GRUND}", f.bezeichnung()))
            .collect()
    }
    /// Eigentümer der Vorhabenflurstücke.
    pub fn vorhabeneigentuemer(&self) -> impl Iterator<Item = (&FlurstuecksInfo, KundeId)> {
        self.vorhaben()
            .flat_map(|f| f.eigentuemer_kunden.iter().map(move |k| (f, *k)))
    }
}
/// Lesender Adapter für Bestandsprojekte; die erste getrennte Speicherung übernimmt beide Teile.
pub fn informationen(p: &Planungsrecht) -> Grundstuecksinformationen {
    p.grundstuecksinformationen
        .clone()
        .unwrap_or_else(|| Grundstuecksinformationen {
            flurstuecke: p
                .baugrundstuecke
                .as_ref()
                .map(|b| {
                    b.flurstuecke
                        .iter()
                        .map(|f| FlurstuecksInfo {
                            id: f.id.clone(),
                            gemarkung: f.gemarkung.clone(),
                            gemarkungsnummer: String::new(),
                            flur: f.flur.clone(),
                            nummer: f.nummer.clone(),
                            flaeche_m2: f.flaeche_m2,
                            grundbuchbezirk: f.grundbuchbezirk.clone(),
                            grundbuchblatt: f.grundbuchblatt.clone(),
                            laufende_nummer: Some(f.laufende_nummer).filter(|n| *n > 0),
                            grundbuchauszug: None,
                            eigentuemer: String::new(),
                            eigentuemer_kunden: vec![],
                            bemerkung: String::new(),
                            rolle: FlurstuecksRolle::Vorhaben,
                            planbindung: None,
                            planflaeche_m2: None,
                            flaeche_aus_plan: false,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
}
/// Mit erfassten Flurstücken ist die Projektfläche die Summe der Vorhabenflurstücke;
/// ohne Flurstücke bleibt die Angabe des Bestandsprojekts stehen.
pub fn projektflaeche_ableiten(p: &mut crate::domain::Project) {
    if let Some(f) = informationen(&p.planungsrecht).vorhabenflaeche_m2() {
        p.grundstuecksflaeche_m2 = Some(f);
    }
}
pub fn ermittlung(p: &Planungsrecht) -> CoreResult<Option<BaugrundstuecksErmittlung>> {
    if let Some(e) = &p.baugrundstuecksermittlung {
        return Ok(Some(e.clone()));
    }
    p.baugrundstuecke
        .as_ref()
        .map(|b| {
            Ok(BaugrundstuecksErmittlung {
                ansaetze: b
                    .flurstuecke
                    .iter()
                    .map(|f| FlurstuecksAnsatz {
                        flurstueck_id: f.id.clone(),
                        abzuege: f.abzuege.clone(),
                        abweichende_zuordnung: f.abweichende_zuordnung.clone(),
                    })
                    .collect(),
                berechnungen: b.berechnungen.clone(),
                quellen_hash: Some(
                    informationen(&Planungsrecht {
                        baugrundstuecke: p.baugrundstuecke.clone(),
                        ..Default::default()
                    })
                    .fingerprint()?,
                ),
            })
        })
        .transpose()
}
pub fn normalisieren(p: &mut Planungsrecht) -> CoreResult<()> {
    if p.baugrundstuecke.is_some() {
        let infos = informationen(p);
        let e = ermittlung(p)?;
        p.grundstuecksinformationen = Some(infos);
        p.baugrundstuecksermittlung = e;
        p.baugrundstuecke = None;
    }
    Ok(())
}
pub fn plan(p: &Planungsrecht) -> CoreResult<Option<BaugrundstuecksPlan>> {
    if p.grundstuecksinformationen.is_none() && p.baugrundstuecksermittlung.is_none() {
        return Ok(p.baugrundstuecke.clone());
    }
    let infos = informationen(p);
    infos.validate()?;
    let e = ermittlung(p)?.unwrap_or_default();
    Ok(Some(BaugrundstuecksPlan {
        flurstuecke: infos
            .vorhaben()
            .map(|f| {
                let a = e.ansaetze.iter().find(|a| a.flurstueck_id == f.id);
                Flurstueck {
                    id: f.id.clone(),
                    gemarkung: f.gemarkung.clone(),
                    flur: f.flur.clone(),
                    nummer: f.nummer.clone(),
                    grundbuchbezirk: f.grundbuchbezirk.clone(),
                    grundbuchblatt: f.grundbuchblatt.clone(),
                    laufende_nummer: f.laufende_nummer.unwrap_or(0),
                    flaeche_m2: f.flaeche_m2,
                    abzuege: a.map(|a| a.abzuege.clone()).unwrap_or_default(),
                    abweichende_zuordnung: a.and_then(|a| a.abweichende_zuordnung.clone()),
                }
            })
            .collect(),
        berechnungen: e.berechnungen,
    }))
}
pub fn quellen_aktuell(p: &Planungsrecht) -> CoreResult<bool> {
    if p.grundstuecksinformationen.is_none() {
        return Ok(true);
    }
    let Some(gespeichert) = ermittlung(p)?.and_then(|e| e.quellen_hash) else {
        return Ok(false);
    };
    let infos = informationen(p);
    Ok(gespeichert == infos.fingerprint()? || gespeichert == infos.fingerprint_v1()?)
}
/// Neue Berechnungseingaben dürfen keine eigenen Katasterfakten unterschieben.
pub fn pruefe_quellen(p: &Planungsrecht, b: &BaugrundstuecksPlan) -> CoreResult<()> {
    if p.grundstuecksinformationen.is_none() {
        return Ok(());
    }
    let mut erwartung = plan(p)?.unwrap_or_default().flurstuecke;
    let mut eingabe = b.flurstuecke.clone();
    for f in erwartung.iter_mut().chain(eingabe.iter_mut()) {
        f.abzuege.clear();
        f.abweichende_zuordnung = None;
    }
    erwartung.sort_by(|a, b| a.id.cmp(&b.id));
    eingabe.sort_by(|a, b| a.id.cmp(&b.id));
    if erwartung != eingabe {
        return Err(CoreError::InvalidInput("Flurstücksdaten wurden geändert. Grundstücksinformationen zuerst speichern und die Baugrundstücksberechnung neu laden.".into()));
    }
    Ok(())
}
pub fn validiere_ansaetze(p: &Planungsrecht, e: &BaugrundstuecksErmittlung) -> CoreResult<()> {
    let infos = informationen(p);
    let mut ids = std::collections::HashSet::new();
    if e.ansaetze.len() > 1000
        || e.ansaetze.iter().any(|a| {
            !ids.insert(&a.flurstueck_id)
                || !infos.flurstuecke.iter().any(|f| f.id == a.flurstueck_id)
        })
    {
        return Err(CoreError::InvalidInput(
            "Berechnungsansätze müssen eindeutig auf vorhandene Flurstücke verweisen".into(),
        ));
    }
    Ok(())
}

/// Nachweise verbrauchen die bestätigten Grundlagen des Grundstücksmoduls.
/// Eigene GRZ-/GFZ-Eingaben bleiben zulässig; geänderte Zuordnungen/Abzüge/Zuschläge
/// müssen dagegen zuerst im Liefermodul gespeichert werden.
pub fn pruefe_bestaetigte_ermittlung(p: &Planungsrecht, b: &BaugrundstuecksPlan) -> CoreResult<()> {
    pruefe_quellen(p, b)?;
    if p.grundstuecksinformationen.is_none() {
        return Ok(());
    }
    let fehler = || {
        CoreError::InvalidInput("Baugrundstücksermittlung zuerst prüfen und speichern. Zuordnungen, Abzüge und Gemeinschaftsanteile müssen dem bestätigten Stand entsprechen.".into())
    };
    if !quellen_aktuell(p)? {
        return Err(fehler());
    }
    let gespeichert = plan(p)?.ok_or_else(fehler)?;
    for f in &b.flurstuecke {
        let alt = gespeichert
            .flurstuecke
            .iter()
            .find(|a| a.id == f.id)
            .ok_or_else(fehler)?;
        if f.abzuege != alt.abzuege || f.abweichende_zuordnung != alt.abweichende_zuordnung {
            return Err(fehler());
        }
    }
    let anteile = |b: &BaugrundstueckBerechnung| {
        b.eingabe
            .aufstellung
            .as_ref()
            .and_then(|a| a.grundstueck.as_ref())
            .map(|g| g.gemeinschaftsanteile.clone())
            .unwrap_or_default()
    };
    for berechnung in &b.berechnungen {
        let alt = gespeichert
            .berechnungen
            .iter()
            .find(|a| a.schluessel == berechnung.schluessel)
            .ok_or_else(fehler)?;
        if anteile(berechnung) != anteile(alt) {
            return Err(fehler());
        }
    }
    Ok(())
}

/// Eigentümer am Flurstück sind Kunden. Vorhabenflurstücke gehören dem Bauherrn
/// oder einem Projektbeteiligten mit Verfügungsrecht, damit die Zustimmungspflicht
/// aus den Projektbeteiligten folgt. Die frühere Freitextangabe kann nur entfernt
/// oder unverändert behalten werden.
pub fn eigentuemer_pruefen(
    neu: &Grundstuecksinformationen,
    alt: Option<&Grundstuecksinformationen>,
    verfuegungsberechtigt: &std::collections::HashSet<KundeId>,
) -> CoreResult<()> {
    for f in &neu.flurstuecke {
        if !f.eigentuemer.trim().is_empty()
            && alt
                .and_then(|a| a.flurstuecke.iter().find(|v| v.id == f.id))
                .is_none_or(|v| v.eigentuemer != f.eigentuemer)
        {
            return Err(CoreError::InvalidInput(format!(
                "Flurstück {}: Eigentümer als Kunden zuordnen. Die bisherige Freitextangabe kann nur entfernt werden.",
                f.nummer.trim()
            )));
        }
    }
    if let Some((f, _)) = neu
        .vorhabeneigentuemer()
        .find(|(_, k)| !verfuegungsberechtigt.contains(k))
    {
        return Err(CoreError::InvalidInput(format!(
            "Flurstück {}: Eigentümer eines Vorhabenflurstücks zuerst als Bauherr oder als Grundstückseigentümer bzw. Erbbauberechtigten unter den Projektbeteiligten erfassen.",
            f.nummer.trim()
        )));
    }
    Ok(())
}

/// Neue und geänderte Flurstücksnummern haben die Form „Zähler/Nenner“, damit
/// XBau und Kataster sie eindeutig trennen. Bestandsangaben bleiben unverändert
/// speicherbar.
pub fn kennzeichen_pruefen(
    neu: &Grundstuecksinformationen,
    alt: Option<&Grundstuecksinformationen>,
) -> CoreResult<()> {
    for f in &neu.flurstuecke {
        if !f.nummer_gueltig()
            && alt
                .and_then(|a| a.flurstuecke.iter().find(|v| v.id == f.id))
                .is_none_or(|v| v.nummer != f.nummer)
        {
            return Err(CoreError::InvalidInput(format!(
                "Flurstück {}: Zähler mit höchstens fünf und Nenner mit höchstens vier Ziffern angeben.",
                f.bezeichnung()
            )));
        }
    }
    Ok(())
}

/// Prüft Flurstücksnummern und Eigentümerzuordnungen gegen den bisherigen
/// Stand, die Kundendatenbank und die Projektbeteiligten.
pub(crate) async fn aenderungen_pruefen_im_projekt(
    core: &crate::CoreServices,
    projekt: crate::domain::ProjectId,
    neu: &Planungsrecht,
    alt: Option<&Planungsrecht>,
) -> CoreResult<()> {
    let neu = informationen(neu);
    let alt = alt.map(informationen);
    kennzeichen_pruefen(&neu, alt.as_ref())?;
    let mut geprueft = std::collections::HashSet::new();
    for k in neu.flurstuecke.iter().flat_map(|f| &f.eigentuemer_kunden) {
        if geprueft.insert(*k) {
            core.repos.kunden.kunde(*k).await?;
        }
    }
    let verfuegungsberechtigt = if neu.vorhabeneigentuemer().next().is_some() {
        core.verfuegungsberechtigte(projekt).await?
    } else {
        Default::default()
    };
    eigentuemer_pruefen(&neu, alt.as_ref(), &verfuegungsberechtigt)
}

pub(crate) async fn dokumente_pruefen(
    core: &crate::CoreServices,
    projekt: crate::domain::ProjectId,
    p: &Planungsrecht,
) -> CoreResult<()> {
    let mut geprueft = std::collections::HashSet::new();
    for f in informationen(p).flurstuecke {
        if let Some(d) = f.grundbuchauszug
            && geprueft.insert((d.dokument_id, d.version))
            && !core
                .repos
                .documents
                .history(d.dokument_id)
                .await?
                .iter()
                .any(|v| v.version == d.version && v.project_id == projekt)
        {
            return Err(CoreError::InvalidInput(
                "Grundbuchauszug gehört nicht zu diesem Projekt".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flaechenabweichung_wird_nach_toleranz_eingestuft() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let mut f = p.grundstuecksinformationen.unwrap().flurstuecke.remove(0);
        f.flaeche_m2 = 1000.;
        assert_eq!(f.flaechenabweichung(), None, "ohne Zeichnung");
        let stufe = |f: &mut FlurstuecksInfo, plan: f64| {
            f.planflaeche_m2 = Some(plan);
            f.flaechenabweichung()
        };
        assert_eq!(stufe(&mut f, 1010.), Some(Flaechenabweichung::Unauffaellig));
        assert_eq!(stufe(&mut f, 989.), Some(Flaechenabweichung::Warnung));
        assert_eq!(stufe(&mut f, 1050.), Some(Flaechenabweichung::Warnung));
        assert_eq!(stufe(&mut f, 1051.), Some(Flaechenabweichung::Fehler));
        // Kleine Flurstücke: mindestens 1 m² gilt als Messgenauigkeit.
        f.flaeche_m2 = 50.;
        assert_eq!(stufe(&mut f, 50.9), Some(Flaechenabweichung::Unauffaellig));
        f.flaeche_m2 = 1000.;
        stufe(&mut f, 1080.);
        let (s, text) = f.flaechenabweichung_hinweis().unwrap();
        assert_eq!(s, Flaechenabweichung::Fehler);
        assert!(
            text.contains("80,00 m²") && text.contains("(8,0 %)"),
            "{text}"
        );
        // „Fläche aus Plan“: kein Vergleich, die Zeichnung ist der Wert.
        f.flaeche_aus_plan = true;
        assert_eq!(f.flaechenabweichung(), None);
    }
    #[test]
    fn optionale_grundbuchangaben_fuehren_nicht_zu_heimlicher_gruppierung() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        for f in &mut infos.flurstuecke {
            f.grundbuchbezirk.clear();
            f.grundbuchblatt.clear();
            f.laufende_nummer = None;
        }
        infos.validate().unwrap();
        let gruppen = plan(&p).unwrap().unwrap().gruppen(true).unwrap();
        assert_eq!(gruppen.len(), 3);
        assert!(gruppen.iter().all(|g| g.flurstueck_ids.len() == 1));
        for a in &mut p.baugrundstuecksermittlung.as_mut().unwrap().ansaetze {
            a.abweichende_zuordnung = Some(AbweichendeZuordnung {
                kennung: a.flurstueck_id.clone(),
                grundlage: "Dokumentierte Einzelfallzuordnung".into(),
            });
        }
        assert_eq!(plan(&p).unwrap().unwrap().gruppen(true).unwrap().len(), 3);
    }
    #[test]
    fn eigentuemerzuordnung_laesst_ermittlung_aktuell() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let k = KundeId::new();
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        infos.flurstuecke[0].eigentuemer_kunden = vec![k];
        assert!(quellen_aktuell(&p).unwrap());
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        let berechtigt = std::collections::HashSet::from([k]);
        eigentuemer_pruefen(infos, None, &berechtigt).unwrap();
        assert!(eigentuemer_pruefen(infos, None, &Default::default()).is_err());
        infos.flurstuecke[0].eigentuemer_kunden.push(k);
        assert!(infos.validate().is_err(), "doppelt zugeordnet");
        infos.flurstuecke[0].eigentuemer_kunden.pop();
        // Bestehender Freitext bleibt erlaubt, neuer nicht.
        let alt = infos.clone();
        infos.flurstuecke[1].eigentuemer = "Huber".into();
        assert!(eigentuemer_pruefen(infos, Some(&alt), &berechtigt).is_err());
        let alt = infos.clone();
        eigentuemer_pruefen(infos, Some(&alt), &berechtigt).unwrap();
        infos.flurstuecke[1].eigentuemer.clear();
        eigentuemer_pruefen(infos, Some(&alt), &berechtigt).unwrap();
    }
    #[test]
    fn nur_flaechenrelevante_angaben_veralten_die_ermittlung() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        assert!(quellen_aktuell(&p).unwrap());
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        infos.flurstuecke[0].bemerkung = "Zufahrt über Nachbargrundstück".into();
        let mut nachbar = infos.flurstuecke[0].clone();
        nachbar.id = "nachbar".into();
        nachbar.nummer = "999/1".into();
        nachbar.rolle = FlurstuecksRolle::Nachbar;
        infos.flurstuecke.push(nachbar);
        assert!(
            quellen_aktuell(&p).unwrap(),
            "Bemerkung und Nachbar ohne Flächenbezug"
        );
        p.grundstuecksinformationen.as_mut().unwrap().flurstuecke[0].flaeche_m2 += 1.;
        assert!(!quellen_aktuell(&p).unwrap(), "Flächenänderung");
    }
    #[test]
    fn gespeicherter_alter_quellenstand_bleibt_gueltig() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let alt = p
            .grundstuecksinformationen
            .as_ref()
            .unwrap()
            .fingerprint_v1()
            .unwrap();
        p.baugrundstuecksermittlung.as_mut().unwrap().quellen_hash = Some(alt);
        assert!(quellen_aktuell(&p).unwrap());
        p.grundstuecksinformationen.as_mut().unwrap().flurstuecke[0].flaeche_m2 += 1.;
        assert!(!quellen_aktuell(&p).unwrap());
    }
    #[test]
    fn flurstuecksnummer_und_gemarkungsnummer() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        let f = &mut infos.flurstuecke[0];
        for (nummer, gueltig, teile) in [
            ("123", true, ("123", "")),
            (" 12345/6789 ", true, ("12345", "6789")),
            ("123456", false, ("123456", "")),
            ("12/34567", false, ("12", "34567")),
            ("12a", false, ("12a", "")),
            ("12/", false, ("12", "")),
        ] {
            f.nummer = nummer.into();
            assert_eq!(f.nummer_gueltig(), gueltig, "{nummer}");
            assert_eq!(f.zaehler_nenner(), teile, "{nummer}");
        }
        f.nummer = "12a".into();
        // Ein Bestandswert bleibt speicherbar, ein neuer nicht.
        let alt = infos.clone();
        kennzeichen_pruefen(infos, Some(&alt)).unwrap();
        assert!(kennzeichen_pruefen(infos, None).is_err());
        infos.flurstuecke[0].nummer = "12/3".into();
        kennzeichen_pruefen(infos, Some(&alt)).unwrap();
        // Die Gemarkungsnummer hat höchstens vier Ziffern und lässt die
        // Ermittlung aktuell.
        let vorher = infos.fingerprint().unwrap();
        infos.flurstuecke[0].gemarkungsnummer = "0915".into();
        infos.validate().unwrap();
        assert_eq!(infos.fingerprint().unwrap(), vorher);
        for falsch in ["09155", "09a"] {
            infos.flurstuecke[0].gemarkungsnummer = falsch.into();
            assert!(infos.validate().is_err(), "{falsch}");
        }
    }
    #[test]
    fn gleiche_nummer_in_unterschiedlichen_grundbuchblaettern_bleibt_getrennt() {
        let mut b = musterplan();
        b.flurstuecke[1].grundbuchblatt = "Anderes Blatt".into();
        assert_eq!(b.gruppen(true).unwrap().len(), 3);
    }
    #[test]
    fn rein_formatbedingte_trennung_erhaelt_plan_und_quellenstand() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        let vorher = plan(&p).unwrap();
        normalisieren(&mut p).unwrap();
        assert_eq!(plan(&p).unwrap(), vorher);
        assert!(quellen_aktuell(&p).unwrap());
        normalisieren(&mut p).unwrap();
        assert_eq!(plan(&p).unwrap(), vorher);
    }
    #[test]
    fn nur_vorhabenflurstuecke_bilden_baugrundstuecke_und_projektflaeche() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let vorher = informationen(&p).fingerprint().unwrap();
        assert!(
            !serde_json::to_string(&informationen(&p))
                .unwrap()
                .contains("rolle"),
            "Vorhabenflurstücke behalten ihre bisherige Darstellung"
        );
        assert_eq!(informationen(&p).vorhabenflaeche_m2(), Some(900.));

        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        infos.flurstuecke[2].rolle = FlurstuecksRolle::Nachbar;
        infos.validate().unwrap();
        assert_ne!(informationen(&p).fingerprint().unwrap(), vorher);
        assert!(
            !quellen_aktuell(&p).unwrap(),
            "Ermittlung muss neu bestätigt werden"
        );
        let ids: Vec<_> = plan(&p)
            .unwrap()
            .unwrap()
            .flurstuecke
            .into_iter()
            .map(|f| f.id)
            .collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(informationen(&p).vorhabenflaeche_m2(), Some(500.));

        let mut projekt = crate::domain::Project {
            id: crate::domain::ProjectId::new(),
            name: "P".into(),
            bundesland: crate::domain::Bundesland::By,
            grundstuecksflaeche_m2: Some(9999.),
            planungsrecht: p,
            version: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        projektflaeche_ableiten(&mut projekt);
        assert_eq!(projekt.grundstuecksflaeche_m2, Some(500.));
        projekt.planungsrecht = Planungsrecht::default();
        projekt.grundstuecksflaeche_m2 = Some(812.);
        projektflaeche_ableiten(&mut projekt);
        assert_eq!(
            projekt.grundstuecksflaeche_m2,
            Some(812.),
            "Bestandsprojekt"
        );
    }
    #[test]
    fn buchgrundstueck_gehoert_ganz_oder_gar_nicht_zum_vorhaben() {
        let mut p = Planungsrecht {
            baugrundstuecke: Some(musterplan()),
            ..Default::default()
        };
        normalisieren(&mut p).unwrap();
        let infos = p.grundstuecksinformationen.as_mut().unwrap();
        infos.flurstuecke[1].rolle = FlurstuecksRolle::Sonstiges;
        let err = infos.validate().unwrap_err().to_string();
        assert!(err.contains("selben Buchgrundstück"), "{err}");
        for f in &mut infos.flurstuecke {
            f.rolle = FlurstuecksRolle::Nachbar;
        }
        let err = infos.validate().unwrap_err().to_string();
        assert!(err.contains("Mindestens ein Flurstück"), "{err}");
    }
}
