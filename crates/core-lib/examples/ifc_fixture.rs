//! UI-Fixture des IFC-Testmodells (Einfamilienhaus, IFC4): Datei, Auszug,
//! Geschosse, Grundrisse und Pläne je Geschoss, wie die Tauri-Befehle sie
//! liefern. Erzeugt `apps/desktop/tests/fixtures/ifc-efh.json`:
//!
//! ```sh
//! cargo run -p core-lib --features testdaten --example ifc_fixture > apps/desktop/tests/fixtures/ifc-efh.json
//! ```
use base64::Engine;
use core_lib::ifc::{self, Modell, testdaten};
use serde_json::{Map, Value, json};

fn main() {
    let text = testdaten::efh_ifc4();
    let m = Modell::lesen("EFH.ifc", text.as_bytes()).expect("Testmodell lesbar");
    let mut grundrisse = Map::new();
    let mut plaene = Map::new();
    for g in &m.auszug.geschosse {
        let gr = ifc::grundriss(&m.auszug, &g.global_id, None).expect("Grundriss");
        plaene.insert(g.global_id.clone(), json!(ifc::als_plan(&gr)));
        grundrisse.insert(g.global_id.clone(), json!(gr));
    }
    let v = json!({
        "datei_name": "EFH-Mustermann.ifc",
        "datei_base64": base64::engine::general_purpose::STANDARD.encode(text.as_bytes()),
        "auszug": m.auszug,
        "geschosse": ifc::geschossliste(&m.auszug),
        "grundrisse": Value::Object(grundrisse),
        "plaene": Value::Object(plaene),
    });
    println!("{}", serde_json::to_string(&v).expect("JSON"));
}
