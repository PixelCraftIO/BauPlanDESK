//! Zeigt, was der DXF-Import aus einer Datei macht: Einheit, Ebenen,
//! Elementzahlen, Hinweise und die Flächen aller geschlossenen Linienzüge.
//!
//! ```sh
//! cargo run -p core-lib --example plan_info -- Lageplan.dxf
//! ```

use core_lib::cad::{Laengeneinheit, dxf, messen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pfad = std::env::args()
        .nth(1)
        .ok_or("Aufruf: plan_info <datei.dxf>")?;
    let plan = dxf::lesen(&std::fs::read(&pfad)?)?;
    let einheit = plan.einheit.unwrap_or(Laengeneinheit::Meter);
    println!("Einheit: {:?}", plan.einheit);
    println!("Grenzen: {:?}", plan.grenzen);
    for (i, e) in plan.ebenen.iter().enumerate() {
        let linien = plan.linien.iter().filter(|l| l.ebene == i).count();
        let texte = plan.texte.iter().filter(|t| t.ebene == i).count();
        println!(
            "Ebene {:<16} Farbe {:<9} sichtbar {:<5} Linien {linien:>4} Texte {texte:>3}",
            e.name,
            e.farbe.as_deref().unwrap_or("-"),
            e.sichtbar
        );
    }
    for t in &plan.texte {
        println!("Text {:?} bei {:?}", t.inhalt, t.position);
    }
    for l in plan.linien.iter().filter(|l| l.geschlossen) {
        let m = messen(&l.punkte, true, einheit)?;
        println!(
            "Fläche auf {:<12} {:>10.3} m²  Umfang {:>8.3} m",
            plan.ebenen[l.ebene].name,
            m.flaeche_m2.unwrap_or_default(),
            m.laenge_m
        );
    }
    for h in &plan.hinweise {
        println!("Hinweis: {h}");
    }
    Ok(())
}
