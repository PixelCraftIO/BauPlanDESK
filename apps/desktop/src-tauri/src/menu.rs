//! Native Menüleiste (Datei, Bearbeiten, Ansicht, Berechnung, Hilfe).
//!
//! Menüpunkte, die Dialoge im Frontend öffnen, werden als Event `menu` mit
//! der Menü-ID an das Hauptfenster weitergereicht. Rein native Aktionen
//! (Zwischenablage, Vollbild, Zoom, Beenden) erledigt das Betriebssystem bzw.
//! dieses Modul selbst.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::menu::{
    AboutMetadataBuilder, Menu, MenuEvent, MenuItem, MenuItemBuilder, PredefinedMenuItem, Submenu,
    SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, Wry};

pub mod id {
    pub const NEUES_PROJEKT: &str = "datei.neu";
    pub const PROJEKT_OEFFNEN: &str = "datei.oeffnen";
    pub const PROJEKT_SCHLIESSEN: &str = "datei.schliessen";
    pub const ZOOM_PLUS: &str = "ansicht.zoom_plus";
    pub const ZOOM_MINUS: &str = "ansicht.zoom_minus";
    pub const ZOOM_RESET: &str = "ansicht.zoom_reset";
    pub const UPDATES: &str = "hilfe.updates";
    pub const BEENDEN: &str = "datei.beenden";
    /// Nur im Netzwerkbetrieb (Anmeldung mit Kennwort).
    pub const ABMELDEN: &str = "anwendung.abmelden";
    pub const RUECKGAENGIG: &str = "bearbeiten.rueckgaengig";
    pub const WIEDERHOLEN: &str = "bearbeiten.wiederholen";
    pub const ZULETZT_PREFIX: &str = "zuletzt:";
    pub const ZULETZT_LEEREN: &str = "zuletzt.leeren";
}

const MAX_ZULETZT: usize = 8;

/// Zuletzt geöffnete Projekte, gespeichert in `<app_config_dir>/zuletzt.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZuletztListe(pub Vec<ZuletztEintrag>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZuletztEintrag {
    pub id: String,
    pub name: String,
}

impl ZuletztListe {
    pub fn laden(pfad: &std::path::Path) -> Self {
        std::fs::read(pfad)
            .ok()
            .and_then(|raw| serde_json::from_slice(&raw).ok())
            .unwrap_or_default()
    }

    pub fn speichern(&self, pfad: &std::path::Path) {
        if let Ok(raw) = serde_json::to_vec_pretty(self)
            && let Err(e) = std::fs::write(pfad, raw)
        {
            tracing::warn!(fehler = %e, "Liste zuletzt geöffneter Projekte nicht gespeichert");
        }
    }

    /// Setzt `eintrag` an den Anfang (ohne Duplikate, höchstens 8 Einträge).
    pub fn vorne_einfuegen(&mut self, eintrag: ZuletztEintrag) {
        self.0.retain(|e| e.id != eintrag.id);
        self.0.insert(0, eintrag);
        self.0.truncate(MAX_ZULETZT);
    }

    pub fn entfernen(&mut self, id: &str) {
        self.0.retain(|e| e.id != id);
    }
}

/// Zustand der Menüleiste, der zur Laufzeit geändert wird.
pub struct MenuState {
    /// Nur bei geöffnetem Projekt aktiv.
    projekt_items: Vec<MenuItem<Wry>>,
    zuletzt_menu: Submenu<Wry>,
    pub zuletzt: Mutex<ZuletztListe>,
    zuletzt_pfad: PathBuf,
    zoom: Mutex<f64>,
}

impl MenuState {
    pub fn set_projekt_offen(&self, offen: bool) {
        for item in &self.projekt_items {
            let _ = item.set_enabled(offen);
        }
    }

    /// Projekt an den Anfang der Liste "Zuletzt geöffnet" setzen.
    pub fn merke_projekt(&self, app: &AppHandle, eintrag: ZuletztEintrag) {
        let mut liste = self.zuletzt.lock().expect("Menü-Lock vergiftet");
        liste.vorne_einfuegen(eintrag);
        liste.speichern(&self.zuletzt_pfad);
        self.zuletzt_neu_aufbauen(app, &liste);
    }

    /// Z. B. wenn ein Projekt nicht mehr existiert.
    pub fn vergiss_projekt(&self, app: &AppHandle, id: &str) {
        let mut liste = self.zuletzt.lock().expect("Menü-Lock vergiftet");
        liste.entfernen(id);
        liste.speichern(&self.zuletzt_pfad);
        self.zuletzt_neu_aufbauen(app, &liste);
    }

