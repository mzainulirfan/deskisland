use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub app: String,
    pub title: String,
    pub body: String,
}

const POLL: Duration = Duration::from_millis(500);

/// Windows exposes no notification listener to unpackaged desktop apps (the WinRT
/// `UserNotificationListener` is capability-gated), so the toast history is read straight
/// from the WPN SQLite database instead. Reads are read-only and transient; nothing is
/// copied out of the database.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut watcher = Watcher::new();
        loop {
            for event in watcher.sample() {
                let _ = app.emit("notification-hud", event);
            }
            std::thread::sleep(POLL);
        }
    });
}

/// The most recent toasts, newest first, so the idle notification style has content
/// straight after launch instead of waiting for the next toast.
#[tauri::command]
pub async fn recent_notifications() -> Vec<NotificationEvent> {
    Watcher::new().recent(8)
}

struct Watcher {
    path: Option<PathBuf>,
    /// Kept open across samples: the WPN database is locked by the service, and
    /// reopening it twice a second was measurable busywork.
    conn: Option<Connection>,
    /// Highest `Notification."Order"` already handled; `None` until the first read primes it.
    last: Option<i64>,
    names: HashMap<String, String>,
}

impl Watcher {
    fn new() -> Self {
        let path = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| {
                root.join("Microsoft")
                    .join("Windows")
                    .join("Notifications")
                    .join("wpndatabase.db")
            })
            .filter(|path| path.is_file());
        Self {
            path,
            conn: None,
            last: None,
            names: HashMap::new(),
        }
    }

    fn connection(&mut self) -> Option<&Connection> {
        if self.conn.is_none() {
            let path = self.path.as_ref()?;
            let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            let conn = Connection::open_with_flags(path, flags).ok()?;
            let _ = conn.busy_timeout(Duration::from_millis(250));
            self.conn = Some(conn);
        }
        self.conn.as_ref()
    }

    /// A read error may mean the service rotated the file; force a reopen next sample.
    fn drop_connection(&mut self) {
        self.conn = None;
    }

    fn sample(&mut self) -> Vec<NotificationEvent> {
        if self.path.is_none() {
            return Vec::new();
        }
        let max = match self.connection() {
            Some(conn) => conn.query_row(
                "SELECT COALESCE(MAX(\"Order\"), 0) FROM Notification",
                [],
                |row| row.get(0),
            ),
            None => return Vec::new(),
        };
        let max: i64 = match max {
            Ok(max) => max,
            Err(_) => {
                self.drop_connection();
                return Vec::new();
            }
        };

        match self.last {
            None => {
                self.last = Some(max);
                return Vec::new();
            }
            Some(last) if max <= last => {
                // A recreated database restarts `Order` at a lower value; rebase silently.
                if max < last {
                    self.last = Some(max);
                }
                return Vec::new();
            }
            Some(_) => {}
        }

        let after = self.last.unwrap_or(0);
        let rows = {
            let Some(conn) = self.connection() else {
                return Vec::new();
            };
            match conn.prepare(
                "SELECT n.Payload, h.PrimaryId
                 FROM Notification n
                 LEFT JOIN NotificationHandler h ON h.RecordId = n.HandlerId
                 WHERE n.\"Order\" > ?1 AND n.Type = 'toast' AND n.PayloadType = 'Xml'
                 ORDER BY n.\"Order\" DESC
                 LIMIT 8",
            ) {
                Ok(mut stmt) => stmt
                    .query_map([after], |row| {
                        Ok((
                            row.get::<_, Option<Vec<u8>>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                        ))
                    })
                    .map(|rows| rows.flatten().collect::<Vec<_>>())
                    .unwrap_or_default(),
                Err(_) => return Vec::new(),
            }
        };

        // Every row in the window is published, oldest first so the pill ends on the
        // newest toast. Stopping at the first parseable row used to discard the rest for
        // good, because `last` still jumped to `max` below.
        let mut events: Vec<NotificationEvent> = rows
            .into_iter()
            .filter_map(|(payload, aumid)| self.parse(payload, aumid))
            .collect();
        events.reverse();

        self.last = Some(max);
        events
    }

    fn recent(&mut self, limit: usize) -> Vec<NotificationEvent> {
        let Some(path) = self.path.as_ref() else {
            return Vec::new();
        };
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let Ok(conn) = Connection::open_with_flags(path, flags) else {
            return Vec::new();
        };
        let _ = conn.busy_timeout(Duration::from_millis(250));

        let Ok(mut stmt) = conn.prepare(
            "SELECT n.Payload, h.PrimaryId
             FROM Notification n
             LEFT JOIN NotificationHandler h ON h.RecordId = n.HandlerId
             WHERE n.Type = 'toast' AND n.PayloadType = 'Xml'
             ORDER BY n.\"Order\" DESC
             LIMIT ?1",
        ) else {
            return Vec::new();
        };
        let Ok(rows) = stmt.query_map([limit as i64], |row| {
            Ok((
                row.get::<_, Option<Vec<u8>>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        }) else {
            return Vec::new();
        };

        let mut events = Vec::new();
        for (payload, aumid) in rows.flatten() {
            if let Some(event) = self.parse(payload, aumid) {
                events.push(event);
            }
        }
        events
    }

    fn parse(
        &mut self,
        payload: Option<Vec<u8>>,
        aumid: Option<String>,
    ) -> Option<NotificationEvent> {
        let payload = payload?;
        let xml = String::from_utf8_lossy(&payload);
        let texts = text_elements(&xml);
        let title = texts.first()?.trim().to_string();
        if title.is_empty() {
            return None;
        }
        let body = texts
            .iter()
            .skip(1)
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");

        Some(NotificationEvent {
            app: self.app_label(aumid.as_deref().unwrap_or_default()),
            title,
            body,
        })
    }

    fn app_label(&mut self, aumid: &str) -> String {
        let key = aumid.trim();
        if key.is_empty() {
            return "Notifikasi".to_string();
        }
        if let Some(name) = self.names.get(key) {
            return name.clone();
        }
        let name = known_app(key)
            .map(str::to_string)
            .or_else(|| shell_app_name(key))
            .unwrap_or_else(|| fallback_app_name(key));
        self.names.insert(key.to_string(), name.clone());
        name
    }
}

fn known_app(aumid: &str) -> Option<&'static str> {
    if aumid == "Windows.Defender.SecurityCenter" {
        return Some("Microsoft Defender");
    }
    if aumid.starts_with("MicrosoftWindows.Client.CBS_")
        || aumid.starts_with("Microsoft.Windows.ShellExperienceHost")
        || aumid.starts_with("Windows.ShellExperienceHost")
    {
        return Some("Windows");
    }
    if aumid.starts_with("Microsoft.WindowsStore_") {
        return Some("Microsoft Store");
    }
    None
}

