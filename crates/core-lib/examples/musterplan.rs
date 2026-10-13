//! Schreibt einen kleinen Lageplan als DXF zum Ausprobieren des Plan-Viewers.
//!
//! ```sh
//! cargo run -p core-lib --features testdaten --example musterplan -- Lageplan.dxf
//! ```

fn main() -> std::io::Result<()> {
    let ziel = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "musterplan.dxf".into());
    std::fs::write(&ziel, core_lib::cad::testdaten::musterplan())?;
    println!("{ziel} geschrieben");
    Ok(())
}
