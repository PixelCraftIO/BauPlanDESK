use super::*;
use std::io::{Cursor, Write};

#[test]
fn eml_mime_adressen_zeichensaetze_anhaenge_und_html_als_text() {
    let eml=b"From: =?UTF-8?Q?Daniel_M=C3=BCller?= <daniel@example.org>\r\nTo: Bauamt <amt@example.org>\r\nCc: Planung <plan@example.org>\r\nDate: Thu, 15 Oct 2026 10:30:00 +0200\r\nSubject: =?UTF-8?B?TsOkY2hmb3JkZXJ1bmc=?=\r\nMessage-ID: <antrag-42@example.org>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=teil\r\n\r\n--teil\r\nContent-Type: text/plain; charset=iso-8859-1\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nGr=FC=DFe vom Bauamt.\r\n--teil\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=Plan.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nUERG\r\n--teil--\r\n";
    let m = lesen("Bauamt.EML", eml).unwrap();
    assert_eq!(m.betreff, "Nächforderung");
    assert_eq!(m.absender[0].name, "Daniel Müller");
    assert_eq!(m.empfaenger[0].adresse, "amt@example.org");
    assert_eq!(m.cc[0].adresse, "plan@example.org");
    assert_eq!(m.datum.as_deref(), Some("2026-10-15T10:30:00+02:00"));
    assert_eq!(m.nachrichten_id.as_deref(), Some("antrag-42@example.org"));
    assert!(m.text.contains("Grüße vom Bauamt."));
    assert_eq!(m.anhaenge[0].name, "Plan.pdf");
    assert_eq!(m.anhaenge[0].groesse_bytes, Some(3));
    let html=b"Subject: HTML\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body><p>Nachforderung &amp; Bauantrag</p><img src=\"https://example.org/tracker\"><script>alert('x')</script></body></html>";
    let m = lesen("HTML.eml", html).unwrap();
    assert!(m.text.contains("Nachforderung & Bauantrag"));
    assert!(!m.text.contains("<script>"));
    assert!(!m.text.contains("<img"));
    assert!(lesen("Falsch.eml", b"kein Mailformat").is_err());
    assert!(lesen("Falsch.msg", eml).is_err());
    assert!(lesen("Falsch.pdf", eml).is_err());
}

fn eigenschaft(tag: u32, wert: u64) -> Vec<u8> {
    let mut p = vec![0; 16];
    p[..4].copy_from_slice(&tag.to_le_bytes());
    p[8..].copy_from_slice(&wert.to_le_bytes());
    p
}
fn schreiben(c: &mut cfb::CompoundFile<Cursor<Vec<u8>>>, name: &str, data: &[u8]) {
    c.create_stream(name).unwrap().write_all(data).unwrap();
}
fn unicode(c: &mut cfb::CompoundFile<Cursor<Vec<u8>>>, root: &str, id: u16, s: &str) {
    let data: Vec<_> = s
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    schreiben(c, &format!("{root}/__substg1.0_{id:04X}001F"), &data);
}
#[test]
fn msg_unicode_ansi_empfaenger_datum_und_anhaenge() {
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    let mut p = vec![0; 32];
    p.extend(eigenschaft(0x3FFD0003, (3u64 << 32) | 1252));
    p.extend(eigenschaft(0x00390040, 134_365_266_000_000_000));
    schreiben(&mut c, "/__properties_version1.0", &p);
    unicode(&mut c, "", 0x0037, "Nachforderung zum Bauantrag");
    unicode(&mut c, "", 0x0C1A, "Bauamt München");
    unicode(&mut c, "", 0x5D01, "amt@example.org");
    schreiben(
        &mut c,
        "/__substg1.0_1000001E",
        b"Bitte erg\xe4nzen Sie den Lageplan.\0",
    );
    for (n, typ) in [(0, 1), (1, 2), (2, 3)] {
        let root = format!("/__recip_version1.0_#{n:08X}");
        c.create_storage(&root).unwrap();
        let mut p = vec![0; 8];
        p.extend(eigenschaft(0x0C150003, typ));
        schreiben(&mut c, &format!("{root}/__properties_version1.0"), &p);
        unicode(&mut c, &root, 0x3001, "Daniel Müller");
        unicode(&mut c, &root, 0x39FE, &format!("plan{n}@example.org"));
    }
    let root = "/__attach_version1.0_#00000000";
    c.create_storage(root).unwrap();
    unicode(&mut c, root, 0x3707, "Prüfung.pdf");
    unicode(&mut c, root, 0x370E, "application/pdf");
    schreiben(&mut c, &format!("{root}/__substg1.0_37010102"), b"PDF");
    let original = c.into_inner().into_inner();
    let m = lesen("Bauamt.msg", &original).unwrap();
    assert_eq!(m.betreff, "Nachforderung zum Bauantrag");
    assert_eq!(m.absender[0].name, "Bauamt München");
    assert_eq!(m.absender[0].adresse, "amt@example.org");
    assert!(m.text.contains("ergänzen"));
    assert_eq!(m.empfaenger[0].adresse, "plan0@example.org");
    assert_eq!(m.cc[0].adresse, "plan1@example.org");
    assert_eq!(m.bcc[0].adresse, "plan2@example.org");
    assert!(m.datum.as_deref().unwrap().starts_with("2026-"));
    assert_eq!(m.anhaenge[0].name, "Prüfung.pdf");
    assert_eq!(m.anhaenge[0].groesse_bytes, Some(3));
    // Ein nicht verwendetes Ersatzfeld darf eine lesbare SMTP-Adresse nicht verhindern.
    let mut original_cfb = cfb::CompoundFile::open(Cursor::new(original)).unwrap();
    schreiben(&mut original_cfb, "/__substg1.0_0C1F001F", &[0x41]);
    let original = original_cfb.into_inner().into_inner();
    assert_eq!(
        lesen("Mit Ersatzfeld.msg", &original).unwrap().absender[0].adresse,
        "amt@example.org"
    );
    // Unvollständige Unicode-Streams ergeben keine still verfälschte Vorschau.
    let mut c = cfb::CompoundFile::open(Cursor::new(original)).unwrap();
    schreiben(&mut c, "/__substg1.0_0037001F", &[0x41]);
    assert!(lesen("Kaputt.msg", &c.into_inner().into_inner()).is_err());
}

