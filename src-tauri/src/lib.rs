use tauri::Manager;

mod application_control;
mod assistant;
mod audio;
mod command_history;
mod file_control;
mod language_model;
mod settings;
mod speech;
mod system_control;
mod tool_router;
mod tray;
mod wake_shortcut;
mod window_control;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(audio::AudioService::default())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--background")
                .app_name("NOVA")
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            settings::setup(app)?;
            if let Err(error) =
                audio::synchronize_wake_engine(app.handle(), &app.state::<audio::AudioService>())
            {
                eprintln!("Could not initialize the local wake engine: {error}");
            }
            tray::setup(app)?;
            wake_shortcut::setup(app);

            if !std::env::args_os().any(|argument| argument == "--background") {
                tray::show_dashboard(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            assistant::show_assistant,
            assistant::hide_assistant,
            assistant::resize_assistant,
            settings::get_startup_settings,
            settings::set_nova_enabled,
            settings::set_skill_preference,
            settings::set_wake_sensitivity,
            settings::set_assistant_paused,
            settings::set_speech_model_path,
            speech::get_speech_engine_info,
            language_model::get_local_model_info,
            language_model::interpret_natural_language,
            audio::get_wake_engine_status,
            tool_router::route_tool_request,
            tool_router::execute_confirmed_application_tool,
            command_history::get_command_history,
            command_history::clear_command_history_day,
            tool_router::execute_confirmed_file_tool,
            tool_router::execute_confirmed_system_tool,
            file_control::get_known_projects,
            file_control::add_known_project,
            audio::list_microphone_devices,
            audio::set_selected_microphone,
            audio::start_microphone_test,
            audio::get_microphone_test_state,
            audio::stop_microphone_test
        ])
        .on_window_event(assistant::handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
