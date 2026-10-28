//! Übernahme eines Modells in die Fachobjekte des Projekts: Gebäude mit
//! Geschossen, je Geschoss eine eigene Zeichnung (Meter) mit dem
//! IFC-Grundriss als Hintergrund und Markierungen für Räume, Außenkante und
//! Gebäudegrundfläche, auf Wunsch je Geschoss ein Raumbuch.
//!
//! Die Übernahme ist wiederholbar: Gebäude, Geschosse und Zeichnungen der
//! letzten Übernahme (in [`Modelluebernahme`] der Modellakte) werden
//! aktualisiert. Übernommene Markierungen behalten ihre IDs (Bindungen an
//! Berechnungen bleiben gültig), selbst gezeichnete bleiben unberührt.
use super::geometrie::{multipolygon, planpolygone, vereinen};
use super::{Modellakte, Modellauszug, Modellelement, Strukturelement, modellumfang};
use crate::commit::Fachstand;
use crate::planaufmass::{
    Planaufmass, Planbedeutung, Planebene, Plangeometrie, Planhintergrund, Plankalibrierung,
    Planmarkierung, Planpolygon,
};
use crate::plattform::{
    ObjektArt,
    objekte::{Dokumentbezug, Raumbuch},
};
use crate::{CoreError, CoreResult, CoreServices, benutzer::Benutzer, domain::ProjectId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

/// Abstand des Modells vom Blattrand der Zeichnung (m).
pub const RAND_M: f64 = 2.0;

/// Ergebnis der letzten Übernahme, in der Modellakte gespeichert.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Modelluebernahme {
    pub gebaeude_id: Uuid,
    /// Dokumentversion, aus der übernommen wurde.
    pub dokument: Dokumentbezug,
    pub geschosse: Vec<Geschossuebernahme>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Geschossuebernahme {
    /// GlobalId des IFC-Geschosses.
    pub global_id: String,
    /// Geschoss im Gebäude.
    pub geschoss_id: Uuid,
    /// Eigene Zeichnung (Fachobjekt `planaufmass`) des Geschosses.
    pub aufmass_id: Uuid,
    /// Raumbuch des Geschosses, sofern mit übernommen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub raumbuch_id: Option<Uuid>,
    /// Übernommene Markierungen und ihre Herkunft im Modell.
    pub markierungen: Vec<Modellbezug>,
}

/// Herkunft einer übernommenen Markierung. `global_id` ist die GlobalId des
/// Elements oder bei zusammengesetzten Umrissen `aussenkante`/`grundflaeche`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Modellbezug {
    pub markierung_id: Uuid,
    pub global_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Modelluebernahmewunsch {
    pub akte_id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    /// Vorhandenes Gebäude, in das übernommen wird; ohne Angabe das Gebäude
    /// der letzten Übernahme oder ein neues.
    #[serde(default)]
    pub gebaeude_id: Option<Uuid>,
    /// Je Geschoss ein Raumbuch aus den Räumen füllen.
    #[serde(default)]
    pub raumbuch: bool,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Modelluebernahmeergebnis {
    pub akte: Fachstand,
    pub gebaeude: Fachstand,
    pub zeichnungen: Vec<Fachstand>,
    pub raumbuecher: Vec<Fachstand>,
    pub hinweise: Vec<String>,
}

/// Modellpunkt (m, y nach Norden) → Zeichnungspunkt (m, y nach unten), so
/// dass der IFC-Hintergrund mit Lage (RAND, RAND) deckungsgleich liegt.
pub fn zeichnungspunkt(p: [f64; 2], min: [f64; 2], max: [f64; 2]) -> [f64; 2] {
    let r = |v: f64| (v * 1e4).round() / 1e4;
    [r(RAND_M + p[0] - min[0]), r(RAND_M + max[1] - p[1])]
}

fn zeichnungspolygone(ps: &[Planpolygon], min: [f64; 2], max: [f64; 2]) -> Vec<Planpolygon> {
    let t = |ring: &Vec<[f64; 2]>| ring.iter().map(|q| zeichnungspunkt(*q, min, max)).collect();
    ps.iter()
        .map(|p| Planpolygon {
            aussen: t(&p.aussen),
            innen: p.innen.iter().map(t).collect(),
        })
        .collect()
}

/// Bauteile, deren Grundriss die Außenkante eines Geschosses bildet.
pub(super) fn traegt_aussenkante(e: &Modellelement) -> bool {
    let k = e
        .klasse
        .trim_end_matches("STANDARDCASE")
        .trim_end_matches("ELEMENTEDCASE");
    match k {
        "IFCWALL" | "IFCCURTAINWALL" | "IFCCOLUMN" => true,
        "IFCSLAB" => !matches!(e.vordefiniert.as_deref(), Some("ROOF") | Some("LANDING")),
        _ => false,
    }
}

/// Äußere Umrisse (ohne Innenhöfe und Lichthöfe kleiner Bauteillücken) der
/// vereinigten Grundrisse.
pub(super) fn umriss<'a>(els: impl Iterator<Item = &'a Modellelement>) -> Vec<Planpolygon> {
    let polys: Vec<_> = els
        .filter_map(|e| e.geometrie.as_ref())
        .flat_map(|g| multipolygon(&g.grundriss).0)
        .collect();
    if polys.is_empty() {
        return Vec::new();
    }
    planpolygone(&vereinen(polys))
        .into_iter()
        .map(|p| Planpolygon {
            aussen: p.aussen,
            innen: Vec::new(),
        })
        .collect()
}

pub(super) fn geschossname(g: &Strukturelement) -> String {
    if g.name.is_empty() {
        g.langname.clone().unwrap_or_else(|| "Geschoss".into())
    } else {
        g.name.clone()
    }
}

/// Raumname: Langname („Wohnen“) vor Name („0.01“), sonst „Raum“.
fn raumname(e: &Modellelement) -> String {
    match (e.langname.as_deref(), e.name.as_str()) {
        (Some(l), n) if !l.is_empty() && !n.is_empty() && l != n => format!("{n} {l}"),
        (Some(l), _) if !l.is_empty() => l.into(),
        (_, n) if !n.is_empty() => n.into(),
        _ => "Raum".into(),
    }
}

/// Lichte Raumhöhe aus den Mengen oder Eigenschaften des Raums.
pub fn raumhoehe(e: &Modellelement) -> Option<f64> {
    let namen = ["FinishCeilingHeight", "Height", "ClearHeight"];
    namen
        .iter()
        .find_map(|n| e.mengen.iter().find(|m| m.name == *n).map(|m| m.wert))
        .or_else(|| {
            namen.iter().find_map(|n| {
                e.eigenschaften
                    .iter()
                    .find(|p| p.name == *n)
                    .and_then(|p| p.zahl)
            })
        })
        .filter(|h| *h > 0. && *h < 50.)
        .map(|h| (h * 1e3).round() / 1e3)
}

/// Geschoss, dessen Ebene der Geländeoberfläche am nächsten liegt (Träger der
/// Gebäudegrundfläche).
pub(super) fn erdgeschoss(a: &Modellauszug) -> Option<&Strukturelement> {
    a.geschosse.iter().min_by(|x, y| {
        let h = |g: &Strukturelement| g.hoehe_m.unwrap_or(0.).abs();
        h(x).total_cmp(&h(y))
    })
}

struct Bauplan<'a> {
    a: &'a Modellauszug,
    min: [f64; 2],
    max: [f64; 2],
}

