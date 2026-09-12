use tauri::{menu::MenuBuilder, tray::TrayIconBuilder, App, Manager};

use crate::assistant;

const TRAY_ID: &str = "nova-tray";
const OPEN_ID: &str = "nova-tray-open";
const WAKE_ID: &str = "nova-tray-wake";
const PAUSE_ID: &str = "nova-tray-pause";
const HIDE_ID: &str = "nova-tray-hide";
const QUIT_ID: &str = "nova-tray-quit";

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = MenuBuilder::new(app)
        .text(OPEN_ID, "Open NOVA")
        .text(WAKE_ID, "Wake NOVA")
        .text(HIDE_ID, "Hide NOVA")
        .text(PAUSE_ID, "Pause / resume assistant")
        .separator()
        .text(QUIT_ID, "Quit NOVA")
        .build()?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("NOVA")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_ID => show_dashboard(app),
            WAKE_ID => wake_assistant(app),
            HIDE_ID => hide_dashboard(app),
            PAUSE_ID => {
                let result = crate::settings::assistant_paused(app).and_then(|paused|
                    crate::settings::set_assistant_paused(app.clone(), app.state::<crate::audio::AudioService>(), !paused));
                if result.is_err() { crate::local_log::event("tray", "pause_failed"); show_dashboard(app); }
            },
            QUIT_ID => { app.state::<crate::developer::DeveloperService>().stop_all(); app.exit(0); },
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    } else {
        eprintln!("NOVA tray icon is unavailable; using the platform default icon.");
    }

    tray.build(app)?;
    crate::local_log::event("tray", "ready");
    Ok(())
}

pub fn show_dashboard(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("Could not open NOVA: the dashboard window is unavailable.");
        return;
    };

    if let Err(error) = window.unminimize() {
        eprintln!("Could not restore the NOVA dashboard: {error}");
    }
    if let Err(error) = window.show() {
        eprintln!("Could not show the NOVA dashboard: {error}");
        return;
    }
    if let Err(error) = window.set_focus() {
        eprintln!("Could not focus the NOVA dashboard: {error}");
    }
}

fn wake_assistant(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("Could not wake NOVA: the dashboard window is unavailable.");
        return;
    };

    if let Err(error) = assistant::show_assistant_window(&window) {
        eprintln!("Could not wake NOVA from the tray: {error}");
    } else { assistant::start_manual_listening(app.clone()); }
}

fn hide_dashboard(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        eprintln!("Could not hide NOVA: the dashboard window is unavailable.");
        return;
    };

    if let Err(error) = window.hide() {
        eprintln!("Could not hide the NOVA dashboard: {error}");
    }
}
