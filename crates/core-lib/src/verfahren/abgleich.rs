//! Abgleich der im Bauantrag gewählten Landes-Verfahrensart mit der
//! Verfahrensakte. Die Akte ist für Art und Prüfverfahren maßgebend; der
//! Bauantrag benennt den Code der Landes-Codeliste für XBau. Widersprüche werden
//! als Prüfhinweis gemeldet, nicht still korrigiert: Die Codelisten der Länder
//! sind nur über ihre Beschreibung vergleichbar.
use super::{Pruefverfahren, Verfahrensart};

/// Landes-Verfahrensart des Bauantrags, wie sie der Abgleich sieht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landesverfahrensart<'a> {
    /// Gewählter Code; Beschreibung laut Landes-Codeliste.
    Gewaehlt(&'a str),
    /// Im Bauantrag nicht gewählt, obwohl die Landes-Codeliste geladen ist.
    NichtGewaehlt,
    /// Für das Land ist keine Codeliste der Verfahrensarten geladen (URI);
    /// eine Wahl ist im Bauantrag dann nicht möglich.
    KeineListe(&'a str),
}

/// Hinweise zum Abgleich von Bauantrag und Verfahrensakte. Ohne geladene
/// Landes-Codeliste gibt es nur einen informativen Hinweis ohne
/// Handlungsaufforderung: Die Verfahrensart lässt sich dann nicht wählen und
/// wird nicht in XBau übermittelt; maßgebend bleibt die Verfahrensakte.
pub fn verfahrensart_hinweise(
    art: &Verfahrensart,
    pruefverfahren: Option<&Pruefverfahren>,
    landesart: Landesverfahrensart<'_>,
) -> Vec<String> {
    let akte = match pruefverfahren {
        Some(Pruefverfahren::Vereinfacht) => format!("{} (vereinfacht)", art.bezeichnung()),
        Some(Pruefverfahren::Regulaer) => format!("{} (regulär)", art.bezeichnung()),
        None => art.bezeichnung().to_string(),
    };
    let antrag = matches!(
        art,
        Verfahrensart::Bauantrag
            | Verfahrensart::Aenderungsantrag
            | Verfahrensart::Freistellung
            | Verfahrensart::Nutzungsaenderung
    );
    let b = match landesart {
        Landesverfahrensart::Gewaehlt(b) => b,
        Landesverfahrensart::NichtGewaehlt if antrag => {
            return vec![format!(
                "Verfahrensart im Bauantrag nicht gewählt; laut Verfahrensakte: {akte}. Landes-Verfahrensart im Bauantrag ergänzen."
            )];
        }
        Landesverfahrensart::KeineListe(uri) if antrag => {
            return vec![format!(
                "Zur Information: Die XBau-Nachricht enthält keine Landes-Verfahrensart, weil keine Codeliste der Verfahrensarten ({uri}) geladen ist. Maßgebend ist die Verfahrensakte: {akte}."
            )];
        }
        _ => return vec![],
    };
    let t = b.to_lowercase();
    let mut widerspruch = Vec::new();
    let vereinfacht = t.contains("vereinfacht");
    match pruefverfahren {
        Some(Pruefverfahren::Vereinfacht) if !vereinfacht => {
            widerspruch.push("Prüfverfahren vereinfacht, Landes-Verfahrensart nicht vereinfacht")
        }
        Some(Pruefverfahren::Regulaer) if vereinfacht => {
            widerspruch.push("Prüfverfahren regulär, Landes-Verfahrensart vereinfacht")
        }
        _ => {}
    }
    let freistellung = t.contains("freistell") || t.contains("kenntnisgabe");
    if (*art == Verfahrensart::Freistellung) != freistellung {
        widerspruch.push("Genehmigungsfreistellung nur auf einer Seite angegeben");
    }
    if (*art == Verfahrensart::Vorbescheid) != t.contains("vorbescheid") {
        widerspruch.push("Vorbescheid nur auf einer Seite angegeben");
    }
    if widerspruch.is_empty() {
        return vec![];
    }
    vec![format!(
        "Verfahrensart im Bauantrag („{b}“) passt nicht zur Verfahrensakte ({akte}): {}. Maßgebend ist die Verfahrensakte; Bauantrag prüfen.",
        widerspruch.join("; ")
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widersprueche_werden_benannt() {
        let v = Some(&Pruefverfahren::Vereinfacht);
        assert!(
            verfahrensart_hinweise(
                &Verfahrensart::Bauantrag,
                v,
                Landesverfahrensart::Gewaehlt("Vereinfachtes Baugenehmigungsverfahren")
            )
            .is_empty()
        );
        let h = verfahrensart_hinweise(
            &Verfahrensart::Bauantrag,
            v,
            Landesverfahrensart::Gewaehlt("Baugenehmigungsverfahren"),
        );
        assert!(h[0].contains("vereinfacht"));
        let h = verfahrensart_hinweise(
            &Verfahrensart::Bauantrag,
            None,
            Landesverfahrensart::Gewaehlt("Genehmigungsfreistellungsverfahren"),
        );
        assert!(h[0].contains("freistellung"));
        assert!(
            verfahrensart_hinweise(
                &Verfahrensart::Bauantrag,
                None,
                Landesverfahrensart::NichtGewaehlt
            )[0]
            .contains("nicht gewählt")
        );
        assert!(
            verfahrensart_hinweise(
                &Verfahrensart::Widerspruch,
                None,
                Landesverfahrensart::NichtGewaehlt
            )
            .is_empty()
        );
    }

    #[test]
    fn ohne_landesliste_nur_information() {
        let uri = "urn:xoev-de:xbau:nw:codeliste:verfahrensart";
        let h = verfahrensart_hinweise(
            &Verfahrensart::Bauantrag,
            Some(&Pruefverfahren::Vereinfacht),
            Landesverfahrensart::KeineListe(uri),
        );
        assert_eq!(h.len(), 1);
        assert!(
            h[0].contains(uri) && h[0].contains("(vereinfacht)"),
            "{h:?}"
        );
        // Keine unerfüllbare Handlungsaufforderung.
        assert!(!h[0].contains("ergänzen"), "{h:?}");
        assert!(
            verfahrensart_hinweise(
                &Verfahrensart::Widerspruch,
                None,
                Landesverfahrensart::KeineListe(uri)
            )
            .is_empty()
        );
    }
}
