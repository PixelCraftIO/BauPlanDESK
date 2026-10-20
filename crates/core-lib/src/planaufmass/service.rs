use super::*;
use crate::{
    CoreServices,
    berechnungen::{BerechnungsArt, FachPlan},
    domain::ProjectId,
    plattform::{ObjektArt, objekte::Gebaeude},
};

#[derive(Debug, Serialize, ts_rs::TS)]
pub struct Planziel {
    pub ziel: String,
    pub titel: String,
    pub groesse: Planmessgroesse,
}
pub fn planziele(plan: &FachPlan) -> CoreResult<Vec<Planziel>> {
    let vorlage = crate::berechnungen::vorlagen()?
        .into_iter()
        .find(|v| v.plan.modul == plan.modul)
        .ok_or_else(|| fehler("Berechnungsvorlage fehlt"))?;
    let mut ziele = Vec::new();
    for t in &vorlage.tabellen {
        if let Some(zeilen) = plan.eingaben.get(&t.pfad).and_then(|v| v.as_array()) {
            for z in zeilen {
                let Some(id) = z.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let name = ["bezeichnung", "raum", "id"]
                    .iter()
                    .find_map(|k| z.get(k).and_then(|v| v.as_str()))
                    .unwrap_or(id);
                for f in &t.felder {
                    if let Some(groesse) = zielgroesse(plan.modul, &t.pfad, &f.name) {
                        ziele.push(Planziel {
                            ziel: format!("{}.{}.{}", t.pfad, id, f.name),
                            titel: format!("{} · {} · {}", t.titel, name, f.titel),
                            groesse,
                        });
                    }
                }
            }
        }
    }
    Ok(ziele)
}

/// Für eine Bindung maßgebender Stand eines Aufmaßes: Unterlage, Seite,
/// Kalibrierung, Höhenbezug, Aufmaßbezeichnung und die gebundene Markierung
/// ohne ihre Zeichenebene. Ebenen, Sichtbarkeit und andere Markierungen gehören
/// nicht dazu. Grundlage für Aktualitätsprüfung und Nachweis-Fingerprints.
pub(crate) fn bindungsstand(a: &Planaufmass, markierung: Uuid) -> serde_json::Value {
    let m = a.markierungen.iter().find(|m| m.id == markierung).map(|m| {
        serde_json::json!({
            "id": m.id,
            "bezeichnung": m.bezeichnung,
            "bedeutung": m.bedeutung,
            "geometrie": m.geometrie,
            "grundstueck_id": m.grundstueck_id,
            "gebaeude_id": m.gebaeude_id,
            "geschoss_id": m.geschoss_id,
        })
    });
    serde_json::json!({
        "bezeichnung": a.bezeichnung,
        "dokument": a.dokument,
        "seite": a.seite,
        "kalibrierung": a.kalibrierung,
        "hoehenbezug": a.hoehenbezug,
        "markierung": m,
    })
}

