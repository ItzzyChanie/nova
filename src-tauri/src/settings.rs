use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;

use crate::assistant;
use crate::audio::{self, AudioService};
use crate::speech::{self, SpeechEngineInfo};
use crate::tool_router::{SkillId, SkillPreferences};

const SETTINGS_FILE: &str = "nova-settings.json";
const NOVA_ENABLED_KEY: &str = "novaEnabled";
const SKILL_PREFERENCES_KEY: &str = "skillPreferences";
const WAKE_SENSITIVITY_KEY: &str = "wakeSensitivity";
const ASSISTANT_PAUSED_KEY: &str = "assistantPaused";
const SPEECH_MODEL_PATH_KEY: &str = "speechModelPath";
const DEFAULT_NOVA_ENABLED: bool = true;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum WakeSensitivity {
    Low,
    Medium,
    High,
}

impl Default for WakeSensitivity {
    fn default() -> Self {
        Self::Medium
    }
}

impl WakeSensitivity {
    pub fn threshold(self) -> f32 {
        match self {
            Self::Low => 0.35,
            Self::Medium => 0.25,
            Self::High => 0.18,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSettings {
    wake_sensitivity: WakeSensitivity,
    assistant_paused: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSettings {
    nova_enabled: bool,
    autostart_enabled: bool,
    skill_preferences: SkillPreferences,
    wake_sensitivity: WakeSensitivity,
    assistant_paused: bool,
}

pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let store = app.store(SETTINGS_FILE)?;
    let mut changed = false;

    if store
        .get(NOVA_ENABLED_KEY)
        .and_then(|value| value.as_bool())
        .is_none()
    {
        store.set(NOVA_ENABLED_KEY, Value::Bool(DEFAULT_NOVA_ENABLED));
        changed = true;
    }

    let stored_skills = store
        .get(SKILL_PREFERENCES_KEY)
        .and_then(|value| serde_json::from_value::<SkillPreferences>(value).ok());
    if stored_skills.is_none() {
        store.set(
            SKILL_PREFERENCES_KEY,
            serde_json::to_value(SkillPreferences::default())?,
        );
        changed = true;
    }

    if store
        .get(WAKE_SENSITIVITY_KEY)
        .and_then(|value| serde_json::from_value::<WakeSensitivity>(value).ok())
        .is_none()
    {
        store.set(
            WAKE_SENSITIVITY_KEY,
            serde_json::to_value(WakeSensitivity::default())?,
        );
        changed = true;
    }

    if store
        .get(ASSISTANT_PAUSED_KEY)
        .and_then(|value| value.as_bool())
        .is_none()
    {
        store.set(ASSISTANT_PAUSED_KEY, Value::Bool(false));
        changed = true;
    }

    if changed {
        store.save()?;
    }

    let enabled = nova_enabled(app.handle())?;
    if let Err(error) = apply_autostart(app.handle(), enabled) {
        eprintln!("Could not synchronize Windows startup with the saved NOVA state: {error}");
    }
    Ok(())
}

pub fn nova_enabled(app: &AppHandle) -> Result<bool, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    Ok(store
        .get(NOVA_ENABLED_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(DEFAULT_NOVA_ENABLED))
}

pub fn wake_sensitivity(app: &AppHandle) -> Result<WakeSensitivity, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    match store.get(WAKE_SENSITIVITY_KEY) {
        Some(value) => serde_json::from_value(value)
            .map_err(|error| format!("Saved wake sensitivity is invalid: {error}")),
        None => Ok(WakeSensitivity::default()),
    }
}

pub fn assistant_paused(app: &AppHandle) -> Result<bool, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    Ok(store
        .get(ASSISTANT_PAUSED_KEY)
        .and_then(|value| value.as_bool())
        .unwrap_or(false))
}
pub fn speech_model_path(app: &AppHandle) -> Result<Option<String>, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    Ok(store
        .get(SPEECH_MODEL_PATH_KEY)
        .and_then(|value| value.as_str().map(str::to_owned))
        .filter(|path| !path.trim().is_empty()))
}

