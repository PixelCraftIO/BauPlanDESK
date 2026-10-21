//! Projektbezogene, archivfähige Abstandsprüfung. Rechtliche Entscheidungen,
//! Geometrie und Nachweisarchiv besitzen getrennte Verträge.
pub mod bericht;
mod cadhoehen;
mod geometrie;
mod hoehen;
mod umriss;
use crate::{CoreError, CoreResult, domain::Bundesland, plattform::objekte::Dokumentbezug};
pub use cadhoehen::{CadHoehe, gelaende_vorschlag};
use serde::{Deserialize, Serialize};
pub use umriss::{UmrissAbgleich, umrisse_abgleichen, umrisse_einbinden};
use uuid::Uuid;
/// Ab Version 2: Höhenprofile, Wandteile und Wandhöhenansatz des Regelpakets.
/// Pläne ohne diese Angaben rechnen wie Version 1.
/// 3: Abstandsfläche, die die maßgebliche Grenze von innen nur berührt, ist
/// eingehalten; Grenzanlagen zählen nur ihre Wandlängen an der Grenze, übrige
/// Wände derselben Anlage haben keine eigene Abstandsfläche.
pub const RECHENSTAND: &str = "abstand-projekt-3";
/// Ältere Rechenstände, die der aktuelle Kern für Altpläne unverändert reproduziert.
pub const RECHENSTAENDE_KOMPATIBEL: &[&str] = &["abstand-projekt-1", "abstand-projekt-2"];
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct APunkt {
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Tiefenregel {
    pub gebiete: Vec<String>,
    pub faktor: f64,
    pub mindesttiefe_m: f64,
    pub quelle: String,
    /// Besonderer Tiefenfall; fehlt bei der Regeltiefe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fall: Option<Tiefenfall>,
}
/// Tiefenfälle, die eine Landesbauordnung abweichend von der Regeltiefe regelt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Tiefenfall {
    #[default]
    Regel,
    /// Abstandsfläche zu öffentlichen Verkehrs-, Grün- und Wasserflächen.
    OeffentlicheFlaeche,
    /// Wohngebäude der Gebäudeklassen 1 und 2 mit höchstens drei oberirdischen Geschossen.
    WohngebaeudeGk12,
}
/// Wie aus dem Höhenverlauf einer Wand(teil)s die Wandhöhe H wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Wandhoehenansatz {
    /// Größte Wandhöhe je Wandteil (konservativ; Verhalten vor Version 2).
    #[default]
    Maximal,
    /// Mittel der Wandhöhen an den Gebäudekanten bzw. Wandteilgrenzen.
    MittelKanten,
    /// Flächengemittelte Wandhöhe über das gesamte Profil.
    MittelProfil,
    /// Wandhöhe an jeder Stelle; Abstandsfläche mit wechselnder Tiefe.
    Punktweise,
}
impl Tiefenfall {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Regel => "Regeltiefe",
            Self::OeffentlicheFlaeche => "Zu öffentlicher Verkehrs-, Grün- oder Wasserfläche",
            Self::WohngebaeudeGk12 => {
                "Wohngebäude GK 1/2 mit höchstens drei oberirdischen Geschossen"
            }
        }
    }
}
impl Wandhoehenansatz {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Maximal => "größte Wandhöhe je Wandteil",
            Self::MittelKanten => "im Mittel gemessene Wandhöhe aus den Kanten je Wandteil",
            Self::MittelProfil => "flächengemittelte Wandhöhe je Wandteil",
            Self::Punktweise => "Wandhöhe an jeder Stelle (wechselnde Tiefe)",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Giebelregel {
    /// Haben beide Dachseiten eine größere Neigung, gilt `faktor_steil`.
    pub steil_ueber_grad: f64,
    pub faktor_steil: f64,
    pub faktor_sonst: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Gaubenregel {
    /// Anteil der Gaubenlänge an der darunterliegenden Wand, ab dem `faktor` gilt.
    pub anteil_ueber: f64,
    pub faktor: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Ueberdeckungsregel {
    /// Überdeckung zulässig, wenn die Wände in einem größeren Winkel zueinander stehen.
    pub winkel_ueber_grad: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dachregel {
    pub ueber_grad: f64,
    pub faktor: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Grenzanlagenregel {
    pub typen: Vec<String>,
    pub hoehe_m: f64,
    pub laenge_je_grenze_m: f64,
    pub laenge_gesamt_m: f64,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsRegeln {
    pub id: String,
    pub version: String,
    pub land: Bundesland,
    pub rechtsstand: String,
    pub hinweis: String,
    pub tiefen: Vec<Tiefenregel>,
    pub dach: Vec<Dachregel>,
    pub grenzanlagen: Option<Grenzanlagenregel>,
    /// Fehlt in Paketen vor Version 2: dann [`Wandhoehenansatz::Maximal`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wandhoehe: Option<Wandhoehenansatz>,
    /// Automatische Giebelanrechnung; ohne Regel bleibt der manuelle Ansatz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub giebel: Option<Giebelregel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gauben: Option<Gaubenregel>,
    /// Ausnahme vom Überdeckungsverbot nach Winkel; ohne Regel keine Ausnahme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ueberdeckung: Option<Ueberdeckungsregel>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsRecht {
    pub gemeinde: String,
    pub fassung: String,
    pub anwendung: String,
    pub gebiet: String,
    pub satzung: String,
    pub planfestsetzung: String,
    pub geprueft: bool,
    pub pruefvermerk: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsGrundstueck {
    pub id: Uuid,
    pub bezeichnung: String,
    pub flurstueck_ids: Vec<String>,
    pub grenze: Vec<APunkt>,
    #[serde(default)]
    pub innenringe: Vec<Vec<APunkt>>,
    pub grundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Flaechenart {
    Sicherung,
    OeffentlicheFlaeche,
    Baukoerper,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsFlaeche {
    pub id: Uuid,
    pub grundstueck_id: Uuid,
    pub art: Flaechenart,
    pub bezeichnung: String,
    pub polygon: Vec<APunkt>,
    pub grundlage: String,
    pub wirksam: bool,
    pub beleg: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aussenrichtung {
    Links,
    Rechts,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Dachart {
    Flach,
    Dachflaeche,
    Giebel,
    Manuell,
    Ungeklaert,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Wandfall {
    Regel,
    Garage,
    Carport,
    /// Gebäude ohne Aufenthaltsräume mit begrenztem Rauminhalt (z. B. Gartenhaus).
    Nebengebaeude,
    Vorbau,
    Grenzbebauung,
    Bestand,
    Sonderbau,
    Sonstiges,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Sonderansatz {
    pub tiefe_m: f64,
    pub grundlage: String,
    pub wirksam: bool,
    pub beleg: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsWand {
    pub id: Uuid,
    /// Optionaler Bezug zu Gebäude/Geschosse. Mit Bezug übernimmt der Core die
    /// Gebäudebezeichnung; fehlt in älteren Plänen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gebaeude_id: Option<Uuid>,
    pub gebaeude: String,
    pub bezeichnung: String,
    pub grundstueck_id: Option<Uuid>,
    pub von: APunkt,
    pub bis: APunkt,
    pub aussen: Aussenrichtung,
    pub gelaende_a_m: Option<f64>,
    pub gelaende_b_m: Option<f64>,
    pub abschluss_a_m: Option<f64>,
    pub abschluss_b_m: Option<f64>,
    pub dachart: Dachart,
    pub dachhoehe_m: Option<f64>,
    pub dachneigung_grad: Option<f64>,
    pub dachfaktor_manuell: Option<f64>,
    pub hoehenquelle: String,
    pub dachgrundlage: String,
    pub fall: Wandfall,
    pub grenzkennung: String,
    pub nutzung_geprueft: bool,
    pub sonderansatz: Option<Sonderansatz>,
    pub polygon_manuell: Option<Vec<APunkt>>,
    /// Geländehöhen entlang der Wand. Ersetzt `gelaende_a_m`/`gelaende_b_m`;
    /// fehlt in älteren Plänen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Hoehenpunkt>>")]
    pub gelaende: Vec<Hoehenpunkt>,
    /// Wandoberkante (Schnittlinie mit der Dachhaut bzw. oberer Abschluss).
    /// Zwei Punkte an derselben Station bilden einen Höhensprung und damit eine
    /// Wandteilgrenze. Ersetzt `abschluss_a_m`/`abschluss_b_m`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Hoehenpunkt>>")]
    pub oberkante: Vec<Hoehenpunkt>,
    /// Zusätzliche vertikale Wandteilgrenzen (Station in m ab Wandanfang).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<f64>>")]
    pub wandteilgrenzen: Vec<f64>,
    /// Neigung der zweiten Dachseite bei Giebeln.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dachneigung_b_grad: Option<f64>,
    /// Gesamtlänge der Gauben/Dachaufbauten auf der Dachfläche über dieser Wand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gauben_laenge_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tiefenfall: Option<Tiefenfall>,
    /// Begründung für einen besonderen Tiefenfall.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(optional, as = "Option<String>")]
    pub tiefenfall_grundlage: String,
    /// Aus einem Gebäudeumriss erzeugte Seite; Lage und Eckhöhen führt der Umriss.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub umriss: Option<UmrissSeite>,
}
/// Höhe an einer Stelle der Wand; `station_m` ab Wandanfang (`von`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Hoehenpunkt {
    pub station_m: f64,
    pub hoehe_m: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(optional, as = "Option<String>")]
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct UmrissSeite {
    pub umriss_id: Uuid,
    /// Seite von Ecke `seite` zu Ecke `seite + 1`.
    pub seite: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct UmrissEcke {
    pub x: f64,
    pub y: f64,
    /// Maßgebliche Geländehöhe an der Gebäudeecke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gelaende_m: Option<f64>,
}
/// Außenkontur eines Gebäudes; daraus entstehen die Wände je Seite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsUmriss {
    pub id: Uuid,
    pub bezeichnung: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gebaeude_id: Option<Uuid>,
    pub grundstueck_id: Option<Uuid>,
    pub ecken: Vec<UmrissEcke>,
    /// Vorgabe der Wandoberkante (z. B. Traufe) für neue Seiten.
    pub oberkante_m: Option<f64>,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct AbstandsPlan {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<crate::planaufmass::Planbindung>>")]
    pub planbindungen: Vec<crate::planaufmass::Planbindung>,
    pub bezeichnung: String,
    pub regeln: AbstandsRegeln,
    pub recht: AbstandsRecht,
    pub hoehenbezug: String,
    pub koordinatenbezug: String,
    pub grundstuecke: Vec<AbstandsGrundstueck>,
    pub waende: Vec<AbstandsWand>,
    pub flaechen: Vec<AbstandsFlaeche>,
    pub quellen: Vec<Dokumentbezug>,
    pub umfang_geprueft: bool,
    pub umfang_vermerk: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<AbstandsUmriss>>")]
    pub umrisse: Vec<AbstandsUmriss>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AbstandsPolygon {
    pub aussen: Vec<APunkt>,
    pub innen: Vec<Vec<APunkt>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AbstandsStatus {
    Offen,
    Tiefenvorpruefung,
    Eingehalten,
    Konflikt,
    Grenzfall,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct WandErgebnis {
    pub id: Uuid,
    pub bezeichnung: String,
    pub laenge_m: f64,
    pub wandhoehe_m: Option<f64>,
    pub dachanteil_m: Option<f64>,
    pub h_m: Option<f64>,
    pub faktor: Option<f64>,
    pub mindesttiefe_m: Option<f64>,
    pub tiefe_m: Option<f64>,
    pub herleitung: Vec<String>,
    pub offen: Vec<String>,
    pub status: AbstandsStatus,
    pub flaeche_m2: Option<f64>,
    pub ausserhalb_m2: Option<f64>,
    pub polygon: Vec<APunkt>,
    pub konflikte: Vec<AbstandsPolygon>,
    /// Angewandter Wandhöhenansatz; fehlt in Ergebnissen vor Version 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ansatz: Option<Wandhoehenansatz>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Wandteilergebnis>>")]
    pub wandteile: Vec<Wandteilergebnis>,
    /// Höhenverlauf an allen Profil- und Teilungsstellen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Profilzeile>>")]
    pub profil: Vec<Profilzeile>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Wandteilergebnis {
    pub von_m: f64,
    pub bis_m: f64,
    /// Wandhöhe an den Kanten des Wandteils.
    pub h_anfang_m: f64,
    pub h_ende_m: f64,
    /// Maßgebliche Wandhöhe nach dem Ansatz, ohne Dachanteil.
    pub wandhoehe_m: f64,
    pub h_m: Option<f64>,
    /// Größte Tiefe im Wandteil.
    pub tiefe_m: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Profilzeile {
    pub station_m: f64,
    pub gelaende_m: f64,
    pub oberkante_m: f64,
    pub wandhoehe_m: f64,
    /// Tiefe an dieser Stelle (bei gemittelten Ansätzen die des Wandteils).
    pub tiefe_m: Option<f64>,
    /// Laufende Nummer des Wandteils ab 1.
    pub wandteil: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Ueberdeckung {
    pub wand_a: Uuid,
    pub wand_b: Uuid,
    pub flaeche_m2: f64,
    pub polygone: Vec<AbstandsPolygon>,
    /// Winkel, in dem die Wände zueinander stehen (0° = parallel gegenüber).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub winkel_grad: Option<f64>,
    /// Rechtsgrundlage, nach der die Überdeckung zulässig ist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub zulaessig: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AbstandsErgebnis {
    pub rechenstand: String,
    pub waende: Vec<WandErgebnis>,
    pub ueberdeckungen: Vec<Ueberdeckung>,
    pub offen: Vec<String>,
    pub vollstaendig: bool,
    pub konfliktfrei: bool,
}
pub(super) fn err(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn text(s: &str, max: usize) -> CoreResult<()> {
    if s.len() > max {
        Err(err("Abstandsangabe zu lang"))
    } else {
        Ok(())
    }
}
pub fn katalog() -> CoreResult<Vec<AbstandsRegeln>> {
    serde_json::from_str(include_str!(
        "../../../../regelwerke/abstandsflaechen/pilot.json"
    ))
    .map_err(|e| err(e.to_string()))
}
impl AbstandsPlan {
    pub fn dokumente(&self) -> Vec<Dokumentbezug> {
        self.quellen
            .iter()
            .chain(self.flaechen.iter().filter_map(|f| f.beleg.as_ref()))
            .chain(
                self.waende
                    .iter()
                    .filter_map(|w| w.sonderansatz.as_ref().and_then(|s| s.beleg.as_ref())),
            )
            .cloned()
            .collect()
    }
    pub fn validate(&self) -> CoreResult<()> {
        text(&self.bezeichnung, 500)?;
        text(&self.hoehenbezug, 1000)?;
        text(&self.koordinatenbezug, 1000)?;
        if self.waende.len() > 200
            || self.grundstuecke.len() > 100
            || self.flaechen.len() > 200
            || self.quellen.len() > 200
        {
            return Err(err(
                "Höchstens 200 Wände/Flächen/Quellen und 100 Grundstücke",
            ));
        }
        for s in [
            &self.recht.gemeinde,
            &self.recht.fassung,
            &self.recht.anwendung,
            &self.recht.gebiet,
            &self.recht.satzung,
            &self.recht.planfestsetzung,
            &self.recht.pruefvermerk,
            &self.umfang_vermerk,
        ] {
            text(s, 8000)?;
        }
        let r = &self.regeln;
        if r.id.trim().is_empty()
            || r.version.trim().is_empty()
            || r.tiefen.is_empty()
            || r.tiefen.len() > 30
            || r.dach.len() > 10
        {
            return Err(err(
                "Abstandsregelpaket benötigt Kennung, Version und Tiefenregeln",
            ));
        }
        for s in [&r.id, &r.version, &r.rechtsstand, &r.hinweis] {
            text(s, 8000)?;
        }
        for t in &r.tiefen {
            if t.gebiete.is_empty()
                || t.gebiete.len() > 30
                || !t.faktor.is_finite()
                || t.faktor < 0.
                || t.faktor > 10.
                || (t.faktor == 0. && t.mindesttiefe_m <= 0.)
                || !t.mindesttiefe_m.is_finite()
                || t.mindesttiefe_m < 0.
                || t.mindesttiefe_m > 1000.
                || t.quelle.trim().is_empty()
            {
                return Err(err("Ungültige Tiefenregel"));
            }
            text(&t.quelle, 4000)?;
        }
        for d in &r.dach {
            if !d.ueber_grad.is_finite()
                || !(0. ..=90.).contains(&d.ueber_grad)
                || !d.faktor.is_finite()
                || !(0. ..=10.).contains(&d.faktor)
                || d.quelle.trim().is_empty()
            {
                return Err(err("Ungültige Dachregel"));
            }
        }
        if let Some(g) = &r.grenzanlagen {
            for n in [g.hoehe_m, g.laenge_je_grenze_m, g.laenge_gesamt_m] {
                if !n.is_finite() || n <= 0. || n > 1000. {
                    return Err(err("Ungültige Grenzanlagenregel"));
                }
            }
            text(&g.quelle, 4000)?;
        }
        let faktor = |f: f64| f.is_finite() && (0. ..=10.).contains(&f);
        if let Some(g) = &r.giebel
            && (!faktor(g.faktor_steil)
                || !faktor(g.faktor_sonst)
                || !(0. ..=90.).contains(&g.steil_ueber_grad)
                || g.quelle.trim().is_empty())
        {
            return Err(err("Ungültige Giebelregel"));
        }
        if let Some(g) = &r.gauben
            && (!faktor(g.faktor)
                || !(0. ..=1.).contains(&g.anteil_ueber)
                || g.quelle.trim().is_empty())
        {
            return Err(err("Ungültige Gaubenregel"));
        }
        if let Some(u) = &r.ueberdeckung
            && (!(0. ..=180.).contains(&u.winkel_ueber_grad) || u.quelle.trim().is_empty())
        {
            return Err(err("Ungültige Überdeckungsregel"));
        }
        for s in r
            .giebel
            .iter()
            .map(|g| &g.quelle)
            .chain(r.gauben.iter().map(|g| &g.quelle))
            .chain(r.ueberdeckung.iter().map(|u| &u.quelle))
        {
            text(s, 4000)?;
        }
        if self.umrisse.len() > 50 {
            return Err(err("Höchstens 50 Gebäudeumrisse"));
        }
        for u in &self.umrisse {
            text(&u.bezeichnung, 500)?;
            text(&u.quelle, 4000)?;
            if !(3..=500).contains(&u.ecken.len()) {
                return Err(err("Gebäudeumriss benötigt 3 bis 500 Ecken"));
            }
            geometrie::polygon(
                &u.ecken
                    .iter()
                    .map(|e| APunkt { x: e.x, y: e.y })
                    .collect::<Vec<_>>(),
            )?;
            if u.ecken
                .iter()
                .filter_map(|e| e.gelaende_m)
                .chain(u.oberkante_m)
                .any(|h| !h.is_finite() || h.abs() > 10000.)
            {
                return Err(err("Ungültige Höhe am Gebäudeumriss"));
            }
            if u.grundstueck_id
                .is_some_and(|id| !self.grundstuecke.iter().any(|g| g.id == id))
            {
                return Err(err("Gebäudeumriss verweist auf unbekanntes Grundstück"));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for id in self
            .waende
            .iter()
            .map(|w| w.id)
            .chain(self.grundstuecke.iter().map(|g| g.id))
            .chain(self.flaechen.iter().map(|f| f.id))
            .chain(self.umrisse.iter().map(|u| u.id))
        {
            if id.is_nil() || !ids.insert(id) {
                return Err(err("Eindeutige Abschnitts-/Flächenkennungen erforderlich"));
            }
        }
        for g in &self.grundstuecke {
            text(&g.bezeichnung, 500)?;
            text(&g.grundlage, 4000)?;
            if g.flurstueck_ids.len() > 1000 {
                return Err(err("Zu viele Flurstücksbezüge"));
            }
            if !g.grenze.is_empty() {
                geometrie::grundstueck(g)?;
            }
        }
        for f in &self.flaechen {
            text(&f.bezeichnung, 500)?;
            text(&f.grundlage, 4000)?;
            if !self.grundstuecke.iter().any(|g| g.id == f.grundstueck_id) {
                return Err(err("Fläche benötigt eine vorhandene Grundstückseinheit"));
            }
            if !f.polygon.is_empty() {
                geometrie::polygon(&f.polygon)?;
            }
        }
        for w in &self.waende {
            for s in [
                &w.bezeichnung,
                &w.gebaeude,
                &w.hoehenquelle,
                &w.dachgrundlage,
                &w.grenzkennung,
                &w.tiefenfall_grundlage,
            ] {
                text(s, 4000)?;
            }
            if w.gelaende.len() > 500 || w.oberkante.len() > 500 || w.wandteilgrenzen.len() > 200 {
                return Err(err(
                    "Höchstens 500 Höhenpunkte und 200 Wandteilgrenzen je Wand",
                ));
            }
            for h in w.gelaende.iter().chain(&w.oberkante) {
                text(&h.quelle, 1000)?;
                if !h.station_m.is_finite()
                    || !h.hoehe_m.is_finite()
                    || h.station_m.abs() > 100_000.
                    || h.hoehe_m.abs() > 10000.
                {
                    return Err(err("Ungültiger Höhenpunkt"));
                }
            }
            if w.wandteilgrenzen
                .iter()
                .any(|s| !s.is_finite() || s.abs() > 100_000.)
                || w.gauben_laenge_m
                    .is_some_and(|l| !l.is_finite() || !(0. ..=100_000.).contains(&l))
                || w.dachneigung_b_grad
                    .is_some_and(|v| !v.is_finite() || !(0. ..=90.).contains(&v))
            {
                return Err(err(
                    "Ungültige Wandteilgrenze, Gaubenlänge oder Dachneigung",
                ));
            }
            if let Some(s) = &w.umriss
                && !self
                    .umrisse
                    .iter()
                    .any(|u| u.id == s.umriss_id && s.seite < u.ecken.len())
            {
                return Err(err(
                    "Wand verweist auf eine nicht vorhandene Umrissseite; Umriss erneut übernehmen",
                ));
            }
            for p in [&w.von, &w.bis] {
                geometrie::punkt(p)?;
            }
            for n in [
                w.gelaende_a_m,
                w.gelaende_b_m,
                w.abschluss_a_m,
                w.abschluss_b_m,
                w.dachhoehe_m,
                w.dachneigung_grad,
                w.dachfaktor_manuell,
            ]
            .into_iter()
            .flatten()
            {
                if !n.is_finite() || n.abs() > 10000. {
                    return Err(err("Ungültige Höhen-/Dachangabe"));
                }
            }
            if w.dachhoehe_m.is_some_and(|v| v < 0.)
                || w.dachneigung_grad
                    .is_some_and(|v| !(0. ..=90.).contains(&v))
                || w.dachfaktor_manuell
                    .is_some_and(|v| !(0. ..=10.).contains(&v))
            {
                return Err(err(
                    "Dachhöhe/Faktor positiv, Neigung zwischen 0 und 90 Grad erforderlich",
                ));
            }
            if w.grundstueck_id
                .is_some_and(|id| !self.grundstuecke.iter().any(|g| g.id == id))
            {
                return Err(err("Wand verweist auf unbekanntes Grundstück"));
            }
            if let Some(s) = &w.sonderansatz {
                text(&s.grundlage, 8000)?;
                if !s.tiefe_m.is_finite() || s.tiefe_m < 0. || s.tiefe_m > 1000. {
                    return Err(err("Ungültige besondere Tiefe"));
                }
            }
            if let Some(p) = &w.polygon_manuell {
                geometrie::polygon(p)?;
            }
        }
        Ok(())
    }
}
pub fn berechnen(plan: &AbstandsPlan) -> CoreResult<AbstandsErgebnis> {
    berechnen_stand(plan, RECHENSTAND)
}
/// Rechnet mit einem bestimmten Rechenstand. Für archivierte Nachweise
/// `abstand-projekt-1` entsteht dasselbe Ergebnis wie damals (Maximalansatz aus
/// den Endwerten, Texte und Felder von Version 1).
pub fn berechnen_stand(plan: &AbstandsPlan, stand: &str) -> CoreResult<AbstandsErgebnis> {
    plan.validate()?;
    let (v1, v3) = match stand {
        RECHENSTAND => (false, true),
        "abstand-projekt-2" => (false, false),
        "abstand-projekt-1" => (true, false),
        _ => return Err(err("Unbekannter Rechenstand der Abstandsberechnung")),
    };
    // Ab Stand 3 einheitlich mit Dezimalkomma und höchstens drei Nachkommastellen;
    // ältere Stände behalten ihre archivierten Herleitungstexte.
    let z = |v: f64| if v3 { hoehen::zahl(v) } else { v.to_string() };
    let ansatz = plan.regeln.wandhoehe.unwrap_or_default();
    let mut offen = Vec::new();
    if plan.waende.is_empty() {
        offen.push("Noch keine Außenwände erfasst".into());
    }
    if plan.recht.satzung.trim().is_empty() || plan.recht.planfestsetzung.trim().is_empty() {
        offen.push("Ergebnis der Satzungs- und Planfestsetzungsprüfung dokumentieren (auch wenn keine Abweichung vorliegt)".into());
    }
    if !plan.recht.geprueft
        || plan.recht.pruefvermerk.trim().is_empty()
        || plan.recht.fassung.trim().is_empty()
        || plan.recht.anwendung.trim().is_empty()
        || plan.recht.gemeinde.trim().is_empty()
    {
        offen.push("Anwendbare Rechtsfassung, Gemeinde, Satzungs-/Planprüfung und Prüfvermerk fachlich bestätigen".into());
    }
    if !plan.umfang_geprueft || plan.umfang_vermerk.trim().is_empty() {
        offen.push("Vollständigkeit aller Wände, Ecken, Geländeabschnitte und Sonderfälle prüfen und begründen".into());
    }
    if plan.hoehenbezug.trim().is_empty() {
        offen.push("Maßgebliche Geländeoberfläche und Höhenbezug festlegen".into());
    }
    if plan.koordinatenbezug.trim().is_empty() {
        offen.push("Koordinatenbezug (Meter) bestätigen".into());
    }
    for f in &plan.flaechen {
        if f.polygon.len() < 3 {
            offen.push(format!("Fläche {} ohne gültigen Umriss", f.bezeichnung));
        }
    }
    let mut waende = Vec::new();
    for w in &plan.waende {
        let laenge = (w.bis.x - w.von.x).hypot(w.bis.y - w.von.y);
        let mut e = WandErgebnis {
            id: w.id,
            bezeichnung: w.bezeichnung.clone(),
            laenge_m: laenge,
            wandhoehe_m: None,
            dachanteil_m: None,
            h_m: None,
            faktor: None,
            mindesttiefe_m: None,
            tiefe_m: None,
            herleitung: vec![],
            offen: vec![],
            status: AbstandsStatus::Offen,
            flaeche_m2: None,
            ausserhalb_m2: None,
            polygon: vec![],
            konflikte: vec![],
            ansatz: None,
            wandteile: vec![],
            profil: vec![],
        };
        if w.bezeichnung.trim().is_empty() || w.gebaeude.trim().is_empty() {
            e.offen.push("Gebäude und Wandbezeichnung angeben".into());
        }
        if laenge < 0.001 {
            e.offen
                .push("Wandlänge mindestens 1 mm erforderlich".into());
        }
        if w.hoehenquelle.trim().is_empty() {
            e.offen
                .push("Gelände-/Höhenherleitung und Planquelle dokumentieren".into());
        }
        if v1 {
            e.wandhoehe_m = alt_wandhoehe(w, &mut e);
        }
        if !v1
            && let Some(s) = &w.umriss
            && let Some(u) = plan.umrisse.iter().find(|u| u.id == s.umriss_id)
        {
            let (a, b) = (&u.ecken[s.seite], &u.ecken[(s.seite + 1) % u.ecken.len()]);
            if (a.x, a.y, b.x, b.y) != (w.von.x, w.von.y, w.bis.x, w.bis.y) {
                e.offen.push(format!(
                    "Wand weicht vom Gebäudeumriss {} ab; Umriss erneut übernehmen",
                    u.bezeichnung
                ));
            }
        }
        let teile = if v1 {
            None
        } else {
            e.ansatz = Some(ansatz);
            e.herleitung.push(format!(
                "Wandhöhenansatz: {} ({})",
                ansatz.bezeichnung(),
                if plan.regeln.wandhoehe.is_some() {
                    plan.regeln.rechtsstand.as_str()
                } else {
                    "Regelpaket ohne Angabe – konservativer Ansatz"
                }
            ));
            let teile = if laenge < 0.001 {
                None
            } else {
                hoehen::auswerten(w, laenge, ansatz, &mut e)
            };
            e.wandhoehe_m = teile.as_ref().map(|t| {
                t.iter()
                    .map(|t| t.wandhoehe)
                    .fold(f64::NEG_INFINITY, f64::max)
            });
            teile
        };
        let dach = match w.dachart {
            Dachart::Flach => Some(0.),
            Dachart::Dachflaeche => {
                if plan.regeln.dach.is_empty() {
                    None
                } else if let (Some(h), Some(n)) = (w.dachhoehe_m, w.dachneigung_grad) {
                    let r = plan
                        .regeln
                        .dach
                        .iter()
                        .filter(|r| n > r.ueber_grad)
                        .max_by(|a, b| a.ueber_grad.total_cmp(&b.ueber_grad));
                    let mut f = r.map(|r| r.faktor).unwrap_or(0.);
                    e.herleitung.push(format!(
                        "Dachfläche: {} m × {} bei {}°; {}",
                        z(h),
                        z(f),
                        z(n),
                        r.map(|r| r.quelle.as_str())
                            .unwrap_or("unterhalb erster konfigurierter Schwelle")
                    ));
                    if let (Some(g), Some(lg)) = (&plan.regeln.gauben, w.gauben_laenge_m)
                        && lg > g.anteil_ueber * laenge
                        && g.faktor > f
                    {
                        f = g.faktor;
                        e.herleitung.push(format!(
                            "Gauben/Dachaufbauten {} m > {} × Wandlänge {} m: Dachhöhe {} m × {}; {}",
                            hoehen::zahl(lg),
                            hoehen::zahl(g.anteil_ueber),
                            hoehen::zahl(laenge),
                            z(h),
                            hoehen::zahl(f),
                            g.quelle
                        ));
                    }
                    Some(h * f)
                } else {
                    None
                }
            }
            Dachart::Giebel if plan.regeln.giebel.is_some() => {
                let g = plan.regeln.giebel.as_ref().expect("geprüft");
                let neigungen = [w.dachneigung_grad, w.dachneigung_b_grad];
                let steil = if neigungen
                    .iter()
                    .any(|n| n.is_some_and(|n| n <= g.steil_ueber_grad))
                {
                    Some(false)
                } else if neigungen.iter().all(Option::is_some) {
                    Some(true)
                } else {
                    None
                };
                match (w.dachhoehe_m, steil) {
                    (Some(h), Some(steil)) => {
                        let f = if steil {
                            g.faktor_steil
                        } else {
                            g.faktor_sonst
                        };
                        e.herleitung.push(format!(
                            "Giebelfläche: Höhe {} m × {} ({} beidseitig über {}°); {}",
                            hoehen::zahl(h),
                            hoehen::zahl(f),
                            if steil {
                                "Dachneigung"
                            } else {
                                "Dachneigung nicht"
                            },
                            hoehen::zahl(g.steil_ueber_grad),
                            g.quelle
                        ));
                        Some(h * f)
                    }
                    _ => {
                        e.offen
                            .push("Giebelhöhe und Neigung beider Dachseiten angeben".into());
                        None
                    }
                }
            }
            Dachart::Giebel | Dachart::Manuell => {
                if !w.dachgrundlage.trim().is_empty() {
                    w.dachhoehe_m.zip(w.dachfaktor_manuell).map(|(h, f)| {
                        e.herleitung.push(format!(
                            "Fachlich angesetzter Dach-/Giebelanteil: {} m × {}; {}",
                            z(h),
                            z(f),
                            w.dachgrundlage
                        ));
                        h * f
                    })
                } else {
                    None
                }
            }
            Dachart::Ungeklaert => None,
        };
        e.dachanteil_m = dach;
        if dach.is_none() {
            e.offen
                .push("Dach-/Giebelansatz einschließlich Rechtsgrundlage klären".into());
        }
        e.h_m = e.wandhoehe_m.zip(dach).map(|(a, b)| a + b);
        let fall = w.tiefenfall.unwrap_or_default();
        let passend = |f: Tiefenfall| {
            plan.regeln
                .tiefen
                .iter()
                .filter(|r| {
                    r.gebiete.contains(&plan.recht.gebiet) && r.fall.unwrap_or_default() == f
                })
                .collect::<Vec<_>>()
        };
        let mut regeln = passend(fall);
        if fall != Tiefenfall::Regel {
            if w.tiefenfall_grundlage.trim().is_empty() {
                e.offen.push(format!(
                    "Tiefenfall „{}“ begründen (z. B. Gebäudeklasse, Geschosse, angrenzende Fläche)",
                    fall.bezeichnung()
                ));
            }
            if regeln.is_empty() {
                e.herleitung.push(format!(
                    "Tiefenfall „{}“: Das Regelpaket sieht für das Gebiet {} keine abweichende Tiefe vor; Regeltiefe angesetzt.",
                    fall.bezeichnung(),
                    plan.recht.gebiet
                ));
                regeln = passend(Tiefenfall::Regel);
            } else {
                e.herleitung.push(format!(
                    "Tiefenfall „{}“: {}",
                    fall.bezeichnung(),
                    w.tiefenfall_grundlage
                ));
            }
        }
        let regel = if regeln.len() == 1 {
            let r = regeln[0];
            e.faktor = Some(r.faktor);
            e.mindesttiefe_m = Some(r.mindesttiefe_m);
            e.herleitung.push(if r.faktor == 0. {
                format!("T = {} m (feste Tiefe); {}", z(r.mindesttiefe_m), r.quelle)
            } else {
                format!(
                    "T = max({} × H, {} m); {}",
                    z(r.faktor),
                    z(r.mindesttiefe_m),
                    r.quelle
                )
            });
            Some((r.faktor, r.mindesttiefe_m))
        } else {
            e.offen
                .push("Baugebiet hat keine eindeutige konfigurierte Tiefenregel".into());
            None
        };
        let mut verlauf = Vec::new();
        if v1 && let Some(t) = e.h_m.zip(regel).map(|(h, (f, m))| (h * f).max(m)) {
            e.tiefe_m = Some(t);
            verlauf = vec![(0., t), (laenge, t)];
        }
        if let Some(teile) = &teile {
            hoehen::ergebnis(teile, dach, regel, ansatz, &mut e);
            if let (Some(d), Some((f, m))) = (dach, regel) {
                verlauf = hoehen::tiefen(teile, d, f, m, ansatz);
                e.tiefe_m = verlauf.iter().map(|v| v.1).reduce(f64::max);
            }
        }
        if matches!(
            w.fall,
            Wandfall::Garage | Wandfall::Carport | Wandfall::Nebengebaeude
        ) {
            let typ = match w.fall {
                Wandfall::Garage => "garage",
                Wandfall::Carport => "carport",
                _ => "nebengebaeude",
            };
            if let Some(r) = &plan.regeln.grenzanlagen {
                let anlagen = plan
                    .waende
                    .iter()
                    .filter(|a| {
                        a.grundstueck_id == w.grundstueck_id
                            && matches!(
                                a.fall,
                                Wandfall::Garage | Wandfall::Carport | Wandfall::Nebengebaeude
                            )
                            // Ab Rechenstand 3 zählen nur Wände an der Grenze.
                            && (!v3 || !a.grenzkennung.trim().is_empty())
                    })
                    .collect::<Vec<_>>();
                // Ab Rechenstand 3: Wand derselben Anlage, die selbst nicht an der
                // Grenze steht, wenn die Anlage mit einer Wand an der Grenze steht.
                let gleiche_anlage = |a: &&AbstandsWand| match (a.gebaeude_id, w.gebaeude_id) {
                    (Some(x), Some(y)) => x == y,
                    _ => !w.gebaeude.trim().is_empty() && a.gebaeude.trim() == w.gebaeude.trim(),
                };
                let teil_der_anlage = v3
                    && w.grenzkennung.trim().is_empty()
                    && anlagen.iter().any(|a| {
                        a.id != w.id && a.fall == w.fall && a.nutzung_geprueft && gleiche_anlage(a)
                    });
                let sum = |grenze: bool| {
                    anlagen
                        .iter()
                        .filter(|a| !grenze || a.grenzkennung == w.grenzkennung)
                        .map(|a| (a.bis.x - a.von.x).hypot(a.bis.y - a.von.y))
                        .sum::<f64>()
                };
                if r.typen.iter().any(|s| s == typ)
                    && w.nutzung_geprueft
                    && (!w.grenzkennung.trim().is_empty() || teil_der_anlage)
                    && e.h_m.is_some_and(|h| h <= r.hoehe_m)
                    && sum(true) <= r.laenge_je_grenze_m
                    && sum(false) <= r.laenge_gesamt_m
                {
                    e.tiefe_m = Some(0.);
                    verlauf = vec![(0., 0.), (laenge, 0.)];
                    if teil_der_anlage {
                        e.herleitung.push(format!("Wand einer Grenzanlage, die an der Grenze steht: ohne eigene Abstandsfläche; Höhe ≤ {}, Grenzlänge gesamt {} ≤ {}; {}", hoehen::zahl(r.hoehe_m), hoehen::zahl(sum(false)), hoehen::zahl(r.laenge_gesamt_m), r.quelle));
                    } else {
                        e.herleitung.push(format!("Grenzanlage: Höhe ≤ {}, Grenzlänge {} ≤ {}, Gesamtlänge {} ≤ {}; Nutzung/Bedingungen bestätigt; {}",z(r.hoehe_m),z(sum(true)),z(r.laenge_je_grenze_m),z(sum(false)),z(r.laenge_gesamt_m),r.quelle));
                    }
                } else {
                    e.offen.push("Grenzanlagenprivileg nicht vollständig belegt oder Höchstmaße/Summen überschritten; Regelabstand bleibt angesetzt".into());
                }
            } else {
                e.offen
                    .push("Keine konfigurierte Privilegierung für diese Grenzanlage".into());
            }
        } else if w.fall != Wandfall::Regel {
            e.offen.push(
                "Sonderfall benötigt dokumentierten besonderen Ansatz oder Einzelprüfung".into(),
            );
        }
        if let Some(s) = &w.sonderansatz {
            if s.wirksam && s.beleg.is_some() && !s.grundlage.trim().is_empty() {
                e.tiefe_m = Some(s.tiefe_m);
                verlauf = vec![(0., s.tiefe_m), (laenge, s.tiefe_m)];
                e.offen.retain(|v| {
                    !v.starts_with("Sonderfall")
                        && !v.starts_with("Grenzanlagenprivileg")
                        && !v.starts_with("Keine konfigurierte")
                });
                e.herleitung.push(format!(
                    "Besonderer belegter Ansatz: {} m; {}",
                    z(s.tiefe_m),
                    s.grundlage
                ));
            } else {
                e.offen
                    .push("Besonderer Ansatz ist noch nicht wirksam oder nicht belegt".into());
            }
        }
        if !verlauf.is_empty() && laenge >= 0.001 {
            geometrie::wand(plan, w, &mut e, &verlauf, v3)?;
        }
        if !e.offen.is_empty() {
            e.status = AbstandsStatus::Offen;
        }
        waende.push(e);
    }
    let mut ueberdeckungen = geometrie::ueberdeckungen(plan, &mut waende)?;
    if v1 {
        for u in &mut ueberdeckungen {
            (u.winkel_grad, u.zulaessig) = (None, None);
        }
    }
    let unzulaessig = ueberdeckungen.iter().any(|u| u.zulaessig.is_none());
    if unzulaessig {
        offen.push("Überdeckungen zwischen Abstandsflächen: Zulässigkeit gesondert prüfen; keine automatische Ausnahme".into());
    }
    let vollstaendig = offen.is_empty()
        && !waende.is_empty()
        && waende.iter().all(|w| {
            w.offen.is_empty()
                && w.status != AbstandsStatus::Tiefenvorpruefung
                && w.status != AbstandsStatus::Grenzfall
        });
    let konfliktfrei = vollstaendig
        && waende
            .iter()
            .all(|w| w.status == AbstandsStatus::Eingehalten)
        && !unzulaessig;
    Ok(AbstandsErgebnis {
        rechenstand: stand.into(),
        waende,
        ueberdeckungen,
        offen,
        vollstaendig,
        konfliktfrei,
    })
}
/// Abgleich mit dem Planungsrecht des Projekts (ab Rechenstand 3): Das Baugebiet
/// muss der Festsetzung entsprechen, und die Grenze eines Vorhabengrundstücks muss
/// zur Katasterfläche seiner Flurstücke passen (Toleranz 2 %). Abweichungen sind
/// offene Punkte. Der Abgleich hängt nur vom archivierten Rechtskontext ab und ist
/// damit bei der Nachweiswiederholung reproduzierbar.
pub fn projektabgleich(
    plan: &AbstandsPlan,
    recht: &crate::domain::Planungsrecht,
    r: &mut AbstandsErgebnis,
) {
    if r.rechenstand != RECHENSTAND {
        return;
    }
    let mut neu = Vec::new();
    if let Some(g) = recht.festgesetzte_werte.baugebiet
        && g.code() != plan.recht.gebiet.trim()
    {
        neu.push(format!(
            "Baugebiet „{}“ weicht von der Festsetzung des Projekts ({}) ab; Gebiet im Abstandsplan angleichen",
            plan.recht.gebiet,
            g.code()
        ));
    }
    let infos = crate::grundstuecksinformationen::informationen(recht);
    for g in &plan.grundstuecke {
        let flurstuecke = g
            .flurstueck_ids
            .iter()
            .filter_map(|id| infos.flurstuecke.iter().find(|f| &f.id == id))
            .collect::<Vec<_>>();
        let kataster = flurstuecke.iter().map(|f| f.flaeche_m2).sum::<f64>();
        if kataster <= 0. || !flurstuecke.iter().any(|f| f.rolle.ist_vorhaben()) {
            continue;
        }
        let flaeche = |p: &[APunkt]| {
            use geo::Area;
            geometrie::polygon(p)
                .map(|p| p.unsigned_area())
                .unwrap_or(0.)
        };
        let grenze = flaeche(&g.grenze) - g.innenringe.iter().map(|r| flaeche(r)).sum::<f64>();
        if (grenze - kataster).abs() > 0.02 * kataster {
            neu.push(format!(
                "{}: Grenze umschließt {} m², Katasterfläche {} m² (Abweichung über 2 %); Grenzgeometrie, Maßstab und Flurstückszuordnung prüfen",
                g.bezeichnung,
                hoehen::zahl((grenze * 100.).round() / 100.),
                hoehen::zahl(kataster)
            ));
        }
    }
    if !neu.is_empty() {
        r.offen.extend(neu);
        r.vollstaendig = false;
        r.konfliktfrei = false;
    }
}
/// Wandhöhe wie in Rechenstand 1: größte Höhe der beiden Wandenden.
fn alt_wandhoehe(w: &AbstandsWand, e: &mut WandErgebnis) -> Option<f64> {
    let (Some(ga), Some(gb), Some(oa), Some(ob)) = (
        w.gelaende_a_m,
        w.gelaende_b_m,
        w.abschluss_a_m,
        w.abschluss_b_m,
    ) else {
        e.offen
            .push("Gelände und Wandabschluss an beiden Abschnittsenden eingeben".into());
        return None;
    };
    let h = (oa - ga).max(ob - gb);
    if h < 0. || oa < ga || ob < gb {
        e.offen
            .push("Wandabschluss darf nicht unter maßgeblichem Gelände liegen".into());
        return None;
    }
    e.herleitung.push(format!("Konservativer Maximalansatz: max({oa} − {ga}, {ob} − {gb}) = {h} m. Bei nichtlinearem Verlauf Wand in Abschnitte aufteilen."));
    Some(h)
}
mod service;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;
#[cfg(test)]
mod tests;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct CadKontur {
    pub linie: usize,
    pub ebene: String,
    pub punkte: Vec<APunkt>,
    pub quelle: Dokumentbezug,
}