impl CoreServices {
    pub(crate) async fn planbindung_aufloesen(
        &self,
        pid: ProjectId,
        b: &Planbindung,
    ) -> CoreResult<(Planaufmass, Planmarkierung, f64)> {
        let f = self
            .fachobjekte(pid, ObjektArt::Planaufmass)
            .await?
            .into_iter()
            .find(|f| f.objekt == b.aufmass_id.to_string())
            .ok_or_else(|| fehler("Planaufmaß fehlt im Projekt"))?;
        let geaendert = || {
            fehler("Planaufmaß geändert oder Datenversion nicht unterstützt. Quelle erneut prüfen.")
        };
        if f.schema_version != 1 || b.revision < 1 || f.revision < b.revision {
            return Err(geaendert());
        }
        let a: Planaufmass = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
        // Spätere Revisionen sind nur maßgebend, wenn sich der für diese Bindung
        // maßgebende Stand geändert hat; Ebenen und andere Markierungen nicht.
        if f.revision != b.revision {
            let alt = self
                .fachverlauf(pid, ObjektArt::Planaufmass, b.aufmass_id)
                .await?
                .into_iter()
                .find(|s| s.revision == b.revision && s.schema_version == 1)
                .ok_or_else(geaendert)?;
            let alt: Planaufmass =
                serde_json::from_value(alt.daten).map_err(|e| fehler(e.to_string()))?;
            if bindungsstand(&alt, b.markierung_id) != bindungsstand(&a, b.markierung_id) {
                return Err(geaendert());
            }
        }
        let aktuell = self
            .document_history(a.dokument.dokument_id)
            .await?
            .into_iter()
            .filter(|d| d.project_id == pid)
            .map(|d| d.version)
            .max();
        if aktuell != Some(a.dokument.version) {
            return Err(fehler(
                "PDF-Unterlage hat eine neuere Version. Aufmaß auf neuer Unterlage prüfen.",
            ));
        }
        let m = a
            .markierungen
            .iter()
            .find(|m| m.id == b.markierung_id)
            .ok_or_else(|| fehler("Planmarkierung fehlt"))?
            .clone();
        // Nur die gebundene Markierung prüfen: eine andere, nicht mehr passende
        // Markierung desselben Aufmaßes blockiert diese Übernahme nicht.
        let mut einzeln = a.clone();
        einzeln.markierungen.retain(|x| x.id == m.id);
        let werte = self.planaufmass_pruefen(pid, &einzeln).await?;
        if m.bedeutung == Planbedeutung::Unbestimmt {
            return Err(fehler("Planmarkierung zuerst fachlich zuordnen"));
        }
        let wert = werte
            .iter()
            .find(|w| w.id == m.id)
            .ok_or_else(|| fehler("Messwert fehlt"))?
            .wert(b.groesse)?;
        Ok((a, m, wert))
    }
    /// Reine Entwurfsübernahme; Speichern erfolgt anschließend über die bestehende Fachgrenze.
    pub async fn planaufmass_einsetzen(
        &self,
        pid: ProjectId,
        mut plan: FachPlan,
    ) -> CoreResult<FachPlan> {
        self.planwerte_einsetzen(pid, &mut plan).await?;
        plan.regelkontext.anwendbarkeit_bestaetigt = false;
        Ok(plan)
    }
    pub async fn planaufmass_pruefen(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Vec<Planmesswert>> {
        let werte = plan.messen()?;
        let dokument = self
            .document_history(plan.dokument.dokument_id)
            .await?
            .into_iter()
            .find(|d| d.project_id == pid && d.version == plan.dokument.version)
            .ok_or_else(|| fehler("PDF-Version gehört nicht zum Projekt"))?;
        if !dokument.file_name.to_lowercase().ends_with(".pdf") {
            return Err(fehler("Aufmaß benötigt eine PDF-Unterlage"));
        }
        let projekt = self.get_project(pid).await?;
        let grund = if plan.markierungen.iter().any(|m| m.grundstueck_id.is_some()) {
            crate::grundstueckskontext::ermitteln(&projekt)?
        } else {
            vec![]
        };
        let gebaeude = self.fachobjekte(pid, ObjektArt::Gebaeude).await?;
        for m in &plan.markierungen {
            // Die Meldung nennt die Markierung, damit sie im Editor auffindbar ist.
            let name = |t: &str| fehler(format!("Markierung „{}“: {t}", m.bezeichnung));
            if let Some(id) = m.grundstueck_id {
                if !grund.iter().any(|g| g.id == id) {
                    return Err(name(
                        "Baugrundstückszuordnung ist nicht mehr aktuell; Baugrundstück neu wählen",
                    ));
                }
            }
            if let Some(id) = m.gebaeude_id {
                let g = gebaeude
                    .iter()
                    .find(|g| g.objekt == id.to_string())
                    .ok_or_else(|| name("Gebäude ist nicht mehr erfasst; Gebäude neu wählen"))?;
                let g: Gebaeude =
                    serde_json::from_value(g.daten.clone()).map_err(|e| fehler(e.to_string()))?;
                if let Some(s) = m.geschoss_id {
                    if !g.geschosse.iter().any(|g| g.id == s) {
                        return Err(name(
                            "Geschoss ist im Gebäude nicht mehr erfasst; Geschoss neu wählen",
                        ));
                    }
                }
            }
        }
        Ok(werte)
    }

    pub(crate) async fn planwerte_einsetzen(
        &self,
        pid: ProjectId,
        plan: &mut FachPlan,
    ) -> CoreResult<()> {
        if plan.planbindungen.is_empty() {
            return Ok(());
        }
        if plan.gebaeudedaten_uebernehmen == Some(true) {
            return Err(fehler(
                "Gebäudeübernahme und PDF-Aufmaß sind konkurrierende Eingabequellen. Gebäudeübernahme zuerst ausdrücklich deaktivieren.",
            ));
        }
        if plan.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Planverknüpfungen"));
        }
        let mut ziele = std::collections::HashSet::new();
        for b in plan.planbindungen.clone() {
            if !ziele.insert(b.ziel.clone()) || plan.uebernahmen.iter().any(|u| u.ziel == b.ziel) {
                return Err(fehler("Eingabefeld hat mehrere Quellen"));
            }
            let (a, m, wert) = self.planbindung_aufloesen(pid, &b).await?;
            if m.grundstueck_id != plan.grundstueck_id || m.gebaeude_id != plan.gebaeude_id {
                return Err(fehler(
                    "Planmarkierung und Berechnung benötigen dieselbe Grundstücks-/Gebäudezuordnung",
                ));
            }
            einsetzen(plan, &b, &m, &a, wert)?;
        }
        Ok(())
    }
}

