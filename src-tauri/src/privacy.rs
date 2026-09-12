use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::local_store::SafeStoreExt;

#[tauri::command]
pub fn get_privacy_settings(app: AppHandle) -> Result<Value, String> {
    Ok(
        json!({"audioStorage":false,"commandLog":crate::settings::command_log(&app)?,"networkAccess":crate::settings::network_access(&app)?}),
    )
}
#[tauri::command]
pub fn set_privacy_setting(
    window: WebviewWindow,
    key: String,
    enabled: bool,
) -> Result<Value, String> {
    if window.label() != "main" || !matches!(key.as_str(), "commandLog" | "networkAccess") {
        return Err("Invalid privacy setting.".into());
    }
    let app = window.app_handle();
    let store = app.store("nova-settings.json").map_err(|e| e.to_string())?;
    let previous = store.get(&key);
    store.set(&key, json!(enabled));
    if let Err(error) = store.save() {
        match previous {
            Some(v) => store.set(&key, v),
            None => {
                store.delete(&key);
            }
        }
        return Err(error.to_string());
    }
    get_privacy_settings(app.clone())
}
#[tauri::command]
pub async fn clear_local_data(window: WebviewWindow, confirmation: String) -> Result<(), String> {
    if window.label() != "main" || confirmation != "CLEAR NOVA DATA" {
        return Err("Type CLEAR NOVA DATA to confirm.".into());
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Disable logging before cancelling in-flight work; a late completion cannot restore history.
        let store = app.store("nova-settings.json").map_err(|e| e.to_string())?;
        store.set("commandLog", json!(false));
        store.save().map_err(|e| e.to_string())?;
        crate::settings::set_nova_enabled(
            app.clone(),
            app.state::<crate::audio::AudioService>(),
            false,
        )?;
        crate::tts::stop(&app);
        app.state::<crate::developer::DeveloperService>().stop_all();
        let _guard = crate::developer::CONFIG_LOCK
            .lock()
            .map_err(|_| "Configuration is busy.")?;
        crate::command_history::clear_all(&app)?;
        for file in ["nova-projects.json", "nova-workflows.json"] {
            let data = app.store(file).map_err(|e| e.to_string())?;
            data.clear();
            data.save().map_err(|e| e.to_string())?;
        }
        crate::language_model::clear_context();
        crate::file_control::clear_cached_results();
        store.clear();
        store.set("novaEnabled", json!(false));
        store.set("commandLog", json!(false));
        store.save().map_err(|e| e.to_string())?;
        crate::audio::synchronize_wake_engine(&app, &app.state::<crate::audio::AudioService>())?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn get_runtime_info(app: AppHandle) -> Value {
    json!({"version":app.package_info().version.to_string(),"platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,"tts":"Windows SAPI (installed David/Zira desktop voice)","audioStorage":false})
}
