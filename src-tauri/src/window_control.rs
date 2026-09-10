use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(windows)]
use std::{ffi::OsString, os::windows::ffi::OsStringExt};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlErrorCode {
    #[allow(dead_code)]
    UnsupportedPlatform,
    InvalidArguments,
    WindowNotFound,
    AmbiguousWindow,
    FocusFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowControlError {
    pub code: WindowControlErrorCode,
    pub message: String,
}

impl WindowControlError {
    fn new(code: WindowControlErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowFocusArguments {
    pub title: Option<String>,
    pub window_id: Option<String>,
    #[serde(default)]
    pub exact: bool,
}

#[cfg(windows)]
#[derive(Debug, Clone)]
struct WindowRecord {
    handle: HWND,
    title: String,
    process_id: u32,
}

#[cfg(windows)]
fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(windows)]
fn window_id(window: HWND) -> String {
    format!("hwnd-{:x}", window as usize)
}

#[cfg(windows)]
fn wide_string(buffer: &[u16]) -> OsString {
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    OsString::from_wide(&buffer[..length])
}

#[cfg(windows)]
unsafe extern "system" fn collect_windows(window: HWND, parameter: LPARAM) -> i32 {
    let windows = &mut *(parameter as *mut Vec<WindowRecord>);
    let title_length = GetWindowTextLengthW(window);
    if IsWindowVisible(window) == 0 || title_length <= 0 {
        return 1;
    }

    let mut title = vec![0u16; title_length as usize + 1];
    GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32);
    let title = wide_string(&title).to_string_lossy().trim().to_string();
    if title.is_empty() || title == "Program Manager" {
        return 1;
    }

    let mut process_id = 0;
    GetWindowThreadProcessId(window, &mut process_id);
    windows.push(WindowRecord {
        handle: window,
        title,
        process_id,
    });
    1
}

#[cfg(windows)]
fn visible_windows() -> Vec<WindowRecord> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(collect_windows),
            &mut windows as *mut Vec<WindowRecord> as LPARAM,
        );
    }
    windows
}

#[cfg(windows)]
fn public_window(value: &WindowRecord) -> Value {
    json!({
        "windowId": window_id(value.handle),
        "title": value.title,
        "processId": value.process_id,
    })
}

#[cfg(windows)]
pub fn list_windows() -> Result<Value, WindowControlError> {
    let windows: Vec<_> = visible_windows().iter().map(public_window).collect();
    Ok(json!({
        "windows": windows,
        "count": windows.len(),
        "message": format!("Found {} visible windows.", windows.len())
    }))
}

#[cfg(not(windows))]
pub fn list_windows() -> Result<Value, WindowControlError> {
    Err(WindowControlError::new(
        WindowControlErrorCode::UnsupportedPlatform,
        "Window listing is currently available only on Windows.",
    ))
}

#[cfg(windows)]
fn resolve_window(arguments: &WindowFocusArguments) -> Result<WindowRecord, WindowControlError> {
    if arguments.window_id.is_none() && arguments.title.as_deref().unwrap_or("").trim().is_empty() {
        return Err(WindowControlError::new(
            WindowControlErrorCode::InvalidArguments,
            "window.focus requires either windowId or title.",
        ));
    }

    let windows = visible_windows();
    if let Some(requested_id) = &arguments.window_id {
        return windows
            .into_iter()
            .find(|window| window_id(window.handle).eq_ignore_ascii_case(requested_id.trim()))
            .ok_or_else(|| {
                WindowControlError::new(
                    WindowControlErrorCode::WindowNotFound,
                    format!("No visible window matched {requested_id}."),
                )
            });
    }

    let requested_title = arguments.title.as_deref().unwrap_or("").trim();
    let requested_normalized = normalize(requested_title);
    let matches: Vec<_> = windows
        .into_iter()
        .filter(|window| {
            let title = normalize(&window.title);
            if arguments.exact {
                title == requested_normalized
            } else {
                title.contains(&requested_normalized)
            }
        })
        .collect();

    match matches.as_slice() {
        [] => Err(WindowControlError::new(
            WindowControlErrorCode::WindowNotFound,
            format!("No visible window matched '{requested_title}'."),
        )),
        [window] => Ok(window.clone()),
        _ => Err(WindowControlError::new(
            WindowControlErrorCode::AmbiguousWindow,
            format!(
                "'{requested_title}' matched {} windows. Use window.list and retry with windowId.",
                matches.len()
            ),
        )),
    }
}

#[cfg(windows)]
pub fn focus_window(arguments: &WindowFocusArguments) -> Result<Value, WindowControlError> {
    let window = resolve_window(arguments)?;
    unsafe {
        if IsIconic(window.handle) != 0 {
            ShowWindow(window.handle, SW_RESTORE);
        }
        if SetForegroundWindow(window.handle) == 0 {
            return Err(WindowControlError::new(
                WindowControlErrorCode::FocusFailed,
                format!("Windows did not allow NOVA to focus '{}'.", window.title),
            ));
        }
    }

    Ok(json!({
        "windowId": window_id(window.handle),
        "title": window.title,
        "processId": window.process_id,
        "focused": true,
        "message": "Focused the requested window."
    }))
}

#[cfg(not(windows))]
pub fn focus_window(_arguments: &WindowFocusArguments) -> Result<Value, WindowControlError> {
    Err(WindowControlError::new(
        WindowControlErrorCode::UnsupportedPlatform,
        "Window focus is currently available only on Windows.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_focus_requires_title_or_id() {
        let arguments = WindowFocusArguments {
            title: None,
            window_id: None,
            exact: false,
        };
        let error = resolve_window(&arguments).unwrap_err();
        assert_eq!(error.code, WindowControlErrorCode::InvalidArguments);
    }
}
