//! Kleine Nachrichten ohne reale personenbezogene Daten für Vertrags- und UI-Prüfungen.
use std::io::{Cursor, Write};
pub fn muster_eml() -> Vec<u8> {
    b"From: =?UTF-8?Q?Daniel_M=C3=BCller?= <daniel@example.org>\r\nTo: Bauamt <amt@example.org>\r\nDate: Thu, 15 Oct 2026 10:30:00 +0200\r\nSubject: Bauantrag Musterstrasse\r\nMessage-ID: <antrag-42@example.org>\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=teil\r\n\r\n--teil\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nBitte pruefen Sie den Bauantrag.\r\n--teil\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=Lageplan.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nUERG\r\n--teil--\r\n".to_vec()
}
pub fn muster_msg() -> Vec<u8> {
    let mut c = cfb::CompoundFile::create(Cursor::new(vec![])).unwrap();
    fn schreiben(c: &mut cfb::CompoundFile<Cursor<Vec<u8>>>, path: &str, data: &[u8]) {
        c.create_stream(path).unwrap().write_all(data).unwrap();
    }
    fn prop(tag: u32, value: u64) -> Vec<u8> {
        let mut p = vec![0; 16];
        p[..4].copy_from_slice(&tag.to_le_bytes());
        p[4..8].copy_from_slice(&6u32.to_le_bytes());
        p[8..].copy_from_slice(&value.to_le_bytes());
        p
    }
    fn text(
        c: &mut cfb::CompoundFile<Cursor<Vec<u8>>>,
        root: &str,
        id: u16,
        value: &str,
        props: &mut Vec<u8>,
    ) {
        let data: Vec<_> = value
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        schreiben(c, &format!("{root}/__substg1.0_{id:04X}001F"), &data);
        props.extend(prop(((id as u32) << 16) | 0x1F, data.len() as u64));
    }
    let mut p = vec![0; 32];
    for offset in [8, 12, 16, 20] {
        p[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
    }
    // Nichtnull-Reservierungsbytes dürfen PT_LONG nicht zu einem 64-Bit-Wert machen.
    p.extend(prop(0x3FFD0003, (3u64 << 32) | 1252));
    p.extend(prop(0x00390040, 134_365_266_000_000_000));
    for (id, value) in [
        (0x001A, "IPM.Note"),
        (0x0037, "Nachforderung Lageplan"),
        (0x0C1A, "Bauamt München"),
        (0x5D01, "amt@example.org"),
        (
            0x1000,
            "Bitte ergänzen Sie den Lageplan.\nMit freundlichen Grüßen\nIhr Bauamt",
        ),
        (0x1035, "<nachforderung-42@example.org>"),
    ] {
        text(&mut c, "", id, value, &mut p);
    }
    schreiben(&mut c, "/__properties_version1.0", &p);
    let root = "/__recip_version1.0_#00000000";
    c.create_storage(root).unwrap();
    let mut p = vec![0; 8];
    p.extend(prop(0x0C150003, 1));
    text(&mut c, root, 0x3001, "Daniel Müller", &mut p);
    text(&mut c, root, 0x39FE, "daniel@example.org", &mut p);
    schreiben(&mut c, &format!("{root}/__properties_version1.0"), &p);
    let root = "/__attach_version1.0_#00000000";
    c.create_storage(root).unwrap();
    let mut p = vec![0; 8];
    text(&mut c, root, 0x3707, "Anmerkungen.pdf", &mut p);
    text(&mut c, root, 0x370E, "application/pdf", &mut p);
    p.extend(prop(0x37050003, 1));
    p.extend(prop(0x37010102, 3));
    schreiben(&mut c, &format!("{root}/__properties_version1.0"), &p);
    schreiben(&mut c, &format!("{root}/__substg1.0_37010102"), b"PDF");
    c.into_inner().into_inner()
}