impl Bauplan<'_> {
    /// Markierungen eines Geschosses: (Herkunft, Markierung ohne IDs).
    fn markierungen(
        &self,
        g: &Strukturelement,
        ebene_raum: Uuid,
        ebene_umriss: Uuid,
    ) -> Vec<(String, Planmarkierung)> {
        let a = self.a;
        let im = |e: &&Modellelement| e.geschoss.as_deref() == Some(g.global_id.as_str());
        let mut aus = Vec::new();
        let neu =
            |bezeichnung: String, bedeutung, ebene_id, polygone: Vec<Planpolygon>| Planmarkierung {
                id: Uuid::nil(),
                ebene_id,
                bezeichnung,
                bedeutung,
                geometrie: Plangeometrie::Flaeche {
                    polygone: zeichnungspolygone(&polygone, self.min, self.max),
                },
                grundstueck_id: None,
                gebaeude_id: None,
                geschoss_id: None,
                ausschnitt_id: None,
                hoehenzone_von: None,
                flurstueck_id: None,
            };
        for e in a
            .elemente
            .iter()
            .filter(im)
            .filter(|e| e.klasse == "IFCSPACE")
        {
            let Some(geo) = e.geometrie.as_ref().filter(|g| !g.grundriss.is_empty()) else {
                continue;
            };
            aus.push((
                e.global_id.clone(),
                neu(
                    raumname(e),
                    Planbedeutung::Raum,
                    ebene_raum,
                    geo.grundriss.clone(),
                ),
            ));
        }
        let kante = umriss(
            a.elemente
                .iter()
                .filter(im)
                .filter(|e| traegt_aussenkante(e)),
        );
        if !kante.is_empty() {
            aus.push((
                "aussenkante".into(),
                neu(
                    format!("Außenkante {}", geschossname(g)),
                    Planbedeutung::GeschossAussenkante,
                    ebene_umriss,
                    kante,
                ),
            ));
        }
        if erdgeschoss(a).is_some_and(|eg| eg.global_id == g.global_id) {
            // Gebäudegrundfläche: Außenkanten aller oberirdischen Geschosse.
            let oberirdisch: Vec<&str> = a
                .geschosse
                .iter()
                .filter(|s| s.hoehe_m.unwrap_or(0.) >= g.hoehe_m.unwrap_or(0.) - 1e-6)
                .map(|s| s.global_id.as_str())
                .collect();
            let flaeche = umriss(a.elemente.iter().filter(|e| {
                traegt_aussenkante(e)
                    && e.geschoss
                        .as_deref()
                        .is_some_and(|s| oberirdisch.contains(&s))
            }));
            if !flaeche.is_empty() {
                aus.push((
                    "grundflaeche".into(),
                    neu(
                        "Gebäudegrundfläche".into(),
                        Planbedeutung::Gebaeudegrundflaeche,
                        ebene_umriss,
                        flaeche,
                    ),
                ));
            }
        }
        aus
    }
}

