use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::modules::{timer, window};

/// A wall-clock alarm the user owns: fired by this thread rather than the webview so it
/// still goes off while the island is hidden by the tray.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Reminder {
    /// Assigned by [`normalize`] when the incoming id is 0 or already taken.
    pub id: u32,
    pub text: String,
    /// Local 24-hour wall clock, `HH:MM`. Minute resolution is all an alarm needs.
    pub time: String,
    /// Bitmask of weekdays, bit 0 = Monday .. bit 6 = Sunday.
    pub days: u32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderEvent {
    pub id: u32,
    pub text: String,
    pub time: String,
}

/// Same cap as favorites: a pill that scrolls forever is a settings screen, not an island.
const MAX_REMINDERS: usize = 12;
const TEXT_MAX: usize = 40;
const DAY_MASK: u32 = 0b0111_1111;
const EVERY_DAY: u32 = DAY_MASK;
/// A minute is 60 s wide, so a 2 s tick cannot miss or double one.
const TICK: Duration = Duration::from_millis(2000);

fn parse_minutes(time: &str) -> Option<u32> {
    let (hour, minute) = time.trim().split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 {
        return None;
    }
    let hour: u32 = hour.parse().ok()?;
    let minute: u32 = minute.parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some(hour * 60 + minute)
}

struct Schedule {
    reminders: Vec<Reminder>,
    next_id: u32,
    /// `YYYY-MM-DD#id` keys already fired, dropped when the local date rolls over so the
    /// list stays bounded and a restart inside the same minute cannot double-fire.
    fired: HashSet<String>,
}

impl Schedule {
    fn new(reminders: Vec<Reminder>) -> Self {
        let mut schedule = Schedule {
            reminders: Vec::new(),
            next_id: 1,
            fired: HashSet::new(),
        };
        normalize(&mut schedule, reminders);
        schedule
    }
}

pub struct RemindersState {
    current: Mutex<Schedule>,
    path: PathBuf,
}

/// Replaces the list with a corrected copy of `incoming`, assigning ids to the entries that
/// arrive without one. An entry with an unknown id is a new one: the settings file is
/// user-editable, so a stale id is not trusted over a fresh item.
fn normalize(schedule: &mut Schedule, incoming: Vec<Reminder>) {
    let mut next_id = schedule.next_id;
    let mut reminders: Vec<Reminder> = Vec::with_capacity(incoming.len().min(MAX_REMINDERS));
    for mut reminder in incoming.into_iter().take(MAX_REMINDERS) {
        let text = reminder.text.trim();
        reminder.text = if text.is_empty() {
            "Pengingat".to_string()
        } else {
            text.chars().take(TEXT_MAX).collect()
        };
        if parse_minutes(&reminder.time).is_none() {
            reminder.time = "09:00".to_string();
        }
        reminder.days &= DAY_MASK;
        if reminder.days == 0 {
            reminder.days = EVERY_DAY;
        }
        if reminder.id == 0 || reminders.iter().any(|kept| kept.id == reminder.id) {
            while schedule.reminders.iter().any(|kept| kept.id == next_id)
                || reminders.iter().any(|kept| kept.id == next_id)
            {
                next_id += 1;
            }
            reminder.id = next_id;
        }
        reminders.push(reminder);
    }
    schedule.next_id = schedule
        .reminders
        .iter()
        .filter(|kept| !reminders.iter().any(|next| next.id == kept.id))
        .map(|kept| kept.id + 1)
        .max()
        .map_or(next_id + 1, |high| high.max(next_id + 1));
    schedule.reminders = reminders;
}

/// Local date key, minute of the day, and the weekday bit for right now.
#[cfg(windows)]
fn wall_clock() -> (String, u32, u32) {
    use windows::Win32::System::SystemInformation::GetLocalTime;

    let time = unsafe { GetLocalTime() };
    // `wDayOfWeek` is 0 for Sunday, the list is Monday-first, so Sunday wraps to bit 6.
    let day_bit = 1u32 << ((time.wDayOfWeek as u32 + 6) % 7);
    (
        format!("{:04}-{:02}-{:02}", time.wYear, time.wMonth, time.wDay),
        time.wHour as u32 * 60 + time.wMinute as u32,
        day_bit,
    )
}

