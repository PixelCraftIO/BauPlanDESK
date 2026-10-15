//! Öffentlicher Vertrag für eigene PDF-Felder. Positionen sind in Millimetern
//! ab der linken oberen Ecke des sichtbaren Seitenbereichs; Seiten zählen ab 1.
//! Das Original wird nie in-place verändert. Ausgabe bleibt ein Standard-AcroForm.
use super::pdf::{self, Feldart};
use crate::{CoreError, CoreResult};
use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
const PT_MM: f32 = 72.0 / 25.4;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfSeite {
    pub nummer: u32,
    pub breite_mm: f32,
    pub hoehe_mm: f32,
    pub rotation: i64,
    pub platzierbar: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenesPdfFeld {
    pub name: String,
    pub beschriftung: String,
    pub art: Feldart,
    pub seite: u32,
    pub x_mm: f32,
    pub y_mm: f32,
    pub breite_mm: f32,
    pub hoehe_mm: f32,
    #[serde(default = "standard_schrift")]
    pub schriftgroesse_pt: f32,
    #[serde(default)]
    pub mehrzeilig: bool,
}
fn standard_schrift() -> f32 {
    10.0
}
fn err(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("PDF-Vorlage: {e}"))
}
fn geerbt(doc: &Document, mut id: ObjectId, key: &[u8]) -> CoreResult<Option<Object>> {
    let mut seen = HashSet::new();
    while seen.insert(id) {
        let d = doc.get_dictionary(id).map_err(err)?;
        if let Ok(v) = d.get(key) {
            return Ok(Some(doc.dereference(v).map_err(err)?.1.clone()));
        }
        id = match d.get(b"Parent").and_then(Object::as_reference) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
    }
    Err(err("Zyklische Seitenstruktur"))
}
fn seitenbox(doc: &Document, id: ObjectId) -> CoreResult<[f32; 4]> {
    let parse = |o: Object| -> CoreResult<[f32; 4]> {
        let a = o.as_array().map_err(err)?;
        if a.len() != 4 {
            return Err(err("Ungültige Seitengröße"));
        }
        let mut b = [0.0; 4];
        for (i, v) in a.iter().enumerate() {
            b[i] = v.as_float().map_err(err)?;
        }
        Ok(b)
    };
    let media = parse(geerbt(doc, id, b"MediaBox")?.ok_or_else(|| err("Seitengröße fehlt"))?)?;
    let crop = match geerbt(doc, id, b"CropBox")? {
        Some(o) => parse(o)?,
        None => media,
    };
    if !media.iter().chain(crop.iter()).all(|v| v.is_finite()) {
        return Err(err("Ungültige Seitengröße"));
    }
    // PDF viewers show only the intersection of the crop and media box.
    let b = [
        media[0].max(crop[0]),
        media[1].max(crop[1]),
        media[2].min(crop[2]),
        media[3].min(crop[3]),
    ];
    if !b.iter().all(|v| v.is_finite()) || b[2] <= b[0] || b[3] <= b[1] {
        return Err(err("Ungültige Seitengröße"));
    }
    Ok(b)
}
pub fn seiten(pdf: &[u8]) -> CoreResult<Vec<PdfSeite>> {
    let doc = pdf::lade(pdf)?;
    seiten_im_dokument(&doc)
}
fn seiten_im_dokument(doc: &Document) -> CoreResult<Vec<PdfSeite>> {
    doc.get_pages()
        .into_iter()
        .map(|(nummer, id)| {
            let b = seitenbox(doc, id)?;
            let rotation = geerbt(doc, id, b"Rotate")?
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0)
                .rem_euclid(360);
            let unit = geerbt(doc, id, b"UserUnit")?
                .and_then(|o| o.as_float().ok())
                .unwrap_or(1.0);
            Ok(PdfSeite {
                nummer,
                breite_mm: (b[2] - b[0]) / PT_MM,
                hoehe_mm: (b[3] - b[1]) / PT_MM,
                rotation,
                platzierbar: rotation == 0 && (unit - 1.0).abs() < 0.001,
            })
        })
        .collect()
}
fn normaler_name(name: &str) -> CoreResult<String> {
    let n = name.trim();
    let n = if n.starts_with('<') && n.ends_with('>') {
        n[1..n.len() - 1].trim()
    } else {
        n
    };
    if n.is_empty()
        || n.chars().count() > 120
        || n.chars().any(|c| c.is_control() || matches!(c, '<' | '>'))
    {
        return Err(err(
            "Feldname muss 1–120 Zeichen enthalten. Beispiele: Vorname oder <Platzhalter1>.",
        ));
    }
    Ok(n.to_owned())
}
// A partial PDF field name may not contain a dot. Context names such as
// "bauherr.vorname" therefore become a genuine parent/child field tree.
fn feldbaum_einfuegen(
    doc: &mut Document,
    roots: &mut Vec<Object>,
    leaf: ObjectId,
    name: &str,
) -> CoreResult<()> {
    let parts: Vec<_> = name.split('.').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return Err(err("Feldname enthält einen leeren Namensabschnitt"));
    }
    let mut parent = None;
    let mut children = roots.clone();
    for part in parts.iter().take(parts.len() - 1) {
        let found = children
            .iter()
            .filter_map(|o| o.as_reference().ok())
            .find(|id| {
                doc.get_dictionary(*id)
                    .ok()
                    .and_then(|d| d.get(b"T").ok())
                    .and_then(|o| o.as_str().ok())
                    .is_some_and(|t| pdf::text_dekodieren(t) == *part)
            });
        let id = if let Some(id) = found {
            let d = doc.get_dictionary(id).map_err(err)?;
            if !d.has(b"Kids") || d.has(b"V") {
                return Err(err("Feldname kollidiert mit einem bestehenden Eingabefeld"));
            }
            id
        } else {
            let mut d = dictionary! {"T"=>pdf::text_kodieren(part),"Kids"=>Vec::<Object>::new()};
            if let Some(p) = parent {
                d.set("Parent", p);
            }
            let id = doc.add_object(d);
            children.push(id.into());
            if let Some(p) = parent {
                doc.get_dictionary_mut(p)
                    .map_err(err)?
                    .set("Kids", children.clone());
            } else {
                *roots = children.clone();
            }
            id
        };
        let d = doc.get_dictionary(id).map_err(err)?;
        children = doc
            .dereference(d.get(b"Kids").map_err(err)?)
            .map_err(err)?
            .1
            .as_array()
            .map_err(err)?
            .clone();
        parent = Some(id);
    }
    doc.get_dictionary_mut(leaf).map_err(err)?.set(
        "T",
        pdf::text_kodieren(parts.last().ok_or_else(|| err("Feldname fehlt"))?),
    );
    children.push(leaf.into());
    if let Some(p) = parent {
        doc.get_dictionary_mut(leaf).map_err(err)?.set("Parent", p);
        doc.get_dictionary_mut(p)
            .map_err(err)?
            .set("Kids", children);
    } else {
        *roots = children;
    }
    Ok(())
}

