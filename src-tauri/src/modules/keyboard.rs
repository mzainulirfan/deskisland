#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::OnceLock;
    use tauri::{AppHandle, Emitter};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    };

    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_SYSKEYDOWN: u32 = 0x0104;
    const WM_SYSKEYUP: u32 = 0x0105;

    static APP: OnceLock<AppHandle> = OnceLock::new();
    static HELD: AtomicU32 = AtomicU32::new(0);

    /// Whitelist only: the hook never inspects other keys, so typed text stays out of reach.
    fn watched(vk: u32) -> Option<(u32, &'static str)> {
        Some(match vk {
            0x09 => (1 << 0, "Tab"),
            0x0D => (1 << 1, "Enter"),
            0x5B | 0x5C => (1 << 2, "Win"),
            0x1B => (1 << 3, "Esc"),
            0x20 => (1 << 4, "Space"),
            0x08 => (1 << 5, "Backspace"),
            0x26 => (1 << 6, "↑"),
            0x28 => (1 << 7, "↓"),
            0x25 => (1 << 8, "←"),
            0x27 => (1 << 9, "→"),
            0x14 => (1 << 10, "Caps Lock"),
            _ => return None,
        })
    }

    unsafe extern "system" fn on_key(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            if let Some((bit, name)) = watched(info.vkCode) {
                let message = wparam.0 as u32;
                if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
                    // Auto-repeat keeps sending WM_KEYDOWN; only the first one is shown.
                    if HELD.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
                        if let Some(app) = APP.get() {
                            let _ = app.emit("key-hud", name);
                        }
                    }
                } else if message == WM_KEYUP || message == WM_SYSKEYUP {
                    HELD.fetch_and(!bit, Ordering::Relaxed);
                }
            }
        }
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    pub fn start(app: AppHandle) {
        let _ = APP.set(app);
        std::thread::spawn(|| unsafe {
            // Low-level hooks need a thread with its own message pump; this one lives for
            // the rest of the process, which also keeps the hook installed.
            let Ok(_hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(on_key), None, 0) else {
                return;
            };
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
    }
}

#[cfg(not(windows))]
mod imp {
    use tauri::AppHandle;

    pub fn start(_app: AppHandle) {}
}

pub use imp::start;