fn json_fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(e.to_string())
}

impl CoreServices {
    pub async fn ifc_uebernehmen(
        &self,
        p: ProjectId,
        w: &Modelluebernahmewunsch,
        wer: &Benutzer,
    ) -> CoreResult<Modelluebernahmeergebnis> {
        let f = self
            .fachobjekt_lesen(p, ObjektArt::Ifcmodell, w.akte_id)
            .await?
            .ok_or_else(|| CoreError::InvalidInput("Modellakte fehlt im Projekt".into()))?;
        if f.revision != w.revision {
            return Err(CoreError::InvalidInput(
                "Modellakte wurde inzwischen geändert. Bitte neu laden.".into(),
            ));
        }
        let mut akte: Modellakte = serde_json::from_value(f.daten).map_err(json_fehler)?;
        let a = self.ifc_akte_auszug(p, &akte).await?;
        if a.geschosse.is_empty() {
            return Err(CoreError::InvalidInput(
                "Das Modell hat keine Geschosse. Zuerst in der Nacharbeit Elemente Geschossen zuordnen.".into(),
            ));
        }
        let mut hinweise = Vec::new();
        let alt = akte.uebernahme.clone();

        // 1. Gebäude mit Geschossen.
        let gebaeude_id = w
            .gebaeude_id
            .or(alt.as_ref().map(|s| s.gebaeude_id))
            .unwrap_or_else(Uuid::new_v4);
        let vorhanden = self
            .fachobjekt_lesen(p, ObjektArt::Gebaeude, gebaeude_id)
            .await?;
        if w.gebaeude_id.is_some() && vorhanden.is_none() {
            return Err(CoreError::InvalidInput("Gebäude fehlt im Projekt".into()));
        }
        let basis = vorhanden.as_ref().map_or(0, |f| f.revision);
        let mut name = a
            .gebaeude
            .first()
            .map(|b| b.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| akte.bezeichnung.clone());
        if vorhanden.is_none() {
            // Ein neues Gebäude soll in Auswahllisten unterscheidbar sein.
            let andere = self.fachobjekte(p, ObjektArt::Gebaeude).await?;
            if andere
                .iter()
                .any(|f| f.daten["bezeichnung"].as_str() == Some(name.as_str()))
                && name != akte.bezeichnung
            {
                name = format!("{name} ({})", akte.bezeichnung);
            }
        }
        let mut haus: Value = vorhanden.map(|f| f.daten).unwrap_or_else(|| {
            json!({
                "bezeichnung": name,
                "grundstueck_ids": [],
                "geschosse": [],
                "quelle": akte.dokument,
            })
        });
        let mut zuordnung: Vec<(String, Uuid)> = Vec::new();
        {
            let liste = haus
                .get_mut("geschosse")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| CoreError::InvalidInput("Gebäude ohne Geschossliste".into()))?;
            for (i, g) in a.geschosse.iter().enumerate() {
                let frueher = alt
                    .as_ref()
                    .and_then(|s| s.geschosse.iter().find(|x| x.global_id == g.global_id))
                    .map(|x| x.geschoss_id)
                    .filter(|id| {
                        liste
                            .iter()
                            .any(|v| v["id"].as_str() == Some(&id.to_string()))
                    });
                // Gleichnamiges Geschoss im vorhandenen Gebäude wiederverwenden.
                let gleichnamig = liste
                    .iter()
                    .find(|v| v["bezeichnung"].as_str() == Some(&geschossname(g)))
                    .and_then(|v| v["id"].as_str()?.parse().ok())
                    .filter(|id: &Uuid| !zuordnung.iter().any(|(_, z)| z == id));
                let id = match frueher.or(gleichnamig) {
                    Some(id) => id,
                    None => {
                        let id = Uuid::new_v4();
                        liste.push(json!({
                            "id": id,
                            "bezeichnung": geschossname(g),
                            "vollgeschoss": null,
                            "grundlage": format!("IFC-Modell „{}“", akte.bezeichnung),
                        }));
                        id
                    }
                };
                // Höhen nur ergänzen, nie überschreiben.
                if let Some(v) = liste
                    .iter_mut()
                    .find(|v| v["id"].as_str() == Some(&id.to_string()))
                {
                    let geo = v
                        .as_object_mut()
                        .expect("Geschoss ist ein Objekt")
                        .entry("geometrie")
                        .or_insert_with(|| json!({}));
                    if geo.is_null() {
                        *geo = json!({});
                    }
                    let naechstes = a.geschosse.get(i + 1).and_then(|n| n.hoehe_m);
                    if let (Some(h), Some(n)) = (g.hoehe_m, naechstes)
                        && geo.get("geschosshoehe_m").is_none_or(Value::is_null)
                    {
                        geo["geschosshoehe_m"] = json!(((n - h) * 1e3).round() / 1e3);
                    }
                }
                zuordnung.push((g.global_id.clone(), id));
            }
        }
        let gebaeude = self
            .fachobjekt_speichern(p, ObjektArt::Gebaeude, gebaeude_id, basis, haus, wer)
            .await?;

