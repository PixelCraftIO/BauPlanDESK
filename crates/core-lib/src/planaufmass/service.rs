use super::*;
use crate::{
    CoreServices,
    berechnungen::{BerechnungsArt, FachPlan},
    domain::ProjectId,
    plattform::{ObjektArt, objekte::Gebaeude},
};
pub(super) struct Pruefkontext {
    grund: Result<std::collections::BTreeSet<Uuid>, String>,
    gebaeude: std::collections::BTreeMap<String, Result<Gebaeude, String>>,
    /// IDs der erfassten Flurstücke (Grundstücksinformationen).
    flurstuecke: std::collections::BTreeSet<String>,
}
impl Pruefkontext {
    pub(super) fn zuordnung(&self, m: &Planmarkierung) -> CoreResult<()> {
        let name = |t: &str| fehler(format!("Markierung „{}“: {t}", m.bezeichnung));
        if let Some(id) = &m.flurstueck_id {
            if m.bedeutung != Planbedeutung::Flurstueck && m.bedeutung.abzugsart().is_none() {
                return Err(name(
                    "Flurstückszuordnung gilt nur für Flurstücksgrenzen und Abzugsflächen",
                ));
            }
            if !self.flurstuecke.contains(id) {
                return Err(name(
                    "Flurstück ist nicht mehr erfasst; Flurstück neu wählen",
                ));
            }
        }
        if let Some(id) = m.grundstueck_id {
            let grund = self.grund.as_ref().map_err(|e| name(e))?;
            if !grund.contains(&id) {
                return Err(name(
                    "Baugrundstückszuordnung ist nicht mehr aktuell; Baugrundstück neu wählen",
                ));
            }
        }
        if let Some(id) = m.gebaeude_id {
            let g = self
                .gebaeude
                .get(&id.to_string())
                .ok_or_else(|| name("Gebäude ist nicht mehr erfasst; Gebäude neu wählen"))?
                .as_ref()
                .map_err(|e| name(e))?;
            if m.geschoss_id
                .is_some_and(|id| !g.geschosse.iter().any(|s| s.id == id))
            {
                return Err(name(
                    "Geschoss ist im Gebäude nicht mehr erfasst; Geschoss neu wählen",
                ));
            }
        }
        Ok(())
    }
}

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
        let mut stand = serde_json::json!({
            "id": m.id,
            "bezeichnung": m.bezeichnung,
            "bedeutung": m.bedeutung,
            "geometrie": m.geometrie,
            "grundstueck_id": m.grundstueck_id,
            "gebaeude_id": m.gebaeude_id,
            "geschoss_id": m.geschoss_id,
            "ausschnitt": m.ausschnitt_id.and_then(|id| a.ausschnitte.iter().find(|x| x.id == id)).map(|a| {
                let mut a=serde_json::to_value(a).expect("Ausschnitt serialisierbar");
                a.as_object_mut().expect("Ausschnitt ist Objekt").remove("ebene_id");
                a
            }),
        });
        // Nur wenn gesetzt, damit bestehende Bindungsstände gleich bleiben.
        if let Some(raum) = m.hoehenzone_von {
            stand["hoehenzone_von"] = serde_json::json!(raum);
        }
        if let Some(f) = &m.flurstueck_id {
            stand["flurstueck_id"] = serde_json::json!(f);
        }
        stand
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
        // Eigene Zeichnungen hängen an keiner Unterlagenversion; ihre
        // Hintergründe sind Darstellung und machen keine Bindung ungültig.
        if let Some(d) = &a.dokument {
            let aktuell = self
                .document_history(d.dokument_id)
                .await?
                .into_iter()
                .filter(|d| d.project_id == pid)
                .map(|d| d.version)
                .max();
            if aktuell != Some(d.version) {
                return Err(fehler(
                    "Unterlage hat eine neuere Version. Aufmaß auf neuer Unterlage prüfen.",
                ));
            }
        }
        let m = a
            .markierungen
            .iter()
            .find(|m| m.id == b.markierung_id)
            .ok_or_else(|| fehler("Planmarkierung fehlt"))?
            .clone();
        // Nur die gebundene Markierung prüfen (bei einer Höhenzone mit ihrem
        // Raum): eine andere, nicht mehr passende Markierung desselben Aufmaßes
        // blockiert diese Übernahme nicht.
        let mut einzeln = a.clone();
        einzeln
            .markierungen
            .retain(|x| x.id == m.id || Some(x.id) == m.hoehenzone_von);
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
        let kontext = self.planaufmass_kontext(pid, plan).await?;
        for m in &plan.markierungen {
            kontext.zuordnung(m)?;
        }
        Ok(werte)
    }
    pub(super) async fn planaufmass_kontext(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Pruefkontext> {
        if let Some(dokument) = &plan.dokument {
            let name = self.unterlage_name(pid, dokument, false).await?;
            kalibrierung_passend(name.ends_with(".dxf"), plan)?;
        } else {
            // Eigene Zeichnung: Hintergründe müssen Unterlagen dieses Projekts sein.
            for h in &plan.hintergruende {
                let name = self.unterlage_name(pid, &h.dokument, true).await?;
                if (name.ends_with(".dxf") || crate::ifc::ist_ifc(&name)) && h.seite != 1 {
                    return Err(fehler(
                        "DXF- und IFC-Hintergründe haben nur eine Zeichnung (Seite 1)",
                    ));
                }
                if crate::ifc::ist_ifc(&name) != h.geschoss.is_some() {
                    return Err(fehler(
                        "Ein IFC-Hintergrund benötigt ein Geschoss; andere Hintergründe haben keines",
                    ));
                }
            }
        }
        let projekt = self.get_project(pid).await?;
        let grund = if plan.markierungen.iter().any(|m| m.grundstueck_id.is_some()) {
            crate::grundstueckskontext::ermitteln(&projekt)
                .map(|g| g.into_iter().map(|g| g.id).collect())
                .map_err(|e| e.to_string())
        } else {
            Ok(std::collections::BTreeSet::new())
        };
        let gebaeude = self
            .fachobjekte(pid, ObjektArt::Gebaeude)
            .await?
            .into_iter()
            .map(|g| {
                (
                    g.objekt,
                    serde_json::from_value::<Gebaeude>(g.daten).map_err(|e| e.to_string()),
                )
            })
            .collect();
        let flurstuecke = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht)
            .flurstuecke
            .into_iter()
            .map(|f| f.id)
            .collect();
        Ok(Pruefkontext {
            grund,
            gebaeude,
            flurstuecke,
        })
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
                "Gebäudeübernahme und Aufmaß sind konkurrierende Eingabequellen. Gebäudeübernahme zuerst ausdrücklich deaktivieren.",
            ));
        }
        if plan.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Planverknüpfungen"));
        }
        let mut ziele = std::collections::HashSet::new();
        // Quellangaben je Zielzeile in Bindungsreihenfolge; mehrere Bindungen
        // einer Zeile ergänzen sich, statt sich zu überschreiben.
        let mut quellen: Vec<Zeilenquellen> = Vec::new();
        for b in plan.planbindungen.clone() {
            if !ziele.insert(b.ziel.clone()) || plan.uebernahmen.iter().any(|u| u.ziel == b.ziel) {
                return Err(fehler("Eingabefeld hat mehrere Quellen"));
            }
            let (a, m, wert) = self.planbindung_aufloesen(pid, &b).await?;
            if m.grundstueck_id != plan.grundstueck_id || m.gebaeude_id != plan.gebaeude_id {
                self.gebaeude_auf_grundstueck(pid, plan, &m).await?;
            }
            if einsetzen(plan, &b, &m, &a, wert)? {
                let text = format!(
                    "{} · {}",
                    self.planunterlage(pid, &a).await?,
                    quelltext(&a, &m, &b)
                );
                let t: Vec<_> = b.ziel.splitn(3, '.').collect();
                let feld = match b.groesse {
                    Planmessgroesse::Flaeche => "Fläche",
                    Planmessgroesse::Laenge => "Länge",
                    Planmessgroesse::Horizontale => "Horizontale",
                    Planmessgroesse::Hoehe => "Höhe",
                    Planmessgroesse::Neigung => "Neigung",
                };
                let eintrag = (feld, text);
                match quellen.iter_mut().find(|(x, y, _)| x == t[0] && y == t[1]) {
                    Some((_, _, v)) => v.push(eintrag),
                    None => quellen.push((t[0].into(), t[1].into(), vec![eintrag])),
                }
            }
        }
        for (tabelle, id, texte) in quellen {
            let text = match texte.as_slice() {
                // Eine Quelle: ohne Feldvorsatz.
                [(_, einzig)] => einzig.clone(),
                _ => texte
                    .iter()
                    .map(|(feld, t)| format!("{feld}: {t}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
            };
            if let Some(obj) = zeile(plan, &tabelle, &id) {
                obj.insert("quelle".into(), serde_json::json!(text));
            }
        }
        Ok(())
    }
    /// Grundstücksbezogene Bilanzen (Freiflächen, Entwässerung) dürfen
    /// Markierungen eines Gebäudes binden, das eindeutig auf ihrer
    /// Grundstückseinheit steht (z. B. die Gebäudegrundfläche als Dachfläche).
    async fn gebaeude_auf_grundstueck(
        &self,
        pid: ProjectId,
        plan: &FachPlan,
        m: &Planmarkierung,
    ) -> CoreResult<()> {
        let abweichend = || {
            fehler("Planmarkierung und Berechnung benötigen dieselbe Grundstücks-/Gebäudezuordnung")
        };
        let (Some(grund), Some(gid), None) = (plan.grundstueck_id, m.gebaeude_id, plan.gebaeude_id)
        else {
            return Err(abweichend());
        };
        if !matches!(
            plan.modul,
            BerechnungsArt::Freiflaechen | BerechnungsArt::Entwaesserung
        ) || m.grundstueck_id.is_some_and(|g| g != grund)
        {
            return Err(abweichend());
        }
        let p = self.get_project(pid).await?;
        let lageplan = crate::grundstuecksinformationen::plan(&p.planungsrecht)?;
        let f = self
            .fachobjekte(pid, ObjektArt::Gebaeude)
            .await?
            .into_iter()
            .find(|f| f.objekt == gid.to_string())
            .ok_or_else(|| fehler("Gebäude der Planmarkierung fehlt im Projekt"))?;
        let g: Gebaeude = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
        let eindeutig = lageplan.is_some_and(|l| {
            crate::plattform::einbindung::Lagebezug::neu(pid, &l).eindeutig_id(&g, grund)
        });
        if !eindeutig {
            return Err(fehler(format!(
                "Planmarkierung „{}“ gehört zum Gebäude „{}“, das nicht eindeutig auf dem Baugrundstück dieser Bilanz steht",
                m.bezeichnung, g.bezeichnung
            )));
        }
        Ok(())
    }
    /// Lesbare Bezeichnung der Unterlage: Dateiname, Plannummer und
    /// Index aus den Dokumentdaten (falls erfasst) und Dokumentversion.
    /// Dateiname (klein geschrieben) einer PDF- oder DXF-Unterlage des Projekts.
    async fn unterlage_name(
        &self,
        pid: ProjectId,
        d: &crate::plattform::objekte::Dokumentbezug,
        ifc_erlaubt: bool,
    ) -> CoreResult<String> {
        let dokument = self
            .document_history(d.dokument_id)
            .await?
            .into_iter()
            .find(|x| x.project_id == pid && x.version == d.version)
            .ok_or_else(|| fehler("Dokumentversion gehört nicht zum Projekt"))?;
        let name = dokument.file_name.to_lowercase();
        if ifc_erlaubt && crate::ifc::ist_ifc(&name) {
            return Ok(name);
        }
        if !name.ends_with(".dxf") && !name.ends_with(".pdf") {
            return Err(fehler("Aufmaß benötigt eine PDF- oder DXF-Unterlage"));
        }
        Ok(name)
    }
    async fn planunterlage(&self, pid: ProjectId, a: &Planaufmass) -> CoreResult<String> {
        let Some(dokument) = &a.dokument else {
            return Ok("Eigene Zeichnung (Meter)".into());
        };
        let datei = self
            .document_history(dokument.dokument_id)
            .await?
            .into_iter()
            .find(|d| d.project_id == pid && d.version == dokument.version)
            .map(|d| d.file_name)
            .unwrap_or_else(|| "Unterlage".into());
        let daten = self
            .fachobjekte(pid, ObjektArt::Dokumentdaten)
            .await?
            .into_iter()
            .find(|f| f.objekt == dokument.dokument_id.0.to_string())
            .and_then(|f| {
                serde_json::from_value::<crate::plattform::objekte::Dokumentdaten>(f.daten).ok()
            })
            .filter(|d| d.dokument.dokument_id == dokument.dokument_id);
        let mut t = datei;
        if let Some(d) = daten.filter(|d| !d.plannummer.trim().is_empty()) {
            t.push_str(&format!(" · Plan {}", d.plannummer.trim()));
            if !d.revisionsindex.trim().is_empty() {
                t.push_str(&format!(" Index {}", d.revisionsindex.trim()));
            }
        }
        Ok(format!("PDF {t} V{}", dokument.version))
    }
}

