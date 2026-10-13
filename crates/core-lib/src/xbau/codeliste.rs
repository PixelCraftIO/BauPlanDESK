//! Codelisten im Format OASIS Genericode 1.0, wie sie das XRepository
//! (www.xrepository.de) bereitstellt.
//!
//! Codelisten vom Typ 3 und 4 stehen nicht in den XBau-Schemas; ihre Werte
//! und ihre Version kommen zur Laufzeit aus Genericode-Dateien. So lassen
//! sich neue Versionen ohne Programm-Update einspielen: Datei vom XRepository
//! herunterladen und in den Codelisten-Ordner legen.

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use serde::Serialize;

use crate::error::{CoreError, CoreResult};

/// Art der Anlage (Bauvorlage), Typ 3.
pub const ANLAGEN: &str = "urn:xoev-de:xbau:codeliste:anlagen";
/// MIME-Typen der Anlagen, Typ 3.
pub const MIMETYPES: &str = "urn:xoev-de:xbau:codeliste:xbau-mimetypes";
/// Verzeichnisdienste für Behördenkennungen (DVDV).
pub const VERZEICHNISDIENST: &str = "urn:xoev-de:kosit:codeliste:verzeichnisdienst";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Codeeintrag {
    pub code: String,
    pub beschreibung: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Codeliste {
    /// Kennung, z. B. `urn:xoev-de:xbau:codeliste:anlagen`.
    pub uri: String,
    pub version: String,
    pub name: String,
    pub eintraege: Vec<Codeeintrag>,
}

fn attribut(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == name)
        .map(|a| {
            a.normalized_value(XmlVersion::Implicit1_0)
                .map(|v| v.into_owned())
                .unwrap_or_default()
        })
}

/// `Column` und `Key/ColumnRef` kommen als leeres Element oder mit Kindern vor.
fn spalte_oder_schluessel(
    e: &BytesStart,
    pfad: &[String],
    spalten: &mut Vec<String>,
    schluessel: &mut Option<String>,
) {
    match e.local_name().as_ref() {
        "Column" => spalten.extend(attribut(e, "Id")),
        "ColumnRef" if pfad.last().is_some_and(|p| p == "Key") && schluessel.is_none() => {
            *schluessel = attribut(e, "Ref");
        }
        _ => {}
    }
}

impl Codeliste {
    pub fn enthaelt(&self, code: &str) -> bool {
        self.eintraege.iter().any(|e| e.code == code)
    }

    /// Liest eine Genericode-Datei. Als Code gilt die Spalte des
    /// Schlüssels (`Key/ColumnRef`), als Beschreibung die nächste Spalte.
    pub fn genericode(xml: &str) -> CoreResult<Self> {
        let fehler = |m: &str| CoreError::InvalidInput(format!("Genericode-Codeliste: {m}"));
        let mut reader = Reader::from_str(xml);
        let mut pfad: Vec<String> = Vec::new();
        let mut text = String::new();
        let mut uri = None;
        let mut version = None;
        let mut name = None;
        let mut spalten: Vec<String> = Vec::new();
        let mut schluessel: Option<String> = None;
        let mut zeilen: Vec<BTreeMap<String, String>> = Vec::new();
        let mut spalte: Option<String> = None;
        loop {
            match reader.read_event().map_err(|e| fehler(&e.to_string()))? {
                Event::Start(e) => {
                    let lokal = e.local_name().as_ref().to_string();
                    match lokal.as_str() {
                        "Row" => zeilen.push(BTreeMap::new()),
                        "Value" => spalte = attribut(&e, "ColumnRef"),
                        _ => spalte_oder_schluessel(&e, &pfad, &mut spalten, &mut schluessel),
                    }
                    pfad.push(lokal);
                    text.clear();
                }
                Event::Empty(e) => spalte_oder_schluessel(&e, &pfad, &mut spalten, &mut schluessel),
                Event::Text(t) => text.push_str(&t.xml10_content()),
                Event::CData(t) => text.push_str(&t.xml10_content()),
                Event::GeneralRef(r) => {
                    let n = r.xml10_content();
                    match r.resolve_char_ref().ok().flatten() {
                        Some(c) => text.push(c),
                        None => text.push(match n.as_ref() {
                            "amp" => '&',
                            "lt" => '<',
                            "gt" => '>',
                            "quot" => '"',
                            "apos" => '\'',
                            _ => '?',
                        }),
                    }
                }
                Event::End(_) => {
                    let lokal = pfad.pop().unwrap_or_default();
                    let eltern = pfad.last().map(String::as_str);
                    let wert = text.trim().to_string();
                    match (eltern, lokal.as_str()) {
                        (Some("Identification"), "CanonicalUri") => uri = Some(wert),
                        (Some("Identification"), "Version") => version = Some(wert),
                        (Some("Identification"), "ShortName") => name = name.or(Some(wert)),
                        (Some("Identification"), "LongName") => name = Some(wert),
                        (Some("Value"), "SimpleValue") => {
                            if let (Some(z), Some(s)) = (zeilen.last_mut(), &spalte) {
                                z.insert(s.clone(), wert);
                            }
                        }
                        _ => {}
                    }
                    text.clear();
                }
                Event::Eof => break,
                _ => {}
            }
        }
        let schluessel = schluessel
            .or_else(|| spalten.first().cloned())
            .ok_or_else(|| fehler("keine Spalten"))?;
        let beschreibung = spalten.iter().find(|s| **s != schluessel).cloned();
        let eintraege = zeilen
            .into_iter()
            .filter_map(|mut z| {
                let code = z.remove(&schluessel)?;
                let beschreibung = beschreibung
                    .as_ref()
                    .and_then(|b| z.remove(b))
                    .unwrap_or_default();
                Some(Codeeintrag { code, beschreibung })
            })
            .collect();
        Ok(Self {
            uri: uri.ok_or_else(|| fehler("CanonicalUri fehlt"))?,
            version: version.ok_or_else(|| fehler("Version fehlt"))?,
            name: name.unwrap_or_default(),
            eintraege,
        })
    }
}

