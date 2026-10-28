use std::time::Duration;

use tauri::{AppHandle, Emitter};

#[cfg(windows)]
fn active() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };

    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => {
            state == QUNS_RUNNING_D3D_FULL_SCREEN
                || state == QUNS_PRESENTATION_MODE
                || state == QUNS_BUSY
        }
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn active() -> bool {
    false
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = false;
        loop {
            let current = active();
            if current != last {
                last = current;
                crate::modules::window::set_cursor_tracking(!current);
                let _ = app.emit("fullscreen-changed", current);
            }
            std::thread::sleep(Duration::from_millis(1500));
        }
    });
}
