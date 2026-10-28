//! Lesen und Schreiben von IFC-Dateien im Klartextformat ISO 10303-21 (STEP,
//! „Part 21“). Der Leser kennt kein Schema: Jede Instanz `#id=TYP(…);` wird mit
//! ihren Attributen als [`Wert`]-Baum abgelegt. Der Schreiber gibt
//! unveränderte Instanzen Zeichen für Zeichen aus der Quelle wieder, damit ein
//! nachgearbeitetes Modell außer den Änderungen dateigleich bleibt.
use crate::{CoreError, CoreResult};
use std::collections::BTreeMap;

/// Attributwert einer STEP-Instanz.
#[derive(Debug, Clone, PartialEq)]
pub enum Wert {
    /// `$`: nicht gesetzt.
    Null,
    /// `*`: vom Supertyp abgeleitet.
    Abgeleitet,
    Ganz(i64),
    Real(f64),
    Text(String),
    /// `.WERT.` ohne Punkte, Großbuchstaben.
    Enum(String),
    Ref(u64),
    Liste(Vec<Wert>),
    /// `IFCLABEL('…')`: Typname in Großbuchstaben und Wert.
    Typisiert(String, Box<Wert>),
    /// `"…"`: Binärwert in Hexadezimalschreibweise.
    Binaer(String),
}

