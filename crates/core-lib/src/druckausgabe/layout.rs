//! A4-Satz mit Tabellen, fortgesetzten Zeilen und wiederholten Spaltenköpfen.
use super::{berichte::Zeile, pdf::umbruch, *};
use crate::formular::schrift;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
const LINKS: f32 = 48.;
const BREITE: f32 = 499.;
const UNTEN: f32 = 75.;
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
    fn linie(&mut self, x: f32, y: f32, w: f32) {
        self.ops.extend([
            Operation::new("q", vec![]),
            Operation::new("G", vec![0.7.into()]),
            Operation::new("w", vec![0.4.into()]),
            Operation::new("m", vec![x.into(), y.into()]),
            Operation::new("l", vec![(x + w).into(), y.into()]),
            Operation::new("S", vec![]),
            Operation::new("Q", vec![]),
        ]);
    }
}
struct Satz {
    seiten: Vec<Seite>,
    y: f32,
    oben: f32,
    titel: Vec<String>,
}
impl Satz {
    fn neu(titel: &str) -> CoreResult<Self> {
        let titel = umbruch(titel, BREITE, 18.)?;
        if titel.len() > 6 {
            return Err(fehler("Berichtstitel benötigt zu viele Zeilen"));
        }
        let mut s = Self {
            seiten: Vec::new(),
            y: 0.,
            oben: 0.,
            titel,
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
        for t in &self.titel {
            page.text(LINKS, y, BREITE, 18., t);
            y -= 25.;
        }
        page.linie(LINKS, y - 5., BREITE);
        self.oben = y - 24.;
        self.y = self.oben;
        self.seiten.push(page);
        Ok(())
    }
    fn page(&mut self) -> &mut Seite {
        self.seiten.last_mut().unwrap()
    }
    fn absatz(&mut self, t: &str, titel: bool) -> CoreResult<()> {
        let size = if titel { 11. } else { 9.5 };
        let lines = umbruch(t, BREITE - 12., size)?;
        let min = if titel { 50. } else { 16. };
        if self.y - min < UNTEN {
            self.seite()?;
        }
        if titel {
            self.y -= 9.;
        }
        for line in lines {
            if self.y - 16. < UNTEN {
                self.seite()?;
            }
            let y = self.y;
            if titel {
                self.page().rechteck(LINKS, y - 5., BREITE, 21., 0.92);
            }
            self.page().text(LINKS + 6., y, BREITE - 12., size, &line);
            self.y -= 16.;
        }
        self.y -= 6.;
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
                .rechteck(LINKS, top - h, BREITE, h, if kopf { 0.87 } else { 0.97 });
        }
        for (cell, w) in cells.iter().zip(widths) {
            for (i, t) in cell.iter().skip(start).take(count).enumerate() {
                self.page()
                    .text(x + 6., top - 15. - i as f32 * 12.5, *w - 12., 8.5, t);
            }
            x += w;
        }
        self.page().linie(LINKS, top - h, BREITE);
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
        if rows.is_empty() {
            self.absatz("Keine Positionen erfasst.", false)?;
        }
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
        self.y -= 20.;
        Ok(())
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
pub(super) fn bericht(_projekt: &str, titel: &str, zeilen: &[Zeile]) -> CoreResult<Document> {
    let mut s = Satz::neu(titel)?;

    for z in zeilen {
        match z {
            Zeile::Seitenwechsel => s.seite()?,
            Zeile::Text { text, titel } => s.absatz(text, *titel)?,
            Zeile::Tabelle {
                koepfe,
                breiten,
                zeilen,
            } => s.tabelle(koepfe, breiten, zeilen)?,
        }
    }
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
        "Seite von 0123456789".into(),
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
        page.text(450., 31., 97., 8., &format!("Seite {n} von {count}"));
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
    #[test]
    fn tabellen_wiederholen_koepfe_und_erhalten_lange_zellen() {
        let mut s=Satz::neu("Prüfbericht mit einem bewusst langen Titel, der vollständig und ohne Abschneiden über mehrere Zeilen erscheinen muss").unwrap();
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
    #[test]
    fn ungueltige_tabellen_werden_nicht_still_falsch_ausgegeben() {
        let mut s = Satz::neu("Kontrolle").unwrap();
        assert!(s.tabelle(&["A".into()], &[0.], &[]).is_err());
        assert!(s.tabelle(&["A".into()], &[1.], &[vec![]]).is_err());
    }
}
