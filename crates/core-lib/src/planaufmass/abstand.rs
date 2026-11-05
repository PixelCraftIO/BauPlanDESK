use super::*;
use crate::{
    CoreServices,
    abstandsflaechen::{
        APunkt, AbstandsGrundstueck, AbstandsPlan, AbstandsWand, Aussenrichtung, Dachart, Wandfall,
    },
    domain::ProjectId,
    planaufmass::grundstueck::Aufmassstand,
};

/// Ergebnis der Übernahme der Abstandsflächen-Linien eines Lageplans.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct AbstandslinienUebernahme {
    pub plan: AbstandsPlan,
    pub hinweise: Vec<String>,
}

/// Lage einer Wand (Linie „Abstandsfläche“) bzw. Grenze eines Grundstücks
/// (Markierung „Flurstück“) aus dem Lageplan.
const LAGE: &str = "lage";
const GRENZE: &str = "grenze";
/// Wandbezug in Ansicht oder Schnitt: Linie „Abstandsfläche“ zwischen den
/// Gebäudekanten; liefert die Breite in der Ansicht und später die Stationen
/// der Höhen.
const ANSICHT: &str = "ansicht";
fn ansichtsziel(b: &Planbindung) -> bool {
    b.ziel.ends_with(&format!(".{ANSICHT}"))
}
/// Liegt die Markierung in einer Ansicht, einem Schnitt oder Detail?
fn in_ansicht(a: &Planaufmass, m: &Planmarkierung) -> bool {
    m.ausschnitt_id
        .and_then(|id| a.ausschnitte.iter().find(|x| x.id == id))
        .is_some_and(|x| x.art.mit_hoehen())
}
fn geometrieziel(b: &Planbindung) -> bool {
    b.ziel.ends_with(&format!(".{LAGE}")) || b.ziel.ends_with(&format!(".{GRENZE}"))
}