        // 2. Je Geschoss eine eigene Zeichnung.
        let (min, max) = modellumfang(&a);
        let plan = Bauplan { a: &a, min, max };
        let mut zeichnungen = Vec::new();
        let mut stand = Modelluebernahme {
            gebaeude_id,
            dokument: akte.dokument.clone(),
            geschosse: Vec::new(),
        };
        for g in &a.geschosse {
            let geschoss_id = zuordnung
                .iter()
                .find(|(gid, _)| *gid == g.global_id)
                .map(|(_, id)| *id)
                .expect("jedes Geschoss zugeordnet");
            let frueher = alt
                .as_ref()
                .and_then(|s| s.geschosse.iter().find(|x| x.global_id == g.global_id));
            let aufmass_id = frueher.map_or_else(Uuid::new_v4, |x| x.aufmass_id);
            let vorhanden = self
                .fachobjekt_lesen(p, ObjektArt::Planaufmass, aufmass_id)
                .await?;
            let basis = vorhanden.as_ref().map_or(0, |f| f.revision);
            let mut z: Planaufmass = match vorhanden {
                Some(f) => serde_json::from_value(f.daten).map_err(json_fehler)?,
                None => Planaufmass {
                    bezeichnung: format!("{} · {}", akte.bezeichnung, geschossname(g)),
                    dokument: None,
                    seite: 1,
                    kalibrierung: Some(Plankalibrierung::Einheit {
                        einheit: crate::cad::Laengeneinheit::Meter,
                    }),
                    hoehenbezug: None,
                    ebenen: Vec::new(),
                    markierungen: Vec::new(),
                    ausschnitte: Vec::new(),
                    hintergruende: Vec::new(),
                    elemente: Vec::new(),
                },
            };
            let mut ebene = |name: &str| -> Uuid {
                if let Some(e) = z.ebenen.iter().find(|e| e.bezeichnung == name) {
                    return e.id;
                }
                let id = Uuid::new_v4();
                z.ebenen.push(Planebene {
                    id,
                    bezeichnung: name.into(),
                    sichtbar: true,
                    gesperrt: false,
                });
                id
            };
            let ebene_raum = ebene("Räume (IFC)");
            let ebene_umriss = ebene("Umrisse (IFC)");
            // Hintergrund: dieses Geschoss der aktuellen Modellversion.
            z.hintergruende.retain(|h| {
                h.geschoss.is_none() || h.dokument.dokument_id != akte.dokument.dokument_id
            });
            z.hintergruende.insert(
                0,
                Planhintergrund {
                    id: Uuid::new_v4(),
                    dokument: akte.dokument.clone(),
                    seite: 1,
                    ursprung: [0., 0.],
                    lage: [RAND_M, RAND_M],
                    drehung_grad: 0.,
                    meter_pro_punkt: 1.,
                    deckkraft: 0.6,
                    sichtbar: true,
                    geschoss: Some(g.global_id.clone()),
                },
            );
            // Frühere Übernahme ersetzen, IDs je Herkunft behalten.
            let bezuege = frueher.map(|x| x.markierungen.clone()).unwrap_or_default();
            z.markierungen
                .retain(|m| !bezuege.iter().any(|b| b.markierung_id == m.id));
            let mut neu_bezuege = Vec::new();
            for (herkunft, mut m) in plan.markierungen(g, ebene_raum, ebene_umriss) {
                m.id = bezuege
                    .iter()
                    .find(|b| b.global_id == herkunft)
                    .map_or_else(Uuid::new_v4, |b| b.markierung_id);
                m.gebaeude_id = Some(gebaeude_id);
                m.geschoss_id = Some(geschoss_id);
                neu_bezuege.push(Modellbezug {
                    markierung_id: m.id,
                    global_id: herkunft,
                });
                z.markierungen.push(m);
            }
            let raeume = neu_bezuege.len().saturating_sub(
                neu_bezuege
                    .iter()
                    .filter(|b| b.global_id == "aussenkante" || b.global_id == "grundflaeche")
                    .count(),
            );
            if raeume == 0 {
                hinweise.push(format!(
                    "{}: keine Räume mit Geometrie im Modell; Räume in der Nacharbeit aus Wänden bilden oder zeichnen.",
                    geschossname(g)
                ));
            }
            let gespeichert = self
                .fachobjekt_speichern(
                    p,
                    ObjektArt::Planaufmass,
                    aufmass_id,
                    basis,
                    serde_json::to_value(&z).map_err(json_fehler)?,
                    wer,
                )
                .await?;
            stand.geschosse.push(Geschossuebernahme {
                global_id: g.global_id.clone(),
                geschoss_id,
                aufmass_id,
                raumbuch_id: frueher.and_then(|x| x.raumbuch_id),
                markierungen: neu_bezuege,
            });
            zeichnungen.push(gespeichert);
        }

