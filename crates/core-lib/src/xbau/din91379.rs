//! Zeichenvorrat nach DIN 91379 (Datentyp C, „normative Schriftzeichen“), wie
//! ihn die XBau-Schemas für Namen, Anschriften und die meisten Texte verlangen.
//!
//! Die Tabellen sind aus dem Muster `din91379:datatypeC` der mitgelieferten
//! Schemadatei `xsd_dev/din-norm-91379-datatypes.xsd` (Fassung 2022-08)
//! übernommen: einzelne Zeichen und Folgen aus Grundbuchstabe und
//! kombinierenden Zeichen. Griechische und kyrillische Buchstaben,
//! Steuerzeichen und die meisten Symbole gehören nicht dazu.

/// Zeichen, die in `text` nicht zum Datentyp C gehören, in Reihenfolge und
/// ohne Wiederholung. Leer = zulässig.
pub fn unzulaessige_zeichen(text: &str) -> Vec<char> {
    let mut falsch = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(f) = FOLGEN.iter().find(|f| rest.starts_with(**f)) {
            rest = &rest[f.len()..];
            continue;
        }
        if !EINZELN.iter().any(|(a, b)| (*a..=*b).contains(&c)) && !falsch.contains(&c) {
            falsch.push(c);
        }
        rest = &rest[c.len_utf8()..];
    }
    falsch
}

