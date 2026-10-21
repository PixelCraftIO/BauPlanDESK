//! Zeitleiste der Verfahrensakte. Jede Revision erhält ein Aktenereignis; der
//! Core leitet daraus die Art der Änderung und die Aktion im Projektprotokoll
//! ab. Erfasste Werte wie Zustell- oder Vollständigkeitsdatum werden nicht
//! still überschrieben, sondern nur mit einer begründeten Berichtigung, deren
//! alten und neuen Wert der Core in das Ereignis schreibt.
use super::{Verfahrensakte, fehler};
use crate::{CoreResult, domain::Aktion, plattform::text};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

/// Art einer Verfahrensänderung in der Zeitleiste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Ereignisart {
    Angelegt,
    Status,
    Vollstaendigkeit,
    Ruecknahme,
    Kommunikation,
    Nachforderung,
    Schritt,
    Abgabe,
    Versand,
    Bescheid,
    Zustellung,
    Nebenbestimmung,
    Ausfuehrung,
    Berichtigung,
    /// Manuell erfasstes Ereignis ohne weitere Änderung.
    Notiz,
    /// Übrige Verfahrensangaben (Stammdaten, Anforderungen, Profil …).
    Angaben,
    /// Abweichungen, Ausnahmen und Befreiungen.
    Abweichung,
    /// Gebührenbescheide und Zahlungen.
    Gebuehr,
}
impl Ereignisart {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Angelegt => "Verfahren angelegt",
            Self::Status => "Bearbeitungsstand",
            Self::Vollstaendigkeit => "Vollständigkeit",
            Self::Ruecknahme => "Rücknahme",
            Self::Kommunikation => "Kommunikation",
            Self::Nachforderung => "Nachforderung",
            Self::Schritt => "Behördliche Schritte",
            Self::Abgabe => "Abgabestand",
            Self::Versand => "Übermittlung",
            Self::Bescheid => "Bescheid",
            Self::Zustellung => "Zustellung",
            Self::Nebenbestimmung => "Nebenbestimmung",
            Self::Ausfuehrung => "Bauausführung",
            Self::Berichtigung => "Berichtigung",
            Self::Notiz => "Notiz",
            Self::Angaben => "Verfahrensangaben",
            Self::Abweichung => "Abweichungen",
            Self::Gebuehr => "Gebühren",
        }
    }
}

/// Erfasster Wert, der nur mit begründeter Berichtigung geändert wird.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Berichtigungsgegenstand {
    /// Zustelldatum eines Bescheids.
    Zustellung {
        bescheid: Uuid,
    },
    Vollstaendigkeit,
    Ruecknahme,
    Baubeginn,
    Fertigstellung,
}
impl Berichtigungsgegenstand {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Zustellung { .. } => "Zustelldatum",
            Self::Vollstaendigkeit => "Vollständigkeitsbestätigung",
            Self::Ruecknahme => "Rücknahme",
            Self::Baubeginn => "Baubeginn",
            Self::Fertigstellung => "Fertigstellung",
        }
    }
}

/// Begründete Berichtigung, mitgeführt in einem neuen Aktenereignis derselben
/// Speicherung. `vorher` und `nachher` trägt der Core aus dem gespeicherten und
/// dem neuen Stand ein; übermittelte Werte werden überschrieben.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Wertberichtigung {
    pub gegenstand: Berichtigungsgegenstand,
    pub grund: String,
    #[serde(default)]
    pub vorher: String,
    #[serde(default)]
    pub nachher: String,
}

fn tag(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}