/// Erweitert ein PDF um Textfelder und Checkboxen, ohne Hintergrundseiten oder
/// vorhandene Felder zu ersetzen. Signierte Formulare müssen vorab entsiegelt werden.
pub fn felder_hinzufuegen(pdf: &[u8], felder: &[EigenesPdfFeld]) -> CoreResult<Vec<u8>> {
    if felder.is_empty() || felder.len() > 200 {
        return Err(err("Zwischen 1 und 200 neue Felder erforderlich"));
    }
    let mut doc = pdf::lade(pdf)?;
    let existierende = pdf::felder_lesen(pdf)?;
    pdf::bearbeitbar(&doc)?;
    let pages = doc.get_pages();
    let infos = seiten_im_dokument(&doc)?;
    let mut names: HashSet<String> = existierende.into_iter().map(|f| f.name).collect();
    let mut definiert = Vec::new();
    for f in felder {
        let name = normaler_name(&f.name)?;
        if !names.insert(name.clone()) {
            return Err(err(format!(
                "Feld {name:?} existiert bereits. Für ein zusätzliches Feld einen anderen Namen wählen."
            )));
        }
        if f.beschriftung.chars().count() > 500 {
            return Err(err("Beschriftung zu lang"));
        }
        if !matches!(f.art, Feldart::Text | Feldart::Ankreuzfeld) {
            return Err(err("Eigene Felder unterstützen Text oder Checkbox"));
        }
        let page = infos
            .iter()
            .find(|p| p.nummer == f.seite)
            .ok_or_else(|| err("Seite existiert nicht"))?;
        if !page.platzierbar {
            return Err(err(
                "Neue Felder benötigen eine Seite ohne gespeicherte Rotation und mit normalem Maßstab. PDF vor dem Vorlagenimport entsprechend ausrichten.",
            ));
        }
        if f.mehrzeilig && f.art != Feldart::Text {
            return Err(err("Nur Textfelder können mehrzeilig sein"));
        }
        let values = [f.x_mm, f.y_mm, f.breite_mm, f.hoehe_mm, f.schriftgroesse_pt];
        if !values.iter().all(|v| v.is_finite())
            || f.x_mm < 0.0
            || f.y_mm < 0.0
            || f.breite_mm < 2.0
            || f.hoehe_mm < 2.0
            || f.x_mm + f.breite_mm > page.breite_mm + 0.01
            || f.y_mm + f.hoehe_mm > page.hoehe_mm + 0.01
            || !(6.0..=24.0).contains(&f.schriftgroesse_pt)
        {
            return Err(err(format!(
                "Feld {name:?}: Position oder Größe außerhalb der Seite; Schriftgröße 6–24 pt erforderlich"
            )));
        }
        definiert.push((f, name));
    }
    let helvetica=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let mut acro = match doc.catalog().map_err(err)?.get(b"AcroForm") {
        Ok(o) => doc
            .dereference(o)
            .map_err(err)?
            .1
            .as_dict()
            .map_err(err)?
            .clone(),
        Err(_) => lopdf::Dictionary::new(),
    };
    let mut roots = match acro.get(b"Fields") {
        Ok(o) => doc
            .dereference(o)
            .map_err(err)?
            .1
            .as_array()
            .map_err(err)?
            .clone(),
        Err(_) => vec![],
    };
    let mut dr = match acro.get(b"DR") {
        Ok(o) => doc
            .dereference(o)
            .map_err(err)?
            .1
            .as_dict()
            .map_err(err)?
            .clone(),
        Err(_) => lopdf::Dictionary::new(),
    };
    let mut fonts = match dr.get(b"Font") {
        Ok(o) => doc
            .dereference(o)
            .map_err(err)?
            .1
            .as_dict()
            .map_err(err)?
            .clone(),
        Err(_) => lopdf::Dictionary::new(),
    };
    fonts.set("BauHelv", helvetica);
    dr.set("Font", fonts);
    acro.set("DR", dr);
    for (f, name) in definiert {
        let page = *pages.get(&f.seite).ok_or_else(|| err("Seite fehlt"))?;
        let b = seitenbox(&doc, page)?;
        let w = f.breite_mm * PT_MM;
        let h = f.hoehe_mm * PT_MM;
        let x = b[0] + f.x_mm * PT_MM;
        let top = b[3] - f.y_mm * PT_MM;
        let mut d = dictionary! {"Type"=>"Annot","Subtype"=>"Widget","P"=>page,"Rect"=>vec![x.into(),(top-h).into(),(x+w).into(),top.into()],"T"=>pdf::text_kodieren(&name),"TU"=>pdf::text_kodieren(if f.beschriftung.trim().is_empty(){&name}else{f.beschriftung.trim()}),"F"=>4,"BauProjektIOFeld"=>true,"BauProjektIOSchrift"=>f.schriftgroesse_pt,"BS"=>dictionary!{"W"=>0},"MK"=>dictionary!{}};
        if f.art == Feldart::Text {
            d.set("FT", "Tx");
            if f.mehrzeilig {
                d.set("Ff", pdf::FF_MULTILINE);
            }
            d.set("MaxLen", 4000);
            d.set(
                "DA",
                Object::string_literal(format!("/BauHelv {} Tf 0 g", f.schriftgroesse_pt)),
            );
            d.set("V", Object::string_literal(""));
        } else {
            d.set("FT", "Btn");
            d.set("V", "Off");
            d.set("AS", "Off");
            let empty=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),w.into(),h.into()]},vec![]));
            let path = format!(
                "q 0 G 1.5 w {} {} m {} {} l {} {} l S Q",
                w * 0.15,
                h * 0.5,
                w * 0.4,
                h * 0.2,
                w * 0.85,
                h * 0.8
            );
            let checked=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),w.into(),h.into()]},path.into_bytes()));
            d.set(
                "AP",
                dictionary! {"N"=>dictionary!{"Off"=>empty,"Yes"=>checked}},
            );
        }
        let id = doc.add_object(d);
        feldbaum_einfuegen(&mut doc, &mut roots, id, &name)?;
        let mut annots = match doc.get_dictionary(page).map_err(err)?.get(b"Annots") {
            Ok(o) => doc
                .dereference(o)
                .map_err(err)?
                .1
                .as_array()
                .map_err(err)?
                .clone(),
            Err(_) => vec![],
        };
        annots.push(id.into());
        doc.get_dictionary_mut(page)
            .map_err(err)?
            .set("Annots", annots);
    }
    acro.set("Fields", roots);
    acro.set("NeedAppearances", true);
    doc.catalog_mut().map_err(err)?.set("AcroForm", acro);
    let mut out = vec![];
    doc.save_to(&mut out).map_err(err)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::formular::{
        pdf::{Feldwert, befuellen, felder_lesen},
        testdaten::musterformular,
    };
    use std::collections::BTreeMap;
    fn feld(name: &str) -> EigenesPdfFeld {
        EigenesPdfFeld {
            name: name.into(),
            beschriftung: "Vorname".into(),
            art: Feldart::Text,
            seite: 1,
            x_mm: 20.0,
            y_mm: 30.0,
            breite_mm: 70.0,
            hoehe_mm: 8.0,
            schriftgroesse_pt: 10.0,
            mehrzeilig: false,
        }
    }
    #[test]
    fn eigene_felder_ergänzen_erhalten_original_und_unicode() {
        let original = musterformular();
        let mut checkbox = feld("Zustimmung");
        checkbox.art = Feldart::Ankreuzfeld;
        checkbox.y_mm = 50.0;
        checkbox.breite_mm = 5.0;
        checkbox.hoehe_mm = 5.0;
        let template = felder_hinzufuegen(&original, &[feld("< Vorname >"), checkbox]).unwrap();
        assert_eq!(felder_lesen(&original).unwrap().len(), 5);
        assert_eq!(felder_lesen(&template).unwrap().len(), 7);
        let pdf = befuellen(
            &template,
            &BTreeMap::from([
                (
                    "Vorname".into(),
                    Feldwert::Text("Daniel Müller Łukasz €".into()),
                ),
                ("Zustimmung".into(), Feldwert::Ja(true)),
            ]),
        )
        .unwrap();
        let fields = felder_lesen(&pdf).unwrap();
        assert_eq!(
            fields.iter().find(|f| f.name == "Vorname").unwrap().wert,
            Some(Feldwert::Text("Daniel Müller Łukasz €".into()))
        );
        assert_eq!(
            fields.iter().find(|f| f.name == "Zustimmung").unwrap().wert,
            Some(Feldwert::Ja(true))
        );
        let doc = pdf::lade(&pdf).unwrap();
        assert!(
            doc.objects
                .values()
                .any(|o| o.as_dict().ok().is_some_and(|d| d.has(b"FontFile2")))
        );
    }
    #[test]
    fn hierarchische_namen_bleiben_standardkonform() {
        let t = felder_hinzufuegen(
            &musterformular(),
            &[feld("bauherr.vorname"), feld("bauherr.familienname")],
        )
        .unwrap();
        let fields = felder_lesen(&t).unwrap();
        assert!(fields.iter().any(|f| f.name == "bauherr.vorname"));
        assert!(fields.iter().any(|f| f.name == "bauherr.familienname"));
        let doc = pdf::lade(&t).unwrap();
        assert!(
            doc.objects
                .values()
                .filter_map(|o| o.as_dict().ok())
                .filter_map(|d| d.get(b"T").ok())
                .filter_map(|o| o.as_str().ok())
                .all(|t| !pdf::text_dekodieren(t).contains('.'))
        );
        assert!(felder_hinzufuegen(&musterformular(), &[feld("Bauherr_Name.vorname")]).is_err());
    }

    #[test]
    fn leere_signaturfelder_bleiben_erhalten_signierte_und_xfa_werden_abgewiesen() {
        let mut doc = pdf::lade(&musterformular()).unwrap();
        doc.add_object(dictionary! {"FT"=>"Sig","T"=>Object::string_literal("Unterschrift")});
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        assert!(felder_hinzufuegen(&bytes, &[feld("Vorname")]).is_ok());
        doc.add_object(dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),1.into(),2.into(),3.into()],"Contents"=>Object::string_literal("Signatur")});
        bytes.clear();
        doc.save_to(&mut bytes).unwrap();
        assert!(felder_hinzufuegen(&bytes, &[feld("Vorname")]).is_err());
        assert!(befuellen(&bytes, &BTreeMap::new()).is_err());
        let mut doc = pdf::lade(&musterformular()).unwrap();
        let acro = doc
            .catalog()
            .unwrap()
            .get(b"AcroForm")
            .unwrap()
            .as_reference()
            .unwrap();
        doc.get_dictionary_mut(acro)
            .unwrap()
            .set("XFA", Object::string_literal("xfa"));
        bytes.clear();
        doc.save_to(&mut bytes).unwrap();
        assert!(felder_hinzufuegen(&bytes, &[feld("Vorname")]).is_err());
    }

    #[test]
    fn unzulässige_positionen_und_namen() {
        let pdf = musterformular();
        for f in [
            feld("Bauherr_Name"),
            feld("<>"),
            EigenesPdfFeld {
                x_mm: 999.0,
                ..feld("x")
            },
            EigenesPdfFeld {
                seite: 8,
                ..feld("x")
            },
            EigenesPdfFeld {
                breite_mm: -1.0,
                ..feld("x")
            },
        ] {
            assert!(felder_hinzufuegen(&pdf, &[f]).is_err());
        }
        assert!(felder_hinzufuegen(&pdf, &[feld("x"), feld("<x>")]).is_err());
    }
    #[test]
    fn pdf_ohne_felder_wird_zur_vorlage() {
        let mut doc = pdf::lade(&musterformular()).unwrap();
        doc.catalog_mut().unwrap().remove(b"AcroForm");
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        assert!(felder_lesen(&bytes).unwrap().is_empty());
        let template = felder_hinzufuegen(&bytes, &[feld("Platzhalter1")]).unwrap();
        assert_eq!(felder_lesen(&template).unwrap()[0].name, "Platzhalter1");
    }
    #[test]
    fn seiten_und_cropbox() {
        let mut doc = pdf::lade(&musterformular()).unwrap();
        let id = *doc.get_pages().get(&1).unwrap();
        doc.get_dictionary_mut(id).unwrap().set(
            "CropBox",
            vec![20.into(), 20.into(), 500.into(), 800.into()],
        );
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        let info = seiten(&bytes).unwrap();
        assert!((info[0].breite_mm - 480.0 / PT_MM).abs() < 0.01);
        let template = felder_hinzufuegen(&bytes, &[feld("x")]).unwrap();
        assert_eq!(seiten(&template).unwrap()[0].nummer, 1);
    }
    #[test]
    fn cropbox_außerhalb_der_seite_wird_begrenzt() {
        let mut doc = pdf::lade(&musterformular()).unwrap();
        let id = *doc.get_pages().get(&1).unwrap();
        doc.get_dictionary_mut(id).unwrap().set(
            "CropBox",
            vec![(-20).into(), (-20).into(), 700.into(), 900.into()],
        );
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        assert!((seiten(&bytes).unwrap()[0].breite_mm - 595.0 / PT_MM).abs() < 0.01);
    }

    #[test]
    fn rotierte_seite_wird_nicht_falsch_beschrieben() {
        let mut doc = pdf::lade(&musterformular()).unwrap();
        let id = *doc.get_pages().get(&1).unwrap();
        doc.get_dictionary_mut(id).unwrap().set("Rotate", 90);
        let mut bytes = vec![];
        doc.save_to(&mut bytes).unwrap();
        assert!(!seiten(&bytes).unwrap()[0].platzierbar);
        assert!(felder_hinzufuegen(&bytes, &[feld("x")]).is_err());
    }
}
