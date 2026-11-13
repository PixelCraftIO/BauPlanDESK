//! Portable Dateinamen: eine Regel für alle Namen, die BauPlanDESK neu vergibt
//! (DMS-Ablage, Druckausgabe, E-Mail-Anhänge, XBau-Pakete, Abgabe- und
//! Einreichungs-ZIPs, Projektordner, Aktenexport).
//!
//! Ein portabler Name ist unter Windows, macOS und Linux sowie in ZIP-Archiven
//! verwendbar: keine Steuerzeichen, keines von `< > : " / \ | ? *`, kein Punkt
//! oder Leerzeichen am Anfang oder Ende, kein reservierter Gerätename (`CON`,
//! `NUL`, `COM1` …) und eine begrenzte Länge in Byte. Beim Kürzen bleibt die
//! Dateiendung erhalten, gekürzt wird nur der Stamm.

/// Zeichen, die unter Windows in Dateinamen unzulässig sind.
pub const UNZULAESSIGE_ZEICHEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Längste Endung (ohne Punkt), die beim Kürzen als Endung erhalten bleibt.
const MAX_ENDUNG: usize = 16;

/// Standardlänge für Namen im DMS. Der Storage-Schlüssel stellt eine UUID
/// (37 Byte) voran und die Metadatei hängt `.json` an; zusammen bleiben sie
/// unter der üblichen Grenze von 255 Byte je Pfadsegment.
pub const MAX_DMS_BYTES: usize = 200;

/// `true` für Windows-Gerätenamen wie `CON`, `nul.txt` oder `COM1.pdf`.
pub fn reservierter_name(name: &str) -> bool {
    let basis = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    matches!(basis.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|p| {
            basis
                .strip_prefix(p)
                .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        })
}

/// Erfüllt `name` bereits alle Regeln für höchstens `max_bytes` Byte?
pub fn ist_portabel(name: &str, max_bytes: usize) -> bool {
    portabel(name, max_bytes).as_deref() == Some(name)
}

fn kuerzen(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut ende = max_bytes;
    while !text.is_char_boundary(ende) {
        ende -= 1;
    }
    &text[..ende]
}

fn zerlegen(name: &str) -> (&str, &str) {
    match name.rsplit_once('.') {
        Some((stamm, endung))
            if !stamm.trim_matches([' ', '.']).is_empty()
                && !endung.is_empty()
                && endung.len() <= MAX_ENDUNG
                && !endung.contains(' ') =>
        {
            (stamm, &name[stamm.len()..])
        }
        _ => (name, ""),
    }
}

/// Macht aus `name` einen portablen Dateinamen von höchstens `max_bytes`
/// Byte. Unzulässige Zeichen werden durch `_` ersetzt, Punkte und Leerzeichen
/// an den Rändern entfernt, reservierte Gerätenamen mit `_` versehen und zu
/// lange Namen am Stamm gekürzt. `None`, wenn kein Name übrig bleibt.
pub fn portabel(name: &str, max_bytes: usize) -> Option<String> {
    mit_zusatz(name, "", max_bytes)
}

/// Wie [`portabel`], hängt aber `zusatz` (z. B. `-2`) vor der Endung an und
/// kürzt dafür den Stamm, sodass der Gesamtname `max_bytes` nicht übersteigt.
pub fn mit_zusatz(name: &str, zusatz: &str, max_bytes: usize) -> Option<String> {
    let ersetzt: String = name
        .chars()
        .map(|c| {
            if c.is_control() || UNZULAESSIGE_ZEICHEN.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let bereinigt = ersetzt.trim_matches([' ', '.']);
    if bereinigt.is_empty() {
        return None;
    }
    let (stamm, endung) = zerlegen(bereinigt);
    let mut stamm = stamm.trim_end_matches([' ', '.']).to_owned();
    if reservierter_name(&format!("{stamm}{endung}")) {
        stamm.insert(0, '_');
    }
    let rest = max_bytes.checked_sub(endung.len() + zusatz.len())?;
    let stamm = kuerzen(&stamm, rest).trim_end_matches([' ', '.']);
    if stamm.is_empty() {
        // Endung allein passt nicht sinnvoll: ganzen Namen kürzen.
        let ganz = format!("{bereinigt}{zusatz}");
        let ganz = kuerzen(&ganz, max_bytes).trim_matches([' ', '.']);
        return (!ganz.is_empty()).then(|| ganz.to_owned());
    }
    Some(format!("{stamm}{zusatz}{endung}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unzulaessige_zeichen_raender_und_geraetenamen() {
        assert_eq!(
            portabel("Lageplan M 1:500.pdf", 240).unwrap(),
            "Lageplan M 1_500.pdf"
        );
        assert_eq!(
            portabel("Was? <Neu> \"A\"|*.pdf", 240).unwrap(),
            "Was_ _Neu_ _A___.pdf"
        );
        assert_eq!(portabel("  .Notiz. . ", 240).unwrap(), "Notiz");
        assert_eq!(portabel("Plan.pdf.", 240).unwrap(), "Plan.pdf");
        assert_eq!(portabel("CON.pdf", 240).unwrap(), "_CON.pdf");
        assert_eq!(portabel("com1", 240).unwrap(), "_com1");
        assert_eq!(portabel("Console.pdf", 240).unwrap(), "Console.pdf");
        assert_eq!(
            portabel("Zeile\neins\t.txt", 240).unwrap(),
            "Zeile_eins_.txt"
        );
        assert_eq!(portabel(" . ", 240), None);
        assert_eq!(portabel("", 240), None);
        assert!(ist_portabel("Bauantrag.pdf", 240));
        assert!(!ist_portabel("Lageplan 1:500.pdf", 240));
    }

    #[test]
    fn kuerzt_nur_den_stamm_und_erhaelt_die_endung() {
        let lang = format!("{}.pdf", "Ä".repeat(200));
        let p = portabel(&lang, 240).unwrap();
        assert!(p.len() <= 240);
        assert!(p.ends_with("Ä.pdf"));
        let p = mit_zusatz(&lang, "-12", 240).unwrap();
        assert!(p.len() <= 240 && p.ends_with("Ä-12.pdf"), "{p}");
        // Leerzeichen am gekürzten Ende werden entfernt.
        let p = portabel(&format!("{} x.pdf", "a".repeat(10)), 14).unwrap();
        assert_eq!(p, "aaaaaaaaaa.pdf");
        // Ohne erkennbare Endung wird der ganze Name gekürzt.
        assert_eq!(portabel(&"b".repeat(300), 100).unwrap().len(), 100);
        assert_eq!(portabel("x.verylongextensionname", 5).unwrap(), "x.ver");
    }
}