/// Gesperrte Werte, die sich gegenüber dem gespeicherten Stand ändern, mit
/// lesbarem alten und neuen Wert.
fn gesperrte_aenderungen(
    alt: &Verfahrensakte,
    a: &Verfahrensakte,
) -> Vec<(Berichtigungsgegenstand, String, String)> {
    use Berichtigungsgegenstand::*;
    let entfernt = || "entfernt".to_string();
    let schreiben = |akte: &Verfahrensakte, datum: NaiveDate, id: Uuid| {
        let betreff = akte
            .kommunikation
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.betreff.as_str())
            .unwrap_or("unbekanntes Schreiben");
        format!("{} · {betreff}", tag(datum))
    };
    let mut r = Vec::new();
    for b in &alt.bescheide {
        if let Some(vorher) = b.zugestellt
            && let Some(neu) = a.bescheide.iter().find(|n| n.id == b.id)
            && let Some(nachher) = neu.zugestellt
            && nachher != vorher
        {
            // Entfernen ist keine Berichtigung; die Bescheidprüfung weist es ab.
            r.push((Zustellung { bescheid: b.id }, tag(vorher), tag(nachher)));
        }
    }
    if let Some(v) = &alt.vollstaendigkeit
        && a.vollstaendigkeit.as_ref() != Some(v)
    {
        r.push((
            Vollstaendigkeit,
            schreiben(alt, v.datum, v.kommunikation),
            a.vollstaendigkeit
                .as_ref()
                .map(|n| schreiben(a, n.datum, n.kommunikation))
                .unwrap_or_else(entfernt),
        ));
    }
    if let Some(v) = &alt.ruecknahme
        && a.ruecknahme.as_ref() != Some(v)
    {
        r.push((
            Ruecknahme,
            schreiben(alt, v.datum, v.kommunikation),
            a.ruecknahme
                .as_ref()
                .map(|n| schreiben(a, n.datum, n.kommunikation))
                .unwrap_or_else(entfernt),
        ));
    }
    if let Some(x) = &alt.ausfuehrung {
        let neu = a.ausfuehrung.as_ref();
        if neu.map(|n| n.baubeginn) != Some(x.baubeginn) {
            r.push((
                Baubeginn,
                tag(x.baubeginn),
                neu.map(|n| tag(n.baubeginn)).unwrap_or_else(entfernt),
            ));
        }
        if let Some(f) = x.fertigstellung
            && neu.and_then(|n| n.fertigstellung) != Some(f)
        {
            r.push((
                Fertigstellung,
                tag(f),
                neu.and_then(|n| n.fertigstellung)
                    .map(tag)
                    .unwrap_or_else(entfernt),
            ));
        }
    }
    r
}

fn benennen(g: &Berichtigungsgegenstand, a: &Verfahrensakte) -> String {
    match g {
        Berichtigungsgegenstand::Zustellung { bescheid } => format!(
            "Zustelldatum des Bescheids vom {}",
            a.bescheide
                .iter()
                .find(|b| b.id == *bescheid)
                .map(|b| tag(b.datum))
                .unwrap_or_default()
        ),
        g => g.bezeichnung().into(),
    }
}

/// Trägt in neue Ereignisse mit Berichtigung den alten und neuen Wert und
/// einen lesbaren Text ein. Arten neuer Ereignisse setzt allein der Core.
pub(super) fn berichtigungen_ausfuellen(
    alt: Option<&Verfahrensakte>,
    a: &mut Verfahrensakte,
    bisher: usize,
) {
    let geaendert = alt
        .map(|alt| gesperrte_aenderungen(alt, a))
        .unwrap_or_default();
    let namen: Vec<String> = geaendert.iter().map(|(g, ..)| benennen(g, a)).collect();
    for e in a.ereignisse.iter_mut().skip(bisher) {
        e.arten.clear();
        let Some(b) = &mut e.berichtigung else {
            continue;
        };
        if let Some(i) = geaendert.iter().position(|(g, ..)| *g == b.gegenstand) {
            let (_, vorher, nachher) = &geaendert[i];
            b.vorher = vorher.clone();
            b.nachher = nachher.clone();
            e.text = format!(
                "Berichtigung {}: {vorher} → {nachher} · Grund: {}",
                namen[i],
                b.grund.trim()
            )
            .chars()
            .take(4000)
            .collect();
            e.arten = vec![Ereignisart::Berichtigung];
        }
    }
}

