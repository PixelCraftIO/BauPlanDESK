//! Gebäudeumrisse erzeugen und führen ihre Wände: Lage, Außenseite und
//! Geländehöhen an den Ecken kommen aus dem Umriss, alles andere bleibt an der Wand.
use super::{hoehen::TOL, *};

/// Vom Umriss gesetzte Höhenpunkte erkennt der Abgleich an ihrer Quelle.
const ECKE: &str = "Gebäudeumriss, Ecke ";
const VORGABE: &str = "Gebäudeumriss, Vorgabe Wandoberkante";
fn vom_umriss(p: &Hoehenpunkt) -> bool {
    p.quelle.starts_with(ECKE) || p.quelle == VORGABE
}
/// Ergebnis des Umrissabgleichs für die Oberfläche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct UmrissAbgleich {
    pub plan: AbstandsPlan,
    pub hinweise: Vec<String>,
}
/// Abgleich während der Bearbeitung: prüft nur die Umrisse, nicht den ganzen
/// (noch unvollständigen) Plan.
pub fn umrisse_abgleichen(mut plan: AbstandsPlan) -> CoreResult<UmrissAbgleich> {
    if plan.umrisse.len() > 50
        || plan.umrisse.iter().any(|u| u.ecken.len() > 500)
        || plan.waende.len() > 200
    {
        return Err(err("Höchstens 50 Umrisse mit je 500 Ecken und 200 Wände"));
    }
    if plan
        .umrisse
        .iter()
        .flat_map(|u| &u.ecken)
        .any(|e| geometrie::punkt(&APunkt { x: e.x, y: e.y }).is_err())
    {
        return Err(err("Umrissecken benötigen endliche Koordinaten in Metern"));
    }
    let hinweise = umrisse_einbinden(&mut plan);
    if plan.waende.len() > 200 {
        return Err(err("Höchstens 200 Wände"));
    }
    Ok(UmrissAbgleich { plan, hinweise })
}
/// Gleicht die Wände aller Umrisse ab und liefert Hinweise auf Änderungen,
/// die der Anwender prüfen sollte.
pub fn umrisse_einbinden(plan: &mut AbstandsPlan) -> Vec<String> {
    let mut hinweise = vec![];
    // Wände ohne gültige Umrissseite entfernen: Ihre Lage gibt es nicht mehr.
    plan.waende.retain(|w| match &w.umriss {
        Some(s)
            if !plan
                .umrisse
                .iter()
                .any(|u| u.id == s.umriss_id && s.seite < u.ecken.len()) =>
        {
            hinweise.push(format!(
                "{}: Umrissseite entfällt, Wand entfernt",
                w.bezeichnung
            ));
            false
        }
        _ => true,
    });
    for u in &plan.umrisse {
        let n = u.ecken.len();
        if n < 3 {
            continue;
        }
        // Gaußsche Flächenformel: positiv bei Umlauf gegen den Uhrzeigersinn,
        // dann liegt das Gebäude links jeder Seite und außen ist rechts.
        let flaeche: f64 = (0..n)
            .map(|i| {
                let (a, b) = (&u.ecken[i], &u.ecken[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum();
        let aussen = if flaeche > 0. {
            Aussenrichtung::Rechts
        } else {
            Aussenrichtung::Links
        };
        for i in 0..n {
            let (a, b) = (&u.ecken[i], &u.ecken[(i + 1) % n]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            let seite = UmrissSeite {
                umriss_id: u.id,
                seite: i,
            };
            let pos = match plan
                .waende
                .iter()
                .position(|w| w.umriss.as_ref() == Some(&seite))
            {
                Some(pos) => pos,
                None => {
                    plan.waende.push(neue_wand(u, i));
                    plan.waende.len() - 1
                }
            };
            let w = &mut plan.waende[pos];
            let neu_lage = (APunkt { x: a.x, y: a.y }, APunkt { x: b.x, y: b.y });
            if (w.von.clone(), w.bis.clone()) != neu_lage
                && w.gelaende
                    .iter()
                    .chain(&w.oberkante)
                    .any(|p| !vom_umriss(p) && p.station_m > TOL && p.station_m < l - TOL)
            {
                hinweise.push(format!(
                    "{}: Lage geändert – Zwischenpunkte des Höhenprofils prüfen",
                    w.bezeichnung
                ));
            }
            (w.von, w.bis) = neu_lage;
            w.aussen = aussen.clone();
            w.grundstueck_id = u.grundstueck_id;
            w.gebaeude_id = u.gebaeude_id;
            if u.gebaeude_id.is_none() {
                w.gebaeude = u.bezeichnung.clone();
            }
            // Das Profil ersetzt die Einzelwerte älterer Pläne.
            if w.gelaende.is_empty()
                && let (Some(ga), Some(gb)) = (w.gelaende_a_m, w.gelaende_b_m)
            {
                w.gelaende = vec![punkt(0., ga, ""), punkt(l, gb, "")];
            }
            if w.oberkante.is_empty()
                && let (Some(oa), Some(ob)) = (w.abschluss_a_m, w.abschluss_b_m)
            {
                w.oberkante = vec![punkt(0., oa, ""), punkt(l, ob, "")];
            }
            (
                w.gelaende_a_m,
                w.gelaende_b_m,
                w.abschluss_a_m,
                w.abschluss_b_m,
            ) = (None, None, None, None);
            // Eckhöhen und Oberkantenvorgabe führt der Umriss und setzt sie
            // nach jeder Änderung neu; eigene Punkte der Wand bleiben.
            w.gelaende.retain(|p| !vom_umriss(p));
            for (ecke, station, h) in [(i, 0., a.gelaende_m), ((i + 1) % n, l, b.gelaende_m)] {
                let Some(h) = h else { continue };
                w.gelaende.retain(|p| (p.station_m - station).abs() > TOL);
                w.gelaende
                    .push(punkt(station, h, &format!("{ECKE}{}", ecke + 1)));
            }
            w.gelaende
                .sort_by(|x, y| x.station_m.total_cmp(&y.station_m));
            let vorgabe = w.oberkante.iter().any(vom_umriss);
            w.oberkante.retain(|p| !vom_umriss(p));
            if let Some(o) = u.oberkante_m
                && (vorgabe || w.oberkante.is_empty())
            {
                w.oberkante = vec![punkt(0., o, VORGABE), punkt(l, o, VORGABE)];
            }
        }
    }
    hinweise
}
fn punkt(station_m: f64, hoehe_m: f64, quelle: &str) -> Hoehenpunkt {
    Hoehenpunkt {
        station_m,
        hoehe_m,
        quelle: quelle.into(),
    }
}
fn neue_wand(u: &AbstandsUmriss, i: usize) -> AbstandsWand {
    AbstandsWand {
        // Stabil je Umriss und Seite, damit wiederholtes Einbinden dieselbe Wand trifft.
        id: Uuid::new_v5(&u.id, format!("seite-{i}").as_bytes()),
        gebaeude_id: u.gebaeude_id,
        gebaeude: u.bezeichnung.clone(),
        bezeichnung: format!("{} – Seite {}", u.bezeichnung, i + 1),
        grundstueck_id: u.grundstueck_id,
        von: APunkt { x: 0., y: 0. },
        bis: APunkt { x: 0., y: 0. },
        aussen: Aussenrichtung::Rechts,
        gelaende_a_m: None,
        gelaende_b_m: None,
        abschluss_a_m: None,
        abschluss_b_m: None,
        dachart: Dachart::Ungeklaert,
        dachhoehe_m: None,
        dachneigung_grad: None,
        dachfaktor_manuell: None,
        hoehenquelle: u.quelle.clone(),
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
        umriss: Some(UmrissSeite {
            umriss_id: u.id,
            seite: i,
        }),
        breite_aus_ansicht: false,
        ansicht_laenge_m: None,
        grenzabstand_m: None,
    }
}
