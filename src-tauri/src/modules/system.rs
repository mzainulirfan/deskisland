use std::time::Instant;

use serde::Serialize;
use sysinfo::{Disks, Networks, System};

use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu: f32,
    pub ram: f32,
    pub ram_used: u64,
    pub ram_total: u64,
    pub rx: f64,
    pub tx: f64,
    pub disk: f32,
    pub disk_used: u64,
    pub disk_total: u64,
    pub battery: Option<Battery>,
}

pub struct Monitor {
    pub(crate) system: System,
    networks: Networks,
    disks: Disks,
    sampled_at: Instant,
}

impl Monitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        // CPU usage is reported as a delta between two samples, so take the baseline now.
        system.refresh_cpu_usage();

        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);

        Self {
            system,
            networks,
            disks: Disks::new_with_refreshed_list(),
            sampled_at: Instant::now(),
        }
    }

    pub fn sample(&mut self) -> SystemStats {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.networks.refresh(true);
        self.disks.refresh(true);

        let elapsed = self.sampled_at.elapsed().as_secs_f64().max(0.25);
        self.sampled_at = Instant::now();

        let (mut rx, mut tx) = (0_u64, 0_u64);
        for data in self.networks.values() {
            rx += data.received();
            tx += data.transmitted();
        }

        let used = self.system.used_memory();
        let total = self.system.total_memory();

        let (mut disk_total, mut disk_available) = (0_u64, 0_u64);
        for disk in self.disks.list() {
            if disk.is_removable() || disk.total_space() == 0 {
                continue;
            }
            disk_total += disk.total_space();
            disk_available += disk.available_space();
        }
        let disk_used = disk_total.saturating_sub(disk_available);

        SystemStats {
            cpu: self.system.global_cpu_usage(),
            ram: if total > 0 {
                (used as f64 / total as f64 * 100.0) as f32
            } else {
                0.0
            },
            ram_used: used,
            ram_total: total,
            rx: rx as f64 / elapsed,
            tx: tx as f64 / elapsed,
            disk: if disk_total > 0 {
                (disk_used as f64 / disk_total as f64 * 100.0) as f32
            } else {
                0.0
            },
            disk_used,
            disk_total,
            battery: battery(),
        }
    }
}

#[cfg(windows)]
mod power {
    #[repr(C)]
    struct SystemPowerStatus {
        ac_line_status: u8,
        battery_flag: u8,
        battery_life_percent: u8,
        _reserved: u8,
        battery_life_time: u32,
        battery_full_life_time: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
    }

    const NO_BATTERY: u8 = 0x80;
    const CHARGING: u8 = 0x08;

    pub fn query() -> Option<(u8, bool)> {
        let mut status = SystemPowerStatus {
            ac_line_status: 0,
            battery_flag: NO_BATTERY,
            battery_life_percent: 255,
            _reserved: 0,
            battery_life_time: 0,
            battery_full_life_time: 0,
        };

        if unsafe { GetSystemPowerStatus(&mut status) } == 0
            || status.battery_flag & NO_BATTERY != 0
            || status.battery_life_percent == 255
        {
            return None;
        }

        Some((
            status.battery_life_percent,
            status.battery_flag & CHARGING != 0,
        ))
    }
}

#[cfg(not(windows))]
mod power {
    pub fn query() -> Option<(u8, bool)> {
        None
    }
}

fn battery() -> Option<Battery> {
    power::query().map(|(percent, charging)| Battery { percent, charging })
}

/// Async so the sysinfo refresh runs on Tauri's worker pool; a synchronous command
/// would block the main thread, delaying tray events and every emitted update.
/// The state is resolved from the AppHandle inside the body because an async
/// command's future must be `'static` and cannot capture a borrowed `State<'_>`.
#[tauri::command]
pub async fn system_stats(app: tauri::AppHandle) -> SystemStats {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let mut monitor = state.system.lock().expect("system state poisoned");
    monitor.sample()
}
