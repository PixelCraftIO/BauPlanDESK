//! Darstellung eigener Textfelder mit eingebetteter Unicode-Schrift.
//! Die Schrift bleibt ein internes PDF-Detail; Verträge verwenden Textwerte.
use crate::{CoreError, CoreResult};
use lopdf::{
    Document, Object, ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
};
use std::collections::BTreeMap;
const FONT: &[u8] = include_bytes!("../../assets/DejaVuSans.ttf");
fn fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("PDF-Schrift: {e}"))
}
fn face() -> CoreResult<FontRef<'static>> {
    FontRef::new(FONT).map_err(fehler)
}
fn glyph(f: &FontRef<'_>, c: char) -> CoreResult<u16> {
    f.charmap()
        .map(c)
        .ok_or_else(|| {
            fehler(format!(
                "Zeichen {c:?} wird von der eingebetteten Schrift nicht unterstützt"
            ))
        })
        .and_then(|id| u16::try_from(id.to_u32()).map_err(fehler))
}
pub(crate) fn einbetten(doc: &mut Document, texte: &[&str]) -> CoreResult<ObjectId> {
    let f = face()?;
    let metrics = f.metrics(Size::unscaled(), LocationRef::default());
    let units = metrics.units_per_em as f32;
    let bbox = metrics
        .bounds
        .ok_or_else(|| fehler("Schriftbegrenzung fehlt"))?;
    let glyph_metrics = f.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut chars = BTreeMap::new();
    for c in texte.iter().flat_map(|t| t.chars()) {
        if matches!(c, '\r' | '\n') {
            continue;
        }
        if c.is_control() {
            return Err(fehler(
                "Steuerzeichen außer Zeilenumbrüchen sind nicht zulässig",
            ));
        }
        chars.insert(glyph(&f, c)?, c);
    }
    let stream = doc.add_object(Stream::new(
        dictionary! {"Length1"=>FONT.len() as i64},
        FONT.to_vec(),
    ));
    let scale = |v: f32| v * 1000.0 / units;
    let descriptor=doc.add_object(dictionary!{"Type"=>"FontDescriptor","FontName"=>"DejaVuSans","Flags"=>32,"FontBBox"=>vec![scale(bbox.x_min).into(),scale(bbox.y_min).into(),scale(bbox.x_max).into(),scale(bbox.y_max).into()],"ItalicAngle"=>0,"Ascent"=>scale(metrics.ascent),"Descent"=>scale(metrics.descent),"CapHeight"=>scale(metrics.ascent),"StemV"=>80,"FontFile2"=>stream});
    let mut widths = Vec::<Object>::new();
    for id in chars.keys() {
        widths.push((*id as i64).into());
        widths.push(Object::Array(vec![
            scale(
                glyph_metrics
                    .advance_width(GlyphId::new(*id as u32))
                    .ok_or_else(|| fehler("Zeichenbreite fehlt"))?,
            )
            .into(),
        ]));
    }
    let cid=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"CIDFontType2","BaseFont"=>"DejaVuSans","CIDSystemInfo"=>dictionary!{"Registry"=>Object::string_literal("Adobe"),"Ordering"=>Object::string_literal("Identity"),"Supplement"=>0},"FontDescriptor"=>descriptor,"DW"=>1000,"W"=>widths,"CIDToGIDMap"=>"Identity"});
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /BauProjektUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = chars.into_iter().collect();
    for chunk in entries.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (id, c) in chunk {
            let hex = c
                .encode_utf16(&mut [0; 2])
                .iter()
                .map(|u| format!("{u:04X}"))
                .collect::<String>();
            cmap.push_str(&format!("<{id:04X}> <{hex}>\n"));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend end\n");
    let unicode = doc.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    Ok(doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type0","BaseFont"=>"DejaVuSans","Encoding"=>"Identity-H","DescendantFonts"=>vec![cid.into()],"ToUnicode"=>unicode}))
}
/// Prüft, ob `text` mit der eingebetteten Schrift dargestellt werden kann
/// (alle Zeichen vorhanden, keine Steuerzeichen außer Zeilenumbrüchen).
pub(crate) fn darstellbar(text: &str) -> CoreResult<()> {
    let f = face()?;
    for c in text.chars() {
        if matches!(c, '\r' | '\n') {
            continue;
        }
        if c.is_control() {
            return Err(fehler(
                "Steuerzeichen außer Zeilenumbrüchen sind nicht zulässig",
            ));
        }
        glyph(&f, c)?;
    }
    Ok(())
}

/// Textfarbe aus dem `/DA`-Eintrag eines Formularfelds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Farbe {
    Grau(f32),
    Rgb(f32, f32, f32),
    Cmyk(f32, f32, f32, f32),
}
impl Farbe {
    fn operation(self) -> Operation {
        match self {
            Farbe::Grau(g) => Operation::new("g", vec![g.into()]),
            Farbe::Rgb(r, g, b) => Operation::new("rg", vec![r.into(), g.into(), b.into()]),
            Farbe::Cmyk(c, m, y, k) => {
                Operation::new("k", vec![c.into(), m.into(), y.into(), k.into()])
            }
        }
    }
    /// Operator für den `/DA`-Eintrag, z. B. `0 g`.
    pub(crate) fn da(self) -> String {
        match self {
            Farbe::Grau(g) => format!("{g} g"),
            Farbe::Rgb(r, g, b) => format!("{r} {g} {b} rg"),
            Farbe::Cmyk(c, m, y, k) => format!("{c} {m} {y} {k} k"),
        }
    }
}
/// Darstellung eines Textfelds: Schriftgröße (Obergrenze, bei Platzmangel
/// wird verkleinert), Farbe und Ausrichtung (`/Q`: 0 links, 1 zentriert,
/// 2 rechts).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Textstil {
    pub groesse: f32,
    pub farbe: Farbe,
    pub ausrichtung: i64,
}
impl Textstil {
    pub(crate) fn standard(groesse: f32) -> Self {
        Self {
            groesse,
            farbe: Farbe::Grau(0.),
            ausrichtung: 0,
        }
    }
}
fn einzug(ausrichtung: i64, width: f32, textbreite: f32) -> f32 {
    match ausrichtung {
        1 => ((width - textbreite) / 2.).max(2.),
        2 => (width - 2. - textbreite).max(2.),
        _ => 2.,
    }
}
fn formular_xobject(
    doc: &mut Document,
    width: f32,
    height: f32,
    font: Option<ObjectId>,
    data: Vec<u8>,
) -> ObjectId {
    let mut d = dictionary! {"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),width.into(),height.into()]};
    if let Some(font) = font {
        d.set(
            "Resources",
            dictionary! {"Font"=>dictionary!{"BauUnicode"=>font}},
        );
    }
    doc.add_object(Stream::new(d, data))
}
pub(crate) fn darstellung(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    size: f32,
    font: ObjectId,
    mehrzeilig: bool,
) -> CoreResult<ObjectId> {
    darstellung_mit_stil(
        doc,
        text,
        width,
        height,
        Textstil::standard(size),
        font,
        mehrzeilig,
    )
}
/// Erzeugt die Normaldarstellung (`/AP /N`) eines Textfelds mit der
/// eingebetteten Unicode-Schrift. Ein leerer Text ergibt eine leere Darstellung.
pub(crate) fn darstellung_mit_stil(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    stil: Textstil,
    font: ObjectId,
    mehrzeilig: bool,
) -> CoreResult<ObjectId> {
    let size = stil.groesse;
    if [width, height, size]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(fehler("Ungültige Feld- oder Schriftgröße"));
    }
    if !mehrzeilig && text.contains(['\r', '\n']) {
        return Err(fehler("Einzeiliges Feld erlaubt keine Zeilenumbrüche"));
    }
    if text.is_empty() {
        return Ok(formular_xobject(doc, width, height, None, Vec::new()));
    }
    if mehrzeilig {
        return mehrzeilige_darstellung(doc, text, width, height, stil, font);
    }
    let f = face()?;
    let metrics = f.metrics(Size::unscaled(), LocationRef::default());
    let glyph_metrics = f.glyph_metrics(Size::unscaled(), LocationRef::default());
    let mut codes = Vec::new();
    let mut advance = 0.0;
    for c in text.chars() {
        let id = glyph(&f, c)?;
        codes.extend_from_slice(&id.to_be_bytes());
        advance += glyph_metrics
            .advance_width(GlyphId::new(id as u32))
            .ok_or_else(|| fehler("Zeichenbreite fehlt"))?
            / metrics.units_per_em as f32;
    }
    let size = size.min((height - 2.0) * 0.8).min(if advance > 0.0 {
        (width - 4.0) / advance
    } else {
        size
    });
    if size < 4.0 {
        return Err(fehler(
            "Text passt nicht in das Feld. Feld vergrößern oder Text kürzen.",
        ));
    }
    let baseline = (height - size) * 0.5 - size * metrics.descent / metrics.units_per_em as f32;
    let operations = vec![
        Operation::new("q", vec![]),
        Operation::new("re", vec![0.into(), 0.into(), width.into(), height.into()]),
        Operation::new("W", vec![]),
        Operation::new("n", vec![]),
        Operation::new("BT", vec![]),
        Operation::new(
            "Tf",
            vec![Object::Name(b"BauUnicode".to_vec()), size.into()],
        ),
        stil.farbe.operation(),
        Operation::new(
            "Td",
            vec![
                einzug(stil.ausrichtung, width, advance * size).into(),
                baseline.into(),
            ],
        ),
        Operation::new(
            "Tj",
            vec![Object::String(codes, lopdf::StringFormat::Hexadecimal)],
        ),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ];
    let data = Content { operations }.encode().map_err(fehler)?;
    Ok(formular_xobject(doc, width, height, Some(font), data))
}

