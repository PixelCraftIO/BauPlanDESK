fn main() {
    for key in [
        "BAUPROJEKT_UPDATER_PUBKEY",
        "BAUPROJEKT_UPDATE_URL",
        "BAUPROJEKT_LIZENZSERVER",
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    tauri_build::build()
}