/// Not the target platform, so UTC is close enough to keep the module testable elsewhere.
#[cfg(not(windows))]
fn wall_clock() -> (String, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = seconds / 86_400;
    // 1970-01-01 was a Thursday, which is bit 3 of a Monday-first week.
    let day_bit = 1u32 << ((days as u32 + 3) % 7);
    let (year, month, day) = civil_from_days(days);
    (format!("{year:04}-{month:02}-{day:02}"), (seconds % 86_400) as u32 / 60, day_bit)
}

#[cfg(not(windows))]
fn civil_from_days(days_since_epoch: u64) -> (u64, u64, u64) {
    // Howard Hinnant's `civil_from_days`, so the date key does not need a calendar crate.
    let z = days_since_epoch + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// One tick of the watcher: the reminders due at this local minute that have not already
/// gone off today.
fn due(schedule: &mut Schedule) -> Vec<ReminderEvent> {
    let (date, minute, day_bit) = wall_clock();
    if schedule.fired.iter().any(|key| !key.starts_with(&date)) {
        schedule.fired.clear();
    }
    let mut events = Vec::new();
    for reminder in &schedule.reminders {
        if !reminder.enabled || reminder.days & day_bit == 0 {
            continue;
        }
        if parse_minutes(&reminder.time) != Some(minute) {
            continue;
        }
        let key = format!("{date}#{}", reminder.id);
        if !schedule.fired.insert(key) {
            continue;
        }
        events.push(ReminderEvent {
            id: reminder.id,
            text: reminder.text.clone(),
            time: reminder.time.clone(),
        });
    }
    events
}

/// Loaded and watched from disk before the first pill paint, mirroring `settings::init`.
pub fn init(app: &AppHandle) {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("reminders.json");
    let stored = fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<Reminder>>(&bytes).ok())
        .unwrap_or_default();
    app.manage(RemindersState {
        current: Mutex::new(Schedule::new(stored)),
        path,
    });
}

/// A hidden island would swallow the alert, and the chime alone is easy to miss on a
/// quiet machine, so this mirrors what the timer deadline thread does.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(TICK);
        let events = {
            let state = app.state::<RemindersState>();
            let mut schedule = state.current.lock().expect("reminders state poisoned");
            due(&mut schedule)
        };
        for event in events {
            window::reveal(&app);
            timer::system_beep();
            let _ = app.emit("reminder-fired", event);
        }
    });
}

fn persist(state: &RemindersState) -> Result<(), String> {
    let schedule = state.current.lock().expect("reminders state poisoned");
    let bytes = serde_json::to_vec_pretty(&schedule.reminders).map_err(|e| e.to_string())?;
    fs::write(&state.path, bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_reminders(state: State<'_, RemindersState>) -> Vec<Reminder> {
    state
        .current
        .lock()
        .expect("reminders state poisoned")
        .reminders
        .clone()
}

/// Whole-list replacement, like `set_favorites`: the page owns the editing and this owns the
/// ids and the file. The normalized list comes back so the store adopts the ids it assigned.
#[tauri::command]
pub async fn save_reminders(app: AppHandle, reminders: Vec<Reminder>) -> Result<Vec<Reminder>, String> {
    let state = app.state::<RemindersState>();
    let saved = {
        let mut schedule = state.current.lock().expect("reminders state poisoned");
        normalize(&mut schedule, reminders);
        schedule.reminders.clone()
    };
    persist(&state)?;
    Ok(saved)
}

/// Fires `reminder` as if its minute had just arrived, for the panel's test button.
#[tauri::command]
pub async fn preview_reminder(app: AppHandle, id: u32) -> Result<ReminderEvent, String> {
    let event = {
        let state = app.state::<RemindersState>();
        let schedule = state.current.lock().expect("reminders state poisoned");
        let reminder = schedule
            .reminders
            .iter()
            .find(|reminder| reminder.id == id)
            .ok_or_else(|| "pengingat tidak ditemukan".to_string())?;
        ReminderEvent {
            id: reminder.id,
            text: reminder.text.clone(),
            time: reminder.time.clone(),
        }
    };
    window::reveal(&app);
    timer::system_beep();
    app.emit("reminder-fired", event.clone())
        .map_err(|e| e.to_string())?;
    Ok(event)
}
