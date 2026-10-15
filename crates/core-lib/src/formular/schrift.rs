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
pub(crate) fn darstellung(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    size: f32,
    font: ObjectId,
    mehrzeilig: bool,
) -> CoreResult<ObjectId> {
    if [width, height, size]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(fehler("Ungültige Feld- oder Schriftgröße"));
    }
    if mehrzeilig {
        return mehrzeilige_darstellung(doc, text, width, height, size, font);
    }
    if text.contains(['\r', '\n']) {
        return Err(fehler("Einzeiliges Feld erlaubt keine Zeilenumbrüche"));
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
        Operation::new("g", vec![0.into()]),
        Operation::new("Td", vec![2.into(), baseline.into()]),
        Operation::new(
            "Tj",
            vec![Object::String(codes, lopdf::StringFormat::Hexadecimal)],
        ),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ];
    let data = Content { operations }.encode().map_err(fehler)?;
    Ok(doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),width.into(),height.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"BauUnicode"=>font}}},data)))
}

/// Explizite Umbrüche und automatische Wortumbrüche; Text bleibt unverändert im Feldwert.
fn mehrzeilige_darstellung(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    size: f32,
    font: ObjectId,
) -> CoreResult<ObjectId> {
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
        Operation::new("g", vec![0.into()]),
        Operation::new(
            "Td",
            vec![
                2.into(),
                (height - 2. - size * metrics.ascent / units).into(),
            ],
        ),
    ];
    for (i, zeile) in zeilen.iter().enumerate() {
        if i > 0 {
            ops.push(Operation::new("Td", vec![0.into(), (-size * 1.2).into()]));
        }
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
    Ok(doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),width.into(),height.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"BauUnicode"=>font}}},data)))
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