pub fn skill_preferences(app: &AppHandle) -> Result<SkillPreferences, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    match store.get(SKILL_PREFERENCES_KEY) {
        Some(value) => serde_json::from_value(value)
            .map_err(|error| format!("Saved skill preferences are invalid: {error}")),
        None => Ok(SkillPreferences::default()),
    }
}

fn autostart_enabled(app: &AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("Could not read Windows startup status: {error}"))
}

fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<bool, String> {
    let current = autostart_enabled(app)?;
    if !enabled && !current {
        return Ok(false);
    }

    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|error| format!("Could not enable Start with Windows: {error}"))?;
    } else {
        manager
            .disable()
            .map_err(|error| format!("Could not disable Start with Windows: {error}"))?;
    }

    let actual = autostart_enabled(app)?;
    if actual != enabled {
        return Err(format!(
            "Windows startup reported {} after NOVA requested {}.",
            if actual { "enabled" } else { "disabled" },
            if enabled { "enabled" } else { "disabled" }
        ));
    }
    Ok(actual)
}

fn persist_nova_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    store.set(NOVA_ENABLED_KEY, Value::Bool(enabled));
    store
        .save()
        .map_err(|error| format!("Could not save NOVA settings: {error}"))
}

fn persist_voice_settings(
    app: &AppHandle,
    sensitivity: WakeSensitivity,
    paused: bool,
) -> Result<VoiceSettings, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    store.set(
        WAKE_SENSITIVITY_KEY,
        serde_json::to_value(sensitivity)
            .map_err(|error| format!("Could not encode wake sensitivity: {error}"))?,
    );
    store.set(ASSISTANT_PAUSED_KEY, Value::Bool(paused));
    store
        .save()
        .map_err(|error| format!("Could not save voice settings: {error}"))?;
    Ok(VoiceSettings {
        wake_sensitivity: sensitivity,
        assistant_paused: paused,
    })
}

fn startup_settings(app: &AppHandle) -> Result<StartupSettings, String> {
    Ok(StartupSettings {
        nova_enabled: nova_enabled(app)?,
        autostart_enabled: autostart_enabled(app)?,
        skill_preferences: skill_preferences(app)?,
        wake_sensitivity: wake_sensitivity(app)?,
        assistant_paused: assistant_paused(app)?,
    })
}

#[tauri::command]
pub fn get_startup_settings(app: AppHandle) -> Result<StartupSettings, String> {
    startup_settings(&app)
}

#[tauri::command]
pub fn set_nova_enabled(
    app: AppHandle,
    audio_service: State<'_, AudioService>,
    enabled: bool,
) -> Result<StartupSettings, String> {
    update_nova_enabled(&app, &audio_service, enabled)
}

pub fn update_nova_enabled(
    app: &AppHandle,
    audio_service: &AudioService,
    enabled: bool,
) -> Result<StartupSettings, String> {
    let previous = nova_enabled(app)?;
    persist_nova_enabled(app, enabled)?;

    if let Err(error) = apply_autostart(app, enabled) {
        let mut rollback_failures = Vec::new();
        if let Err(rollback_error) = apply_autostart(app, previous) {
            rollback_failures.push(format!("Windows startup: {rollback_error}"));
        }
        if let Err(rollback_error) = persist_nova_enabled(app, previous) {
            rollback_failures.push(format!("saved setting: {rollback_error}"));
        }

        if rollback_failures.is_empty() {
            return Err(format!("{error} The NOVA setting was not changed."));
        }
        return Err(format!(
            "{error} NOVA could not fully restore its previous state ({})",
            rollback_failures.join("; ")
        ));
    }

    if let Err(error) = audio::synchronize_wake_engine(app, audio_service) {
        eprintln!("NOVA state changed, but the wake engine could not be synchronized: {error}");
    }

    if !enabled {
        if let Err(error) = assistant::hide_assistant_window(app) {
            eprintln!("NOVA was disabled, but its assistant window could not be hidden: {error}");
        }
    }

    let current = startup_settings(app)?;
    if let Err(error) = app.emit("nova-settings-changed", current.clone()) {
        eprintln!("NOVA settings changed, but the dashboard could not be notified: {error}");
    }
    Ok(current)
}

