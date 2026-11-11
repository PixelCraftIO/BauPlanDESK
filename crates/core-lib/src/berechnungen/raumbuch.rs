//! Explizite Übernahme von Raumfakten. Anrechnung/Klassifikation bleibt beim Zielmodul.
use super::*;
use crate::{CoreServices, domain::ProjectId, plattform::objekte::Raumbuch};
impl CoreServices {
    pub async fn berechnung_aus_raumbuch(
        &self,
        p: ProjectId,
        id: Uuid,
        modul: BerechnungsArt,
    ) -> CoreResult<FachPlan> {
        if !matches!(modul, BerechnungsArt::Wohnflaechen | BerechnungsArt::Din277) {
            return Err(err(
                "Raumfakten sind für dieses Modul nicht als direkter Import vorgesehen",
            ));
        }
        let f = self
            .fachobjekte(p, ObjektArt::Raumbuch)
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string())
            .ok_or_else(|| err("Raumbuch gehört nicht zu diesem Projekt"))?;
        let r: Raumbuch = decode(&f.daten)?;
        self.raumbuch_planquellen_pruefen(p, &r).await?;
        let mut plan = vorlagen()?
            .into_iter()
            .find(|v| v.plan.modul == modul)
            .ok_or_else(|| err("Modulvorlage fehlt"))?
            .plan;
        plan.gebaeude_id = Some(r.gebaeude_id);
        plan.fachquellen.push(Fachquelle {
            art: ObjektArt::Raumbuch,
            objekt: id,
            revision: f.revision,
        });
        // DIN 277: Flächenart aus der Klassifikation der gebundenen Planmarkierung,
        // sonst aus der Raumbuch-Nutzung; Geschoss aus dem Gebäude.
        let mut din: std::collections::HashMap<Uuid, (&'static str, Option<Uuid>)> =
            std::collections::HashMap::new();
        let mut geschosse: std::collections::HashMap<Uuid, String> =
            std::collections::HashMap::new();
        if modul == BerechnungsArt::Din277 {
            for b in &r.planbindungen {
                let Some(raum) = b
                    .ziel
                    .strip_prefix("raeume.")
                    .and_then(|z| z.split('.').next())
                    .and_then(|z| z.parse::<Uuid>().ok())
                else {
                    continue;
                };
                let (_, m, _) = self.planbindung_aufloesen(p, b).await?;
                din.insert(
                    raum,
                    (flaechenart(m.bedeutung).unwrap_or(""), m.geschoss_id),
                );
            }
            if let Some(g) = self
                .fachobjekte(p, ObjektArt::Gebaeude)
                .await?
                .into_iter()
                .find(|f| f.objekt == r.gebaeude_id.to_string())
            {
                let g: crate::plattform::objekte::Gebaeude = decode(&g.daten)?;
                for (i, s) in g.geschosse.iter().enumerate() {
                    let name = match s.bezeichnung.trim() {
                        "" => format!("Geschoss {}", i + 1),
                        b => b.to_string(),
                    };
                    geschosse.insert(s.id, name);
                }
            }
        }
        let din_art = |raum: &crate::plattform::objekte::Raum| {
            din.get(&raum.id)
                .map(|(a, _)| *a)
                .filter(|a| !a.is_empty())
                .or_else(|| nutzungsart(&raum.nutzung))
                .unwrap_or("")
        };
        let din_geschoss = |raum: &crate::plattform::objekte::Raum| {
            din.get(&raum.id)
                .and_then(|(_, g)| *g)
                .or(r.geschoss_id)
                .and_then(|g| geschosse.get(&g).cloned())
                .unwrap_or_default()
        };
        // Wohnfläche: je Höhenzone eine eigene Position (ID „Raum_Zone“) mit
        // der kleinsten lichten Höhe der Zone; die Restfläche behält die Raum-ID
        // und die Raumhöhe. Die Anrechnung nach Höhe bestimmt das WoFlV-Modul.
        // DIN 277 zählt die Grundfläche unabhängig von der Höhe: ein Raum, eine Position.
        let wohnflaeche = |raum: &crate::plattform::objekte::Raum| {
            let name = format!("{} {}", raum.nummer, raum.bezeichnung);
            let quelle = format!(
                "Raumbuch {} · Revision {} · Raum {}",
                r.bezeichnung, f.revision, raum.nummer
            );
            let position = |id: String,
                            name: String,
                            flaeche: f64,
                            hoehe: Option<f64>,
                            begruendung: &str| {
                serde_json::json!({"id":id,"wohnung":"","raum":name,"typ":"","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":flaeche,"lichte_hoehe_m":hoehe,"sonderfaktor":null,"begruendung":begruendung,"quelle":quelle})
            };
            if raum.hoehenzonen.is_empty() {
                return vec![position(
                    raum.id.to_string(),
                    name,
                    raum.flaeche_m2,
                    raum.hoehe_m,
                    "Raumart, Wohnungszuordnung und Höhenzonen prüfen",
                )];
            }
            let zonen: f64 = raum.hoehenzonen.iter().map(|z| z.flaeche_m2).sum();
            let rest = raum.flaeche_m2 - zonen;
            let mut liste = Vec::new();
            if rest >= 0.005 {
                liste.push(position(
                    raum.id.to_string(),
                    name.clone(),
                    rest,
                    raum.hoehe_m,
                    "Raumart und Wohnungszuordnung prüfen; Restfläche außerhalb der Höhenzonen",
                ));
            }
            for z in &raum.hoehenzonen {
                liste.push(position(
                    format!("{}_{}", raum.id, z.id),
                    format!("{name} – {}", z.bezeichnung),
                    z.flaeche_m2,
                    z.lichte_hoehe_min_m,
                    "Raumart und Wohnungszuordnung prüfen; Höhenzone des Raumbuchs (kleinste lichte Höhe)",
                ));
            }
            liste
        };
        plan.eingaben["positionen"] = serde_json::Value::Array(
            r.raeume
                .iter()
                .flat_map(|raum| {
                    if modul == BerechnungsArt::Wohnflaechen {
                        wohnflaeche(raum)
                    } else {
                        vec![serde_json::json!({"id":raum.id.to_string(),"geschoss":din_geschoss(raum),"raum":format!("{} {}",raum.nummer,raum.bezeichnung),"art":din_art(raum),"bereich":"R","form":"aufmass","laenge_m":0,"breite_m":0,"breite2_m":0,"aufmass_m2":raum.flaeche_m2,"quelle":format!("Raumbuch {} · Revision {} · Klassifikation und Maßbezug prüfen",r.bezeichnung,f.revision)})]
                    }
                })
                .collect(),
        );
        Ok(plan)
    }
    /// Führt eine Berechnung ihrem Raumbuch nach: Flächen, Höhen und Höhenzonen
    /// kommen aus dem aktuellen Raumbuch, die eigenen Angaben der Berechnung
    /// (Wohnung, Raumart, Geschoss, Flächenart) bleiben je Raum erhalten.
    /// Entfallene Räume entfallen, neue kommen hinzu; nur dann sind die
    /// Bestätigungen der Berechnung erneut nötig. `false`, wenn nichts zu tun war.
    pub async fn berechnung_raumbuch_nachfuehren(
        &self,
        p: ProjectId,
        plan: &mut FachPlan,
    ) -> CoreResult<bool> {
        if !matches!(
            plan.modul,
            BerechnungsArt::Wohnflaechen | BerechnungsArt::Din277
        ) || plan.fachquellen.is_empty()
        {
            return Ok(false);
        }
        let staende = self.fachobjekte(p, ObjektArt::Raumbuch).await?;
        if plan.fachquellen.iter().all(|q| {
            staende
                .iter()
                .any(|f| f.objekt == q.objekt.to_string() && f.revision == q.revision)
        }) {
            return Ok(false);
        }
        let mut neu = Vec::new();
        let mut quellen = Vec::new();
        for q in plan.fachquellen.clone() {
            let r = self
                .berechnung_aus_raumbuch(p, q.objekt, plan.modul)
                .await?;
            if let Some(serde_json::Value::Array(z)) = r.eingaben.get("positionen") {
                neu.extend(z.iter().cloned());
            }
            quellen.extend(r.fachquellen);
        }
        let alt = plan
            .eingaben
            .get("positionen")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let kennung = |x: &serde_json::Value| x["id"].as_str().unwrap_or_default().to_string();
        let aus_raumbuch = |x: &serde_json::Value| {
            x["quelle"]
                .as_str()
                .is_some_and(|q| q.starts_with("Raumbuch"))
        };
        let leer = |v: &serde_json::Value| v.is_null() || v.as_str() == Some("");
        let eigene: &[&str] = if plan.modul == BerechnungsArt::Wohnflaechen {
            &["wohnung", "typ", "sonderfaktor", "begruendung"]
        } else {
            &["geschoss", "art", "bereich"]
        };
        let mut unbekannt = false;
        let mut positionen = Vec::new();
        for mut n in neu.iter().cloned() {
            let id = kennung(&n);
            let raum = id.split('_').next().unwrap_or_default().to_string();
            let gleich = alt.iter().find(|a| kennung(a) == id);
            // Neue Höhenzone eines bekannten Raums: Raumart und Wohnung vom Raum.
            let verwandt = gleich.or_else(|| {
                id.contains('_')
                    .then(|| {
                        alt.iter().find(|a| {
                            let k = kennung(a);
                            k == raum || k.starts_with(&format!("{raum}_"))
                        })
                    })
                    .flatten()
            });
            match verwandt {
                Some(a) => {
                    for k in eigene {
                        if (*k != "begruendung" || gleich.is_some())
                            && let Some(v) = a.get(*k)
                            && !leer(v)
                        {
                            n[*k] = v.clone();
                        }
                    }
                    if gleich.is_some()
                        && n.get("lichte_hoehe_m").is_some_and(|v| v.is_null())
                        && a.get("lichte_hoehe_m").is_some_and(|v| !v.is_null())
                    {
                        n["lichte_hoehe_m"] = a["lichte_hoehe_m"].clone();
                    }
                }
                None => unbekannt = true,
            }
            positionen.push(n);
        }
        let manuell: Vec<_> = alt
            .iter()
            .filter(|a| !aus_raumbuch(a) && !neu.iter().any(|n| kennung(n) == kennung(a)))
            .cloned()
            .collect();
        // Planbindungen bleiben nur an Positionen ohne Raumbuchquelle.
        plan.planbindungen.retain(|b| {
            b.ziel.strip_prefix("positionen.").is_none_or(|rest| {
                let id = rest.split('.').next().unwrap_or_default();
                manuell.iter().any(|m| kennung(m) == id)
            })
        });
        positionen.extend(manuell);
        plan.eingaben["positionen"] = serde_json::Value::Array(positionen);
        plan.fachquellen = quellen;
        if unbekannt {
            for f in [
                "aufmass_vollstaendig",
                "wohneinheiten_bestaetigt",
                "klassifikation_geprueft",
                "kgf_vollstaendig",
            ] {
                if plan.eingaben.get(f).is_some() {
                    plan.eingaben[f] = serde_json::Value::Bool(false);
                }
            }
            plan.regelkontext.anwendbarkeit_bestaetigt = false;
        }
        Ok(true)
    }
}

/// DIN-277-Flächenart einer klassifizierten Planmarkierung.
fn flaechenart(b: crate::planaufmass::Planbedeutung) -> Option<&'static str> {
    use crate::planaufmass::Planbedeutung::*;
    match b {
        Nuf => Some("nuf"),
        Tf => Some("tf"),
        Vf => Some("vf"),
        Kgf => Some("kgf"),
        _ => None,
    }
}
/// DIN-277-Flächenart aus einer eindeutig klassifizierenden Raumbuch-Nutzung
/// (so führt die Planübernahme NUF/TF/VF); andere Nutzungen bleiben offen.
fn nutzungsart(nutzung: &str) -> Option<&'static str> {
    match nutzung.trim().to_lowercase().as_str() {
        "nuf" => Some("nuf"),
        "tf" | "technik" => Some("tf"),
        "vf" | "verkehr" => Some("vf"),
        "kgf" => Some("kgf"),
        _ => None,
    }
}
