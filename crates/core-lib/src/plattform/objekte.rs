use super::*;
use crate::{
    benutzer::BenutzerId,
    domain::{DocumentId, Project},
};
use chrono::NaiveDate;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Projektphase {
    Vorbereitung,
    Vorplanung,
    Entwurf,
    Genehmigung,
    Ausfuehrung,
    Abgeschlossen,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Projektorganisation {
    pub kennzeichen: String,
    pub phase: Projektphase,
    pub verantwortlich: Option<BenutzerId>,
    pub beschreibung: String,
    /// Navigationsbereiche, die das Büro für dieses Projekt ausdrücklich als
    /// nicht benötigt gekennzeichnet hat (siehe [`crate::bereichsstatus`]).
    /// Fehlt in älteren Ständen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub nicht_benoetigt: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dokumentbezug {
    pub dokument_id: DocumentId,
    #[ts(type = "number")]
    pub version: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Gebaeude {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<crate::planaufmass::Planbindung>>")]
    pub planbindungen: Vec<crate::planaufmass::Planbindung>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geometrie: Option<super::gebaeude::GebaeudeGeometrie>,
    pub bezeichnung: String,
    /// Flurstücke (IDs der Grundstücksinformationen), auf denen das Gebäude
    /// steht, bei Vereinigungsbaulast auch mehrere. Das Baugrundstück leitet der
    /// Core je Berechnung daraus ab. Fehlt in älteren Ständen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub flurstueck_ids: Vec<String>,
    /// `true`: Die Flurstücke stammen aus einer früheren Baugrundstückszuordnung
    /// und sind noch auf die tatsächlich bebauten Flurstücke zu prüfen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flurstuecke_zu_pruefen: Option<bool>,
    /// Veraltet: frühere Zuordnung zu Baugrundstücken. Wird nur noch gelesen,
    /// solange keine Flurstücke zugeordnet sind; neu wird sie nicht gesetzt.
    #[serde(default)]
    pub grundstueck_ids: Vec<Uuid>,
    pub geschosse: Vec<GebaeudeGeschoss>,
    pub quelle: Option<Dokumentbezug>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct GebaeudeGeschoss {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geometrie: Option<super::gebaeude::GeschossGeometrie>,
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub einstufungsquelle: Option<crate::berechnungen::Einstufungsquelle>,
    pub bezeichnung: String,
    pub vollgeschoss: Option<bool>,
    pub grundlage: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AufgabenStatus {
    Offen,
    InArbeit,
    Erledigt,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Aufgabe {
    pub titel: String,
    pub beschreibung: String,
    pub faellig: Option<NaiveDate>,
    pub verantwortlich: Option<BenutzerId>,
    pub status: AufgabenStatus,
    pub bezug: Option<Uuid>,
}
/// Art eines Projekttagebucheintrags.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Tagebuchart {
    Notiz,
    Telefonat,
    Besprechung,
    Ortstermin,
    /// Mangel oder Vorfall, etwa auf der Baustelle oder bei einer Lieferung.
    Vorfall,
    /// Entscheidung oder Weisung des Bauherrn.
    EntscheidungBauherr,
    Sonstiges,
}
impl Tagebuchart {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Notiz => "Notiz",
            Self::Telefonat => "Telefonat",
            Self::Besprechung => "Besprechung",
            Self::Ortstermin => "Ortstermin",
            Self::Vorfall => "Vorfall / Mangel",
            Self::EntscheidungBauherr => "Entscheidung Bauherr",
            Self::Sonstiges => "Sonstiges",
        }
    }
}
/// Wiedervorlage eines Tagebucheintrags; erscheint im Büro-Cockpit und im
/// Kalender-Abo, bis sie erledigt ist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Wiedervorlage {
    pub datum: NaiveDate,
    pub verantwortlich: Option<BenutzerId>,
    pub erledigt: bool,
}
/// Eintrag im Projekttagebuch: Vorfälle und Absprachen außerhalb der
/// Behördenkommunikation (die bleibt in der Verfahrensakte).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Tagebucheintrag {
    /// Tag des Ereignisses (Ortsdatum), nicht nach heute.
    pub datum: NaiveDate,
    pub art: Tagebuchart,
    pub titel: String,
    pub text: String,
    /// Beteiligte Personen oder Firmen als freie Angabe.
    #[serde(default)]
    pub beteiligte: String,
    /// Fotos, Protokolle oder Schreiben aus den Projektdateien.
    #[serde(default)]
    pub anhaenge: Vec<Dokumentbezug>,
    /// Optionaler Bezug auf eine Verfahrensakte des Projekts.
    #[serde(default)]
    pub verfahren: Option<Uuid>,
    #[serde(default)]
    pub wiedervorlage: Option<Wiedervorlage>,
}
/// Fachdisziplin eines Fachplaners oder Gutachters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Fachdisziplin {
    Tragwerksplanung,
    Brandschutz,
    Waermeschutz,
    Schallschutz,
    Vermessung,
    Entwaesserung,
    Baugrund,
    Haustechnik,
    Freianlagen,
    Sonstiges,
}
impl Fachdisziplin {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Tragwerksplanung => "Tragwerksplanung / Statik",
            Self::Brandschutz => "Brandschutz",
            Self::Waermeschutz => "Wärmeschutz / GEG",
            Self::Schallschutz => "Schallschutz",
            Self::Vermessung => "Vermessung / Lageplan",
            Self::Entwaesserung => "Entwässerung",
            Self::Baugrund => "Baugrund / Bodengutachten",
            Self::Haustechnik => "Haustechnik",
            Self::Freianlagen => "Freianlagen",
            Self::Sonstiges => "Sonstiges",
        }
    }
}
/// Stand einer Lieferung, die beim Fachplaner angefragt ist.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Lieferstatus {
    /// Noch nicht beauftragt oder angefragt.
    Offen,
    Angefragt,
    /// Teillieferung oder Entwurf liegt vor.
    Teilweise,
    Geliefert,
    /// Vom Büro geprüft und für die Einreichung übernommen.
    Geprueft,
    Entfallen,
}
impl Lieferstatus {
    /// Wartet das Büro noch auf den Fachplaner?
    pub fn ausstehend(self) -> bool {
        matches!(self, Self::Offen | Self::Angefragt | Self::Teilweise)
    }
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Offen => "Offen",
            Self::Angefragt => "Angefragt",
            Self::Teilweise => "Teilweise geliefert",
            Self::Geliefert => "Geliefert, Prüfung offen",
            Self::Geprueft => "Geprüft",
            Self::Entfallen => "Entfallen",
        }
    }
}
/// Einzelne angeforderte Unterlage (z. B. „Standsicherheitsnachweis“).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Lieferposition {
    pub bezeichnung: String,
    pub geliefert: bool,
}
/// Lieferliste je Fachplaner: was angefordert ist, bis wann, und in welchem
/// Stand. Ausstehende Lieferungen mit Termin erscheinen im Büro-Cockpit, im
/// Kalender-Abo und in der Einreichungsprüfung.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fachplanerlieferung {
    pub disziplin: Fachdisziplin,
    /// Büro oder Person des Fachplaners.
    pub fachplaner: String,
    #[serde(default)]
    pub kontakt: String,
    #[serde(default)]
    pub positionen: Vec<Lieferposition>,
    #[serde(default)]
    pub angefragt_am: Option<NaiveDate>,
    /// Zugesagter Liefertermin.
    #[serde(default)]
    pub faellig: Option<NaiveDate>,
    pub status: Lieferstatus,
    /// Wer im Büro die Lieferung nachhält.
    #[serde(default)]
    pub verantwortlich: Option<BenutzerId>,
    /// Gelieferte Unterlagen aus den Projektdateien.
    #[serde(default)]
    pub dokumente: Vec<Dokumentbezug>,
    /// Verfahren, für das geliefert wird; leer = alle Verfahren des Projekts.
    #[serde(default)]
    pub verfahren: Option<Uuid>,
    #[serde(default)]
    pub bemerkung: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum UnterlagenStatus {
    Entwurf,
    Pruefung,
    Freigegeben,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Unterlage {
    #[serde(default)]
    pub vorlage: Option<super::ablauf::ChecklistenQuelle>,
    pub bezeichnung: String,
    pub erforderlich: bool,
    pub dokument: Option<Dokumentbezug>,
    pub status: UnterlagenStatus,
    pub bemerkung: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Dokumentdaten {
    pub dokument: Dokumentbezug,
    pub dokumentart: String,
    pub plannummer: String,
    pub revisionsindex: String,
    pub massstab: String,
    pub verfasser: String,
    pub datum: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Raumbuch {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<crate::planaufmass::Planbindung>>")]
    pub planbindungen: Vec<crate::planaufmass::Planbindung>,
    pub bezeichnung: String,
    pub gebaeude_id: Uuid,
    pub geschoss_id: Option<Uuid>,
    pub raeume: Vec<Raum>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Raum {
    pub id: Uuid,
    pub nummer: String,
    pub bezeichnung: String,
    pub nutzung: String,
    /// Grundfläche des Raums einschließlich seiner Höhenzonen.
    pub flaeche_m2: f64,
    /// Lichte Höhe der Restfläche außerhalb der Höhenzonen.
    pub hoehe_m: Option<f64>,
    /// Teilflächen mit abweichender lichter Höhe, z. B. unter Dachschrägen.
    /// Leer = einheitliche Raumhöhe. Fehlt in älteren Ständen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<Hoehenzone>>")]
    pub hoehenzonen: Vec<Hoehenzone>,
}
/// Teilfläche eines Raums mit abweichender lichter Höhe. Reine Raumfakten:
/// Die Anrechnung (z. B. WoFlV) bestimmt das jeweilige Fachmodul anhand der
/// kleinsten lichten Höhe. Zonen werden daher an den maßgebenden Höhenlinien
/// (z. B. 1 m und 2 m) abgegrenzt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Hoehenzone {
    pub id: Uuid,
    pub bezeichnung: String,
    pub flaeche_m2: f64,
    /// Kleinste lichte Höhe in der Zone (m); `None` = noch nicht erfasst
    /// (z. B. nach Übernahme aus dem Aufmaß).
    pub lichte_hoehe_min_m: Option<f64>,
    /// Größte lichte Höhe in der Zone (m); für den Rauminhalt bei geneigter
    /// Decke. `None` = nicht erfasst.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lichte_hoehe_max_m: Option<f64>,
}
/// Höchstzahl Höhenzonen je Raum.
const MAX_ZONEN: usize = 50;
#[derive(Debug, Serialize, ts_rs::TS)]
pub struct RaumSummen {
    pub anzahl: usize,
    pub flaeche_m2: f64,
    pub volumen_m3: Option<f64>,
}
impl Raumbuch {
    pub fn summen(&self) -> CoreResult<RaumSummen> {
        let mut ids = std::collections::HashSet::new();
        let mut nummern = std::collections::HashSet::new();
        let mut flaeche = 0.;
        let mut volumen = Some(0.);
        if self.raeume.len() > 5000 {
            return Err(CoreError::InvalidInput(
                "Höchstens 5000 Räume je Raumbuch".into(),
            ));
        }
        for r in &self.raeume {
            text(&r.nummer, 120)?;
            text(&r.bezeichnung, 500)?;
            if !ids.insert(r.id)
                || !nummern.insert(r.nummer.trim().to_lowercase())
                || !r.flaeche_m2.is_finite()
                || r.flaeche_m2 < 0.
                || r.hoehe_m.is_some_and(|h| !h.is_finite() || h <= 0.)
            {
                return Err(CoreError::InvalidInput("Räume benötigen eindeutige IDs/Nummern, endliche Fläche und gegebenenfalls positive Höhe".into()));
            }
            let raum = format!("Raum {} „{}“", r.nummer.trim(), r.bezeichnung.trim());
            if r.hoehenzonen.len() > MAX_ZONEN {
                return Err(CoreError::InvalidInput(format!(
                    "{raum}: höchstens {MAX_ZONEN} Höhenzonen"
                )));
            }
            let mut zonen_ids = std::collections::HashSet::new();
            let mut zonen_flaeche = 0.;
            // Rauminhalt: Restfläche × Raumhöhe + Zonen × mittlere Höhe.
            let mut zonen_volumen = Some(0.);
            for z in &r.hoehenzonen {
                text(&z.bezeichnung, 200)?;
                let gueltig = |h: f64| h.is_finite() && h >= 0.;
                if !zonen_ids.insert(z.id)
                    || !z.flaeche_m2.is_finite()
                    || z.flaeche_m2 < 0.
                    || z.lichte_hoehe_min_m.is_some_and(|h| !gueltig(h))
                    || z.lichte_hoehe_max_m
                        .is_some_and(|m| !gueltig(m) || z.lichte_hoehe_min_m.is_some_and(|h| m < h))
                {
                    return Err(CoreError::InvalidInput(format!(
                        "{raum}, Höhenzone „{}“: eindeutige ID, Fläche ≥ 0 und lichte Höhen ≥ 0 (größte nicht unter kleinster) erforderlich",
                        z.bezeichnung
                    )));
                }
                zonen_flaeche += z.flaeche_m2;
                zonen_volumen = zonen_volumen
                    .zip(z.lichte_hoehe_min_m.zip(z.lichte_hoehe_max_m))
                    .map(|(v, (min, max))| v + z.flaeche_m2 * (min + max) / 2.);
            }
            if zonen_flaeche > r.flaeche_m2 + 0.005 {
                return Err(CoreError::InvalidInput(format!(
                    "{raum}: Höhenzonen ({zonen_flaeche:.2} m²) größer als die Raumfläche ({:.2} m²)",
                    r.flaeche_m2
                )));
            }
            let rest = (r.flaeche_m2 - zonen_flaeche).max(0.);
            // Ohne Höhenzonen wie bisher: Rauminhalt nur mit Raumhöhe.
            let rest_volumen = if !r.hoehenzonen.is_empty() && rest < 0.005 {
                Some(0.)
            } else {
                r.hoehe_m.map(|h| rest * h)
            };
            flaeche += r.flaeche_m2;
            volumen = volumen
                .zip(rest_volumen)
                .zip(zonen_volumen)
                .map(|((v, a), b)| v + a + b);
        }
        if !flaeche.is_finite() || volumen.is_some_and(|v| !v.is_finite()) {
            return Err(CoreError::InvalidInput(
                "Raumsummen außerhalb des Zahlenbereichs".into(),
            ));
        }
        Ok(RaumSummen {
            anzahl: self.raeume.len(),
            flaeche_m2: flaeche,
            volumen_m3: volumen,
        })
    }
}
async fn dokument(core: &CoreServices, p: ProjectId, d: &Dokumentbezug) -> CoreResult<()> {
    let meta = core
        .repos
        .documents
        .history(d.dokument_id)
        .await?
        .into_iter()
        .find(|v| v.version == d.version && v.project_id == p);
    if meta.is_none() {
        return Err(CoreError::InvalidInput(
            "Dokumentversion gehört nicht zu diesem Projekt".into(),
        ));
    }
    Ok(())
}
pub(super) async fn validieren(
    core: &CoreServices,
    p: &Project,
    art: ObjektArt,
    objekt: Uuid,
    daten: serde_json::Value,
    wer: &Benutzer,
) -> CoreResult<serde_json::Value> {
    let neu = match art {
        ObjektArt::Cadaufmass => {
            let a: crate::cad::projektaufmass::CadProjektaufmass =
                serde_json::from_value(daten).map_err(json_err)?;
            core.cadaufmass_pruefen(p.id, &a).await?;
            serde_json::to_value(a)
        }
        ObjektArt::Ifcmodell => {
            let a: crate::ifc::Modellakte = serde_json::from_value(daten).map_err(json_err)?;
            core.modellakte_pruefen(p.id, &a).await?;
            serde_json::to_value(a)
        }
        ObjektArt::Planaufmass => {
            let mut plan: crate::planaufmass::Planaufmass =
                serde_json::from_value(daten).map_err(json_err)?;
            plan.konturen_angleichen();
            core.planaufmass_pruefen(p.id, &plan).await?;
            // Unterlage bzw. Hintergründe müssen lesbar sein und die Seite besitzen.
            let unterlagen = plan.dokument.iter().map(|d| (d, plan.seite, None)).chain(
                plan.hintergruende
                    .iter()
                    .map(|h| (&h.dokument, h.seite, h.geschoss.as_deref())),
            );
            for (d, seite, geschoss) in unterlagen {
                if let Some(g) = geschoss {
                    // IFC-Hintergrund: Modell lesbar, Geschoss vorhanden.
                    if core.ifc_modell(p.id, d).await?.auszug.geschoss(g).is_none() {
                        return Err(CoreError::InvalidInput(
                            "Geschoss des IFC-Hintergrunds fehlt in dieser Modellversion".into(),
                        ));
                    }
                    continue;
                }
                let (version, bytes) = core.load_version(d.dokument_id, d.version).await?;
                if version.file_name.to_lowercase().ends_with(".dxf") {
                    // Die Unterlage muss als Zeichnung lesbar sein (Seite 1 prüft die Fachprüfung).
                    crate::cad::dxf::lesen(&bytes)?;
                } else if !crate::formular::editor::seiten(&bytes)?
                    .iter()
                    .any(|s| s.nummer == seite)
                {
                    return Err(CoreError::InvalidInput(
                        "PDF-Seite existiert nicht in dieser Dokumentversion".into(),
                    ));
                }
            }
            serde_json::to_value(plan)
        }
        ObjektArt::Stellplaetze
        | ObjektArt::Wohnflaechen
        | ObjektArt::Din277
        | ObjektArt::Vollgeschosse
        | ObjektArt::Ueberbaubarkeit
        | ObjektArt::Hoehen
        | ObjektArt::Freiflaechen
        | ObjektArt::Entwaesserung => {
            let plan: crate::berechnungen::FachPlan =
                serde_json::from_value(daten).map_err(json_err)?;
            if plan.modul.objekt() != art {
                return Err(CoreError::InvalidInput(
                    "Fachplan gehört zu einem anderen Modul".into(),
                ));
            }
            crate::berechnungen::berechnen(&plan)?;
            serde_json::to_value(plan)
        }
        ObjektArt::Abstandsflaechen => {
            let plan = serde_json::from_value(daten).map_err(json_err)?;
            return core.abstandsplan_validieren(p.id, plan, wer).await;
        }
        ObjektArt::Verfahren => {
            return Err(CoreError::InvalidInput(
                "Verfahren benötigen ihren eigenen Commit".into(),
            ));
        }
        ObjektArt::Projektorganisation => {
            let o: Projektorganisation = serde_json::from_value(daten).map_err(json_err)?;
            if o.kennzeichen.len() > 120
                || o.beschreibung.len() > 10000
                || o.nicht_benoetigt.len() > 100
                || o.nicht_benoetigt.iter().any(|b| b.len() > 60)
            {
                return Err(CoreError::InvalidInput(
                    "Projektorganisation: Texte sind zu lang".into(),
                ));
            }
            if let Some(id) = o.verantwortlich
                && !core.repos.benutzer.get(id).await?.aktiv
            {
                return Err(CoreError::InvalidInput(
                    "Projektverantwortlicher ist inaktiv".into(),
                ));
            }
            serde_json::to_value(o)
        }
        ObjektArt::Gebaeude => {
            let mut g: Gebaeude = serde_json::from_value(daten).map_err(json_err)?;
            core.planwerte_gebaeude(p.id, objekt, &mut g).await?;
            text(&g.bezeichnung, 500)?;
            g.grundlagen(Uuid::nil())?;
            if g.geschosse.len() > 200
                || g.grundstueck_ids.len() > 100
                || g.flurstueck_ids.len() > 1000
            {
                return Err(CoreError::InvalidInput(
                    "Zu viele Gebäudezuordnungen oder Geschosse".into(),
                ));
            }
            if g.flurstueck_ids.is_empty() || g.flurstuecke_zu_pruefen == Some(false) {
                g.flurstuecke_zu_pruefen = None;
            }
            if !g.flurstueck_ids.is_empty() {
                // Die Flurstückszuordnung ersetzt die frühere Baugrundstückszuordnung.
                g.grundstueck_ids.clear();
                let mut ids = std::collections::HashSet::new();
                if !g.flurstueck_ids.iter().all(|f| ids.insert(f)) {
                    return Err(CoreError::InvalidInput(
                        "Flurstück doppelt zugeordnet".into(),
                    ));
                }
            }
            // Nur neu hinzugefügte Zuordnungen müssen bestehen. Gespeicherte,
            // inzwischen veraltete Angaben blockieren das Speichern anderer
            // Gebäudeangaben nicht; sie werden in den Grundlagen und in GRZ/GFZ
            // als veraltet angezeigt. Flurstücke sind unabhängig von der
            // Baugrundstücksermittlung bekannt.
            let vorher = core
                .repos
                .commit
                .fachstaende(p.id, "gebaeude")
                .await?
                .into_iter()
                .find(|f| f.objekt == objekt.to_string())
                .and_then(|f| serde_json::from_value::<Gebaeude>(f.daten).ok());
            let infos = crate::grundstuecksinformationen::informationen(&p.planungsrecht);
            for f in &g.flurstueck_ids {
                if !infos.flurstuecke.iter().any(|i| &i.id == f)
                    && !vorher
                        .as_ref()
                        .is_some_and(|v| v.flurstueck_ids.contains(f))
                {
                    return Err(CoreError::InvalidInput(
                        "Gebäude verweist auf ein unbekanntes Flurstück".into(),
                    ));
                }
            }
            if g.grundstueck_ids.iter().any(|id| {
                !vorher
                    .as_ref()
                    .is_some_and(|v| v.grundstueck_ids.contains(id))
            }) {
                return Err(CoreError::InvalidInput(
                    "Gebäude über Flurstücke zuordnen; das Baugrundstück wird für die Berechnungen abgeleitet".into(),
                ));
            }
            let mut ids = std::collections::HashSet::new();
            for s in &g.geschosse {
                text(&s.bezeichnung, 500)?;
                if !ids.insert(s.id) {
                    return Err(CoreError::InvalidInput(
                        "Geschoss doppelt zugeordnet".into(),
                    ));
                }
                if s.vollgeschoss.is_some() {
                    text(&s.grundlage, 4000)?;
                }
            }
            if let Some(d) = &g.quelle {
                dokument(core, p.id, d).await?;
            }
            serde_json::to_value(g)
        }
        ObjektArt::Planliste => return core.planeintrag_pruefen(p, objekt, daten).await,
        ObjektArt::Projekttagebuch => {
            let e: Tagebucheintrag = serde_json::from_value(daten).map_err(json_err)?;
            text(&e.titel, 300)?;
            if e.text.chars().count() > 20000 || e.beteiligte.chars().count() > 2000 {
                return Err(CoreError::InvalidInput("Tagebucheintrag zu lang".into()));
            }
            if e.datum > crate::kalender::heute() {
                return Err(CoreError::InvalidInput(
                    "Das Datum eines Tagebucheintrags liegt nicht in der Zukunft; Künftiges als Wiedervorlage erfassen".into(),
                ));
            }
            if e.anhaenge.len() > 100 {
                return Err(CoreError::InvalidInput("Zu viele Anhänge".into()));
            }
            for d in &e.anhaenge {
                dokument(core, p.id, d).await?;
            }
            if let Some(v) = e.verfahren
                && core
                    .repos
                    .commit
                    .fachobjekt(p.id, "verfahren", &v.to_string())
                    .await?
                    .is_none()
            {
                return Err(CoreError::InvalidInput(
                    "Verfahrensakte gehört nicht zu diesem Projekt".into(),
                ));
            }
            if let Some(id) = e.wiedervorlage.as_ref().and_then(|w| w.verantwortlich) {
                let b = core.repos.benutzer.get(id).await?;
                if !b.aktiv {
                    return Err(CoreError::InvalidInput(
                        "Verantwortlicher Benutzer ist inaktiv".into(),
                    ));
                }
            }
            serde_json::to_value(e)
        }
        ObjektArt::Baukennwerte => {
            if objekt != p.id.0 {
                return Err(CoreError::InvalidInput(
                    "Baukennwerte gehören zum Projekt (Objektkennung = Projekt-ID)".into(),
                ));
            }
            let k: crate::baukennwerte::Baukennwerte =
                serde_json::from_value(daten).map_err(json_err)?;
            k.pruefen()?;
            serde_json::to_value(k)
        }
        ObjektArt::Fachplanerlieferung => {
            let l: Fachplanerlieferung = serde_json::from_value(daten).map_err(json_err)?;
            text(&l.fachplaner, 300)?;
            if l.kontakt.chars().count() > 1000 || l.bemerkung.chars().count() > 10000 {
                return Err(CoreError::InvalidInput("Lieferliste: Text zu lang".into()));
            }
            if l.positionen.len() > 200 || l.dokumente.len() > 200 {
                return Err(CoreError::InvalidInput("Lieferliste zu lang".into()));
            }
            for p in &l.positionen {
                text(&p.bezeichnung, 300)?;
            }
            if let (Some(a), Some(f)) = (l.angefragt_am, l.faellig)
                && f < a
            {
                return Err(CoreError::InvalidInput(
                    "Der Liefertermin liegt vor dem Anfragedatum".into(),
                ));
            }
            for d in &l.dokumente {
                dokument(core, p.id, d).await?;
            }
            if let Some(v) = l.verfahren
                && core
                    .repos
                    .commit
                    .fachobjekt(p.id, "verfahren", &v.to_string())
                    .await?
                    .is_none()
            {
                return Err(CoreError::InvalidInput(
                    "Verfahrensakte gehört nicht zu diesem Projekt".into(),
                ));
            }
            if let Some(id) = l.verantwortlich {
                let b = core.repos.benutzer.get(id).await?;
                if !b.aktiv {
                    return Err(CoreError::InvalidInput(
                        "Verantwortlicher Benutzer ist inaktiv".into(),
                    ));
                }
            }
            serde_json::to_value(l)
        }
        ObjektArt::Aufgaben => {
            let a: Aufgabe = serde_json::from_value(daten).map_err(json_err)?;
            text(&a.titel, 500)?;
            if a.beschreibung.len() > 10000 {
                return Err(CoreError::InvalidInput("Beschreibung zu lang".into()));
            }
            if let Some(id) = a.verantwortlich {
                let b = core.repos.benutzer.get(id).await?;
                if !b.aktiv {
                    return Err(CoreError::InvalidInput(
                        "Verantwortlicher Benutzer ist inaktiv".into(),
                    ));
                }
            }
            serde_json::to_value(a)
        }
        ObjektArt::Unterlagen => {
            let u: Unterlage = serde_json::from_value(daten).map_err(json_err)?;
            text(&u.bezeichnung, 500)?;
            if u.bemerkung.len() > 4000 {
                return Err(CoreError::InvalidInput("Bemerkung zu lang".into()));
            }
            if let Some(d) = &u.dokument {
                dokument(core, p.id, d).await?;
            } else if u.status != UnterlagenStatus::Entwurf {
                return Err(CoreError::InvalidInput(
                    "Unterlage ohne Dokument bleibt ein Entwurf".into(),
                ));
            }
            if u.status == UnterlagenStatus::Freigegeben {
                core.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
            }
            serde_json::to_value(u)
        }
        ObjektArt::Dokumentdaten => {
            let d: Dokumentdaten = serde_json::from_value(daten).map_err(json_err)?;
            text(&d.dokumentart, 120)?;
            for s in [&d.plannummer, &d.revisionsindex, &d.massstab, &d.verfasser] {
                if s.len() > 500 {
                    return Err(CoreError::InvalidInput("Dokumentmetadaten zu lang".into()));
                }
            }
            dokument(core, p.id, &d.dokument).await?;
            serde_json::to_value(d)
        }
        ObjektArt::Raumbuch => {
            let mut r: Raumbuch = serde_json::from_value(daten).map_err(json_err)?;
            core.planwerte_raumbuch(p.id, &mut r).await?;
            text(&r.bezeichnung, 500)?;
            r.summen()?;
            let g = core
                .repos
                .commit
                .fachstaende(p.id, "gebaeude")
                .await?
                .into_iter()
                .find(|g| g.objekt == r.gebaeude_id.to_string())
                .ok_or_else(|| {
                    CoreError::InvalidInput("Raumbuch benötigt ein Gebäude dieses Projekts".into())
                })?;
            let g: Gebaeude = serde_json::from_value(g.daten).map_err(json_err)?;
            if let Some(id) = r.geschoss_id
                && !g.geschosse.iter().any(|s| s.id == id)
            {
                return Err(CoreError::InvalidInput(
                    "Geschoss gehört nicht zum gewählten Gebäude".into(),
                ));
            }
            serde_json::to_value(r)
        }
    };
    neu.map_err(json_err)
}

impl CoreServices {
    /// Adapter vom Gebäudevertrag zu den bestehenden Flächeneingaben.
    pub(crate) async fn geschosszuordnung(
        &self,
        p: ProjectId,
        plan: &crate::baugrundstuecke::BaugrundstuecksPlan,
    ) -> CoreResult<crate::baugrundstuecke::BaugrundstuecksPlan> {
        Ok(self.gebaeude_einbinden(p, plan).await?.plan)
    }
    /// Verknüpft alle eindeutig zugeordneten Gebäude mit ihren Baugrundstücken
    /// und aktualisiert verknüpfte Flächen aus den Grundlagen.
    pub async fn gebaeude_einbinden(
        &self,
        p: ProjectId,
        plan: &crate::baugrundstuecke::BaugrundstuecksPlan,
    ) -> CoreResult<super::einbindung::GebaeudeEinbindung> {
        self.gebaeude_einbinden_fuer(p, plan, None).await
    }
    /// Eine explizit gewählte Einheit unabhängig von fremden Planbindungen
    /// prüfen. Nicht gewählte Berechnungen werden unverändert zurückgegeben.
    pub async fn gebaeude_einbinden_fuer(
        &self,
        p: ProjectId,
        plan: &crate::baugrundstuecke::BaugrundstuecksPlan,
        schluessel: Option<&str>,
    ) -> CoreResult<super::einbindung::GebaeudeEinbindung> {
        if schluessel.is_some_and(|s| !plan.berechnungen.iter().any(|b| b.schluessel == s)) {
            return Err(CoreError::InvalidInput(
                "Baugrundstück nicht im Berechnungsplan".into(),
            ));
        }
        let original = plan.clone();
        let gebaeude = self.repos.commit.fachstaende(p, "gebaeude").await?;
        let quellen = gebaeude
            .iter()
            .map(|f| {
                Ok((
                    f.objekt
                        .parse::<Uuid>()
                        .map_err(|_| CoreError::InvalidInput("Gebäude-ID ungültig".into()))?,
                    serde_json::from_value::<Gebaeude>(f.daten.clone()).map_err(json_err)?,
                ))
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let lage = super::einbindung::Lagebezug::neu(p, plan);
        for (id, g) in &quellen {
            if lage.irgendwo_eindeutig(g) && schluessel.is_none_or(|s| lage.moeglich(g, s)) {
                self.gebaeude_planquellen_pruefen(p, *id, g).await?;
            }
        }
        let super::einbindung::GebaeudeEinbindung { plan, hinweise } =
            super::einbindung::einbinden(p, plan, &quellen);
        let bezug = super::einbindung::Lagebezug::neu(p, &plan);
        let mut neu = plan;
        for b in &mut neu.berechnungen {
            if schluessel.is_some_and(|s| b.schluessel != s) {
                continue;
            }
            if let Some(a) = &mut b.eingabe.aufstellung {
                let mut bauteil_ids = std::collections::HashSet::new();
                for teil in &mut a.bauteile {
                    if let Some(id) = teil.gebaeude_id {
                        if !bauteil_ids.insert(id) {
                            return Err(CoreError::InvalidInput(
                                "Gebäudegrundfläche je Baugrundstück nur einmal übernehmen".into(),
                            ));
                        }
                        let f = gebaeude
                            .iter()
                            .find(|f| f.objekt == id.to_string())
                            .ok_or_else(|| {
                                CoreError::InvalidInput("Gebäudequelle für GRZ fehlt".into())
                            })?;
                        let g: Gebaeude =
                            serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                        self.gebaeude_planquellen_pruefen(p, id, &g).await?;
                        if !bezug.eindeutig(&g, &b.schluessel) {
                            return Err(CoreError::InvalidInput("Flächenübernahme benötigt ein Gebäude, dessen Flurstücke eindeutig zu diesem Baugrundstück gehören; bei mehreren Grundstücken Flächen getrennt manuell aufteilen".into()));
                        }
                        let basis = g.grundlagen(id)?;
                        teil.flaeche_m2 = basis.grundflaeche_m2.ok_or_else(|| {
                            CoreError::InvalidInput(
                                "Gebäudegrundfläche in den Grundlagen ergänzen".into(),
                            )
                        })?;
                        teil.typ = basis.typ.ok_or_else(|| {
                            CoreError::InvalidInput("Gebäudetyp in den Grundlagen ergänzen".into())
                        })?;
                        teil.bezeichnung = g.bezeichnung;
                    }
                }
                let mut quellgeschosse = std::collections::HashSet::new();
                for g in &mut a.geschosse {
                    if g.flaeche_aus_grundlagen == Some(true)
                        && (g.gebaeude_id.is_none()
                            || !quellgeschosse
                                .insert((g.gebaeude_id.clone(), g.geschoss_id.clone())))
                    {
                        return Err(CoreError::InvalidInput("Übernommene Geschossfläche benötigt eine eindeutige Gebäude-/Geschosszuordnung".into()));
                    }
                    if let Some(id) = &g.gebaeude_id {
                        let f = gebaeude.iter().find(|f| &f.objekt == id).ok_or_else(|| {
                            CoreError::InvalidInput(
                                "Geschoss verweist auf unbekanntes Gebäude".into(),
                            )
                        })?;
                        let geb: Gebaeude =
                            serde_json::from_value(f.daten.clone()).map_err(json_err)?;
                        if !bezug.moeglich(&geb, &b.schluessel) {
                            return Err(CoreError::InvalidInput(
                                "Geschoss gehört zu einem anderen Baugrundstück".into(),
                            ));
                        }
                        if g.flaeche_aus_grundlagen == Some(true) {
                            self.gebaeude_planquellen_pruefen(
                                p,
                                f.objekt.parse().map_err(|_| {
                                    CoreError::InvalidInput("Gebäude-ID ungültig".into())
                                })?,
                                &geb,
                            )
                            .await?;
                            if !bezug.eindeutig(&geb, &b.schluessel) {
                                return Err(CoreError::InvalidInput(
                                    "Geschossfläche für mehrere Grundstücke getrennt aufteilen"
                                        .into(),
                                ));
                            }
                            let basis = geb.grundlagen(f.objekt.parse().map_err(|_| {
                                CoreError::InvalidInput("Gebäude-ID ungültig".into())
                            })?)?;
                            let quelle = basis
                                .geschosse
                                .iter()
                                .find(|s| {
                                    g.geschoss_id.as_deref() == Some(s.id.to_string().as_str())
                                        || (s.implizit && g.geschoss_id.is_none())
                                })
                                .ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Quellgeschoss wurde geändert. Geschosse erneut übernehmen"
                                            .into(),
                                    )
                                })?;
                            g.aussenmass_m2 = quelle.flaeche_m2.ok_or_else(|| {
                                CoreError::InvalidInput(
                                    "Geschossfläche in den Grundlagen ergänzen".into(),
                                )
                            })?;
                            // Einzelgeschoss: Eine im Gebäude festgelegte Einstufung ist
                            // maßgebend, damit GFZ und Gebäude nicht auseinanderlaufen.
                            if quelle.implizit
                                && let Some(v) = quelle.vollgeschoss
                            {
                                g.vollgeschoss = v;
                            }
                        }
                        if let Some(id) = &g.geschoss_id {
                            let quelle = geb
                                .geschosse
                                .iter()
                                .find(|s| s.id.to_string() == *id)
                                .ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Geschoss gehört nicht zum gewählten Gebäude".into(),
                                    )
                                })?;
                            g.vollgeschoss = if let Some(q) = &quelle.einstufungsquelle {
                                let projekt = self.get_project(p).await?;
                                let grund = crate::grundstueckskontext::ermitteln(&projekt)?
                                    .into_iter()
                                    .find(|k| k.schluessel == b.schluessel)
                                    .map(|k| k.id);
                                self.geschosseinstufung_aus_nachweis(
                                    p,
                                    q,
                                    g.gebaeude_id.as_deref().unwrap_or("").parse().map_err(
                                        |_| CoreError::InvalidInput("Gebäude-ID ungültig".into()),
                                    )?,
                                    quelle.id,
                                    grund,
                                )
                                .await?
                                .0
                            } else {
                                quelle.vollgeschoss.ok_or_else(|| {
                                    CoreError::InvalidInput(
                                        "Vollgeschosseinstufung zuerst im Gebäude festlegen".into(),
                                    )
                                })?
                            };
                            g.bezeichnung = quelle.bezeichnung.clone();
                        } else if !geb.geschosse.is_empty() {
                            return Err(CoreError::InvalidInput(
                                "Geschoss aus dem Gebäudemodell auswählen".into(),
                            ));
                        }
                    } else if g.geschoss_id.is_some() || gebaeude.len() > 1 {
                        return Err(CoreError::InvalidInput(
                            "Geschosse den einzelnen Gebäuden zuordnen".into(),
                        ));
                    }
                }
            }
        }
        if let Some(s) = schluessel {
            for b in &mut neu.berechnungen {
                if b.schluessel != s
                    && let Some(alt) = original
                        .berechnungen
                        .iter()
                        .find(|a| a.schluessel == b.schluessel)
                {
                    *b = alt.clone();
                }
            }
        }
        let hinweise = hinweise
            .into_iter()
            .filter(|h| schluessel.is_none_or(|s| h.schluessel == s))
            .collect();
        Ok(super::einbindung::GebaeudeEinbindung {
            plan: neu,
            hinweise,
        })
    }
}
