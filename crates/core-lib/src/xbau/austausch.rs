//! Nachrichtenprüfung unabhängig von Oberfläche, Ablage und Transport.
//! XML/ZIP bleibt unverändert; aus XML gelesene Identitäten sind keine Authentisierung.
use crate::{CoreError, CoreResult, storage::ContentHash};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quick_xml::{NsReader, events::Event, name::ResolveResult};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read},
    sync::Arc,
};
use uuid::Uuid;
pub const HOCHBAU: &str = "http://www.xleitstelle.de/xbau/2/7";
pub const KERN: &str = "http://www.xleitstelle.de/xbau/kernmodul/1/4/0";
pub const MAX_PAKET: u64 = 64 * 1024 * 1024;
pub const MAX_XML: u64 = 8 * 1024 * 1024;
pub const MAX_ENTPACKT: u64 = 128 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachrichtentyp {
    pub element: String,
    pub namensraum: String,
    pub titel: String,
    pub schema: String,
}
pub fn katalog() -> Vec<Nachrichtentyp> {
    serde_json::from_str(include_str!("nachrichten.json")).expect("eingebauter XBau-Katalog")
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachrichtenkopf {
    pub id: Uuid,
    pub element: String,
    pub namensraum: String,
    pub titel: String,
    pub erstellt: DateTime<Utc>,
    pub absender: String,
    pub empfaenger: String,
    pub antragsreferenz: Option<Uuid>,
    pub aktenzeichen: Option<String>,
    pub bezugsnachricht: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct AnlagePruefung {
    pub dateiname: String,
    pub dokumentid: String,
    #[ts(type = "number")]
    pub bytes: u64,
    pub sha256: String,
    pub hash_in_nachricht: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Pruefbericht {
    pub kopf: Nachrichtenkopf,
    pub sha256: String,
    pub anlagen: Vec<AnlagePruefung>,
    pub schemapruefer: String,
    pub geprueft_am: DateTime<Utc>,
    pub hinweise: Vec<String>,
}
/// Implementierung muss ausschließlich das fest mitgelieferte Schema prüfen;
/// schemaLocation der Nachricht darf niemals die Validator-Konfiguration bestimmen.
#[async_trait]
pub trait SchemaPruefer: Send + Sync {
    async fn pruefen(&self, xml: &[u8]) -> CoreResult<String>;
}
pub struct KeinSchemaPruefer;
#[async_trait]
impl SchemaPruefer for KeinSchemaPruefer {
    async fn pruefen(&self, _: &[u8]) -> CoreResult<String> {
        Err(fehler(
            "XBau-XSD-Prüfer nicht eingerichtet. XML-Validator installieren bzw. konfigurieren; Nachricht wurde nicht freigegeben.",
        ))
    }
}
pub fn kein_pruefer() -> Arc<dyn SchemaPruefer> {
    Arc::new(KeinSchemaPruefer)
}
pub(crate) fn fehler(t: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(format!("XBau: {}", t.into()))
}
pub(crate) fn dateiname(n: &str) -> CoreResult<()> {
    if n.is_empty()
        || n.len() > 240
        || n == "."
        || n == ".."
        || n.ends_with([' ', '.'])
        || n.chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
    {
        return Err(fehler(
            "Nur eindeutige Dateinamen ohne Pfade sind im XBau-Paket zulässig",
        ));
    }
    Ok(())
}
struct Anhang {
    name: String,
    id: String,
    hash: Option<String>,
    size: Option<u64>,
}
struct Gelesen {
    kopf: Nachrichtenkopf,
    anhaenge: Vec<Anhang>,
    links: usize,
}
// Die Pfade berücksichtigen gerade den unqualifizierten XÖV-Nachrichtenkopf.
fn xml_lesen(xml: &[u8]) -> CoreResult<Gelesen> {
    if xml.len() as u64 > MAX_XML {
        return Err(fehler("XML größer als 8 MiB"));
    }
    let raw = std::str::from_utf8(xml).map_err(|_| fehler("Nur UTF-8 XML wird unterstützt"))?;
    let mut r = NsReader::from_str(raw);
    r.config_mut().expand_empty_elements = true;
    let mut pfad: Vec<(String, String)> = vec![];
    let mut werte: BTreeMap<String, String> = BTreeMap::new();
    let mut element = None;
    let mut fertig = false;
    let mut anhaenge = vec![];
    let mut anhang: Option<(usize, Anhang)> = None;
    let mut links = 0;
    let mut nodes = 0;
    loop {
        let (ns, e) = r
            .read_resolved_event()
            .map_err(|e| fehler(format!("XML nicht lesbar: {e}")))?;
        match e {
            Event::Start(e) => {
                nodes += 1;
                if nodes > 100000 || pfad.len() >= 64 {
                    return Err(fehler(
                        "XML-Struktur überschreitet die zulässige Komplexität",
                    ));
                }
                let ns = match ns {
                    ResolveResult::Bound(n) => n.as_ref().to_string(),
                    ResolveResult::Unbound => String::new(),
                    ResolveResult::Unknown(_) => return Err(fehler("Unbekanntes XML-Präfix")),
                };
                let name = e.local_name().as_ref().to_string();
                if pfad.is_empty() {
                    if fertig {
                        return Err(fehler("Mehrere XML-Wurzelelemente"));
                    }
                    let typ=katalog().into_iter().find(|t|t.element==name && t.namensraum==ns).ok_or_else(||fehler("Nachrichtentyp oder XBau-Version wird nicht unterstützt (Hochbau 2.7 / Kernmodul 1.4.0)"))?;
                    element = Some(typ);
                }
                // Keine Namensraum-Imitation der Metadaten.
                pfad.push((ns.clone(), name.clone()));
                let p = pfad
                    .iter()
                    .skip(1)
                    .map(|(_, n)| n.as_str())
                    .collect::<Vec<_>>()
                    .join("/");
                let meta = p.starts_with("nachrichtenkopf.g2g/")
                    || p.starts_with("bezug/")
                    || p.starts_with("referenzAntragsservice/")
                    || p.starts_with("referenzAnzeigeservice/");
                if meta
                    && !ns.is_empty()
                    && ns != HOCHBAU
                    && ns != KERN
                    && !(p.starts_with("bezug/bezugNachricht/")
                        && ns == "http://xoev.de/schemata/basisnachricht/g2g/1_1")
                {
                    return Err(fehler("Fremder Namensraum im Nachrichtenbezug"));
                }
                if pfad.len() == 2 && name == "nachrichtenkopf.g2g" && !ns.is_empty() {
                    return Err(fehler("Nachrichtenkopf muss unqualifiziert sein"));
                }
                if [
                    "nachrichtenkopf.g2g",
                    "nachrichtenkopf.g2g/identifikation.nachricht",
                    "nachrichtenkopf.g2g/identifikation.nachricht/nachrichtenUUID",
                    "nachrichtenkopf.g2g/identifikation.nachricht/nachrichtentyp",
                    "nachrichtenkopf.g2g/identifikation.nachricht/nachrichtentyp/code",
                    "nachrichtenkopf.g2g/identifikation.nachricht/erstellungszeitpunkt",
                    "nachrichtenkopf.g2g/autor",
                    "nachrichtenkopf.g2g/autor/kennung",
                    "nachrichtenkopf.g2g/leser",
                    "nachrichtenkopf.g2g/leser/kennung",
                    "bezug",
                    "bezug/referenz",
                    "bezug/vorgang",
                    "bezug/bezugNachricht",
                    "bezug/bezugNachricht/nachrichtenUUID",
                    "referenzAntragsservice/referenzAntragsservice",
                    "referenzAnzeigeservice/referenzAntragsservice",
                ]
                .contains(&p.as_str())
                    && werte.insert(format!("#{}", p), String::new()).is_some()
                {
                    return Err(fehler("Mehrdeutiger Nachrichtenkopf oder Bezug"));
                }
                let attrs = e
                    .attributes()
                    .map(|a| a.map_err(|e| fehler(e.to_string())))
                    .collect::<CoreResult<Vec<_>>>()?;
                if name == "anhang" && (ns.is_empty() || ns == KERN || ns == HOCHBAU) {
                    if anhang.is_some() {
                        return Err(fehler("Verschachtelte Anhänge"));
                    }
                    let get = |key: &str| -> CoreResult<Option<String>> {
                        attrs
                            .iter()
                            .find(|a| a.key.as_ref() == key)
                            .map(|a| {
                                a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .map(|s| s.to_string())
                                    .map_err(|e| fehler(e.to_string()))
                            })
                            .transpose()
                    };
                    anhang = Some((
                        pfad.len(),
                        Anhang {
                            name: String::new(),
                            id: String::new(),
                            hash: get("hashValue")?,
                            size: get("filesize")?
                                .map(|s| {
                                    s.parse::<u64>()
                                        .map_err(|_| fehler("Ungültige Anhanggröße"))
                                })
                                .transpose()?,
                        },
                    ));
                }
                if name == "verlinkung" {
                    links += 1;
                }
            }
            Event::Text(t) => {
                let t = quick_xml::escape::unescape(&t.xml10_content())
                    .map_err(|e| fehler(e.to_string()))?
                    .to_string();
                text(&pfad, &mut werte, &mut anhang, &t)?;
            }
            Event::CData(t) => text(&pfad, &mut werte, &mut anhang, &t.xml10_content())?,
            Event::GeneralRef(e) => {
                let s = if let Some(c) = e.resolve_char_ref().map_err(|e| fehler(e.to_string()))? {
                    c.to_string()
                } else {
                    match e.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err(fehler("Unzulässige XML-Entität")),
                    }
                    .into()
                };
                text(&pfad, &mut werte, &mut anhang, &s)?;
            }
            Event::End(_) => {
                if let Some((tiefe, _)) = &anhang
                    && *tiefe == pfad.len()
                {
                    anhaenge.push(anhang.take().unwrap().1);
                }
                if pfad.pop().is_none() {
                    return Err(fehler("XML-Endelement ohne Anfang"));
                }
                if pfad.is_empty() {
                    fertig = true;
                }
            }
            Event::DocType(_) | Event::PI(_) => {
                return Err(fehler(
                    "DTD und Verarbeitungsanweisungen sind nicht zulässig",
                ));
            }
            Event::Decl(d) => {
                if let Some(enc) = d.encoding()
                    && !enc
                        .map_err(|e| fehler(e.to_string()))?
                        .eq_ignore_ascii_case("UTF-8")
                {
                    return Err(fehler("Nur UTF-8 XML wird unterstützt"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !fertig || !pfad.is_empty() {
        return Err(fehler("Unvollständiges XML"));
    }
    let typ = element.ok_or_else(|| fehler("Leeres XML"))?;
    let wert = |k: &str| werte.get(k).map(|s| s.trim()).filter(|s| !s.is_empty());
    let uuid = |k: &str| -> CoreResult<Option<Uuid>> {
        wert(k)
            .map(|s| {
                s.parse()
                    .map_err(|_| fehler("Ungültige Nachrichten-/Vorgangs-ID"))
            })
            .transpose()
    };
    let id = uuid("nachrichtenkopf.g2g/identifikation.nachricht/nachrichtenUUID")?
        .ok_or_else(|| fehler("Nachrichten-ID fehlt"))?;
    let erstellt = wert("nachrichtenkopf.g2g/identifikation.nachricht/erstellungszeitpunkt")
        .ok_or_else(|| fehler("Erstellungszeitpunkt fehlt"))?
        .parse::<DateTime<Utc>>()
        .map_err(|_| fehler("Erstellungszeitpunkt ungültig"))?;
    let code = wert("nachrichtenkopf.g2g/identifikation.nachricht/nachrichtentyp/code")
        .ok_or_else(|| fehler("Nachrichtencode fehlt"))?;
    if !typ.element.ends_with(&format!(".{code}")) {
        return Err(fehler("Nachrichtencode passt nicht zum Wurzelelement"));
    }
    let absender = wert("nachrichtenkopf.g2g/autor/kennung")
        .ok_or_else(|| fehler("Absenderkennung fehlt"))?
        .to_string();
    let empfaenger = wert("nachrichtenkopf.g2g/leser/kennung")
        .ok_or_else(|| fehler("Empfängerkennung fehlt"))?
        .to_string();
    Ok(Gelesen {
        kopf: Nachrichtenkopf {
            id,
            element: typ.element,
            namensraum: typ.namensraum,
            titel: typ.titel,
            erstellt,
            absender,
            empfaenger,
            antragsreferenz: uuid("referenzAntragsservice/referenzAntragsservice")?
                .or(uuid("referenzAnzeigeservice/referenzAntragsservice")?)
                .or(uuid("bezug/referenz")?),
            aktenzeichen: wert("bezug/vorgang").map(String::from),
            bezugsnachricht: uuid("bezug/bezugNachricht/nachrichtenUUID")?,
        },
        anhaenge,
        links,
    })
}
fn text(
    pfad: &[(String, String)],
    werte: &mut BTreeMap<String, String>,
    anhang: &mut Option<(usize, Anhang)>,
    t: &str,
) -> CoreResult<()> {
    if pfad.is_empty() {
        if !t.trim().is_empty() {
            return Err(fehler("Text außerhalb des XML-Wurzelelements"));
        }
        return Ok(());
    }
    let p = pfad
        .iter()
        .skip(1)
        .map(|(_, n)| n.as_str())
        .collect::<Vec<_>>()
        .join("/");
    // Nur den für Routing benötigten kleinen Teil behalten; Fachinhalt bleibt im Original.
    if p.starts_with("nachrichtenkopf.g2g/")
        || p.starts_with("bezug/")
        || p.starts_with("referenzAntragsservice/")
        || p.starts_with("referenzAnzeigeservice/")
    {
        let s = werte.entry(p).or_default();
        if s.len() + t.len() > 16384 {
            return Err(fehler("Nachrichtenkopf zu lang"));
        }
        s.push_str(t);
    }
    if let Some((tiefe, a)) = anhang
        && pfad.len() == *tiefe + 1
    {
        match pfad.last().unwrap().1.as_str() {
            "dateiname" => a.name.push_str(t),
            "dokumentid" => a.id.push_str(t),
            _ => {}
        }
    }
    Ok(())
}
pub fn vorpruefen(xml: &[u8]) -> CoreResult<Nachrichtenkopf> {
    Ok(xml_lesen(xml)?.kopf)
}
pub async fn pruefen(
    name: &str,
    daten: &[u8],
    pruefer: &dyn SchemaPruefer,
) -> CoreResult<Pruefbericht> {
    if daten.len() as u64 > MAX_PAKET {
        return Err(fehler("Paket größer als 64 MiB"));
    }
    let mut dateien = BTreeMap::new();
    let xml = if name.to_ascii_lowercase().ends_with(".zip") {
        let mut z = zip::ZipArchive::new(Cursor::new(daten))
            .map_err(|e| fehler(format!("ZIP nicht lesbar: {e}")))?;
        if z.is_empty() || z.len() > 256 {
            return Err(fehler("Paket benötigt 1 bis 256 Dateien"));
        }
        let mut gesamt = 0u64;
        let mut namen = HashSet::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| fehler(e.to_string()))?;
            let n = f.name().map_err(|e| fehler(e.to_string()))?.into_owned();
            dateiname(&n)?;
            if !namen.insert(n.to_lowercase())
                || f.is_dir()
                || f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err(fehler("Doppelte Namen, Ordner oder Verknüpfungen im Paket"));
            }
            gesamt = gesamt
                .checked_add(f.size())
                .ok_or_else(|| fehler("ZIP-Größe ungültig"))?;
            if gesamt > MAX_ENTPACKT || f.size() > MAX_PAKET {
                return Err(fehler("Entpackte Paketgröße überschritten"));
            }
            let mut b = vec![];
            (&mut f)
                .take(MAX_PAKET + 1)
                .read_to_end(&mut b)
                .map_err(|e| fehler(e.to_string()))?;
            if b.len() as u64 > MAX_PAKET {
                return Err(fehler("Dateigröße überschritten"));
            }
            dateien.insert(n, b);
        }
        // Eine Bauvorlage darf selbst XML sein. Nur XML mit einem XBau-Wurzelelement
        // ist Nachrichtenkandidat; beliebiges fehlerhaftes Nachrichten-XML bleibt Fehler.
        let kandidaten: Vec<_> = dateien
            .iter()
            .filter(|(n, b)| n.to_ascii_lowercase().ends_with(".xml") && xml_lesen(b).is_ok())
            .map(|(n, _)| n.clone())
            .collect();
        if kandidaten.len() != 1 {
            return Err(fehler(
                "Genau eine lesbare XBau-Nachricht pro Paket erforderlich",
            ));
        }
        dateien.remove(&kandidaten[0]).unwrap()
    } else if name.to_ascii_lowercase().ends_with(".xml") {
        daten.to_vec()
    } else {
        return Err(fehler("Erwartet wird eine XML- oder ZIP-Datei"));
    };
    let g = xml_lesen(&xml)?;
    let engine = pruefer.pruefen(&xml).await?;
    let mut anlagen = vec![];
    let mut ids = HashSet::new();
    let mut refs = HashSet::new();
    for a in g.anhaenge {
        dateiname(&a.name)?;
        if a.id.trim().is_empty()
            || !ids.insert(a.id.clone())
            || !refs.insert(a.name.to_lowercase())
        {
            return Err(fehler(
                "Anhänge benötigen eindeutige Dokumentkennungen und Dateinamen",
            ));
        }
        let b = dateien
            .remove(&a.name)
            .ok_or_else(|| fehler(format!("Anlage fehlt: {}", a.name)))?;
        let hash = ContentHash::of(&b).to_string();
        if a.size.is_some_and(|s| s != b.len() as u64)
            || a.hash
                .as_ref()
                .is_some_and(|h| !h.eq_ignore_ascii_case(&hash))
        {
            return Err(fehler(format!(
                "Anlage beschädigt (Größe/SHA-256): {}",
                a.name
            )));
        }
        anlagen.push(AnlagePruefung {
            dateiname: a.name,
            dokumentid: a.id,
            bytes: b.len() as u64,
            sha256: hash,
            hash_in_nachricht: a.hash.is_some(),
        });
    }
    if !dateien.is_empty() {
        return Err(fehler("Nicht referenzierte Dateien im Paket"));
    }
    let mut hinweise=vec!["Absender und Empfänger stammen aus der Datei und sind nicht authentisiert. XSD-Gültigkeit ist keine behördliche Annahme.".into(),"Dynamische Typ-3/4-Codelisten und empfangsspezifische Regeln sind gesondert zu prüfen.".into()];
    if g.links > 0 {
        hinweise.push("Nachricht enthält externe Verlinkungen. Diese wurden nicht abgerufen oder auf Vollständigkeit geprüft.".into());
    }
    if anlagen.iter().any(|a| !a.hash_in_nachricht) {
        hinweise.push("Für mindestens eine Anlage fehlt ein SHA-256 in der XML-Nachricht. Der Archivhash wurde lokal berechnet.".into());
    }
    Ok(Pruefbericht {
        kopf: g.kopf,
        sha256: ContentHash::of(daten).to_string(),
        anlagen,
        schemapruefer: engine,
        geprueft_am: Utc::now(),
        hinweise,
    })
}

/// Nur für einen bereits geprüften Archivbezug: exakt benannte Anlage laden,
/// begrenzen und nochmals gegen den archivierten Inhalts-Hash prüfen.
pub(crate) fn anlage_lesen(paket: &[u8], name: &str, sha256: &str) -> CoreResult<Vec<u8>> {
    dateiname(name)?;
    if paket.len() as u64 > MAX_PAKET {
        return Err(fehler("Paketgröße überschritten"));
    }
    let mut z = zip::ZipArchive::new(Cursor::new(paket)).map_err(|e| fehler(e.to_string()))?;
    let mut f = z.by_name(name).map_err(|e| fehler(e.to_string()))?;
    if f.size() > MAX_PAKET {
        return Err(fehler("Anlagengröße überschritten"));
    }
    let mut b = vec![];
    (&mut f)
        .take(MAX_PAKET + 1)
        .read_to_end(&mut b)
        .map_err(|e| fehler(e.to_string()))?;
    if b.len() as u64 > MAX_PAKET || ContentHash::of(&b).as_str() != sha256 {
        return Err(fehler("Anlageninhalt passt nicht zum Prüfbericht"));
    }
    Ok(b)
}
