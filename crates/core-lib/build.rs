fn main() {
    // Öffentlicher Lizenzschlüssel des Herstellers, siehe license/datei.rs.
    println!("cargo:rerun-if-env-changed=BAUPROJEKT_LIZENZ_SCHLUESSEL");
}
