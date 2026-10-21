//! Adapter Gebäude/Geschosse → Flächenaufstellung je Baugrundstück.
//!
//! Vertrag: Gebäude werden in den Grundlagen Flurstücken zugeordnet (stabile
//! Flurstück-IDs). Das Baugrundstück leitet dieser Adapter je Berechnung aus der
//! aktuellen Baugrundstücksermittlung ab: Liegen alle Vorhabenflurstücke eines
//! Gebäudes in derselben Einheit, wird es mit Grundfläche (GRZ) und Geschossen
//! (GFZ/Geschossigkeit) in deren Aufstellung verknüpft, sofern die Grundlagen
//! vollständig sind. Eine Vereinigungsbaulast wird über die gemeinsame Zuordnung
//! der Flurstücke in der Ermittlung abgebildet. Gebäude auf Flurstücken mehrerer
//! Einheiten werden nie automatisch aufgeteilt. Ohne Flurstückszuordnung zählt ein
//! Gebäude nur bei genau einem Baugrundstück. Ältere Zuordnungen zu
//! Baugrundstücken (`grundstueck_ids`) werden gelesen, solange keine Flurstücke
//! zugeordnet sind. Fehlende Grundlagen führen zu Hinweisen, nie zu Nullflächen.
//! Verknüpfte Einträge, deren Gebäude oder Geschoss nicht mehr eindeutig passt,
//! entfernt der Adapter mit Hinweis; ihre Werte aktualisiert `geschosszuordnung`.
use super::objekte::Gebaeude;
use crate::{
    baugrundstuecke::BaugrundstuecksPlan,
    domain::ProjectId,
    flaechen::{Bauteil, Geschoss},
    plattform::gebaeude::GebaeudeGrundlagen,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Einbindungshinweis {
    /// Schlüssel des betroffenen Baugrundstücks.
    pub schluessel: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct GebaeudeEinbindung {
    pub plan: BaugrundstuecksPlan,
    pub hinweise: Vec<Einbindungshinweis>,
}

/// Stabile Identität einer Grundstückseinheit wie im `GrundstueckKontext`.
pub fn grundstueck_id(p: ProjectId, schluessel: &str) -> Uuid {
    Uuid::new_v5(&p.0, schluessel.as_bytes())
}

/// Lage eines Gebäudes auf den aktuellen Grundstückseinheiten.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Lage {
    /// Schlüssel der Einheiten, auf denen das Gebäude steht.
    pub einheiten: BTreeSet<String>,
    /// Ohne gültige Zuordnung dem einzigen Baugrundstück zugerechnet.
    pub angenommen: bool,
    /// Zugeordnete Flurstücke außerhalb des Vorhabengrundstücks.
    pub ausserhalb: usize,
    /// Nur eine nicht mehr bestehende Baugrundstückszuordnung (Altbestand).
    pub veraltet: bool,
}

/// Bezug der Gebäude auf die Grundstückseinheiten eines Plans.
pub(crate) struct Lagebezug {
    p: ProjectId,
    flurstuecke: BTreeMap<String, String>,
    einheiten: Vec<String>,
}
impl Lagebezug {
    pub(crate) fn neu(p: ProjectId, plan: &BaugrundstuecksPlan) -> Self {
        Self {
            p,
            flurstuecke: plan.einheit_je_flurstueck(),
            einheiten: plan
                .berechnungen
                .iter()
                .map(|b| b.schluessel.clone())
                .collect(),
        }
    }
    pub(crate) fn lage(&self, g: &Gebaeude) -> Lage {
        let mut l = Lage::default();
        if !g.flurstueck_ids.is_empty() {
            for f in &g.flurstueck_ids {
                match self.flurstuecke.get(f) {
                    Some(e) => {
                        l.einheiten.insert(e.clone());
                    }
                    None => l.ausserhalb += 1,
                }
            }
        } else {
            for id in &g.grundstueck_ids {
                if let Some(e) = self
                    .einheiten
                    .iter()
                    .find(|e| grundstueck_id(self.p, e) == *id)
                {
                    l.einheiten.insert(e.clone());
                }
            }
            l.veraltet = !g.grundstueck_ids.is_empty() && l.einheiten.is_empty();
        }
        if l.einheiten.is_empty()
            && let [einzig] = self.einheiten.as_slice()
        {
            l.einheiten.insert(einzig.clone());
            l.angenommen = true;
        }
        l
    }
    /// Steht das Gebäude ausschließlich auf dieser Einheit?
    pub(crate) fn eindeutig(&self, g: &Gebaeude, schluessel: &str) -> bool {
        let l = self.lage(g);
        l.einheiten.len() == 1 && l.einheiten.contains(schluessel)
    }
    /// Wie [`Self::eindeutig`], mit der öffentlichen Grundstücks-ID.
    pub(crate) fn eindeutig_id(&self, g: &Gebaeude, id: Uuid) -> bool {
        self.einheiten
            .iter()
            .find(|e| grundstueck_id(self.p, e) == id)
            .is_some_and(|e| self.eindeutig(g, e))
    }
    /// Wie [`Self::moeglich`], mit der öffentlichen Grundstücks-ID.
    pub(crate) fn moeglich_id(&self, g: &Gebaeude, id: Uuid) -> bool {
        self.einheiten
            .iter()
            .find(|e| grundstueck_id(self.p, e) == id)
            .is_some_and(|e| self.moeglich(g, e))
    }
    /// Steht das Gebäude eindeutig auf irgendeiner Einheit dieses Plans?
    pub(crate) fn irgendwo_eindeutig(&self, g: &Gebaeude) -> bool {
        self.lage(g).einheiten.len() == 1
    }
    /// Kann das Gebäude (auch anteilig) zu dieser Einheit gehören?
    pub(crate) fn moeglich(&self, g: &Gebaeude, schluessel: &str) -> bool {
        let l = self.lage(g);
        l.einheiten.is_empty() || l.einheiten.contains(schluessel)
    }
}

pub fn einbinden(
    p: ProjectId,
    plan: &BaugrundstuecksPlan,
    gebaeude: &[(Uuid, Gebaeude)],
) -> GebaeudeEinbindung {
    let mut neu = plan.clone();
    let mut hinweise = vec![];
    let bezug = Lagebezug::neu(p, &neu);
    let grundlagen: BTreeMap<Uuid, Result<GebaeudeGrundlagen, String>> = gebaeude
        .iter()
        .map(|(id, g)| (*id, g.grundlagen(*id).map_err(|e| e.to_string())))
        .collect();
    for b in &mut neu.berechnungen {
        let schluessel = b.schluessel.clone();
        let Some(a) = &mut b.eingabe.aufstellung else {
            continue;
        };
        let mut hinweis = |t: String| {
            hinweise.push(Einbindungshinweis {
                schluessel: schluessel.clone(),
                text: t,
            })
        };
        // Verknüpfte Einträge gehören dem Adapter: nicht mehr passende entfallen.
        let passt = |id: &str| {
            gebaeude
                .iter()
                .find(|(g, _)| g.to_string() == id)
                .filter(|(_, g)| bezug.eindeutig(g, &schluessel))
        };
        a.bauteile.retain(|t| {
            let Some(id) = t.gebaeude_id else { return true };
            let ok = passt(&id.to_string()).is_some();
            if !ok {
                hinweis(format!(
                    "{}: verknüpfte Gebäudegrundfläche entfernt, weil das Gebäude nicht mehr eindeutig auf diesem Baugrundstück steht.",
                    t.bezeichnung
                ));
            }
            ok
        });
        a.geschosse.retain(|s| {
            if s.flaeche_aus_grundlagen != Some(true) {
                return true;
            }
            let ok = s.gebaeude_id.as_deref().and_then(passt).is_some_and(|(id, _)| {
                grundlagen.get(id).and_then(|r| r.as_ref().ok()).is_some_and(|basis| {
                    basis.geschosse.iter().any(|q| {
                        s.geschoss_id.as_deref() == Some(q.id.to_string().as_str())
                            || (q.implizit && s.geschoss_id.is_none())
                    })
                })
            });
            if !ok {
                hinweis(format!(
                    "{}: verknüpfte Geschossfläche entfernt, weil Gebäude oder Geschoss nicht mehr passen; Geschosse werden aus den Grundlagen neu übernommen.",
                    s.bezeichnung
                ));
            }
            ok
        });
        let manuelle_geschosse = a.geschosse.iter().any(|g| g.gebaeude_id.is_none());
        for (id, g) in gebaeude {
            let name = &g.bezeichnung;
            let lage = bezug.lage(g);
            if !(lage.einheiten.len() == 1 && lage.einheiten.contains(&schluessel)) {
                if lage.einheiten.len() > 1 && lage.einheiten.contains(&schluessel) {
                    hinweis(format!(
                        "{name}: steht auf Flurstücken mehrerer Baugrundstücke. Bei einer Vereinigungsbaulast die Flurstücke in der Baugrundstücksermittlung gemeinsam zuordnen (abweichende Zuordnung mit Grundlage); sonst die Flächen je Grundstück getrennt erfassen."
                    ));
                } else if lage.einheiten.is_empty() {
                    hinweis(format!(
                        "{name}: {} Das Gebäude wird nicht berücksichtigt.",
                        ohne_lage(g, &lage)
                    ));
                }
                continue;
            }
            if lage.angenommen {
                hinweis(format!(
                    "{name}: {} Dem einzigen Baugrundstück zugerechnet.",
                    ohne_lage(g, &lage)
                ));
            }
            if g.flurstuecke_zu_pruefen == Some(true) {
                hinweis(format!(
                    "{name}: Flurstücke aus einer früheren Baugrundstückszuordnung übernommen. Unter Gebäude/Geschosse auf die tatsächlich bebauten Flurstücke prüfen und bestätigen."
                ));
            }
            if lage.ausserhalb > 0 && !lage.angenommen {
                hinweis(format!(
                    "{name}: steht auch auf {} Flurstück(en) außerhalb des Vorhabengrundstücks. Baulast bzw. Zuordnung der Flurstücke prüfen.",
                    lage.ausserhalb
                ));
            }
            let basis = match &grundlagen[id] {
                Ok(b) => b,
                Err(e) => {
                    hinweis(format!("{name}: Grundlagen nicht auswertbar ({e})"));
                    continue;
                }
            };
            // GRZ: Gebäudegrundfläche einmal je Gebäude.
            if !a.bauteile.iter().any(|t| t.gebaeude_id == Some(*id)) {
                if a.bauteile.iter().any(|t| {
                    t.gebaeude_id.is_none()
                        && t.bezeichnung.trim().to_lowercase() == name.trim().to_lowercase()
                }) {
                    hinweis(format!(
                        "{name}: als manuelles Bauteil gleichen Namens erfasst und deshalb nicht zusätzlich übernommen."
                    ));
                } else if let (Some(flaeche), Some(typ)) = (basis.grundflaeche_m2, basis.typ) {
                    if a.bauteile.iter().any(|t| t.gebaeude_id.is_none()) {
                        hinweis(format!(
                            "{name}: Grundfläche übernommen. Manuelle Bauteile auf Doppelungen und Überlagerungen prüfen."
                        ));
                    }
                    a.bauteile.push(Bauteil {
                        gebaeude_id: Some(*id),
                        bezeichnung: name.clone(),
                        typ,
                        flaeche_m2: flaeche,
                        abweichende_anrechnung: None,
                        begruendung: String::new(),
                    });
                } else {
                    hinweis(format!(
                        "{name}: Gebäudegrundfläche und Gebäudetyp unter Gebäude/Geschosse ergänzen, damit sie in die GRZ eingeht."
                    ));
                }
            }
            // GFZ: fehlende Geschosse des Gebäudes ergänzen.
            let gid = id.to_string();
            let vorhanden: Vec<&Geschoss> = a
                .geschosse
                .iter()
                .filter(|s| s.gebaeude_id.as_deref() == Some(gid.as_str()))
                .collect();
            let fehlend: Vec<_> = basis
                .geschosse
                .iter()
                .filter(|s| {
                    !vorhanden.iter().any(|v| {
                        v.geschoss_id.as_deref() == Some(s.id.to_string().as_str())
                            || (s.implizit && v.geschoss_id.is_none())
                    })
                })
                .collect();
            if fehlend.is_empty() {
                continue;
            }
            if manuelle_geschosse {
                hinweis(format!(
                    "{name}: Geschosse nicht übernommen, weil manuell erfasste Geschosse keinem Gebäude zugeordnet sind."
                ));
                continue;
            }
            if vorhanden.iter().any(|v| v.geschoss_id.is_none()) {
                hinweis(format!(
                    "{name}: bereits als Einzelgeschoss bzw. manuelle Ebene erfasst. Zuordnung der Geschosse prüfen."
                ));
                continue;
            }
            let offen: Vec<String> = fehlend
                .iter()
                .filter_map(|s| match (s.flaeche_m2, s.vollgeschoss, s.implizit) {
                    (None, _, _) => Some(format!("{}: Geschossfläche", s.bezeichnung)),
                    (_, None, true) => Some("Einzelgeschoss: Vollgeschosseinstufung (unter Gebäude/Geschosse festlegen)".into()),
                    (_, None, false) => Some(format!("{}: Vollgeschosseinstufung", s.bezeichnung)),
                    _ => None,
                })
                .collect();
            if !offen.is_empty() {
                hinweis(format!(
                    "{name}: Geschosse noch nicht in der GFZ, es fehlt: {}.",
                    offen.join("; ")
                ));
                continue;
            }
            for s in fehlend {
                a.geschosse.push(Geschoss {
                    flaeche_aus_grundlagen: Some(true),
                    teilflaechen_geprueft: Some(false),
                    gebaeude_id: Some(gid.clone()),
                    geschoss_id: (!s.implizit).then(|| s.id.to_string()),
                    bezeichnung: s.bezeichnung.clone(),
                    vollgeschoss: s.vollgeschoss.unwrap_or_default(),
                    aussenmass_m2: s.flaeche_m2.unwrap_or_default(),
                    aufenthalt_m2: 0.,
                    waende_m2: 0.,
                    erschliessung_m2: 0.,
                });
            }
        }
    }
    GebaeudeEinbindung {
        plan: neu,
        hinweise,
    }
}

/// Warum ein Gebäude keiner Einheit eindeutig zugeordnet ist.
fn ohne_lage(g: &Gebaeude, l: &Lage) -> &'static str {
    if l.veraltet {
        "Die frühere Baugrundstückszuordnung besteht nicht mehr; unter Gebäude/Geschosse die Flurstücke zuordnen."
    } else if !g.flurstueck_ids.is_empty() {
        "Steht nur auf Flurstücken außerhalb des Vorhabengrundstücks oder auf nicht mehr erfassten Flurstücken."
    } else {
        "Keinem Flurstück zugeordnet; Zuordnung unter Gebäude/Geschosse ergänzen."
    }
}