/// Alle bekannten Codelisten, nach Kennung.
#[derive(Debug, Clone, Default)]
pub struct Codelisten {
    listen: BTreeMap<String, Codeliste>,
}

impl Codelisten {
    /// Codelisten, die im Programm eingebaut sind (nur solche, deren Inhalt
    /// feststeht).
    pub fn eingebaut() -> Self {
        let mut c = Self::default();
        c.hinzufuegen(Codeliste {
            uri: VERZEICHNISDIENST.into(),
            version: "1".into(),
            name: "Verzeichnisdienst".into(),
            eintraege: vec![Codeeintrag {
                code: "DVDV".into(),
                beschreibung: "Deutsches Verwaltungsdiensteverzeichnis (DVDV)".into(),
            }],
        });
        c
    }

    /// Fügt eine Liste hinzu; eine vorhandene Liste mit derselben Kennung
    /// wird ersetzt.
    pub fn hinzufuegen(&mut self, liste: Codeliste) {
        self.listen.insert(liste.uri.clone(), liste);
    }

    /// Liest alle `*.xml`-Dateien eines Ordners. Fehlerhafte Dateien werden
    /// übersprungen und als Fehler zurückgegeben.
    pub fn laden(&mut self, ordner: &Path) -> CoreResult<Vec<CoreError>> {
        let mut fehler = Vec::new();
        let eintraege = match std::fs::read_dir(ordner) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(fehler),
            Err(e) => {
                return Err(CoreError::InvalidInput(format!(
                    "{}: {e}",
                    ordner.display()
                )));
            }
        };
        let mut pfade: Vec<_> = eintraege
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")))
            .collect();
        pfade.sort();
        for pfad in pfade {
            match std::fs::read_to_string(&pfad)
                .map_err(|e| CoreError::InvalidInput(e.to_string()))
                .and_then(|t| Codeliste::genericode(&t))
            {
                Ok(liste) => self.hinzufuegen(liste),
                Err(e) => fehler.push(CoreError::InvalidInput(format!("{}: {e}", pfad.display()))),
            }
        }
        Ok(fehler)
    }

    pub fn liste(&self, uri: &str) -> Option<&Codeliste> {
        self.listen.get(uri)
    }

    /// Wie [`Self::liste`], aber mit verständlicher Fehlermeldung.
    pub fn benoetigt(&self, uri: &str, name: &str) -> CoreResult<&Codeliste> {
        self.liste(uri).ok_or_else(|| {
            CoreError::InvalidInput(format!(
                "Die Codeliste „{name}“ ({uri}) fehlt. Bitte die aktuelle Version als Genericode-Datei vom XRepository (www.xrepository.de) herunterladen und in den Codelisten-Ordner legen."
            ))
        })
    }

    /// Für den Bauantrag benötigte Listen, die nicht geladen sind.
    pub fn fehlende(&self) -> Vec<String> {
        [(ANLAGEN, "Anlagenarten"), (MIMETYPES, "MIME-Typen")]
            .into_iter()
            .filter(|(uri, _)| self.liste(uri).is_none())
            .map(|(uri, name)| format!("Codeliste {name} ({uri})"))
            .collect()
    }

    pub fn alle(&self) -> impl Iterator<Item = &Codeliste> {
        self.listen.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const BEISPIEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gc:CodeList xmlns:gc="http://docs.oasis-open.org/codelist/ns/genericode/1.0/">
   <Identification>
      <ShortName>verzeichnisdienst</ShortName>
      <LongName>Verzeichnisdienst
</LongName>
      <Version>1</Version>
      <CanonicalUri>urn:xoev-de:kosit:codeliste:verzeichnisdienst</CanonicalUri>
   </Identification>
   <ColumnSet>
      <Column Id="code" Use="required"><ShortName>code</ShortName></Column>
      <Column Id="beschreibung" Use="required"><ShortName>beschreibung</ShortName></Column>
      <Key Id="codeKey"><ShortName>codeKey</ShortName><ColumnRef Ref="code"/></Key>
   </ColumnSet>
   <SimpleCodeList>
      <Row>
         <Value ColumnRef="code"><SimpleValue>DVDV</SimpleValue></Value>
         <Value ColumnRef="beschreibung"><SimpleValue>Deutsches Verwaltungsdiensteverzeichnis (DVDV) &amp; mehr</SimpleValue></Value>
      </Row>
   </SimpleCodeList>
</gc:CodeList>"#;

    #[test]
    fn mitgelieferte_codelisten_sind_vollstaendig() {
        let ordner = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../codelisten");
        let mut c = Codelisten::eingebaut();
        assert!(c.laden(&ordner).unwrap().is_empty());
        assert!(c.fehlende().is_empty(), "{:?}", c.fehlende());
        let anlagen = c.liste(ANLAGEN).unwrap();
        assert_eq!(anlagen.version, "4.0");
        assert!(anlagen.enthaelt("2001"));
        let mimes = c.liste(MIMETYPES).unwrap();
        assert_eq!(mimes.version, "1");
        for datei in ["plan.pdf", "modell.ifc", "modell.ifczip", "kommentare.bcf"] {
            let m = crate::xbau::mime_typ(datei).unwrap();
            assert!(mimes.enthaelt(m), "{m}");
        }
    }

    #[test]
    fn genericode_mit_leeren_spaltenelementen() {
        let xml = r#"<gc:CodeList xmlns:gc="http://docs.oasis-open.org/codelist/ns/genericode/1.0/"><Identification><ShortName>anlagen</ShortName><Version>3</Version><CanonicalUri>urn:x</CanonicalUri></Identification><ColumnSet><Column Id="schluessel"/><Column Id="wert"/><Key Id="k"><ColumnRef Ref="schluessel"/></Key></ColumnSet><SimpleCodeList><Row><Value ColumnRef="wert"><SimpleValue>Lageplan</SimpleValue></Value><Value ColumnRef="schluessel"><SimpleValue>0200</SimpleValue></Value></Row></SimpleCodeList></gc:CodeList>"#;
        let liste = Codeliste::genericode(xml).unwrap();
        assert_eq!(
            liste.eintraege,
            vec![Codeeintrag {
                code: "0200".into(),
                beschreibung: "Lageplan".into()
            }]
        );
    }

    #[test]
    fn liest_genericode() {
        let l = Codeliste::genericode(BEISPIEL).unwrap();
        assert_eq!(l.uri, VERZEICHNISDIENST);
        assert_eq!(l.version, "1");
        assert_eq!(l.name, "Verzeichnisdienst");
        assert_eq!(
            l.eintraege,
            vec![Codeeintrag {
                code: "DVDV".into(),
                beschreibung: "Deutsches Verwaltungsdiensteverzeichnis (DVDV) & mehr".into()
            }]
        );
        assert!(l.enthaelt("DVDV"));
    }

    #[test]
    fn unsinn_wird_abgelehnt() {
        assert!(Codeliste::genericode("<a/>").is_err());
        assert!(Codeliste::genericode("kein xml <").is_err());
    }

    #[test]
    fn ordner_laden() {
        let dir = std::env::temp_dir().join(format!("codelisten-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.xml"), BEISPIEL).unwrap();
        std::fs::write(dir.join("b.xml"), "<kaputt/>").unwrap();
        std::fs::write(dir.join("c.txt"), "egal").unwrap();
        let mut c = Codelisten::default();
        let fehler = c.laden(&dir).unwrap();
        assert_eq!(fehler.len(), 1);
        assert!(c.liste(VERZEICHNISDIENST).is_some());
        assert!(c.benoetigt(ANLAGEN, "XBau Anlage Art").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
