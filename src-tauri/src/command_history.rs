use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use std::sync::Mutex;
use crate::local_store::SafeStoreExt;

use crate::tool_router::{ToolResult, ToolResultStatus};

const HISTORY_FILE: &str = "nova-command-history.json";
const HISTORY_KEY: &str = "entries";
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandHistoryEntry {
    pub timestamp: u64,
    #[serde(default)]
    pub duration_millis: u64,
    #[serde(default = "default_source")]
    pub input_source: String,
    pub tool: String,
    pub display_command: String,
    pub result: String,
    pub status: String,
    pub error: Option<String>,
}

fn status_label(status: ToolResultStatus) -> &'static str {
    match status {
        ToolResultStatus::Completed => "completed",
        ToolResultStatus::Rejected => "rejected",
        ToolResultStatus::Denied => "denied",
        ToolResultStatus::ConfirmationRequired => "confirmationRequired",
        ToolResultStatus::Unavailable => "unavailable",
    }
}

fn default_source() -> String { "typed".into() }
thread_local! { static SUPPRESS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
pub struct SuppressHistory(bool);
impl SuppressHistory { pub fn new() -> Self { Self(SUPPRESS.with(|v| v.replace(true))) } }
impl Drop for SuppressHistory { fn drop(&mut self) { SUPPRESS.with(|v|v.set(self.0)); } }
pub fn record_duration(app: &AppHandle, tool: &str, input: String, result: &ToolResult, duration: u64) -> Result<(), String> {
    record_entry(app,tool,input,result,duration,"typed")
}
pub fn record(
    app: &AppHandle,
    tool: &str,
    display_command: String,
    result: &ToolResult,
) -> Result<(), String> {
    record_entry(app,tool,display_command,result,0,"typed")
}
pub fn record_entry(app: &AppHandle, tool: &str, display_command: String, result: &ToolResult, duration: u64, source: &str) -> Result<(),String> {
    crate::local_log::event(if tool == "workflow.run" { "workflow" } else { "tool" }, status_label(result.status));
    if SUPPRESS.with(|v|v.get()) { return Ok(()); }
    let _guard = HISTORY_LOCK.lock().map_err(|_| "Command history is busy.".to_string())?;
    if !crate::settings::command_log(app)? { return Ok(()); }
    let store = app
        .store(HISTORY_FILE)
        .map_err(|error| format!("Could not open command history: {error}"))?;
    let mut entries = store
        .get(HISTORY_KEY)
        .and_then(|value| serde_json::from_value::<Vec<CommandHistoryEntry>>(value).ok())
        .unwrap_or_default();

    let result_message = result
        .data
        .as_ref()
        .and_then(|data| data.get("message"))
        .and_then(|message| message.as_str())
        .unwrap_or_else(|| status_label(result.status))
        .to_string();
    entries.insert(
        0,
        CommandHistoryEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            duration_millis: duration,
            input_source: source.into(),
            tool: tool.to_string(),
            display_command,
            result: result_message,
            status: status_label(result.status).to_string(),
            error: result.error.as_ref().map(|error| error.message.clone()),
        },
    );
    entries.truncate(2000);
    store.set(
        HISTORY_KEY,
        serde_json::to_value(entries)
            .map_err(|error| format!("Could not encode command history: {error}"))?,
    );
    store
        .save()
        .map_err(|error| format!("Could not save command history: {error}"))?;
    let _ = app.emit("command-history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn get_command_history(app: AppHandle) -> Result<Vec<CommandHistoryEntry>, String> {
    let _guard = HISTORY_LOCK.lock().map_err(|_| "Command history is busy.".to_string())?;
    let store = app
        .store(HISTORY_FILE)
        .map_err(|error| format!("Could not open command history: {error}"))?;
    Ok(store
        .get(HISTORY_KEY)
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default())
}

/// Called only after the dashboard's day-specific confirmation modal is accepted.
#[tauri::command]
pub fn clear_command_history_day(window: WebviewWindow, start: u64, end: u64) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Only the dashboard can clear history.".into());
    }
    if end <= start || end - start > 27 * 60 * 60 * 1000 {
        return Err("Choose a valid day to clear.".into());
    }
    let _guard = HISTORY_LOCK.lock().map_err(|_| "Command history is busy.".to_string())?;
    let store = window.app_handle().store(HISTORY_FILE)
        .map_err(|error| format!("Could not open command history: {error}"))?;
    let mut entries: Vec<CommandHistoryEntry> = store.get(HISTORY_KEY)
        .and_then(|value| serde_json::from_value(value).ok()).unwrap_or_default();
    entries.retain(|entry| !in_day(entry.timestamp, start, end));
    store.set(HISTORY_KEY, serde_json::to_value(entries).map_err(|error| error.to_string())?);
    store.save().map_err(|error| format!("Could not clear history: {error}"))?;
    let _ = window.app_handle().emit("command-history-changed", ());
    Ok(())
}

fn in_day(timestamp: u64, start: u64, end: u64) -> bool {
    timestamp >= start && timestamp < end
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clear_day_preserves_adjacent_days() {
        assert!(!in_day(999, 1000, 2000));
        assert!(in_day(1000, 1000, 2000));
        assert!(in_day(1999, 1000, 2000));
        assert!(!in_day(2000, 1000, 2000));
    }
}


pub fn clear_all(app: &AppHandle) -> Result<(),String> {
    let _guard = HISTORY_LOCK.lock().map_err(|_| "Command history is busy.")?;
    let store = app.store(HISTORY_FILE).map_err(|e|e.to_string())?;
    store.clear(); store.save().map_err(|e|e.to_string())?;
    let _ = app.emit("command-history-changed",()); Ok(())
}
pub fn record_natural(app: &AppHandle, result: &crate::language_model::NaturalCommandResult, duration: u64, source: &str) {
    use crate::tool_router::*;
    let fallback = ToolResult { request_id:None,tool:None,category:None,risk:None,permission:None,
        status: if matches!(result.status,crate::language_model::NaturalCommandStatus::Ready | crate::language_model::NaturalCommandStatus::AssistantHidden | crate::language_model::NaturalCommandStatus::NovaEnabled | crate::language_model::NaturalCommandStatus::NovaDisabled) {ToolResultStatus::Completed} else {ToolResultStatus::Rejected}, data:Some(serde_json::json!({"message":result.message})),error:None };
    let tool = result.request.as_ref().and_then(|v|v.get("tool")).and_then(|v|v.as_str()).unwrap_or("natural.command");
    let _ = record_entry(app,tool,result.input.clone(),result.routing.as_ref().unwrap_or(&fallback),duration,source);
}
