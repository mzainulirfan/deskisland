use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Position, Size, WebviewWindow};

/// Logical pixels are 1/96 inch on Windows, so this is a physical distance.
const MARGIN: f64 = 16.0;

/// Transparent room around the island so the glow/shadow is not clipped by the
/// native window bounds. The webview paints the island with this padding.
const PAD: f64 = 32.0;

/// Where the webview pins its content inside the window: flush left, centred, or
/// flush right. Must mirror the `anchor` strings sent by the frontend.
#[derive(Clone, Copy, PartialEq)]
enum Anchor {
    Left,
    Center,
    Right,
}

fn parse_anchor(s: &str) -> Anchor {
    match s {
        "left" => Anchor::Left,
        "right" => Anchor::Right,
        _ => Anchor::Center,
    }
}

/// The island window is sized once and only ever *moved* (anchor changes), so its
/// bounds and scale factor are effectively static. The per-tick hover probe reads
/// these from here instead of making four main-thread window API calls, which used
/// to starve rendering, the settings emit and eye-tracking.
#[derive(Clone, Copy)]
struct IslandGeom {
    /// Physical x of the window's left edge.
    left: f64,
    /// Physical width of the window.
    width: f64,
    /// Physical y of the pill's top edge (window top + PAD·scale).
    top: f64,
    scale: f64,
    anchor: Anchor,
}

/// Physical left edge of the pill for its current width. The webview aligns content
/// to the anchor edge, so a left/right-anchored pill sits flush against `PAD`, not
/// in the middle of the oversized window.
fn pill_left(g: &IslandGeom, pill_w: f64) -> f64 {
    match g.anchor {
        Anchor::Left => g.left + PAD * g.scale,
        Anchor::Right => g.left + g.width - PAD * g.scale - pill_w,
        Anchor::Center => g.left + (g.width - pill_w) / 2.0,
    }
}

static GEOM: OnceLock<Mutex<Option<IslandGeom>>> = OnceLock::new();

fn store_geom(g: IslandGeom) {
    *GEOM.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(g);
}

fn read_geom() -> Option<IslandGeom> {
    *GEOM.get_or_init(|| Mutex::new(None)).lock().unwrap()
}

/// The pill's current logical width/height (the webview's visible island box). The
/// frontend pushes this whenever the mode or idle variant changes — never per frame —
/// so the hover region the background monitor tests against is always the resting size.
static REGION: OnceLock<Mutex<Option<(f64, f64)>>> = OnceLock::new();

fn store_region(w: f64, h: f64) {
    *REGION.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some((w, h));
}

fn read_region() -> Option<(f64, f64)> {
    *REGION.get_or_init(|| Mutex::new(None)).lock().unwrap()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorProbe {
    pub over: bool,
    pub dx: f64,
    pub dy: f64,
}

/// Bumped by every anchor request; an animation whose ticket is no longer the newest
/// one gives up, so rapid anchor clicks never leave two threads fighting over the window.
#[derive(Default)]
pub struct MoveGen(pub AtomicU64);

/// Eye tracking IPC is pointless while the island is invisible (fullscreen or hidden
/// from the tray), and keeps the game/desktop competing with WebView work.
static TRACKING: AtomicBool = AtomicBool::new(true);

/// Logical pixels between the screen edge and the pill, vertical. Seeded from
/// `settings.top_margin` and pushed again on every settings write, so both `place` (used
/// by `fit_island_window`) and the anchor glide agree without threading it through every
/// call site. Horizontal edges keep the fixed `MARGIN`.
static TOP_MARGIN: AtomicU32 = AtomicU32::new(16);

pub fn set_top_margin(px: u32) {
    TOP_MARGIN.store(px, Ordering::Relaxed);
}

fn top_margin() -> f64 {
    TOP_MARGIN.load(Ordering::Relaxed) as f64
}

/// Where the top-left corner of the native window should sit, in logical pixels. The
/// island is painted inside `PAD` of transparent padding, so left/right anchoring uses
/// the island width while centring uses the native window width.
fn place_target(
    window: &WebviewWindow,
    outer_width: f64,
    island_width: f64,
    anchor: &str,
) -> Result<(f64, f64), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no primary monitor".to_string())?;

    let scale = monitor.scale_factor();
    let origin = monitor.position();
    let origin_x = origin.x as f64 / scale;
    let origin_y = origin.y as f64 / scale;
    let logical_width = monitor.size().width as f64 / scale;

    let x = match anchor {
        "left" => origin_x + MARGIN - PAD,
        "right" => origin_x + logical_width - MARGIN - island_width - PAD,
        _ => origin_x + (logical_width - outer_width) / 2.0,
    };

    // The window's transparent padding grows upward too, so offset the window by it to
    // keep the pill itself `top_margin` from the screen edge.
    Ok((x, origin_y + top_margin() - PAD))
}

