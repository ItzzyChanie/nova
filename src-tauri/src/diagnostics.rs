use serde_json::{json, Value};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

fn row(name: &str, status: &str, guidance: &str) -> Value {
    json!({"name":name,"status":status,"guidance":guidance})
}

fn inspect(app: AppHandle) -> Vec<Value> {
    let mut checks = vec![row(
        "Desktop runtime",
        "Ready",
        "Native desktop diagnostics completed.",
    )];
    match crate::audio::list_microphone_devices(app.clone())
        .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()))
    {
        Ok(value) => {
            let devices = value["devices"].as_array();
            let selected = value["selectedDeviceId"].as_str();
            let found = devices.is_some_and(|items| {
                items.iter().any(|device| {
                    selected.map_or(device["isDefault"] == true, |id| device["id"] == id)
                })
            });
            checks.push(row("Microphone", if found { "Detected; capture unverified" } else { "Microphone unavailable" }, "Use Voice settings to test capture. Check Windows microphone privacy permissions and the selected input device."));
        }
        Err(error) => checks.push(row("Microphone", "Unavailable", &error)),
    }
    let wake = serde_json::to_value(crate::audio::get_wake_engine_status(
        app.state::<crate::audio::AudioService>(),
    ))
    .unwrap_or(Value::Null);
    checks.push(row(
        "Wake engine",
        wake["status"].as_str().unwrap_or("Unavailable"),
        wake["message"].as_str().unwrap_or("Open Voice settings."),
    ));
    let wake_files = [
        "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
        "decoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
        "joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
        "tokens.txt",
        "keywords.txt",
    ];
    let wake_present = wake_files.iter().all(|file| {
        crate::runtime_paths::resource(&app, &format!("resources/wake-word/{file}"))
            .is_ok_and(|p| p.is_file())
    });
    checks.push(row(
        "Wake resources",
        if wake_present {
            "Present"
        } else {
            "Model missing"
        },
        "Bundled with NOVA. Reinstall if missing. Presence does not verify recognition accuracy.",
    ));
    match crate::speech::engine_info(&app) {
        Ok(info) => checks.push(row("Whisper model", if info.model_available { "Present" } else { "Model missing" }, "Bundled Tiny English model. Reset a stale override in Voice settings or configure an absolute compatible model directory.")),
        Err(error) => checks.push(row("Whisper model", "Configuration required", &error)),
    }
    checks.push(row("Voice reply", "Playback unverified", "Windows SAPI uses installed David/Zira desktop voices. Test a reply in Voice settings; text remains available if speech fails."));
    let vad = crate::runtime_paths::resource(&app, "resources/vad/silero_vad.onnx")
        .is_ok_and(|p| p.is_file());
    checks.push(row(
        "Speech activity model",
        if vad { "Present" } else { "Model missing" },
        "Reinstall NOVA if missing. Transcription and voice reply need a manual audio test.",
    ));
    let model = crate::language_model::model_info_at("127.0.0.1:11434");
    use crate::language_model::LocalModelStatus as S;
    let (service, model_status) = match model.status {
        S::Loaded => ("Running", "Loaded"),
        S::NotLoaded => ("Running", "Installed; loads on demand"),
        S::ModelMissing => ("Running", "Model missing"),
        S::ServiceUnavailable => (
            if ollama_detected() {
                "Service not running"
            } else {
                "Not detected"
            },
            "Unverified",
        ),
        S::Error => ("Invalid response", "Unverified"),
    };
    crate::local_log::event(
        "ollama",
        match model.status {
            S::Loaded | S::NotLoaded => "model_available",
            S::ModelMissing => "model_missing",
            _ => "unavailable",
        },
    );
    checks.push(row("Ollama", service, "Loopback 127.0.0.1:11434. Start Ollama; if not installed, install it yourself. Unusual installation locations may not be detected."));
    checks.push(row(crate::language_model::LOCAL_MODEL, model_status, "If missing, explicitly run: ollama pull qwen3:1.7b. This downloads a large model. Deterministic tools remain available without it."));
    let stores = [
        "nova-settings.json",
        "nova-projects.json",
        "nova-workflows.json",
        "nova-command-history.json",
    ];
    let readable = app.path().app_data_dir().is_ok_and(|directory| {
        stores
            .iter()
            .all(|file| match std::fs::read(directory.join(file)) {
                Ok(bytes) => {
                    serde_json::from_slice::<Value>(&bytes).is_ok_and(|value| value.is_object())
                }
                Err(error) => error.kind() == std::io::ErrorKind::NotFound,
            })
    });
    checks.push(row("Local settings", if readable { "Readable JSON / new stores" } else { "Unreadable or invalid JSON" }, "Existing store files are checked directly; absent stores use defaults. Persistence across restart needs acceptance testing. Back up data before recovering a corrupt store."));
    match crate::settings::autostart_enabled(&app) {
        Ok(enabled) => checks.push(row(
            "Windows autostart",
            if enabled { "Enabled" } else { "Disabled" },
            "Follows NOVA ON/OFF. Sign-in startup uses --background; verify after signing in.",
        )),
        Err(error) => checks.push(row("Windows autostart", "Unavailable", &error)),
    }
    checks.push(row(
        "Alt+N",
        if app.global_shortcut().is_registered("Alt+N") {
            "Registered"
        } else {
            "Unavailable / conflict"
        },
        "Close the conflicting shortcut owner and restart NOVA. Tray Wake NOVA is also available.",
    ));
    let logging = crate::local_log::directory(&app).is_ok_and(|p| {
        std::fs::OpenOptions::new()
            .append(true)
            .open(p.join("nova.log"))
            .is_ok()
    });
    checks.push(row("Local logs", if logging { "Writable" } else { "Unavailable" }, "LocalAppData/com.nova.assistant/logs. Two files, at most 1 MiB each. Event labels only; no audio, transcripts or command arguments."));
    checks.push(row(
        "Publisher signing",
        "Not configured",
        "This release is unsigned. Windows may show an unknown-publisher or SmartScreen prompt.",
    ));
    checks
}

fn ollama_detected() -> bool {
    if crate::project_process::executable("ollama.exe").is_ok() {
        return true;
    }
    [
        ("LOCALAPPDATA", "Programs/Ollama/ollama.exe"),
        ("ProgramFiles", "Ollama/ollama.exe"),
    ]
    .iter()
    .any(|(key, suffix)| {
        std::env::var_os(key)
            .map(std::path::PathBuf::from)
            .filter(|p| p.is_absolute())
            .is_some_and(|p| p.join(suffix).is_file())
    })
}

#[tauri::command]
pub async fn run_diagnostics(window: WebviewWindow) -> Result<Vec<Value>, String> {
    if window.label() != "main" {
        return Err("Diagnostics require the dashboard.".into());
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || inspect(app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_logs_folder(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Logs require the dashboard.".into());
    }
    let directory = crate::local_log::directory(window.app_handle())?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let root = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or("Windows directory unavailable.")?;
    std::process::Command::new(root.join("explorer.exe"))
        .arg(directory)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
