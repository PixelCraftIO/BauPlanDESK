//! A4-Satz für Behördenausgaben: Kopfblock mit Anlage und Blattzählung,
//! nummerierte Abschnitte, Tabellen mit wiederholten Spaltenköpfen, Hinweiskästen
//! und Aufstellungsvermerk mit Unterschriftsfeldern.
use super::{berichte::Zeile, pdf::umbruch, *};
use crate::formular::schrift;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
const LINKS: f32 = 48.;
const BREITE: f32 = 499.;
const UNTEN: f32 = 75.;
/// Kopfblock unterhalb des Briefbogens.
const KOPF_OBEN: f32 = 768.;
const KOPF_HOEHE: f32 = 54.;
const KOPF_RECHTS: f32 = 122.;

/// Kennzeichnung jedes erzeugten Blatts: Wozu gehört es, welcher Stand,
/// welche Anlage? So bleibt jedes Blatt auch einzeln zuordenbar.
#[derive(Debug, Clone, Default)]
pub(super) struct Blattkopf {
    pub bauvorhaben: String,
    pub gegenstand: String,
    pub stand: String,
    /// Anlagennummer im Paket; ohne Nummer steht `art` im rechten Feld.
    pub anlage: Option<String>,
    pub art: String,
    /// Weitere Blätter derselben Anlage, die außerhalb des Satzes entstehen
    /// (z. B. die Vektorplanskizze); sie zählen in „Blatt k von m“ mit.
    pub zusatzblaetter: usize,
}

