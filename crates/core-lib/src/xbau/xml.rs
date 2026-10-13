//! Minimaler XML-Schreiber für XBau-Nachrichten: Elemente, Attribute und
//! Text, eingerückt mit zwei Leerzeichen. Mehr braucht der Export nicht.

pub(crate) struct XmlSchreiber {
    aus: String,
    offen: Vec<&'static str>,
}

impl XmlSchreiber {
    pub fn new() -> Self {
        Self {
            aus: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"),
            offen: Vec::new(),
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
            maskieren(&mut self.aus, w);
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
        maskieren(&mut self.aus, text);
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

    pub fn fertig(self) -> String {
        assert!(self.offen.is_empty(), "offene Elemente: {:?}", self.offen);
        self.aus
    }
}

fn maskieren(aus: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => aus.push_str("&amp;"),
            '<' => aus.push_str("&lt;"),
            '>' => aus.push_str("&gt;"),
            '"' => aus.push_str("&quot;"),
            '\'' => aus.push_str("&apos;"),
            _ => aus.push(c),
        }
    }
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
            x.fertig(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<a v=\"1 &quot;2&quot;\">\n  <b>Müller &amp; Söhne &lt;GmbH&gt;</b>\n</a>\n"
        );
    }
}
