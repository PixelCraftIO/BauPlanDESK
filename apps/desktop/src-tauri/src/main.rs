// Unter Windows kein zusätzliches Konsolenfenster im Release-Build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;
mod commands;
mod config;
mod lizenzdienst;
mod menu;
mod updater;

use tauri::{Emitter, Manager, RunEvent};

/// So oft meldet sich der Arbeitsplatz als belegt (die Lizenz gibt ihn nach
/// 15 Minuten ohne Meldung frei).
const LIZENZ_MELDUNG: std::time::Duration = std::time::Duration::from_secs(5 * 60);

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
            lizenz_melden(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::ablage_status,
            commands::ablage_wechseln,
            commands::projektordner_exportieren,
            commands::projekte_auflisten,
            commands::projekt_anlegen,
            commands::projekt_aktualisieren,
            commands::projekt_oeffnen,
            commands::projekt_laden,
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
            commands::lizenz_status,
            commands::lizenz_einspielen,
            commands::lizenz_aktivieren,
            commands::lizenz_bestaetigen,
            commands::lizenz_abmelden,
            commands::arbeitsplatz_belegen,
            commands::dokument_exportieren,
            commands::kunden_auflisten,
            commands::kunde_anlegen,
            commands::kunde_aendern,
            commands::kunde_loeschen,
            commands::kunde_projekte,
            commands::bauherren,
            commands::bauherren_setzen,
            commands::bauantrag,
            commands::bauantrag_speichern,
            commands::buero,
            commands::buero_speichern,
            commands::xbau_codelisten,
            commands::xbau_exportieren,
            commands::formular_analysieren,
            commands::pdf_vorschau,
            commands::formular_vorlage_erstellen,
            commands::formular_ausfuellen,
            commands::cad_plan,
            commands::aufmass,
            commands::projekt_nutzung_pruefen,
            commands::projekt_baugrundstuecke_pruefen,
            commands::baugrenzen_pruefen,
            commands::abstandsflaeche,
            commands::mass_der_nutzung,
            updater::update_pruefen,
            updater::update_installieren,
        ])
        .build(tauri::generate_context!())
        .expect("BauProjektIO konnte nicht gestartet werden")
        .run(|app, event| {
            // Arbeitsplatz sofort freigeben, damit ein Kollege ihn nutzen kann.
            if let RunEvent::Exit = event
                && let Some(state) = app.try_state::<bootstrap::AppState>()
            {
                let ergebnis = tauri::async_runtime::block_on(
                    state
                        .core
                        .license
                        .arbeitsplatz_freigeben(&state.arbeitsplatz),
                );
                if let Err(e) = ergebnis {
                    tracing::warn!(error = %e, "Arbeitsplatz nicht freigegeben");
                }
            }
        });
}

/// Meldet den Arbeitsplatz regelmäßig als belegt bzw. versucht, einen zu
/// bekommen, und schickt den Lizenzstatus an die Oberfläche.
fn lizenz_melden(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(LIZENZ_MELDUNG).await;
            let state = app.state::<bootstrap::AppState>();
            let name = state.benutzer().anzeigename;
            match state
                .core
                .license
                .arbeitsplatz_belegen(&state.arbeitsplatz, &name)
                .await
            {
                Ok(status) => {
                    let _ = app.emit("lizenz", status);
                }
                Err(e) => tracing::warn!(error = %e, "Lizenzmeldung fehlgeschlagen"),
            }
        }
    });
}
