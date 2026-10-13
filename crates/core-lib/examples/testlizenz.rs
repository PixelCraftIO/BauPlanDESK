//! Stellt eine Lizenz mit dem Testschlüssel aus. Debug-Builds der App
//! vertrauen diesem Schlüssel, Release-Builds nicht.
//!
//! ```sh
//! cargo run -p core-lib --features testdaten --example testlizenz -- test.bplizenz 3 "Architekturbüro Muster"
//! ```

use chrono::Utc;
use core_lib::license::datei::{ausstellen, testschluessel};
use core_lib::license::{Edition, Lizenz};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ziel = args.first().map_or("test.bplizenz", String::as_str);
    let arbeitsplaetze = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(3);
    let lizenznehmer = args.get(2).map_or("Architekturbüro Muster", String::as_str);
    let heute = Utc::now().date_naive();
    let lizenz = Lizenz {
        lizenz_id: uuid::Uuid::new_v4(),
        lizenznehmer: lizenznehmer.into(),
        edition: Edition::Buero,
        arbeitsplaetze,
        ausgestellt_am: heute,
        gueltig_bis: None,
        wartung_bis: heute.checked_add_months(chrono::Months::new(12)),
    };
    std::fs::write(ziel, ausstellen(&lizenz, &testschluessel::geheim())).expect("schreiben");
    println!("{ziel}: {lizenznehmer}, {arbeitsplaetze} Arbeitsplätze (Testschlüssel)");
}
