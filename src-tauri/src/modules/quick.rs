#[cfg(windows)]
fn start(target: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    std::process::Command::new("cmd")
        .args(["/C", "start", "", target])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_windows_settings(section: String) -> Result<(), String> {
    let uri = match section.as_str() {
        "wifi" => "ms-settings:network-wifi",
        "bluetooth" => "ms-settings:bluetooth",
        "focus" => "ms-settings:quietmomentshome",
        "nightlight" => "ms-settings:nightlight",
        "microphone" => "ms-settings:privacy-microphone",
        "brightness" => "ms-settings:display",
        "display" => "ms-settings:display",
        "battery" => "ms-settings:batterysaver",
        "screenshot" => "ms-screenclip:",
        _ => return Err("unknown settings section".to_string()),
    };

    #[cfg(windows)]
    {
        start(uri)
    }

    #[cfg(not(windows))]
    {
        let _ = uri;
        Err("quick settings hanya didukung di Windows".to_string())
    }
}

#[tauri::command]
pub async fn open_task_manager() -> Result<(), String> {
    #[cfg(windows)]
    {
        start("taskmgr")
    }

    #[cfg(not(windows))]
    {
        Err("Task Manager hanya didukung di Windows".to_string())
    }
}