/// Tabelle, Zeilen-ID und die Quellangaben (Messgröße, Text) einer Zielzeile.
type Zeilenquellen = (String, String, Vec<(&'static str, String)>);
/// Herkunft einer Bindung ohne technische Kennungen.
fn quelltext(a: &Planaufmass, m: &Planmarkierung, b: &Planbindung) -> String {
    format!(
        "Seite {} · {} · {} · Aufmaß R{} · {}",
        a.seite, a.bezeichnung, m.bezeichnung, b.revision, RECHENSTAND
    )
}
fn zeile<'a>(
    plan: &'a mut FachPlan,
    tabelle: &str,
    id: &str,
) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
    plan.eingaben
        .get_mut(tabelle)
        .and_then(|v| v.as_array_mut())
        .and_then(|v| {
            v.iter_mut()
                .find(|v| v.get("id").and_then(|v| v.as_str()) == Some(id))
        })
        .and_then(|v| v.as_object_mut())
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
/// Setzt den Messwert ein; `true`, wenn die Zielzeile eine Quellangabe führt.
fn einsetzen(
    plan: &mut FachPlan,
    b: &Planbindung,
    m: &Planmarkierung,
    a: &Planaufmass,
    wert: f64,
) -> CoreResult<bool> {
    let teile: Vec<_> = b.ziel.split('.').collect();
    if teile.len() != 3 || zielgroesse(plan.modul, teile[0], teile[2]) != Some(b.groesse) {
        return Err(fehler("Dieses Feld unterstützt diese Planmessgröße nicht"));
    }
    // Absolute Höhen dürfen nicht aus einer bloßen Streckenlänge entstehen.
    if plan.modul == BerechnungsArt::Hoehen && teile[0] == "punkte" {
        // Höhensystem der Markierung: Bezugslinie ihres Ausschnitts, sonst blattweit.
        let aufmass_system = a.hoehensystem(m);
        if !matches!(m.geometrie, Plangeometrie::Hoehenpunkt { .. }) || aufmass_system.is_none() {
            return Err(fehler(
                "Absolute Höhe benötigt einen Höhenpunkt mit benanntem Höhenbezug",
            ));
        }
        let system = plan
            .eingaben
            .get("hoehenbezug")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if Some(system) != aufmass_system {
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
    Ok(obj.contains_key("quelle"))
}

/// PDF-Blätter werden über Maßstab oder Referenzstrecke kalibriert, DXF-
/// Zeichnungen über ihre Zeichnungseinheit oder eine Referenzstrecke; eine
/// DXF-Unterlage besitzt genau eine Zeichnung (Seite 1).
fn kalibrierung_passend(dxf: bool, plan: &Planaufmass) -> CoreResult<()> {
    if dxf && plan.seite != 1 {
        return Err(fehler("DXF-Unterlagen haben nur eine Zeichnung (Seite 1)"));
    }
    for k in plan.kalibrierung.iter().chain(
        plan.ausschnitte
            .iter()
            .filter_map(|a| a.kalibrierung.as_ref()),
    ) {
        match (dxf, k) {
            (true, Plankalibrierung::Massstab { .. }) => {
                return Err(fehler(
                    "Ein Blattmaßstab gilt nur für PDF-Blätter; DXF-Aufmaße verwenden die Zeichnungseinheit oder eine Referenzstrecke",
                ));
            }
            (false, Plankalibrierung::Einheit { .. }) => {
                return Err(fehler(
                    "Eine Zeichnungseinheit gilt nur für DXF-Unterlagen; PDF-Blätter werden über Maßstab oder Referenzstrecke kalibriert",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
