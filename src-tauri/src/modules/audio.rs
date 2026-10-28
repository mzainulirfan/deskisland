use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub percent: u8,
    pub muted: bool,
    pub device: String,
}

#[cfg(windows)]
mod backend {
    use super::Volume;
    use crate::modules::com::ensure_mta;
    use windows::core::GUID;
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDevice, IMMDeviceEnumerator};
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, STGM_READ, CLSCTX_ALL};

    const DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);

    fn endpoint() -> Result<(IMMDevice, IAudioEndpointVolume), String> {
        ensure_mta()?;
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&DEVICE_ENUMERATOR, None, CLSCTX_ALL)
                    .map_err(|e| e.to_string())?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| e.to_string())?;
            let volume = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            Ok((device, volume))
        }
    }

    fn device_name(device: &IMMDevice) -> String {
        unsafe {
            let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
                return String::new();
            };
            let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) else {
                return String::new();
            };
            let Ok(name) = PropVariantToStringAlloc(&value) else {
                return String::new();
            };
            let text = name.to_string().unwrap_or_default();
            CoTaskMemFree(Some(name.0 as *const core::ffi::c_void));
            text.trim().to_string()
        }
    }

    fn read(device: &IMMDevice, volume: &IAudioEndpointVolume) -> Result<Volume, String> {
        unsafe {
            let scalar = volume
                .GetMasterVolumeLevelScalar()
                .map_err(|e| e.to_string())?;
            let muted = volume.GetMute().map_err(|e| e.to_string())?.as_bool();
            Ok(Volume {
                percent: (scalar * 100.0).round().clamp(0.0, 100.0) as u8,
                muted,
                device: device_name(device),
            })
        }
    }

    pub fn query() -> Result<Volume, String> {
        let (device, volume) = endpoint()?;
        read(&device, &volume)
    }

    pub fn set_percent(percent: u8) -> Result<Volume, String> {
        let (device, volume) = endpoint()?;
        unsafe {
            volume
                .SetMasterVolumeLevelScalar(percent.min(100) as f32 / 100.0, core::ptr::null())
                .map_err(|e| e.to_string())?;
            // Dragging the slider means the user wants sound, so a mute is cleared too.
            volume
                .SetMute(false, core::ptr::null())
                .map_err(|e| e.to_string())?;
        }
        read(&device, &volume)
    }

    pub fn set_mute(muted: bool) -> Result<Volume, String> {
        let (device, volume) = endpoint()?;
        unsafe {
            volume
                .SetMute(muted, core::ptr::null())
                .map_err(|e| e.to_string())?;
        }
        read(&device, &volume)
    }
}

#[cfg(not(windows))]
mod backend {
    use super::Volume;

    fn unsupported() -> Result<Volume, String> {
        Err("volume hanya didukung di Windows".to_string())
    }

    pub fn query() -> Result<Volume, String> {
        unsupported()
    }
    pub fn set_percent(_percent: u8) -> Result<Volume, String> {
        unsupported()
    }
    pub fn set_mute(_muted: bool) -> Result<Volume, String> {
        unsupported()
    }
}

// All three are async because every call is a COM round-trip to the audio service;
// run synchronously they would stall the main thread twice a second (volume poll).
#[tauri::command]
pub async fn master_volume() -> Result<Volume, String> {
    backend::query()
}

#[tauri::command]
pub async fn set_master_volume(percent: u8) -> Result<Volume, String> {
    backend::set_percent(percent)
}

#[tauri::command]
pub async fn set_master_mute(muted: bool) -> Result<Volume, String> {
    backend::set_mute(muted)
}