/// Jeder geänderte gesperrte Wert braucht genau eine begründete Berichtigung
/// in derselben Speicherung, und jede Berichtigung einen geänderten Wert.
pub(super) fn berichtigungen_pruefen(
    alt: Option<&Verfahrensakte>,
    a: &Verfahrensakte,
) -> CoreResult<()> {
    let bisher = alt.map(|x| x.ereignisse.len()).unwrap_or(0);
    let neu: Vec<_> = a
        .ereignisse
        .iter()
        .skip(bisher)
        .filter_map(|e| e.berichtigung.as_ref())
        .collect();
    let geaendert = alt
        .map(|alt| gesperrte_aenderungen(alt, a))
        .unwrap_or_default();
    for b in &neu {
        text(&b.grund, 4000)
            .map_err(|_| fehler("Berichtigung benötigt einen Grund mit höchstens 4000 Zeichen"))?;
        if !geaendert.iter().any(|(g, ..)| *g == b.gegenstand) {
            return Err(fehler(format!(
                "Berichtigung {} ohne geänderten erfassten Wert",
                b.gegenstand.bezeichnung()
            )));
        }
        if neu.iter().filter(|n| n.gegenstand == b.gegenstand).count() > 1 {
            return Err(fehler("Je Wert genügt eine Berichtigung"));
        }
    }
    if let Some((g, ..)) = geaendert
        .iter()
        .find(|(g, ..)| !neu.iter().any(|b| b.gegenstand == *g))
    {
        return Err(fehler(format!(
            "{} ist erfasst und wird nur mit begründeter Berichtigung geändert.",
            benennen(g, a)
        )));
    }
    Ok(())
}

/// Arten der Änderung zwischen gespeichertem und neuem Stand.
pub fn arten(alt: Option<&Verfahrensakte>, a: &Verfahrensakte) -> Vec<Ereignisart> {
    use Ereignisart::*;
    let Some(alt) = alt else {
        return vec![Angelegt];
    };
    let mut r = BTreeSet::new();
    let mut wenn = |bedingung: bool, art| {
        if bedingung {
            r.insert(art);
        }
    };
    wenn(alt.status != a.status, Status);
    wenn(alt.vollstaendigkeit != a.vollstaendigkeit, Vollstaendigkeit);
    wenn(alt.ruecknahme != a.ruecknahme, Ruecknahme);
    wenn(alt.ausfuehrung != a.ausfuehrung, Ausfuehrung);
    let neue_kommunikation = a
        .kommunikation
        .iter()
        .any(|c| !alt.kommunikation.iter().any(|v| v.id == c.id));
    wenn(neue_kommunikation, Kommunikation);
    wenn(alt.nachforderungen != a.nachforderungen, Nachforderung);
    wenn(alt.schritte != a.schritte, Schritt);
    wenn(
        a.abgaben
            .iter()
            .any(|b| !alt.abgaben.iter().any(|v| v.id == b.id)),
        Abgabe,
    );
    wenn(
        a.abgaben.iter().any(|b| {
            alt.abgaben
                .iter()
                .find(|v| v.id == b.id)
                .is_some_and(|v| v.versand != b.versand)
        }),
        Versand,
    );
    // Eine nachgetragene externe Einreichung ist eine Übermittlung.
    wenn(
        alt.externe_einreichungen != a.externe_einreichungen,
        Versand,
    );
    wenn(alt.abweichungen != a.abweichungen, Abweichung);
    wenn(alt.gebuehren != a.gebuehren, Gebuehr);
    for b in &a.bescheide {
        match alt.bescheide.iter().find(|v| v.id == b.id) {
            None => wenn(true, Bescheid),
            Some(v) => {
                wenn(v.zugestellt != b.zugestellt, Zustellung);
                wenn(v.auflagen != b.auflagen, Nebenbestimmung);
            }
        }
    }
    let neue_ereignisse = &a.ereignisse[alt.ereignisse.len().min(a.ereignisse.len())..];
    wenn(
        neue_ereignisse.iter().any(|e| e.berichtigung.is_some()),
        Berichtigung,
    );
    let angaben = alt.bezeichnung != a.bezeichnung
        || alt.art != a.art
        || alt.antragsangaben != a.antragsangaben
        || alt.behoerde != a.behoerde
        || alt.aktenzeichen != a.aktenzeichen
        || alt.verantwortlich != a.verantwortlich
        || alt.vorgaenger != a.vorgaenger
        || alt.pruefverfahren != a.pruefverfahren
        || alt.bearbeitungsfrist != a.bearbeitungsfrist
        || alt.profil != a.profil
        || alt.portal != a.portal
        || alt.anforderungen != a.anforderungen
        || alt.dokumentbeziehungen != a.dokumentbeziehungen
        || (!neue_kommunikation && alt.kommunikation != a.kommunikation);
    wenn(angaben, Angaben);
    if r.is_empty() && !neue_ereignisse.is_empty() {
        r.insert(Notiz);
    }
    if r.is_empty() {
        r.insert(Angaben);
    }
    r.into_iter().collect()
}

