use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub name: String,
    pub size: u64,
    /// Full path, so the notice can offer open/reveal actions.
    pub path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    pub count: usize,
    /// Set once per new file, so the island does not re-announce the same download.
    pub event: Option<DownloadEvent>,
}

#[derive(Clone, PartialEq, Eq)]
struct Entry {
    modified: SystemTime,
    name: String,
    size: u64,
    path: PathBuf,
}

#[derive(Default)]
pub struct Watcher {
    folder: Option<PathBuf>,
    /// Newest entry seen so far; `None` while the folder has no files.
    newest: Option<Entry>,
    primed: bool,
}

/// A download grows to its final size over a second or two, so only announce an entry once
/// its newest timestamp is older than this; mid-write files stay silent.
const SETTLE: Duration = Duration::from_secs(2);

impl Watcher {
    pub fn new() -> Self {
        Self {
            folder: folder(),
            ..Default::default()
        }
    }

    pub fn sample(&mut self) -> Downloads {
        let Some(folder) = &self.folder else {
            return Downloads {
                count: 0,
                event: None,
            };
        };

        let Ok(entries) = std::fs::read_dir(folder) else {
            return Downloads {
                count: 0,
                event: None,
            };
        };

        let mut count = 0_usize;
        let mut newest: Option<Entry> = None;
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            count += 1;

            let modified = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let is_newer = match &newest {
                Some(current) => modified > current.modified,
                None => true,
            };
            if is_newer {
                newest = Some(Entry {
                    modified,
                    name: entry.file_name().to_string_lossy().into_owned(),
                    size: metadata.len(),
                    path: folder.join(entry.file_name()),
                });
            }
        }

        // An empty folder must never produce a notice (that rendered as a blank
        // "Terunduh · 0.0 MB" card), and it must not forget the last announced entry:
        // the next real download still has to be announced.
        let Some(entry) = newest else {
            return Downloads {
                count,
                event: None,
            };
        };

        if !self.primed {
            self.primed = true;
            self.newest = Some(entry);
            return Downloads {
                count,
                event: None,
            };
        }

        let settled = SystemTime::now()
            .duration_since(entry.modified)
            .map(|age| age >= SETTLE)
            .unwrap_or(false);
        // Only announce an entry that is newer than the last one seen. When the newest
        // file is deleted, the next candidate is older and must not be re-announced.
        let unseen = match &self.newest {
            Some(seen) => {
                entry.modified > seen.modified
                    || (entry.modified == seen.modified && *seen != entry)
            }
            None => true,
        };
        if settled && unseen {
            let event = DownloadEvent {
                name: entry.name.clone(),
                size: entry.size,
                path: entry.path.to_string_lossy().into_owned(),
            };
            self.newest = Some(entry);
            return Downloads {
                count,
                event: Some(event),
            };
        }

        Downloads {
            count,
            event: None,
        }
    }
}

#[cfg(windows)]
fn folder() -> Option<PathBuf> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    let path = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_Downloads,
            KF_FLAG_DEFAULT,
            Some(HANDLE::default()),
        )
        .ok()
        .map(|pwstr| {
            let path = pwstr.to_string().ok().map(PathBuf::from);
            CoTaskMemFree(Some(pwstr.0 as *const core::ffi::c_void));
            path
        })
    };
    path.flatten()
}

#[cfg(not(windows))]
fn folder() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Downloads"))
        .filter(|candidate| candidate.is_dir())
}

#[tauri::command]
pub async fn downloads_status(app: tauri::AppHandle) -> Downloads {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    let mut watcher = state.downloads.lock().expect("downloads state poisoned");
    watcher.sample()
}

/// Newest-first listing for the panel's Downloads tab. Reads the folder directly
/// rather than the watcher, whose state is optimised for change detection.
#[tauri::command]
pub async fn recent_downloads(limit: usize) -> Vec<DownloadEvent> {
    let Some(folder) = folder() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };

    let mut files: Vec<(SystemTime, DownloadEvent)> = entries
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((
                metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                DownloadEvent {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    size: metadata.len(),
                    path: folder.join(entry.file_name()).to_string_lossy().into_owned(),
                },
            ))
        })
        .collect();

    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.truncate(limit.min(50));
    files.into_iter().map(|(_, file)| file).collect()
}

/// Opens a finished download with its default handler. The path only ever comes from
/// the watcher above (a real file inside the Downloads folder), but re-checking keeps a
/// stale notice from launching something that has since been replaced by a directory.
#[tauri::command]
pub async fn open_download(path: String) -> Result<(), String> {
    let file = PathBuf::from(&path);
    if !file.is_file() {
        return Err("file not found".to_string());
    }
    #[cfg(windows)]
    {
        // Browsers stamp downloaded files with a Zone.Identifier marker, which makes
        // Windows interrupt every open with a "trusted source" prompt. The user
        // explicitly clicked "open", so the marker is cleared first — files opened
        // from Explorer or elsewhere keep the normal Windows protection.
        let _ = std::fs::remove_file(format!(r"{path}:Zone.Identifier"));
        shellexecute("open", &path, "")
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        Err("hanya didukung di Windows".to_string())
    }
}

/// Opens the Downloads folder with the file selected in Explorer.
#[tauri::command]
pub async fn reveal_download(path: String) -> Result<(), String> {
    let file = PathBuf::from(&path);
    if !file.is_file() {
        return Err("file not found".to_string());
    }
    #[cfg(windows)]
    {
        // Explorer parses its own command line and breaks on quoted-or-escaped paths
        // handed over through std::process, so ShellExecuteW gets the literal
        // `/select,"path"` form — the only spelling that survives spaces.
        shellexecute("open", "explorer.exe", &format!(r#"/select,"{path}""#))
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        Err("hanya didukung di Windows".to_string())
    }
}

/// Opens the Downloads folder itself, for the panel's header button.
#[tauri::command]
pub async fn open_downloads_folder() -> Result<(), String> {
    let folder = folder().ok_or_else(|| "downloads folder not found".to_string())?;
    #[cfg(windows)]
    {
        shellexecute("open", &folder.to_string_lossy(), "")
    }
    #[cfg(not(windows))]
    {
        let _ = folder;
        Err("hanya didukung di Windows".to_string())
    }
}

#[cfg(windows)]
fn shellexecute(operation: &str, target: &str, parameters: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWDEFAULT;

    let op = HSTRING::from(operation);
    let tgt = HSTRING::from(target);
    let prm = HSTRING::from(parameters);
    // windows 0.62 will not infer `None` for the generic PCWSTR parameters, so an
    // empty string stands in for NULL — ShellExecuteW treats both identically here.
    let empty = HSTRING::new();
    let result = unsafe { ShellExecuteW(None, &op, &tgt, &prm, &empty, SW_SHOWDEFAULT) };
    // ShellExecute returns the spawned instance handle, or a SE_ERR_* code <= 32.
    if (result.0 as isize) <= 32 {
        return Err(format!("ShellExecuteW gagal (kode {})", result.0 as isize));
    }
    Ok(())
}
