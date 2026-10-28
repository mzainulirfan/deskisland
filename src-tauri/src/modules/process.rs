use serde::Serialize;
use std::collections::HashMap;
use sysinfo::{ProcessesToUpdate, System};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
}

/// Hottest apps by CPU. Processes that share an executable name (browser renderers,
/// Electron helpers) are merged so one app cannot fill every row. `cpu_usage` is
/// per-core, so a multi-threaded app can exceed 100; the UI shows it as-is because
/// that is what the task manager does.
pub fn hottest(system: &mut System, limit: usize) -> Vec<ProcessInfo> {
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut merged: HashMap<String, (f32, u64)> = HashMap::new();
    for process in system.processes().values() {
        let raw = process.name().to_string_lossy();
        let name = raw.strip_suffix(".exe").unwrap_or(raw.as_ref()).to_string();
        let entry = merged.entry(name).or_insert((0.0, 0));
        entry.0 += process.cpu_usage();
        entry.1 += process.memory();
    }

    let mut processes: Vec<ProcessInfo> = merged
        .into_iter()
        .filter(|(_, (cpu, _))| *cpu > 0.5)
        .map(|(name, (cpu, memory))| ProcessInfo {
            name,
            cpu,
            memory,
        })
        .collect();

    processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    processes.truncate(limit);
    processes
}

#[tauri::command]
pub async fn top_processes(
    app: tauri::AppHandle,
    limit: Option<usize>,
) -> Vec<ProcessInfo> {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let mut monitor = state.system.lock().expect("system state poisoned");
    hottest(&mut monitor.system, limit.unwrap_or(3))
}