/// Toast XML is tiny and predictable here: only `<text>` elements carry the content.
fn text_elements(xml: &str) -> Vec<String> {
    const CLOSE: &str = "</text>";
    let mut texts = Vec::new();
    let mut cursor = 0;
    while let Some(found) = xml[cursor..].find("<text") {
        let open = cursor + found;
        let after = &xml[open..];
        let Some(tag_end) = after.find('>') else {
            break;
        };
        let content = open + tag_end + 1;
        if after[..tag_end].ends_with('/') {
            cursor = content;
            continue;
        }
        let Some(end) = xml[content..].find(CLOSE) else {
            break;
        };
        texts.push(decode_entities(&xml[content..content + end]));
        cursor = content + end + CLOSE.len();
    }
    texts
}

fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let tail = &rest[amp..];
        let entity = tail
            .find(';')
            .filter(|end| *end <= 10)
            .map(|end| (end, &tail[1..end]));
        let decoded = entity.and_then(|(_, name)| match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => {
                let digits = name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))?;
                let code = u32::from_str_radix(digits, 16).ok()?;
                char::from_u32(code)
            }
        });
        match (entity, decoded) {
            (Some((end, _)), Some(ch)) => {
                out.push(ch);
                rest = &tail[end + 1..];
            }
            // Decimal character references: &#123;
            (Some((end, name)), None) if name.starts_with('#') => {
                let ch = name[1..].parse::<u32>().ok().and_then(char::from_u32);
                if let Some(ch) = ch {
                    out.push(ch);
                    rest = &tail[end + 1..];
                } else {
                    out.push('&');
                    rest = &tail[1..];
                }
            }
            _ => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn fallback_app_name(aumid: &str) -> String {
    let head = aumid.split('!').next().unwrap_or(aumid);
    let head = match head.rsplit_once('_') {
        Some((base, hash))
            if hash.len() == 13
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()) =>
        {
            base
        }
        _ => head,
    };
    let mut name = head.rsplit('.').next().unwrap_or(head);
    for suffix in ["Desktop", "Messenger", "Music"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            if !stripped.is_empty() {
                name = stripped;
            }
        }
    }
    let name = name.trim_matches(['_', '-', ' ']);
    if name.is_empty() {
        "Notifikasi".to_string()
    } else {
        name.replace(['_', '.'], " ")
    }
}