/// Aktion im Projektprotokoll für eine Revision mit diesen Arten. Bei
/// mehreren Arten gilt die fachlich gewichtigste; der Protokolltext nennt alle.
pub fn aktion(arten: &[Ereignisart]) -> Aktion {
    use Ereignisart::*;
    let hat = |x: &[Ereignisart]| arten.iter().any(|a| x.contains(a));
    if hat(&[Angelegt]) {
        Aktion::VerfahrenAngelegt
    } else if hat(&[Berichtigung]) {
        Aktion::VerfahrenBerichtigt
    } else if hat(&[Status, Vollstaendigkeit, Ruecknahme]) {
        Aktion::VerfahrensstatusGeaendert
    } else if hat(&[Bescheid, Zustellung, Nebenbestimmung]) {
        Aktion::VerfahrenBescheid
    } else if hat(&[Abgabe, Versand]) {
        Aktion::VerfahrenAbgabe
    } else if hat(&[Nachforderung]) {
        Aktion::VerfahrenNachforderung
    } else if hat(&[Schritt]) {
        Aktion::VerfahrenSchritt
    } else if hat(&[Ausfuehrung]) {
        Aktion::VerfahrenAusfuehrung
    } else {
        Aktion::VerfahrenGeaendert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gewichtigste_art_bestimmt_die_protokollaktion() {
        use Ereignisart::*;
        assert_eq!(aktion(&[Angelegt]), Aktion::VerfahrenAngelegt);
        assert_eq!(
            aktion(&[Kommunikation, Bescheid, Status]),
            Aktion::VerfahrensstatusGeaendert
        );
        assert_eq!(aktion(&[Versand, Nachforderung]), Aktion::VerfahrenAbgabe);
        assert_eq!(
            aktion(&[Zustellung, Berichtigung]),
            Aktion::VerfahrenBerichtigt
        );
        assert_eq!(aktion(&[Notiz]), Aktion::VerfahrenGeaendert);
        for a in [
            Aktion::VerfahrenAngelegt,
            Aktion::VerfahrenBerichtigt,
            Aktion::VerfahrenAusfuehrung,
        ] {
            assert_eq!(a.code().parse::<Aktion>().unwrap(), a);
        }
    }

    #[test]
    fn berichtigungsgegenstand_ist_getaggt() {
        let g: Berichtigungsgegenstand =
            serde_json::from_str(r#"{"art":"vollstaendigkeit"}"#).unwrap();
        assert_eq!(g, Berichtigungsgegenstand::Vollstaendigkeit);
        let id = Uuid::new_v4();
        let z = serde_json::to_value(Berichtigungsgegenstand::Zustellung { bescheid: id }).unwrap();
        assert_eq!(z["art"], "zustellung");
        assert_eq!(z["bescheid"], id.to_string());
    }
}
