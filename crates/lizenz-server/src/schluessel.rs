//! Lizenzschlüssel, die Kunden abtippen: `BPIO-XXXXX-XXXXX-XXXXX-XXXXX-XXXXX`.

/// 32 Zeichen ohne die leicht verwechselbaren 0/O, 1/I.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Neuer zufälliger Schlüssel mit 125 Bit Zufall (25 Zeichen à 5 Bit).
pub fn neuer_lizenzschluessel() -> String {
    let mut zufall = [0u8; 25];
    getrandom::fill(&mut zufall).expect("Zufallszahlen des Betriebssystems");
    let zeichen: Vec<char> = zufall
        .iter()
        // 256 ist ein Vielfaches von 32: keine Verzerrung.
        .map(|b| ALPHABET[usize::from(*b) % 32] as char)
        .collect();
    let gruppen: Vec<String> = zeichen.chunks(5).map(|c| c.iter().collect()).collect();
    format!("BPIO-{}", gruppen.join("-"))
}

/// Wie die App: Leerzeichen weg, Großbuchstaben.
pub fn normalisieren(eingabe: &str) -> String {
    eingabe
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        let s = neuer_lizenzschluessel();
        assert_eq!(s.len(), 4 + 5 * 6);
        assert!(s.starts_with("BPIO-"));
        assert!(
            s[5..]
                .split('-')
                .all(|g| g.len() == 5 && g.bytes().all(|b| ALPHABET.contains(&b)))
        );
        assert_ne!(s, neuer_lizenzschluessel());
        assert_eq!(normalisieren(&s.to_lowercase()), s);
    }
}
