mod modules;

use std::sync::Mutex;

use tauri::Emitter;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use modules::{
    apps, audio, autostart, downloads, fullscreen, keyboard, media, notifications, process, quick,
    reminders, settings, system, timer, tray, window,
};

pub struct AppState {
    system: Mutex<system::Monitor>,
    downloads: Mutex<downloads::Watcher>,
    timer: Mutex<timer::Timer>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if window::reveal(app) {
                            tray::sync_visibility(app, true);
                        }
                        let _ = app.emit("toggle-panel", ());
                    }
                })
                .build(),
        )
        .manage(AppState {
            system: Mutex::new(system::Monitor::new()),
            downloads: Mutex::new(downloads::Watcher::new()),
            timer: Mutex::new(timer::Timer::default()),
        })
        .manage(window::MoveGen::default())
        .setup(|app| {
            settings::init(&app.handle().clone());
            reminders::init(&app.handle().clone());
            keyboard::start(app.handle().clone());
            notifications::start(app.handle().clone());
            fullscreen::start(app.handle().clone());
            timer::start(app.handle().clone());
            reminders::start(app.handle().clone());
            tray::init(app)?;
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);
            let _ = app.global_shortcut().register(shortcut);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system::system_stats,
            process::top_processes,
            downloads::downloads_status,
            downloads::recent_downloads,
            downloads::open_download,
            downloads::reveal_download,
            downloads::open_downloads_folder,
            apps::list_app_shortcuts,
            apps::launch_app,
            apps::app_icon,
            apps::get_favorites,
            apps::set_favorites,
            notifications::recent_notifications,
            audio::master_volume,
            audio::set_master_volume,
            audio::set_master_mute,
            media::now_playing,
            media::media_control,
            timer::timer_status,
            timer::start_timer,
            timer::pause_timer,
            timer::resume_timer,
            timer::cancel_timer,
            timer::add_timer_time,
            timer::play_system_beep,
            reminders::get_reminders,
            reminders::save_reminders,
            reminders::preview_reminder,
            window::cursor_probe,
            window::fit_island_window,
            window::set_island_position,
            window::set_island_hover_region,
            window::start_cursor_monitor,
            window::set_island_passthrough,
            window::quit_app,
            autostart::autostart_status,
            autostart::set_autostart,
            settings::get_settings,
            settings::set_settings,
            settings::open_settings,
            quick::open_windows_settings,
            quick::open_task_manager
        ])
        .run(tauri::generate_context!())
        .expect("error while running Desk Island");
}
