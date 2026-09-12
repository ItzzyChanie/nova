use tauri::Manager;

mod application_control;
mod assistant;
mod audio;
mod command_history;
mod file_control;
mod language_model;
mod settings;
mod speech;
mod tts;
mod developer;
mod project_process;
mod privacy;
mod single_instance;
mod system_control;
mod tool_router;
mod tray;
mod wake_shortcut;
mod window_control;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _instance = match single_instance::acquire() {
        Ok(Some(instance)) => instance,
        Ok(None) => return,
        Err(error) => { eprintln!("Could not acquire the NOVA instance lock: {error}"); return; }
    };
    tauri::Builder::default()
        .manage(audio::AudioService::default())
        .manage(tts::TtsService::default())
        .manage(developer::DeveloperService::default())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--background")
                .app_name("NOVA")
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            if let Err(error) = settings::setup(app) { eprintln!("Settings startup failed: {error}"); }
            if let Err(error) =
                audio::synchronize_wake_engine(app.handle(), &app.state::<audio::AudioService>())
            {
                eprintln!("Could not initialize the local wake engine: {error}");
            }
            if let Err(error) = tray::setup(app) { eprintln!("Tray startup failed: {error}"); }
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
            developer::save_project_profile,
            developer::delete_project_profile,
            developer::get_project_status,
            developer::get_workflows,
            developer::save_workflow,
            developer::delete_workflow,
            tool_router::execute_registered_tool,
            privacy::get_privacy_settings,
            privacy::set_privacy_setting,
            privacy::clear_local_data,
            privacy::get_runtime_info,
            settings::set_nova_enabled,
            settings::set_skill_preference,
            settings::set_wake_sensitivity,
            settings::set_assistant_paused,
            settings::set_speech_model_path,
            settings::set_voice_reply,
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
