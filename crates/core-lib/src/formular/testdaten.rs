//! Testdaten für Formular-Tests (auch für andere Crates, Feature `testdaten`).

use lopdf::{Dictionary, Document, Object, Stream, dictionary};

use super::pdf::FF_RADIO;

/// Erzeugt ein kleines Formular wie ein amtliches Muster: Textfelder (eines
/// verschachtelt), ein Ankreuzfeld, eine Optionsgruppe und eine Auswahl.
pub fn musterformular() -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let ap = |doc: &mut Document, an: &str| {
        let leer = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), 100.into(), 20.into()],
            },
            Vec::new(),
        ));
        let mut n = Dictionary::new();
        n.set(an, Object::Reference(leer));
        n.set("Off", Object::Reference(leer));
        dictionary! { "N" => n }
    };

    let widget = |extra: Dictionary| {
        let mut d = dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "P" => page_id,
            "Rect" => vec![0.into(), 0.into(), 100.into(), 20.into()],
        };
        d.extend(&extra);
        d
    };

    let name = doc.add_object(widget(dictionary! {
        "FT" => "Tx", "T" => Object::string_literal("Bauherr_Name"),
        "TU" => Object::string_literal("Name des Bauherrn"), "MaxLen" => 40,
    }));
    let flaeche_kind = doc.add_object(widget(dictionary! {
        "T" => Object::string_literal("Flaeche"),
    }));
    let grundstueck = doc.add_object(dictionary! {
        "FT" => "Tx", "T" => Object::string_literal("Grundstueck"),
        "Kids" => vec![flaeche_kind.into()],
    });
    doc.get_dictionary_mut(flaeche_kind)
        .unwrap()
        .set("Parent", grundstueck);

    let ap_ja = ap(&mut doc, "Ja");
    let check = doc.add_object(widget(dictionary! {
        "FT" => "Btn", "T" => Object::string_literal("Sonderbau"),
        "V" => "Off", "AS" => "Off", "AP" => ap_ja,
    }));

    let ap_a = ap(&mut doc, "Neubau");
    let ap_b = ap(&mut doc, "Umbau");
    let radio = doc.new_object_id();
    let r1 = doc.add_object(widget(
        dictionary! { "Parent" => radio, "AS" => "Off", "AP" => ap_a },
    ));
    let r2 = doc.add_object(widget(
        dictionary! { "Parent" => radio, "AS" => "Off", "AP" => ap_b },
    ));
    doc.objects.insert(
        radio,
        dictionary! {
            "FT" => "Btn", "Ff" => FF_RADIO, "T" => Object::string_literal("Vorhaben"),
            "Kids" => vec![r1.into(), r2.into()],
        }
        .into(),
    );

    let auswahl = doc.add_object(widget(dictionary! {
        "FT" => "Ch", "T" => Object::string_literal("Gebaeudeklasse"),
        "Opt" => vec![Object::string_literal("1"), Object::string_literal("2"), Object::string_literal("3")],
    }));

    let annots: Vec<Object> = [name, flaeche_kind, check, r1, r2, auswahl]
        .into_iter()
        .map(Object::from)
        .collect();
    doc.objects.insert(
        page_id,
        dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Annots" => annots,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }
        .into(),
    );
    doc.objects.insert(
        pages_id,
        dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }.into(),
    );
    let acro = doc.add_object(dictionary! {
        "Fields" => vec![name.into(), grundstueck.into(), check.into(), radio.into(), auswahl.into()],
    });
    let catalog = doc.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages_id, "AcroForm" => acro,
    });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    out
}
