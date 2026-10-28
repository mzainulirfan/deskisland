use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub app: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position: u64,
    pub duration: u64,
}

#[cfg(windows)]
mod backend {
    use super::NowPlaying;
    use crate::modules::com::{ensure_mta, wait};
    use windows::core::HSTRING;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };

    fn text(value: windows::core::Result<HSTRING>) -> String {
        value.map(|text| text.to_string()).unwrap_or_default()
    }

    /// 100-nanosecond ticks to whole seconds; negative or missing values read as zero.
    fn seconds(ticks: i64) -> u64 {
        (ticks / 10_000_000).max(0) as u64
    }

    /// "Microsoft.ZuneMusic_8wekyb3d8bbwe!App" is the AUMID, so keep the readable part.
    fn app_name(session: &GlobalSystemMediaTransportControlsSession) -> String {
        let id = text(session.SourceAppUserModelId());
        id.split(['!', '_'])
            .next()
            .unwrap_or("")
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_string()
    }

    fn manager() -> Result<Option<GlobalSystemMediaTransportControlsSessionManager>, String> {
        ensure_mta()?;
        let request = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| e.to_string())?;
        match wait(request) {
            Some(Ok(manager)) => Ok(Some(manager)),
            Some(Err(error)) => Err(error.to_string()),
            None => Ok(None),
        }
    }

    fn is_playing(session: &GlobalSystemMediaTransportControlsSession) -> bool {
        session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|status| {
                status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
            })
            .unwrap_or(false)
    }

    fn media_properties(
        session: &GlobalSystemMediaTransportControlsSession,
    ) -> Option<(String, String)> {
        let request = session.TryGetMediaPropertiesAsync().ok()?;
        let Some(Ok(properties)) = wait(request) else {
            return None;
        };
        let title = text(properties.Title());
        let artist = text(properties.Artist());
        if title.is_empty() && artist.is_empty() {
            None
        } else {
            Some((title, artist))
        }
    }

    /// Browsers register one GSMTC session per media tab and only mark one of them as
    /// "current" — which may be a silent or dead tab. Prefer a session that is
    /// actually playing; fall back to whatever GSMTC reports as current.
    fn pick_session(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
    ) -> Option<GlobalSystemMediaTransportControlsSession> {
        if let Ok(sessions) = manager.GetSessions() {
            let count = sessions.Size().unwrap_or(0).min(8);
            for index in 0..count {
                if let Ok(session) = sessions.GetAt(index) {
                    if is_playing(&session) {
                        return Some(session);
                    }
                }
            }
        }
        manager.GetCurrentSession().ok()
    }

    pub fn control(action: &str) -> Result<bool, String> {
        let Some(manager) = manager()? else {
            return Ok(false);
        };
        let session = pick_session(&manager).ok_or_else(|| "no media session".to_string())?;

        let request = match action {
            "toggle" => session.TryTogglePlayPauseAsync(),
            "play" => session.TryPlayAsync(),
            "pause" => session.TryPauseAsync(),
            "next" => session.TrySkipNextAsync(),
            "previous" => session.TrySkipPreviousAsync(),
            _ => return Err(format!("unknown media action: {action}")),
        }
        .map_err(|e| e.to_string())?;

        match wait(request) {
            Some(Ok(ok)) => Ok(ok),
            Some(Err(error)) => Err(error.to_string()),
            None => Ok(false),
        }
    }

    pub fn query() -> Result<Option<NowPlaying>, String> {
        let Some(manager) = manager()? else {
            return Ok(None);
        };

        // Walk the sessions (bounded): take the first that is playing with metadata,
        // otherwise the first with metadata, so a paused YouTube tab still renders.
        let mut chosen = None;
        if let Ok(sessions) = manager.GetSessions() {
            let count = sessions.Size().unwrap_or(0).min(6);
            for index in 0..count {
                let Ok(session) = sessions.GetAt(index) else {
                    continue;
                };
                let Some((title, artist)) = media_properties(&session) else {
                    continue;
                };
                if is_playing(&session) {
                    chosen = Some((session, title, artist));
                    break;
                }
                if chosen.is_none() {
                    chosen = Some((session, title, artist));
                }
            }
        }

        let (session, title, artist) = match chosen {
            Some(found) => found,
            None => return Ok(None),
        };

        let playback = session.GetPlaybackInfo().map_err(|e| e.to_string())?;
        let status = playback.PlaybackStatus().map_err(|e| e.to_string())?;
        let timeline = session.GetTimelineProperties().map_err(|e| e.to_string())?;

        Ok(Some(NowPlaying {
            app: app_name(&session),
            title,
            artist,
            playing: status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing,
            position: seconds(timeline.Position().map(|t| t.Duration).unwrap_or(0)),
            duration: seconds(timeline.EndTime().map(|t| t.Duration).unwrap_or(0)),
        }))
    }
}

#[cfg(not(windows))]
mod backend {
    use super::NowPlaying;

    pub fn query() -> Result<Option<NowPlaying>, String> {
        Ok(None)
    }

    pub fn control(_action: &str) -> Result<bool, String> {
        Ok(false)
    }
}

/// Async on purpose: the WinRT wait loop can sleep for seconds when a media app
/// stops answering. On the main thread that would freeze tray events, settings
/// delivery and the cursor glide.
#[tauri::command]
pub async fn now_playing() -> Result<Option<NowPlaying>, String> {
    backend::query()
}

#[tauri::command]
pub async fn media_control(action: String) -> Result<bool, String> {
    backend::control(&action)
}
