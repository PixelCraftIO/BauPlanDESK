//! Darstellung eigener Textfelder mit eingebetteter Unicode-Schrift.
//! Die Schrift bleibt ein internes PDF-Detail; Verträge verwenden Textwerte.
use crate::{CoreError, CoreResult};
use lopdf::{
    Document, Object, ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};
use std::collections::BTreeMap;
const FONT: &[u8] = include_bytes!("../../assets/DejaVuSans.ttf");
fn fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("PDF-Schrift: {e}"))
}
fn face() -> CoreResult<ttf_parser::Face<'static>> {
    ttf_parser::Face::parse(FONT, 0).map_err(fehler)
}
fn glyph(f: &ttf_parser::Face<'_>, c: char) -> CoreResult<ttf_parser::GlyphId> {
    f.glyph_index(c).ok_or_else(|| {
        fehler(format!(
            "Zeichen {c:?} wird von der eingebetteten Schrift nicht unterstützt"
        ))
    })
}
pub(super) fn einbetten(doc: &mut Document, texte: &[&str]) -> CoreResult<ObjectId> {
    let f = face()?;
    let units = f.units_per_em() as f32;
    let bbox = f.global_bounding_box();
    let mut chars = BTreeMap::new();
    for c in texte.iter().flat_map(|t| t.chars()) {
        if c.is_control() {
            return Err(fehler(
                "Eigene Textfelder sind einzeilig; Steuerzeichen sind nicht zulässig",
            ));
        }
        chars.insert(glyph(&f, c)?.0, c);
    }
    let stream = doc.add_object(Stream::new(
        dictionary! {"Length1"=>FONT.len() as i64},
        FONT.to_vec(),
    ));
    let scale = |v: i16| v as f32 * 1000.0 / units;
    let descriptor=doc.add_object(dictionary!{"Type"=>"FontDescriptor","FontName"=>"DejaVuSans","Flags"=>32,"FontBBox"=>vec![scale(bbox.x_min).into(),scale(bbox.y_min).into(),scale(bbox.x_max).into(),scale(bbox.y_max).into()],"ItalicAngle"=>0,"Ascent"=>scale(f.ascender()),"Descent"=>scale(f.descender()),"CapHeight"=>scale(f.ascender()),"StemV"=>80,"FontFile2"=>stream});
    let mut widths = Vec::<Object>::new();
    for id in chars.keys() {
        widths.push((*id as i64).into());
        widths.push(Object::Array(vec![
            (f.glyph_hor_advance(ttf_parser::GlyphId(*id)).unwrap_or(0) as f32 * 1000.0 / units)
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
pub(super) fn darstellung(
    doc: &mut Document,
    text: &str,
    width: f32,
    height: f32,
    size: f32,
    font: ObjectId,
) -> CoreResult<ObjectId> {
    let f = face()?;
    let mut codes = Vec::new();
    let mut advance = 0.0;
    for c in text.chars() {
        let id = glyph(&f, c)?;
        codes.extend_from_slice(&id.0.to_be_bytes());
        advance += f.glyph_hor_advance(id).unwrap_or(0) as f32 / f.units_per_em() as f32;
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
    let baseline = (height - size) * 0.5 - size * f.descender() as f32 / f.units_per_em() as f32;
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