/// Bewusst begrenzte Eingangsgrößen: Rohaufmaß ist keine rechtlich angerechnete Fläche.
pub fn zielgroesse(modul: BerechnungsArt, tabelle: &str, feld: &str) -> Option<Planmessgroesse> {
    use BerechnungsArt::*;
    use Planmessgroesse::*;
    match (modul, tabelle, feld) {
        (Din277 | Wohnflaechen, "positionen", "aufmass_m2")
        | (Freiflaechen | Entwaesserung, "positionen", "flaeche_m2")
        | (Din277, "volumen", "aufmass_m2")
        | (Wohnflaechen, "abzuege", "flaeche_m2")
        | (Vollgeschosse, "geschosse", "grundflaeche_m2" | "unterliegend_m2")
        | (Vollgeschosse, "teilflaechen", "flaeche_m2") => Some(Flaeche),
        (Wohnflaechen, "positionen", "lichte_hoehe_m")
        | (Din277, "volumen", "hoehe_m")
        | (Vollgeschosse, "teilflaechen", "lichte_hoehe_m")
        | (Hoehen, "punkte", "absolut_m" | "bezug_m") => Some(Hoehe),
        (Hoehen, "daecher", "horizontale_m") => Some(Horizontale),
        _ => None,
    }
}
fn einsetzen(
    plan: &mut FachPlan,
    b: &Planbindung,
    m: &Planmarkierung,
    a: &Planaufmass,
    wert: f64,
) -> CoreResult<()> {
    let teile: Vec<_> = b.ziel.split('.').collect();
    if teile.len() != 3 || zielgroesse(plan.modul, teile[0], teile[2]) != Some(b.groesse) {
        return Err(fehler("Dieses Feld unterstützt diese Planmessgröße nicht"));
    }
    // Absolute Höhen dürfen nicht aus einer bloßen Streckenlänge entstehen.
    if plan.modul == BerechnungsArt::Hoehen && teile[0] == "punkte" {
        if !matches!(m.geometrie, Plangeometrie::Hoehenpunkt { .. }) || a.hoehenbezug.is_none() {
            return Err(fehler(
                "Absolute Höhe benötigt einen Höhenpunkt mit benanntem Höhenbezug",
            ));
        }
        let system = plan
            .eingaben
            .get("hoehenbezug")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if Some(system) != a.hoehenbezug.as_ref().map(|h| h.system.as_str()) {
            return Err(fehler(
                "Höhensysteme von Aufmaß und Berechnung stimmen nicht überein",
            ));
        }
    } else if b.groesse == Planmessgroesse::Hoehe
        && !matches!(m.geometrie, Plangeometrie::Strecke { .. })
    {
        return Err(fehler(
            "Relative Höhe benötigt eine gemessene Höhendifferenz",
        ));
    }
    let zeile = plan
        .eingaben
        .get_mut(teile[0])
        .and_then(|v| v.as_array_mut())
        .and_then(|v| {
            v.iter_mut()
                .find(|v| v.get("id").and_then(|v| v.as_str()) == Some(teile[1]))
        })
        .ok_or_else(|| {
            fehler("Zielposition fehlt; Zuordnung wird nicht auf eine andere Position verschoben")
        })?;
    let obj = zeile
        .as_object_mut()
        .ok_or_else(|| fehler("Zielposition ungültig"))?;
    if plan.modul == BerechnungsArt::Din277 && teile[0] == "positionen" {
        let art = match m.bedeutung {
            Planbedeutung::Nuf => "nuf",
            Planbedeutung::Tf => "tf",
            Planbedeutung::Vf => "vf",
            Planbedeutung::Kgf => "kgf",
            _ => {
                return Err(fehler(
                    "DIN-277-Aufmaß zuerst als NUF, TF, VF oder KGF klassifizieren",
                ));
            }
        };
        obj.insert("art".into(), serde_json::json!(art));
    }
    if !obj.contains_key(teile[2]) {
        return Err(fehler("Zielfeld fehlt"));
    }
    obj.insert(teile[2].into(), serde_json::json!(wert));
    if teile[2] == "aufmass_m2" {
        obj.insert("form".into(), serde_json::json!("aufmass"));
    }
    if obj.contains_key("quelle") {
        obj.insert(
            "quelle".into(),
            serde_json::json!(format!(
                "PDF {} V{} · Seite {} · {} · {} · Aufmaß {} R{} · {}",
                a.dokument.dokument_id.0,
                a.dokument.version,
                a.seite,
                a.bezeichnung,
                m.bezeichnung,
                b.aufmass_id,
                b.revision,
                RECHENSTAND
            )),
        );
    }
    Ok(())
}
