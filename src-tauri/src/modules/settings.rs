use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::modules::apps::Favorite;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub anchor: String,
    /// Logical pixels between the top of the screen and the pill.
    pub top_margin: u32,
    pub idle: String,
    pub nav: String,
    pub greeting_mode: String,
    pub greeting_text: String,
    pub greeting_text2: String,
    pub greeting_show2: bool,
    /// Newline-separated greetings shown when the island comes back after being hidden.
    /// Each line is `judul` or `judul | pesan`.
    pub greeting_pool: String,
    pub mascot_scale: u32,
    pub radius_idle: u32,
    pub radius_panel: u32,
    pub welcome_ms: u64,
    pub morph_ms: u64,
    pub notifications: bool,
    pub hud_ms: u64,
    pub notice_ms: u64,
    pub poll_ms: u64,
    pub favorites: Vec<Favorite>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "dark".into(),
            anchor: "center".into(),
            top_margin: 16,
            idle: "eyes".into(),
            nav: "rail".into(),
            greeting_mode: "auto".into(),
            greeting_text: String::new(),
            greeting_text2: String::new(),
            greeting_show2: true,
            greeting_pool: String::new(),
            mascot_scale: 120,
            radius_idle: 25,
            radius_panel: 26,
            welcome_ms: 6000,
            morph_ms: 420,
            notifications: true,
            hud_ms: 1200,
            notice_ms: 5800,
            poll_ms: 1000,
            favorites: Vec::new(),
        }
    }
}

impl Settings {
    /// The file is user-editable, so unknown values and out-of-range timings are
    /// corrected on load and on every write. A zero `poll_ms` in particular would
    /// turn the frontend's setTimeout chain into a busy loop.
    fn sanitized(mut self) -> Self {
        if self.theme != "dark" && self.theme != "light" {
            self.theme = "dark".into();
        }
        if !matches!(self.anchor.as_str(), "left" | "center" | "right") {
            self.anchor = "center".into();
        }
        if !matches!(
            self.idle.as_str(),
            "auto" | "eyes" | "min" | "music" | "mon" | "timer" | "battery" | "notification"
        ) {
            self.idle = "eyes".into();
        }
        if !matches!(self.nav.as_str(), "rail" | "tabs") {
            self.nav = "rail".into();
        }
        // 0 pins the pill to the screen edge; past ~200 px the expanded panel starts to
        // run off short displays, and the island is meant to live at the top anyway.
        self.top_margin = self.top_margin.clamp(0, 200);
        if !matches!(self.greeting_mode.as_str(), "auto" | "custom") {
            self.greeting_mode = "auto".into();
        }
        self.greeting_text = self.greeting_text.trim().chars().take(60).collect();
        self.greeting_text2 = self.greeting_text2.trim().chars().take(40).collect();
        // Twelve rotating lines is already more than a user reads; a hand-edited file
        // with megabytes of lines would only slow every settings-changed emit down.
        self.greeting_pool = self
            .greeting_pool
            .lines()
            .take(12)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.chars().take(120).collect::<String>())
            .collect::<Vec<String>>()
            .join("\n");
        // 140% of the 34px base is the largest badge that still fits the 50px-tall pill.
        self.mascot_scale = self.mascot_scale.clamp(70, 140);
        // The idle pill is 50px tall, so 25px is a full stadium; the panel can stay
        // squarer because its corners frame a lot of text.
        self.radius_idle = self.radius_idle.clamp(8, 25);
        self.radius_panel = self.radius_panel.clamp(8, 40);
        // Long enough to type both lines and drain the ring, short enough to not linger.
        self.welcome_ms = self.welcome_ms.clamp(3500, 20000);
        self.morph_ms = self.morph_ms.clamp(150, 900);
        self.hud_ms = self.hud_ms.clamp(500, 3000);
        self.notice_ms = self.notice_ms.clamp(2000, 12000);
        self.poll_ms = self.poll_ms.clamp(500, 5000);
        self.favorites.truncate(12);
        self
    }
}

pub struct SettingsState {
    pub current: Mutex<Settings>,
    pub path: PathBuf,
}

pub fn init(app: &AppHandle) {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("settings.json");
    let current = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
        .unwrap_or_default()
        .sanitized();
    // The pill's first native placement happens in the frontend's onMount, so the margin
    // has to be cached before then.
    crate::modules::window::set_top_margin(current.top_margin);
    app.manage(SettingsState {
        current: Mutex::new(current),
        path,
    });
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> Settings {
    state.current.lock().unwrap().clone()
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, mut settings: Settings) -> Result<(), String> {
    // The settings window does not know about favorites; a payload that carries an
    // empty list is a round-trip of the other fields, not a request to clear them.
    // Favorites are managed exclusively through apps::set_favorites.
    if settings.favorites.is_empty() {
        let existing = app.state::<SettingsState>();
        settings.favorites = existing.current.lock().unwrap().favorites.clone();
    }
    let settings = settings.sanitized();
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    let state = app.state::<SettingsState>();
    // Write first, then publish. Emitting before the write told the island about a
    // setting the disk never received, so a failed write left UI and file diverged with
    // no way for either side to notice.
    fs::write(&state.path, bytes).map_err(|e| e.to_string())?;
    crate::modules::window::set_top_margin(settings.top_margin);
    *state.current.lock().unwrap() = settings.clone();
    app.emit("settings-changed", &settings).map_err(|e| e.to_string())
}

/// Shows the settings window, creating it on first use and restoring it afterwards.
#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("index.html".into()))
        .initialization_script("window.__DESK_VIEW__ = 'settings';")
        .title("Desk Island — Setelan")
        // The page is laid out in two columns and never scrolls: the webview re-fits its
        // own height to the content, so only the width is really up to the user.
        .inner_size(820.0, 460.0)
        .min_inner_size(660.0, 420.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}
