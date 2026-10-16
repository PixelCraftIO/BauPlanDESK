//! PDF-Engine hinter dem Ausgabe-Vertrag. Originale bleiben unverändert.
use super::{
    berichte::{Zeile, text},
    *,
};
use crate::formular::schrift;
use chrono::{DateTime, Utc};
use lopdf::{
    Dictionary, Document, Object, ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};

const MAX_SEITEN: usize = 300;
/// Fotos mit begrenztem Decoderspeicher, EXIF-Orientierung und weißem Hintergrund.
pub(super) fn bild(bytes: &[u8]) -> CoreResult<Document> {
    use image::ImageDecoder;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(fehler)?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(fehler("Bildformat passt nicht zu PNG/JPEG"));
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(20000);
    limits.max_image_height = Some(20000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(fehler)?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 25_000_000 {
        return Err(fehler("Foto benötigt 1 bis 25 Millionen Pixel"));
    }
    let orientation = decoder.orientation().map_err(fehler)?;
    let mut img = image::DynamicImage::from_decoder(decoder).map_err(fehler)?;
    img.apply_orientation(orientation);
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    for p in rgba.pixels() {
        let a = u16::from(p[3]);
        for c in &p.0[..3] {
            rgb.push(((u16::from(*c) * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let mut bildstream = Stream::new(
        dictionary! {"Type"=>"XObject","Subtype"=>"Image","Width"=>w as i64,"Height"=>h as i64,"ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8},
        rgb,
    );
    bildstream.compress().map_err(fehler)?;
    let image = d.add_object(bildstream);
    let (pw, ph) = if w > h {
        (841.89f32, 595.276f32)
    } else {
        (595.276, 841.89)
    };
    let f = ((pw - 56.) / w as f32).min((ph - 56.) / h as f32);
    let bw = w as f32 * f;
    let bh = h as f32 * f;
    let c = d.add_object(Stream::new(
        dictionary! {},
        format!(
            "q {bw} 0 0 {bh} {} {} cm /Foto Do Q",
            (pw - bw) / 2.,
            (ph - bh) / 2.
        )
        .into_bytes(),
    ));
    let page=d.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),pw.into(),ph.into()],"Resources"=>dictionary!{"XObject"=>dictionary!{"Foto"=>image}},"Contents"=>c});
    d.objects.insert(
        root,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1}),
    );
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    d.trailer.set("Root", cat);
    Ok(d)
}
fn speichern(mut d: Document) -> CoreResult<Vec<u8>> {
    let mut b = Vec::new();
    d.compress();
    d.save_to(&mut b).map_err(fehler)?;
    if b.len() > 128 * 1024 * 1024 {
        return Err(fehler("Ausgabe größer als 128 MB"));
    }
    Ok(b)
}
pub(super) fn umbruch(t: &str, breite: f32, size: f32) -> CoreResult<Vec<String>> {
    let mut result = Vec::new();
    for absatz in t
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ")
        .split('\n')
    {
        let mut rest = absatz;
        while !rest.is_empty() {
            let mut ende = 0;
            let mut wort = None;
            let mut breite_summe = 0.;
            for (i, c) in rest.char_indices() {
                breite_summe += schrift::textbreite(&c.to_string(), size)?;
                if breite_summe > breite {
                    break;
                }
                ende = i + c.len_utf8();
                if c == ' ' && i > 0 {
                    wort = Some(i);
                }
            }
            if ende == 0 {
                return Err(fehler("Zeichen passt nicht auf die Seite"));
            }
            if ende == rest.len() {
                result.push(rest.to_owned());
                break;
            }
            let e = wort.unwrap_or(ende);
            result.push(rest[..e].to_owned());
            rest = rest[e..].trim_start_matches(' ');
        }
        if absatz.is_empty() {
            result.push(String::new());
        }
    }
    Ok(result)
}
pub(super) fn bericht(
    projekt: &str,
    titel: &str,
    zeilen: &[Zeile],
    _datum: DateTime<Utc>,
) -> CoreResult<Document> {
    super::layout::bericht(projekt, titel, zeilen)
}
/// Seitenauswahl ist 1-basiert, geordnet und ohne doppelte Seiten.
pub(super) fn auswahl(bytes: &[u8], seiten: &[u32]) -> CoreResult<Document> {
    let mut d = Document::load_mem(bytes).map_err(fehler)?;
    if d.is_encrypted() {
        return Err(fehler("Verschlüsseltes PDF zuerst entsperren"));
    }
    let pages = d.get_pages();
    for id in pages.values() {
        d.get_dictionary_mut(*id)
            .map_err(fehler)?
            .remove(b"BauBericht");
    }
    if pages.is_empty() || pages.len() > MAX_SEITEN {
        return Err(fehler("PDF benötigt 1 bis 300 Seiten"));
    }
    if !seiten.is_empty() {
        let mut seen = std::collections::HashSet::new();
        if seiten.len() > MAX_SEITEN
            || seiten
                .iter()
                .any(|n| !pages.contains_key(n) || !seen.insert(n))
        {
            return Err(fehler("Seitenauswahl ungültig oder doppelt"));
        }
        let root = d.new_object_id();
        let mut kids = Vec::new();
        for n in seiten {
            let id = pages[n];
            let mut page = geerbte_seite(&d, id)?;
            page.set("Parent", root);
            d.objects.insert(id, Object::Dictionary(page));
            kids.push(id.into());
        }
        d.objects.insert(
            root,
            Object::Dictionary(
                dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>seiten.len() as i64},
            ),
        );
        d.catalog_mut().map_err(fehler)?.set("Pages", root);
    }
    Ok(d)
}
fn geerbte_seite(d: &Document, id: ObjectId) -> CoreResult<Dictionary> {
    let mut result = d.get_dictionary(id).map_err(fehler)?.clone();
    let mut current = id;
    let mut seen = std::collections::HashSet::new();
    while seen.insert(current) && seen.len() < 100 {
        let dict = d.get_dictionary(current).map_err(fehler)?;
        for k in [b"Resources".as_slice(), b"MediaBox", b"CropBox", b"Rotate"] {
            if !result.has(k)
                && let Ok(v) = dict.get(k)
            {
                result.set(k, v.clone());
            }
        }
        if let Ok(parent) = dict.get(b"Parent").and_then(Object::as_reference) {
            current = parent;
        } else {
            return Ok(result);
        }
    }
    Err(fehler("Zyklischer oder zu tiefer PDF-Seitenbaum"))
}
fn zahlen(d: &Document, o: &Object, len: usize) -> CoreResult<Vec<f32>> {
    let (_, o) = d.dereference(o).map_err(fehler)?;
    let a = o.as_array().map_err(fehler)?;
    if a.len() != len {
        return Err(fehler("Ungültige PDF-Geometrie"));
    }
    a.iter()
        .map(|v| {
            let n = v.as_float().map_err(fehler)?;
            if !n.is_finite() {
                return Err(fehler("Ungültige PDF-Koordinate"));
            }
            Ok(n)
        })
        .collect()
}
/// Widgets werden über ihre vorhandenen Appearance-Streams in Seiteninhalt
/// überführt. So bleiben auch gleichnamige Felder verschiedener Vorlagen sichtbar.
pub(super) fn formulare_einfrieren(d: &mut Document) -> CoreResult<()> {
    for (_, id) in d.get_pages() {
        let mut page = geerbte_seite(d, id)?;
        page.remove(b"AA");
        let annots = page
            .get(b"Annots")
            .ok()
            .map(|o| d.dereference(o).map(|(_, o)| o.clone()))
            .transpose()
            .map_err(fehler)?
            .and_then(|o| o.as_array().ok().cloned())
            .unwrap_or_default();
        let mut behalten = Vec::new();
        let mut ops = Vec::new();
        let mut resources = page
            .get(b"Resources")
            .ok()
            .and_then(|o| d.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        let mut x = resources
            .get(b"XObject")
            .ok()
            .and_then(|o| d.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        for (i, o) in annots.into_iter().enumerate() {
            let (_, v) = d.dereference(&o).map_err(fehler)?;
            let mut a = v.as_dict().map_err(fehler)?.clone();
            a.remove(b"A");
            a.remove(b"AA");
            if a.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Widget") {
                // Nicht druckrelevante Aktionen/Dateianhänge werden nicht übernommen.
                if a.get(b"Subtype")
                    .and_then(Object::as_name)
                    .ok()
                    .is_some_and(|s| {
                        [
                            b"Text".as_slice(),
                            b"FreeText",
                            b"Highlight",
                            b"Ink",
                            b"Stamp",
                            b"Square",
                            b"Circle",
                            b"Line",
                            b"Polygon",
                            b"PolyLine",
                            b"Underline",
                            b"StrikeOut",
                        ]
                        .contains(&s)
                    })
                {
                    behalten.push(d.add_object(a).into());
                }
                continue;
            }
            let flags = a.get(b"F").and_then(Object::as_i64).unwrap_or(0);
            if flags & 3 != 0 {
                continue;
            }
            let normal = a
                .get(b"AP")
                .ok()
                .and_then(|o| d.dereference(o).ok())
                .and_then(|(_, o)| o.as_dict().ok())
                .and_then(|a| a.get(b"N").ok());
            let appearance = normal
                .and_then(|n| d.dereference(n).ok())
                .and_then(|(_, n)| {
                    if n.as_stream().is_ok() {
                        Some(n.clone())
                    } else {
                        let state = a.get(b"AS").and_then(Object::as_name).unwrap_or(b"Off");
                        n.as_dict()
                            .ok()?
                            .get(state)
                            .ok()
                            .and_then(|v| d.dereference(v).ok())
                            .map(|(_, v)| v.clone())
                    }
                });
            let Some(appearance) = appearance else {
                // Fehlende Darstellung gefüllter Felder niemals still als leer ausgeben.
                let mut field = a.clone();
                let mut value = false;
                let mut visited = std::collections::HashSet::new();
                for _ in 0..100 {
                    if let Ok(v) = field.get(b"V") {
                        value = !matches!(v, Object::Null)
                            && v.as_str().map(|s| !s.is_empty()).unwrap_or(true);
                        break;
                    }
                    let Ok(parent) = field.get(b"Parent").and_then(Object::as_reference) else {
                        break;
                    };
                    if !visited.insert(parent) {
                        return Err(fehler("Zyklische PDF-Feldhierarchie"));
                    }
                    field = d.get_dictionary(parent).map_err(fehler)?.clone();
                }
                if value {
                    return Err(fehler(
                        "PDF enthält ausgefüllte Felder ohne Druckdarstellung. Im PDF-Programm speichern oder mit der Formularfunktion neu befüllen.",
                    ));
                }
                continue;
            };
            let stream = appearance.as_stream().map_err(fehler)?;
            let bbox = zahlen(d, stream.dict.get(b"BBox").map_err(fehler)?, 4)?;
            let m = stream
                .dict
                .get(b"Matrix")
                .ok()
                .map(|v| zahlen(d, v, 6))
                .transpose()?
                .unwrap_or_else(|| vec![1., 0., 0., 1., 0., 0.]);
            let mut xx = Vec::new();
            let mut yy = Vec::new();
            for px in [bbox[0], bbox[2]] {
                for py in [bbox[1], bbox[3]] {
                    xx.push(m[0] * px + m[2] * py + m[4]);
                    yy.push(m[1] * px + m[3] * py + m[5]);
                }
            }
            let minx = xx.iter().copied().fold(f32::INFINITY, f32::min);
            let maxx = xx.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let miny = yy.iter().copied().fold(f32::INFINITY, f32::min);
            let maxy = yy.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let rect = zahlen(d, a.get(b"Rect").map_err(fehler)?, 4)?;
            if maxx <= minx || maxy <= miny {
                return Err(fehler("Leere PDF-Felddarstellung"));
            }
            let sx = (rect[2] - rect[0]) / (maxx - minx);
            let sy = (rect[3] - rect[1]) / (maxy - miny);
            let name = format!("BauDruckFeld{i}");
            if x.has(name.as_bytes()) {
                return Err(fehler("Kollision im PDF-Ressourcenverzeichnis"));
            }
            let aid = d.add_object(appearance);
            x.set(name.clone(), aid);
            ops.extend([
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        sx.into(),
                        0.into(),
                        0.into(),
                        sy.into(),
                        (rect[0] - minx * sx).into(),
                        (rect[1] - miny * sy).into(),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(name.into_bytes())]),
                Operation::new("Q", vec![]),
            ]);
        }
        if !ops.is_empty() {
            let content = d.add_object(Stream::new(
                dictionary! {},
                Content { operations: ops }.encode().map_err(fehler)?,
            ));
            let mut contents = match page
                .get(b"Contents")
                .ok()
                .map(|o| d.dereference(o).map(|(_, v)| v))
                .transpose()
                .map_err(fehler)?
            {
                Some(Object::Array(a)) => a.clone(),
                Some(_) => vec![page.get(b"Contents").map_err(fehler)?.clone()],
                None => vec![],
            };
            contents.push(content.into());
            page.set("Contents", contents);
        }
        resources.set("XObject", x);
        page.set("Resources", resources);
        page.set("Annots", behalten);
        d.objects.insert(id, Object::Dictionary(page));
    }
    Ok(())
}
pub(super) fn paket(
    projekt: &str,
    a: Druckauftrag,
    mut quellen: Vec<(Inhaltsangabe, Document)>,
    datum: DateTime<Utc>,
) -> CoreResult<Druckergebnis> {
    let mut vorlauf = Vec::new();
    if a.mit_anschreiben {
        let z = vec![
            text(&a.absender),
            text(""),
            text(&a.empfaenger),
            text(""),
            text(
                a.datum
                    .unwrap_or(datum.date_naive())
                    .format("%d.%m.%Y")
                    .to_string(),
            ),
            text(""),
            super::berichte::titel(&a.titel),
            text(format!("Bauvorhaben: {projekt}")),
            text(if a.aktenzeichen.is_empty() {
                String::new()
            } else {
                format!("Aktenzeichen / Bezug: {}", a.aktenzeichen)
            }),
            text(""),
            text(&a.anschreiben),
        ];
        vorlauf.push(bericht(projekt, "Anschreiben", &z, datum)?);
    }
    let vorne = vorlauf
        .iter()
        .map(|d| d.get_pages().len() as u32)
        .sum::<u32>();
    let mut toc_seiten = 0;
    let toc = |offset: u32| -> CoreResult<Document> {
        let mut seite = vorne + offset + 1;
        let mut rows = Vec::new();
        for (i, (titel, d)) in quellen.iter().enumerate() {
            let n = d.get_pages().len() as u32;
            rows.push(vec![
                (i + 1).to_string(),
                titel.kategorie.clone(),
                titel.name.clone(),
                titel.version.clone(),
                if n == 1 {
                    seite.to_string()
                } else {
                    format!("{seite}–{}", seite + n - 1)
                },
            ]);
            seite += n;
        }
        let z = vec![
            text(format!("Bauvorhaben: {projekt}")),
            super::berichte::tabelle(
                &[
                    "Nr.",
                    "Dokumentkategorie",
                    "Dateiname / Bezeichnung",
                    "Version / Stand",
                    "Seite",
                ],
                &[0.45, 1.4, 2.6, 1.3, 0.7],
                rows,
            ),
        ];
        bericht(projekt, "Inhaltsverzeichnis", &z, datum)
    };
    if a.inhaltsverzeichnis {
        toc_seiten = toc(0)?.get_pages().len() as u32;
        let d = toc(toc_seiten)?;
        if d.get_pages().len() as u32 != toc_seiten {
            return Err(fehler(
                "Inhaltsverzeichnis konnte nicht stabil paginiert werden",
            ));
        }
        vorlauf.push(d);
    }
    let mut erste = vorne + toc_seiten + 1;
    let mut positionen = Vec::new();
    for (titel, d) in &quellen {
        let seiten = d.get_pages().len() as u32;
        positionen.push(Druckposition {
            titel: titel.titel.clone(),
            erste_seite: erste,
            seiten,
        });
        erste += seiten;
    }
    if erste - 1 > MAX_SEITEN as u32 {
        return Err(fehler("Druckpaket über 300 Seiten; bitte aufteilen"));
    }
    let mut out = Document::with_version("1.7");
    let root = out.new_object_id();
    let mut kids = Vec::new();
    for mut d in vorlauf.into_iter().chain(quellen.drain(..).map(|(_, d)| d)) {
        d.renumber_objects_with(out.max_id + 1);
        let pages = d.get_pages();
        for id in pages.values() {
            let mut page = geerbte_seite(&d, *id)?;
            page.set("Parent", root);
            page.remove(b"AA");
            d.objects.insert(*id, Object::Dictionary(page));
            kids.push((*id).into());
        }
        out.max_id = d.max_id;
        out.objects.extend(d.objects);
    }
    out.objects.insert(
        root,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>(erste-1) as i64}),
    );
    let cat = out.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    out.trailer.set("Root", cat);
    // Entfernt u.a. nicht ausgewählte Seiten, ursprüngliche Formfelder und Katalog-Aktionen.
    super::layout::briefbogen(&mut out, projekt, &a, a.datum.unwrap_or(datum.date_naive()))?;
    out.prune_objects();
    Ok(Druckergebnis {
        pdf: speichern(out)?,
        positionen,
        seiten: erste - 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lange_unicode_berichte_und_verzeichnis_bleiben_paginiert() {
        let datum = Utc::now();
        let zeilen = (0..140)
            .map(|n| text(format!("Position {n}: Müller & Söhne – Fläche 123,45 m²")))
            .collect::<Vec<_>>();
        let bericht = bericht("Projekt", "Berechnung", &zeilen, datum).unwrap();
        assert!(bericht.get_pages().len() > 3);
        let auftrag = Druckauftrag {
            gestaltung: None,
            aktenzeichen: String::new(),
            datum: None,
            titel: "Unterlagen".into(),
            absender: "Büro Müller".into(),
            empfaenger: "Bauamt".into(),
            anschreiben: "Langer Text\n".repeat(160),
            mit_anschreiben: true,
            inhaltsverzeichnis: true,
            projekt_version: 1,
            quellen: vec![],
        };
        let quellen=(0..80).map(|i|(format!("Unterlage {i} mit einer ausführlichen Bezeichnung zur Kontrolle des Umbruchs und der Seitenzahlen").into(),bericht.clone())).collect();
        // 80 * vierseitige Berichte überschreiten die Stapelgrenze ausdrücklich.
        assert!(paket("Projekt", auftrag.clone(), quellen, datum).is_err());
        let leer = super::bericht("Projekt", "Einzelblatt", &[text("Müller")], datum).unwrap();
        let quellen=(0..80).map(|i|(format!("Unterlage {i} mit ausführlicher Bezeichnung zur Kontrolle des Umbruchs und der Seitenzahlen").into(),leer.clone())).collect();
        let out = paket("Projekt", auftrag, quellen, datum).unwrap();
        assert_eq!(out.positionen.len(), 80);
        assert!(out.positionen[0].erste_seite > 7);
        assert_eq!(out.positionen[79].erste_seite, out.seiten);
    }
    #[test]
    fn fotos_erhalten_proportionen_und_transparenz_auf_weiss() {
        let img = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 0, 0, 0]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let d = bild(bytes.get_ref()).unwrap();
        let page = *d.get_pages().values().next().unwrap();
        let bbox = d
            .get_dictionary(page)
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(bbox[2].as_float().unwrap() > bbox[3].as_float().unwrap());
        assert!(
            d.objects
                .values()
                .filter_map(|o| o.as_stream().ok())
                .any(|s| s
                    .decompressed_content()
                    .unwrap_or_else(|_| s.content.clone())
                    == vec![255; 24])
        );
        assert!(bild(b"kein Foto").is_err());
    }
    #[test]
    fn gefuellte_felder_ohne_appearance_werden_nicht_leer_gedruckt() {
        let bytes = crate::formular::testdaten::musterformular();
        let mut d = Document::load_mem(&bytes).unwrap();
        let id = d
            .objects
            .iter()
            .find_map(|(id, o)| {
                o.as_dict()
                    .ok()
                    .filter(|d| {
                        d.get(b"T").ok().and_then(|v| v.as_str().ok()) == Some(b"Bauherr_Name")
                    })
                    .map(|_| *id)
            })
            .unwrap();
        d.get_dictionary_mut(id)
            .unwrap()
            .set("V", Object::string_literal("Daniel"));
        assert!(
            formulare_einfrieren(&mut d)
                .unwrap_err()
                .to_string()
                .contains("ohne Druckdarstellung")
        );
    }
}
