use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, Wry,
};
use tauri_plugin_autostart::ManagerExt;

pub struct TrayMenu {
    visibility: MenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

pub fn init(app: &mut App) -> tauri::Result<()> {
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let visibility =
        MenuItem::with_id(app, "visibility", "Sembunyikan island", true, None::<&str>)?;
    let panel = MenuItem::with_id(
        app,
        "panel",
        "Buka panel (Ctrl+Alt+Space)",
        true,
        None::<&str>,
    )?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Mulai otomatis",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "Setelan…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &visibility,
            &panel,
            &autostart,
            &separator,
            &settings,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("island")
        .tooltip("Desk Island")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "visibility" => toggle_visibility(app),
            "panel" => {
                if crate::modules::window::reveal(app) {
                    sync_visibility(app, true);
                }
                let _ = app.emit("toggle-panel", ());
            }
            "settings" => {
                let _ = crate::modules::settings::open_settings(app.clone());
            }
            "autostart" => {
                let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                let _ = crate::modules::autostart::apply(app, !enabled);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_visibility(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    app.manage(TrayMenu {
        visibility,
        autostart,
    });
    Ok(())
}

pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.autostart.set_checked(enabled);
    }
}

pub fn sync_visibility(app: &AppHandle, visible: bool) {
    if let Some(menu) = app.try_state::<TrayMenu>() {
        let _ = menu.visibility.set_text(if visible {
            "Sembunyikan island"
        } else {
            "Tampilkan island"
        });
    }
}

fn toggle_visibility(app: &AppHandle) {
    let Some(window) = app.get_webview_window("island") else {
        return;
    };

    let visible = window.is_visible().unwrap_or(true);
    if visible {
        let _ = window.hide();
    } else {
        // Goes through `reveal` so the webview is notified and can greet again.
        crate::modules::window::reveal(app);
    }
    crate::modules::window::set_cursor_tracking(!visible);
    sync_visibility(app, !visible);
}
