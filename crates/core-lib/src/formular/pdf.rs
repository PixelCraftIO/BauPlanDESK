//! AcroForm-Zugriff auf PDF-Dateien.

use std::collections::{BTreeMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feldart {
    Text,
    Ankreuzfeld,
    Optionsfeld,
    Auswahl,
    /// Schaltflächen und Signaturfelder: werden angezeigt, aber nicht befüllt.
    Sonstiges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Formularfeld {
    /// Vollqualifizierter Name (`Eltern.Kind`), eindeutig im Dokument.
    pub name: String,
    /// Tooltip (`/TU`), oft die lesbare Beschriftung im amtlichen Formular.
    pub beschriftung: Option<String>,
    pub art: Feldart,
    pub wert: Option<Feldwert>,
    /// Zulässige Werte bei Auswahl- und Optionsfeldern bzw. der
    /// "angekreuzt"-Zustand bei Ankreuzfeldern.
    pub optionen: Vec<String>,
    pub schreibgeschuetzt: bool,
    pub max_laenge: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Feldwert {
    Text(String),
    Ja(bool),
}

const FF_READ_ONLY: i64 = 1;
pub(crate) const FF_RADIO: i64 = 1 << 15;
const FF_PUSHBUTTON: i64 = 1 << 16;

fn pdf_err(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("PDF: {e}"))
}

/// Dekodiert einen PDF-Textstring (UTF-16BE mit BOM oder PDFDocEncoding,
/// hier vereinfacht als Latin-1, was alle deutschen Sonderzeichen abdeckt).
pub(crate) fn text_dekodieren(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Kodiert als PDFDocEncoding, wenn möglich, sonst als UTF-16BE mit BOM.
pub(crate) fn text_kodieren(text: &str) -> Object {
    let bytes = if text
        .chars()
        .all(|c| (c as u32) < 0x80 || ('\u{A0}'..='\u{FF}').contains(&c))
    {
        text.chars().map(|c| c as u8).collect()
    } else {
        let mut b = vec![0xFE, 0xFF];
        for unit in text.encode_utf16() {
            b.extend_from_slice(&unit.to_be_bytes());
        }
        b
    };
    Object::String(bytes, StringFormat::Literal)
}

/// Ein Feld im Feldbaum mit den geerbten Eigenschaften.
struct Knoten {
    id: ObjectId,
    name: String,
    ft: Option<Vec<u8>>,
    ff: i64,
    /// Widget-Annotationen, die das Feld darstellen (das Feld selbst, wenn
    /// Feld und Widget zusammenfallen).
    widgets: Vec<ObjectId>,
}

fn dict(doc: &Document, id: ObjectId) -> CoreResult<&Dictionary> {
    doc.get_dictionary(id).map_err(pdf_err)
}

fn felder_sammeln(doc: &Document) -> CoreResult<Vec<Knoten>> {
    let catalog = doc.catalog().map_err(pdf_err)?;
    let Ok(acro) = catalog.get(b"AcroForm") else {
        return Ok(Vec::new());
    };
    let (_, acro) = doc.dereference(acro).map_err(pdf_err)?;
    let acro = acro.as_dict().map_err(pdf_err)?;
    let Ok(fields) = acro.get(b"Fields") else {
        return Ok(Vec::new());
    };
    let (_, fields) = doc.dereference(fields).map_err(pdf_err)?;
    let mut out = Vec::new();
    let mut gesehen = HashSet::new();
    for f in fields.as_array().map_err(pdf_err)? {
        if let Ok(id) = f.as_reference() {
            besuchen(doc, id, "", None, 0, &mut out, &mut gesehen)?;
        }
    }
    Ok(out)
}

fn besuchen(
    doc: &Document,
    id: ObjectId,
    eltern: &str,
    ft: Option<Vec<u8>>,
    ff: i64,
    out: &mut Vec<Knoten>,
    gesehen: &mut HashSet<ObjectId>,
) -> CoreResult<()> {
    if !gesehen.insert(id) {
        return Ok(()); // Zyklen in defekten PDFs
    }
    let d = dict(doc, id)?;
    let teil = d
        .get(b"T")
        .ok()
        .and_then(|t| t.as_str().ok())
        .map(text_dekodieren);
    let name = match (&teil, eltern.is_empty()) {
        (Some(t), true) => t.clone(),
        (Some(t), false) => format!("{eltern}.{t}"),
        (None, _) => eltern.to_owned(),
    };
    let ft = d
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec)
        .or(ft);
    let ff = d
        .get(b"Ff")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(ff);

    let kinder: Vec<ObjectId> = d
        .get(b"Kids")
        .ok()
        .and_then(|k| doc.dereference(k).ok())
        .and_then(|(_, k)| k.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default();

    // Kinder mit eigenem /T sind Unterfelder, ohne /T nur Widgets dieses Felds.
    let (unterfelder, widgets): (Vec<_>, Vec<_>) = kinder
        .into_iter()
        .partition(|k| dict(doc, *k).map(|kd| kd.has(b"T")).unwrap_or(false));

    // Nur Blätter des Feldbaums tragen Werte.
    let ist_blatt = unterfelder.is_empty();
    for k in unterfelder {
        besuchen(doc, k, &name, ft.clone(), ff, out, gesehen)?;
    }
    if ist_blatt && teil.is_some() && (ft.is_some() || !widgets.is_empty()) {
        let widgets = if widgets.is_empty() {
            vec![id]
        } else {
            widgets
        };
        out.push(Knoten {
            id,
            name,
            ft,
            ff,
            widgets,
        });
    }
    Ok(())
}

/// Zustandsnamen außer `Off` aus `/AP /N` eines Widgets.
fn ein_zustaende(doc: &Document, widget: ObjectId) -> Vec<String> {
    let Ok(w) = dict(doc, widget) else {
        return Vec::new();
    };
    let Some(n) = w
        .get(b"AP")
        .ok()
        .and_then(|ap| doc.dereference(ap).ok())
        .and_then(|(_, ap)| ap.as_dict().ok())
        .and_then(|ap| ap.get(b"N").ok())
        .and_then(|n| doc.dereference(n).ok())
        .and_then(|(_, n)| n.as_dict().ok())
    else {
        return Vec::new();
    };
    n.iter()
        .map(|(k, _)| String::from_utf8_lossy(k).into_owned())
        .filter(|k| k != "Off")
        .collect()
}

fn feldart(k: &Knoten) -> Feldart {
    match k.ft.as_deref() {
        Some(b"Tx") => Feldart::Text,
        Some(b"Ch") => Feldart::Auswahl,
        Some(b"Btn") if k.ff & FF_PUSHBUTTON != 0 => Feldart::Sonstiges,
        Some(b"Btn") if k.ff & FF_RADIO != 0 => Feldart::Optionsfeld,
        Some(b"Btn") => Feldart::Ankreuzfeld,
        _ => Feldart::Sonstiges,
    }
}

fn lade(pdf: &[u8]) -> CoreResult<Document> {
    let doc = Document::load_mem(pdf).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(CoreError::InvalidInput(
            "verschlüsselte PDF-Formulare werden noch nicht unterstützt".into(),
        ));
    }
    Ok(doc)
}

/// Liest alle Formularfelder eines PDF.
pub fn felder_lesen(pdf: &[u8]) -> CoreResult<Vec<Formularfeld>> {
    let doc = lade(pdf)?;
    let mut felder = Vec::new();
    for k in felder_sammeln(&doc)? {
        let d = dict(&doc, k.id)?;
        let art = feldart(&k);
        let roh = d
            .get(b"V")
            .ok()
            .and_then(|v| doc.dereference(v).ok())
            .map(|(_, v)| v);
        let optionen = match art {
            Feldart::Auswahl => d
                .get(b"Opt")
                .ok()
                .and_then(|o| doc.dereference(o).ok())
                .and_then(|(_, o)| o.as_array().ok())
                .map(|a| {
                    a.iter()
                        .filter_map(|o| match o {
                            // [Exportwert Anzeigetext] oder nur Text
                            Object::Array(paar) => paar.first()?.as_str().ok().map(text_dekodieren),
                            o => o.as_str().ok().map(text_dekodieren),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Feldart::Ankreuzfeld | Feldart::Optionsfeld => {
                let mut z: Vec<String> = k
                    .widgets
                    .iter()
                    .flat_map(|w| ein_zustaende(&doc, *w))
                    .collect();
                z.dedup();
                z
            }
            _ => Vec::new(),
        };
        let wert = match (&art, roh) {
            (Feldart::Ankreuzfeld, Some(v)) => Some(Feldwert::Ja(
                v.as_name().map(|n| n != b"Off").unwrap_or(false),
            )),
            (Feldart::Optionsfeld, Some(v)) => v
                .as_name()
                .ok()
                .filter(|n| *n != b"Off")
                .map(|n| Feldwert::Text(String::from_utf8_lossy(n).into_owned())),
            (_, Some(v)) => v.as_str().ok().map(|s| Feldwert::Text(text_dekodieren(s))),
            (_, None) => None,
        };
        felder.push(Formularfeld {
            name: k.name.clone(),
            beschriftung: d
                .get(b"TU")
                .ok()
                .and_then(|t| t.as_str().ok())
                .map(text_dekodieren),
            art,
            wert,
            optionen,
            schreibgeschuetzt: k.ff & FF_READ_ONLY != 0,
            max_laenge: d
                .get(b"MaxLen")
                .ok()
                .and_then(|m| m.as_i64().ok())
                .map(|m| m as u32),
        });
    }
    Ok(felder)
}

/// Setzt Feldwerte und liefert das geänderte PDF. Unbekannte Feldnamen und
/// unpassende Werte sind Fehler, damit kein Formular unbemerkt unvollständig
/// eingereicht wird.
pub fn befuellen(pdf: &[u8], werte: &BTreeMap<String, Feldwert>) -> CoreResult<Vec<u8>> {
    let mut doc = lade(pdf)?;
    let knoten = felder_sammeln(&doc)?;
    for (name, wert) in werte {
        let k = knoten.iter().find(|k| &k.name == name).ok_or_else(|| {
            CoreError::InvalidInput(format!("Formularfeld {name:?} existiert nicht"))
        })?;
        if k.ff & FF_READ_ONLY != 0 {
            return Err(CoreError::InvalidInput(format!(
                "Formularfeld {name:?} ist schreibgeschützt"
            )));
        }
        match (feldart(k), wert) {
            (Feldart::Text | Feldart::Auswahl, Feldwert::Text(t)) => {
                let max = dict(&doc, k.id)?
                    .get(b"MaxLen")
                    .ok()
                    .and_then(|m| m.as_i64().ok());
                if let Some(max) = max
                    && t.chars().count() as i64 > max
                {
                    return Err(CoreError::InvalidInput(format!(
                        "Formularfeld {name:?}: höchstens {max} Zeichen"
                    )));
                }
                doc.get_dictionary_mut(k.id)
                    .map_err(pdf_err)?
                    .set("V", text_kodieren(t));
                // Alte Darstellung verwerfen, damit Viewer den neuen Wert zeichnen.
                for w in &k.widgets {
                    doc.get_dictionary_mut(*w).map_err(pdf_err)?.remove(b"AP");
                }
            }
            (Feldart::Ankreuzfeld, Feldwert::Ja(an)) => {
                for w in &k.widgets {
                    let zustand = if *an {
                        ein_zustaende(&doc, *w)
                            .into_iter()
                            .next()
                            .unwrap_or_else(|| "Yes".into())
                    } else {
                        "Off".into()
                    };
                    doc.get_dictionary_mut(*w)
                        .map_err(pdf_err)?
                        .set("AS", Object::Name(zustand.clone().into_bytes()));
                    doc.get_dictionary_mut(k.id)
                        .map_err(pdf_err)?
                        .set("V", Object::Name(zustand.into_bytes()));
                }
            }
            (Feldart::Optionsfeld, Feldwert::Text(option)) => {
                let mut gefunden = false;
                for w in &k.widgets {
                    let hat = ein_zustaende(&doc, *w).contains(option);
                    gefunden |= hat;
                    let zustand = if hat { option.as_str() } else { "Off" };
                    doc.get_dictionary_mut(*w)
                        .map_err(pdf_err)?
                        .set("AS", Object::Name(zustand.as_bytes().to_vec()));
                }
                if !gefunden {
                    return Err(CoreError::InvalidInput(format!(
                        "Formularfeld {name:?} kennt die Option {option:?} nicht"
                    )));
                }
                doc.get_dictionary_mut(k.id)
                    .map_err(pdf_err)?
                    .set("V", Object::Name(option.as_bytes().to_vec()));
            }
            (art, _) => {
                return Err(CoreError::InvalidInput(format!(
                    "Formularfeld {name:?} ({art:?}) passt nicht zum Wert"
                )));
            }
        }
    }

    // Viewer sollen die Darstellung der geänderten Felder neu erzeugen.
    let acro = doc
        .catalog()
        .map_err(pdf_err)?
        .get(b"AcroForm")
        .map_err(pdf_err)?
        .clone();
    match acro {
        Object::Reference(id) => {
            doc.get_dictionary_mut(id)
                .map_err(pdf_err)?
                .set("NeedAppearances", true);
        }
        Object::Dictionary(mut d) => {
            d.set("NeedAppearances", true);
            doc.catalog_mut().map_err(pdf_err)?.set("AcroForm", d);
        }
        _ => {}
    }

    let mut out = Vec::new();
    doc.save_to(&mut out).map_err(pdf_err)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formular::testdaten::musterformular;

    fn feld<'a>(felder: &'a [Formularfeld], name: &str) -> &'a Formularfeld {
        felder.iter().find(|f| f.name == name).unwrap()
    }

    #[test]
    fn liest_felder() {
        let felder = felder_lesen(&musterformular()).unwrap();
        let namen: Vec<_> = felder.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            namen,
            [
                "Bauherr_Name",
                "Grundstueck.Flaeche",
                "Sonderbau",
                "Vorhaben",
                "Gebaeudeklasse"
            ]
        );
        let bauherr = feld(&felder, "Bauherr_Name");
        assert_eq!(bauherr.art, Feldart::Text);
        assert_eq!(bauherr.beschriftung.as_deref(), Some("Name des Bauherrn"));
        assert_eq!(bauherr.max_laenge, Some(40));
        assert_eq!(feld(&felder, "Grundstueck.Flaeche").art, Feldart::Text);
        assert_eq!(feld(&felder, "Sonderbau").art, Feldart::Ankreuzfeld);
        assert_eq!(feld(&felder, "Sonderbau").optionen, ["Ja"]);
        assert_eq!(feld(&felder, "Vorhaben").art, Feldart::Optionsfeld);
        assert_eq!(feld(&felder, "Vorhaben").optionen, ["Neubau", "Umbau"]);
        assert_eq!(feld(&felder, "Gebaeudeklasse").optionen, ["1", "2", "3"]);
    }

    #[test]
    fn befuellt_und_liest_zurueck() {
        let werte = BTreeMap::from([
            (
                "Bauherr_Name".to_owned(),
                Feldwert::Text("Jörg Müßig".into()),
            ),
            (
                "Grundstueck.Flaeche".to_owned(),
                Feldwert::Text("640,00".into()),
            ),
            ("Sonderbau".to_owned(), Feldwert::Ja(true)),
            ("Vorhaben".to_owned(), Feldwert::Text("Umbau".into())),
            ("Gebaeudeklasse".to_owned(), Feldwert::Text("2".into())),
        ]);
        let pdf = befuellen(&musterformular(), &werte).unwrap();
        let felder = felder_lesen(&pdf).unwrap();
        for (name, wert) in &werte {
            assert_eq!(feld(&felder, name).wert.as_ref(), Some(wert), "{name}");
        }
    }

    #[test]
    fn nicht_latin1_wird_utf16() {
        let werte = BTreeMap::from([(
            "Bauherr_Name".to_owned(),
            Feldwert::Text("Łukasz € GmbH".into()),
        )]);
        let pdf = befuellen(&musterformular(), &werte).unwrap();
        let felder = felder_lesen(&pdf).unwrap();
        assert_eq!(
            feld(&felder, "Bauherr_Name").wert,
            Some(Feldwert::Text("Łukasz € GmbH".into()))
        );
    }

    #[test]
    fn fehler_bei_unbekanntem_feld_und_falschem_wert() {
        let pdf = musterformular();
        let unbekannt = BTreeMap::from([("Gibtsnicht".to_owned(), Feldwert::Text("x".into()))]);
        assert!(befuellen(&pdf, &unbekannt).is_err());
        let falsch = BTreeMap::from([("Vorhaben".to_owned(), Feldwert::Text("Abriss".into()))]);
        assert!(befuellen(&pdf, &falsch).is_err());
        let zu_lang = BTreeMap::from([("Bauherr_Name".to_owned(), Feldwert::Text("x".repeat(41)))]);
        assert!(befuellen(&pdf, &zu_lang).is_err());
    }

    #[test]
    fn kein_formular() {
        assert!(felder_lesen(b"kein pdf").is_err());
    }
}
