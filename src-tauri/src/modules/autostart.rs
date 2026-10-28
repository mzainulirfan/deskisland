use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt;

use crate::modules::tray;

/// Enables or disables the HKCU Run entry and keeps the tray checkbox and the open
/// settings page in sync. Returns the state actually reported by the plugin.
pub fn apply(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };

    result.map_err(|e| e.to_string())?;

    let actual = manager.is_enabled().unwrap_or(enabled);
    tray::sync_autostart(app, actual);
    let _ = app.emit("autostart-changed", actual);
    Ok(actual)
}

#[tauri::command]
pub async fn autostart_status(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    apply(&app, enabled)
}
