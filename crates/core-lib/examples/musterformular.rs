//! Schreibt ein ausfüllbares Muster-PDF zum Ausprobieren der Formularfunktion.
//!
//! ```sh
//! cargo run -p core-lib --features testdaten --example musterformular -- muster.pdf
//! ```

fn main() -> std::io::Result<()> {
    let ziel = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "musterformular.pdf".into());
    std::fs::write(&ziel, core_lib::formular::testdaten::musterformular())?;
    println!("{ziel} geschrieben");
    Ok(())
}
