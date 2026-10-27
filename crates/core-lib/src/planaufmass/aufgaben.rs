//! Aufmaß-Aufgaben: Welche Zielfelder der Flurstücke, Gebäude und manuellen
//! Abstandswände sind noch offen, welche Bedeutung und Geometrie braucht eine Markierung
//! dafür, und gibt es in diesem Aufmaß bereits eine passende Markierung?
//! Reine Auskunft für die Oberfläche; Übernahme und Prüfung bleiben bei den
//! bestehenden Diensten (`planaufmass_flurstuecke`, `planaufmass_gebaeude`,
//! `planaufmass_abstand`).
use super::*;
use crate::{
    CoreServices, abstandsflaechen::AbstandsPlan, domain::ProjectId, plattform::ObjektArt,
    plattform::objekte::Gebaeude,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aufgabengeometrie {
    Flaeche,
    Hoehenpunkt,
    Strecke,
}
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Planaufgabe {
    /// `flurstueck`, `abzug`, `bauteil`, `gebaeude` oder `abstand`.
    pub verbraucher: String,
    /// Flurstücks-ID, Zeilenkennung des Abzugs bzw. Bauteils, Gebäude-ID bzw. ID des Abstandsplans.
    pub objekt: String,
    pub objekt_name: String,
    /// Anzeigegruppe, z. B. „Lagerhalle · OG“ oder „Südwand“.
    pub gruppe: String,
    pub ziel: String,
    pub titel: String,
    pub groesse: Planmessgroesse,
    pub bedeutung: Planbedeutung,
    pub geometrie: Aufgabengeometrie,
    pub gebaeude_id: Option<Uuid>,
    pub geschoss_id: Option<Uuid>,
    pub grundstueck_id: Option<Uuid>,
    /// Flurstück, dem eine neue Markierung zugeordnet wird (Verbraucher `flurstueck`).
    pub flurstueck_id: Option<String>,
    /// Zielfeld trägt bereits einen (manuellen) Wert.
    pub wert_vorhanden: bool,
    /// Bereits verknüpfte Markierung (aus einem beliebigen Aufmaß).
    pub gebunden: Option<Uuid>,
    /// Passende, noch nicht verknüpfte Markierung dieses Aufmaßes.
    pub markierung: Option<Uuid>,
    pub hinweis: Option<String>,
}

fn gebaeudefeld(
    feld: &str,
    geschoss: bool,
) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "grundflaeche_m2" => (Gebaeudegrundflaeche, Flaeche, None),
        "flaeche_m2" if geschoss => (GeschossAussenkante, Flaeche, None),
        "nullpunkt_m_nhn" => (
            Nullpunkt,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie im System „m ü. NHN“"),
        ),
        "gelaende_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Maßgebendes Gelände; Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "traufe_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "first_m" => (
            First,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "fundamentsohle_m" => (
            Fundamentsohle,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "fussboden_m" => (
            Fussboden,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "decke_oberkante_m" => (
            Decke,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        _ => (
            Hoehenmass,
            Strecke,
            Some("Senkrechte Strecke im Schnitt (Höhendifferenz)"),
        ),
    }
}
fn wandfeld(feld: &str) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "gelaende_a_m" | "gelaende_b_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "abschluss_a_m" | "abschluss_b_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "dachhoehe_m" => (
            Dach,
            Strecke,
            Some("Senkrechte Strecke vom Wandabschluss bis zur Dachoberkante"),
        ),
        _ => (
            Dach,
            Strecke,
            Some("Strecke entlang der Dachhaut; Neigung zur Horizontalen"),
        ),
    }
}
fn passt(m: &Planmarkierung, geometrie: Aufgabengeometrie) -> bool {
    matches!(
        (geometrie, &m.geometrie),
        (
            Aufgabengeometrie::Flaeche,
            Plangeometrie::Polygon { .. }
                | Plangeometrie::Flaeche { .. }
                | Plangeometrie::Rechteck { .. }
                | Plangeometrie::Kreis { .. }
        ) | (
            Aufgabengeometrie::Hoehenpunkt,
            Plangeometrie::Hoehenpunkt { .. }
        ) | (Aufgabengeometrie::Strecke, Plangeometrie::Strecke { .. })
    )
}
fn gefuellt(v: &serde_json::Value, ziel: &str) -> bool {
    // Pfade sind `tabelle.id.feld` oder `geometrie.feld`.
    let t: Vec<&str> = ziel.split('.').collect();
    let feld = match t.as_slice() {
        [tabelle, id, feld] => v
            .get(tabelle)
            .and_then(|a| a.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|z| z.get("id").and_then(|i| i.as_str()) == Some(id))
            })
            .and_then(|z| z.get("geometrie").or(Some(z)))
            .and_then(|z| z.get(feld))
            .cloned(),
        [gruppe, feld] => v.get(gruppe).and_then(|g| g.get(feld)).cloned(),
        _ => None,
    };
    feld.is_some_and(|f| !f.is_null())
}

