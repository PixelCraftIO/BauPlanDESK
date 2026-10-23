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
pub struct Plananlage {
    pub seite: u32,
    pub titel: String,
    pub herkunft: Vec<String>,
    pub legende: Vec<Vec<String>>,
    pub konturen: Vec<Plankontur>,
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
    let mut ops = vec![
        Operation::new("q", vec![]),
        Operation::new("cm", matrix.iter().map(|v| Object::Real(*v)).collect()),
        Operation::new("RG", vec![0.07.into(), 0.43.into(), 0.59.into()]),
        Operation::new("w", vec![1.2.into()]),
    ];
    for (i, k) in a.konturen.iter().enumerate() {
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
        ops.push(Operation::new("S", vec![]));
        if let Some(p) = k.ringe.first().and_then(|r| r.first()) {
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
    d.get_dictionary_mut(page)
        .map_err(fehler)?
        .set("Resources", ressourcen);
    d.add_page_contents(page, Content { operations: ops }.encode().map_err(fehler)?)
        .map_err(fehler)?;
    let mut zeilen: Vec<_> = a.herkunft.iter().map(berichte::text).collect();
    zeilen.push(berichte::tabelle(
        &["Nr. / Markierung", "Bedeutung / Ebene", "Messwerte"],
        &[2., 2., 2.],
        a.legende.clone(),
    ));
    let legende = pdf::bericht(&a.titel, None, &zeilen)?;
    pdf::verbinden(vec![d, legende])
}
