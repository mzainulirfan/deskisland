use std::path::{Path, PathBuf};

use auto_launch::{AutoLaunch, WindowsEnableMode};
use tauri::{AppHandle, Emitter};

use crate::modules::tray;

fn launcher(app: &AppHandle, exe: &Path) -> Result<AutoLaunch, String> {
    let exe = exe
        .to_str()
        .ok_or_else(|| "Lokasi aplikasi tidak valid.".to_string())?;
    // The install folder contains a space, and Windows tokenizes an unquoted Run value,
    // so the path has to be quoted to survive logon.
    Ok(AutoLaunch::new(
        &app.package_info().name,
        &format!("\"{exe}\""),
        WindowsEnableMode::CurrentUser,
        &[] as &[&str],
    ))
}

fn current_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("Tidak bisa menemukan lokasi aplikasi: {e}"))
}

fn installed_exe() -> Option<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    let exe = root.join("Desk Island").join("desk-island.exe");
    exe.is_file().then_some(exe)
}

/// Dev builds embed no frontend and only run next to the Vite server, so autostart from a
/// dev build has to point at the installed release executable. A release build is its own
/// launch target.
fn launch_target() -> Result<PathBuf, String> {
    if !cfg!(debug_assertions) {
        return current_exe();
    }
    installed_exe().ok_or_else(|| {
        "Mulai otomatis baru tersedia setelah aplikasi dipasang. Jalankan installer dulu."
            .to_string()
    })
}

fn disable(app: &AppHandle) -> Result<(), String> {
    launcher(app, &current_exe()?)?
        .disable()
        .map_err(|e| format!("Gagal mematikan mulai otomatis: {e}"))
}

pub fn is_enabled(app: &AppHandle) -> bool {
    let Ok(exe) = current_exe() else {
        return false;
    };
    launcher(app, &exe)
        .ok()
        .and_then(|auto| auto.is_enabled().ok())
        .unwrap_or(false)
}

/// Logon must never start the debug binary that needs the Vite server. When autostart is
/// on and this is a dev build, the entry is rewritten to the installed executable — or
/// dropped when nothing installed exists. Release builds rewrite to themselves so a
/// moved install keeps working.
pub fn repair(app: &AppHandle) {
    if !is_enabled(app) {
        return;
    }
    let target = if cfg!(debug_assertions) {
        installed_exe()
    } else {
        current_exe().ok()
    };
    let _ = match target {
        Some(exe) => launcher(app, &exe).and_then(|auto| auto.enable().map_err(|e| e.to_string())),
        None => disable(app),
    };
}

/// Enables or disables the HKCU Run entry and keeps the tray checkbox and the open
/// settings page in sync. Returns the state the registry reports afterwards.
pub fn apply(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    if enabled {
        let target = launch_target()?;
        launcher(app, &target)?
            .enable()
            .map_err(|e| format!("Gagal mengaktifkan mulai otomatis: {e}"))?;
    } else {
        disable(app)?;
    }

    let actual = is_enabled(app);
    tray::sync_autostart(app, actual);
    let _ = app.emit("autostart-changed", actual);
    Ok(actual)
}

#[tauri::command]
pub async fn autostart_status(app: AppHandle) -> bool {
    is_enabled(&app)
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    apply(&app, enabled)
}
