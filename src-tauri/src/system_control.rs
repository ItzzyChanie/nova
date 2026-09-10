use std::{fs, path::PathBuf};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[cfg(windows)]
use std::{ptr::null_mut, thread, time::Duration};

#[cfg(windows)]
use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    Media::Audio::{
        eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::FILETIME,
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    },
    System::{
        Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
    UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemControlErrorCode {
    #[allow(dead_code)]
    UnsupportedPlatform,
    InvalidArguments,
    VolumeUnavailable,
    SystemInfoUnavailable,
    ScreenshotFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemControlError {
    pub code: SystemControlErrorCode,
    pub message: String,
}

impl SystemControlError {
    fn new(code: SystemControlErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(windows)]
fn scalar_to_percent(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u8
}

#[cfg(windows)]
fn percent_to_scalar(percent: u8) -> f32 {
    f32::from(percent.min(100)) / 100.0
}

#[cfg(windows)]
struct ComGuard(bool);

#[cfg(windows)]
impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(windows)]
fn with_default_audio_endpoint<T>(
    action: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T>,
) -> Result<T, SystemControlError> {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED);
        let guard = if initialized.is_ok() {
            ComGuard(true)
        } else if initialized == RPC_E_CHANGED_MODE {
            ComGuard(false)
        } else {
            return Err(SystemControlError::new(
                SystemControlErrorCode::VolumeUnavailable,
                format!("Could not initialize Windows Core Audio: {initialized:?}."),
            ));
        };
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|error| {
                SystemControlError::new(
                    SystemControlErrorCode::VolumeUnavailable,
                    format!("Could not create the Windows audio-device enumerator: {error}"),
                )
            })?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|error| {
                SystemControlError::new(
                    SystemControlErrorCode::VolumeUnavailable,
                    format!("Could not access the default Windows output device: {error}"),
                )
            })?;
        let endpoint: IAudioEndpointVolume =
            device.Activate(CLSCTX_ALL, None).map_err(|error| {
                SystemControlError::new(
                    SystemControlErrorCode::VolumeUnavailable,
                    format!("Could not access the Windows master-volume endpoint: {error}"),
                )
            })?;
        let result = action(&endpoint).map_err(|error| {
            SystemControlError::new(
                SystemControlErrorCode::VolumeUnavailable,
                format!("Windows rejected the master-volume request: {error}"),
            )
        });
        drop(endpoint);
        drop(device);
        drop(enumerator);
        drop(guard);
        result
    }
}

#[cfg(windows)]
pub fn get_volume() -> Result<Value, SystemControlError> {
    let (volume, muted) = with_default_audio_endpoint(|endpoint| unsafe {
        Ok((
            scalar_to_percent(endpoint.GetMasterVolumeLevelScalar()?),
            endpoint.GetMute()?.as_bool(),
        ))
    })?;
    Ok(json!({
        "volume": volume,
        "muted": muted,
        "message": format!("System volume is {volume}%.")
    }))
}

#[cfg(not(windows))]
pub fn get_volume() -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "Volume controls are currently available only on Windows.",
    ))
}

#[cfg(windows)]
pub fn set_volume(volume: u8) -> Result<Value, SystemControlError> {
    if volume > 100 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::InvalidArguments,
            "Volume must be between 0 and 100.",
        ));
    }
    let scalar = percent_to_scalar(volume);
    with_default_audio_endpoint(|endpoint| unsafe {
        endpoint.SetMasterVolumeLevelScalar(scalar, std::ptr::null())?;
        if volume > 0 {
            endpoint.SetMute(false, std::ptr::null())?;
        }
        Ok(())
    })?;
    Ok(json!({
        "volume": volume,
        "muted": volume == 0,
        "message": format!("Set system volume to {volume}%.")
    }))
}

#[cfg(not(windows))]
pub fn set_volume(_volume: u8) -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "Volume controls are currently available only on Windows.",
    ))
}

pub fn adjust_volume(delta: i8) -> Result<Value, SystemControlError> {
    if delta == 0 || !(-100..=100).contains(&delta) {
        return Err(SystemControlError::new(
            SystemControlErrorCode::InvalidArguments,
            "Volume adjustment must be between -100 and 100 and cannot be zero.",
        ));
    }
    let current = get_volume()?
        .get("volume")
        .and_then(Value::as_u64)
        .unwrap_or(0) as i16;
    let target = (current + i16::from(delta)).clamp(0, 100) as u8;
    let mut result = set_volume(target)?;
    if let Some(data) = result.as_object_mut() {
        data.insert("previousVolume".into(), json!(current));
        data.insert("delta".into(), json!(delta));
        data.insert(
            "message".into(),
            json!(format!(
                "Adjusted system volume from {current}% to {target}%."
            )),
        );
    }
    Ok(result)
}

