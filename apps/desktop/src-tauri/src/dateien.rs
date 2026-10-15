//! Native Dateiassoziation und lokale Arbeitskopien; DMS-Übernahme im Core.
use crate::{bootstrap::AppState, commands::CmdError};
use core_lib::{
    CoreError,
    benutzer::Recht,
    domain::{DocumentId, ProjectId},
};
use std::path::{Path, PathBuf};
use storage::arbeitskopie::{self, Arbeitskopie};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;
fn root(app: &AppHandle) -> Result<PathBuf, CmdError> {
    Ok(app.state::<AppState>().data_dir.join("Arbeitskopien"))
}
fn starten(pfad: &Path) -> Result<(), CoreError> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn ShellExecuteW(
                hwnd: isize,
                op: *const u16,
                file: *const u16,
                params: *const u16,
                dir: *const u16,
                show: i32,
            ) -> isize;
        }
        let path: Vec<u16> = pfad.as_os_str().encode_wide().chain(Some(0)).collect();
        let op: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        #[link(name = "ole32")]
        unsafe extern "system" {
            fn CoInitializeEx(reserved: *mut std::ffi::c_void, flags: u32) -> i32;
            fn CoUninitialize();
        }
        // Shell extensions may need COM on this worker thread. Balance every
        // successful initialization, including S_FALSE, before returning.
        let com = unsafe { CoInitializeEx(std::ptr::null_mut(), 2 | 4) };
        if com < 0 {
            return Err(CoreError::Storage(format!(
                "Windows-Dateizuordnung konnte nicht initialisiert werden ({com})"
            )));
        }
        let code = unsafe {
            let result = ShellExecuteW(
                0,
                op.as_ptr(),
                path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
            CoUninitialize();
            result
        };
        if code <= 32 {
            return Err(CoreError::Storage(format!(
                "Kein zugeordnetes Programm konnte gestartet werden (Windows: {code})"
            )));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        let app = "/usr/bin/open";
        #[cfg(not(target_os = "macos"))]
        let app = "xdg-open";
        let mut child = std::process::Command::new(app)
            .arg(pfad)
            .spawn()
            .map_err(|e| CoreError::Storage(format!("Zugeordnetes Programm: {e}")))?;
        // Some Linux associations keep the dispatcher alive for the viewer's
        // entire lifetime. The UI must still permit reviewing/importing changes.
        for _ in 0..20 {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| CoreError::Storage(e.to_string()))?
            {
                return if status.success() {
                    Ok(())
                } else {
                    Err(CoreError::Storage(
                        "Kein zugeordnetes Programm konnte gestartet werden".into(),
                    ))
                };
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}
#[tauri::command]
pub async fn dokument_extern_oeffnen(
    app: AppHandle,
    state: State<'_, AppState>,
    dokument_id: DocumentId,
    version: i64,
) -> Result<Arbeitskopie, CmdError> {
    let (doc, bytes) = state.core.externe_datei(dokument_id, version).await?;
    let copy = arbeitskopie::erstellen(&root(&app)?, doc, &bytes).await?;
    let pfad = copy.pfad.clone();
    tauri::async_runtime::spawn_blocking(move || starten(&pfad))
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
    Ok(copy)
}
#[tauri::command]
pub async fn arbeitskopien(
    app: AppHandle,
    projekt_id: ProjectId,
) -> Result<Vec<Arbeitskopie>, CmdError> {
    Ok(arbeitskopie::auflisten(&root(&app)?, projekt_id).await?)
}
#[tauri::command]
pub async fn arbeitskopie_oeffnen(app: AppHandle, id: Uuid) -> Result<(), CmdError> {
    let copy = arbeitskopie::laden(&root(&app)?, id).await?;
    core_lib::dateiverwaltung::extern_oeffnen_erlaubt(&copy.stand.dokument.file_name)?;
    tauri::async_runtime::spawn_blocking(move || starten(&copy.pfad))
        .await
        .map_err(|e| CoreError::Storage(e.to_string()))??;
    Ok(())
}
#[tauri::command]
pub async fn arbeitskopie_uebernehmen(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Uuid,
) -> Result<crate::projektordner::ProjektDokument, CmdError> {
    state
        .core
        .berechtigung_pruefen(&state.benutzer(), Recht::DokumentAblegen)
        .await?;
    let copy = arbeitskopie::laden(&root(&app)?, id).await?;
    if !copy.geaendert {
        return Err(CoreError::InvalidInput("Arbeitskopie ist unverändert".into()).into());
    }
    let data = arbeitskopie::inhalt(&copy.pfad).await?;
    let d = &copy.stand.dokument;
    let gespeichert = state
        .core
        .store_document(
            d.project_id,
            Some(d.document_id),
            Some(d.version),
            &d.file_name,
            data.into(),
            &state.benutzer(),
        )
        .await?;
    arbeitskopie::basis_aktualisieren(&root(&app)?, id, gespeichert.clone()).await.map_err(|e|CoreError::Storage(format!("Dokument bereits als Version {} gespeichert, lokale Basis konnte nicht aktualisiert werden: {e}",gespeichert.version)))?;
    Ok(crate::projektordner::ablegen(&state, gespeichert).await)
}
