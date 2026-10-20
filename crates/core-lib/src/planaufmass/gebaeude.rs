use super::*;
use crate::{CoreServices, domain::ProjectId, plattform::objekte::Gebaeude};

pub fn gebaeudeziele(g: &Gebaeude) -> Vec<Planziel> {
    let mut z = vec![Planziel {
        ziel: "geometrie.grundflaeche_m2".into(),
        titel: "Gebäudegrundfläche · horizontale Projektion".into(),
        groesse: Planmessgroesse::Flaeche,
    }];
    for (feld, name) in [
        ("nullpunkt_m_nhn", "Gebäudenullpunkt (NHN)"),
        ("gelaende_m", "Maßgebende Geländehöhe"),
        ("traufe_m", "Traufe"),
        ("first_m", "First"),
        ("fundamentsohle_m", "Fundamentsohle"),
    ] {
        z.push(Planziel {
            ziel: format!("geometrie.{feld}"),
            titel: name.into(),
            groesse: Planmessgroesse::Hoehe,
        });
    }
    for s in &g.geschosse {
        for (feld, name, groesse) in [
            (
                "flaeche_m2",
                "Fläche nach Außenmaßen",
                Planmessgroesse::Flaeche,
            ),
            ("fussboden_m", "OK Fertigfußboden", Planmessgroesse::Hoehe),
            ("decke_oberkante_m", "OK Decke", Planmessgroesse::Hoehe),
            ("lichte_hoehe_m", "Lichte Höhe", Planmessgroesse::Hoehe),
            ("geschosshoehe_m", "Geschosshöhe", Planmessgroesse::Hoehe),
            ("deckendicke_m", "Deckendicke", Planmessgroesse::Hoehe),
        ] {
            z.push(Planziel {
                ziel: format!("geschosse.{}.{feld}", s.id),
                titel: format!("{} · {name}", s.bezeichnung),
                groesse,
            });
        }
    }
    z
}
impl CoreServices {
    pub async fn planaufmass_gebaeude(
        &self,
        pid: ProjectId,
        id: Uuid,
        mut g: Gebaeude,
    ) -> CoreResult<Gebaeude> {
        self.planwerte_gebaeude(pid, id, &mut g).await?;
        g.grundlagen(id)?;
        Ok(g)
    }
    pub(crate) async fn planwerte_gebaeude(
        &self,
        pid: ProjectId,
        id: Uuid,
        g: &mut Gebaeude,
    ) -> CoreResult<()> {
        if g.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Gebäudeverknüpfungen"));
        }
        let ziele = gebaeudeziele(g);
        let mut seen = std::collections::HashSet::new();
        let mut bindungen = g.planbindungen.clone();
        // Der Gebäudenullpunkt wird vor den davon abhängigen relativen Höhen aufgelöst.
        bindungen.sort_by_key(|b| b.ziel != "geometrie.nullpunkt_m_nhn");
        for b in bindungen {
            if !seen.insert(b.ziel.clone())
                || !ziele
                    .iter()
                    .any(|z| z.ziel == b.ziel && z.groesse == b.groesse)
            {
                return Err(fehler(
                    "Gebäudefeld fehlt, ist mehrfach verknüpft oder unterstützt die Messgröße nicht",
                ));
            }
            let (a, m, mut wert) = self.planbindung_aufloesen(pid, &b).await?;
            if m.gebaeude_id != Some(id) {
                return Err(fehler("Planmarkierung gehört zu einem anderen Gebäude"));
            }
            if let Some(grund) = m.grundstueck_id {
                let projekt = self.get_project(pid).await?;
                let k = crate::grundstueckskontext::ermitteln(&projekt)?;
                let k = k
                    .iter()
                    .find(|k| k.id == grund)
                    .ok_or_else(|| fehler("Quellgrundstück fehlt"))?;
                if g.flurstueck_ids.is_empty()
                    || !g
                        .flurstueck_ids
                        .iter()
                        .all(|f| k.flurstueck_ids.contains(f))
                {
                    return Err(fehler(
                        "Planmarkierung und Gebäude benötigen dieselbe Flurstücks-/Grundstückszuordnung",
                    ));
                }
            }
            let t: Vec<_> = b.ziel.split('.').collect();
            let feld = *t.last().unwrap();
            let ist_geschoss = t[0] == "geschosse";
            if ist_geschoss && m.geschoss_id.map(|s| s.to_string()).as_deref() != Some(t[1]) {
                return Err(fehler("Planmarkierung gehört zu einem anderen Geschoss"));
            }
            if b.groesse == Planmessgroesse::Flaeche {
                let bedeutung = if ist_geschoss {
                    Planbedeutung::GeschossAussenkante
                } else {
                    Planbedeutung::Gebaeudegrundflaeche
                };
                if m.bedeutung != bedeutung {
                    return Err(fehler(
                        "Geschossaußenkante und Gebäudegrundfläche ausdrücklich unterscheiden und passend klassifizieren",
                    ));
                }
            } else if matches!(feld, "lichte_hoehe_m" | "geschosshoehe_m" | "deckendicke_m") {
                if !matches!(m.geometrie, Plangeometrie::Strecke { .. }) {
                    return Err(fehler(
                        "Höhenabmessung benötigt eine gemessene Höhendifferenz",
                    ));
                }
            } else {
                if !matches!(m.geometrie, Plangeometrie::Hoehenpunkt { .. })
                    || !a
                        .hoehenbezug
                        .as_ref()
                        .is_some_and(|h| matches!(h.system.as_str(), "NHN" | "m ü. NHN"))
                {
                    return Err(fehler(
                        "Gebäudehöhen benötigen einen Höhenpunkt im System „m ü. NHN“",
                    ));
                }
                if feld != "nullpunkt_m_nhn" {
                    wert -= g
                        .geometrie
                        .as_ref()
                        .and_then(|g| g.nullpunkt_m_nhn)
                        .ok_or_else(|| fehler("Gebäudenullpunkt in NHN zuerst festlegen"))?;
                }
            }
            if ist_geschoss {
                let s = g
                    .geschosse
                    .iter_mut()
                    .find(|s| s.id.to_string() == t[1])
                    .ok_or_else(|| fehler("Zielgeschoss fehlt"))?;
                let geo = s.geometrie.get_or_insert_with(Default::default);
                match feld {
                    "flaeche_m2" => geo.flaeche_m2 = Some(wert),
                    "fussboden_m" => geo.fussboden_m = Some(wert),
                    "decke_oberkante_m" => geo.decke_oberkante_m = Some(wert),
                    "lichte_hoehe_m" => geo.lichte_hoehe_m = Some(wert),
                    "geschosshoehe_m" => geo.geschosshoehe_m = Some(wert),
                    "deckendicke_m" => geo.deckendicke_m = Some(wert),
                    _ => unreachable!(),
                }
            } else {
                let geo = g.geometrie.get_or_insert_with(Default::default);
                match feld {
                    "grundflaeche_m2" => geo.grundflaeche_m2 = Some(wert),
                    "nullpunkt_m_nhn" => geo.nullpunkt_m_nhn = Some(wert),
                    "gelaende_m" => geo.gelaende_m = Some(wert),
                    "traufe_m" => geo.traufe_m = Some(wert),
                    "first_m" => geo.first_m = Some(wert),
                    "fundamentsohle_m" => geo.fundamentsohle_m = Some(wert),
                    _ => unreachable!(),
                }
            }
        }
        Ok(())
    }
    pub async fn gebaeude_planquellen_pruefen(
        &self,
        pid: ProjectId,
        id: Uuid,
        g: &Gebaeude,
    ) -> CoreResult<()> {
        let mut neu = g.clone();
        self.planwerte_gebaeude(pid, id, &mut neu).await?;
        if serde_json::to_value(neu).map_err(|e| fehler(e.to_string()))?
            != serde_json::to_value(g).map_err(|e| fehler(e.to_string()))?
        {
            return Err(fehler(
                "Gebäudewerte weichen vom PDF-Aufmaß ab. Planquellen aktualisieren und speichern.",
            ));
        }
        Ok(())
    }
}