pub fn mute() -> Result<Value, SystemControlError> {
    #[cfg(windows)]
    with_default_audio_endpoint(|endpoint| unsafe { endpoint.SetMute(true, std::ptr::null()) })?;
    #[cfg(not(windows))]
    return set_volume(0);
    Ok(json!({ "muted": true, "message": "Muted system volume." }))
}

pub fn unmute() -> Result<Value, SystemControlError> {
    #[cfg(windows)]
    let volume = with_default_audio_endpoint(|endpoint| unsafe {
        if endpoint.GetMasterVolumeLevelScalar()? <= 0.001 {
            endpoint.SetMasterVolumeLevelScalar(0.5, std::ptr::null())?;
        }
        endpoint.SetMute(false, std::ptr::null())?;
        endpoint.GetMasterVolumeLevelScalar()
    })?;
    #[cfg(not(windows))]
    return set_volume(50);
    let target = scalar_to_percent(volume);
    Ok(json!({
        "volume": target,
        "muted": false,
        "message": format!("Unmuted system volume to {target}%.")
    }))
}

#[cfg(windows)]
pub fn get_battery() -> Result<Value, SystemControlError> {
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::SystemInfoUnavailable,
            "Windows battery status is unavailable.",
        ));
    }
    let percent = if status.BatteryLifePercent == 255 {
        None
    } else {
        Some(status.BatteryLifePercent)
    };
    Ok(json!({
        "batteryPercent": percent,
        "acLineStatus": status.ACLineStatus,
        "charging": status.ACLineStatus == 1,
        "batteryPresent": status.BatteryFlag != 128,
        "batteryFlag": status.BatteryFlag,
        "message": match percent {
            Some(value) => format!("Battery is at {value}%."),
            None => "Battery percentage is unavailable.".to_string(),
        }
    }))
}

#[cfg(not(windows))]
pub fn get_battery() -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "Battery status is currently available only on Windows.",
    ))
}

#[cfg(windows)]
fn filetime_to_u64(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

#[cfg(windows)]
fn cpu_times() -> Result<(u64, u64), SystemControlError> {
    let mut idle: FILETIME = unsafe { std::mem::zeroed() };
    let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
    let mut user: FILETIME = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::SystemInfoUnavailable,
            "Windows CPU usage counters are unavailable.",
        ));
    }
    let idle = filetime_to_u64(idle);
    let kernel = filetime_to_u64(kernel);
    let user = filetime_to_u64(user);
    Ok((idle, kernel + user))
}

#[cfg(windows)]
pub fn get_cpu_usage() -> Result<Value, SystemControlError> {
    let (idle_a, total_a) = cpu_times()?;
    thread::sleep(Duration::from_millis(250));
    let (idle_b, total_b) = cpu_times()?;
    let total_delta = total_b.saturating_sub(total_a);
    let idle_delta = idle_b.saturating_sub(idle_a);
    let usage = if total_delta == 0 {
        0.0
    } else {
        ((total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64) * 100.0
    };
    let usage = (usage * 10.0).round() / 10.0;
    Ok(json!({
        "cpuUsagePercent": usage,
        "sampleMilliseconds": 250,
        "message": format!("CPU usage is {usage:.1}%.")
    }))
}

#[cfg(not(windows))]
pub fn get_cpu_usage() -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "CPU usage is currently available only on Windows.",
    ))
}

#[cfg(windows)]
pub fn get_memory_usage() -> Result<Value, SystemControlError> {
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::SystemInfoUnavailable,
            "Windows memory status is unavailable.",
        ));
    }
    let used = status.ullTotalPhys.saturating_sub(status.ullAvailPhys);
    let used_percent = if status.ullTotalPhys == 0 {
        0.0
    } else {
        used as f64 / status.ullTotalPhys as f64 * 100.0
    };
    let used_percent = (used_percent * 10.0).round() / 10.0;
    Ok(json!({
        "memoryUsagePercent": used_percent,
        "totalBytes": status.ullTotalPhys,
        "availableBytes": status.ullAvailPhys,
        "usedBytes": used,
        "message": format!("Memory usage is {used_percent:.1}%.")
    }))
}

