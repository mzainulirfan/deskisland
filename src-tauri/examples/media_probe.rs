//! Diagnostic: lists every GSMTC session Windows sees, with playback status and
//! metadata, so browser media can be inspected without opening the panel.
//! Run with `cargo run --example media_probe` while music/video is playing.

use std::time::{Duration, Instant};

use windows::core::{HSTRING, Interface};
use windows_future::{AsyncStatus, IAsyncInfo};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

macro_rules! finish {
    ($op:expr) => {{
        let op = &$op;
        let info: windows::core::Result<IAsyncInfo> = op.cast();
        let mut out = None;
        if let Ok(info) = info {
            let started = Instant::now();
            loop {
                match info.Status() {
                    Ok(AsyncStatus::Completed) => {
                        out = op.GetResults().ok();
                        break;
                    }
                    Ok(AsyncStatus::Error) | Ok(AsyncStatus::Canceled) => break,
                    _ => {
                        if started.elapsed() > Duration::from_secs(30) {
                            println!("  (operation still running after 30 s)");
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        }
        out
    }};
}

fn txt(value: windows::core::Result<HSTRING>) -> String {
    value.map(|text| text.to_string()).unwrap_or_default()
}

fn describe(session: &GlobalSystemMediaTransportControlsSession, label: &str) {
    let aumid = txt(session.SourceAppUserModelId());
    let status = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|status| format!("{status:?}"))
        .unwrap_or_else(|e| format!("err {e}"));
    let meta = session
        .TryGetMediaPropertiesAsync()
        .ok()
        .and_then(|request| finish!(request))
        .map(|properties| {
            format!(
                "title={:?} artist={:?}",
                txt(properties.Title()),
                txt(properties.Artist())
            )
        })
        .unwrap_or_else(|| "metadata: timeout/err".to_string());
    println!("[{label}] aumid={aumid} status={status} {meta}");
}

fn main() {
    let init = unsafe {
        windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        )
    };
    println!("CoInitializeEx(MTA) -> {init:?}");

    let Ok(request) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() else {
        println!("RequestAsync failed");
        return;
    };
    let Some(manager) = finish!(request) else {
        println!("manager: no result");
        return;
    };

    match manager.GetSessions() {
        Ok(sessions) => {
            let count = sessions.Size().unwrap_or(0);
            println!("sessions: {count}");
            for index in 0..count.min(12) {
                if let Ok(session) = sessions.GetAt(index) {
                    describe(&session, &index.to_string());
                }
            }
        }
        Err(error) => println!("GetSessions failed: {error}"),
    }

    match manager.GetCurrentSession() {
        Ok(current) => describe(&current, "current"),
        Err(error) => println!("GetCurrentSession failed: {error}"),
    }
}