impl crate::CoreServices {
    /// Altbestand: Gebäude, die noch einer nicht mehr bestehenden Einheit
    /// (`alt`, auch `projektflaeche`) zugeordnet sind, erhalten ausdrücklich die
    /// Flurstücke der aktuellen Einheit `neu` als Flurstückszuordnung. Gebäude
    /// mit Flurstückszuordnung bleiben unberührt. Jedes geänderte Gebäude erhält
    /// eine neue Revision; die Flurstücke sind danach unter Gebäude/Geschosse zu
    /// prüfen. Liefert die Anzahl geänderter Gebäude.
    pub async fn gebaeudezuordnung_uebertragen(
        &self,
        p: ProjectId,
        alt: &str,
        neu: &str,
        wer: &crate::benutzer::Benutzer,
    ) -> crate::CoreResult<usize> {
        let fehler = |s: &str| crate::CoreError::InvalidInput(s.into());
        let projekt = self.get_project(p).await?;
        let aktuell = crate::grundstueckskontext::ermitteln(&projekt)?;
        let Some(ziel) = aktuell.iter().find(|g| g.schluessel == neu) else {
            return Err(fehler("Ziel ist kein aktuelles Baugrundstück"));
        };
        if aktuell.iter().any(|g| g.schluessel == alt) {
            return Err(fehler(
                "Das bisherige Baugrundstück besteht noch; Zuordnung unter Gebäude/Geschosse ändern",
            ));
        }
        let alt_id = grundstueck_id(p, alt);
        let mut anzahl = 0;
        for f in self.repos.commit.fachstaende(p, "gebaeude").await? {
            let mut g: Gebaeude =
                serde_json::from_value(f.daten).map_err(|e| fehler(&e.to_string()))?;
            // Nur Altbestand: Flurstückszuordnungen bleiben bei Umgruppierungen gültig.
            if !g.flurstueck_ids.is_empty() || !g.grundstueck_ids.contains(&alt_id) {
                continue;
            }
            g.grundstueck_ids.clear();
            g.flurstueck_ids = ziel.flurstueck_ids.clone();
            g.flurstuecke_zu_pruefen = Some(true);
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Gebäude-ID ungültig"))?;
            self.fachobjekt_speichern(
                p,
                super::ObjektArt::Gebaeude,
                id,
                f.revision,
                serde_json::to_value(g).map_err(|e| fehler(&e.to_string()))?,
                wer,
            )
            .await?;
            anzahl += 1;
        }
        Ok(anzahl)
    }
    /// Projekt, dessen gespeicherte Flächenaufstellungen um die eindeutig
    /// zugeordneten Gebäude ergänzt und aus deren Grundlagen aktualisiert sind.
    /// Gemeinsame Grundlage für Formulare und XBau, damit diese dieselben
    /// GRZ-/GFZ-Werte wie Berechnung und Nachweis verwenden. Kann der Abgleich
    /// nicht erfolgen, bleibt der gespeicherte Stand erhalten und ein Hinweis
    /// nennt den Grund; er wird nie still übergangen.
    pub async fn projekt_mit_gebaeuden(
        &self,
        p: &crate::domain::Project,
    ) -> crate::CoreResult<(crate::domain::Project, Vec<String>)> {
        let mut wirksam = p.clone();
        if !crate::grundstuecksinformationen::quellen_aktuell(&p.planungsrecht)? {
            return Ok((wirksam, vec![]));
        }
        let Some(plan) = crate::grundstuecksinformationen::plan(&p.planungsrecht)? else {
            return Ok((wirksam, vec![]));
        };
        let mehrere = plan.berechnungen.len() > 1;
        let e = match self.gebaeude_einbinden(p.id, &plan).await {
            Ok(e) => e,
            Err(e) => {
                return Ok((
                    wirksam,
                    vec![format!(
                        "Gebäudeflächen konnten nicht in die GRZ/GFZ übernommen werden; verwendet wird der gespeicherte Berechnungsstand: {e}"
                    )],
                ));
            }
        };
        let r = &mut wirksam.planungsrecht;
        if let Some(erm) = &mut r.baugrundstuecksermittlung {
            erm.berechnungen = e.plan.berechnungen;
        } else if let Some(b) = &mut r.baugrundstuecke {
            b.berechnungen = e.plan.berechnungen;
        }
        let hinweise = e
            .hinweise
            .into_iter()
            .map(|h| {
                if mehrere {
                    let name = plan
                        .gruppen(r.mit_bebauungsplan())
                        .ok()
                        .and_then(|g| g.into_iter().find(|g| g.schluessel == h.schluessel))
                        .map(|g| g.bezeichnung)
                        .unwrap_or_default();
                    format!("{name}: {}", h.text)
                } else {
                    h.text
                }
            })
            .collect();
        Ok((wirksam, hinweise))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baugrundstuecke::AbweichendeZuordnung;
    use crate::flaechen::FlaechenAufstellung;
    use serde_json::json;
    /// Musterplan: Flurstücke a+b bilden ein Baugrundstück, c ein zweites.
    fn plan_ohne_flaechen() -> BaugrundstuecksPlan {
        let mut plan = crate::baugrundstuecke::musterplan();
        for b in &mut plan.berechnungen {
            let a = b.eingabe.aufstellung.as_mut().unwrap();
            a.bauteile.clear();
            a.geschosse.clear();
        }
        plan
    }
    fn einheit(plan: &BaugrundstuecksPlan, f: &str) -> String {
        plan.einheit_je_flurstueck()[f].clone()
    }
    fn aufstellung(plan: &BaugrundstuecksPlan, s: &str) -> FlaechenAufstellung {
        plan.berechnungen
            .iter()
            .find(|b| b.schluessel == s)
            .unwrap()
            .eingabe
            .aufstellung
            .clone()
            .unwrap()
    }
    fn haus(flurstuecke: &[&str], geschosse: serde_json::Value) -> Gebaeude {
        serde_json::from_value(json!({"bezeichnung":"Wohnhaus","flurstueck_ids":flurstuecke,"grundstueck_ids":[],"quelle":null,"geometrie":{"grundflaeche_m2":120,"typ":"hauptanlage"},"geschosse":geschosse})).unwrap()
    }
    fn text(r: &GebaeudeEinbindung, t: &str) -> bool {
        r.hinweise.iter().any(|h| h.text.contains(t))
    }
    #[test]
    fn gebaeude_folgen_ihren_flurstuecken_je_grundstueck() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let eg = Uuid::new_v4();
        let h = haus(
            &["a", "b"],
            json!([{"id":eg,"bezeichnung":"EG","vollgeschoss":true,"grundlage":"x","geometrie":{"flaeche_m2":120}}]),
        );
        let quellen = [(Uuid::new_v4(), h)];
        let r = einbinden(p, &plan, &quellen);
        let a0 = aufstellung(&r.plan, &einheit(&plan, "a"));
        assert_eq!(a0.bauteile.len(), 1);
        assert_eq!(a0.bauteile[0].flaeche_m2, 120.);
        assert_eq!(a0.geschosse[0].geschoss_id, Some(eg.to_string()));
        let a1 = aufstellung(&r.plan, &einheit(&plan, "c"));
        assert!(a1.bauteile.is_empty() && a1.geschosse.is_empty());
        // Wiederholte Einbindung ist idempotent.
        assert_eq!(einbinden(p, &r.plan, &quellen).plan, r.plan);
    }
    #[test]
    fn vereinigungsbaulast_ueber_gemeinsame_zuordnung() {
        let p = ProjectId::new();
        let mut plan = plan_ohne_flaechen();
        let quellen = [(Uuid::new_v4(), haus(&["a", "c"], json!([])))];
        let r = einbinden(p, &plan, &quellen);
        assert!(text(&r, "mehrerer Baugrundstücke"));
        assert!(text(&r, "Vereinigungsbaulast"));
        // Mit gemeinsamer Zuordnung bilden a, b und c ein Baugrundstück.
        for f in &mut plan.flurstuecke {
            f.abweichende_zuordnung = Some(AbweichendeZuordnung {
                kennung: "VB-1".into(),
                grundlage: "Vereinigungsbaulast Nr. 12/2026".into(),
            });
        }
        let s = plan.gruppen(true).unwrap()[0].schluessel.clone();
        plan.berechnungen.truncate(1);
        plan.berechnungen[0].schluessel = s.clone();
        let r = einbinden(p, &plan, &quellen);
        assert_eq!(aufstellung(&r.plan, &s).bauteile.len(), 1);
        assert!(!text(&r, "mehrerer Baugrundstücke"));
    }
    #[test]
    fn unvollstaendige_unbekannte_und_veraltete_lagen_nur_als_hinweis() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let offen = haus(
            &["a"],
            json!([{"id":Uuid::new_v4(),"bezeichnung":"DG","vollgeschoss":null,"grundlage":"","geometrie":{"flaeche_m2":80}}]),
        );
        let ohne = haus(&[], json!([]));
        let aussen = haus(&["nachbar"], json!([]));
        let mut alt = haus(&[], json!([]));
        alt.grundstueck_ids = vec![Uuid::new_v4()];
        let r = einbinden(
            p,
            &plan,
            &[
                (Uuid::new_v4(), offen),
                (Uuid::new_v4(), ohne),
                (Uuid::new_v4(), aussen),
                (Uuid::new_v4(), alt),
            ],
        );
        let a0 = aufstellung(&r.plan, &einheit(&plan, "a"));
        assert_eq!(a0.bauteile.len(), 1, "Grundfläche ist bekannt");
        assert!(
            a0.geschosse.is_empty(),
            "Einstufung offen: keine GFZ-Übernahme"
        );
        assert!(text(&r, "Vollgeschosseinstufung"));
        assert!(text(&r, "Keinem Flurstück zugeordnet"));
        assert!(text(&r, "außerhalb des Vorhabengrundstücks"));
        assert!(text(&r, "frühere Baugrundstückszuordnung"));
        let a1 = aufstellung(&r.plan, &einheit(&plan, "c"));
        assert!(a1.bauteile.is_empty());
    }
    #[test]
    fn altbestand_mit_baugrundstueckszuordnung_bleibt_lesbar() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let s = einheit(&plan, "c");
        let mut alt = haus(&[], json!([]));
        alt.grundstueck_ids = vec![grundstueck_id(p, &s)];
        let r = einbinden(p, &plan, &[(Uuid::new_v4(), alt)]);
        assert_eq!(aufstellung(&r.plan, &s).bauteile.len(), 1);
    }
    #[test]
    fn einziges_grundstueck_und_manuelle_bauteile() {
        let p = ProjectId::new();
        let mut plan = plan_ohne_flaechen();
        plan.flurstuecke.retain(|f| f.id != "c");
        let s = einheit(&plan, "a");
        plan.berechnungen.retain(|b| b.schluessel == s);
        let a = plan.berechnungen[0].eingabe.aufstellung.as_mut().unwrap();
        a.bauteile.push(Bauteil {
            gebaeude_id: None,
            bezeichnung: "wohnhaus ".into(),
            typ: crate::flaechen::Bauteiltyp::Hauptanlage,
            flaeche_m2: 100.,
            abweichende_anrechnung: None,
            begruendung: String::new(),
        });
        // Zuordnung zur früheren Projektfläche: bei einem Baugrundstück zugerechnet.
        let garage: Gebaeude = serde_json::from_value(json!({"bezeichnung":"Garage","grundstueck_ids":[grundstueck_id(p, "projektflaeche")],"quelle":null,"geometrie":{"grundflaeche_m2":18,"typ":"garage"},"geschosse":[]})).unwrap();
        let r = einbinden(
            p,
            &plan,
            &[
                (Uuid::new_v4(), haus(&[], json!([]))),
                (Uuid::new_v4(), garage),
            ],
        );
        let a = aufstellung(&r.plan, &s);
        // Gleichnamiges manuelles Bauteil wird nicht verdoppelt, die Garage wird ergänzt.
        assert_eq!(a.bauteile.len(), 2);
        assert_eq!(a.bauteile[1].flaeche_m2, 18.);
        // Einzelgeschoss ohne geprüfte Einstufung bleibt offen.
        assert!(a.geschosse.is_empty());
        assert!(text(&r, "Dem einzigen Baugrundstück zugerechnet"));
        assert!(text(&r, "frühere Baugrundstückszuordnung"));
    }
    #[test]
    fn nicht_mehr_passende_verknuepfungen_entfallen_mit_hinweis() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let (id, eg) = (Uuid::new_v4(), Uuid::new_v4());
        let geschoss = json!([{"id":eg,"bezeichnung":"EG","vollgeschoss":true,"grundlage":"x","geometrie":{"flaeche_m2":120}}]);
        let r = einbinden(p, &plan, &[(id, haus(&["a"], geschoss))]);
        // Gebäude auf ein Flurstück des anderen Grundstücks verlegt, Geschoss entfernt.
        let r = einbinden(p, &r.plan, &[(id, haus(&["c"], json!([])))]);
        let a0 = aufstellung(&r.plan, &einheit(&plan, "a"));
        assert!(a0.bauteile.is_empty() && a0.geschosse.is_empty());
        assert!(text(&r, "verknüpfte Gebäudegrundfläche entfernt"));
        assert!(text(&r, "verknüpfte Geschossfläche entfernt"));
        assert_eq!(aufstellung(&r.plan, &einheit(&plan, "c")).bauteile.len(), 1);
    }
    #[test]
    fn eingestuftes_einzelgeschoss_geht_in_die_gfz() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let mut garage = haus(&["c"], json!([]));
        let geo = garage.geometrie.as_mut().unwrap();
        geo.einzelgeschoss_vollgeschoss = Some(false);
        assert!(
            garage.grundlagen(Uuid::new_v4()).is_err(),
            "Grundlage fehlt"
        );
        let geo = garage.geometrie.as_mut().unwrap();
        geo.einzelgeschoss_grundlage = Some("Garage, kein Aufenthaltsraum".into());
        let r = einbinden(p, &plan, &[(Uuid::new_v4(), garage)]);
        let a = aufstellung(&r.plan, &einheit(&plan, "c"));
        assert_eq!(a.geschosse.len(), 1);
        assert!(!a.geschosse[0].vollgeschoss);
        assert_eq!(a.geschosse[0].geschoss_id, None);
    }
    #[test]
    fn uebernommene_flurstuecke_sind_zu_pruefen() {
        let p = ProjectId::new();
        let plan = plan_ohne_flaechen();
        let mut h = haus(&["a", "b"], json!([]));
        h.flurstuecke_zu_pruefen = Some(true);
        let r = einbinden(p, &plan, &[(Uuid::new_v4(), h)]);
        assert!(text(&r, "früheren Baugrundstückszuordnung übernommen"));
        assert_eq!(aufstellung(&r.plan, &einheit(&plan, "a")).bauteile.len(), 1);
    }
}
