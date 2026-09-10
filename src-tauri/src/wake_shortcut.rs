use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::assistant;

const WAKE_SHORTCUT: &str = "Alt+N";

pub fn setup(app: &tauri::App) {
    if let Err(error) = app
        .global_shortcut()
        .on_shortcut(WAKE_SHORTCUT, |app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let Some(window) = app.get_webview_window("main") else {
                eprintln!("{WAKE_SHORTCUT} fired, but the dashboard window is unavailable.");
                return;
            };

            if let Err(error) = assistant::show_assistant_window(&window) {
                eprintln!("{WAKE_SHORTCUT} fired, but the assistant could not be shown: {error}");
            } else {
                assistant::start_manual_listening(app.clone());
            }
        })
    {
        eprintln!(
            "Could not register the global wake shortcut {WAKE_SHORTCUT}. It may already be registered by another application: {error}"
        );
    }
}
