use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// One countdown at a time: the pill can only glance at a single deadline, and a list of
/// timers would need a page the island has no room for.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Done,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerInfo {
    pub status: TimerStatus,
    pub label: String,
    /// Requested length in ms, so the ring knows the size of a full sweep.
    pub total: i64,
    /// Deadline in epoch ms while running, `0` in every other state.
    pub ends_at: i64,
    /// Milliseconds left: live while running, frozen while paused.
    pub remaining: i64,
}

#[derive(Clone)]
pub struct Timer {
    status: TimerStatus,
    label: String,
    total: i64,
    /// Deadline while running, remaining ms while paused. Idle and done leave it unused.
    at: i64,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            status: TimerStatus::Idle,
            label: String::new(),
            total: 0,
            at: 0,
        }
    }
}

const MAX_MINUTES: i64 = 24 * 60;
const LABEL_MAX: usize = 28;
/// Interval between repeated alarm chimes while the timer sits in `Done`.
const ALARM_INTERVAL_MS: u64 = 3000;
/// Maximum number of alarm repeats before auto-silencing, so a forgotten timer
/// does not beep forever.
const ALARM_MAX_REPEATS: u32 = 20;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Play a short Windows system sound on the timer thread via user32 MessageBeep.
#[cfg(windows)]
pub(crate) fn system_beep() {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBeep(u_type: u32) -> i32;
    }
    // 0x30 = MB_ICONEXCLAMATION: brief, attention-grabbing system ding.
    unsafe {
        let _ = MessageBeep(0x30);
    }
}

#[cfg(not(windows))]
pub(crate) fn system_beep() {}

impl Timer {
    fn info(&self) -> TimerInfo {
        let (ends_at, remaining) = match self.status {
            TimerStatus::Running => {
                let left = (self.at - now_ms()).max(0);
                (self.at, left)
            }
            TimerStatus::Paused => (0, self.at.max(0)),
            _ => (0, 0),
        };
        TimerInfo {
            status: self.status,
            label: self.label.clone(),
            total: self.total,
            ends_at,
            remaining,
        }
    }

    fn start(&mut self, minutes: f64, label: &str) -> TimerInfo {
        let trimmed = label.trim();
        self.label = if trimmed.is_empty() {
            "Timer".to_string()
        } else {
            trimmed.chars().take(LABEL_MAX).collect()
        };
        let ms = (minutes * 60_000.0).round().clamp(1000.0, MAX_MINUTES as f64 * 60_000.0) as i64;
        self.total = ms;
        self.at = now_ms() + ms;
        self.status = TimerStatus::Running;
        self.info()
    }

    /// Switches the stored deadline into remaining milliseconds.
    fn pause(&mut self) -> TimerInfo {
        if self.status == TimerStatus::Running {
            self.at = (self.at - now_ms()).max(0);
            self.status = TimerStatus::Paused;
        }
        self.info()
    }

    fn resume(&mut self) -> TimerInfo {
        if self.status == TimerStatus::Paused {
            self.at = now_ms() + self.at.max(1000);
            self.status = TimerStatus::Running;
        }
        self.info()
    }

    fn finish(&mut self) -> TimerInfo {
        self.status = TimerStatus::Done;
        self.at = 0;
        self.info()
    }

    fn cancel(&mut self) -> TimerInfo {
        *self = Self::default();
        self.info()
    }

    fn add_time(&mut self, minutes: f64) -> TimerInfo {
        let ms = (minutes * 60_000.0).round() as i64;
        let max_ms = MAX_MINUTES * 60_000;
        if self.status == TimerStatus::Running {
            self.at = (self.at + ms).min(now_ms() + max_ms);
            self.total = (self.total + ms).clamp(1000, max_ms);
        } else if self.status == TimerStatus::Paused {
            self.at = (self.at + ms).clamp(1000, max_ms);
            self.total = (self.total + ms).clamp(1000, max_ms);
        }
        self.info()
    }
}

/// The deadline is watched here rather than in the webview so a timer still fires while the
/// island is hidden by the tray or covered by a fullscreen app. Sleeping 200 ms only while
/// something runs keeps an idle app at one wakeup per second.
///
/// When the countdown reaches zero the thread transitions to an alarm phase: it emits a
/// `timer-done` event once, then keeps emitting `timer-alarm` every [`ALARM_INTERVAL_MS`]
/// with a system beep until the user dismisses the timer (cancel/repeat moves it out of
/// `Done`) or [`ALARM_MAX_REPEATS`] is reached.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut running = false;
        let mut alarm_count: u32 = 0;
        loop {
            let sleep = if running {
                200
            } else if alarm_count > 0 {
                ALARM_INTERVAL_MS
            } else {
                1000
            };
            std::thread::sleep(Duration::from_millis(sleep));

            // --- Check for deadline ---
            let fired = {
                let state = app.state::<AppState>();
                let mut timer = state.timer.lock().expect("timer state poisoned");
                running = timer.status == TimerStatus::Running;
                if running && now_ms() >= timer.at {
                    alarm_count = 0;
                    Some(timer.finish())
                } else {
                    None
                }
            };

            if let Some(info) = fired {
                // A hidden island would swallow the alert entirely.
                crate::modules::window::reveal(&app);
                system_beep();
                let _ = app.emit("timer-done", info);
                alarm_count = 1;
                continue;
            }

            // --- Alarm repeat phase: keep chiming while `Done` ---
            if alarm_count > 0 {
                let still_done = {
                    let state = app.state::<AppState>();
                    let timer = state.timer.lock().expect("timer state poisoned");
                    timer.status == TimerStatus::Done
                };
                if still_done && alarm_count < ALARM_MAX_REPEATS {
                    alarm_count += 1;
                    system_beep();
                    let _ = app.emit("timer-alarm", alarm_count);
                } else {
                    // User dismissed or max reached — stop the alarm loop.
                    alarm_count = 0;
                }
            }
        }
    });
}

#[tauri::command]
pub async fn timer_status(app: AppHandle) -> TimerInfo {
    let state = app.state::<AppState>();
    let timer = state.timer.lock().expect("timer state poisoned");
    timer.info()
}

#[tauri::command]
pub async fn start_timer(app: AppHandle, minutes: f64, label: String) -> TimerInfo {
    let state = app.state::<AppState>();
    let mut timer = state.timer.lock().expect("timer state poisoned");
    timer.start(minutes, &label)
}

#[tauri::command]
pub async fn pause_timer(app: AppHandle) -> TimerInfo {
    let state = app.state::<AppState>();
    let mut timer = state.timer.lock().expect("timer state poisoned");
    timer.pause()
}

#[tauri::command]
pub async fn resume_timer(app: AppHandle) -> TimerInfo {
    let state = app.state::<AppState>();
    let mut timer = state.timer.lock().expect("timer state poisoned");
    timer.resume()
}

#[tauri::command]
pub async fn cancel_timer(app: AppHandle) -> TimerInfo {
    let state = app.state::<AppState>();
    let mut timer = state.timer.lock().expect("timer state poisoned");
    timer.cancel()
}

#[tauri::command]
pub async fn add_timer_time(app: AppHandle, minutes: f64) -> TimerInfo {
    let state = app.state::<AppState>();
    let mut timer = state.timer.lock().expect("timer state poisoned");
    timer.add_time(minutes)
}

#[tauri::command]
pub fn play_system_beep() {
    #[cfg(windows)]
    system_beep();
}