        // 3. Raumbücher aus den Raummarkierungen.
        let mut raumbuecher = Vec::new();
        if w.raumbuch {
            let alle = self.fachobjekte(p, ObjektArt::Raumbuch).await?;
            for (i, gu) in stand.geschosse.iter_mut().enumerate() {
                let g = &a.geschosse[i];
                let vorhanden = alle.iter().find(|f| {
                    gu.raumbuch_id.is_some_and(|id| f.objekt == id.to_string())
                        || serde_json::from_value::<Raumbuch>(f.daten.clone()).is_ok_and(|r| {
                            r.gebaeude_id == gebaeude_id && r.geschoss_id == Some(gu.geschoss_id)
                        })
                });
                let (id, basis, r) = match vorhanden {
                    Some(f) => (
                        f.objekt.parse::<Uuid>().map_err(json_fehler)?,
                        f.revision,
                        serde_json::from_value(f.daten.clone()).map_err(json_fehler)?,
                    ),
                    None => (
                        Uuid::new_v4(),
                        0,
                        Raumbuch {
                            planbindungen: Vec::new(),
                            bezeichnung: format!("Raumbuch {}", geschossname(g)),
                            gebaeude_id,
                            geschoss_id: Some(gu.geschoss_id),
                            raeume: Vec::new(),
                        },
                    ),
                };
                let z = &zeichnungen[i];
                // Gleichnamige, von Hand erfasste Räume verknüpft das Raumbuch
                // nur nach Entscheidung; dann bleibt dieses Raumbuch unberührt.
                let mut r = match self
                    .planaufmass_raumbuch(p, gu.aufmass_id, z.revision, &[], r)
                    .await
                {
                    Ok(r) => r,
                    Err(CoreError::InvalidInput(e)) => {
                        hinweise.push(format!(
                            "Raumbuch {}: {e} Im Raumbuch „Aus Aufmaß übernehmen“ wählen.",
                            geschossname(g)
                        ));
                        continue;
                    }
                    Err(e) => return Err(e),
                };
                // Lichte Höhe aus dem Modell, wo der Raum noch keine hat.
                for b in &gu.markierungen {
                    let Some(e) = a.element(&b.global_id) else {
                        continue;
                    };
                    let raum_id = r
                        .planbindungen
                        .iter()
                        .find(|x| x.markierung_id == b.markierung_id)
                        .and_then(|x| {
                            x.ziel
                                .strip_prefix("raeume.")?
                                .strip_suffix(".flaeche_m2")?
                                .parse::<Uuid>()
                                .ok()
                        });
                    if let Some(raum) = r.raeume.iter_mut().find(|x| Some(x.id) == raum_id)
                        && raum.hoehe_m.is_none()
                    {
                        // In der Nacharbeit gebildete Räume: Höhe ihres Körpers.
                        raum.hoehe_m = raumhoehe(e).or_else(|| {
                            e.geometrie
                                .as_ref()
                                .filter(|_| e.step_id == 0)
                                .map(|g| ((g.oben_m - g.unten_m) * 1e3).round() / 1e3)
                        });
                    }
                }
                let gespeichert = self
                    .fachobjekt_speichern(
                        p,
                        ObjektArt::Raumbuch,
                        id,
                        basis,
                        serde_json::to_value(&r).map_err(json_fehler)?,
                        wer,
                    )
                    .await?;
                gu.raumbuch_id = Some(id);
                raumbuecher.push(gespeichert);
            }
        }

        akte.uebernahme = Some(stand);
        let akte = self
            .fachobjekt_speichern(
                p,
                ObjektArt::Ifcmodell,
                w.akte_id,
                w.revision,
                serde_json::to_value(&akte).map_err(json_fehler)?,
                wer,
            )
            .await?;
        Ok(Modelluebernahmeergebnis {
            akte,
            gebaeude,
            zeichnungen,
            raumbuecher,
            hinweise,
        })
    }
}