/// Seitenpunkte in Meter im Koordinatenbezug des Lageplans: Maßstab bzw.
/// Zeichnungseinheit des Ausschnitts oder Blattes, y nach oben.
fn meterpunkte(a: &Planaufmass, m: &Planmarkierung, ps: &[Planpunkt]) -> CoreResult<Vec<APunkt>> {
    let ausschnitt = m
        .ausschnitt_id
        .and_then(|id| a.ausschnitte.iter().find(|x| x.id == id));
    if ausschnitt.is_some_and(|x| x.art.mit_hoehen()) {
        return Err(fehler(format!(
            "{}: Lage und Grenzen im Lageplan oder Grundriss zeichnen, nicht in Ansicht oder Schnitt",
            m.bezeichnung
        )));
    }
    let f = match ausschnitt
        .and_then(|x| x.kalibrierung.clone())
        .or_else(|| a.kalibrierung_wirksam())
    {
        Some(k) => k.meter_pro_punkt()?,
        None => {
            return Err(fehler(
                "Für die Lage aus dem Lageplan zuerst Maßstab bzw. Zeichnungseinheit festlegen",
            ));
        }
    };
    Ok(ps
        .iter()
        .map(|p| APunkt {
            x: p[0] * f,
            y: -p[1] * f,
        })
        .collect())
}
fn strecke_m(a: &Planaufmass, m: &Planmarkierung) -> CoreResult<(APunkt, APunkt)> {
    if m.bedeutung != Planbedeutung::Abstandsflaeche {
        return Err(fehler(format!(
            "{}: Lage nur aus einer Linie mit Bedeutung „Abstandsfläche“",
            m.bezeichnung
        )));
    }
    let Plangeometrie::Strecke { von, bis } = &m.geometrie else {
        return Err(fehler(format!(
            "{}: Eine Abstandsfläche ist eine Linie (Strecke) entlang der Außenwand",
            m.bezeichnung
        )));
    };
    let mut ps = meterpunkte(a, m, &[*von, *bis])?.into_iter();
    Ok((ps.next().unwrap(), ps.next().unwrap()))
}
/// Umriss einer Fläche in Seitenpunkten (Außenring, Aussparungen).
fn ring(m: &Planmarkierung) -> Option<(Vec<Planpunkt>, Vec<Vec<Planpunkt>>)> {
    match &m.geometrie {
        Plangeometrie::Polygon { punkte } => Some((punkte.clone(), vec![])),
        Plangeometrie::Rechteck { von, bis } => {
            Some((vec![*von, [bis[0], von[1]], *bis, [von[0], bis[1]]], vec![]))
        }
        Plangeometrie::Flaeche { polygone } if polygone.len() == 1 => {
            Some((polygone[0].aussen.clone(), polygone[0].innen.clone()))
        }
        _ => None,
    }
}
fn ring_m(a: &Planaufmass, m: &Planmarkierung) -> CoreResult<(Vec<APunkt>, Vec<Vec<APunkt>>)> {
    if m.bedeutung != Planbedeutung::Flurstueck {
        return Err(fehler(format!(
            "{}: Grundstücksgrenze nur aus einer Markierung „Flurstück“",
            m.bezeichnung
        )));
    }
    let (aussen, innen) = ring(m).ok_or_else(|| {
        fehler(format!(
            "{}: Grundstücksgrenze als einzelne Fläche zeichnen",
            m.bezeichnung
        ))
    })?;
    Ok((
        meterpunkte(a, m, &aussen)?,
        innen
            .iter()
            .map(|r| meterpunkte(a, m, r))
            .collect::<CoreResult<_>>()?,
    ))
}
/// Punkt im Polygon (gerade-ungerade-Regel).
fn innen(p: &APunkt, ring: &[APunkt]) -> bool {
    let mut drin = false;
    for i in 0..ring.len() {
        let (a, b) = (&ring[i], &ring[(i + ring.len() - 1) % ring.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            drin = !drin;
        }
    }
    drin
}
/// Neue Wand zu einer Linie; die ID folgt der Markierung, damit eine erneute
/// Übernahme dieselbe Wand trifft.
fn linienwand(m: &Planmarkierung) -> AbstandsWand {
    AbstandsWand {
        id: Uuid::new_v5(&m.id, b"abstandsflaeche"),
        gebaeude_id: m.gebaeude_id,
        gebaeude: String::new(),
        bezeichnung: m.bezeichnung.clone(),
        grundstueck_id: None,
        von: APunkt { x: 0., y: 0. },
        bis: APunkt { x: 0., y: 0. },
        aussen: Aussenrichtung::Links,
        gelaende_a_m: None,
        gelaende_b_m: None,
        abschluss_a_m: None,
        abschluss_b_m: None,
        dachart: Dachart::Ungeklaert,
        dachhoehe_m: None,
        dachneigung_grad: None,
        dachfaktor_manuell: None,
        hoehenquelle: String::new(),
        dachgrundlage: String::new(),
        fall: Wandfall::Regel,
        grenzkennung: String::new(),
        nutzung_geprueft: false,
        sonderansatz: None,
        polygon_manuell: None,
        gelaende: vec![],
        oberkante: vec![],
        wandteilgrenzen: vec![],
        dachneigung_b_grad: None,
        gauben_laenge_m: None,
        tiefenfall: None,
        tiefenfall_grundlage: String::new(),
        umriss: None,
        breite_aus_ansicht: false,
        ansicht_laenge_m: None,
        grenzabstand_m: None,
    }
}

fn zielgroesse(feld: &str) -> Option<Planmessgroesse> {
    match feld {
        LAGE => Some(Planmessgroesse::Laenge),
        ANSICHT => Some(Planmessgroesse::Horizontale),
        GRENZE => Some(Planmessgroesse::Flaeche),
        "gelaende_a_m" | "gelaende_b_m" | "abschluss_a_m" | "abschluss_b_m" | "dachhoehe_m" => {
            Some(Planmessgroesse::Hoehe)
        }
        "dachneigung_grad" | "dachneigung_b_grad" => Some(Planmessgroesse::Neigung),
        _ => None,
    }
}
pub fn abstandsziele(plan: &AbstandsPlan) -> Vec<Planziel> {
    plan.grundstuecke
        .iter()
        .map(|g| Planziel {
            ziel: format!("grundstuecke.{}.{GRENZE}", g.id),
            titel: format!("{} · Grenze aus dem Lageplan (Flurstück)", g.bezeichnung),
            groesse: Planmessgroesse::Flaeche,
        })
        // Auch Umrisswände bekommen ihre Ansicht.
        .chain(plan.waende.iter().map(|w| Planziel {
            ziel: format!("waende.{}.{ANSICHT}", w.id),
            titel: format!(
                "{} · Wandbezug in Ansicht/Schnitt (Linie Abstandsfläche)",
                w.bezeichnung
            ),
            groesse: Planmessgroesse::Horizontale,
        }))
        .chain(
            plan.waende
                .iter()
                .filter(|w| w.umriss.is_none())
                .flat_map(|w| {
                    [
                        (LAGE, "Lage aus dem Lageplan (Linie Abstandsfläche)"),
                        ("gelaende_a_m", "Gelände am Wandanfang"),
                        ("gelaende_b_m", "Gelände am Wandende"),
                        ("abschluss_a_m", "Wandabschluss am Anfang"),
                        ("abschluss_b_m", "Wandabschluss am Ende"),
                        ("dachhoehe_m", "Dachhöhe über Wandabschluss"),
                        ("dachneigung_grad", "Dachneigung"),
                        ("dachneigung_b_grad", "Dachneigung zweite Seite"),
                    ]
                    .into_iter()
                    .map(move |(feld, titel)| Planziel {
                        ziel: format!("waende.{}.{feld}", w.id),
                        titel: format!("{} · {titel}", w.bezeichnung),
                        groesse: zielgroesse(feld).unwrap(),
                    })
                }),
        )
        .collect()
}
fn herkunftstext(feld: &str, a: &Planaufmass, b: &Planbindung, m: &Planmarkierung) -> String {
    match &a.dokument {
        Some(d) => format!(
            "{feld}: Aufmaß „{}“ R{} · Markierung „{}“ · PDF V{} · Seite {}",
            a.bezeichnung, b.revision, m.bezeichnung, d.version, a.seite
        ),
        None => format!(
            "{feld}: Zeichnung „{}“ R{} · Markierung „{}“ · eigene Zeichnung in Metern",
            a.bezeichnung, b.revision, m.bezeichnung
        ),
    }
}

impl CoreServices {
    pub async fn planaufmass_abstand(
        &self,
        pid: ProjectId,
        mut plan: AbstandsPlan,
    ) -> CoreResult<AbstandsPlan> {
        self.planwerte_abstand(pid, &mut plan).await?;
        plan.umfang_geprueft = false;
        plan.recht.geprueft = false;
        Ok(plan)
    }
    pub(crate) async fn planwerte_abstand(
        &self,
        pid: ProjectId,
        plan: &mut AbstandsPlan,
    ) -> CoreResult<()> {
        if plan.planbindungen.len() > 500 {
            return Err(fehler("Zu viele Wandverknüpfungen"));
        }
        let mut seen = std::collections::HashSet::new();
        // Herkunft je Wand aus allen ihren Bindungen, nicht nur der letzten.
        let mut herkunft: std::collections::BTreeMap<Uuid, Vec<String>> = Default::default();
        // Lage und Grenzen teilen einen Koordinatenbezug: ein Aufmaß, ein Ausschnitt.
        let mut rahmen: Option<(Uuid, Option<Uuid>)> = None;
        let mut ansichtslinien = std::collections::HashSet::new();
        // Breite aus der Ansicht gilt nur, solange eine Ansicht verknüpft ist.
        for w in &mut plan.waende {
            if !plan
                .planbindungen
                .iter()
                .any(|b| b.ziel == format!("waende.{}.{ANSICHT}", w.id))
            {
                w.ansicht_laenge_m = None;
            }
        }
        for b in plan.planbindungen.clone() {
            if !seen.insert(b.ziel.clone()) {
                return Err(fehler("Wandfeld mehrfach verknüpft"));
            }
            let t: Vec<_> = b.ziel.split('.').collect();
            if t.len() != 3
                || !(t[0] == "waende" || (t[0] == "grundstuecke" && t[2] == GRENZE))
                || zielgroesse(t[2]) != Some(b.groesse)
            {
                return Err(fehler("Ungültiges Wandfeld / Messgröße"));
            }
            let (a, m, wert) = self.planbindung_aufloesen(pid, &b).await?;
            if t[2] == ANSICHT {
                let w = plan
                    .waende
                    .iter_mut()
                    .find(|w| w.id.to_string() == t[1])
                    .ok_or_else(|| fehler("Zielwand fehlt"))?;
                if m.bedeutung != Planbedeutung::Abstandsflaeche
                    || !matches!(m.geometrie, Plangeometrie::Strecke { .. })
                    || !in_ansicht(&a, &m)
                {
                    return Err(fehler(format!(
                        "{}: Wandbezug ist eine Linie „Abstandsfläche“ in einer Ansicht oder einem Schnitt",
                        w.bezeichnung
                    )));
                }
                if m.gebaeude_id.is_some() && m.gebaeude_id != w.gebaeude_id {
                    return Err(fehler(format!(
                        "{}: Linie in der Ansicht gehört zu einem anderen Gebäude",
                        w.bezeichnung
                    )));
                }
                if !ansichtslinien.insert(m.id) {
                    return Err(fehler(format!(
                        "Linie „{}“ in der Ansicht gehört schon zu einer anderen Wand",
                        m.bezeichnung
                    )));
                }
                w.ansicht_laenge_m = Some(wert);
                if w.breite_aus_ansicht {
                    (w.von, w.bis) = (APunkt { x: 0., y: 0. }, APunkt { x: wert, y: 0. });
                }
                herkunft
                    .entry(w.id)
                    .or_default()
                    .push(herkunftstext("Ansicht", &a, &b, &m));
                if let Some(d) = a.dokument
                    && !plan.quellen.contains(&d)
                {
                    plan.quellen.push(d);
                }
                continue;
            }
            if geometrieziel(&b) {
                let r = (b.aufmass_id, m.ausschnitt_id);
                if *rahmen.get_or_insert(r) != r {
                    return Err(fehler(
                        "Lage und Grenzen aus genau einem Lageplan (Aufmaß und Ausschnitt) übernehmen; sonst passen die Koordinaten nicht zueinander",
                    ));
                }
                if t[0] == "grundstuecke" {
                    let g = plan
                        .grundstuecke
                        .iter_mut()
                        .find(|g| g.id.to_string() == t[1])
                        .ok_or_else(|| fehler("Zielgrundstück fehlt"))?;
                    if let Some(f) = &m.flurstueck_id
                        && !g.flurstueck_ids.is_empty()
                        && !g.flurstueck_ids.contains(f)
                    {
                        return Err(fehler(format!(
                            "{}: Flurstück der Markierung gehört nicht zu diesem Grundstück",
                            g.bezeichnung
                        )));
                    }
                    (g.grenze, g.innenringe) = ring_m(&a, &m)?;
                    g.grundlage = herkunftstext("Grenze", &a, &b, &m);
                } else {
                    let w = plan
                        .waende
                        .iter_mut()
                        .find(|w| w.id.to_string() == t[1])
                        .ok_or_else(|| fehler("Zielwand fehlt"))?;
                    if w.umriss.is_some() {
                        return Err(fehler("Umrisswände erhalten ihre Lage vom Gebäudeumriss"));
                    }
                    if m.gebaeude_id != w.gebaeude_id {
                        return Err(fehler("Linie gehört zu einem anderen Gebäude"));
                    }
                    (w.von, w.bis) = strecke_m(&a, &m)?;
                    // Linie im Lageplan nachgezogen: Lage gilt, Breiten werden verglichen.
                    w.breite_aus_ansicht = false;
                    w.grenzabstand_m = None;
                    herkunft
                        .entry(w.id)
                        .or_default()
                        .push(herkunftstext("Lage", &a, &b, &m));
                }
                if let Some(d) = a.dokument
                    && !plan.quellen.contains(&d)
                {
                    plan.quellen.push(d);
                }
                continue;
            }
            let w = plan
                .waende
                .iter_mut()
                .find(|w| w.id.to_string() == t[1])
                .ok_or_else(|| fehler("Zielwand fehlt"))?;
            if w.umriss.is_some() {
                return Err(fehler(
                    "Umrisswände erhalten ihre Eckhöhen vom Gebäudeumriss",
                ));
            }
            if m.gebaeude_id != w.gebaeude_id {
                return Err(fehler("Höhenquelle gehört zu einem anderen Gebäude"));
            }
            if let Some(grund) = m.grundstueck_id {
                let zid = w
                    .grundstueck_id
                    .ok_or_else(|| fehler("Wandgrundstück fehlt"))?;
                if zid != grund {
                    let projekt = self.get_project(pid).await?;
                    let kontexte = crate::grundstueckskontext::ermitteln(&projekt)?;
                    let g = kontexte
                        .iter()
                        .find(|g| g.id == grund)
                        .ok_or_else(|| fehler("Quellgrundstück fehlt"))?;
                    let ziel = plan
                        .grundstuecke
                        .iter()
                        .find(|g| g.id == zid)
                        .ok_or_else(|| fehler("Wandgrundstück fehlt"))?;
                    if ziel.flurstueck_ids.is_empty()
                        || !ziel
                            .flurstueck_ids
                            .iter()
                            .all(|id| g.flurstueck_ids.contains(id))
                    {
                        return Err(fehler(
                            "Quellgrundstück und Wandgrundstück besitzen keine übereinstimmende Flurstückszuordnung",
                        ));
                    }
                }
            }
            let absolut = matches!(
                t[2],
                "gelaende_a_m" | "gelaende_b_m" | "abschluss_a_m" | "abschluss_b_m"
            );
            if absolut {
                if !matches!(m.geometrie, Plangeometrie::Hoehenpunkt { .. })
                    || a.hoehensystem(&m) != Some(plan.hoehenbezug.as_str())
                {
                    return Err(fehler(
                        "Wandhöhe benötigt einen absoluten Höhenpunkt im selben Höhensystem",
                    ));
                }
                if (t[2].starts_with("gelaende") && !w.gelaende.is_empty())
                    || (t[2].starts_with("abschluss") && !w.oberkante.is_empty())
                {
                    return Err(fehler(
                        "Vorhandenes Wandprofil zuerst ausdrücklich entfernen; Eckwerte dürfen es nicht verdeckt übersteuern",
                    ));
                }
            } else if !matches!(m.geometrie, Plangeometrie::Strecke { .. }) {
                return Err(fehler("Dachhöhe / Neigung benötigt eine gemessene Strecke"));
            }
            match t[2] {
                "gelaende_a_m" => w.gelaende_a_m = Some(wert),
                "gelaende_b_m" => w.gelaende_b_m = Some(wert),
                "abschluss_a_m" => w.abschluss_a_m = Some(wert),
                "abschluss_b_m" => w.abschluss_b_m = Some(wert),
                "dachhoehe_m" => w.dachhoehe_m = Some(wert),
                "dachneigung_grad" => w.dachneigung_grad = Some(wert),
                "dachneigung_b_grad" => w.dachneigung_b_grad = Some(wert),
                _ => unreachable!(),
            }
            let feld = match t[2] {
                "gelaende_a_m" => "Gelände A",
                "gelaende_b_m" => "Gelände B",
                "abschluss_a_m" => "Wandabschluss A",
                "abschluss_b_m" => "Wandabschluss B",
                "dachhoehe_m" => "Dachhöhe",
                "dachneigung_grad" => "Dachneigung",
                _ => "Dachneigung B",
            };
            herkunft
                .entry(w.id)
                .or_default()
                .push(herkunftstext(feld, &a, &b, &m));
            if let Some(d) = a.dokument
                && !plan.quellen.contains(&d)
            {
                plan.quellen.push(d);
            }
        }
        for w in &mut plan.waende {
            if let Some(h) = herkunft.remove(&w.id) {
                w.hoehenquelle = format!("{} · {RECHENSTAND}", h.join("; "));
            }
        }
        Ok(())
    }
    /// Übernimmt alle Linien „Abstandsfläche“ und die zugeordneten
    /// Flurstücksgrenzen eines gespeicherten Lageplans in den Abstandsplan:
    /// je Linie eine Wand (A1, A2 … aus dem Namen der Linie), je Flurstück ein
    /// Grundstück. Lage und Grenzen bleiben an die Markierungen gebunden.
    /// Außenseite und Grundstück der Wand werden aus dem Lageplan abgeleitet,
    /// soweit er sie eindeutig zeigt; sonst meldet ein Hinweis die Prüfung.
    pub async fn abstandslinien_uebernehmen(
        &self,
        pid: ProjectId,
        mut plan: AbstandsPlan,
        stand: Aufmassstand,
    ) -> CoreResult<AbstandslinienUebernahme> {
        let a = self.aufmassstand(pid, stand).await?;
        let mut hinweise = vec![];
        let (ansichtslinien, linien): (Vec<&Planmarkierung>, Vec<&Planmarkierung>) = a
            .markierungen
            .iter()
            .filter(|m| m.bedeutung == Planbedeutung::Abstandsflaeche)
            .partition(|m| in_ansicht(&a, m));
        if linien.is_empty() && ansichtslinien.is_empty() {
            return Err(fehler(
                "Im Aufmaß gibt es keine Linie mit Bedeutung „Abstandsfläche“",
            ));
        }
        if !linien.is_empty()
            && let Some(b) = plan
                .planbindungen
                .iter()
                .find(|b| geometrieziel(b) && b.aufmass_id != stand.aufmass_id)
        {
            return Err(fehler(format!(
                "Lage und Grenzen stammen bereits aus einem anderen Lageplan ({}); ein Abstandsplan nutzt genau einen Lageplan als Koordinatenbezug",
                b.aufmass_id
            )));
        }
        let ausschnitt = linien.first().and_then(|m| m.ausschnitt_id);
        if linien.iter().any(|m| m.ausschnitt_id != ausschnitt) {
            return Err(fehler(
                "Alle Linien „Abstandsfläche“ im selben Ausschnitt bzw. auf dem Blatt zeichnen",
            ));
        }
        // Gelöschte Linien: Bindung lösen, die Wand behält ihre letzte Lage.
        plan.planbindungen.retain(|b| {
            let weg = b.aufmass_id == stand.aufmass_id
                && (geometrieziel(b) || ansichtsziel(b))
                && !a.markierungen.iter().any(|m| m.id == b.markierung_id);
            if weg {
                let name = plan
                    .waende
                    .iter()
                    .find(|w| b.ziel.starts_with(&format!("waende.{}.", w.id)))
                    .map(|w| w.bezeichnung.clone())
                    .or_else(|| {
                        plan.grundstuecke
                            .iter()
                            .find(|g| b.ziel == format!("grundstuecke.{}.{GRENZE}", g.id))
                            .map(|g| g.bezeichnung.clone())
                    })
                    .unwrap_or_default();
                hinweise.push(format!(
                    "{name}: Markierung im Lageplan gelöscht; Verknüpfung gelöst, letzte Lage bleibt als Eingabe"
                ));
            }
            !weg
        });
        // Bestehende Bindungen folgen dem übernommenen Stand des Lageplans.
        for b in plan
            .planbindungen
            .iter_mut()
            .filter(|b| b.aufmass_id == stand.aufmass_id && (geometrieziel(b) || ansichtsziel(b)))
        {
            b.revision = stand.revision;
        }
        let bindung = |m: &Planmarkierung, ziel: String, groesse| Planbindung {
            aufmass_id: stand.aufmass_id,
            revision: stand.revision,
            markierung_id: m.id,
            groesse,
            ziel,
        };
        // Grenzen aus den zugeordneten Flurstücken desselben Ausschnitts.
        for m in a.markierungen.iter().filter(|m| {
            !linien.is_empty()
                && m.bedeutung == Planbedeutung::Flurstueck
                && m.flurstueck_id.is_some()
                && m.ausschnitt_id == ausschnitt
        }) {
            if plan.planbindungen.iter().any(|b| b.markierung_id == m.id) {
                continue;
            }
            let f = m.flurstueck_id.clone().unwrap_or_default();
            let id = match plan
                .grundstuecke
                .iter()
                .find(|g| g.flurstueck_ids == [f.clone()])
            {
                Some(g) => g.id,
                None => {
                    let id = Uuid::new_v5(&m.id, b"grundstueck");
                    plan.grundstuecke.push(AbstandsGrundstueck {
                        id,
                        bezeichnung: m.bezeichnung.clone(),
                        flurstueck_ids: vec![f],
                        grenze: vec![],
                        innenringe: vec![],
                        grundlage: String::new(),
                    });
                    id
                }
            };
            plan.planbindungen.push(bindung(
                m,
                format!("grundstuecke.{id}.{GRENZE}"),
                Planmessgroesse::Flaeche,
            ));
        }
        let gebaeudenamen = self.gebaeudenamen(pid).await?;
        let gebaeudename = |m: &Planmarkierung| {
            m.gebaeude_id
                .and_then(|id| gebaeudenamen.iter().find(|(g, _)| *g == id))
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| "Gebäude".into())
        };
        let mut neu = vec![];
        for m in &linien {
            if plan.planbindungen.iter().any(|b| b.markierung_id == m.id) {
                continue;
            }
            let mut w = linienwand(m);
            w.gebaeude = gebaeudename(m);
            if plan.waende.iter().any(|x| x.id == w.id) {
                // Bindung früher gelöst: Wand wieder an die Linie binden.
            } else if let Some(x) = plan.waende.iter().find(|x| {
                x.bezeichnung == w.bezeichnung
                    && x.breite_aus_ansicht
                    && !plan
                        .planbindungen
                        .iter()
                        .any(|b| b.ziel == format!("waende.{}.{LAGE}", x.id))
            }) {
                // Wand bisher nur aus der Ansicht: Lage und Breite kommen jetzt
                // aus dem Lageplan, die Ansicht bleibt Wandbezug.
                w.id = x.id;
                neu.push(w.id);
            } else if let Some(x) = plan.waende.iter().find(|x| x.bezeichnung == w.bezeichnung) {
                return Err(fehler(format!(
                    "Die Wand „{}“ gibt es schon; Linie im Lageplan umbenennen, damit Bezeichnungen eindeutig bleiben",
                    x.bezeichnung
                )));
            } else {
                neu.push(w.id);
                plan.waende.push(w.clone());
            }
            plan.planbindungen.push(bindung(
                m,
                format!("waende.{}.{LAGE}", w.id),
                Planmessgroesse::Laenge,
            ));
        }
        // Linien in Ansichten: Wandbezug der gleichnamigen Wand, sonst eine
        // Wand mit Breite aus der Ansicht.
        for m in &ansichtslinien {
            if plan.planbindungen.iter().any(|b| b.markierung_id == m.id) {
                continue;
            }
            let ansicht = m
                .ausschnitt_id
                .and_then(|id| a.ausschnitte.iter().find(|x| x.id == id))
                .map(|x| x.bezeichnung.clone())
                .unwrap_or_default();
            let id = match plan.waende.iter().find(|w| w.bezeichnung == m.bezeichnung) {
                Some(w) => {
                    if plan
                        .planbindungen
                        .iter()
                        .any(|b| b.ziel == format!("waende.{}.{ANSICHT}", w.id))
                    {
                        hinweise.push(format!(
                            "{}: Wand hat schon eine Ansicht; Linie in „{ansicht}“ nicht verknüpft",
                            w.bezeichnung
                        ));
                        continue;
                    }
                    w.id
                }
                None => {
                    let mut w = linienwand(m);
                    w.gebaeude = gebaeudename(m);
                    w.breite_aus_ansicht = true;
                    hinweise.push(format!(
                        "{}: Breite aus „{ansicht}“, keine Linie im Lageplan; Abstand zur Grundstücksgrenze angeben (Lage wird nicht geprüft)",
                        w.bezeichnung
                    ));
                    let id = w.id;
                    plan.waende.push(w);
                    id
                }
            };
            plan.planbindungen.push(bindung(
                m,
                format!("waende.{id}.{ANSICHT}"),
                Planmessgroesse::Horizontale,
            ));
        }
        if plan.planbindungen.len() > 500 || plan.waende.len() > 200 {
            return Err(fehler("Höchstens 200 Wände und 500 Verknüpfungen"));
        }
        self.planwerte_abstand(pid, &mut plan).await?;
        // Gebäude im selben Ausschnitt zeigen die Innenseite.
        let gebaeude: Vec<Vec<APunkt>> = a
            .markierungen
            .iter()
            .filter(|m| {
                matches!(
                    m.bedeutung,
                    Planbedeutung::Vorhabenumriss | Planbedeutung::Gebaeudegrundflaeche
                ) && m.ausschnitt_id == ausschnitt
            })
            .filter_map(|m| ring(m).and_then(|(r, _)| meterpunkte(&a, m, &r).ok()))
            .filter(|ps| ps.len() >= 3)
            .collect();
        for w in plan.waende.iter_mut().filter(|w| neu.contains(&w.id)) {
            let (dx, dy) = (w.bis.x - w.von.x, w.bis.y - w.von.y);
            let l = dx.hypot(dy).max(1e-9);
            let mitte = APunkt {
                x: (w.von.x + w.bis.x) / 2.,
                y: (w.von.y + w.bis.y) / 2.,
            };
            // Prüfpunkt 10 cm links der Linie (Außenseite „links“).
            let links = APunkt {
                x: mitte.x - dy / l * 0.1,
                y: mitte.y + dx / l * 0.1,
            };
            let rechts = APunkt {
                x: mitte.x + dy / l * 0.1,
                y: mitte.y - dx / l * 0.1,
            };
            match (
                gebaeude.iter().any(|g| innen(&links, g)),
                gebaeude.iter().any(|g| innen(&rechts, g)),
            ) {
                (true, false) => w.aussen = Aussenrichtung::Rechts,
                (false, true) => w.aussen = Aussenrichtung::Links,
                _ => hinweise.push(format!(
                    "{}: Außenseite nicht aus einem Vorhabenumriss ableitbar; Seite in der Skizze prüfen",
                    w.bezeichnung
                )),
            }
            let auf: Vec<&AbstandsGrundstueck> = plan
                .grundstuecke
                .iter()
                .filter(|g| g.grenze.len() >= 3 && innen(&mitte, &g.grenze))
                .collect();
            match auf.as_slice() {
                [g] => w.grundstueck_id = Some(g.id),
                _ => {
                    if let [g] = plan.grundstuecke.as_slice() {
                        w.grundstueck_id = Some(g.id);
                    }
                    hinweise.push(format!(
                        "{}: Grundstück der Wand prüfen (Linie liegt nicht eindeutig in einer Flurstücksgrenze)",
                        w.bezeichnung
                    ));
                }
            }
        }
        if !linien.is_empty()
            && plan.grundstuecke.iter().any(|g| {
                !plan
                    .planbindungen
                    .iter()
                    .any(|b| b.ziel == format!("grundstuecke.{}.{GRENZE}", g.id))
            })
        {
            hinweise.push(
                "Grundstücke mit von Hand erfassten Grenzen liegen nicht im Koordinatenbezug des Lageplans; Flurstücke im Lageplan zeichnen und zuordnen"
                    .into(),
            );
        }
        let bezug = format!(
            "Lageplan „{}“{} · Meter, Blattmaßstab, y nach oben",
            a.bezeichnung,
            ausschnitt
                .and_then(|id| a.ausschnitte.iter().find(|x| x.id == id))
                .map(|x| format!(" › {}", x.bezeichnung))
                .unwrap_or_default()
        );
        if !linien.is_empty()
            && (plan.koordinatenbezug.trim().is_empty()
                || plan.koordinatenbezug.starts_with("Lageplan „"))
        {
            plan.koordinatenbezug = bezug;
        }
        plan.umfang_geprueft = false;
        plan.recht.geprueft = false;
        Ok(AbstandslinienUebernahme { plan, hinweise })
    }
    pub async fn abstands_planquellen_pruefen(
        &self,
        pid: ProjectId,
        plan: &AbstandsPlan,
    ) -> CoreResult<()> {
        let mut neu = plan.clone();
        self.planwerte_abstand(pid, &mut neu).await?;
        if neu != *plan {
            return Err(fehler(
                "Wandwerte weichen vom Aufmaß ab. Quellen erneut übernehmen und speichern.",
            ));
        }
        Ok(())
    }
}
