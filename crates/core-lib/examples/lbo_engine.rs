//! Initialisierung der modularen LBO-Berechnungs-Engine.
//!
//! ```sh
//! cargo run -p core-lib --example lbo_engine
//! ```

use std::path::Path;

use core_lib::calc::Baugebiet;
use core_lib::domain::Bundesland;
use core_lib::lbo::{AbstandsflaecheInput, LboEngine, RulePackSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Engine erzeugen: Sandbox-Limits + native Hilfsfunktionen, noch ohne Landesrecht.
    let engine = LboEngine::new();

    // 2. Mitgelieferte Regelpakete laden (in der Desktop-App: Ressourcenverzeichnis
    //    des Installers; auf dem Server: konfigurierbares Verzeichnis).
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lbo-rules");
    let (geladen, fehler) = engine.load_directory(&dir)?;
    for info in &geladen {
        println!(
            "geladen: {} {} – {}",
            info.land.name(),
            info.version,
            info.rechtsstand
        );
    }
    for e in &fehler {
        eprintln!("übersprungen: {e}");
    }

    // 3. Berechnen.
    let input = AbstandsflaecheInput {
        wandhoehe_m: 9.5,
        gebiet: Baugebiet::WA,
    };
    for land in [Bundesland::By, Bundesland::Bw, Bundesland::Nw] {
        let r = engine.abstandsflaeche(land, &input)?;
        println!(
            "{:<20} H = {:.2} m → T = {:.2} m ({} × H, min. {} m; {})",
            land.name(),
            input.wandhoehe_m,
            r.ergebnis.tiefe_m,
            r.ergebnis.faktor,
            r.ergebnis.mindesttiefe_m,
            r.ergebnis.rechtsgrundlage
        );
    }

    // 4. LBO-Update zur Laufzeit: neues Skript (z. B. vom Update-Server)
    //    ersetzt das alte Paket atomar – ohne Neukompilierung der App.
    let update = r#"
        fn meta() { #{ land: "BY", version: "2026.2", rechtsstand: "BayBO (fiktive Novelle)" } }
        fn abstandsflaeche(input) {
            #{ tiefe_m: aufrunden(max(0.35 * input.wandhoehe_m, 3.0), 2), faktor: 0.35,
               mindesttiefe_m: 3.0, rechtsgrundlage: "Art. 6 BayBO n. F." }
        }
    "#;
    engine.load_rule_pack(RulePackSource {
        land: Bundesland::By,
        script: update.into(),
        origin: "update-server".into(),
    })?;
    let r = engine.abstandsflaeche(Bundesland::By, &input)?;
    println!(
        "nach Update ({}): Bayern T = {:.2} m",
        r.regelpaket.version, r.ergebnis.tiefe_m
    );
    Ok(())
}