#[derive(Clone)]
struct Zelle {
    x: f32,
    y: f32,
    breite: f32,
    size: f32,
    text: String,
}
#[derive(Default)]
struct Seite {
    texte: Vec<Zelle>,
    ops: Vec<Operation>,
}
impl Seite {
    fn text(&mut self, x: f32, y: f32, breite: f32, size: f32, text: &str) {
        self.texte.push(Zelle {
            x,
            y,
            breite,
            size,
            text: text.into(),
        });
    }
    fn rechteck(&mut self, x: f32, y: f32, w: f32, h: f32, grau: f32) {
        self.ops.extend([
            Operation::new("q", vec![]),
            Operation::new("g", vec![grau.into()]),
            Operation::new("re", vec![x.into(), y.into(), w.into(), h.into()]),
            Operation::new("f", vec![]),
            Operation::new("Q", vec![]),
        ]);
    }
    fn rahmen(&mut self, x: f32, y: f32, w: f32, h: f32, grau: f32, staerke: f32) {
        self.ops.extend([
            Operation::new("q", vec![]),
            Operation::new("G", vec![grau.into()]),
            Operation::new("w", vec![staerke.into()]),
            Operation::new("re", vec![x.into(), y.into(), w.into(), h.into()]),
            Operation::new("S", vec![]),
            Operation::new("Q", vec![]),
        ]);
    }
    fn strich(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, grau: f32, staerke: f32) {
        self.ops.extend([
            Operation::new("q", vec![]),
            Operation::new("G", vec![grau.into()]),
            Operation::new("w", vec![staerke.into()]),
            Operation::new("m", vec![x1.into(), y1.into()]),
            Operation::new("l", vec![x2.into(), y2.into()]),
            Operation::new("S", vec![]),
            Operation::new("Q", vec![]),
        ]);
    }
    fn linie(&mut self, x: f32, y: f32, w: f32) {
        self.strich(x, y, x + w, y, 0.7, 0.4);
    }
}
/// Eine Zeile, bei Überlänge mit Auslassungszeichen; die volle Angabe steht im Bericht.
fn einzeilig(t: &str, breite: f32, size: f32) -> CoreResult<String> {
    let zeilen = umbruch(t, breite, size)?;
    Ok(match zeilen.as_slice() {
        [] => String::new(),
        [eine] => eine.clone(),
        [erste, ..] => {
            let mut s = erste.trim_end().to_string();
            while !s.is_empty() && umbruch(&format!("{s} …"), breite, size)?.len() > 1 {
                s.pop();
            }
            format!("{} …", s.trim_end())
        }
    })
}
struct Satz {
    seiten: Vec<Seite>,
    y: f32,
    oben: f32,
    titel: Vec<String>,
    kopf: Option<(Blattkopf, [String; 3])>,
    abschnitt: u32,
    unterabschnitt: u32,
}
impl Satz {
    fn neu(titel: &str, kopf: Option<&Blattkopf>) -> CoreResult<Self> {
        let titel = umbruch(titel, BREITE, 16.)?;
        if titel.len() > 6 {
            return Err(fehler("Berichtstitel benötigt zu viele Zeilen"));
        }
        let breite = BREITE - KOPF_RECHTS - 16.;
        let kopf = kopf
            .map(|k| -> CoreResult<_> {
                Ok((
                    k.clone(),
                    [
                        einzeilig(&format!("Bauvorhaben: {}", k.bauvorhaben), breite, 8.)?,
                        einzeilig(&format!("Gegenstand: {}", k.gegenstand), breite, 8.)?,
                        einzeilig(&format!("Stand: {}", k.stand), breite, 8.)?,
                    ],
                ))
            })
            .transpose()?;
        let mut s = Self {
            seiten: Vec::new(),
            y: 0.,
            oben: 0.,
            titel,
            kopf,
            abschnitt: 0,
            unterabschnitt: 0,
        };
        s.seite()?;
        Ok(s)
    }
    fn seite(&mut self) -> CoreResult<()> {
        if self.seiten.len() >= 300 {
            return Err(fehler("Bericht über 300 Seiten"));
        }
        let mut page = Seite::default();
        let mut y = 735.;
        if let Some((k, zeilen)) = &self.kopf {
            let unten = KOPF_OBEN - KOPF_HOEHE;
            let teiler = LINKS + BREITE - KOPF_RECHTS;
            page.rahmen(LINKS, unten, BREITE, KOPF_HOEHE, 0.25, 0.8);
            page.strich(teiler, unten, teiler, KOPF_OBEN, 0.25, 0.8);
            page.rechteck(teiler, unten, KOPF_RECHTS, KOPF_HOEHE, 0.94);
            for (i, z) in zeilen.iter().enumerate() {
                page.text(
                    LINKS + 8.,
                    KOPF_OBEN - 16. - i as f32 * 14.,
                    BREITE - KOPF_RECHTS - 16.,
                    8.,
                    z,
                );
            }
            match &k.anlage {
                Some(a) => {
                    page.text(
                        teiler + 8.,
                        KOPF_OBEN - 14.,
                        KOPF_RECHTS - 16.,
                        7.,
                        "Anlage",
                    );
                    page.text(teiler + 8., KOPF_OBEN - 36., KOPF_RECHTS - 16., 15., a);
                }
                None => page.text(teiler + 8., KOPF_OBEN - 26., KOPF_RECHTS - 16., 9.5, &k.art),
            }
            y = unten - 30.;
        }
        // Mit Kopfblock steht der Titel nur auf dem ersten Blatt.
        if self.kopf.is_none() || self.seiten.is_empty() {
            for t in &self.titel {
                page.text(LINKS, y, BREITE, 16., t);
                y -= 22.;
            }
            page.strich(LINKS, y + 2., LINKS + BREITE, y + 2., 0.25, 1.);
            y -= 22.;
        } else {
            y -= 4.;
        }
        self.oben = y;
        self.y = y;
        self.seiten.push(page);
        Ok(())
    }
    fn page(&mut self) -> &mut Seite {
        self.seiten.last_mut().unwrap()
    }
    fn absatz(&mut self, t: &str) -> CoreResult<()> {
        let lines = umbruch(t, BREITE - 12., 9.5)?;
        if self.y - 16. < UNTEN {
            self.seite()?;
        }
        for line in lines {
            if self.y - 16. < UNTEN {
                self.seite()?;
            }
            let y = self.y;
            self.page().text(LINKS + 6., y, BREITE - 12., 9.5, &line);
            self.y -= 15.;
        }
        self.y -= 5.;
        Ok(())
    }
    /// Überschrift mit Bandfläche; hält mindestens `min` Punkte Folgeinhalt zusammen.
    fn ueberschrift(&mut self, t: &str, size: f32, grau: f32, min: f32) -> CoreResult<()> {
        let lines = umbruch(t, BREITE - 12., size)?;
        let schritt = size + 8.;
        let h = (lines.len() as f32 - 1.) * schritt + size + 10.;
        if self.y - h - min < UNTEN {
            self.seite()?;
        }
        self.y -= 6.;
        // Band symmetrisch um die Schrift: Oberkante knapp über der Versalhöhe,
        // Unterkante knapp unter der letzten Grundlinie.
        let top = self.y + size + 6.;
        self.page().rechteck(LINKS, top - h, BREITE, h, grau);
        for line in lines {
            let y = self.y;
            self.page().text(LINKS + 6., y, BREITE - 12., size, &line);
            self.y -= schritt;
        }
        self.y -= 8.;
        Ok(())
    }
    fn abschnitt(&mut self, t: &str) -> CoreResult<()> {
        self.abschnitt += 1;
        self.unterabschnitt = 0;
        let t = format!("{}  {t}", self.abschnitt);
        // Hauptabschnitt mit Unterabschnitt und Tabellenanfang zusammenhalten.
        self.ueberschrift(&t, 12., 0.82, 120.)
    }
    fn unterabschnitt(&mut self, t: &str) -> CoreResult<()> {
        let t = if self.abschnitt > 0 {
            self.unterabschnitt += 1;
            format!("{}.{}  {t}", self.abschnitt, self.unterabschnitt)
        } else {
            t.into()
        };
        // Überschrift nie allein am Blattende: Tabellenkopf und erste Zeile folgen.
        self.ueberschrift(&t, 10.5, 0.92, 85.)
    }
    /// Hinweiskasten mit Randbalken für Status, Abweichungen und Vorbehalte.
    fn hinweis(&mut self, t: &str) -> CoreResult<()> {
        let lines = umbruch(t, BREITE - 28., 9.5)?;
        let max = ((self.oben - UNTEN - 20.) / 15.).floor().max(1.) as usize;
        for teil in lines.chunks(max) {
            let h = teil.len() as f32 * 15. + 12.;
            if self.y - h < UNTEN {
                self.seite()?;
            }
            let top = self.y + 13.;
            self.page().rechteck(LINKS, top - h, BREITE, h, 0.96);
            self.page().rechteck(LINKS, top - h, 3.5, h, 0.2);
            self.page().rahmen(LINKS, top - h, BREITE, h, 0.6, 0.4);
            for line in teil {
                let y = self.y;
                self.page().text(LINKS + 16., y, BREITE - 28., 9.5, line);
                self.y -= 15.;
            }
            self.y -= 16.;
        }
        Ok(())
    }
    /// Unterschriftsfelder je Rolle: Ort/Datum und Unterschrift mit Namensangabe.
    fn unterschrift(&mut self, felder: &[(String, String)]) -> CoreResult<()> {
        let spalte = (BREITE - 24.) / 2.;
        for paar in felder.chunks(2) {
            let h = 92.;
            if self.y - h < UNTEN {
                self.seite()?;
            }
            let top = self.y;
            for (i, (rolle, name)) in paar.iter().enumerate() {
                let x = LINKS + i as f32 * (spalte + 24.);
                let rolle = einzeilig(rolle, spalte, 8.5)?;
                self.page().text(x, top - 4., spalte, 8.5, &rolle);
                let linie = top - 46.;
                let datum = spalte * 0.38;
                self.page().strich(x, linie, x + datum, linie, 0.2, 0.6);
                self.page()
                    .strich(x + datum + 12., linie, x + spalte, linie, 0.2, 0.6);
                self.page().text(x, linie - 13., datum, 7.5, "Ort, Datum");
                self.page().text(
                    x + datum + 12.,
                    linie - 13.,
                    spalte - datum - 12.,
                    7.5,
                    "Unterschrift",
                );
                if !name.trim().is_empty() {
                    let name = einzeilig(name, spalte - datum - 12., 7.5)?;
                    self.page().text(
                        x + datum + 12.,
                        linie - 25.,
                        spalte - datum - 12.,
                        7.5,
                        &name,
                    );
                }
            }
            self.y -= h;
        }
        Ok(())
    }
    fn zeile(
        &mut self,
        cells: &[Vec<String>],
        widths: &[f32],
        start: usize,
        count: usize,
        kopf: bool,
        schattiert: bool,
    ) {
        let h = count as f32 * 12.5 + 12.;
        let top = self.y;
        let mut x = LINKS;
        if kopf || schattiert {
            self.page()
                .rechteck(LINKS, top - h, BREITE, h, if kopf { 0.86 } else { 0.97 });
        }
        for (cell, w) in cells.iter().zip(widths) {
            for (i, t) in cell.iter().skip(start).take(count).enumerate() {
                self.page()
                    .text(x + 6., top - 15. - i as f32 * 12.5, *w - 12., 8.5, t);
            }
            x += w;
        }
        if kopf {
            self.page()
                .strich(LINKS, top, LINKS + BREITE, top, 0.25, 0.8);
            self.page()
                .strich(LINKS, top - h, LINKS + BREITE, top - h, 0.25, 0.8);
        } else {
            self.page().linie(LINKS, top - h, BREITE);
        }
        self.y -= h;
    }
    fn tabelle(
        &mut self,
        koepfe: &[String],
        breiten: &[f32],
        rows: &[Vec<String>],
    ) -> CoreResult<()> {
        if koepfe.is_empty()
            || koepfe.len() != breiten.len()
            || breiten.iter().any(|w| !w.is_finite() || *w <= 0.)
            || rows.iter().any(|r| r.len() != koepfe.len())
        {
            return Err(fehler("Ungültiger Tabellenvertrag"));
        }
        if rows.is_empty() {
            return self.absatz("Keine Positionen erfasst.");
        }
        let sum: f32 = breiten.iter().sum();
        let widths: Vec<f32> = breiten.iter().map(|w| w / sum * BREITE).collect();
        let wrap = |r: &[String]| -> CoreResult<Vec<Vec<String>>> {
            r.iter()
                .zip(&widths)
                .map(|(t, w)| umbruch(t, *w - 12., 8.5))
                .collect()
        };
        let header = wrap(koepfe)?;
        let hn = header.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let hh = hn as f32 * 12.5 + 12.;
        if hh > self.oben - UNTEN - 50. {
            return Err(fehler("Tabellenkopf zu umfangreich"));
        }
        if self.y - hh - 37. < UNTEN {
            self.seite()?;
        }
        self.zeile(&header, &widths, 0, hn, true, false);
        for (i, row) in rows.iter().enumerate() {
            let cells = wrap(row)?;
            let n = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let mut start = 0;
            // Normale Zeilen zusammenhalten; überhohe Zeilen spaltenweise fortsetzen.
            let full = n as f32 * 12.5 + 12.;
            if full <= self.oben - UNTEN - hh && self.y - full < UNTEN {
                self.seite()?;
                self.zeile(&header, &widths, 0, hn, true, false);
            }
            while start < n {
                let fit = ((self.y - UNTEN - 12.) / 12.5).floor().max(0.) as usize;
                if fit == 0 {
                    self.seite()?;
                    self.zeile(&header, &widths, 0, hn, true, false);
                    continue;
                }
                let count = fit.min(n - start);
                self.zeile(&cells, &widths, start, count, false, i % 2 == 1);
                start += count;
                if start < n {
                    self.seite()?;
                    self.zeile(&header, &widths, 0, hn, true, false);
                }
            }
        }
        self.y -= 18.;
        Ok(())
    }
    /// Blattzählung je Anlage, erst nach dem Umbruch bekannt.
    fn blattzaehlung(&mut self) {
        if self.kopf.is_none() {
            return;
        }
        let m = self.seiten.len()
            + self
                .kopf
                .as_ref()
                .map(|(k, _)| k.zusatzblaetter)
                .unwrap_or(0);
        let x = LINKS + BREITE - KOPF_RECHTS + 8.;
        for (k, page) in self.seiten.iter_mut().enumerate() {
            page.text(
                x,
                KOPF_OBEN - KOPF_HOEHE + 6.,
                KOPF_RECHTS - 16.,
                8.,
                &format!("Blatt {} von {m}", k + 1),
            );
        }
    }
}
fn zeichnen(d: &mut Document, page: Seite, font: ObjectId) -> CoreResult<(Dictionary, Vec<u8>)> {
    let mut x = Dictionary::new();
    let mut ops = page.ops;
    for (i, z) in page.texte.iter().enumerate() {
        let id = schrift::darstellung(d, &z.text, z.breite, z.size * 1.5, z.size, font, false)?;
        let name = format!("Satz{i}");
        x.set(name.clone(), id);
        ops.extend([
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![
                    1.into(),
                    0.into(),
                    0.into(),
                    1.into(),
                    z.x.into(),
                    z.y.into(),
                ],
            ),
            Operation::new("Do", vec![Object::Name(name.into_bytes())]),
            Operation::new("Q", vec![]),
        ]);
    }
    Ok((x, Content { operations: ops }.encode().map_err(fehler)?))
}
pub(super) fn bericht(
    titel: &str,
    kopf: Option<&Blattkopf>,
    zeilen: &[Zeile],
) -> CoreResult<Document> {
    let mut s = Satz::neu(titel, kopf)?;
    for z in zeilen {
        match z {
            Zeile::Seitenwechsel => s.seite()?,
            Zeile::Text { text, titel: false } => s.absatz(text)?,
            Zeile::Text { text, titel: true } => s.unterabschnitt(text)?,
            Zeile::Abschnitt(t) => s.abschnitt(t)?,
            Zeile::Hinweis(t) => s.hinweis(t)?,
            Zeile::Unterschrift(felder) => s.unterschrift(felder)?,
            Zeile::Tabelle {
                koepfe,
                breiten,
                zeilen,
            } => s.tabelle(koepfe, breiten, zeilen)?,
        }
    }
    s.blattzaehlung();
    let mut d = Document::with_version("1.7");
    let root = d.new_object_id();
    let texts: Vec<&str> = s
        .seiten
        .iter()
        .flat_map(|p| p.texte.iter().map(|t| t.text.as_str()))
        .collect();
    let font = schrift::einbetten(&mut d, &texts)?;
    let mut kids = Vec::new();
    for page in s.seiten {
        let (x, b) = zeichnen(&mut d, page, font)?;
        let c = d.add_object(Stream::new(dictionary! {}, b));
        let p=d.add_object(dictionary!{"Type"=>"Page","Parent"=>root,"MediaBox"=>vec![0.into(),0.into(),595.276.into(),841.89.into()],"Resources"=>dictionary!{"XObject"=>x},"Contents"=>c,"BauBericht"=>true});
        kids.push(p.into());
    }
    let count = kids.len() as i64;
    d.objects.insert(
        root,
        dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count}.into(),
    );
    let cat = d.add_object(dictionary! {"Type"=>"Catalog","Pages"=>root});
    d.trailer.set("Root", cat);
    Ok(d)
}
/// Nur selbst erzeugte Blätter erhalten Briefbogen und Paket-Seitenzahlen.
/// Originalformulare und maßstäbliche Fremdpläne werden niemals skaliert/überdruckt.
pub(super) fn briefbogen(
    d: &mut Document,
    projekt: &str,
    a: &Druckauftrag,
    datum: chrono::NaiveDate,
) -> CoreResult<()> {
    let g = a.gestaltung.clone().unwrap_or_default();
    let count = d.get_pages().len();
    let all = [
        g.kopfzeile.replace('\t', "    "),
        g.fusszeile.replace('\t', "    "),
        projekt.into(),
        datum.format("%d.%m.%Y").to_string(),
        "Seite von 0123456789 im Paket".into(),
    ];
    let texts: Vec<&str> = all.iter().map(String::as_str).collect();
    let font = schrift::einbetten(d, &texts)?;
    for (n, id) in d.get_pages() {
        if !d
            .get_dictionary(id)
            .map_err(fehler)?
            .get(b"BauBericht")
            .ok()
            .and_then(|o| o.as_bool().ok())
            .unwrap_or(false)
        {
            continue;
        }
        let ersetzen = |t: &str| {
            t.replace("{Projekt}", projekt)
                .replace("{Datum}", &datum.format("%d.%m.%Y").to_string())
                .replace("{Seite}", &n.to_string())
                .replace("{Seiten}", &count.to_string())
        };
        let head = umbruch(&ersetzen(&g.kopfzeile), BREITE, 9.)?;
        let foot = umbruch(&ersetzen(&g.fusszeile), BREITE - 105., 8.)?;
        if head.len() > 3 || foot.len() > 3 {
            return Err(fehler(
                "Kopf-/Fußzeile benötigt mehr als drei Zeilen. Bitte Text kürzen.",
            ));
        }
        let mut page = Seite::default();
        for (i, t) in head.iter().enumerate() {
            page.text(LINKS, 808. - i as f32 * 12., BREITE, 9., t);
        }
        page.linie(LINKS, 774., BREITE);
        for (i, t) in foot.iter().enumerate() {
            page.text(LINKS, 51. - i as f32 * 10., BREITE - 105., 8., t);
        }
        page.linie(LINKS, 64., BREITE);
        page.text(
            420.,
            31.,
            127.,
            8.,
            &format!("Seite {n} von {count} im Paket"),
        );
        let (x, b) = zeichnen(d, page, font)?;
        let overlay = d.add_object(Stream::new(dictionary! {}, b));
        let old = d.get_dictionary(id).map_err(fehler)?.clone();
        let mut resources = old
            .get(b"Resources")
            .ok()
            .and_then(|o| d.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        let mut forms = resources
            .get(b"XObject")
            .ok()
            .and_then(|o| d.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok())
            .cloned()
            .unwrap_or_default();
        // Namespacing avoids colliding with content resources.
        let mut content = Content::decode(
            &d.get_object(overlay)
                .map_err(fehler)?
                .as_stream()
                .map_err(fehler)?
                .content,
        )
        .map_err(fehler)?;
        for (name, obj) in x.iter() {
            let mut pref = b"Briefbogen".to_vec();
            pref.extend(name);
            forms.set(pref, obj.clone());
        }
        for op in &mut content.operations {
            if op.operator == "Do"
                && let Some(Object::Name(n)) = op.operands.first_mut()
            {
                let mut p = b"Briefbogen".to_vec();
                p.extend(n.clone());
                *n = p;
            }
        }
        d.objects.insert(
            overlay,
            Object::Stream(Stream::new(
                dictionary! {},
                content.encode().map_err(fehler)?,
            )),
        );
        resources.set("XObject", forms);
        let mut contents = match old.get(b"Contents").ok() {
            Some(Object::Array(a)) => a.clone(),
            Some(o) => vec![o.clone()],
            None => vec![],
        };
        contents.push(overlay.into());
        let p = d.get_dictionary_mut(id).map_err(fehler)?;
        p.set("Resources", resources);
        p.set("Contents", contents);
        p.remove(b"BauBericht");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn kopf() -> Blattkopf {
        Blattkopf {
            bauvorhaben: "Neubau Musterstraße".into(),
            gegenstand: "GRZ-Nachweis mit einer sehr langen Bezeichnung, die im Kopfblock gekürzt werden muss, weil sie nicht in eine Zeile passt".into(),
            stand: "Nachweis vom 16.10.2026".into(),
            anlage: Some("3".into()),
            art: String::new(),
            zusatzblaetter: 0,
        }
    }
    #[test]
    fn tabellen_wiederholen_koepfe_und_erhalten_lange_zellen() {
        for k in [None, Some(kopf())] {
            let mut s=Satz::neu("Prüfbericht mit einem bewusst langen Titel, der vollständig und ohne Abschneiden über mehrere Zeilen erscheinen muss", k.as_ref()).unwrap();
            let header = vec!["Position".into(), "Beschreibung".into(), "Fläche m²".into()];
            let mut rows = (0..120)
                .map(|i| vec![i.to_string(), format!("Raum Müller {i}"), "123,45".into()])
                .collect::<Vec<_>>();
            rows.insert(
                5,
                vec![
                    "Langzeile".into(),
                    "Vollständige Begründung mit Umlauten äöü. ".repeat(400),
                    "9".into(),
                ],
            );
            s.tabelle(&header, &[1., 3., 1.], &rows).unwrap();
            assert!(s.seiten.len() > 6);
            for page in &s.seiten {
                assert!(
                    page.texte.iter().any(|t| t.text == "Position"),
                    "Spaltenkopf fehlt auf Fortsetzungsseite"
                );
                assert!(
                    page.texte
                        .iter()
                        .all(|t| t.y >= UNTEN && t.y + t.size * 1.5 < 774.),
                    "Satz gerät in den Briefbogen"
                );
            }
            let all = s
                .seiten
                .iter()
                .flat_map(|s| s.texte.iter())
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(all.matches("Vollständige").count(), 400);
            assert!(all.contains("Raum Müller 119"));
        }
    }
    #[test]
    fn kopfblock_anlage_blattzaehlung_und_gliederung() {
        let mut z = vec![Zeile::Abschnitt("Grundlagen".into())];
        z.extend((0..80).map(|i| super::super::berichte::text(format!("Absatz {i}"))));
        z.push(super::super::berichte::titel("Teilergebnis"));
        z.push(Zeile::Hinweis("Nachweis veraltet".into()));
        z.push(Zeile::Unterschrift(vec![(
            "Entwurfsverfasser/in".into(),
            "Anna Weber, Architektin".into(),
        )]));
        let mut s = Satz::neu("GRZ-Nachweis", Some(&kopf())).unwrap();
        for zeile in &z {
            match zeile {
                Zeile::Abschnitt(t) => s.abschnitt(t).unwrap(),
                Zeile::Text { text, titel: true } => s.unterabschnitt(text).unwrap(),
                Zeile::Text { text, .. } => s.absatz(text).unwrap(),
                Zeile::Hinweis(t) => s.hinweis(t).unwrap(),
                Zeile::Unterschrift(f) => s.unterschrift(f).unwrap(),
                _ => {}
            }
        }
        s.blattzaehlung();
        let m = s.seiten.len();
        assert!(m > 1);
        for (k, page) in s.seiten.iter().enumerate() {
            let texte: Vec<&str> = page.texte.iter().map(|t| t.text.as_str()).collect();
            assert!(texte.contains(&"3"), "Anlagennummer auf jedem Blatt");
            assert!(texte.contains(&format!("Blatt {} von {m}", k + 1).as_str()));
            assert!(
                texte
                    .iter()
                    .any(|t| t.starts_with("Gegenstand: GRZ-Nachweis") && t.ends_with('…'))
            );
            // Titel nur auf dem ersten Blatt.
            assert_eq!(texte.contains(&"GRZ-Nachweis"), k == 0);
            assert!(page.texte.iter().all(|t| t.y >= UNTEN - 30.));
        }
        let alle: Vec<&str> = s
            .seiten
            .iter()
            .flat_map(|p| p.texte.iter().map(|t| t.text.as_str()))
            .collect();
        assert!(alle.contains(&"1  Grundlagen"));
        assert!(alle.contains(&"1.1  Teilergebnis"));
        assert!(alle.contains(&"Unterschrift") && alle.contains(&"Ort, Datum"));
    }
    #[test]
    fn ungueltige_tabellen_werden_nicht_still_falsch_ausgegeben() {
        let mut s = Satz::neu("Kontrolle", None).unwrap();
        assert!(s.tabelle(&["A".into()], &[0.], &[]).is_err());
        assert!(s.tabelle(&["A".into()], &[1.], &[vec![]]).is_err());
    }
}