/// Explizite Umbrüche und automatische Wortumbrüche; Text bleibt unverändert im Feldwert.
fn mehrzeilige_darstellung(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    stil: Textstil,
    font: ObjectId,
) -> CoreResult<ObjectId> {
    let size = stil.groesse;
    let f = face()?;
    let metrics = f.metrics(Size::unscaled(), LocationRef::default());
    let gm = f.glyph_metrics(Size::unscaled(), LocationRef::default());
    let units = metrics.units_per_em as f32;
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut zeichen = BTreeMap::new();
    for c in text.chars().filter(|c| *c != '\n') {
        let id = glyph(&f, c)?;
        zeichen.insert(
            c,
            (
                id,
                gm.advance_width(GlyphId::new(id as u32))
                    .ok_or_else(|| fehler("Zeichenbreite fehlt"))?
                    / units,
            ),
        );
    }
    let umbruch = |size: f32| -> Option<Vec<String>> {
        let mut zeilen = Vec::new();
        for absatz in text.split('\n') {
            let mut rest = absatz;
            while !rest.is_empty() {
                let mut breite = 0.;
                let mut ende = 0;
                let mut wortgrenze = None;
                for (pos, c) in rest.char_indices() {
                    if breite + zeichen[&c].1 * size > width - 4. {
                        break;
                    }
                    breite += zeichen[&c].1 * size;
                    ende = pos + c.len_utf8();
                    if c == ' ' && pos > 0 {
                        wortgrenze = Some(pos);
                    }
                }
                if ende == rest.len() {
                    zeilen.push(rest.into());
                    rest = "";
                } else if ende == 0 {
                    return None;
                } else {
                    let ende = wortgrenze.unwrap_or(ende);
                    zeilen.push(rest[..ende].into());
                    rest = rest[ende..].trim_start_matches(' ');
                }
            }
            if absatz.is_empty() {
                zeilen.push(String::new());
            }
        }
        Some(zeilen)
    };
    let mut size = size.min((height - 4.) / 1.2);
    let zeilen = loop {
        if size < 4. {
            return Err(fehler(
                "Text passt nicht in das mehrzeilige Feld. Feld vergrößern oder Text kürzen.",
            ));
        }
        if let Some(zeilen) = umbruch(size)
            && zeilen.len() as f32 * size * 1.2 <= height - 4.
        {
            break zeilen;
        }
        if size <= 4. {
            return Err(fehler(
                "Text passt nicht in das mehrzeilige Feld. Feld vergrößern oder Text kürzen.",
            ));
        }
        size = (size - 0.25).max(4.);
    };
    let mut ops = vec![
        Operation::new("q", vec![]),
        Operation::new("re", vec![0.into(), 0.into(), width.into(), height.into()]),
        Operation::new("W", vec![]),
        Operation::new("n", vec![]),
        Operation::new("BT", vec![]),
        Operation::new(
            "Tf",
            vec![Object::Name(b"BauUnicode".to_vec()), size.into()],
        ),
        stil.farbe.operation(),
    ];
    let mut x_vorher = 0.;
    for (i, zeile) in zeilen.iter().enumerate() {
        let breite: f32 = zeile.chars().map(|c| zeichen[&c].1 * size).sum();
        let x = einzug(stil.ausrichtung, width, breite);
        let y = if i == 0 {
            height - 2. - size * metrics.ascent / units
        } else {
            -size * 1.2
        };
        ops.push(Operation::new("Td", vec![(x - x_vorher).into(), y.into()]));
        x_vorher = x;
        let codes = zeile
            .chars()
            .flat_map(|c| zeichen[&c].0.to_be_bytes())
            .collect();
        ops.push(Operation::new(
            "Tj",
            vec![Object::String(codes, lopdf::StringFormat::Hexadecimal)],
        ));
    }
    ops.extend([Operation::new("ET", vec![]), Operation::new("Q", vec![])]);
    let data = Content { operations: ops }.encode().map_err(fehler)?;
    Ok(formular_xobject(doc, width, height, Some(font), data))
}