    fn zuletzt_leeren(&self, app: &AppHandle) {
        let mut liste = self.zuletzt.lock().expect("Menü-Lock vergiftet");
        liste.0.clear();
        liste.speichern(&self.zuletzt_pfad);
        self.zuletzt_neu_aufbauen(app, &liste);
    }

    fn zuletzt_neu_aufbauen(&self, app: &AppHandle, liste: &ZuletztListe) {
        if let Err(e) = fuelle_zuletzt(app, &self.zuletzt_menu, liste) {
            tracing::warn!(fehler = %e, "Menü 'Zuletzt geöffnet' nicht aktualisiert");
        }
    }

    fn zoom_aendern(&self, app: &AppHandle, aenderung: Option<f64>) {
        let mut zoom = self.zoom.lock().expect("Menü-Lock vergiftet");
        *zoom = match aenderung {
            Some(delta) => (*zoom + delta).clamp(0.5, 2.0),
            None => 1.0,
        };
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_zoom(*zoom);
        }
    }
}

fn fuelle_zuletzt<R: Runtime>(
    app: &AppHandle<R>,
    menu: &Submenu<R>,
    liste: &ZuletztListe,
) -> tauri::Result<()> {
    for item in menu.items()? {
        menu.remove(&item)?;
    }
    if liste.0.is_empty() {
        menu.append(&MenuItemBuilder::new("(keine)").enabled(false).build(app)?)?;
        return Ok(());
    }
    for (i, eintrag) in liste.0.iter().enumerate() {
        let mut builder = MenuItemBuilder::with_id(
            format!("{}{}", id::ZULETZT_PREFIX, eintrag.id),
            &eintrag.name,
        );
        if i < 9 {
            builder = builder.accelerator(format!("CmdOrCtrl+Alt+{}", i + 1));
        }
        menu.append(&builder.build(app)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItemBuilder::with_id(id::ZULETZT_LEEREN, "Liste leeren").build(app)?)?;
    Ok(())
}

/// Baut die Menüleiste und registriert sie samt Zustand an der App.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str, acc: Option<&str>, enabled: bool| {
        let mut b = MenuItemBuilder::with_id(id, text).enabled(enabled);
        if let Some(acc) = acc {
            b = b.accelerator(acc);
        }
        b.build(app)
    };

    // --- Datei ---
    let neu = item(
        id::NEUES_PROJEKT,
        "Neues Projekt …",
        Some("CmdOrCtrl+N"),
        true,
    )?;
    let oeffnen = item(
        id::PROJEKT_OEFFNEN,
        "Projekt öffnen …",
        Some("CmdOrCtrl+O"),
        true,
    )?;
    let schliessen = item(
        id::PROJEKT_SCHLIESSEN,
        "Projekt schließen",
        Some("CmdOrCtrl+W"),
        false,
    )?;
    let speichern = item(
        "anwendung.speichern",
        "Projekt speichern",
        Some("CmdOrCtrl+S"),
        false,
    )?;
    let zuletzt_menu = SubmenuBuilder::new(app, "Zuletzt geöffnet").build()?;

    let mut datei = SubmenuBuilder::new(app, "&Anwendung")
        .item(&neu)
        .item(&oeffnen)
        .item(&zuletzt_menu)
        .separator()
        .item(&schliessen)
        .item(&speichern)
        .separator()
        .item(&item(
            "anwendung.einstellungen",
            "Einstellungen …",
            None,
            true,
        )?);
    if app
        .state::<crate::bootstrap::AppState>()
        .kennwortanmeldung()
    {
        // Das Frontend prüft offene Eingaben und zeigt danach den Anmeldedialog.
        datei = datei.separator().item(&item(
            id::ABMELDEN,
            "Abmelden / Benutzer wechseln …",
            None,
            true,
        )?);
    }
    // Unter macOS steht "Beenden" im App-Menü. Unter Windows/Linux ein
    // eigener Eintrag, weil das vordefinierte "Quit" unter Linux fehlt.
    if !cfg!(target_os = "macos") {
        datei = datei
            .separator()
            .item(&item(id::BEENDEN, "Beenden", Some("CmdOrCtrl+Q"), true)?);
    }
    let datei = datei.build()?;

    // --- Bearbeiten ---
    let mut bearbeiten = SubmenuBuilder::new(app, "&Bearbeiten");
    bearbeiten = if cfg!(target_os = "linux") {
        // GTK kennt kein vordefiniertes Rückgängig/Wiederholen; das Frontend
        // führt es im fokussierten Eingabefeld aus.
        bearbeiten
            .item(&item(
                id::RUECKGAENGIG,
                "Rückgängig",
                Some("CmdOrCtrl+Z"),
                true,
            )?)
            .item(&item(
                id::WIEDERHOLEN,
                "Wiederholen",
                Some("CmdOrCtrl+Shift+Z"),
                true,
            )?)
    } else {
        bearbeiten
            .undo_with_text("Rückgängig")
            .redo_with_text("Wiederholen")
    };
    let bearbeiten = bearbeiten
        .separator()
        .cut_with_text("Ausschneiden")
        .copy_with_text("Kopieren")
        .paste_with_text("Einfügen")
        .select_all_with_text("Alles auswählen")
        .build()?;

    // --- Ansicht ---
    let ansicht = SubmenuBuilder::new(app, "&Fenster")
        .item(&item(
            id::ZOOM_PLUS,
            "Vergrößern",
            Some("CmdOrCtrl+="),
            true,
        )?)
        .item(&item(
            id::ZOOM_MINUS,
            "Verkleinern",
            Some("CmdOrCtrl+-"),
            true,
        )?)
        .item(&item(
            id::ZOOM_RESET,
            "Originalgröße",
            Some("CmdOrCtrl+0"),
            true,
        )?)
        .separator()
        .fullscreen_with_text("Vollbild")
        .build()?;

    // --- Hilfe ---
    let info = app.package_info();
    let about = AboutMetadataBuilder::new()
        .name(Some(info.name.clone()))
        .version(Some(info.version.to_string()))
        .copyright(Some("© PixelCraftIO"))
        .comments(Some("Bauanträge erstellen, berechnen und einreichen"))
        .build();
    let mut hilfe = SubmenuBuilder::new(app, "&Hilfe").item(&item(
        id::UPDATES,
        "Nach Updates suchen …",
        None,
        true,
    )?);
    if !cfg!(target_os = "macos") {
        hilfe = hilfe
            .separator()
            .about_with_text("Über BauProjektIO", Some(about.clone()));
    }
    let hilfe = hilfe.build()?;

    let menu = Menu::new(app)?;
    if cfg!(target_os = "macos") {
        // macOS erwartet als erstes Menü das App-Menü.
        let app_menu = SubmenuBuilder::new(app, &info.name)
            .about(Some(about))
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .item(&item(
                id::BEENDEN,
                "BauProjektIO beenden",
                Some("CmdOrCtrl+Q"),
                true,
            )?)
            .build()?;
        menu.append(&app_menu)?;
    }
    for submenu in [&datei, &bearbeiten, &ansicht, &hilfe] {
        menu.append(submenu)?;
    }
    app.set_menu(menu)?;

    let zuletzt_pfad = app
        .state::<crate::bootstrap::AppState>()
        .config_dir
        .join("zuletzt.json");
    let zuletzt = ZuletztListe::laden(&zuletzt_pfad);
    fuelle_zuletzt(app, &zuletzt_menu, &zuletzt)?;

    app.manage(MenuState {
        projekt_items: vec![schliessen, speichern],
        zuletzt_menu,
        zuletzt: Mutex::new(zuletzt),
        zuletzt_pfad,
        zoom: Mutex::new(1.0),
    });
    app.on_menu_event(on_menu_event);
    Ok(())
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let state = app.state::<MenuState>();
    match event.id().as_ref() {
        id::ZOOM_PLUS => state.zoom_aendern(app, Some(0.1)),
        id::ZOOM_MINUS => state.zoom_aendern(app, Some(-0.1)),
        id::ZOOM_RESET => state.zoom_aendern(app, None),
        id::ZULETZT_LEEREN => state.zuletzt_leeren(app),
        id::BEENDEN => {
            // Auch Menü/Shortcut müssen den Entwurfs- und Installationsschutz
            // des Hauptfensters durchlaufen; kein unbedingtes app.exit().
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
        // Alles andere entscheidet das Frontend (Dialoge, Ansichten).
        other => {
            let _ = app.emit_to("main", "menu", other.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str) -> ZuletztEintrag {
        ZuletztEintrag {
            id: id.into(),
            name: format!("Projekt {id}"),
        }
    }

    #[test]
    fn zuletzt_ohne_duplikate_und_begrenzt() {
        let mut liste = ZuletztListe::default();
        for i in 0..12 {
            liste.vorne_einfuegen(e(&i.to_string()));
        }
        liste.vorne_einfuegen(e("5"));
        assert_eq!(liste.0.len(), MAX_ZULETZT);
        assert_eq!(liste.0[0].id, "5");
        assert_eq!(liste.0.iter().filter(|x| x.id == "5").count(), 1);
        liste.entfernen("5");
        assert!(liste.0.iter().all(|x| x.id != "5"));
    }
}
