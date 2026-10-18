//! Minimaler XML-Schreiber für XBau-Nachrichten: Elemente, Attribute und
//! Text, eingerückt mit zwei Leerzeichen. Mehr braucht der Export nicht.
//! Zeichen, die XML 1.0 nicht erlaubt (etwa Steuerzeichen aus kopiertem
//! Text), werden nicht still entfernt: [`XmlSchreiber::fertig`] meldet sie.

use crate::error::{CoreError, CoreResult};

pub(crate) struct XmlSchreiber {
    aus: String,
    offen: Vec<&'static str>,
    /// Element und Zeichen, die XML nicht darstellen kann.
    unzulaessig: Vec<(String, char)>,
}

impl XmlSchreiber {
    pub fn new() -> Self {
        Self {
            aus: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"),
            offen: Vec::new(),
            unzulaessig: Vec::new(),
        }
    }

    fn einruecken(&mut self) {
        for _ in 0..self.offen.len() {
            self.aus.push_str("  ");
        }
    }

    fn start(&mut self, name: &str, attribute: &[(&str, &str)]) {
        self.einruecken();
        self.aus.push('<');
        self.aus.push_str(name);
        for (n, w) in attribute {
            self.aus.push(' ');
            self.aus.push_str(n);
            self.aus.push_str("=\"");
            self.maskieren(name, w);
            self.aus.push('"');
        }
        self.aus.push('>');
    }

    /// Öffnet ein Element mit Kindelementen.
    pub fn auf(&mut self, name: &'static str, attribute: &[(&str, &str)]) -> &mut Self {
        self.start(name, attribute);
        self.aus.push('\n');
        self.offen.push(name);
        self
    }

    pub fn zu(&mut self) -> &mut Self {
        let name = self.offen.pop().expect("kein offenes Element");
        self.einruecken();
        self.aus.push_str("</");
        self.aus.push_str(name);
        self.aus.push_str(">\n");
        self
    }

    /// Element mit Textinhalt.
    pub fn text(&mut self, name: &str, text: &str) -> &mut Self {
        self.text_mit(name, &[], text)
    }

    pub fn text_mit(&mut self, name: &str, attribute: &[(&str, &str)], text: &str) -> &mut Self {
        self.start(name, attribute);
        self.maskieren(name, text);
        self.aus.push_str("</");
        self.aus.push_str(name);
        self.aus.push_str(">\n");
        self
    }

    /// Element mit Text, nur wenn der Text nicht leer ist.
    pub fn text_optional(&mut self, name: &str, text: &str) -> &mut Self {
        let t = text.trim();
        if !t.is_empty() {
            self.text(name, t);
        }
        self
    }

    /// Fertiges Dokument; Fehler, wenn ein Text Zeichen enthielt, die XML
    /// nicht zulässt.
    pub fn fertig(self) -> CoreResult<String> {
        assert!(self.offen.is_empty(), "offene Elemente: {:?}", self.offen);
        if self.unzulaessig.is_empty() {
            return Ok(self.aus);
        }
        let stellen: Vec<String> = self
            .unzulaessig
            .iter()
            .map(|(e, c)| format!("{e}: U+{:04X}", u32::from(*c)))
            .collect();
        Err(CoreError::InvalidInput(format!(
            "Die XBau-Nachricht enthält Steuerzeichen, die XML nicht zulässt ({}). Bitte die betroffenen Texte bereinigen.",
            stellen.join(", ")
        )))
    }

    fn maskieren(&mut self, element: &str, text: &str) {
        for c in text.chars() {
            match c {
                '&' => self.aus.push_str("&amp;"),
                '<' => self.aus.push_str("&lt;"),
                '>' => self.aus.push_str("&gt;"),
                '"' => self.aus.push_str("&quot;"),
                '\'' => self.aus.push_str("&apos;"),
                c if xml_zeichen(c) => self.aus.push(c),
                c => {
                    if !self
                        .unzulaessig
                        .iter()
                        .any(|(e, z)| e == element && *z == c)
                    {
                        self.unzulaessig.push((element.to_string(), c));
                    }
                }
            }
        }
    }
}

/// Zeichen nach XML 1.0, Abschnitt 2.2 (`Char`).
fn xml_zeichen(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schreibt_verschachtelt_und_maskiert() {
        let mut x = XmlSchreiber::new();
        x.auf("a", &[("v", "1 \"2\"")])
            .text("b", "Müller & Söhne <GmbH>")
            .text_optional("c", "  ")
            .zu();
        assert_eq!(
            x.fertig().unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<a v=\"1 &quot;2&quot;\">\n  <b>Müller &amp; Söhne &lt;GmbH&gt;</b>\n</a>\n"
        );
    }

    #[test]
    fn steuerzeichen_werden_gemeldet_statt_geschrieben() {
        let mut x = XmlSchreiber::new();
        x.auf("a", &[]).text("textabsatz", "Zeile\u{0007}").zu();
        let e = x.fertig().unwrap_err().to_string();
        assert!(e.contains("textabsatz: U+0007"), "{e}");
    }
}
