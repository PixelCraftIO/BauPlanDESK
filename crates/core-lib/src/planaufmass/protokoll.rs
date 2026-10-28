//! Nachvollziehbarkeit im Projektprotokoll: Welche Werte wurden beim Speichern
//! mit dem Aufmaß verknüpft, aktualisiert oder von ihm gelöst, mit altem und
//! neuem Wert. Arbeitet auf den gespeicherten JSON-Ständen, damit Fachobjekte
//! und Projektstand dieselbe Auswertung nutzen.
use serde_json::Value;
use std::collections::BTreeMap;

struct Eintrag {
    markierung: String,
    revision: i64,
    wert: Option<f64>,
}

fn feld<'a>(mut v: &'a Value, pfad: &str) -> Option<&'a Value> {
    for teil in pfad.split('.') {
        v = match v {
            Value::Array(a) => a
                .iter()
                .find(|x| x.get("id").and_then(Value::as_str) == Some(teil))?,
            Value::Object(o) => o.get(teil)?,
            _ => return None,
        };
    }
    Some(v)
}

fn bindung(b: &Value) -> Option<(String, String, i64)> {
    Some((
        b.get("ziel")?.as_str()?.to_string(),
        b.get("markierung_id")?.as_str()?.to_string(),
        b.get("revision")?.as_i64()?,
    ))
}

fn sammeln(v: &Value, aus: &mut BTreeMap<String, Eintrag>) {
    match v {
        Value::Array(a) => a.iter().for_each(|x| sammeln(x, aus)),
        Value::Object(o) => {
            if let Some((ziel, markierung, revision)) = o.get("planbindung").and_then(bindung) {
                let wert = ziel
                    .rsplit('.')
                    .next()
                    .and_then(|f| o.get(f))
                    .and_then(Value::as_f64);
                aus.insert(
                    ziel,
                    Eintrag {
                        markierung,
                        revision,
                        wert,
                    },
                );
            }
            for b in o
                .get("planbindungen")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some((ziel, markierung, revision)) = bindung(b) {
                    let wert = feld(v, &ziel)
                        .or_else(|| o.get("eingaben").and_then(|e| feld(e, &ziel)))
                        .and_then(Value::as_f64);
                    aus.insert(
                        ziel,
                        Eintrag {
                            markierung,
                            revision,
                            wert,
                        },
                    );
                }
            }
            for (k, x) in o {
                if k != "planbindung" && k != "planbindungen" {
                    sammeln(x, aus);
                }
            }
        }
        _ => {}
    }
}

/// Wert eines noch ungebundenen Felds: direkt am Pfad oder unter `eingaben`,
/// auch in verschachtelten Objekten (Ermittlung → Ansätze → Abzüge).
fn handwert(v: &Value, ziel: &str) -> Option<f64> {
    if let Some(w) = feld(v, ziel)
        .or_else(|| v.get("eingaben").and_then(|e| feld(e, ziel)))
        .and_then(Value::as_f64)
    {
        return Some(w);
    }
    match v {
        Value::Array(a) => a.iter().find_map(|x| handwert(x, ziel)),
        Value::Object(o) => o.values().find_map(|x| handwert(x, ziel)),
        _ => None,
    }
}

fn zahl(w: Option<f64>) -> String {
    match w {
        Some(w) => {
            let t = format!("{:.2}", w);
            let t = t.trim_end_matches('0').trim_end_matches('.');
            t.replace('.', ",")
        }
        None => "—".into(),
    }
}

/// Zusammenfassung der Aufmaßänderungen zwischen zwei Ständen; `None`, wenn
/// sich an Verknüpfungen und verknüpften Werten nichts geändert hat.
pub fn planaenderungen(alt: Option<&Value>, neu: &Value) -> Option<String> {
    let (mut a, mut n) = (BTreeMap::new(), BTreeMap::new());
    if let Some(alt) = alt {
        sammeln(alt, &mut a);
    }
    sammeln(neu, &mut n);
    let mut zeilen = Vec::new();
    for (ziel, x) in &n {
        match a.get(ziel) {
            None => {
                // Vorheriger Handwert desselben Felds, sofern auffindbar.
                let vorher = alt.and_then(|alt| handwert(alt, ziel));
                zeilen.push(format!(
                    "{ziel} verknüpft ({} → {})",
                    zahl(vorher),
                    zahl(x.wert)
                ))
            }
            Some(y) if y.markierung != x.markierung => zeilen.push(format!(
                "{ziel} neu verknüpft ({} → {})",
                zahl(y.wert),
                zahl(x.wert)
            )),
            Some(y) if y.revision != x.revision || y.wert != x.wert => zeilen.push(format!(
                "{ziel} auf Aufmaßstand R{} aktualisiert ({} → {})",
                x.revision,
                zahl(y.wert),
                zahl(x.wert)
            )),
            Some(_) => {}
        }
    }
    for (ziel, y) in &a {
        if !n.contains_key(ziel) {
            zeilen.push(format!(
                "{ziel} von Aufmaß gelöst (Wert {} bleibt manuell)",
                zahl(y.wert)
            ));
        }
    }
    if zeilen.is_empty() {
        return None;
    }
    let mehr = zeilen.len().saturating_sub(10);
    zeilen.truncate(10);
    if mehr > 0 {
        zeilen.push(format!("und {mehr} weitere"));
    }
    Some(format!("Aufmaß: {}", zeilen.join("; ")))
}

#[cfg(test)]
mod pruefungen {
    use super::*;
    use serde_json::json;
    #[test]
    fn verknuepfen_aktualisieren_und_loesen_werden_genannt() {
        let m = "8f0c6d55-0000-4000-8000-000000000001";
        let ohne = json!({"bauteile":[{"id":"z","flaeche_m2":48.0}]});
        let mit = |r: i64, w: f64| {
            json!({"bauteile":[{"id":"z","flaeche_m2":w,
            "planbindung":{"ziel":"bauteile.z.flaeche_m2","markierung_id":m,"revision":r}}]})
        };
        let t = planaenderungen(Some(&ohne), &mit(1, 50.)).unwrap();
        assert!(
            t.contains("bauteile.z.flaeche_m2 verknüpft (48 → 50)"),
            "{t}"
        );
        assert!(planaenderungen(Some(&mit(1, 50.)), &mit(1, 50.)).is_none());
        let t = planaenderungen(Some(&mit(1, 50.)), &mit(2, 50.5)).unwrap();
        assert!(t.contains("R2 aktualisiert (50 → 50,5)"), "{t}");
        let t = planaenderungen(
            Some(&mit(2, 50.5)),
            &json!({"bauteile":[{"id":"z","flaeche_m2":50.5}]}),
        )
        .unwrap();
        assert!(t.contains("gelöst (Wert 50,5 bleibt manuell)"), "{t}");
        // Fachobjekte: Bindungsliste am Objekt, Ziel in den Eingaben.
        let plan = json!({"eingaben":{"positionen":[{"id":"p","flaeche_m2":12.0}]},
            "planbindungen":[{"ziel":"positionen.p.flaeche_m2","markierung_id":m,"revision":3}]});
        let t = planaenderungen(None, &plan).unwrap();
        assert!(
            t.contains("positionen.p.flaeche_m2 verknüpft (— → 12)"),
            "{t}"
        );
    }
}
