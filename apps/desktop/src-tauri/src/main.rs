// Unter Windows kein zusätzliches Konsolenfenster im Release-Build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod abstandsflaechen;
mod anmeldung;
mod beispielprojekt;
mod berechnungen;
mod bibliothek;
mod bootstrap;
#[cfg(test)]
mod command_zugriff_tests;
mod commands;
mod config;
mod dateien;
mod druckausgabe;
mod druckjobs;
mod einreichungen;
mod emails;
mod ifc;
mod kategorien;
mod lizenzdienst;
mod menu;
mod plattform;
mod portale;
mod projektordner;
mod sicherungen;
mod updater;
mod verfahren;
mod xbau;

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
            app.manage(storage::druckartefakte::Druckartefakte::default());
            app.manage(druckjobs::Druckjobs::default());
            menu::install(app.handle())?;
            updater::check_in_background(app.handle().clone());
            projektordner::hintergrund(app.handle().clone());
            lizenz_melden(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::projektmitglieder,
            commands::projektmitglied_setzen,
            portale::portalprofile,
            portale::portalprofil_hinzufuegen,
            portale::portal_vorgang_anlegen,
            portale::portal_abgabe_pruefen,
            portale::portal_abgabe_festhalten,
            portale::portal_ereignis_erfassen,
            portale::portal_rueckmeldung_erfassen,
            portale::portal_paket_exportieren,
            portale::portal_datei_exportieren,
            portale::portal_oeffnen,
            portale::portal_xbau_archivieren,
            xbau::xbau_nachrichtentypen,
            xbau::xbau_nachrichten,
            xbau::xbau_pruefen,
            xbau::xbau_aufnehmen,
            xbau::xbau_beleg,
            xbau::xbau_anlage_ablegen,
            xbau::xbau_in_verfahren,
            xbau::xbau_auswertung,
            xbau::xbau_auswertung_uebernehmen,
            verfahren::verfahrensprofil,
            verfahren::verfahrenskataloge,
            verfahren::verfahren_verfahrenswahl,
            verfahren::verfahren_unterschriften,
            verfahren::verfahren_profil_anwenden,
            verfahren::verfahren_abgabe,
            verfahren::verfahren_export,
            verfahren::verfahren_externe_einreichung,
            verfahren::verfahren_xbau_erfassen,
            verfahren::verfahren_erinnerungen,
            verfahren::verfahren_planunterlagen,
            verfahren::verfahren_abweichungen,
            verfahren::verfahren_schreiben,
            verfahren::verfahren_optionen,
            verfahren::verfahren_checkliste,
            verfahren::verfahren_folgeverfahren,
            verfahren::verfahren_bescheidfristen,
            verfahren::verfahren_termine_ics,
            verfahren::verfahren_suche,
            verfahren::suchtext_speichern,
            verfahren::verfahren_aktenexport,
            verfahren::verfahren_ocr,
            druckausgabe::druckausgabe_ablegen,
            druckausgabe::druckkatalog,
            druckausgabe::druckausgabe_erstellen,
            druckausgabe::druckartefakt_erstellen,
            druckausgabe::druckartefakt_abbrechen,
            druckausgabe::druckartefakt_lesen,
            druckausgabe::druckartefakt_entfernen,
            druckausgabe::druckartefakt_exportieren,
            druckausgabe::druckartefakt_ablegen,
            druckausgabe::druckausgabe_exportieren,
            plattform::fachobjekte,
            plattform::fachobjektseite,
            plattform::fachobjekt_lesen,
            plattform::fachverlaufsseite,
            plattform::projektstatus,
            plattform::projektuebersicht,
            plattform::standard_checkliste,
            plattform::checkliste_anwenden,
            plattform::fachverlauf,
            plattform::fachobjekt_speichern,
            plattform::gebaeudezuordnung_uebertragen,
            plattform::flurstuecksverweise,
            plattform::flurstuecksverweise_entfernen,
            plattform::nachweise,
            plattform::nachweis_erstellen,
            plattform::nachweis_freigeben,
            plattform::nachweis_wiederholen,
            plattform::raumbuch_summen,
            plattform::gebaeude_auswerten,
            projektordner::projektordner_status,
            sicherungen::buero_sicherung_erstellen,
            sicherungen::buero_sicherung_wiederherstellen,
            sicherungen::sicherungsstatus,
            sicherungen::ablage_konsistenz,
            sicherungen::ablage_metadaten_ergaenzen,
            sicherungen::sicherung_erstellen,
            sicherungen::sicherung_pruefen,
            sicherungen::sicherung_wiederherstellen,
            sicherungen::profil_aktivieren,
            sicherungen::standardprofil_aktivieren,
            beispielprojekt::beispielprojekt_status,
            beispielprojekt::beispielprojekt_oeffnen,
            beispielprojekt::beispielprojekt_verlassen,
            projektordner::ausgabeauftraege,
            projektordner::ausgabe_wiederholen,
            projektordner::projektordner_waehlen,
            commands::baugrundstuecke_ermitteln,
            commands::dokument_entfernen,
            projektordner::dokument_im_explorer,
            dateien::dokument_extern_oeffnen,
            dateien::arbeitskopien,
            einreichungen::einreichungen,
            einreichungen::einreichungs_vergleich,
            einreichungen::einreichung_exportieren,
            dateien::arbeitskopie_oeffnen,
            dateien::arbeitskopie_uebernehmen,
            bibliothek::formularbibliothek,
            bibliothek::bibliothek_formular,
            bibliothek::bibliothek_formular_speichern,
            bibliothek::bibliothek_felder_erstellen,
            commands::formular_im_projekt_speichern,
            bibliothek::formularordner_waehlen,
            bibliothek::bibliothek_importieren,
            bibliothek::bibliothek_uebernehmen,
            bibliothek::bibliothek_vorschau,
            bibliothek::bibliothek_veroeffentlichen,
            commands::ablage_status,
            commands::ablage_wechseln,
            commands::projektordner_exportieren,
            commands::projekte_auflisten,
            commands::projekt_anlegen,
            commands::projekt_aktualisieren,
            commands::projekt_loeschen,
            commands::projekt_bereich_speichern,
            commands::grundstueckskontext,
            commands::projekt_oeffnen,
            commands::editor_fenster_oeffnen,
            commands::projekt_laden,
            commands::projekt_recht,
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
            anmeldung::anmeldestatus,
            anmeldung::anmelden,
            anmeldung::anmelden_uebergang,
            anmeldung::kennwort_aendern,
            anmeldung::abmelden,
            anmeldung::erstkennwort_vergeben,
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
            commands::beteiligte,
            commands::beteiligte_setzen,
            commands::bauantrag,
            commands::bauantrag_speichern,
            emails::email_laden,
            emails::email_anhang_ablegen,
            emails::email_in_verfahren,
            druckausgabe::druckgestaltung,
            druckausgabe::druckgestaltung_speichern,
            kategorien::kategorienkatalog,
            kategorien::kategorienkatalog_speichern,
            kategorien::dokumentkategorien,
            kategorien::dokumentkategorie_speichern,
            kategorien::vorlagenkategorie,
            kategorien::vorlagenkategorie_speichern,
            commands::buero,
            commands::buero_speichern,
            commands::xbau_codelisten,
            commands::xbau_codeliste_einspielen,
            commands::formular_analysieren,
            commands::pdf_vorschau,
            commands::formular_livevorschau,
            commands::formular_vorlage_erstellen,
            commands::formular_ausfuellen,
            commands::cad_plan,
            commands::aufmass,
            commands::projekt_nutzung_pruefen,
            commands::projekt_baugrundstuecke_pruefen,
            commands::baugrundstueck_einzeln_speichern,
            commands::projekt_baugrundstuecke_gebaeude_einbinden,
            commands::baugrenzen_pruefen,
            commands::abstandsflaeche,
            abstandsflaechen::abstandsregeln,
            abstandsflaechen::abstandscad_konturen,
            abstandsflaechen::abstandsplan_pruefen,
            abstandsflaechen::abstandsnachweis_export,
            abstandsflaechen::abstandsumrisse_einbinden,
            abstandsflaechen::abstandscad_hoehen,
            abstandsflaechen::abstand_gelaende_vorschlag,
            berechnungen::berechnungsvorlagen,
            berechnungen::planaufmass_pruefen,
            berechnungen::planaufmass_diagnose,
            berechnungen::planaufmass_exportieren,
            berechnungen::planaufmass_flaechenoperation,
            berechnungen::planaufmass_ziele,
            berechnungen::planaufmass_einsetzen,
            berechnungen::planaufmass_raumbuch,
            berechnungen::planaufmass_abstand,
            berechnungen::planaufmass_abstandsziele,
            berechnungen::planaufmass_gebaeude,
            ifc::ifc_auszug,
            ifc::ifc_geschosse,
            ifc::ifc_grundriss,
            ifc::ifc_grundriss_plan,
            ifc::ifc_datei,
            berechnungen::planaufmass_gebaeudeziele,
            berechnungen::planaufmass_flurstuecke,
            berechnungen::planaufmass_abzuege,
            berechnungen::planaufmass_bauteile,
            berechnungen::planaufmass_baugrenzen,
            berechnungen::cadaufmass_uebernehmen,
            berechnungen::planaufmass_aufgaben,
            berechnungen::planaufmass_raumbuch_einsetzen,
            berechnungen::planaufmass_werkzeugkiste,
            berechnungen::berechnung_aus_raumbuch,
            berechnungen::fachberechnung_pruefen,
            updater::update_status,
            updater::update_pruefen,
            updater::update_installieren,
        ])
        .build(tauri::generate_context!())
        .expect("BauPlanIO konnte nicht gestartet werden")
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