impl Wert {
    pub fn referenz(&self) -> Option<u64> {
        match self {
            Self::Ref(r) => Some(*r),
            _ => None,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t),
            Self::Typisiert(_, w) => w.text(),
            _ => None,
        }
    }
    /// Zahl aus Real, Ganzzahl oder typisiertem Messwert.
    pub fn zahl(&self) -> Option<f64> {
        match self {
            Self::Real(r) => Some(*r),
            Self::Ganz(i) => Some(*i as f64),
            Self::Typisiert(_, w) => w.zahl(),
            _ => None,
        }
    }
    pub fn liste(&self) -> &[Wert] {
        match self {
            Self::Liste(l) => l,
            _ => &[],
        }
    }
    pub fn enum_wert(&self) -> Option<&str> {
        match self {
            Self::Enum(e) => Some(e),
            Self::Typisiert(_, w) => w.enum_wert(),
            _ => None,
        }
    }
    /// Referenzen in einer Liste (andere Einträge werden übergangen).
    pub fn referenzen(&self) -> Vec<u64> {
        self.liste().iter().filter_map(Wert::referenz).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instanz {
    pub id: u64,
    /// Entitätsname in Großbuchstaben, z. B. `IFCWALL`.
    pub typ: String,
    pub attribute: Vec<Wert>,
}

impl Instanz {
    pub fn attr(&self, i: usize) -> &Wert {
        self.attribute.get(i).unwrap_or(&Wert::Null)
    }
}

/// Gelesene STEP-Datei mit Quelltext für das verlustfreie Zurückschreiben.
#[derive(Debug, Clone)]
pub struct StepDatei {
    /// Schemabezeichner aus `FILE_SCHEMA`, z. B. `IFC4`, `IFC2X3`, `IFC4X3_ADD2`.
    pub schema: String,
    /// Erzeugendes System aus `FILE_NAME` (`originating_system`), sofern angegeben.
    pub system: Option<String>,
    pub instanzen: BTreeMap<u64, Instanz>,
    quelle: String,
    /// Kopfteil von `ISO-10303-21;` bis einschließlich `ENDSEC;` vor `DATA;`.
    kopf: (usize, usize),
    /// Quelltextbereich jeder Instanz (`#id=…;`) in Dateireihenfolge.
    bereiche: Vec<(u64, usize, usize)>,
}

/// Obergrenze für eine einzelne Modelldatei (unkomprimiert).
pub const MAX_BYTES: usize = 400 * 1024 * 1024;
const MAX_TIEFE: usize = 64;

fn fehler(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(msg.into())
}

struct Leser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Leser<'a> {
    fn leer(&mut self) {
        loop {
            while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.s[self.pos..].starts_with(b"/*") {
                match find(&self.s[self.pos + 2..], b"*/") {
                    Some(e) => self.pos += e + 4,
                    None => self.pos = self.s.len(),
                }
            } else {
                return;
            }
        }
    }
    fn zeichen(&mut self) -> Option<u8> {
        self.leer();
        self.s.get(self.pos).copied()
    }
    fn erwarte(&mut self, c: u8) -> CoreResult<()> {
        if self.zeichen() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.fehler(&format!("„{}“ erwartet", c as char)))
        }
    }
    fn fehler(&self, was: &str) -> CoreError {
        let zeile = self.s[..self.pos.min(self.s.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1;
        fehler(format!("IFC-Datei nicht lesbar (Zeile {zeile}): {was}"))
    }
    fn name(&mut self) -> String {
        self.leer();
        let start = self.pos;
        while self.pos < self.s.len()
            && (self.s[self.pos].is_ascii_alphanumeric() || self.s[self.pos] == b'_')
        {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.pos]).to_ascii_uppercase()
    }
    fn wert(&mut self, tiefe: usize) -> CoreResult<Wert> {
        if tiefe > MAX_TIEFE {
            return Err(self.fehler("zu tief verschachtelt"));
        }
        match self.zeichen() {
            Some(b'$') => {
                self.pos += 1;
                Ok(Wert::Null)
            }
            Some(b'*') => {
                self.pos += 1;
                Ok(Wert::Abgeleitet)
            }
            Some(b'#') => {
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.s.len() && self.s[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
                std::str::from_utf8(&self.s[start..self.pos])
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .map(Wert::Ref)
                    .ok_or_else(|| self.fehler("Instanzverweis ungültig"))
            }
            Some(b'\'') => self.text().map(Wert::Text),
            Some(b'"') => {
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.s.len() && self.s[self.pos] != b'"' {
                    self.pos += 1;
                }
                let t = String::from_utf8_lossy(&self.s[start..self.pos]).into_owned();
                self.erwarte(b'"')?;
                Ok(Wert::Binaer(t))
            }
            Some(b'.') => {
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.s.len() && self.s[self.pos] != b'.' {
                    self.pos += 1;
                }
                let t = String::from_utf8_lossy(&self.s[start..self.pos]).to_ascii_uppercase();
                self.erwarte(b'.')?;
                Ok(Wert::Enum(t))
            }
            Some(b'(') => {
                self.pos += 1;
                let mut l = Vec::new();
                if self.zeichen() == Some(b')') {
                    self.pos += 1;
                    return Ok(Wert::Liste(l));
                }
                loop {
                    l.push(self.wert(tiefe + 1)?);
                    match self.zeichen() {
                        Some(b',') => self.pos += 1,
                        Some(b')') => {
                            self.pos += 1;
                            return Ok(Wert::Liste(l));
                        }
                        _ => return Err(self.fehler("„,“ oder „)“ erwartet")),
                    }
                }
            }
            Some(c) if c == b'-' || c == b'+' || c.is_ascii_digit() => {
                let start = self.pos;
                self.pos += 1;
                let mut real = false;
                while self.pos < self.s.len() {
                    let c = self.s[self.pos];
                    if c.is_ascii_digit() {
                    } else if c == b'.' || c == b'E' || c == b'e' {
                        real = true;
                    } else if (c == b'-' || c == b'+')
                        && matches!(self.s[self.pos - 1], b'E' | b'e')
                    {
                    } else {
                        break;
                    }
                    self.pos += 1;
                }
                let t = std::str::from_utf8(&self.s[start..self.pos]).unwrap_or("");
                if real {
                    // STEP erlaubt „1.“ und „1.E3“; Rust liest beides.
                    t.parse()
                        .map(Wert::Real)
                        .map_err(|_| self.fehler("Zahl ungültig"))
                } else {
                    t.parse::<i64>()
                        .map(Wert::Ganz)
                        .or_else(|_| t.parse().map(Wert::Real))
                        .map_err(|_| self.fehler("Zahl ungültig"))
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let typ = self.name();
                self.erwarte(b'(')?;
                let w = if self.zeichen() == Some(b')') {
                    Wert::Null
                } else {
                    self.wert(tiefe + 1)?
                };
                self.erwarte(b')')?;
                Ok(Wert::Typisiert(typ, Box::new(w)))
            }
            _ => Err(self.fehler("Attributwert erwartet")),
        }
    }
    fn text(&mut self) -> CoreResult<String> {
        self.pos += 1;
        let mut roh = Vec::new();
        loop {
            match self.s.get(self.pos) {
                None => return Err(self.fehler("Zeichenkette nicht abgeschlossen")),
                Some(b'\'') if self.s.get(self.pos + 1) == Some(&b'\'') => {
                    roh.push(b'\'');
                    self.pos += 2;
                }
                Some(b'\'') => {
                    self.pos += 1;
                    return Ok(entschluesseln(&roh));
                }
                Some(&c) => {
                    roh.push(c);
                    self.pos += 1;
                }
            }
        }
    }
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

/// STEP-Zeichenketten: `\X2\…\X0\` (UTF-16), `\X4\…\X0\` (UTF-32), `\X\HH`
/// (ISO 8859-1), `\S\c` (Zeichen + 128), `\\` und `\P?\` (Codeseite, ignoriert).
/// Unmaskierte Bytes über 127 werden als UTF-8 gelesen, wie es viele Exporte tun.
pub fn entschluesseln(roh: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    let hex = |b: &[u8]| u32::from_str_radix(std::str::from_utf8(b).ok()?, 16).ok();
    let mut bytes = Vec::new();
    let flush = |bytes: &mut Vec<u8>, out: &mut String| {
        if !bytes.is_empty() {
            out.push_str(&String::from_utf8_lossy(bytes));
            bytes.clear();
        }
    };
    while i < roh.len() {
        if roh[i] == b'\\' {
            let rest = &roh[i..];
            if rest.starts_with(b"\\\\") {
                bytes.push(b'\\');
                i += 2;
                continue;
            }
            if rest.starts_with(b"\\X2\\") || rest.starts_with(b"\\X4\\") {
                let breite = if rest[2] == b'2' { 4 } else { 8 };
                if let Some(ende) = find(&rest[4..], b"\\X0\\") {
                    flush(&mut bytes, &mut out);
                    let h = &rest[4..4 + ende];
                    let einheiten: Vec<u32> = h.chunks(breite).filter_map(hex).collect();
                    if breite == 4 {
                        let u16s: Vec<u16> = einheiten.iter().map(|&u| u as u16).collect();
                        out.push_str(&String::from_utf16_lossy(&u16s));
                    } else {
                        out.extend(einheiten.into_iter().filter_map(char::from_u32));
                    }
                    i += 4 + ende + 4;
                    continue;
                }
            }
            if rest.starts_with(b"\\X\\")
                && rest.len() >= 5
                && let Some(c) = hex(&rest[3..5]).and_then(char::from_u32)
            {
                flush(&mut bytes, &mut out);
                out.push(c);
                i += 5;
                continue;
            }
            if rest.starts_with(b"\\S\\") && rest.len() >= 4 {
                flush(&mut bytes, &mut out);
                if let Some(c) = char::from_u32(rest[3] as u32 + 128) {
                    out.push(c);
                }
                i += 4;
                continue;
            }
            if rest.starts_with(b"\\P") && rest.len() >= 4 && rest[3] == b'\\' {
                i += 4;
                continue;
            }
        }
        bytes.push(roh[i]);
        i += 1;
    }
    flush(&mut bytes, &mut out);
    out
}

/// Gegenstück zu [`entschluesseln`]: ASCII bleibt, alles andere als `\X2\`.
pub fn verschluesseln(t: &str) -> String {
    let mut out = String::with_capacity(t.len() + 2);
    out.push('\'');
    let mut u16s: Vec<u16> = Vec::new();
    let flush = |u16s: &mut Vec<u16>, out: &mut String| {
        if !u16s.is_empty() {
            out.push_str("\\X2\\");
            for u in u16s.iter() {
                out.push_str(&format!("{u:04X}"));
            }
            out.push_str("\\X0\\");
            u16s.clear();
        }
    };
    for c in t.chars() {
        if c.is_ascii() && !c.is_ascii_control() {
            flush(&mut u16s, &mut out);
            match c {
                '\'' => out.push_str("''"),
                '\\' => out.push_str("\\\\"),
                c => out.push(c),
            }
        } else {
            let mut b = [0u16; 2];
            u16s.extend_from_slice(c.encode_utf16(&mut b));
        }
    }
    flush(&mut u16s, &mut out);
    out.push('\'');
    out
}

/// Schreibt einen Attributwert in STEP-Schreibweise.
pub fn schreiben(w: &Wert, out: &mut String) {
    match w {
        Wert::Null => out.push('$'),
        Wert::Abgeleitet => out.push('*'),
        Wert::Ganz(i) => out.push_str(&i.to_string()),
        Wert::Real(r) => out.push_str(&real(*r)),
        Wert::Text(t) => out.push_str(&verschluesseln(t)),
        Wert::Enum(e) => {
            out.push('.');
            out.push_str(e);
            out.push('.');
        }
        Wert::Ref(r) => {
            out.push('#');
            out.push_str(&r.to_string());
        }
        Wert::Liste(l) => {
            out.push('(');
            for (i, w) in l.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                schreiben(w, out);
            }
            out.push(')');
        }
        Wert::Typisiert(t, w) => {
            out.push_str(t);
            out.push('(');
            schreiben(w, out);
            out.push(')');
        }
        Wert::Binaer(b) => {
            out.push('"');
            out.push_str(b);
            out.push('"');
        }
    }
}

/// Realzahl mit Dezimalpunkt, wie STEP ihn verlangt (`1.`, `0.25`, `1.E-05`).
pub fn real(r: f64) -> String {
    if !r.is_finite() {
        return "0.".into();
    }
    let t = if r != 0. && (r.abs() >= 1e15 || r.abs() < 1e-6) {
        format!("{r:e}")
    } else {
        format!("{r}")
    };
    if t.contains('e') {
        let (m, e) = t.split_once('e').unwrap_or((&t, "0"));
        let m = if m.contains('.') {
            m.to_string()
        } else {
            format!("{m}.")
        };
        format!("{m}E{e}")
    } else if t.contains('.') {
        t
    } else {
        format!("{t}.")
    }
}

pub fn instanz_text(i: &Instanz) -> String {
    let mut out = format!("#{}={}(", i.id, i.typ);
    for (n, w) in i.attribute.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        schreiben(w, &mut out);
    }
    out.push_str(");");
    out
}

impl StepDatei {
    pub fn lesen(daten: &[u8]) -> CoreResult<Self> {
        if daten.len() > MAX_BYTES {
            return Err(fehler("IFC-Datei ist größer als 400 MB"));
        }
        let start = find(daten, b"ISO-10303-21;")
            .ok_or_else(|| fehler("Keine IFC-Datei im STEP-Format (ISO-10303-21 fehlt). IFC-XML wird nicht unterstützt."))?;
        let data = find(daten, b"DATA;").ok_or_else(|| fehler("IFC-Datei ohne DATA-Abschnitt"))?;
        let kopf = &daten[start..data];
        let schema = kopf_text(kopf, b"FILE_SCHEMA")
            .and_then(|w| {
                let erster = w.liste().first()?;
                erster
                    .text()
                    .or_else(|| erster.liste().first()?.text())
                    .map(str::to_string)
            })
            .unwrap_or_default()
            .to_ascii_uppercase();
        let system = kopf_text(kopf, b"FILE_NAME").and_then(|w| match &w {
            Wert::Liste(l) => l
                .get(5)
                .and_then(|w| w.text())
                .filter(|t| !t.trim().is_empty())
                .map(str::to_string),
            _ => None,
        });
        let mut l = Leser {
            s: daten,
            pos: data + 5,
        };
        let mut instanzen = BTreeMap::new();
        let mut bereiche = Vec::new();
        loop {
            match l.zeichen() {
                Some(b'#') => {
                    let anfang = l.pos;
                    l.pos += 1;
                    let id = match l.wert(0) {
                        Ok(Wert::Ganz(i)) if i > 0 => i as u64,
                        _ => return Err(l.fehler("Instanznummer ungültig")),
                    };
                    l.erwarte(b'=')?;
                    let (typ, attribute) = if l.zeichen() == Some(b'(') {
                        // Komplexe Instanz (mehrere Teiltypen): als Ganzes behalten, nicht ausgewertet.
                        ("(KOMPLEX)".to_string(), vec![l.wert(0)?])
                    } else {
                        let typ = l.name();
                        if typ.is_empty() {
                            return Err(l.fehler("Entitätsname erwartet"));
                        }
                        match l.wert(0)? {
                            Wert::Liste(a) => (typ, a),
                            _ => return Err(l.fehler("Attributliste erwartet")),
                        }
                    };
                    l.erwarte(b';')?;
                    bereiche.push((id, anfang, l.pos));
                    if instanzen
                        .insert(id, Instanz { id, typ, attribute })
                        .is_some()
                    {
                        return Err(l.fehler(&format!("Instanz #{id} doppelt vergeben")));
                    }
                }
                Some(b'E') if daten[l.pos..].starts_with(b"ENDSEC") => break,
                None => return Err(l.fehler("ENDSEC fehlt")),
                _ => return Err(l.fehler("Instanz erwartet")),
            }
        }
        Ok(Self {
            schema,
            system,
            instanzen,
            quelle: String::from_utf8_lossy(daten).into_owned(),
            kopf: (start, data),
            bereiche,
        })
    }

    pub fn get(&self, id: u64) -> Option<&Instanz> {
        self.instanzen.get(&id)
    }
    /// Instanz eines Verweises, wenn sie einen der Typen hat.
    pub fn typ(&self, w: &Wert, typen: &[&str]) -> Option<&Instanz> {
        let i = self.get(w.referenz()?)?;
        typen.contains(&i.typ.as_str()).then_some(i)
    }
    pub fn alle<'a>(&'a self, typ: &'a str) -> impl Iterator<Item = &'a Instanz> + 'a {
        self.instanzen.values().filter(move |i| i.typ == typ)
    }
    pub fn hoechste_id(&self) -> u64 {
        self.instanzen.keys().next_back().copied().unwrap_or(0)
    }

    /// Schreibt die Datei neu: `ersetzt` tauscht Instanzen aus (gleiche ID),
    /// `entfernt` lässt Instanzen weg, `neu` hängt Instanzen an. Alle übrigen
    /// Instanzen und der Kopf bleiben zeichengleich mit der Quelle, außer
    /// `kopf_ersatz` setzt einen neuen Kopf.
    pub fn neu_schreiben(
        &self,
        ersetzt: &BTreeMap<u64, Instanz>,
        entfernt: &std::collections::HashSet<u64>,
        neu: &[Instanz],
        kopf_ersatz: Option<&str>,
    ) -> String {
        let mut out = String::with_capacity(self.quelle.len() + neu.len() * 80);
        match kopf_ersatz {
            Some(k) => out.push_str(k),
            None => out.push_str(&self.quelle[self.kopf.0..self.kopf.1]),
        }
        out.push_str("DATA;\n");
        for (id, a, b) in &self.bereiche {
            if entfernt.contains(id) {
                continue;
            }
            match ersetzt.get(id) {
                Some(i) => out.push_str(&instanz_text(i)),
                None => out.push_str(&self.quelle[*a..*b]),
            }
            out.push('\n');
        }
        for i in neu {
            out.push_str(&instanz_text(i));
            out.push('\n');
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

/// Liest den Parameterwert eines Kopfeintrags wie `FILE_SCHEMA((…));`.
fn kopf_text(kopf: &[u8], name: &[u8]) -> Option<Wert> {
    let p = find(kopf, name)?;
    let mut l = Leser {
        s: kopf,
        pos: p + name.len(),
    };
    l.wert(0).ok()
}

/// Kopf einer neu erzeugten Datei.
pub fn kopf(schema: &str, dateiname: &str, zeit: &str, system: &str, beschreibung: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(({}),'2;1');\nFILE_NAME({},{},(''),(''),'',{},'');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n",
        verschluesseln(beschreibung),
        verschluesseln(dateiname),
        verschluesseln(zeit),
        verschluesseln(system),
    )
}

/// Neue IFC-GlobalId (22 Zeichen, Base64-Alphabet von IFC) aus einer UUID.
pub fn global_id(u: uuid::Uuid) -> String {
    const Z: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let n = u.as_u128();
    let mut out = [0u8; 22];
    // Erstes Zeichen trägt die obersten 2 Bit, danach je 6 Bit.
    out[0] = Z[(n >> 126) as usize & 0x3];
    for (i, c) in out.iter_mut().enumerate().skip(1) {
        let shift = 6 * (21 - i);
        *c = Z[((n >> shift) & 0x3f) as usize];
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');\nFILE_NAME('a.ifc','2026-10-28T12:00:00',(''),(''),'Proc','ArchiCAD 27','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n/* Kommentar */\n#1= IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',$,'Au\\X2\\00DF\\X0\\enwand',$,$,#2,$,$,.NOTDEFINED.);\n#2=IFCLOCALPLACEMENT($,#3);\n#3=IFCAXIS2PLACEMENT3D(#4,$,$);\n#4=IFCCARTESIANPOINT((0.,1.5E3,-2.));\n#5=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);\n#6=IFCSPACE('x',$,'It''s',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);\nENDSEC;\nEND-ISO-10303-21;\n";

    #[test]
    fn liest_instanzen_und_kopf() {
        let d = StepDatei::lesen(BEISPIEL.as_bytes()).unwrap();
        assert_eq!(d.schema, "IFC4");
        assert_eq!(d.system.as_deref(), Some("ArchiCAD 27"));
        assert_eq!(d.instanzen.len(), 6);
        let w = d.get(1).unwrap();
        assert_eq!(w.typ, "IFCWALL");
        assert_eq!(w.attr(2).text(), Some("Außenwand"));
        assert_eq!(w.attr(5).referenz(), Some(2));
        assert_eq!(w.attr(8).enum_wert(), Some("NOTDEFINED"));
        let p = d.get(4).unwrap();
        let k: Vec<f64> = p.attr(0).liste().iter().filter_map(Wert::zahl).collect();
        assert_eq!(k, vec![0., 1500., -2.]);
        assert_eq!(d.get(5).unwrap().attr(2).enum_wert(), Some("T"));
        assert_eq!(d.get(6).unwrap().attr(2).text(), Some("It's"));
    }

    #[test]
    fn schreibt_unveraendert_zurueck_und_ersetzt_gezielt() {
        let d = StepDatei::lesen(BEISPIEL.as_bytes()).unwrap();
        let gleich = d.neu_schreiben(&BTreeMap::new(), &Default::default(), &[], None);
        // Kommentare zwischen Instanzen entfallen; Instanzen bleiben zeichengleich.
        assert!(
            gleich.contains("#1= IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',$,'Au\\X2\\00DF\\X0\\enwand'")
        );
        let mut w = d.get(1).unwrap().clone();
        w.typ = "IFCSLAB".into();
        w.attribute[2] = Wert::Text("Decke über EG".into());
        let mut ersetzt = BTreeMap::new();
        ersetzt.insert(1, w);
        let neu = d.neu_schreiben(&ersetzt, &[6].into_iter().collect(), &[], None);
        let d2 = StepDatei::lesen(neu.as_bytes()).unwrap();
        assert_eq!(d2.get(1).unwrap().typ, "IFCSLAB");
        assert_eq!(d2.get(1).unwrap().attr(2).text(), Some("Decke über EG"));
        assert!(d2.get(6).is_none());
        assert_eq!(d2.get(4), d.get(4));
    }

    #[test]
    fn zahlen_und_texte() {
        assert_eq!(real(1.0), "1.");
        assert_eq!(real(0.25), "0.25");
        assert_eq!(real(-3.0), "-3.");
        assert_eq!(real(1e-7), "1.E-7");
        assert_eq!(real(2.5e-6), "0.0000025");
        assert_eq!(real(1e300), "1.E300");
        assert_eq!(verschluesseln("Küche 'A'"), "'K\\X2\\00FC\\X0\\che ''A'''");
        assert_eq!(entschluesseln(b"K\\X2\\00FC\\X0\\che"), "Küche");
        assert_eq!(entschluesseln(b"Gr\\S\\|n"), "Grün");
        assert_eq!(entschluesseln(b"Stra\\X\\DFe"), "Straße");
        assert_eq!(entschluesseln("Höhe".as_bytes()), "Höhe");
    }

    #[test]
    fn global_ids_sind_22_zeichen_und_eindeutig() {
        let a = global_id(uuid::Uuid::from_u128(0));
        let b = global_id(uuid::Uuid::from_u128(u128::MAX));
        assert_eq!(a, "0000000000000000000000");
        assert_eq!(b.len(), 22);
        assert_eq!(&b[..1], "3");
        assert_ne!(global_id(uuid::Uuid::from_u128(1)), a);
    }

    #[test]
    fn fehler_nennen_zeile() {
        let e =
            StepDatei::lesen(b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=IFCWALL('a',;\nENDSEC;")
                .unwrap_err()
                .to_string();
        assert!(e.contains("Zeile 5"), "{e}");
        assert!(StepDatei::lesen(b"<ifc/>").is_err());
    }
}
