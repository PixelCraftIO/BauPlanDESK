//! Abgleich der im Bauantrag gewählten Landes-Verfahrensart mit der
//! Verfahrensakte. Die Akte ist für Art und Prüfverfahren maßgebend; der
//! Bauantrag benennt den Code der Landes-Codeliste für XBau. Widersprüche werden
//! als Prüfhinweis gemeldet, nicht still korrigiert: Die Codelisten der Länder
//! sind nur über ihre Beschreibung vergleichbar.
use super::{Pruefverfahren, Verfahrensart};

/// `landesart`: Beschreibung des gewählten Codes laut Landes-Codeliste;
/// `None`, wenn im Bauantrag keine Verfahrensart gewählt ist.
pub fn verfahrensart_hinweise(
    art: &Verfahrensart,
    pruefverfahren: Option<&Pruefverfahren>,
    landesart: Option<&str>,
) -> Vec<String> {
    let akte = match pruefverfahren {
        Some(Pruefverfahren::Vereinfacht) => format!("{} (vereinfacht)", art.bezeichnung()),
        Some(Pruefverfahren::Regulaer) => format!("{} (regulär)", art.bezeichnung()),
        None => art.bezeichnung().to_string(),
    };
    let Some(b) = landesart else {
        return if matches!(
            art,
            Verfahrensart::Bauantrag
                | Verfahrensart::Aenderungsantrag
                | Verfahrensart::Freistellung
                | Verfahrensart::Nutzungsaenderung
        ) {
            vec![format!(
                "Verfahrensart im Bauantrag nicht gewählt; laut Verfahrensakte: {akte}. Landes-Verfahrensart im Bauantrag ergänzen."
            )]
        } else {
            vec![]
        };
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
                Some("Vereinfachtes Baugenehmigungsverfahren")
            )
            .is_empty()
        );
        let h = verfahrensart_hinweise(
            &Verfahrensart::Bauantrag,
            v,
            Some("Baugenehmigungsverfahren"),
        );
        assert!(h[0].contains("vereinfacht"));
        let h = verfahrensart_hinweise(
            &Verfahrensart::Bauantrag,
            None,
            Some("Genehmigungsfreistellungsverfahren"),
        );
        assert!(h[0].contains("freistellung"));
        assert!(
            verfahrensart_hinweise(&Verfahrensart::Bauantrag, None, None)[0]
                .contains("nicht gewählt")
        );
        assert!(verfahrensart_hinweise(&Verfahrensart::Widerspruch, None, None).is_empty());
    }
}
