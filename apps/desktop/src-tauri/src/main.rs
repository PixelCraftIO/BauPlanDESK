// Unter Windows kein zusätzliches Konsolenfenster im Release-Build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;
mod commands;
mod config;
mod menu;
mod updater;

use tauri::Manager;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        // Zweiter Start (z. B. Doppelklick auf eine Projektdatei) holt das
        // laufende Fenster nach vorn, statt eine zweite Instanz zu öffnen,
        // die sich mit der ersten um die SQLite-Datei streiten würde.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        // Fenstergröße, -position und Maximierung bleiben über Neustarts erhalten.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = tauri::async_runtime::block_on(bootstrap::init(app.handle()))?;
            app.manage(state);
            app.manage(updater::PendingUpdate::default());
            menu::install(app.handle())?;
            updater::check_in_background(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::projekte_auflisten,
            commands::projekt_anlegen,
            commands::projekt_aktualisieren,
            commands::projekt_oeffnen,
            commands::projekt_schliessen,
            commands::zuletzt_geoeffnet,
            commands::dokumente_auflisten,
            commands::dokument_importieren,
            commands::dokument_historie,
            commands::version_wiederherstellen,
            commands::dokument_sperren,
            commands::dokument_entsperren,
            commands::sperren_auflisten,
            commands::protokoll,
            commands::benutzer_auflisten,
            commands::benutzer_anlegen,
            commands::benutzer_aendern,
            commands::dokument_exportieren,
            commands::formular_analysieren,
            commands::formular_ausfuellen,
            commands::cad_plan,
            commands::aufmass,
            commands::abstandsflaeche,
            commands::mass_der_nutzung,
            updater::update_pruefen,
            updater::update_installieren,
        ])
        .run(tauri::generate_context!())
        .expect("BauProjektIO konnte nicht gestartet werden");
}