/// Verständliche Meldung für ein Feld mit unzulässigen Zeichen, `None` wenn
/// der Text zulässig ist.
pub fn meldung(feld: &str, text: &str) -> Option<String> {
    let falsch = unzulaessige_zeichen(text);
    if falsch.is_empty() {
        return None;
    }
    let zeichen = falsch
        .iter()
        .map(|c| {
            if c.is_control() || c.is_whitespace() {
                format!("U+{:04X}", u32::from(*c))
            } else {
                format!("„{c}“ (U+{:04X})", u32::from(*c))
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "{feld}: enthält Zeichen, die Behördennachrichten nach DIN 91379 nicht zulassen ({zeichen}); bitte ersetzen, etwa durch lateinische Umschrift"
    ))
}

const EINZELN: &[(char, char)] = &[
    ('\u{0009}', '\u{000A}'),
    ('\u{000D}', '\u{000D}'),
    ('\u{0020}', '\u{007E}'),
    ('\u{00A0}', '\u{00AC}'),
    ('\u{00AE}', '\u{017E}'),
    ('\u{0187}', '\u{0188}'),
    ('\u{018F}', '\u{018F}'),
    ('\u{0197}', '\u{0197}'),
    ('\u{01A0}', '\u{01A1}'),
    ('\u{01AF}', '\u{01B0}'),
    ('\u{01B7}', '\u{01B7}'),
    ('\u{01CD}', '\u{01DC}'),
    ('\u{01DE}', '\u{01DF}'),
    ('\u{01E2}', '\u{01F0}'),
    ('\u{01F4}', '\u{01F5}'),
    ('\u{01F8}', '\u{01FF}'),
    ('\u{0212}', '\u{0213}'),
    ('\u{0218}', '\u{021B}'),
    ('\u{021E}', '\u{021F}'),
    ('\u{0227}', '\u{0233}'),
    ('\u{0259}', '\u{0259}'),
    ('\u{0268}', '\u{0268}'),
    ('\u{0292}', '\u{0292}'),
    ('\u{02B9}', '\u{02BA}'),
    ('\u{02BE}', '\u{02BF}'),
    ('\u{02C8}', '\u{02C8}'),
    ('\u{02CC}', '\u{02CC}'),
    ('\u{1E02}', '\u{1E03}'),
    ('\u{1E06}', '\u{1E07}'),
    ('\u{1E0A}', '\u{1E11}'),
    ('\u{1E17}', '\u{1E17}'),
    ('\u{1E1C}', '\u{1E2B}'),
    ('\u{1E2F}', '\u{1E37}'),
    ('\u{1E3A}', '\u{1E3B}'),
    ('\u{1E40}', '\u{1E49}'),
    ('\u{1E52}', '\u{1E5B}'),
    ('\u{1E5E}', '\u{1E63}'),
    ('\u{1E6A}', '\u{1E6F}'),
    ('\u{1E80}', '\u{1E87}'),
    ('\u{1E8C}', '\u{1E97}'),
    ('\u{1E9E}', '\u{1E9E}'),
    ('\u{1EA0}', '\u{1EF9}'),
    ('\u{2019}', '\u{2019}'),
    ('\u{2021}', '\u{2021}'),
    ('\u{20AC}', '\u{20AC}'),
];
const FOLGEN: &[&str] = &[
    "\u{0043}\u{0328}\u{0306}",
    "\u{004B}\u{035F}\u{0048}",
    "\u{004B}\u{035F}\u{0068}",
    "\u{004C}\u{0325}\u{0304}",
    "\u{0052}\u{0325}\u{0304}",
    "\u{0053}\u{031B}\u{0304}",
    "\u{0063}\u{0328}\u{0306}",
    "\u{006B}\u{035F}\u{0068}",
    "\u{006C}\u{0325}\u{0304}",
    "\u{0072}\u{0325}\u{0304}",
    "\u{0073}\u{031B}\u{0304}",
    "\u{0041}\u{030B}",
    "\u{0043}\u{0300}",
    "\u{0043}\u{0304}",
    "\u{0043}\u{0306}",
    "\u{0043}\u{0308}",
    "\u{0043}\u{0315}",
    "\u{0043}\u{0323}",
    "\u{0043}\u{0326}",
    "\u{0044}\u{0302}",
    "\u{0046}\u{0300}",
    "\u{0046}\u{0304}",
    "\u{0047}\u{0300}",
    "\u{0048}\u{0304}",
    "\u{0048}\u{0326}",
    "\u{0048}\u{0331}",
    "\u{004A}\u{0301}",
    "\u{004A}\u{030C}",
    "\u{004B}\u{0300}",
    "\u{004B}\u{0302}",
    "\u{004B}\u{0304}",
    "\u{004B}\u{0307}",
    "\u{004B}\u{0315}",
    "\u{004B}\u{031B}",
    "\u{004B}\u{0326}",
    "\u{004C}\u{0302}",
    "\u{004C}\u{0325}",
    "\u{004C}\u{0326}",
    "\u{004D}\u{0300}",
    "\u{004D}\u{0302}",
    "\u{004D}\u{0306}",
    "\u{004D}\u{0310}",
    "\u{004E}\u{0302}",
    "\u{004E}\u{0304}",
    "\u{004E}\u{0306}",
    "\u{004E}\u{0326}",
    "\u{0050}\u{0300}",
    "\u{0050}\u{0304}",
    "\u{0050}\u{0315}",
    "\u{0050}\u{0323}",
    "\u{0052}\u{0306}",
    "\u{0052}\u{0325}",
    "\u{0053}\u{0300}",
    "\u{0053}\u{0304}",
    "\u{0053}\u{0331}",
    "\u{0054}\u{0300}",
    "\u{0054}\u{0304}",
    "\u{0054}\u{0308}",
    "\u{0054}\u{0315}",
    "\u{0054}\u{031B}",
    "\u{0055}\u{0307}",
    "\u{005A}\u{0300}",
    "\u{005A}\u{0304}",
    "\u{005A}\u{0306}",
    "\u{005A}\u{0308}",
    "\u{005A}\u{0327}",
    "\u{0061}\u{030B}",
    "\u{0063}\u{0300}",
    "\u{0063}\u{0304}",
    "\u{0063}\u{0306}",
    "\u{0063}\u{0308}",
    "\u{0063}\u{0315}",
    "\u{0063}\u{0323}",
    "\u{0063}\u{0326}",
    "\u{0064}\u{0302}",
    "\u{0066}\u{0300}",
    "\u{0066}\u{0304}",
    "\u{0067}\u{0300}",
    "\u{0068}\u{0304}",
    "\u{0068}\u{0326}",
    "\u{006A}\u{0301}",
    "\u{006B}\u{0300}",
    "\u{006B}\u{0302}",
    "\u{006B}\u{0304}",
    "\u{006B}\u{0307}",
    "\u{006B}\u{0315}",
    "\u{006B}\u{031B}",
    "\u{006B}\u{0326}",
    "\u{006C}\u{0302}",
    "\u{006C}\u{0325}",
    "\u{006C}\u{0326}",
    "\u{006D}\u{0300}",
    "\u{006D}\u{0302}",
    "\u{006D}\u{0306}",
    "\u{006D}\u{0310}",
    "\u{006E}\u{0302}",
    "\u{006E}\u{0304}",
    "\u{006E}\u{0306}",
    "\u{006E}\u{0326}",
    "\u{0070}\u{0300}",
    "\u{0070}\u{0304}",
    "\u{0070}\u{0315}",
    "\u{0070}\u{0323}",
    "\u{0072}\u{0306}",
    "\u{0072}\u{0325}",
    "\u{0073}\u{0300}",
    "\u{0073}\u{0304}",
    "\u{0073}\u{0331}",
    "\u{0074}\u{0300}",
    "\u{0074}\u{0304}",
    "\u{0074}\u{0315}",
    "\u{0074}\u{031B}",
    "\u{0075}\u{0307}",
    "\u{007A}\u{0300}",
    "\u{007A}\u{0304}",
    "\u{007A}\u{0306}",
    "\u{007A}\u{0308}",
    "\u{007A}\u{0327}",
    "\u{00C7}\u{0306}",
    "\u{00DB}\u{0304}",
    "\u{00E7}\u{0306}",
    "\u{00FB}\u{0304}",
    "\u{00FF}\u{0301}",
    "\u{010C}\u{0315}",
    "\u{010C}\u{0323}",
    "\u{010D}\u{0315}",
    "\u{010D}\u{0323}",
    "\u{0113}\u{030D}",
    "\u{012A}\u{0301}",
    "\u{012B}\u{0301}",
    "\u{014D}\u{030D}",
    "\u{017D}\u{0326}",
    "\u{017D}\u{0327}",
    "\u{017E}\u{0326}",
    "\u{017E}\u{0327}",
    "\u{1E32}\u{0304}",
    "\u{1E33}\u{0304}",
    "\u{1E62}\u{0304}",
    "\u{1E63}\u{0304}",
    "\u{1E6C}\u{0304}",
    "\u{1E6D}\u{0304}",
    "\u{1EA0}\u{0308}",
    "\u{1EA1}\u{0308}",
    "\u{1ECC}\u{0308}",
    "\u{1ECD}\u{0308}",
    "\u{1EE4}\u{0304}",
    "\u{1EE4}\u{0308}",
    "\u{1EE5}\u{0304}",
    "\u{1EE5}\u{0308}",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lateinische_namen_sind_zulaessig() {
        for t in [
            "Müller-Øresund",
            "Wohnbau Süd GmbH & Co. KG",
            "Łódź, Straße 12a",
            "Ðorđević",
            "Jürgen ẞ 1 € §",
            // Grundbuchstabe mit kombinierenden Zeichen (L̥̄).
            "L\u{0325}\u{0304}",
            "Zeile 1\nZeile 2\tEnde",
        ] {
            assert!(unzulaessige_zeichen(t).is_empty(), "{t}");
        }
    }

    #[test]
    fn fremde_schriften_und_steuerzeichen_werden_genannt() {
        assert_eq!(unzulaessige_zeichen("Αθήνα"), vec!['Α', 'θ', 'ή', 'ν', 'α']);
        assert_eq!(
            unzulaessige_zeichen("Иван Иванов"),
            vec!['И', 'в', 'а', 'н', 'о']
        );
        assert_eq!(unzulaessige_zeichen("a\u{0007}b"), vec!['\u{0007}']);
        // Kombinierendes Zeichen ohne passenden Grundbuchstaben.
        assert_eq!(unzulaessige_zeichen("x\u{0301}"), vec!['\u{0301}']);
        assert_eq!(unzulaessige_zeichen("Haus 🏠"), vec!['🏠']);
        let m = meldung("Bauherr: Familienname", "Иванов").unwrap();
        assert!(
            m.contains("Bauherr: Familienname") && m.contains("„И“ (U+0418)"),
            "{m}"
        );
        assert!(meldung("x", "München").is_none());
        assert!(meldung("x", "a\u{0007}").unwrap().contains("U+0007"));
    }
}