#[test]
fn msg_html_und_rtf_hinweis() {
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    schreiben(&mut c, "/__properties_version1.0", &[0; 32]);
    unicode(&mut c, "", 0x0037, "Nur HTML");
    schreiben(
        &mut c,
        "/__substg1.0_10130102",
        b"<p>Bitte <b>Lageplan</b> nachreichen.</p>",
    );
    let m = lesen("HTML.msg", &c.into_inner().into_inner()).unwrap();
    assert!(m.text.contains("Lageplan"));
    assert!(!m.text.contains("<b>"));
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    schreiben(&mut c, "/__properties_version1.0", &[0; 32]);
    schreiben(&mut c, "/__substg1.0_10090102", b"RTF");
    let m = lesen("RTF.msg", &c.into_inner().into_inner()).unwrap();
    assert!(m.text.is_empty());
    assert!(m.hinweise[0].contains("RTF"));
}

#[test]
fn msg_html_verwendet_die_internet_codepage() {
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    let mut props = vec![0; 32];
    props.extend(eigenschaft(0x3FFD0003, (9u64 << 32) | 1252));
    props.extend(eigenschaft(0x3FDE0003, 65001));
    schreiben(&mut c, "/__properties_version1.0", &props);
    schreiben(
        &mut c,
        "/__substg1.0_10130102",
        "<p>Grüße aus München</p>".as_bytes(),
    );
    let m = lesen("HTML.msg", &c.into_inner().into_inner()).unwrap();
    assert!(m.text.contains("Grüße aus München"));
}

#[test]
fn eingebettete_msg_anhaenge_bekommen_keine_erfundene_dateigroesse() {
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    schreiben(&mut c, "/__properties_version1.0", &[0; 32]);
    let root = "/__attach_version1.0_#00000000";
    c.create_storage_all(format!("{root}/__substg1.0_3701000D"))
        .unwrap();
    unicode(&mut c, root, 0x3707, "Weitergeleitete Nachricht.msg");
    let mail = lesen("Anhang.msg", &c.into_inner().into_inner()).unwrap();
    assert!(mail.anhaenge[0].eingebettete_nachricht);
    assert_eq!(mail.anhaenge[0].groesse_bytes, None);
    assert!(serde_json::to_value(&mail.anhaenge[0]).unwrap()["groesse_bytes"].is_null());
}