#[cfg(windows)]
fn shell_app_name(aumid: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{IShellItem, SHCreateItemFromParsingName, SIGDN_NORMALDISPLAY};

    crate::modules::com::ensure_mta().ok()?;
    let path = HSTRING::from(format!("shell:AppsFolder\\{aumid}"));
    let item: IShellItem = unsafe { SHCreateItemFromParsingName(&path, None).ok()? };
    let (text, raw) = unsafe {
        let name = item.GetDisplayName(SIGDN_NORMALDISPLAY).ok()?;
        (name.to_string().unwrap_or_default(), name)
    };
    unsafe { CoTaskMemFree(Some(raw.0 as *const core::ffi::c_void)) };

    let text = text.trim();
    if text.is_empty() || text == aumid {
        None
    } else {
        Some(text.to_string())
    }
}

#[cfg(not(windows))]
fn shell_app_name(_: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_text_nodes_and_entities() {
        let xml = r#"<toast><visual><binding template="ToastGeneric"><text id="1">Pesan &amp; uji</text><text id="2">Halo&#32;dari &#x1F44B; Desk Island</text></binding></visual></toast>"#;
        assert_eq!(
            text_elements(xml),
            ["Pesan & uji", "Halo dari 👋 Desk Island"]
        );
    }

    #[test]
    fn ignores_self_closing_text_nodes() {
        let xml = r#"<toast><text id="1"/><text id="2">A</text><text id="3"> </text></toast>"#;
        assert_eq!(text_elements(xml), ["A", ""]);
    }

    #[test]
    fn maps_known_publishers() {
        assert_eq!(
            known_app("Windows.Defender.SecurityCenter"),
            Some("Microsoft Defender")
        );
        assert_eq!(
            known_app("MicrosoftWindows.Client.CBS_cw5n1h2txyewy!Global.AccountExperiences"),
            Some("Windows")
        );
        assert_eq!(
            known_app("Microsoft.WindowsStore_8wekyb3d8bbwe!App"),
            Some("Microsoft Store")
        );
        assert_eq!(known_app("ZedIndustries.Zed"), None);
    }

    #[test]
    fn builds_fallback_names() {
        assert_eq!(
            fallback_app_name("ZedIndustries.Zed_japxn1gcva8rg!Zed"),
            "Zed"
        );
        assert_eq!(
            fallback_app_name("Acrobat.Reader.Notification.Manager"),
            "Manager"
        );
        assert_eq!(fallback_app_name(""), "Notifikasi");
    }

    #[cfg(windows)]
    #[test]
    fn resolves_names_through_the_shell() {
        let mut watcher = Watcher::new();
        assert_eq!(watcher.app_label("ZedIndustries.Zed"), "Zed");
        assert_eq!(
            watcher.app_label("Windows.Defender.SecurityCenter"),
            "Microsoft Defender"
        );
    }
}
