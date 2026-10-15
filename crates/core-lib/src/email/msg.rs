//! Outlook MSG: CFB-Container mit MAPI-Textfeldern, Empfängern und Anhangsangaben.
use super::*;
use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};
type Container<'a> = cfb::CompoundFile<Cursor<&'a [u8]>>;
pub(super) fn anhang(daten: &[u8], index: usize) -> CoreResult<Vec<u8>> {
    let mut c = cfb::CompoundFile::open(Cursor::new(daten)).map_err(io)?;
    let pfad = c
        .walk()
        .filter(|e| {
            e.is_storage()
                && e.path().parent() == Some(Path::new("/"))
                && e.path()
                    .to_string_lossy()
                    .starts_with("/__attach_version1.0_#")
        })
        .nth(index)
        .map(|e| e.path().to_string_lossy().into_owned())
        .ok_or_else(|| fehler("MSG-Anhang fehlt"))?;
    stream_mit_limit(&mut c, &format!("{pfad}/__substg1.0_37010102"), MAX_BYTES)?
        .ok_or_else(|| fehler("MSG-Anhang enthält keine unterstützten Dateidaten"))
}
fn io(e: std::io::Error) -> CoreError {
    fehler(format!("MSG-Datei konnte nicht gelesen werden: {e}"))
}
fn stream(c: &mut Container<'_>, path: &str) -> CoreResult<Option<Vec<u8>>> {
    stream_mit_limit(c, path, 64 * 1024)
}
fn stream_mit_limit(c: &mut Container<'_>, path: &str, limit: u64) -> CoreResult<Option<Vec<u8>>> {
    if !c.is_stream(path) {
        return Ok(None);
    }
    let s = c.open_stream(path).map_err(io)?;
    if s.len() > limit {
        return Err(fehler("Ein MSG-Textbereich ist zu groß für die Vorschau"));
    }
    let mut daten = Vec::new();
    s.take(limit + 1).read_to_end(&mut daten).map_err(io)?;
    if daten.len() as u64 > limit {
        return Err(fehler("Ein MSG-Textbereich ist zu groß für die Vorschau"));
    }
    Ok(Some(daten))
}
fn wert(props: &[u8], header: usize, tag: u32) -> CoreResult<Option<u64>> {
    let daten = props
        .get(header..)
        .ok_or_else(|| fehler("MSG-Eigenschaftenkopf ist beschädigt"))?;
    if !daten.len().is_multiple_of(16) {
        return Err(fehler("MSG-Eigenschaften sind unvollständig"));
    }
    Ok(daten.as_chunks::<16>().0.iter().find_map(|p| {
        (u32::from_le_bytes(p[..4].try_into().ok()?) == tag).then(|| {
            let value = u64::from_le_bytes(p[8..16].try_into().expect("16-Byte-Eigenschaft"));
            // PT_LONG nutzt vier Bytes; die übrigen vier sind reserviert.
            if tag & 0xFFFF == 0x0003 {
                value & 0xFFFF_FFFF
            } else {
                value
            }
        })
    }))
}
fn text(
    c: &mut Container<'_>,
    root: &str,
    id: u16,
    encoding: &'static encoding_rs::Encoding,
) -> CoreResult<Option<String>> {
    let limit = if matches!(id, 0x1000 | 0x1013) {
        8 * 1024 * 1024
    } else {
        64 * 1024
    };
    if let Some(v) = stream_mit_limit(c, &format!("{root}/__substg1.0_{id:04X}001F"), limit)? {
        if !v.len().is_multiple_of(2) {
            return Err(fehler("MSG-Unicode-Text ist unvollständig"));
        }
        let u: Vec<_> = v
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        return String::from_utf16(&u)
            .map(|s| Some(s.trim_end_matches('\0').into()))
            .map_err(|_| fehler("MSG enthält ungültigen Unicode-Text"));
    }
    if let Some(v) = stream_mit_limit(c, &format!("{root}/__substg1.0_{id:04X}001E"), limit)? {
        let (s, _, fehlerhaft) = encoding.decode(&v);
        if fehlerhaft {
            return Err(fehler("MSG-Text passt nicht zum angegebenen Zeichensatz"));
        }
        return Ok(Some(s.trim_end_matches('\0').into()));
    }
    Ok(None)
}
fn text_oder(
    c: &mut Container<'_>,
    root: &str,
    id: u16,
    ersatz: u16,
    encoding: &'static encoding_rs::Encoding,
) -> CoreResult<Option<String>> {
    match text(c, root, id, encoding)?.filter(|s| !s.is_empty()) {
        Some(s) => Ok(Some(s)),
        None => text(c, root, ersatz, encoding),
    }
}
fn adresse(
    c: &mut Container<'_>,
    root: &str,
    name: u16,
    smtp: u16,
    fallback: u16,
    encoding: &'static encoding_rs::Encoding,
) -> CoreResult<MailAdresse> {
    Ok(MailAdresse {
        name: text(c, root, name, encoding)?.unwrap_or_default(),
        adresse: text_oder(c, root, smtp, fallback, encoding)?.unwrap_or_default(),
    })
}
fn datum(v: Option<u64>) -> Option<String> {
    let ticks = i128::from(v.filter(|v| *v != 0)?) - 116_444_736_000_000_000i128;
    let secs = i64::try_from(ticks.div_euclid(10_000_000)).ok()?;
    let nanos = (ticks.rem_euclid(10_000_000) * 100) as u32;
    chrono::DateTime::from_timestamp(secs, nanos).map(|d| d.to_rfc3339())
}
fn encoding_fuer(codepage: u64) -> CoreResult<&'static encoding_rs::Encoding> {
    let label = match codepage {
        65001 => "utf-8".into(),
        1200 => "utf-16le".into(),
        1201 => "utf-16be".into(),
        932 => "shift_jis".into(),
        936 => "gbk".into(),
        949 => "euc-kr".into(),
        950 => "big5".into(),
        20127 => "us-ascii".into(),
        28591..=28606 => format!("iso-8859-{}", codepage - 28590),
        _ => format!("windows-{codepage}"),
    };
    encoding_rs::Encoding::for_label(label.as_bytes()).ok_or_else(|| {
        fehler(format!(
            "MSG-Zeichensatz {codepage} wird nicht unterstützt. Bitte das Original extern öffnen."
        ))
    })
}
pub(super) fn lesen(daten: &[u8]) -> CoreResult<EmailInhalt> {
    let mut c = cfb::CompoundFile::open(Cursor::new(daten)).map_err(io)?;
    let props = stream(&mut c, "/__properties_version1.0")?
        .ok_or_else(|| fehler("Der Container enthält keine Outlook-Nachricht"))?;
    let codepage = wert(&props, 32, 0x3FFD0003)?
        .or(wert(&props, 32, 0x3FDE0003)?)
        .unwrap_or(1252);
    let encoding = encoding_fuer(codepage)?;
    let mut hinweise = vec![];
    let mut body = text(&mut c, "", 0x1000, encoding)?.unwrap_or_default();
    if body.is_empty() {
        let html = if let Some(h) = text(&mut c, "", 0x1013, encoding)? {
            Some(h)
        } else if let Some(h) = stream_mit_limit(&mut c, "/__substg1.0_10130102", 8 * 1024 * 1024)?
        {
            let html_encoding = if let Some(cp) = wert(&props, 32, 0x3FDE0003)? {
                encoding_fuer(cp)?
            } else if std::str::from_utf8(&h).is_ok() {
                encoding_rs::UTF_8
            } else {
                encoding
            };
            let (s, _, problem) = html_encoding.decode(&h);
            if problem {
                hinweise.push("Der HTML-Text enthält Zeichen mit unklarer Kodierung.".into());
            }
            Some(s.into_owned())
        } else {
            None
        };
        if let Some(html) = html {
            body = mail_parser::decoders::html::html_to_text(&html);
        }
    }
    if body.trim().is_empty() {
        hinweise.push(if c.is_stream("/__substg1.0_10090102") {"Nachrichtentext liegt nur im RTF-Format vor. Bitte das Original im Mailprogramm öffnen."} else {"Kein lesbarer Nachrichtentext vorhanden. Verschlüsselte oder geschützte Nachrichten bitte im Mailprogramm öffnen."}.into());
    }
    let entry_count = c.walk().take(10001).count();
    if entry_count > 10000 {
        return Err(fehler("Die MSG-Datei enthält zu viele Bestandteile"));
    }
    let kinder: Vec<PathBuf> = c
        .walk()
        .filter(|e| e.is_storage() && e.path().parent() == Some(Path::new("/")))
        .map(|e| e.path().to_path_buf())
        .collect();
    let mut empfaenger = vec![];
    let mut cc = vec![];
    let mut bcc = vec![];
    let mut anhaenge = vec![];
    let mut metadaten_bytes = 0usize;
    for pfad in kinder {
        let root = pfad.to_str().ok_or_else(|| fehler("Ungültiger MSG-Pfad"))?;
        if root.starts_with("/__recip_version1.0_#") {
            let p = stream(&mut c, &format!("{root}/__properties_version1.0"))?
                .ok_or_else(|| fehler("MSG-Empfängerangaben fehlen"))?;
            let a = adresse(&mut c, root, 0x3001, 0x39FE, 0x3003, encoding)?;
            metadaten_bytes = metadaten_bytes.saturating_add(a.name.len() + a.adresse.len());
            if metadaten_bytes > 2 * 1024 * 1024 {
                return Err(fehler("Die MSG-Kopfdaten sind zu groß für die Vorschau"));
            }
            match wert(&p, 8, 0x0C150003)? {
                Some(2) => cc.push(a),
                Some(3) => bcc.push(a),
                Some(1) => empfaenger.push(a),
                _ => {
                    hinweise.push("Empfängertyp nicht angegeben; unter An angezeigt.".into());
                    empfaenger.push(a);
                }
            }
        } else if root.starts_with("/__attach_version1.0_#") {
            let name = text_oder(&mut c, root, 0x3707, 0x3704, encoding)?
                .unwrap_or_else(|| "Anhang ohne Dateinamen".into());
            let payload = format!("{root}/__substg1.0_37010102");
            let groesse_bytes = if c.is_stream(&payload) {
                Some(c.entry(&payload).map_err(io)?.len())
            } else {
                None
            };
            let anhang = MailAnhang {
                name,
                medientyp: text(&mut c, root, 0x370E, encoding)?,
                groesse_bytes,
                eingebettete_nachricht: c.is_storage(format!("{root}/__substg1.0_3701000D")),
            };
            metadaten_bytes = metadaten_bytes.saturating_add(
                anhang.name.len() + anhang.medientyp.as_ref().map_or(0, String::len),
            );
            if metadaten_bytes > 2 * 1024 * 1024 {
                return Err(fehler(
                    "Die MSG-Anhangsangaben sind zu groß für die Vorschau",
                ));
            }
            anhaenge.push(anhang);
        }
        if anhaenge.len() > 500 || empfaenger.len() > 500 || cc.len() > 500 || bcc.len() > 500 {
            return Err(fehler(
                "Die MSG-Datei enthält zu viele Empfänger oder Anhänge",
            ));
        }
    }
    let a = adresse(&mut c, "", 0x0C1A, 0x5D01, 0x0C1F, encoding)?;
    let absender = if a.name.is_empty() && a.adresse.is_empty() {
        vec![]
    } else {
        vec![a]
    };
    Ok(EmailInhalt {
        format: "MSG".into(),
        betreff: text(&mut c, "", 0x0037, encoding)?.unwrap_or_default(),
        absender,
        empfaenger,
        cc,
        bcc,
        datum: datum(
            wert(&props, 32, 0x00390040)?
                .filter(|t| *t != 0)
                .or(wert(&props, 32, 0x0E060040)?),
        ),
        nachrichten_id: text(&mut c, "", 0x1035, encoding)?,
        text: body,
        anhaenge,
        hinweise,
    })
}
