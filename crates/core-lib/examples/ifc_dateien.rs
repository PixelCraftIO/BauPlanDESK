//! Schreibt die IFC-Testmodelle als Dateien, z. B. für die Bildschirmfotos
//! des Benutzerhandbuchs (`scripts/handbuch-bilder/aufnahmen.py`):
//!
//! ```sh
//! cargo run -p core-lib --features testdaten --example ifc_dateien -- <Ordner>
//! ```
//!
//! Ergebnis: `EFH-Mustermann.ifc` (erste Fassung) und
//! `EFH-Mustermann-V2.ifc` (zweite Fassung für den Abgleich).
use core_lib::ifc::testdaten;
use std::path::PathBuf;

fn main() {
    let ordner = PathBuf::from(std::env::args().nth(1).expect("Zielordner angeben"));
    std::fs::create_dir_all(&ordner).expect("Zielordner anlegen");
    std::fs::write(ordner.join("EFH-Mustermann.ifc"), testdaten::efh_ifc4()).expect("schreiben");
    std::fs::write(
        ordner.join("EFH-Mustermann-V2.ifc"),
        testdaten::efh_ifc4_v2(),
    )
    .expect("schreiben");
}