/// Pins the island at its top margin from the screen's top edge and `MARGIN` from the
/// used left/right edge. The webview aligns its content to the same edge, so both the idle
/// pill and the expanded panel grow outward from that margin.
fn place(window: &WebviewWindow, outer_width: f64, island_width: f64, anchor: &str) -> Result<(), String> {
    let (x, y) = place_target(window, outer_width, island_width, anchor)?;
    window
        .set_position(Position::Logical(LogicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
}

/// Size and place the island window once, at its maximum (expanded) size. It is never
/// resized afterwards, so no native resize can interrupt the morph animation; while the
/// pill is collapsed the surplus area is made click-through instead.
#[tauri::command]
pub fn fit_island_window(
    app: AppHandle,
    width: f64,
    height: f64,
    anchor: String,
) -> Result<(), String> {
    let window = app
        .get_webview_window("island")
        .ok_or_else(|| "island window not found".to_string())?;

    let outer_width = width + PAD * 2.0;
    window
        .set_size(Size::Logical(LogicalSize::new(outer_width, height + PAD * 2.0)))
        .map_err(|e| e.to_string())?;

    place(&window, outer_width, width, &anchor)?;

    let origin = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    store_geom(IslandGeom {
        left: origin.x as f64,
        width: size.width as f64,
        top: origin.y as f64 + PAD * scale,
        scale,
        anchor: parse_anchor(&anchor),
    });

    Ok(())
}

/// Glides the already-fitted window to another anchor or top margin when the user picks
/// one in the panel. Ease-out cubic over ~320 ms at roughly 80 fps; the final
/// `set_position` snaps to the exact target so rounding never leaves the window a pixel
/// off the margin.
#[tauri::command]
pub fn set_island_position(app: AppHandle, anchor: String, top: u32) -> Result<(), String> {
    let window = app
        .get_webview_window("island")
        .ok_or_else(|| "island window not found".to_string())?;

    set_top_margin(top.clamp(0, 200));
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let outer_width = window.outer_size().map_err(|e| e.to_string())?.width as f64 / scale;
    let island_width = outer_width - PAD * 2.0;
    let (tx, ty) = place_target(&window, outer_width, island_width, &anchor)?;

    // Glide from wherever the cached geometry says the window is now. The cache is
    // kept current by fit and every prior glide, so we avoid reading outer_position
    // from the non-main glide thread.
    let anchor_kind = parse_anchor(&anchor);
    let (sx, sy_phys) = match read_geom() {
        Some(g) => (g.left, g.top - PAD * scale),
        None => (tx * scale, ty * scale),
    };
    let win_w_phys = outer_width * scale;

    let ticket = app.state::<MoveGen>().0.fetch_add(1, Ordering::SeqCst) + 1;

    let mover = window.clone();
    let handle = app.clone();
    std::thread::spawn(move || {
        const DURATION: f64 = 0.32;
        let begin = Instant::now();
        loop {
            if handle.state::<MoveGen>().0.load(Ordering::SeqCst) != ticket {
                return;
            }
            let t = begin.elapsed().as_secs_f64() / DURATION;
            if t >= 1.0 {
                break;
            }
            let e = 1.0 - (1.0 - t).powi(3);
            let lx = sx + (tx * scale - sx) * e;
            let ly = sy_phys + (ty * scale - sy_phys) * e;
            let _ = mover.set_position(Position::Physical(tauri::PhysicalPosition {
                x: lx.round() as i32,
                y: ly.round() as i32,
            }));
            store_geom(IslandGeom {
                left: lx,
                width: win_w_phys,
                top: ly + PAD * scale,
                scale,
                anchor: anchor_kind,
            });
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        let _ = mover.set_position(Position::Logical(LogicalPosition::new(tx, ty)));
        store_geom(IslandGeom {
            left: tx * scale,
            width: win_w_phys,
            top: ty * scale + PAD * scale,
            scale,
            anchor: anchor_kind,
        });
    });

    Ok(())
}

/// Reports whether the OS cursor is over the pill and where it is relative to the pill
/// centre. The idle eyes use the offset to look at the cursor; hover uses `over`.
///
/// Async, and reads only the cached window geometry plus `GetCursorPos` — it never
/// calls into the window object. The old synchronous version made four main-thread
/// window API calls on every 40 ms poll, which blocked WebView2 rendering, the settings
/// emit and eye-tracking and made every interaction feel sluggish.
#[tauri::command]
pub async fn cursor_probe(width: f64, height: f64) -> Result<CursorProbe, String> {
    let g = read_geom().ok_or_else(|| "island geometry not fitted".to_string())?;

    let mut pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) }.is_err() {
        return Err("GetCursorPos failed".to_string());
    }

    let pill_w = width * g.scale;
    let pill_h = height * g.scale;
    let left = pill_left(&g, pill_w);
    let top = g.top;
    let centre_x = left + pill_w / 2.0;
    let centre_y = top + pill_h / 2.0;

    let (cx, cy) = (pt.x as f64, pt.y as f64);
    Ok(CursorProbe {
        over: cx >= left && cy >= top && cx < left + pill_w && cy < top + pill_h,
        dx: (cx - centre_x) / g.scale,
        dy: (cy - centre_y) / g.scale,
    })
}

/// Pushes the pill's resting logical size so the background hover monitor tests the
/// right rectangle. Called on mode/variant changes only, never per animation frame.
#[tauri::command]
pub fn set_island_hover_region(width: f64, height: f64) {
    store_region(width, height);
}

/// Spawns one high-frequency OS-cursor watcher that *emits* `island-cursor` events to
/// the webview. This replaces the old JavaScript poll: no IPC round-trip and no
/// main-thread window API call sit on the hover path, so entering and leaving the pill
/// feels instant. Small enter/leave sample counts give debouncing without added delay.
#[tauri::command]
pub fn start_cursor_monitor(app: AppHandle) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return; // already running
    }
    std::thread::spawn(move || {
        const STEP: Duration = Duration::from_millis(12);
        const ENTER_SAMPLES: u32 = 2; // ~24 ms: ignore fast mouse passes
        const LEAVE_SAMPLES: u32 = 3; // ~36 ms: quick dwell before collapse
        const MOVE_THROTTLE: Duration = Duration::from_millis(32); // ~30 fps eye tracking

        let mut over = false;
        let mut prev_over = false;
        let mut inside = 0u32;
        let mut outside = 0u32;
        let mut last_px = f64::NAN;
        let mut last_py = f64::NAN;
        let mut last_move = Instant::now();

        loop {
            if !TRACKING.load(Ordering::Relaxed) {
                over = false;
                prev_over = false;
                inside = 0;
                outside = 0;
                last_px = f64::NAN;
                last_py = f64::NAN;
                std::thread::sleep(STEP);
                continue;
            }

            let geom = read_geom();
            let region = read_region();
            if let (Some(g), Some((w, h))) = (geom, region) {
                let mut pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) }.is_ok() {
                    let pill_w = w * g.scale;
                    let pill_h = h * g.scale;
                    let left = pill_left(&g, pill_w);
                    let top = g.top;
                    let centre_x = left + pill_w / 2.0;
                    let centre_y = top + pill_h / 2.0;
                    let (cx, cy) = (pt.x as f64, pt.y as f64);
                    let inside_now = cx >= left && cy >= top && cx < left + pill_w && cy < top + pill_h;

                    if inside_now {
                        outside = 0;
                        inside += 1;
                        if !over && inside >= ENTER_SAMPLES {
                            over = true;
                        }
                    } else {
                        inside = 0;
                        outside += 1;
                        if over && outside >= LEAVE_SAMPLES {
                            over = false;
                        }
                    }

                    let dx = (cx - centre_x) / g.scale;
                    let dy = (cy - centre_y) / g.scale;

                    // Emit on a debounced hover-edge flip immediately.
                    // For cursor motion (eye tracking), only emit while a pill-sized view
                    // is visible; notice (460) and panel views do not render eyes.
                    let edge = over != prev_over;
                    let is_idle_or_hud = w <= 400.0;
                    // The eyes clamp the pupil to ±2.2/±2.4 px of dx/170 and dy/140, so
                    // cursor motion far from the pill paints nothing new. Compare the
                    // clamped values: mouse travel across the desktop no longer floods
                    // the webview with 30 fps IPC events that cannot change pixels.
                    let px = (dx / 170.0).clamp(-2.2, 2.2);
                    let py = (dy / 140.0).clamp(-2.4, 2.4);
                    let moved = last_px.is_nan()
                        || (px - last_px).abs() >= 0.06
                        || (py - last_py).abs() >= 0.06;
                    if edge || (is_idle_or_hud && moved && last_move.elapsed() >= MOVE_THROTTLE) {
                        prev_over = over;
                        last_px = px;
                        last_py = py;
                        last_move = Instant::now();
                        let _ = app.emit("island-cursor", CursorProbe { over, dx, dy });
                    }
                }
            }
            std::thread::sleep(STEP);
        }
    });
}

/// Lets the cursor fall through to the applications behind the island. Used while the
/// pill is collapsed so the oversized window never blocks clicks.
#[tauri::command]
pub fn set_island_passthrough(app: AppHandle, ignore: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("island")
        .ok_or_else(|| "island window not found".to_string())?;

    window
        .set_ignore_cursor_events(ignore)
        .map_err(|e| e.to_string())
}

pub fn set_cursor_tracking(enabled: bool) {
    TRACKING.store(enabled, Ordering::Relaxed);
}

/// Brings the island back after it was hidden from the tray. Returns `true` only when
/// the window was actually hidden, so callers can refresh the tray label.
pub fn reveal(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window("island") else {
        return false;
    };
    if window.is_visible().unwrap_or(true) {
        return false;
    }
    if window.show().is_err() {
        return false;
    }
    set_cursor_tracking(true);
    // The pill plays its "welcome back" greeting on becoming visible; hiding through the
    // tray never reaches the webview, so it has to be told from here.
    let _ = app.emit("island-shown", ());
    true
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
