//! ZIP-Paket mit der XBau-Nachricht und ihren Anlagen.
//!
//! Aufbau: `<nachricht>.xml` im Wurzelverzeichnis, die Anlagen unter dem
//! Dateinamen, den die Nachricht in `anhang/dateiname` nennt.

use std::collections::HashSet;
use std::io::{Cursor, Write};

use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::error::{CoreError, CoreResult};

pub const NACHRICHT_DATEI: &str = "baugenehmigung.antrag.0200.xml";
/// Nachrichtendateien der Antragsnachrichten; Anlagen dürfen sie nicht überdecken.
const NACHRICHTEN_DATEIEN: [&str; 5] = [
    NACHRICHT_DATEI,
    "baugenehmigung.antragGeaendert.0202.xml",
    "vorbescheid.antrag.0210.xml",
    "vorbescheid.antragGeaendert.0212.xml",
    "genehmigungsfreistellung.anzeige.0600.xml",
];

pub fn packen(xml: &str, anlagen: &[(String, &[u8])]) -> CoreResult<Vec<u8>> {
    packen_nachricht(NACHRICHT_DATEI, xml, anlagen)
}
pub fn packen_nachricht(
    nachricht: &str,
    xml: &str,
    anlagen: &[(String, &[u8])],
) -> CoreResult<Vec<u8>> {
    if anlagen.len() > 255
        || xml.len() as u64 > super::austausch::MAX_XML
        || anlagen
            .iter()
            .try_fold(xml.len() as u64, |sum, (_, b)| {
                sum.checked_add(b.len() as u64)
            })
            .is_none_or(|s| s > super::austausch::MAX_ENTPACKT)
    {
        return Err(super::austausch::fehler(
            "Paket überschreitet die zulässigen Größen oder die Anlagenanzahl",
        ));
    }
    super::austausch::dateiname(nachricht)?;
    let mut namen = HashSet::from([nachricht.to_lowercase()]);
    for (n, _) in anlagen {
        super::austausch::dateiname(n)?;
        if !namen.insert(n.to_lowercase()) {
            return Err(super::austausch::fehler("Doppelte Dateinamen im Paket"));
        }
    }
    let fehler = |e: zip::result::ZipError| CoreError::Storage(format!("ZIP: {e}"));
    let io = |e: std::io::Error| CoreError::Storage(format!("ZIP: {e}"));
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(nachricht, opt).map_err(fehler)?;
    zip.write_all(xml.as_bytes()).map_err(io)?;
    for (name, inhalt) in anlagen {
        zip.start_file(name.as_str(), opt).map_err(fehler)?;
        zip.write_all(inhalt).map_err(io)?;
    }
    Ok(zip.finish().map_err(fehler)?.into_inner())
}

/// Macht Dateinamen im Paket eindeutig (`plan.pdf`, `plan-2.pdf`, …) und
/// entfernt Pfadanteile.
pub fn eindeutige_namen<'a>(namen: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut vergeben: HashSet<String> = NACHRICHTEN_DATEIEN
        .iter()
        .map(|n| n.to_lowercase())
        .collect();
    namen
        .into_iter()
        .map(|n| {
            let n = n.rsplit(['/', '\\']).next().unwrap_or(n).trim();
            let n = if n.is_empty() { "anlage" } else { n };
            let (stamm, endung) = match n.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
                _ => (n, String::new()),
            };
            let mut kandidat = n.to_string();
            let mut i = 2;
            while !vergeben.insert(kandidat.to_lowercase()) {
                kandidat = format!("{stamm}-{i}{endung}");
                i += 1;
            }
            kandidat
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn namen_werden_eindeutig() {
        assert_eq!(
            eindeutige_namen(["plan.pdf", "Plan.pdf", "a/b/plan.pdf", "", "x"]),
            ["plan.pdf", "Plan-2.pdf", "plan-3.pdf", "anlage", "x"]
        );
    }

    #[test]
    fn paket_enthaelt_nachricht_und_anlagen() {
        let zip = packen("<a/>", &[("plan.pdf".into(), b"%PDF")]).unwrap();
        let mut archiv = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
        let mut s = String::new();
        archiv
            .by_name(NACHRICHT_DATEI)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "<a/>");
        let mut b = Vec::new();
        archiv
            .by_name("plan.pdf")
            .unwrap()
            .read_to_end(&mut b)
            .unwrap();
        assert_eq!(b, b"%PDF");
    }
}
