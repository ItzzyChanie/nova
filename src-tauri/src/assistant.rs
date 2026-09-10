use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow,
    WindowEvent,
};

use crate::{audio, settings};

const ASSISTANT_LABEL: &str = "assistant";
const TOP_MARGIN: f64 = 24.0;

fn emit_assistant_update(
    assistant: &WebviewWindow,
    state: &str,
    transcript: Option<String>,
    error: Option<String>,
) -> Result<(), String> {
    assistant
        .emit("assistant-transcript", transcript)
        .map_err(|emit_error| format!("Could not update the assistant transcript: {emit_error}"))?;
    assistant
        .emit("assistant-error", error)
        .map_err(|emit_error| format!("Could not update the assistant error: {emit_error}"))?;
    assistant
        .emit("assistant-state", state)
        .map_err(|emit_error| format!("Could not update the assistant state: {emit_error}"))
}

// All inputs/outputs here are physical pixels, including negative monitor origins.
fn top_center(
    origin: PhysicalPosition<i32>,
    area: PhysicalSize<u32>,
    panel: PhysicalSize<u32>,
    top_margin: u32,
) -> PhysicalPosition<i32> {
    let free_x = area.width.saturating_sub(panel.width);
    let free_y = area.height.saturating_sub(panel.height);
    PhysicalPosition::new(
        origin.x.saturating_add((free_x / 2) as i32),
        origin.y.saturating_add(top_margin.min(free_y) as i32),
    )
}

#[tauri::command]
pub async fn show_assistant(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the dashboard can show the assistant.".into());
    }
    show_assistant_window(&window)?;
    start_manual_listening(window.app_handle().clone());
    Ok(())
}

pub fn start_manual_listening(app: AppHandle) {
    // The audio worker can call window APIs, so never wait for it on the UI thread.
    tauri::async_runtime::spawn_blocking(move || {
        match audio::begin_followup_capture(&app.state::<audio::AudioService>()) {
            Ok(()) => update_voice_state(&app, "listening", None, None),
            Err(error) => update_voice_state(&app, "error", None, Some(error)),
        }
    });
}

#[tauri::command]
pub async fn resize_assistant(window: WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != ASSISTANT_LABEL || !height.is_finite() {
        return Err("Invalid assistant size request.".into());
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let monitor = window.current_monitor().map_err(|e| e.to_string())?
        .ok_or("The assistant monitor is unavailable.")?;
    let max_height = monitor.work_area().size.height as f64 / scale - TOP_MARGIN;
    let height = height.max(144.0).min(max_height.max(1.0));
    let width = window.inner_size().map_err(|e| e.to_string())?.width as f64 / scale;
    window.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())
}

pub fn show_assistant_window(window: &WebviewWindow) -> Result<(), String> {
    if !settings::nova_enabled(window.app_handle())? {
        return Err("NOVA is off. Enable NOVA before waking the assistant.".into());
    }
    let assistant = window
        .get_webview_window(ASSISTANT_LABEL)
        .ok_or("The assistant window is unavailable. Restart NOVA to try again.")?;
    let monitor = match window.current_monitor() {
        Ok(Some(monitor)) => monitor,
        _ => window
            .primary_monitor()
            .map_err(|error| format!("Could not read the primary monitor: {error}"))?
            .ok_or("No monitor is available for the assistant.")?,
    };
    let work_area = monitor.work_area();
    let config = window
        .app_handle()
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == ASSISTANT_LABEL)
        .ok_or("Assistant window configuration is missing.")?;
    let desired: PhysicalSize<u32> =
        LogicalSize::new(config.width, config.height).to_physical(monitor.scale_factor());
    let size = PhysicalSize::new(
        desired.width.min(work_area.size.width),
        desired.height.min(work_area.size.height),
    );
    let margin = (TOP_MARGIN * monitor.scale_factor()).round() as u32;

    // Move to the target monitor before sizing so DPI changes use that monitor.
    assistant
        .set_position(work_area.position)
        .map_err(|error| format!("Could not move the assistant: {error}"))?;
    assistant
        .set_size(size)
        .map_err(|error| format!("Could not size the assistant: {error}"))?;
    let actual_size = assistant
        .outer_size()
        .map_err(|error| format!("Could not read the assistant size: {error}"))?;
    assistant
        .set_position(top_center(
            work_area.position,
            work_area.size,
            actual_size,
            margin,
        ))
        .map_err(|error| format!("Could not position the assistant: {error}"))?;
    emit_assistant_update(&assistant, "idle", None, None)?;
    assistant
        .show()
        .map_err(|error| format!("Could not show the assistant: {error}"))?;
    assistant
        .set_focus()
        .map_err(|error| format!("Could not focus the assistant: {error}"))
}

