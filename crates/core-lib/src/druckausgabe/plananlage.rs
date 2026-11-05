//! Öffentlicher Darstellungsvertrag für maßstäbliche Plananlagen. Das
//! Aufmaßmodul liefert fertige Werte und Konturen; PDF-Details bleiben hier.
use super::*;
use lopdf::{
    Object,
    content::{Content, Operation},
};

pub struct Plankontur {
    pub ringe: Vec<Vec<[f64; 2]>>,
    pub geschlossen: bool,
    pub beschriftung: String,
}
/// Darstellung einer Kontur; Flächen werden halbtransparent gefüllt, damit
/// der Plan darunter lesbar bleibt.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Konturstil {
    /// Aufmaßmarkierung (blau, Linie).
    #[default]
    Messung,
    /// Grundstücksgrenze (schwarz).
    Grenze,
    /// Wandlinie (dunkel, kräftig).
    Wand,
    /// Abstandsfläche (blau gefüllt).
    Abstandsflaeche,
    /// Fläche jenseits der Grenze (rot gefüllt).
    Konflikt,
    /// Überdeckung zweier Abstandsflächen (violett gefüllt).
    Ueberdeckung,
}
impl Konturstil {
    /// Linienfarbe, Linienbreite, Füllung (Farbe, Deckkraft).
    fn darstellung(self) -> ([f32; 3], f32, Option<f32>) {
        match self {
            Self::Messung => ([0.07, 0.43, 0.59], 1.2, None),
            Self::Grenze => ([0., 0., 0.], 1.0, None),
            Self::Wand => ([0.1, 0.1, 0.1], 2.2, None),
            Self::Abstandsflaeche => ([0.14, 0.36, 0.53], 0.8, Some(0.2)),
            Self::Konflikt => ([0.8, 0.1, 0.1], 1.0, Some(0.35)),
            Self::Ueberdeckung => ([0.55, 0.25, 0.58], 0.8, Some(0.3)),
        }
    }
}
pub struct Plananlage {
    pub seite: u32,
    pub titel: String,
    pub herkunft: Vec<String>,
    pub legende: Vec<Vec<String>>,
    pub konturen: Vec<Plankontur>,
    /// Darstellung je Kontur (gleiche Reihenfolge); fehlt sie, gilt
    /// [`Konturstil::Messung`].
    pub stile: Vec<Konturstil>,
    /// Spaltenköpfe der Legende; leer für die Aufmaßlegende.
    pub spalten: Vec<String>,
}
/// Neue PDF-Ausgabe, Originalbytes bleiben unangetastet. Die Planseite bleibt
/// unverändert maßstäblich; Quellen, Legende und Kalibrierung folgen auf A4.
pub fn erstellen(original: &[u8], a: &Plananlage) -> CoreResult<Vec<u8>> {
    let mut d = pdf::auswahl(original, &[a.seite])?;
    let page = *d
        .get_pages()
        .values()
        .next()
        .ok_or_else(|| fehler("PDF-Seite fehlt"))?;
    let matrix = crate::formular::editor::seitenabbildung(&d, page)?;
    let texts: Vec<&str> = a.konturen.iter().map(|k| k.beschriftung.as_str()).collect();
    let font = crate::formular::schrift::einbetten(&mut d, &texts)?;
    let mut ressourcen = d
        .get_dictionary(page)
        .map_err(fehler)?
        .get(b"Resources")
        .ok()
        .and_then(|o| d.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let mut xobjects = ressourcen
        .get(b"XObject")
        .ok()
        .and_then(|o| d.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    // Füllungen halbtransparent; je Deckkraft ein Grafikzustand.
    let mut zustaende = ressourcen
        .get(b"ExtGState")
        .ok()
        .and_then(|o| d.dereference(o).ok())
        .and_then(|(_, o)| o.as_dict().ok())
        .cloned()
        .unwrap_or_default();
    let mut ops = vec![
        Operation::new("q", vec![]),
        Operation::new("cm", matrix.iter().map(|v| Object::Real(*v)).collect()),
    ];
    for (i, k) in a.konturen.iter().enumerate() {
        let (farbe, breite, fuellung) = a.stile.get(i).copied().unwrap_or_default().darstellung();
        let farbe: Vec<Object> = farbe.iter().map(|v| Object::Real(*v)).collect();
        ops.push(Operation::new("q", vec![]));
        if let Some(deckkraft) = fuellung {
            let name = format!("BauDeckkraft{}", (deckkraft * 100.) as u32);
            if !zustaende.has(name.as_bytes()) {
                let mut gs = lopdf::Dictionary::new();
                gs.set("Type", Object::Name(b"ExtGState".to_vec()));
                gs.set("ca", Object::Real(deckkraft));
                let gs = d.add_object(gs);
                zustaende.set(name.as_bytes(), gs);
            }
            ops.push(Operation::new("gs", vec![Object::Name(name.into_bytes())]));
            ops.push(Operation::new("rg", farbe.clone()));
        }
        ops.push(Operation::new("RG", farbe));
        ops.push(Operation::new("w", vec![Object::Real(breite)]));
        for r in &k.ringe {
            if let Some(p) = r.first() {
                ops.push(Operation::new(
                    "m",
                    vec![Object::Real(p[0] as f32), Object::Real(p[1] as f32)],
                ));
                for p in r.iter().skip(1) {
                    ops.push(Operation::new(
                        "l",
                        vec![Object::Real(p[0] as f32), Object::Real(p[1] as f32)],
                    ));
                }
                if k.geschlossen {
                    ops.push(Operation::new("h", vec![]));
                }
            }
        }
        ops.push(Operation::new(
            if fuellung.is_some() && k.geschlossen {
                "B*"
            } else {
                "S"
            },
            vec![],
        ));
        ops.push(Operation::new("Q", vec![]));
        if let Some(p) = k.ringe.first().and_then(|r| r.first())
            && !k.beschriftung.is_empty()
        {
            let label = crate::formular::schrift::darstellung(
                &mut d,
                &k.beschriftung,
                300.,
                16.,
                9.,
                font,
                false,
            )?;
            let name = format!("BauAufmass{i}");
            xobjects.set(name.as_bytes(), label);
            ops.extend([
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        1.into(),
                        0.into(),
                        0.into(),
                        (-1).into(),
                        Object::Real(p[0] as f32 + 3.),
                        Object::Real(p[1] as f32 - 3.),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(name.into_bytes())]),
                Operation::new("Q", vec![]),
            ]);
        }
    }
    ops.push(Operation::new("Q", vec![]));
    ressourcen.set("XObject", xobjects);
    if !zustaende.is_empty() {
        ressourcen.set("ExtGState", zustaende);
    }
    d.get_dictionary_mut(page)
        .map_err(fehler)?
        .set("Resources", ressourcen);
    d.add_page_contents(page, Content { operations: ops }.encode().map_err(fehler)?)
        .map_err(fehler)?;
    let mut zeilen: Vec<_> = a.herkunft.iter().map(berichte::text).collect();
    let spalten: Vec<&str> = if a.spalten.is_empty() {
        vec!["Nr. / Markierung", "Bedeutung / Ebene", "Messwerte"]
    } else {
        a.spalten.iter().map(String::as_str).collect()
    };
    let breiten = vec![2.; spalten.len()];
    zeilen.push(berichte::tabelle(&spalten, &breiten, a.legende.clone()));
    let legende = pdf::bericht(&a.titel, None, &zeilen)?;
    pdf::verbinden(vec![d, legende])
}
