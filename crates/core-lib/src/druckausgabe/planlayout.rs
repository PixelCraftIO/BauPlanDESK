//! Planblatt und Plankopf als PDF: maßstäbliche Ausgabe eigener Zeichnungen
//! auf DIN-Blätter (Rahmen, Plankopf, Nordpfeil, Maßstabsleiste, Legende) und
//! Plankopf-Stempel auf vorhandenen PDF-Plänen. Gezeichnet wird in
//! Seitenpunkten mit Ursprung links oben und y nach unten; die Abbildung in
//! PDF-Koordinaten übernimmt die Seitenmatrix.
use super::*;
use crate::formular::schrift;
use lopdf::{
    Dictionary, Document, Object, ObjectId, Stream,
    content::{Content, Operation},
    dictionary,
};

/// Punkte je Millimeter.
pub const PT_MM: f64 = 72. / 25.4;

#[derive(Debug, Clone, PartialEq)]
pub struct Stift {
    pub breite_mm: f64,
    /// RGB 0 … 1.
    pub farbe: [f64; 3],
    /// Strichmuster in mm; leer = Volllinie.
    pub muster: Vec<f64>,
}
impl Default for Stift {
    fn default() -> Self {
        Self {
            breite_mm: 0.25,
            farbe: [0.1, 0.1, 0.1],
            muster: vec![],
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Fuellung {
    Keine,
    /// Flächig mit Deckkraft der Stiftfarbe (0 … 1).
    Voll(f64),
    /// Schraffurlinien unter `winkel_grad` im Abstand `abstand_mm`, optional gekreuzt.
    Linien {
        winkel_grad: f64,
        abstand_mm: f64,
        kreuz: bool,
    },
}
/// Linienzug in Blattmetern (y nach unten, wie im Zeicheneditor).
#[derive(Debug, Clone, PartialEq)]
pub struct Planlinie {
    /// Zeichenebene (für DXF-Layer); im PDF ohne Bedeutung.
    pub ebene: String,
    pub punkte: Vec<[f64; 2]>,
    pub geschlossen: bool,
    pub stift: Stift,
    pub fuellung: Fuellung,
}
/// Text in Blattmetern; Höhe in mm auf dem Papier.
#[derive(Debug, Clone, PartialEq)]
pub struct Plantext {
    pub ebene: String,
    pub punkt: [f64; 2],
    pub text: String,
    pub hoehe_mm: f64,
    /// Gegen den Uhrzeigersinn.
    pub drehung_grad: f64,
    pub farbe: [f64; 3],
}
/// Felder des Plankopfs; leere Felder bleiben zum Ausfüllen frei.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Plankopf {
    pub bauherr: String,
    pub vorhaben: String,
    pub grundstueck: String,
    pub planinhalt: String,
    pub planverfasser: String,
    pub plannummer: String,
    pub index: String,
    pub massstab: String,
    pub datum: String,
    pub format: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Legendeneintrag {
    pub name: String,
    pub stift: Stift,
    pub fuellung: Fuellung,
}
pub struct Planblatt {
    pub breite_mm: f64,
    pub hoehe_mm: f64,
    /// Maßstabszahl (100 = 1 : 100).
    pub massstab: f64,
    pub linien: Vec<Planlinie>,
    pub texte: Vec<Plantext>,
    pub kopf: Plankopf,
    /// Richtung Norden in Grad gegen den Uhrzeigersinn von „oben“; `None` = kein Nordpfeil.
    pub nordwinkel_grad: Option<f64>,
    pub legende: Vec<Legendeneintrag>,
}

/// Rand nach DIN 824: 20 mm Heftrand links, sonst 7 mm.
const RAND_LINKS: f64 = 20.;
const RAND: f64 = 7.;
const KOPF_B: f64 = 180.;
const KOPF_H: f64 = 62.;

/// Zeichenbefehle mit Textbausteinen (Form-XObjects mit eingebetteter Schrift).
struct Zeichner<'a> {
    d: &'a mut Document,
    font: ObjectId,
    ops: Vec<Operation>,
    xobjects: Dictionary,
    n: usize,
}
fn r(v: f64) -> Object {
    Object::Real(v as f32)
}
impl Zeichner<'_> {
    fn op(&mut self, o: &str, a: Vec<Object>) {
        self.ops.push(Operation::new(o, a));
    }
    fn stift(&mut self, s: &Stift) {
        self.op("w", vec![r(s.breite_mm * PT_MM)]);
        self.op("RG", s.farbe.iter().map(|v| r(*v)).collect());
        self.op(
            "d",
            vec![
                Object::Array(s.muster.iter().map(|m| r(m * PT_MM)).collect()),
                0.into(),
            ],
        );
    }
    fn pfad(&mut self, ps: &[[f64; 2]], zu: bool) {
        if let Some(p) = ps.first() {
            self.op("m", vec![r(p[0]), r(p[1])]);
            for p in &ps[1..] {
                self.op("l", vec![r(p[0]), r(p[1])]);
            }
            if zu {
                self.op("h", vec![]);
            }
        }
    }
    fn linie(&mut self, ps: &[[f64; 2]], zu: bool, s: &Stift, f: &Fuellung) {
        if ps.len() < 2 {
            return;
        }
        if zu {
            self.fuellen(ps, s, f);
        }
        self.stift(s);
        self.pfad(ps, zu);
        self.op("S", vec![]);
    }
    fn fuellen(&mut self, ps: &[[f64; 2]], s: &Stift, f: &Fuellung) {
        match f {
            Fuellung::Keine => {}
            Fuellung::Voll(a) => {
                let c = s.farbe.map(|v| v + (1. - v) * (1. - a.clamp(0., 1.)));
                self.op("q", vec![]);
                self.op("rg", c.iter().map(|v| r(*v)).collect());
                self.pfad(ps, true);
                self.op("f", vec![]);
                self.op("Q", vec![]);
            }
            Fuellung::Linien {
                winkel_grad,
                abstand_mm,
                kreuz,
            } => {
                let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                for p in ps {
                    x0 = x0.min(p[0]);
                    y0 = y0.min(p[1]);
                    x1 = x1.max(p[0]);
                    y1 = y1.max(p[1]);
                }
                let a = (abstand_mm * PT_MM).max(0.5);
                let diag = (x1 - x0).hypot(y1 - y0);
                if diag / a > 4000. {
                    return;
                }
                self.op("q", vec![]);
                self.pfad(ps, true);
                self.op("W", vec![]);
                self.op("n", vec![]);
                self.stift(&Stift {
                    breite_mm: 0.13,
                    farbe: s.farbe,
                    muster: vec![],
                });
                let m = [(x0 + x1) / 2., (y0 + y1) / 2.];
                let winkel: &[f64] = if *kreuz {
                    &[*winkel_grad, winkel_grad + 90.]
                } else {
                    std::slice::from_ref(winkel_grad)
                };
                for w in winkel {
                    let (sn, cs) = (-w.to_radians()).sin_cos();
                    let mut t = -diag / 2.;
                    while t <= diag / 2. {
                        // Linie in Richtung (cs, sn), um t senkrecht versetzt.
                        let o = [m[0] - sn * t, m[1] + cs * t];
                        self.op("m", vec![r(o[0] - cs * diag), r(o[1] - sn * diag)]);
                        self.op("l", vec![r(o[0] + cs * diag), r(o[1] + sn * diag)]);
                        t += a;
                    }
                }
                self.op("S", vec![]);
                self.op("Q", vec![]);
            }
        }
    }
    fn rechteck(&mut self, x: f64, y: f64, w: f64, h: f64, breite_mm: f64) {
        self.stift(&Stift {
            breite_mm,
            ..Stift::default()
        });
        self.op("re", vec![r(x), r(y), r(w), r(h)]);
        self.op("S", vec![]);
    }
    /// Text mit linker unterer Ecke bei (x, y) (y nach unten), gedreht gegen den Uhrzeigersinn.
    fn text(&mut self, text: &str, x: f64, y: f64, groesse_pt: f64, grad: f64) -> CoreResult<()> {
        if text.trim().is_empty() {
            return Ok(());
        }
        let breite = (text.chars().count() as f64 * groesse_pt * 0.72 + 6.).min(4000.);
        let hoehe = groesse_pt * 1.25 + 2.5;
        let x_obj = schrift::darstellung(
            self.d,
            text,
            breite as f32,
            hoehe as f32,
            groesse_pt as f32,
            self.font,
            false,
        )?;
        let name = format!("BauLayout{}", self.n);
        self.n += 1;
        self.xobjects.set(name.as_bytes(), x_obj);
        let (s, c) = grad.to_radians().sin_cos();
        self.op("q", vec![]);
        self.op("cm", vec![r(c), r(-s), r(-s), r(-c), r(x), r(y)]);
        self.op("Do", vec![Object::Name(name.into_bytes())]);
        self.op("Q", vec![]);
        Ok(())
    }
    /// Plankopf mit linker oberer Ecke (x, y) in Seitenpunkten.
    fn plankopf(&mut self, k: &Plankopf, x: f64, y: f64) -> CoreResult<()> {
        let mm = PT_MM;
        let (w, h) = (KOPF_B * mm, KOPF_H * mm);
        self.op("q", vec![]);
        self.op("rg", vec![1.into(), 1.into(), 1.into()]);
        self.op("re", vec![r(x), r(y), r(w), r(h)]);
        self.op("f", vec![]);
        self.op("Q", vec![]);
        self.rechteck(x, y, w, h, 0.5);
        let zeilen: [(&str, &str, f64, f64); 5] = [
            ("Bauherr", &k.bauherr, 10., 8.5),
            ("Bauvorhaben", &k.vorhaben, 10., 8.5),
            ("Grundstück", &k.grundstueck, 8., 8.),
            ("Planinhalt", &k.planinhalt, 12., 11.),
            ("Entwurfsverfasser", &k.planverfasser, 11., 8.),
        ];
        let mut oben = y;
        for (titel, wert, hoehe, groesse) in zeilen {
            self.text(titel, x + 1.5 * mm, oben + 3. * mm, 5.5, 0.)?;
            self.text(
                wert,
                x + 1.5 * mm,
                oben + hoehe * mm - 1.2 * mm,
                groesse,
                0.,
            )?;
            oben += hoehe * mm;
            self.stift(&Stift {
                breite_mm: 0.18,
                ..Stift::default()
            });
            self.pfad(&[[x, oben], [x + w, oben]], false);
            self.op("S", vec![]);
        }
        // Unterschriftsfeld neben dem Entwurfsverfasser.
        let u = oben - 11. * mm;
        self.pfad(&[[x + w * 0.62, u], [x + w * 0.62, oben]], false);
        self.op("S", vec![]);
        self.text(
            "Unterschrift",
            x + w * 0.62 + 1.5 * mm,
            u + 3. * mm,
            5.5,
            0.,
        )?;
        let unten: [(&str, &str, f64); 5] = [
            ("Plan-Nr.", &k.plannummer, 0.38),
            ("Index", &k.index, 0.1),
            ("Maßstab", &k.massstab, 0.16),
            ("Datum", &k.datum, 0.18),
            ("Format", &k.format, 0.18),
        ];
        let mut links = x;
        for (i, (titel, wert, anteil)) in unten.iter().enumerate() {
            if i > 0 {
                self.pfad(&[[links, oben], [links, y + h]], false);
                self.op("S", vec![]);
            }
            self.text(titel, links + 1.5 * mm, oben + 3. * mm, 5.5, 0.)?;
            self.text(
                wert,
                links + 1.5 * mm,
                y + h - 1.5 * mm,
                if i == 0 { 10. } else { 8.5 },
                0.,
            )?;
            links += w * anteil;
        }
        Ok(())
    }
}
/// Texte des Plankopfs, damit die eingebettete Schrift alle Zeichen enthält.
fn kopftexte(k: &Plankopf) -> Vec<String> {
    let mut t: Vec<String> = [
        "Bauherr",
        "Bauvorhaben",
        "Grundstück",
        "Planinhalt",
        "Entwurfsverfasser",
        "Unterschrift",
        "Plan-Nr.",
        "Index",
        "Maßstab",
        "Datum",
        "Format",
        "N",
        "m",
        "Legende",
        "M 1 : 0123456789,",
    ]
    .map(String::from)
    .to_vec();
    t.extend([
        k.bauherr.clone(),
        k.vorhaben.clone(),
        k.grundstueck.clone(),
        k.planinhalt.clone(),
        k.planverfasser.clone(),
        k.plannummer.clone(),
        k.index.clone(),
        k.massstab.clone(),
        k.datum.clone(),
        k.format.clone(),
    ]);
    t
}
fn zeichnen(
    d: &mut Document,
    page: ObjectId,
    matrix: [f32; 6],
    texte: &[String],
    f: impl FnOnce(&mut Zeichner) -> CoreResult<()>,
) -> CoreResult<()> {
    let refs: Vec<&str> = texte.iter().map(String::as_str).collect();
    let font = schrift::einbetten(d, &refs)?;
    let mut ressourcen = d
        .get_dictionary(page)
        .map_err(fehler)?
        .get(b"Resources")
        .ok()
        .and_then(|o| d.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let xobjects = ressourcen
        .get(b"XObject")
        .ok()
        .and_then(|o| d.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let mut z = Zeichner {
        d,
        font,
        ops: vec![
            Operation::new("q", vec![]),
            Operation::new("cm", matrix.iter().map(|v| Object::Real(*v)).collect()),
            Operation::new("J", vec![0.into()]),
            Operation::new("j", vec![1.into()]),
        ],
        xobjects,
        n: 0,
    };
    f(&mut z)?;
    z.ops.push(Operation::new("Q", vec![]));
    let Zeichner {
        d, ops, xobjects, ..
    } = z;
    ressourcen.set("XObject", xobjects);
    d.get_dictionary_mut(page)
        .map_err(fehler)?
        .set("Resources", ressourcen);
    d.add_page_contents(page, Content { operations: ops }.encode().map_err(fehler)?)
        .map_err(fehler)?;
    Ok(())
}
fn neues_blatt(breite_pt: f64, hoehe_pt: f64) -> (Document, ObjectId) {
    let mut d = Document::with_version("1.5");
    let pages_id = d.new_object_id();
    let content = d.add_object(Stream::new(Dictionary::new(), Vec::new()));
    let page = d.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), r(breite_pt), r(hoehe_pt)],
        "Contents" => content,
        "Resources" => Dictionary::new(),
    });
    d.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! {"Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1},
        ),
    );
    let catalog = d.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    d.trailer.set("Root", catalog);
    (d, page)
}
/// Längen der Maßstabsleiste in Metern: 5 Teile, auf 1-2-5 gerundet, etwa 50 mm Papier.
pub fn massstabsleiste(massstab: f64) -> f64 {
    let ziel = 50. / 1000. * massstab / 5.;
    let e = 10f64.powf(ziel.log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .map(|f| f * e)
        .find(|v| *v >= ziel * 0.7)
        .unwrap_or(10. * e)
}
fn zahl(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}
/// Grenzen (min, max) der Inhalte in Blattmetern; `None` ohne Inhalt.
pub fn umfang(b: &Planblatt) -> Option<([f64; 2], [f64; 2])> {
    let mut it = b
        .linien
        .iter()
        .flat_map(|l| l.punkte.iter())
        .chain(b.texte.iter().map(|t| &t.punkt));
    let first = *it.next()?;
    let (mut lo, mut hi) = (first, first);
    for p in it {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    Some((lo, hi))
}
/// Planblatt als PDF. Die Zeichnung steht maßstäblich mittig in der Zeichenfläche
/// über dem Plankopf; passt sie nicht, schlägt die Ausgabe mit einem Hinweis fehl.
pub fn blatt(b: &Planblatt) -> CoreResult<Vec<u8>> {
    if !(b.massstab >= 1. && b.massstab <= 100_000.) || !(b.breite_mm >= 180. && b.hoehe_mm >= 180.)
    {
        return Err(fehler("Ungültiges Blatt oder ungültiger Maßstab"));
    }
    let mm = PT_MM;
    let (bw, bh) = (b.breite_mm * mm, b.hoehe_mm * mm);
    let (rx, ry, rw, rh) = (
        RAND_LINKS * mm,
        RAND * mm,
        bw - (RAND_LINKS + RAND) * mm,
        bh - 2. * RAND * mm,
    );
    // Zeichenfläche: Rahmen ohne Plankopfstreifen unten.
    let flaeche_h = rh - (KOPF_H + 4.) * mm;
    let k = 1000. / b.massstab * mm;
    let (lo, hi) = umfang(b).unwrap_or(([0., 0.], [0., 0.]));
    let (iw, ih) = ((hi[0] - lo[0]) * k, (hi[1] - lo[1]) * k);
    if iw > rw - 10. * mm || ih > flaeche_h - 10. * mm {
        return Err(fehler(format!(
            "Die Zeichnung ({} × {} m) passt bei 1 : {} nicht auf das Blatt; größeres Format oder kleineren Maßstab wählen",
            zahl(hi[0] - lo[0]),
            zahl(hi[1] - lo[1]),
            zahl(b.massstab)
        )));
    }
    let ox = rx + (rw - iw) / 2. - lo[0] * k;
    let oy = ry + (flaeche_h - ih) / 2. - lo[1] * k;
    let p = |q: &[f64; 2]| [ox + q[0] * k, oy + q[1] * k];
    let (mut d, page) = neues_blatt(bw, bh);
    let mut texte = kopftexte(&b.kopf);
    texte.extend(b.texte.iter().map(|t| t.text.clone()));
    texte.extend(b.legende.iter().map(|l| l.name.clone()));
    texte.push(format!("M 1 : {}", zahl(b.massstab)));
    let leiste = massstabsleiste(b.massstab);
    texte.extend((0..=5).map(|i| zahl(leiste * i as f64)));
    let matrix = [1., 0., 0., -1., 0., bh as f32];
    zeichnen(&mut d, page, matrix, &texte, |z| {
        z.rechteck(rx, ry, rw, rh, 0.5);
        for l in &b.linien {
            let ps: Vec<_> = l.punkte.iter().map(p).collect();
            z.linie(&ps, l.geschlossen, &l.stift, &l.fuellung);
        }
        for t in &b.texte {
            let q = p(&t.punkt);
            z.op("q", vec![]);
            z.text(
                &t.text,
                q[0],
                q[1],
                (t.hoehe_mm * mm).clamp(3., 200.),
                t.drehung_grad,
            )?;
            z.op("Q", vec![]);
        }
        // Plankopf unten rechts.
        let kx = rx + rw - KOPF_B * mm;
        let ky = ry + rh - KOPF_H * mm;
        z.plankopf(&b.kopf, kx, ky)?;
        // Maßstabsleiste unten links.
        let (lx, ly) = (rx + 6. * mm, ry + rh - 10. * mm);
        let teil = leiste * k;
        for i in 0..5 {
            let x = lx + teil * i as f64;
            z.op("q", vec![]);
            if i % 2 == 0 {
                z.op("rg", vec![0.1.into(), 0.1.into(), 0.1.into()]);
            } else {
                z.op("rg", vec![1.into(), 1.into(), 1.into()]);
            }
            z.op("re", vec![r(x), r(ly), r(teil), r(2. * mm)]);
            z.op("B", vec![]);
            z.op("Q", vec![]);
        }
        z.rechteck(lx, ly, teil * 5., 2. * mm, 0.18);
        for i in [0usize, 1, 2, 5] {
            z.text(
                &zahl(leiste * i as f64),
                lx + teil * i as f64 - 2.,
                ly - 1.5 * mm,
                6.,
                0.,
            )?;
        }
        z.text("m", lx + teil * 5. + 2. * mm, ly + 2. * mm, 6., 0.)?;
        z.text(
            &format!("M 1 : {}", zahl(b.massstab)),
            lx,
            ly + 7. * mm,
            9.,
            0.,
        )?;
        // Nordpfeil oben rechts in der Zeichenfläche.
        if let Some(w) = b.nordwinkel_grad {
            let (c, rad) = ([rx + rw - 18. * mm, ry + 18. * mm], 9. * mm);
            let (s, co) = w.to_radians().sin_cos();
            // Spitze zeigt bei 0° nach oben (y nach unten: −y).
            let dreh = |dx: f64, dy: f64| [c[0] + dx * co + dy * s, c[1] - dx * s + dy * co];
            z.stift(&Stift::default());
            let kreis: Vec<_> = (0..48)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 48.;
                    [c[0] + rad * a.cos(), c[1] + rad * a.sin()]
                })
                .collect();
            z.pfad(&kreis, true);
            z.op("S", vec![]);
            let spitze = [
                dreh(0., -rad * 1.25),
                dreh(rad * 0.35, rad * 0.6),
                dreh(0., rad * 0.25),
                dreh(-rad * 0.35, rad * 0.6),
            ];
            z.op("q", vec![]);
            z.op("rg", vec![0.1.into(), 0.1.into(), 0.1.into()]);
            z.pfad(&spitze, true);
            z.op("f", vec![]);
            z.op("Q", vec![]);
            let n = dreh(0., -rad * 1.6);
            z.text("N", n[0] - 3., n[1] + 4., 10., w)?;
        }
        // Legende über dem Plankopf.
        if !b.legende.is_empty() {
            let lx = rx + rw - KOPF_B * mm;
            let zeilen = b.legende.len().min(12);
            let mut ly = ry + rh - KOPF_H * mm - (zeilen as f64 * 5. + 7.) * mm;
            z.text("Legende", lx, ly + 4. * mm, 7.5, 0.)?;
            for e in b.legende.iter().take(12) {
                ly += 5. * mm;
                let feld = [
                    [lx, ly],
                    [lx + 12. * mm, ly],
                    [lx + 12. * mm, ly + 3. * mm],
                    [lx, ly + 3. * mm],
                ];
                if matches!(e.fuellung, Fuellung::Keine) {
                    z.linie(
                        &[[lx, ly + 1.5 * mm], [lx + 12. * mm, ly + 1.5 * mm]],
                        false,
                        &e.stift,
                        &e.fuellung,
                    );
                } else {
                    z.linie(&feld, true, &e.stift, &e.fuellung);
                }
                z.text(&e.name, lx + 15. * mm, ly + 3. * mm, 7., 0.)?;
            }
        }
        Ok(())
    })?;
    pdf::speichern(d)
}
/// Setzt den Plankopf unten rechts auf eine Seite eines vorhandenen PDF-Plans.
/// Alle anderen Seiten und Inhalte bleiben unverändert.
pub fn stempeln(original: &[u8], seite: u32, kopf: &Plankopf) -> CoreResult<Vec<u8>> {
    let mut d = pdf::auswahl(original, &[])?;
    let page = *d
        .get_pages()
        .get(&seite)
        .ok_or_else(|| fehler("Seite nicht vorhanden"))?;
    let info = crate::formular::editor::seiten(original)?
        .into_iter()
        .find(|s| s.nummer == seite)
        .ok_or_else(|| fehler("Seite nicht vorhanden"))?;
    let (bw, bh) = if info.rotation == 90 || info.rotation == 270 {
        (info.hoehe_mm as f64, info.breite_mm as f64)
    } else {
        (info.breite_mm as f64, info.hoehe_mm as f64)
    };
    if bw < KOPF_B + 2. * RAND || bh < KOPF_H + 2. * RAND {
        return Err(fehler(
            "Seite zu klein für den Plankopf (mindestens 194 × 76 mm)",
        ));
    }
    let matrix = crate::formular::editor::seitenabbildung(&d, page)?;
    let mm = PT_MM;
    zeichnen(&mut d, page, matrix, &kopftexte(kopf), |z| {
        z.plankopf(kopf, (bw - RAND - KOPF_B) * mm, (bh - RAND - KOPF_H) * mm)
    })?;
    pdf::speichern(d)
}