#[cfg(not(windows))]
pub fn get_memory_usage() -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "Memory usage is currently available only on Windows.",
    ))
}

fn screenshot_directory(app: &AppHandle) -> Result<PathBuf, SystemControlError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| {
            SystemControlError::new(
                SystemControlErrorCode::ScreenshotFailed,
                format!("Could not resolve NOVA's local data folder: {error}"),
            )
        })?
        .join("screenshots");
    fs::create_dir_all(&directory).map_err(|error| {
        SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            format!("Could not create screenshot folder: {error}"),
        )
    })?;
    Ok(directory)
}

fn screenshot_path(app: &AppHandle) -> Result<PathBuf, SystemControlError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            SystemControlError::new(
                SystemControlErrorCode::ScreenshotFailed,
                format!("System clock is before the Unix epoch: {error}"),
            )
        })?
        .as_secs();
    Ok(screenshot_directory(app)?.join(format!("nova-screenshot-{timestamp}.bmp")))
}

#[cfg(windows)]
fn write_bmp(
    path: &PathBuf,
    width: i32,
    height: i32,
    pixels: &[u8],
) -> Result<(), SystemControlError> {
    let pixel_bytes = pixels.len() as u32;
    let file_size = 14u32 + 40u32 + pixel_bytes;
    let mut bytes = Vec::with_capacity(file_size as usize);
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&file_size.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&(14u32 + 40u32).to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&pixel_bytes.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(pixels);
    fs::write(path, bytes).map_err(|error| {
        SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            format!("Could not write screenshot: {error}"),
        )
    })
}

#[cfg(windows)]
pub fn take_screenshot(app: &AppHandle) -> Result<Value, SystemControlError> {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if width <= 0 || height <= 0 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Windows returned an invalid screen size.",
        ));
    }

    let screen_dc = unsafe { GetDC(null_mut()) };
    if screen_dc.is_null() {
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Could not access the screen device context.",
        ));
    }

    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        unsafe {
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Could not create a memory device context.",
        ));
    }

    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width, height) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Could not create a screenshot bitmap.",
        ));
    }

    let old_object = unsafe { SelectObject(memory_dc, bitmap) };
    let copied = unsafe { BitBlt(memory_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY) } != 0;
    if !old_object.is_null() {
        unsafe {
            SelectObject(memory_dc, old_object);
        }
    }

    if !copied {
        unsafe {
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Windows could not copy the screen into the screenshot bitmap.",
        ));
    }

    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: (width * height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [unsafe { std::mem::zeroed() }; 1],
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let lines = unsafe {
        GetDIBits(
            memory_dc,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };

    unsafe {
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);
    }

    if lines == 0 {
        return Err(SystemControlError::new(
            SystemControlErrorCode::ScreenshotFailed,
            "Windows could not read screenshot pixels.",
        ));
    }

    let path = screenshot_path(app)?;
    write_bmp(&path, width, height, &pixels)?;
    let path = path.to_string_lossy().to_string();
    Ok(json!({
        "path": path,
        "width": width,
        "height": height,
        "format": "bmp",
        "message": format!("Saved screenshot to {path}.")
    }))
}

#[cfg(not(windows))]
pub fn take_screenshot(_app: &AppHandle) -> Result<Value, SystemControlError> {
    Err(SystemControlError::new(
        SystemControlErrorCode::UnsupportedPlatform,
        "Screenshots are currently available only on Windows.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_percent_round_trips_to_native_scalar() {
        assert_eq!(scalar_to_percent(percent_to_scalar(0)), 0);
        assert_eq!(scalar_to_percent(percent_to_scalar(50)), 50);
        assert_eq!(scalar_to_percent(percent_to_scalar(100)), 100);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires an active Windows output device"]
    fn live_core_audio_endpoint_accepts_a_same_value_write() {
        let (before, after) = with_default_audio_endpoint(|endpoint| unsafe {
            let before = endpoint.GetMasterVolumeLevelScalar()?;
            endpoint.SetMasterVolumeLevelScalar(before, std::ptr::null())?;
            Ok((before, endpoint.GetMasterVolumeLevelScalar()?))
        })
        .expect("the default Windows Core Audio endpoint should be available");
        assert!((before - after).abs() < 0.001);
    }
}
