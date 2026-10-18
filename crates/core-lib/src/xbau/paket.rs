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

/// Macht Dateinamen im Paket eindeutig (`plan.pdf`, `plan-2.pdf`, …),
/// entfernt Pfadanteile und bildet portable Namen von höchstens
/// [`MAX_DATEINAME`] Byte (siehe [`crate::dateiname`]). Die Endung bleibt
/// beim Kürzen erhalten.
pub fn eindeutige_namen<'a>(namen: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut vergeben: HashSet<String> = NACHRICHTEN_DATEIEN
        .iter()
        .map(|n| n.to_lowercase())
        .collect();
    namen
        .into_iter()
        .map(|n| {
            let letzter = n.rsplit(['/', '\\']).next().unwrap_or(n);
            let basis = crate::dateiname::portabel(letzter, MAX_DATEINAME)
                .unwrap_or_else(|| "anlage".into());
            let mut kandidat = basis.clone();
            let mut i = 2;
            while !vergeben.insert(kandidat.to_lowercase()) {
                let zusatz = format!("-{i}");
                kandidat = crate::dateiname::mit_zusatz(&basis, &zusatz, MAX_DATEINAME)
                    .unwrap_or_else(|| format!("anlage{zusatz}"));
                i += 1;
            }
            kandidat
        })
        .collect()
}

/// Längster zulässiger Anlagenname im XBau-Paket in Byte.
pub const MAX_DATEINAME: usize = 240;

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
        let lang = format!("{}.pdf", "Ö".repeat(200));
        let namen = eindeutige_namen(["Lageplan M 1:500.pdf", "Ansicht. ", &lang, &lang]);
        assert_eq!(namen[0], "Lageplan M 1_500.pdf");
        assert_eq!(namen[1], "Ansicht");
        assert!(namen[2].ends_with(".pdf") && namen[3].ends_with("-2.pdf"));
        for n in &namen {
            assert!(n.len() <= MAX_DATEINAME);
            super::super::austausch::dateiname(n).unwrap();
        }
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