/// Revisionswolken (Bogenkette um jedes Rechteck) mit Indexdreieck auf einer
/// Seite; Bereiche in sichtbaren Seitenpunkten (links oben, y nach unten).
pub fn wolken(
    original: &[u8],
    seite: u32,
    bereiche: &[[f64; 4]],
    index: &str,
) -> CoreResult<Vec<u8>> {
    let mut d = pdf::auswahl(original, &[])?;
    let page = *d
        .get_pages()
        .get(&seite)
        .ok_or_else(|| fehler("Seite nicht vorhanden"))?;
    let matrix = crate::formular::editor::seitenabbildung(&d, page)?;
    let texte = vec![index.to_string()];
    zeichnen(&mut d, page, matrix, &texte, |z| {
        let rot = Stift {
            breite_mm: 0.5,
            farbe: [0.8, 0.05, 0.1],
            muster: vec![],
        };
        for b in bereiche {
            let (x, y, w, h) = (b[0], b[1], b[2].abs().max(4.), b[3].abs().max(4.));
            // Bögen etwa alle 8 mm, mindestens zwei je Seite.
            let bogen = 8. * PT_MM;
            let ecken = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
            z.stift(&rot);
            z.op("m", vec![r(x), r(y)]);
            for i in 0..4 {
                let (a, e) = (ecken[i], ecken[(i + 1) % 4]);
                let l = (e[0] - a[0]).hypot(e[1] - a[1]);
                let n = ((l / bogen).round() as usize).clamp(2, 400);
                let u = [(e[0] - a[0]) / n as f64, (e[1] - a[1]) / n as f64];
                // Außen liegt links der Laufrichtung im Uhrzeigersinn (y nach unten).
                let aussen = [u[1], -u[0]];
                for k in 0..n {
                    let p0 = [a[0] + u[0] * k as f64, a[1] + u[1] * k as f64];
                    let p1 = [p0[0] + u[0], p0[1] + u[1]];
                    let c1 = [p0[0] + aussen[0] * 0.55, p0[1] + aussen[1] * 0.55];
                    let c2 = [p1[0] + aussen[0] * 0.55, p1[1] + aussen[1] * 0.55];
                    z.op(
                        "c",
                        vec![r(c1[0]), r(c1[1]), r(c2[0]), r(c2[1]), r(p1[0]), r(p1[1])],
                    );
                }
            }
            z.op("S", vec![]);
            // Indexdreieck rechts oben an der Wolke.
            if !index.trim().is_empty() {
                let (tx, ty, s) = (x + w + 3. * PT_MM, y - 3. * PT_MM, 7. * PT_MM);
                z.stift(&rot);
                z.pfad(
                    &[[tx, ty], [tx + s, ty], [tx + s / 2., ty - s * 0.87]],
                    true,
                );
                z.op("S", vec![]);
                z.text(index, tx + s * 0.28, ty - 0.6 * PT_MM, 9., 0.)?;
            }
        }
        Ok(())
    })?;
    pdf::speichern(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kopf() -> Plankopf {
        Plankopf {
            bauherr: "Erika Mustermann".into(),
            vorhaben: "Neubau Einfamilienhaus".into(),
            grundstueck: "Musterstadt, Flur 3, Flurstück 120/4".into(),
            planinhalt: "Grundriss Erdgeschoss".into(),
            planverfasser: "Planungsbüro Beispiel".into(),
            plannummer: "EFH-GR-EG-01".into(),
            index: "B".into(),
            massstab: "1 : 100".into(),
            datum: "01.11.2026".into(),
            format: "A3".into(),
        }
    }
    fn blatt_mit(breite: f64) -> Planblatt {
        Planblatt {
            breite_mm: 420.,
            hoehe_mm: 297.,
            massstab: 100.,
            linien: vec![Planlinie {
                ebene: String::new(),
                punkte: vec![[0., 0.], [breite, 0.], [breite, 8.], [0., 8.]],
                geschlossen: true,
                stift: Stift {
                    breite_mm: 0.5,
                    ..Stift::default()
                },
                fuellung: Fuellung::Linien {
                    winkel_grad: 45.,
                    abstand_mm: 2.,
                    kreuz: false,
                },
            }],
            texte: vec![Plantext {
                ebene: String::new(),
                punkt: [1., 4.],
                text: "Wohnen".into(),
                hoehe_mm: 3.5,
                drehung_grad: 0.,
                farbe: [0., 0., 0.],
            }],
            kopf: kopf(),
            nordwinkel_grad: Some(15.),
            legende: vec![Legendeneintrag {
                name: "Wände Neubau".into(),
                stift: Stift::default(),
                fuellung: Fuellung::Voll(0.5),
            }],
        }
    }

    #[test]
    fn blatt_ist_massstaeblich_und_hat_plankopf() {
        let pdf = blatt(&blatt_mit(12.)).unwrap();
        let d = Document::load_mem(&pdf).unwrap();
        let page = *d.get_pages().get(&1).unwrap();
        let media = d
            .get_dictionary(page)
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        let h = media[3].as_float().unwrap() as f64;
        assert!((h / PT_MM - 297.).abs() < 0.01);
        assert!(pdf.len() > 1000);
    }
    #[test]
    fn zu_grosse_zeichnung_wird_abgewiesen() {
        let e = blatt(&blatt_mit(400.)).unwrap_err().to_string();
        assert!(e.contains("passt bei 1 : 100 nicht"), "{e}");
    }
    #[test]
    fn massstabsleiste_rundet() {
        assert_eq!(massstabsleiste(100.), 1.);
        assert_eq!(massstabsleiste(500.), 5.);
        assert_eq!(massstabsleiste(50.), 0.5);
        assert_eq!(massstabsleiste(1000.), 10.);
    }
    #[test]
    fn stempel_auf_vorhandenem_plan() {
        let (d, _) = neues_blatt(420. * PT_MM, 297. * PT_MM);
        let mut b = Vec::new();
        let mut d = d;
        d.save_to(&mut b).unwrap();
        let aus = stempeln(&b, 1, &kopf()).unwrap();
        assert!(aus.len() > b.len());
        assert!(stempeln(&b, 2, &kopf()).is_err());
        let w = wolken(&b, 1, &[[100., 100., 200., 80.]], "b").unwrap();
        assert!(w.len() > b.len());
    }
}