#[tauri::command]
pub fn set_wake_sensitivity(
    app: AppHandle,
    audio_service: State<'_, AudioService>,
    sensitivity: WakeSensitivity,
) -> Result<VoiceSettings, String> {
    let saved = persist_voice_settings(&app, sensitivity, assistant_paused(&app)?)?;
    audio::synchronize_wake_engine(&app, &audio_service)?;
    Ok(saved)
}

#[tauri::command]
pub fn set_assistant_paused(
    app: AppHandle,
    audio_service: State<'_, AudioService>,
    paused: bool,
) -> Result<VoiceSettings, String> {
    let saved = persist_voice_settings(&app, wake_sensitivity(&app)?, paused)?;
    audio::synchronize_wake_engine(&app, &audio_service)?;
    Ok(saved)
}

#[tauri::command]
pub fn set_speech_model_path(
    app: AppHandle,
    audio_service: State<'_, AudioService>,
    model_path: Option<String>,
) -> Result<SpeechEngineInfo, String> {
    let normalized = model_path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());
    if let Some(path) = normalized.as_deref() {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("The speech-model path must be absolute.".into());
        }
        if !path.is_dir() {
            return Err(format!(
                "The speech-model directory does not exist: {}",
                path.display()
            ));
        }
        if !speech::model_directory_available(&path) {
            return Err("The speech-model directory must contain the Whisper encoder, decoder, and tokens files.".into());
        }
    }

    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    match normalized {
        Some(path) => store.set(SPEECH_MODEL_PATH_KEY, Value::String(path)),
        None => {
            store.delete(SPEECH_MODEL_PATH_KEY);
        }
    }
    store
        .save()
        .map_err(|error| format!("Could not save the speech-model path: {error}"))?;
    audio::synchronize_wake_engine(&app, &audio_service)?;
    speech::engine_info(&app)
}

#[tauri::command]
pub fn set_skill_preference(
    app: AppHandle,
    skill: SkillId,
    enabled: bool,
) -> Result<SkillPreferences, String> {
    let previous = skill_preferences(&app)?;
    let mut updated = previous.clone();
    updated.set(skill, enabled);

    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    let updated_value = serde_json::to_value(&updated)
        .map_err(|error| format!("Could not encode skill preferences: {error}"))?;
    store.set(SKILL_PREFERENCES_KEY, updated_value);

    if let Err(error) = store.save() {
        if let Ok(previous_value) = serde_json::to_value(previous) {
            store.set(SKILL_PREFERENCES_KEY, previous_value);
        }
        return Err(format!("Could not save skill preferences: {error}"));
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::WakeSensitivity;

    #[test]
    fn higher_sensitivity_uses_a_lower_detection_threshold() {
        assert!(WakeSensitivity::High.threshold() < WakeSensitivity::Medium.threshold());
        assert!(WakeSensitivity::Medium.threshold() < WakeSensitivity::Low.threshold());
    }

    #[test]
    fn sensitivity_serialization_matches_the_frontend_contract() {
        assert_eq!(
            serde_json::to_string(&WakeSensitivity::Low).unwrap(),
            "\"Low\""
        );
        assert_eq!(
            serde_json::to_string(&WakeSensitivity::Medium).unwrap(),
            "\"Medium\""
        );
        assert_eq!(
            serde_json::to_string(&WakeSensitivity::High).unwrap(),
            "\"High\""
        );
    }
}
