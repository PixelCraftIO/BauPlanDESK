//! AcroForm-Zugriff auf PDF-Dateien.

use std::collections::{BTreeMap, HashSet};

use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::{Deserialize, Serialize};

use super::schrift;
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
    #[serde(default)]
    pub mehrzeilig: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Feldwert {
    Text(String),
    Ja(bool),
}

const FF_READ_ONLY: i64 = 1;
pub(crate) const FF_MULTILINE: i64 = 1 << 12;
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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_be_bytes(c))
            .collect();
        String::from_utf16_lossy(&units)
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// ASCII ist direkt verwendbar; sonst UTF-16BE mit BOM, um abweichende
/// PDFDocEncoding-Zuordnungen (z. B. NBSP) nicht falsch zu schreiben.
pub(crate) fn text_kodieren(text: &str) -> Object {
    let bytes = if text.is_ascii() {
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

pub(super) fn lade(pdf: &[u8]) -> CoreResult<Document> {
    let doc = Document::load_mem(pdf).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(CoreError::InvalidInput(
            "verschlüsselte PDF-Formulare werden noch nicht unterstützt".into(),
        ));
    }
    Ok(doc)
}

/// Änderungen würden bestehende digitale Signaturen ungültig machen.
/// Leere Signaturfelder bleiben dagegen als Teil der Vorlage erhalten.
pub(super) fn bearbeitbar(doc: &Document) -> CoreResult<()> {
    if doc.objects.values().any(|o| {
        o.as_dict()
            .ok()
            .is_some_and(|d| d.has(b"ByteRange") && d.has(b"Contents"))
    }) {
        return Err(pdf_err(
            "Das PDF ist digital signiert. Bitte eine unsignierte Vorlage verwenden.",
        ));
    }
    if let Ok(acro) = doc.catalog().map_err(pdf_err)?.get(b"AcroForm") {
        let (_, a) = doc.dereference(acro).map_err(pdf_err)?;
        if a.as_dict().map_err(pdf_err)?.has(b"XFA") {
            return Err(pdf_err(
                "XFA-Formulare werden nicht unterstützt. Bitte eine Standard-PDF-/AcroForm-Version verwenden.",
            ));
        }
    }
    Ok(())
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
            mehrzeilig: art == Feldart::Text && k.ff & FF_MULTILINE != 0,
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

/// Geerbter Feldeintrag: am Objekt selbst, sonst entlang `/Parent`.
fn geerbt<'a>(doc: &'a Document, start: ObjectId, key: &[u8]) -> Option<&'a Object> {
    let mut id = start;
    let mut gesehen = HashSet::new();
    while gesehen.insert(id) {
        let d = doc.get_dictionary(id).ok()?;
        if let Ok(v) = d.get(key) {
            return doc.dereference(v).ok().map(|(_, v)| v);
        }
        id = d.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn acroform(doc: &Document) -> Option<&Dictionary> {
    let a = doc.catalog().ok()?.get(b"AcroForm").ok()?;
    doc.dereference(a).ok()?.1.as_dict().ok()
}

/// Normaldarstellung (`/AP /N`) eines Widgets, falls vorhanden.
fn normaldarstellung(doc: &Document, widget: ObjectId) -> Option<&Object> {
    let ap = dict(doc, widget).ok()?.get(b"AP").ok()?;
    let n = doc.dereference(ap).ok()?.1.as_dict().ok()?.get(b"N").ok()?;
    doc.dereference(n).ok().map(|(_, n)| n)
}

/// Die Darstellung stammt aus einer früheren Befüllung mit der eingebetteten
/// Unicode-Schrift und wird daher bei jedem Speichern neu erzeugt.
fn eigene_darstellung(doc: &Document, widget: ObjectId) -> bool {
    normaldarstellung(doc, widget)
        .and_then(|n| n.as_stream().ok())
        .and_then(|s| s.dict.get(b"Resources").ok())
        .and_then(|r| doc.dereference(r).ok())
        .and_then(|(_, r)| r.as_dict().ok())
        .and_then(|r| r.get(b"Font").ok())
        .and_then(|f| doc.dereference(f).ok())
        .and_then(|(_, f)| f.as_dict().ok())
        .is_some_and(|f| f.has(b"BauUnicode"))
}

/// Schriftgröße (0 = automatisch), Farbe und Ausrichtung aus `/DA` und `/Q`
/// des Widgets bzw. des Felds, sonst aus dem AcroForm-Wörterbuch.
fn textstil(doc: &Document, widget: ObjectId, mehrzeilig: bool) -> (schrift::Textstil, f32) {
    let acro = acroform(doc);
    let ausrichtung = geerbt(doc, widget, b"Q")
        .or_else(|| acro.and_then(|a| a.get(b"Q").ok()))
        .and_then(|q| q.as_i64().ok())
        .filter(|q| (0..=2).contains(q))
        .unwrap_or(0);
    let w = dict(doc, widget).ok();
    if w.is_some_and(|w| {
        w.get(b"BauProjektIOFeld")
            .and_then(Object::as_bool)
            .unwrap_or(false)
    }) {
        let size = w
            .and_then(|w| w.get(b"BauProjektIOSchrift").ok())
            .and_then(|s| s.as_float().ok())
            .unwrap_or(10.0);
        let mut stil = schrift::Textstil::standard(size);
        stil.ausrichtung = ausrichtung;
        return (stil, size);
    }
    let da = geerbt(doc, widget, b"DA")
        .or_else(|| acro.and_then(|a| a.get(b"DA").ok()))
        .and_then(|d| d.as_str().ok())
        .and_then(|d| lopdf::content::Content::decode(d).ok());
    let mut groesse = 0.0;
    let mut farbe = schrift::Farbe::Grau(0.);
    for op in da.map(|d| d.operations).unwrap_or_default() {
        let zahlen: Vec<f32> = op
            .operands
            .iter()
            .filter_map(|o| o.as_float().ok())
            .collect();
        match (op.operator.as_str(), zahlen.as_slice()) {
            ("Tf", [g]) if g.is_finite() && *g >= 0. => groesse = *g,
            ("g", [g]) => farbe = schrift::Farbe::Grau(*g),
            ("rg", [r, g, b]) => farbe = schrift::Farbe::Rgb(*r, *g, *b),
            ("k", [c, m, y, k]) => farbe = schrift::Farbe::Cmyk(*c, *m, *y, *k),
            _ => {}
        }
    }
    // Automatische Größe (0): Obergrenze wie in üblichen PDF-Programmen,
    // die Darstellung verkleinert bei Platzmangel.
    let obergrenze = if groesse > 0. {
        groesse
    } else if mehrzeilig {
        10.
    } else {
        12.
    };
    (
        schrift::Textstil {
            groesse: obergrenze,
            farbe,
            ausrichtung,
        },
        groesse,
    )
}

/// Erzeugt die Darstellung eines Text-Widgets und setzt `/DA` passend.
fn widget_darstellen(
    doc: &mut Document,
    widget: ObjectId,
    text: &str,
    mehrzeilig: bool,
    font: Option<ObjectId>,
) -> CoreResult<()> {
    let rect: Vec<f32> = dict(doc, widget)?
        .get(b"Rect")
        .map_err(pdf_err)?
        .as_array()
        .map_err(pdf_err)?
        .iter()
        .map(|v| v.as_float().map_err(pdf_err))
        .collect::<CoreResult<_>>()?;
    let [x1, y1, x2, y2] = rect[..] else {
        return Err(pdf_err("Ungültiges Feldrechteck"));
    };
    let (stil, da_groesse) = textstil(doc, widget, mehrzeilig);
    let (breite, hoehe) = ((x2 - x1).abs(), (y2 - y1).abs());
    // Versteckte oder flächenlose Widgets zeigen nichts an; sie erhalten eine
    // leere Darstellung statt eines Fehlers.
    let unsichtbar = dict(doc, widget)?
        .get(b"F")
        .and_then(Object::as_i64)
        .unwrap_or(0)
        & 3
        != 0
        || breite < 1.
        || hoehe < 1.;
    let ap = match font {
        Some(font) if !text.is_empty() && !unsichtbar => {
            schrift::darstellung_mit_stil(doc, text, breite, hoehe, stil, font, mehrzeilig)?
        }
        _ => schrift::darstellung_mit_stil(
            doc,
            "",
            breite.max(1.),
            hoehe.max(1.),
            stil,
            ObjectId::default(),
            mehrzeilig,
        )?,
    };
    let w = doc.get_dictionary_mut(widget).map_err(pdf_err)?;
    if font.is_some() {
        w.set(
            "DA",
            Object::string_literal(format!("/BauUnicode {da_groesse} Tf {}", stil.farbe.da())),
        );
    }
    w.set("AP", lopdf::dictionary! {"N"=>ap});
    Ok(())
}

/// Objekte, die nur von ersetzten Darstellungen und Schriften früherer
/// Befüllungen erreichbar waren, werden entfernt. Alles, was vom Trailer aus
/// noch erreichbar ist, bleibt unverändert.
fn ersetzte_entfernen(doc: &mut Document, kandidaten: Vec<ObjectId>) {
    fn refs(o: &Object, out: &mut Vec<ObjectId>) {
        match o {
            Object::Reference(id) => out.push(*id),
            Object::Array(a) => a.iter().for_each(|o| refs(o, out)),
            Object::Dictionary(d) => d.iter().for_each(|(_, o)| refs(o, out)),
            Object::Stream(s) => s.dict.iter().for_each(|(_, o)| refs(o, out)),
            _ => {}
        }
    }
    let huelle = |doc: &Document, start: Vec<ObjectId>| {
        let mut gesehen = HashSet::new();
        let mut offen = start;
        while let Some(id) = offen.pop() {
            if gesehen.insert(id)
                && let Some(o) = doc.objects.get(&id)
            {
                refs(o, &mut offen);
            }
        }
        gesehen
    };
    if kandidaten.is_empty() {
        return;
    }
    let kandidaten = huelle(doc, kandidaten);
    let mut wurzeln = Vec::new();
    refs(&Object::Dictionary(doc.trailer.clone()), &mut wurzeln);
    let erreichbar = huelle(doc, wurzeln);
    for id in kandidaten.difference(&erreichbar) {
        doc.objects.remove(id);
    }
}

fn feldfehler(name: &str, e: CoreError) -> CoreError {
    let text = match e {
        CoreError::InvalidInput(t) => t,
        e => e.to_string(),
    };
    CoreError::InvalidInput(format!("Formularfeld {name:?}: {text}"))
}

/// Setzt Feldwerte und liefert das geänderte PDF. Unbekannte Feldnamen und
/// unpassende Werte sind Fehler, damit kein Formular unbemerkt unvollständig
/// eingereicht wird.
///
/// Jedes geänderte Text- und Auswahlfeld erhält eine eigene Darstellung mit
/// der eingebetteten Unicode-Schrift; Feldgröße, Schriftgröße, Farbe und
/// Ausrichtung aus `/DA` bzw. `/Q` werden beachtet, mehrzeilige Felder
/// umbrochen. Bereits befüllte Felder ohne Darstellung und frühere eigene
/// Darstellungen werden ebenfalls neu erzeugt, damit Druck und Abgabe kein
/// leeres Feld zeigen. `NeedAppearances` wird nur gesetzt, nie zurückgesetzt.
pub fn befuellen(pdf: &[u8], werte: &BTreeMap<String, Feldwert>) -> CoreResult<Vec<u8>> {
    let mut doc = lade(pdf)?;
    bearbeitbar(&doc)?;
    let knoten = felder_sammeln(&doc)?;
    let mut need_appearances = false;
    let mut geaendert = BTreeMap::new();
    for (name, wert) in werte {
        let (index, k) = knoten
            .iter()
            .enumerate()
            .find(|(_, k)| &k.name == name)
            .ok_or_else(|| {
                CoreError::InvalidInput(format!("Formularfeld {name:?} existiert nicht"))
            })?;
        if k.ff & FF_READ_ONLY != 0 {
            return Err(CoreError::InvalidInput(format!(
                "Formularfeld {name:?} ist schreibgeschützt"
            )));
        }
        match (feldart(k), wert) {
            (Feldart::Text | Feldart::Auswahl, Feldwert::Text(t)) => {
                if k.ff & FF_MULTILINE == 0 && t.contains(['\r', '\n']) {
                    return Err(CoreError::InvalidInput(format!(
                        "Formularfeld {name:?} ist einzeilig; Zeilenumbrüche sind hier nicht zulässig"
                    )));
                }
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
                schrift::darstellbar(t).map_err(|e| feldfehler(name, e))?;
                doc.get_dictionary_mut(k.id)
                    .map_err(pdf_err)?
                    .set("V", text_kodieren(t));
                geaendert.insert(index, t.clone());
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
            (Feldart::Optionsfeld, Feldwert::Text(option)) if option.is_empty() => {
                for w in &k.widgets {
                    doc.get_dictionary_mut(*w)
                        .map_err(pdf_err)?
                        .set("AS", Object::Name(b"Off".to_vec()));
                }
                doc.get_dictionary_mut(k.id)
                    .map_err(pdf_err)?
                    .set("V", Object::Name(b"Off".to_vec()));
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

    // Welche Textfelder eine eigene Darstellung erhalten.
    let mut aufgaben: Vec<(usize, String, bool)> = Vec::new();
    for (i, k) in knoten.iter().enumerate() {
        let art = feldart(k);
        if !matches!(art, Feldart::Text | Feldart::Auswahl) {
            continue;
        }
        let mehrzeilig = art == Feldart::Text && k.ff & FF_MULTILINE != 0;
        if let Some(t) = geaendert.get(&i) {
            aufgaben.push((i, t.clone(), true));
            continue;
        }
        let text = dict(&doc, k.id)?
            .get(b"V")
            .ok()
            .and_then(|v| doc.dereference(v).ok())
            .and_then(|(_, v)| v.as_str().ok())
            .map(text_dekodieren)
            .unwrap_or_default();
        let ohne = k
            .widgets
            .iter()
            .any(|w| normaldarstellung(&doc, *w).is_none());
        let eigene = k.widgets.iter().any(|w| eigene_darstellung(&doc, *w));
        if !(eigene || (ohne && !text.is_empty())) {
            continue;
        }
        if schrift::darstellbar(&text).is_ok() && (mehrzeilig || !text.contains(['\r', '\n'])) {
            aufgaben.push((i, text, false));
        } else if ohne && !text.is_empty() {
            need_appearances = true;
        }
    }

    // Ersetzte eigene Darstellungen und Schriften später entfernen.
    let mut kandidaten = Vec::new();
    for (i, _, _) in &aufgaben {
        for w in &knoten[*i].widgets {
            let Ok(a) = dict(&doc, *w)?.get(b"AP") else {
                continue;
            };
            if let Ok(id) = a.as_reference() {
                kandidaten.push(id);
            }
            if let Ok((_, Object::Dictionary(ap))) = doc.dereference(a) {
                kandidaten.extend(ap.iter().filter_map(|(_, o)| o.as_reference().ok()));
            }
        }
    }

    let texte: Vec<&str> = aufgaben
        .iter()
        .map(|(_, t, _)| t.as_str())
        .filter(|t| !t.is_empty())
        .collect();
    let unicode_font = if texte.is_empty() {
        None
    } else {
        Some(schrift::einbetten(&mut doc, &texte)?)
    };
    // Die eingebettete Schrift auch als Feldvorgabe registrieren, damit ein
    // PDF-Programm, das Darstellungen selbst erzeugt, dieselbe Schrift nutzt.
    if let Some(font) = unicode_font {
        let acro_obj = doc
            .catalog()
            .map_err(pdf_err)?
            .get(b"AcroForm")
            .map_err(pdf_err)?
            .clone();
        let mut acro = doc
            .dereference(&acro_obj)
            .map_err(pdf_err)?
            .1
            .as_dict()
            .map_err(pdf_err)?
            .clone();
        let mut dr = acro
            .get(b"DR")
            .ok()
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        let mut fonts = dr
            .get(b"Font")
            .ok()
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        if let Ok(alt) = fonts.get(b"BauUnicode").and_then(Object::as_reference) {
            kandidaten.push(alt);
        }
        fonts.set("BauUnicode", font);
        dr.set("Font", fonts);
        acro.set("DR", dr);
        match acro_obj {
            Object::Reference(id) => {
                doc.objects.insert(id, Object::Dictionary(acro));
            }
            _ => {
                doc.catalog_mut().map_err(pdf_err)?.set("AcroForm", acro);
            }
        }
    }
    for (i, text, pflicht) in &aufgaben {
        let k = &knoten[*i];
        let mehrzeilig = feldart(k) == Feldart::Text && k.ff & FF_MULTILINE != 0;
        for w in &k.widgets {
            match widget_darstellen(&mut doc, *w, text, mehrzeilig, unicode_font) {
                Ok(()) => {}
                Err(e) if *pflicht => return Err(feldfehler(&k.name, e)),
                Err(_) => {
                    if normaldarstellung(&doc, *w).is_none() {
                        need_appearances = true;
                    }
                }
            }
        }
    }
    ersetzte_entfernen(&mut doc, kandidaten);

    // Nur setzen, nie zurücksetzen: frühere Einträge anderer Programme können
    // auf die Neuerzeugung durch den Viewer angewiesen sein.
    if need_appearances {
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
    fn mehrzeiliges_merkmal_wird_geerbt_und_zeilen_bleiben_erhalten() {
        let mut d = Document::load_mem(&musterformular()).unwrap();
        let parent = d
            .objects
            .iter()
            .find(|(_, o)| {
                o.as_dict()
                    .ok()
                    .and_then(|v| v.get(b"T").ok())
                    .and_then(|v| v.as_str().ok())
                    == Some(b"Grundstueck".as_slice())
            })
            .map(|(id, _)| *id)
            .unwrap();
        d.get_dictionary_mut(parent)
            .unwrap()
            .set("Ff", FF_MULTILINE);
        let mut pdf = Vec::new();
        d.save_to(&mut pdf).unwrap();
        assert!(feld(&felder_lesen(&pdf).unwrap(), "Grundstueck.Flaeche").mehrzeilig);
        let werte = BTreeMap::from([(
            "Grundstueck.Flaeche".into(),
            Feldwert::Text("Zeile eins\nZeile zwei".into()),
        )]);
        let neu = befuellen(&pdf, &werte).unwrap();
        assert_eq!(
            feld(&felder_lesen(&neu).unwrap(), "Grundstueck.Flaeche")
                .wert
                .as_ref(),
            werte.get("Grundstueck.Flaeche")
        );
        assert!(befuellen(&musterformular(), &werte).is_err());
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
        // Option wieder abwählen
        let leer = BTreeMap::from([("Vorhaben".to_owned(), Feldwert::Text(String::new()))]);
        let pdf = befuellen(&pdf, &leer).unwrap();
        assert_eq!(feld(&felder_lesen(&pdf).unwrap(), "Vorhaben").wert, None);
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

    fn feld_id(doc: &Document, name: &[u8]) -> ObjectId {
        doc.objects
            .iter()
            .find_map(|(id, o)| {
                o.as_dict()
                    .ok()
                    .filter(|d| d.get(b"T").ok().and_then(|v| v.as_str().ok()) == Some(name))
                    .map(|_| *id)
            })
            .unwrap()
    }

    fn need_appearances(doc: &Document) -> Option<bool> {
        acroform(doc)?
            .get(b"NeedAppearances")
            .ok()
            .and_then(|v| v.as_bool().ok())
    }

    fn schriften(doc: &Document) -> usize {
        doc.objects
            .values()
            .filter(|o| {
                o.as_dict()
                    .ok()
                    .and_then(|d| d.get(b"Subtype").ok())
                    .and_then(|s| s.as_name().ok())
                    == Some(b"Type0".as_slice())
            })
            .count()
    }

    #[test]
    fn fremde_textfelder_erhalten_eigene_darstellung() {
        let werte = BTreeMap::from([
            (
                "Bauherr_Name".to_owned(),
                Feldwert::Text("Łukasz Müller".into()),
            ),
            ("Gebaeudeklasse".to_owned(), Feldwert::Text("2".into())),
        ]);
        let pdf = befuellen(&musterformular(), &werte).unwrap();
        let doc = Document::load_mem(&pdf).unwrap();
        for name in [b"Bauherr_Name".as_slice(), b"Gebaeudeklasse"] {
            let id = feld_id(&doc, name);
            assert!(eigene_darstellung(&doc, id), "{name:?}");
            let da = doc
                .get_dictionary(id)
                .unwrap()
                .get(b"DA")
                .unwrap()
                .as_str()
                .unwrap();
            assert!(da.starts_with(b"/BauUnicode"));
        }
        // Keine Neuerzeugung durch den Viewer nötig.
        assert_ne!(need_appearances(&doc), Some(true));
        // Erneutes Speichern ändert nur das Ankreuzfeld; die frühere
        // Darstellung bleibt, die Schrift wird nicht angehäuft.
        let mut pdf = pdf;
        for _ in 0..3 {
            pdf = befuellen(
                &pdf,
                &BTreeMap::from([("Sonderbau".to_owned(), Feldwert::Ja(true))]),
            )
            .unwrap();
        }
        let doc = Document::load_mem(&pdf).unwrap();
        assert!(eigene_darstellung(&doc, feld_id(&doc, b"Bauherr_Name")));
        assert_eq!(schriften(&doc), 1);
        assert_eq!(
            feld(&felder_lesen(&pdf).unwrap(), "Bauherr_Name").wert,
            Some(Feldwert::Text("Łukasz Müller".into()))
        );
    }

    #[test]
    fn need_appearances_wird_nie_zurueckgesetzt() {
        let mut d = Document::load_mem(&musterformular()).unwrap();
        let acro = d
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        d.get_dictionary_mut(acro)
            .unwrap()
            .set("NeedAppearances", true);
        let mut pdf = Vec::new();
        d.save_to(&mut pdf).unwrap();
        let pdf = befuellen(
            &pdf,
            &BTreeMap::from([("Sonderbau".to_owned(), Feldwert::Ja(true))]),
        )
        .unwrap();
        assert_eq!(
            need_appearances(&Document::load_mem(&pdf).unwrap()),
            Some(true)
        );
    }

    #[test]
    fn darstellung_beachtet_schriftgroesse_farbe_und_ausrichtung() {
        let mut d = Document::load_mem(&musterformular()).unwrap();
        let id = feld_id(&d, b"Bauherr_Name");
        let f = d.get_dictionary_mut(id).unwrap();
        f.set("DA", Object::string_literal("/Helv 8 Tf 0 0 1 rg"));
        f.set("Q", 2);
        let mut pdf = Vec::new();
        d.save_to(&mut pdf).unwrap();
        let pdf = befuellen(
            &pdf,
            &BTreeMap::from([("Bauherr_Name".to_owned(), Feldwert::Text("Abc".into()))]),
        )
        .unwrap();
        let doc = Document::load_mem(&pdf).unwrap();
        let id = feld_id(&doc, b"Bauherr_Name");
        let ap = normaldarstellung(&doc, id).unwrap().as_stream().unwrap();
        let ops = lopdf::content::Content::decode(&ap.content)
            .unwrap()
            .operations;
        let tf = ops.iter().find(|o| o.operator == "Tf").unwrap();
        assert_eq!(tf.operands[1].as_float().unwrap(), 8.);
        assert!(ops.iter().any(|o| o.operator == "rg"));
        // Rechtsbündig: deutlich eingerückt im 100 pt breiten Feld.
        let td = ops.iter().find(|o| o.operator == "Td").unwrap();
        assert!(td.operands[0].as_float().unwrap() > 50.);
        let da = doc.get_dictionary(id).unwrap().get(b"DA").unwrap();
        assert_eq!(da.as_str().unwrap(), b"/BauUnicode 8 Tf 0 0 1 rg");
    }

    #[test]
    fn bereits_befuellte_felder_ohne_darstellung_werden_nachgezogen() {
        // Wert aus einem anderen Programm, ohne Darstellung.
        let mut d = Document::load_mem(&musterformular()).unwrap();
        let id = feld_id(&d, b"Bauherr_Name");
        d.get_dictionary_mut(id)
            .unwrap()
            .set("V", Object::string_literal("Fremd"));
        let mut pdf = Vec::new();
        d.save_to(&mut pdf).unwrap();
        let pdf = befuellen(
            &pdf,
            &BTreeMap::from([("Sonderbau".to_owned(), Feldwert::Ja(true))]),
        )
        .unwrap();
        let doc = Document::load_mem(&pdf).unwrap();
        assert!(eigene_darstellung(&doc, feld_id(&doc, b"Bauherr_Name")));
    }

    #[test]
    fn nicht_darstellbarer_wert_nennt_das_feld() {
        let e = befuellen(
            &musterformular(),
            &BTreeMap::from([(
                "Bauherr_Name".to_owned(),
                Feldwert::Text("\u{10FFFF}".into()),
            )]),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("Bauherr_Name"), "{e}");
    }
}