pub fn wake_from_voice(app: &AppHandle) {
    let app = app.clone();
    if let Err(error) = app.clone().run_on_main_thread(move || {
        let Some(main) = app.get_webview_window("main") else {
            eprintln!("Wake phrase detected, but the dashboard window is unavailable.");
            return;
        };

        if let Err(error) = show_assistant_window(&main) {
            eprintln!("Wake phrase detected, but the assistant could not be shown: {error}");
            return;
        }

        if let Some(assistant) = app.get_webview_window(ASSISTANT_LABEL) {
            if let Err(error) = emit_assistant_update(&assistant, "listening", None, None) {
                eprintln!("Could not transition the assistant to listening: {error}");
            }
        }
    }) {
        eprintln!("Could not dispatch the voice wake event to the UI thread: {error}");
    }
}

pub fn update_voice_state(
    app: &AppHandle,
    state: &'static str,
    transcript: Option<String>,
    error: Option<String>,
) {
    let app = app.clone();
    if let Err(dispatch_error) = app.clone().run_on_main_thread(move || {
        let Some(assistant) = app.get_webview_window(ASSISTANT_LABEL) else {
            eprintln!("The assistant window is unavailable for voice-state update.");
            return;
        };
        if let Err(emit_error) = emit_assistant_update(&assistant, state, transcript, error) {
            eprintln!("Could not update the assistant voice state: {emit_error}");
        }
    }) {
        eprintln!("Could not dispatch an assistant voice-state update: {dispatch_error}");
    }
}

pub fn is_assistant_visible(app: &AppHandle) -> bool {
    app.get_webview_window(ASSISTANT_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

pub fn hide_assistant_window(app: &AppHandle) -> Result<(), String> {
    if let Err(error) = audio::cancel_voice_session(&app.state::<audio::AudioService>()) {
        eprintln!("Could not reset the voice session while hiding the assistant: {error}");
    }
    let assistant = app
        .get_webview_window(ASSISTANT_LABEL)
        .ok_or("The assistant window is unavailable. Restart NOVA to try again.")?;
    emit_assistant_update(&assistant, "idle", None, None)?;
    assistant
        .hide()
        .map_err(|error| format!("Could not hide the assistant: {error}"))
}

#[tauri::command]
pub async fn hide_assistant(window: WebviewWindow) -> Result<(), String> {
    if window.label() != ASSISTANT_LABEL {
        return Err("Only the assistant can dismiss itself.".into());
    }
    hide_assistant_window(window.app_handle())
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        match window.label() {
            ASSISTANT_LABEL => {
                api.prevent_close();
                if let Err(error) = hide_assistant_window(window.app_handle()) {
                    eprintln!("Could not hide the assistant after a close request: {error}");
                }
            }
            "main" => {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("Could not hide the dashboard after a close request: {error}");
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_on_primary_work_area() {
        assert_eq!(
            top_center((0, 0).into(), (1920, 1040).into(), (420, 144).into(), 24),
            PhysicalPosition::new(750, 24)
        );
    }

    #[test]
    fn respects_negative_monitor_origins_and_taskbar_offsets() {
        assert_eq!(
            top_center(
                (-2560, -200).into(),
                (2560, 1400).into(),
                (420, 144).into(),
                24
            ),
            PhysicalPosition::new(-1490, -176)
        );
        assert_eq!(
            top_center((0, 48).into(), (1920, 1032).into(), (420, 144).into(), 24),
            PhysicalPosition::new(750, 72)
        );
    }

    #[test]
    fn handles_high_dpi_physical_dimensions() {
        let panel = LogicalSize::new(420.0, 144.0).to_physical(1.5);
        assert_eq!(
            top_center((1920, 0).into(), (2560, 1440).into(), panel, 36),
            PhysicalPosition::new(2885, 36)
        );
    }

    #[test]
    fn keeps_margin_inside_small_work_areas() {
        assert_eq!(
            top_center((100, 20).into(), (420, 150).into(), (420, 144).into(), 24),
            PhysicalPosition::new(100, 26)
        );
        assert_eq!(
            top_center((0, 0).into(), (300, 100).into(), (420, 144).into(), 24),
            PhysicalPosition::new(0, 0)
        );
    }
}