/// Breite in PDF-Punkten für die gemeinsame Berichtsausgabe.
pub(crate) fn textbreite(text: &str, size: f32) -> CoreResult<f32> {
    let f = face()?;
    let gm = f.glyph_metrics(Size::unscaled(), LocationRef::default());
    let units = f
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em as f32;
    text.chars().try_fold(0., |sum, c| {
        let id = glyph(&f, c)?;
        Ok(sum
            + gm.advance_width(GlyphId::new(id as u32))
                .ok_or_else(|| fehler("Zeichenbreite fehlt"))?
                / units
                * size)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mehrzeilige_darstellung_erhaelt_absatzumbrueche_und_bricht_lange_woerter_um() {
        let mut doc = Document::with_version("1.7");
        let text = "Daniel Müller\r\nMusterstraße 10\r10115 Berlin";
        let font = einbetten(&mut doc, &[text]).unwrap();
        let ap = darstellung(&mut doc, text, 200., 70., 11., font, true).unwrap();
        let ops = Content::decode(&doc.get_object(ap).unwrap().as_stream().unwrap().content)
            .unwrap()
            .operations;
        assert_eq!(ops.iter().filter(|o| o.operator == "Tj").count(), 3);
        let y = ops
            .iter()
            .filter(|o| o.operator == "Td")
            .map(|o| o.operands[1].as_float().unwrap())
            .collect::<Vec<_>>();
        assert!(y[0] > 50. && y[0] < 70.);
        assert!(y[1] < 0. && y[2] < 0.);
        assert!(darstellung(&mut doc, text, 200., 70., 11., font, false).is_err());
        let text = "W".repeat(80);
        let font = einbetten(&mut doc, &[&text]).unwrap();
        let ap = darstellung(&mut doc, &text, 100., 120., 10., font, true).unwrap();
        let ops = Content::decode(&doc.get_object(ap).unwrap().as_stream().unwrap().content)
            .unwrap()
            .operations;
        let zeilen = ops
            .iter()
            .filter(|o| o.operator == "Tj")
            .collect::<Vec<_>>();
        assert!(zeilen.len() > 1);
        assert_eq!(
            zeilen
                .iter()
                .map(|o| o.operands[0].as_str().unwrap().len())
                .sum::<usize>(),
            160
        );
        assert!(darstellung(&mut doc, &text, 8., 10., 10., font, true).is_err());
        assert!(darstellung(&mut doc, "W", 8., 14., 10., font, true).is_ok());
        assert!(darstellung(&mut doc, &text, f32::NAN, 10., 10., font, true).is_err());
    }
    #[test]
    fn unicode_und_schriftmetriken_bleiben_im_pdf_erhalten() {
        let mut doc = Document::with_version("1.7");
        let font = einbetten(&mut doc, &["Aä€😀"]).unwrap();
        let font_dict = doc.get_dictionary(font).unwrap();
        let cmap = doc
            .get_object(font_dict.get(b"ToUnicode").unwrap().as_reference().unwrap())
            .unwrap()
            .as_stream()
            .unwrap();
        let cmap = std::str::from_utf8(&cmap.content).unwrap();
        for mapping in [
            "<0024> <0041>",
            "<00A6> <00E4>",
            "<0B84> <20AC>",
            "<16E1> <D83DDE00>",
        ] {
            assert!(cmap.contains(mapping));
        }
        let cid = font_dict
            .get(b"DescendantFonts")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .as_reference()
            .unwrap();
        let cid = doc.get_dictionary(cid).unwrap();
        let widths = cid.get(b"W").unwrap().as_array().unwrap();
        // Referenzwerte der mitgelieferten DejaVuSans in 1000 PDF-Einheiten.
        for (entry, (id, width)) in widths.as_chunks::<2>().0.iter().zip([
            (36, 684.08203),
            (166, 612.79297),
            (2948, 636.23047),
            (5857, 1042.4805),
        ]) {
            assert_eq!(entry[0].as_i64().unwrap(), id);
            let actual = entry[1].as_array().unwrap()[0].as_float().unwrap();
            assert!((actual - width).abs() < 0.001);
        }
        assert_eq!(widths.len(), 8);
        let descriptor = doc
            .get_dictionary(cid.get(b"FontDescriptor").unwrap().as_reference().unwrap())
            .unwrap();
        assert!((descriptor.get(b"Ascent").unwrap().as_float().unwrap() - 928.22266).abs() < 0.001);
        assert!(
            (descriptor.get(b"Descent").unwrap().as_float().unwrap() + 235.83984).abs() < 0.001
        );

        let appearance = darstellung(&mut doc, "Aä€😀", 100.0, 20.0, 10.0, font, false).unwrap();
        let stream = doc.get_object(appearance).unwrap().as_stream().unwrap();
        let content = Content::decode(&stream.content).unwrap();
        let text = content
            .operations
            .iter()
            .find(|op| op.operator == "Tj")
            .unwrap();
        assert_eq!(
            text.operands[0].as_str().unwrap(),
            &[0, 36, 0, 166, 11, 132, 22, 225]
        );
        let position = content
            .operations
            .iter()
            .find(|op| op.operator == "Td")
            .unwrap();
        assert!((position.operands[1].as_float().unwrap() - 7.3583984).abs() < 0.001);

        // Gleiche Zeichenbreiten müssen die automatische Verkleinerung steuern.
        let appearance = darstellung(&mut doc, "AA", 10.0, 20.0, 10.0, font, false).unwrap();
        let content = Content::decode(
            &doc.get_object(appearance)
                .unwrap()
                .as_stream()
                .unwrap()
                .content,
        )
        .unwrap();
        let size = content
            .operations
            .iter()
            .find(|op| op.operator == "Tf")
            .unwrap();
        assert!((size.operands[1].as_float().unwrap() - 4.385439).abs() < 0.001);
        assert!(darstellung(&mut doc, "AA", 9.0, 20.0, 10.0, font, false).is_err());
    }

    #[test]
    fn nicht_darstellbare_zeichen_und_steuerzeichen_werden_abgewiesen() {
        for text in ["\u{10FFFF}", "Tab\tText"] {
            assert!(matches!(
                einbetten(&mut Document::with_version("1.7"), &[text]),
                Err(CoreError::InvalidInput(_))
            ));
        }
    }
}
