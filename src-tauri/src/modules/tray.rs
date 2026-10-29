use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, Wry,
};

pub struct TrayMenu {
    visibility: MenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

pub fn init(app: &mut App) -> tauri::Result<()> {
    let autostart_enabled = crate::modules::autostart::is_enabled(app.handle());

    let panel = MenuItem::with_id(
        app,
        "panel",
        "Buka panel\tCtrl+Alt+Space",
        true,
        None::<&str>,
    )?;
    let visibility =
        MenuItem::with_id(app, "visibility", "Sembunyikan island", true, None::<&str>)?;
    let before_toggles = PredefinedMenuItem::separator(app)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Mulai otomatis",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Setelan…", true, None::<&str>)?;
    let before_quit = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &panel,
            &visibility,
            &before_toggles,
            &autostart,
            &settings,
            &before_quit,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id("island")
        .tooltip("Desk Island — klik untuk panel, klik kanan untuk menu")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "panel" => open_panel(app),
            "visibility" => toggle_visibility(app),
            "settings" => {
                // `open_settings` is async; the future has to be polled on the runtime
                // or the window is never built.
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::modules::settings::open_settings(app).await;
                });
            }
            "autostart" => {
                let enabled = crate::modules::autostart::is_enabled(app);
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
                // Clicking the tray icon is the usual way to bring an app forward; a
                // click that hid the island was the only way back in and surprised users.
                open_panel(tray.app_handle());
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

fn open_panel(app: &AppHandle) {
    if crate::modules::window::reveal(app) {
        sync_visibility(app, true);
    }
    let _ = app.emit("toggle-panel", ());
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
