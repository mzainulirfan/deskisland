use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::modules::settings;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppShortcut {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub name: String,
    pub path: String,
}

/// Resolves a Start-Menu .lnk to its 48-px system icon and returns it as a
/// `data:image/png;base64,…` URL, for the tiles in the Aplikasi tab.
#[tauri::command]
pub async fn app_icon(path: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        // Async commands run on tokio pool threads, so the blocking GDI round-trips
        // here never touch the main thread.
        let png = icon_png(&path).ok_or_else(|| "ikon tidak tersedia".to_string())?;
        use base64::Engine;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("hanya didukung di Windows".to_string())
    }
}

#[cfg(windows)]
fn icon_png(path: &str) -> Option<Vec<u8>> {
    use std::mem::size_of;

    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX, SHIL_EXTRALARGE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

    // Step 1: the system-wide icon index of this file (resolves the .lnk for us).
    let target = HSTRING::from(path);
    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            &target,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX,
        )
    };
    if ok == 0 {
        return None;
    }

    // Step 2: ask the 48-px system image list for a fresh HICON of that index.
    let image_list: IImageList = unsafe { SHGetImageList(SHIL_EXTRALARGE as i32).ok()? };
    let icon: HICON = unsafe { image_list.GetIcon(info.iIcon, ILD_TRANSPARENT.0).ok()? };

    let result = icon_to_png(icon);
    unsafe {
        let _ = DestroyIcon(icon);
    }
    result
}

#[cfg(windows)]
fn icon_to_png(icon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    use std::mem::size_of;

    use windows::Win32::Graphics::Gdi::{
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetDC, GetObjectW, HGDIOBJ, ReleaseDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut ii = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut ii) }.ok()?;
    // For 32-bpp icons the colour bitmap carries the real pixels.
    let mut bm = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            HGDIOBJ::from(ii.hbmColor),
            size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut BITMAP as *mut _),
        )
    };
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(ii.hbmColor));
            let _ = DeleteObject(HGDIOBJ::from(ii.hbmMask));
        }
        return None;
    }
    let (w, h) = (bm.bmWidth as usize, bm.bmHeight as usize);

    let mut pixels = vec![0_u8; w * h * 4];
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bm.bmWidth,
            biHeight: -(h as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    // A DC is only needed as the coordinate-space owner for GetDIBits.
    let screen = unsafe { GetDC(None) };
    let dc = unsafe { CreateCompatibleDC(Some(screen)) };
    let lines = unsafe {
        GetDIBits(
            dc,
            ii.hbmColor,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // If every alpha is zero the icon predates ARGB — fall back to the 1-bpp AND
    // mask (bit set = transparent) so dark glyphs do not render fully opaque.
    let no_alpha = pixels.chunks_exact(4).all(|p| p[3] == 0);
    if no_alpha {
        let stride = (w + 31) / 32 * 4;
        let mut mask = vec![0_u8; stride * h];
        let mut bmi_mask = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bm.bmWidth,
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 1,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let got_mask = unsafe {
            GetDIBits(
                dc,
                ii.hbmMask,
                0,
                h as u32,
                Some(mask.as_mut_ptr() as *mut _),
                &mut bmi_mask,
                DIB_RGB_COLORS,
            )
        };
        if got_mask != 0 {
            for y in 0..h {
                for x in 0..w {
                    let idx = (y * w + x) * 4;
                    let transparent = mask[y * stride + x / 8] & (1 << (7 - x % 8)) != 0;
                    if transparent {
                        pixels[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
                    } else if pixels[idx + 3] == 0 {
                        pixels[idx + 3] = 255;
                    }
                }
            }
        }
    }

    unsafe {
        let _ = DeleteDC(dc);
        let _ = ReleaseDC(None, screen);
        let _ = DeleteObject(HGDIOBJ::from(ii.hbmColor));
        let _ = DeleteObject(HGDIOBJ::from(ii.hbmMask));
    }

    if lines == 0 {
        return None;
    }

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w as u32, h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&pixels).ok()?;
    }
    Some(out)
}

/// Windows exposes installed apps as .lnk files in the two Start Menu trees;
/// scanning them needs no shell COM and covers per-user as well as all-users apps.
fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    roots
}

fn walk(dir: &Path, out: &mut Vec<AppShortcut>, depth: usize) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            walk(&path, out, depth + 1);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("lnk") {
            continue;
        }
        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        if name.is_empty() || name.to_lowercase().starts_with("uninstall") {
            continue;
        }
        out.push(AppShortcut {
            name,
            path: path.to_string_lossy().into_owned(),
        });
    }
}

#[tauri::command]
pub async fn list_app_shortcuts() -> Vec<AppShortcut> {
    let mut shortcuts = Vec::new();
    for root in start_menu_roots() {
        walk(&root, &mut shortcuts, 0);
    }
    shortcuts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    shortcuts.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    shortcuts
}

#[tauri::command]
pub async fn launch_app(path: String) -> Result<(), String> {
    let file = PathBuf::from(&path);
    if file.extension().and_then(|e| e.to_str()) != Some("lnk") || !file.is_file() {
        return Err("bukan shortcut aplikasi".to_string());
    }
    // ShellExecute resolves the .lnk directly; routing through `cmd /c start` or
    // `explorer` would re-parse the path and break on shell metacharacters or spaces.
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWDEFAULT;

        let op = HSTRING::from("open");
        let tgt = HSTRING::from(&path);
        let empty = HSTRING::new();
        let result = unsafe { ShellExecuteW(None, &op, &tgt, &empty, &empty, SW_SHOWDEFAULT) };
        if (result.0 as isize) <= 32 {
            return Err(format!("ShellExecuteW gagal (kode {})", result.0 as isize));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = file;
        Err("hanya didukung di Windows".to_string())
    }
}

#[tauri::command]
pub fn get_favorites(state: tauri::State<'_, settings::SettingsState>) -> Vec<Favorite> {
    state.current.lock().unwrap().favorites.clone()
}

#[tauri::command]
pub async fn set_favorites(app: AppHandle, favorites: Vec<Favorite>) -> Result<(), String> {
    let favorites: Vec<Favorite> = favorites
        .into_iter()
        .filter(|item| item.path.to_lowercase().ends_with(".lnk"))
        .take(12)
        .collect();
    let state = app.state::<settings::SettingsState>();
    {
        let mut current = state.current.lock().unwrap();
        current.favorites = favorites;
        let bytes = serde_json::to_vec_pretty(&*current).map_err(|e| e.to_string())?;
        let _ = fs::write(&state.path, bytes);
    }
    let snapshot = state.current.lock().unwrap().clone();
    let _ = app.emit("settings-changed", &snapshot);
    Ok(())
}