/// Plausibler Bereich einer Höhenabmessung (m): Deckendicken sind dünn,
/// lichte Höhen und Geschosshöhen nicht.
fn hoehenmass_plausibel(feld: &str, h: f64) -> bool {
    match feld {
        "deckendicke_m" => h > 0. && h < 0.6,
        "lichte_hoehe_m" | "geschosshoehe_m" => h >= 0.6,
        _ => true,
    }
}
/// Namensbestandteile, an denen eine Höhenmaß-Markierung ihr Ziel erkennen lässt.
fn hoehenmass_name(feld: &str) -> Option<&'static [&'static str]> {
    match feld {
        "lichte_hoehe_m" => Some(&["licht"]),
        "geschosshoehe_m" => Some(&["geschosshöhe", "geschosshoehe"]),
        "deckendicke_m" => Some(&["decke", "kehlbalken"]),
        _ => None,
    }
}
fn benannt(m: &Planmarkierung, namen: &[&str]) -> bool {
    let b = m.bezeichnung.to_lowercase();
    namen.iter().any(|n| b.contains(n))
}

impl CoreServices {
    pub async fn planaufmass_aufgaben(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Vec<Planaufgabe>> {
        let mut aufgaben = Vec::new();
        // Gemessene Höhendifferenzen für die Plausibilität von Höhenabmessungen;
        // ein nicht messbares Aufmaß schränkt die Vorschläge nicht ein.
        let hoehen: std::collections::HashMap<Uuid, f64> = plan
            .messen()
            .map(|w| {
                w.into_iter()
                    .filter_map(|w| w.hoehe_m.map(|h| (w.id, h)))
                    .collect()
            })
            .unwrap_or_default();
        let frei = |gebunden: Option<Uuid>, geometrie, filter: &dyn Fn(&Planmarkierung) -> bool| {
            if gebunden.is_some() {
                return None;
            }
            plan.markierungen
                .iter()
                .find(|m| passt(m, geometrie) && filter(m))
                .map(|m| m.id)
        };
        // Flurstücke: eine Markierung „Flurstück“ je Flurstück, aus Flurkarte,
        // Liegenschaftskarte oder Lageplan. Ohne Zuordnung in den Eigenschaften
        // wird nichts vorgeschlagen; die Bezeichnung allein ist kein Nachweis.
        let projekt = self.get_project(pid).await?;
        let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
        let mut flurstueck_vergeben: std::collections::HashSet<Uuid> = infos
            .flurstuecke
            .iter()
            .filter_map(|f| f.planbindung.as_ref().map(|b| b.markierung_id))
            .collect();
        for (f, z) in infos.flurstuecke.iter().zip(flurstueckziele(&infos)) {
            let gebunden = f.planbindung.as_ref().map(|b| b.markierung_id);
            let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                m.bedeutung == Planbedeutung::Flurstueck
                    && m.flurstueck_id.as_deref() == Some(f.id.as_str())
                    && !flurstueck_vergeben.contains(&m.id)
            });
            flurstueck_vergeben.extend(markierung);
            aufgaben.push(Planaufgabe {
                verbraucher: "flurstueck".into(),
                objekt: f.id.clone(),
                objekt_name: format!("Flurstück {}", f.bezeichnung()),
                gruppe: "Flurstücke".into(),
                titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                ziel: z.ziel,
                groesse: z.groesse,
                bedeutung: Planbedeutung::Flurstueck,
                geometrie: Aufgabengeometrie::Flaeche,
                gebaeude_id: None,
                geschoss_id: None,
                grundstueck_id: None,
                flurstueck_id: Some(f.id.clone()),
                wert_vorhanden: f.flaeche_m2 > 0. && !f.flaeche_aus_plan,
                gebunden,
                markierung,
                hinweis: Some(
                    "Flurstücksgrenze in Flurkarte, Liegenschaftskarte oder Lageplan; die Katasterfläche bleibt Rechenwert, solange sie erfasst ist"
                        .into(),
                ),
            });
        }
        // Abzugsflächen und GRZ-Bauteile: nur erfasste Zeilen mit Kennung. Neue
        // Zeilen entstehen aus Markierungen über „Aus Aufmaß übernehmen“
        // unter Berechnungen → Baugrundstück; dort wird auch gespeichert.
        if let Some(e) = crate::grundstuecksinformationen::ermittlung(&projekt.planungsrecht)? {
            let mut vergeben: std::collections::HashSet<Uuid> = e
                .ansaetze
                .iter()
                .flat_map(|a| a.abzuege.iter())
                .filter_map(|x| x.planbindung.as_ref().map(|b| b.markierung_id))
                .collect();
            let ziele = abzugsziele(&e, &infos);
            for a in &e.ansaetze {
                for x in &a.abzuege {
                    let (Some(id), Some(bedeutung)) =
                        (&x.id, Planbedeutung::fuer_abzugsart(&x.art))
                    else {
                        continue;
                    };
                    let Some(z) = ziele
                        .iter()
                        .find(|z| z.ziel == format!("abzuege.{id}.flaeche_m2"))
                    else {
                        continue;
                    };
                    let gebunden = x.planbindung.as_ref().map(|b| b.markierung_id);
                    let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                        m.bedeutung == bedeutung
                            && m.flurstueck_id.as_deref() == Some(a.flurstueck_id.as_str())
                            && !vergeben.contains(&m.id)
                    });
                    vergeben.extend(markierung);
                    let (objekt_name, titel) = z
                        .titel
                        .rsplit_once(" · ")
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_else(|| (z.titel.clone(), "Fläche".into()));
                    aufgaben.push(Planaufgabe {
                        verbraucher: "abzug".into(),
                        objekt: id.clone(),
                        objekt_name,
                        gruppe: "Abzugsflächen".into(),
                        titel,
                        ziel: z.ziel.clone(),
                        groesse: z.groesse,
                        bedeutung,
                        geometrie: Aufgabengeometrie::Flaeche,
                        gebaeude_id: None,
                        geschoss_id: None,
                        grundstueck_id: None,
                        flurstueck_id: Some(a.flurstueck_id.clone()),
                        wert_vorhanden: x.flaeche_m2 > 0.,
                        gebunden,
                        markierung,
                        hinweis: Some(
                            "Abzugsfläche im Lageplan; Übernahme unter Berechnungen → Baugrundstück „Abzugsflächen aus Aufmaß übernehmen“"
                                .into(),
                        ),
                    });
                }
            }
            for b in &e.berechnungen {
                let Some(a) = &b.eingabe.aufstellung else {
                    continue;
                };
                let grundstueck = crate::plattform::einbindung::grundstueck_id(pid, &b.schluessel);
                let mut vergeben: std::collections::HashSet<Uuid> = a
                    .bauteile
                    .iter()
                    .filter_map(|x| x.planbindung.as_ref().map(|p| p.markierung_id))
                    .collect();
                let zeilen = a
                    .bauteile
                    .iter()
                    .filter(|x| x.gebaeude_id.is_none() && x.id.is_some());
                for (x, z) in zeilen.zip(bauteilziele(a)) {
                    let Some(bedeutung) = Planbedeutung::fuer_bauteiltyp(x.typ) else {
                        continue;
                    };
                    let gebunden = x.planbindung.as_ref().map(|p| p.markierung_id);
                    let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                        m.bedeutung == bedeutung
                            && m.grundstueck_id == Some(grundstueck)
                            && !vergeben.contains(&m.id)
                    });
                    vergeben.extend(markierung);
                    let (objekt_name, titel) = z
                        .titel
                        .rsplit_once(" · ")
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_else(|| (z.titel.clone(), "Grundfläche".into()));
                    aufgaben.push(Planaufgabe {
                        verbraucher: "bauteil".into(),
                        objekt: x.id.clone().unwrap_or_default(),
                        objekt_name,
                        gruppe: "GRZ-Bauteile".into(),
                        titel,
                        ziel: z.ziel,
                        groesse: z.groesse,
                        bedeutung,
                        geometrie: Aufgabengeometrie::Flaeche,
                        gebaeude_id: None,
                        geschoss_id: None,
                        grundstueck_id: Some(grundstueck),
                        flurstueck_id: None,
                        wert_vorhanden: x.flaeche_m2 > 0.,
                        gebunden,
                        markierung,
                        hinweis: Some(
                            "Bauteil im Lageplan, dem Baugrundstück zugeordnet; Übernahme unter Berechnungen → Baugrundstück (GRZ) „Bauteile aus Aufmaß übernehmen“"
                                .into(),
                        ),
                    });
                }
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Gebaeude).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Gebäude-ID ungültig"))?;
            let g: Gebaeude =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            // Eine Markierung versorgt höchstens ein Gebäudefeld: bereits gebundene
            // oder für ein anderes Ziel vorgeschlagene Markierungen sind vergeben.
            let mut vergeben: std::collections::HashSet<Uuid> =
                g.planbindungen.iter().map(|b| b.markierung_id).collect();
            for z in gebaeudeziele(&g) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let geschoss = t[0] == "geschosse";
                let geschoss_id = if geschoss { t[1].parse().ok() } else { None };
                let (bedeutung, geometrie, hinweis) = gebaeudefeld(t[t.len() - 1], geschoss);
                let gebunden = g
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let feld = t[t.len() - 1];
                let passend = |m: &Planmarkierung| {
                    m.gebaeude_id == Some(id)
                        && m.bedeutung == bedeutung
                        && (!geschoss || m.geschoss_id == geschoss_id)
                        && !vergeben.contains(&m.id)
                        && hoehen
                            .get(&m.id)
                            .is_none_or(|h| hoehenmass_plausibel(feld, *h))
                };
                // Unter mehreren passenden Markierungen die fachlich benannte bevorzugen.
                let markierung = frei(gebunden, geometrie, &|m| {
                    passend(m) && hoehenmass_name(feld).is_some_and(|n| benannt(m, n))
                })
                .or_else(|| frei(gebunden, geometrie, &passend));
                vergeben.extend(markierung);
                let gruppe = match geschoss_id
                    .and_then(|s| g.geschosse.iter().enumerate().find(|(_, x)| x.id == s))
                {
                    Some((i, s)) => format!(
                        "{} · {}",
                        g.bezeichnung,
                        if s.bezeichnung.trim().is_empty() {
                            format!("Geschoss {}", i + 1)
                        } else {
                            s.bezeichnung.clone()
                        }
                    ),
                    None => g.bezeichnung.clone(),
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "gebaeude".into(),
                    objekt: id.to_string(),
                    objekt_name: g.bezeichnung.clone(),
                    gruppe,
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: Some(id),
                    geschoss_id,
                    grundstueck_id: None,
                    flurstueck_id: None,
                    wert_vorhanden: gefuellt(&f.daten, &z.ziel),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Abstandsflaechen).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Abstandsplan-ID ungültig"))?;
            let p: AbstandsPlan =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            for z in abstandsziele(&p) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let Some(w) = p.waende.iter().find(|w| w.id.to_string() == t[1]) else {
                    continue;
                };
                let (bedeutung, geometrie, hinweis) = wandfeld(t[2]);
                let gebunden = p
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let markierung = frei(gebunden, geometrie, &|m| {
                    m.gebaeude_id == w.gebaeude_id && m.bedeutung == bedeutung
                });
                let wert = match t[2] {
                    "gelaende_a_m" => w.gelaende_a_m,
                    "gelaende_b_m" => w.gelaende_b_m,
                    "abschluss_a_m" => w.abschluss_a_m,
                    "abschluss_b_m" => w.abschluss_b_m,
                    "dachhoehe_m" => w.dachhoehe_m,
                    "dachneigung_grad" => w.dachneigung_grad,
                    _ => w.dachneigung_b_grad,
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "abstand".into(),
                    objekt: id.to_string(),
                    objekt_name: p.bezeichnung.clone(),
                    gruppe: format!("{} · {}", p.bezeichnung, w.bezeichnung),
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: w.gebaeude_id,
                    geschoss_id: None,
                    grundstueck_id: w.grundstueck_id,
                    flurstueck_id: None,
                    wert_vorhanden: wert.is_some(),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        Ok(aufgaben)
    }
}
