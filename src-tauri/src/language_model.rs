use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::tool_router::{
    ToolCategory, ToolPermission, ToolResult, ToolResultStatus, ToolRiskLevel,
};
use crate::{
    application_control, assistant, audio, command_history, file_control, settings, tool_router,
};

pub const LOCAL_MODEL: &str = "qwen3:1.7b";
pub const LOCAL_RUNTIME: &str = "Ollama";
pub const MODEL_SIZE: &str = "1.4 GB (Q4_K_M)";
pub const RAM_REQUIREMENT: &str = "Approximately 2.5-3.5 GB available RAM";
pub const EXPECTED_LATENCY: &str = "About 0.5-3 s warm; 2-10 s cold on a modern CPU";
const OLLAMA_ADDRESS: &str = "127.0.0.1:11434";
const MAX_INPUT_LENGTH: usize = 500;
const MAX_HTTP_BYTES: u64 = 2 * 1024 * 1024;
const MIN_MODEL_CONFIDENCE: f32 = 0.65;
const CONTEXT_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_CONTEXT_CANDIDATES: usize = 10;

#[derive(Default)]
struct ConversationContext {
    updated: Option<Instant>,
    candidates: Vec<Value>,
    selected: Option<Value>,
    candidate_action: Option<String>,
    last_application: Option<String>,
    last_application_tool: Option<String>,
}

static CONVERSATION_CONTEXT: OnceLock<Mutex<ConversationContext>> = OnceLock::new();

fn conversation_context() -> &'static Mutex<ConversationContext> {
    CONVERSATION_CONTEXT.get_or_init(|| Mutex::new(ConversationContext::default()))
}

fn with_fresh_context<T>(operation: impl FnOnce(&mut ConversationContext) -> T) -> T {
    let mut context = conversation_context()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if context
        .updated
        .is_some_and(|updated| updated.elapsed() > CONTEXT_TTL)
    {
        *context = ConversationContext::default();
    }
    operation(&mut context)
}

const SUPPORTED_TOOLS: &[&str] = &[
    "developer.open_project",
    "developer.start_project",
    "developer.stop_project",
    "developer.open_editor",
    "developer.open_dev_url",
    "workflow.run",
    "application.open",
    "application.close",
    "application.focus",
    "application.is_running",
    "application.list_running",
    "folder.open",
    "folder.find",
    "file.find_by_name",
    "system.get_volume",
    "system.set_volume",
    "system.adjust_volume",
    "system.mute",
    "system.unmute",
    "system.get_battery",
    "system.get_cpu_usage",
    "system.get_memory_usage",
    "system.take_screenshot",
    "window.list",
    "window.focus",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalModelStatus {
    Loaded,
    NotLoaded,
    ModelMissing,
    ServiceUnavailable,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelInfo {
    pub runtime: &'static str,
    pub model: &'static str,
    pub size: &'static str,
    pub ram_requirement: &'static str,
    pub expected_latency: &'static str,
    pub status: LocalModelStatus,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NaturalCommandStatus {
    Ready,
    AssistantHidden,
    NovaEnabled,
    NovaDisabled,
    ClarificationRequired,
    Unsupported,
    Rejected,
    ModelUnavailable,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InterpretationSource {
    Deterministic,
    Ollama,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NaturalCommandResult {
    pub status: NaturalCommandStatus,
    pub source: InterpretationSource,
    pub model: &'static str,
    pub input: String,
    pub normalized_input: String,
    pub confidence: Option<f32>,
    pub request: Option<Value>,
    pub routing: Option<ToolResult>,
    pub message: String,
    pub candidates: Vec<String>,
    pub latency_millis: u128,
}

impl NaturalCommandResult {
    fn terminal(
        status: NaturalCommandStatus,
        source: InterpretationSource,
        input: &str,
        confidence: Option<f32>,
        message: impl Into<String>,
        candidates: Vec<String>,
        started: Instant,
    ) -> Self {
        Self {
            status,
            source,
            model: LOCAL_MODEL,
            input: input.to_string(),
            normalized_input: normalize_transcript(input),
            confidence,
            request: None,
            routing: None,
            message: message.into(),
            candidates,
            latency_millis: started.elapsed().as_millis(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ModelDecision {
    Tool,
    Sequence,
    Clarification,
    Unsupported,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelStep {
    tool: String,
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelOutput {
    decision: ModelDecision,
    tool: String,
    arguments: Value,
    confidence: f32,
    message: String,
    #[serde(default)]
    steps: Vec<ModelStep>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}
#[derive(Debug, Deserialize)]
struct OllamaTag {
    name: Option<String>,
    model: Option<String>,
}
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTag>,
}

pub(crate) fn model_info_at(address: &str) -> LocalModelInfo {
    let base = |status, message| LocalModelInfo {
        runtime: LOCAL_RUNTIME,
        model: LOCAL_MODEL,
        size: MODEL_SIZE,
        ram_requirement: RAM_REQUIREMENT,
        expected_latency: EXPECTED_LATENCY,
        status,
        message,
    };
    let body = match http_request_at(address, "GET", "/api/tags", None, Duration::from_secs(2)) {
        Ok(body) => body,
        Err(error) => {
            return base(
                LocalModelStatus::ServiceUnavailable,
                format!("Ollama is unavailable at localhost:11434: {error}"),
            )
        }
    };
    let tags: OllamaTagsResponse = match serde_json::from_slice(&body) {
        Ok(tags) => tags,
        Err(error) => {
            return base(
                LocalModelStatus::Error,
                format!("Ollama returned an invalid model list: {error}"),
            )
        }
    };
    let available = tags.models.iter().any(|entry| {
        entry.name.as_deref() == Some(LOCAL_MODEL) || entry.model.as_deref() == Some(LOCAL_MODEL)
    });
    if available {
        let loaded = http_request_at(address,"GET","/api/ps",None,Duration::from_secs(2))
            .ok().and_then(|bytes|serde_json::from_slice::<OllamaTagsResponse>(&bytes).ok())
            .is_some_and(|response|response.models.iter().any(|entry|entry.name.as_deref()==Some(LOCAL_MODEL) || entry.model.as_deref()==Some(LOCAL_MODEL)));
        base(
            if loaded { LocalModelStatus::Loaded } else { LocalModelStatus::NotLoaded },
            if loaded { "The configured model is loaded locally." } else { "The configured model is installed; it will load on demand." }.into(),
        )
    } else {
        base(
            LocalModelStatus::ModelMissing,
            format!("Ollama is running, but {LOCAL_MODEL} is not installed."),
        )
    }
}

fn request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("natural-{millis}")
}

fn tool_request(tool: &str, arguments: Value) -> Value {
    json!({ "requestId": request_id(), "tool": tool, "arguments": arguments })
}

fn normalize(input: &str) -> String {
    input
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '%' {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_transcript(input: &str) -> String {
    let normalized = normalize(input);
    let mut words = normalized.split_whitespace().collect::<Vec<_>>();
    if words.starts_with(&["hey", "nova"]) && words.len() > 2 {
        words.drain(..2);
    } else if words.first() == Some(&"nova")
        && words.len() > 1
        && !matches!(words.get(1), Some(&"on" | &"off" | &"of"))
    {
        words.remove(0);
    }
    words
        .into_iter()
        .filter_map(|word| match word {
            // Small, grammatical Taglish vocabulary; entity names are still
            // resolved dynamically rather than through phrase replacements.
            "yung" | "ang" | "aking" | "ko" | "mo" | "paki" | "please" => None,
            "pakiclose" | "isara" => Some("close"),
            "hanapin" => Some("find"),
            "nasaan" => Some("where"),
            "buksan" => Some("open"),
            "tapos" => Some("and"),
            _ => Some(word),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(value: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| value.contains(candidate))
}

fn contextual_request(input: &str) -> Option<DeterministicDecision> {
    let value = normalize_transcript(input);
    if matches!(value.as_str(), "cancel that" | "never mind" | "nevermind") {
        with_fresh_context(|context| *context = ConversationContext::default());
        return Some(DeterministicDecision::Cancelled);
    }
    with_fresh_context(|context| {
        // Resolve only the replacement entity, never the explicitly negated one.
        let correction = value
            .split_once("i meant ")
            .map(|(_, replacement)| replacement)
            .or_else(|| value.strip_prefix("i said "))
            .or_else(|| {
                value.strip_prefix("not ")?;
                input
                    .rsplit_once(',')
                    .map(|(_, replacement)| replacement.trim())
            })
            .or_else(|| {
                let rejected = value.strip_prefix("not ")?;
                let previous = context.last_application.as_deref()?;
                let previous = normalize(previous);
                rejected.strip_prefix(&previous)?.strip_prefix(' ')
            });
        if let Some(replacement) = correction {
            if let (Some(application), Some(tool)) = (
                application_control::resolve_application_mention(replacement),
                context.last_application_tool.as_deref(),
            ) {
                return Some(DeterministicDecision::Request(tool_request(
                    tool,
                    json!({ "application": application }),
                )));
            }
        }

        let ordinal = if value.contains("first") {
            Some(0)
        } else if value.contains("second") {
            Some(1)
        } else if value.contains("third") {
            Some(2)
        } else {
            None
        };
        let candidate = if value.contains("newer") || value.contains("newest") {
            context
                .candidates
                .iter()
                .max_by_key(|candidate| candidate.get("modifiedAt").and_then(Value::as_u64))
                .cloned()
        } else if let Some(index) = ordinal {
            context.candidates.get(index).cloned()
        } else if value.contains("other one") && context.candidates.len() == 2 {
            let selected_id = context
                .selected
                .as_ref()
                .and_then(|candidate| candidate.get("resultId"))
                .and_then(Value::as_str);
            selected_id?;
            context
                .candidates
                .iter()
                .find(|candidate| candidate.get("resultId").and_then(Value::as_str) != selected_id)
                .cloned()
        } else if contains_any(&value, &["open it", "show it", "its folder"]) {
            context
                .selected
                .clone()
                .or_else(|| (context.candidates.len() == 1).then(|| context.candidates[0].clone()))
        } else {
            None
        };
        let candidate = candidate?;
        context.selected = Some(candidate.clone());
        context.updated = Some(Instant::now());
        let result_id = candidate.get("resultId").and_then(Value::as_str)?;
        let tool = if contains_any(&value, &["show", "folder", "where", "reveal"]) {
            "file.reveal_in_explorer"
        } else if value.split_whitespace().any(|word| word == "open") {
            "file.open"
        } else if contains_any(
            &value,
            &[
                "open", "first", "second", "third", "newer", "newest", "other",
            ],
        ) {
            context.candidate_action.as_deref().unwrap_or("file.open")
        } else {
            return None;
        };
        Some(DeterministicDecision::Request(tool_request(
            tool,
            json!({ "resultId": result_id }),
        )))
    })
}

fn parse_number_word(token: &str) -> Option<u8> {
    match token {
        "zero" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        "sixty" => Some(60),
        "seventy" => Some(70),
        "eighty" => Some(80),
        "ninety" => Some(90),
        "hundred" => Some(100),
        _ => None,
    }
}

fn parse_percentage(value: &str) -> Option<u8> {
    for token in value.split_whitespace() {
        if let Ok(number) = token.trim_end_matches('%').parse::<u8>() {
            if number <= 100 {
                return Some(number);
            }
        }
    }
    let words: Vec<&str> = value.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        let Some(first) = parse_number_word(word) else {
            continue;
        };
        if first >= 20 && first < 100 {
            if let Some(second) = words
                .get(index + 1)
                .and_then(|next| parse_number_word(next))
            {
                if second < 10 {
                    return first.checked_add(second).filter(|number| *number <= 100);
                }
            }
        }
        return Some(first);
    }
    None
}
fn application_target(value: &str, actions: &[&str]) -> Option<&'static str> {
    let words = value.split_whitespace().collect::<Vec<_>>();
    let action_index = words.iter().position(|word| actions.contains(word))?;
    let target = words.get(action_index + 1..)?.join(" ");
    application_control::resolve_spoken_application_name(&target)
}

fn file_query_after_action(value: &str, actions: &[&str]) -> Option<String> {
    let words = value.split_whitespace().collect::<Vec<_>>();
    let action_index = words.iter().position(|word| actions.contains(word))?;
    let query = words
        .get(action_index + 1..)?
        .iter()
        .copied()
        .filter(|word| {
            !matches!(
                *word,
                "a" | "an"
                    | "the"
                    | "my"
                    | "file"
                    | "please"
                    | "called"
                    | "named"
                    | "for"
                    | "me"
                    | "where"
                    | "is"
                    | "located"
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    (!query.is_empty()).then_some(query)
}

// Only complete, bounded assistant-control phrases may bypass tool planning.
// Do not fuzzy-match arbitrary targets or infer a missing action from "NOVA".
fn assistant_control(value: &str) -> Option<DeterministicDecision> {
    let mut phrase = value.trim();
    for prefix in ["can you ", "could you ", "would you ", "will you "] {
        if let Some(rest) = phrase.strip_prefix(prefix) { phrase = rest; break; }
    }
    let words = phrase.split_whitespace().filter(|w| *w != "the").collect::<Vec<_>>();
    let phrase = words.join(" ");
    let phrase = phrase.replace("no va", "nova");
    if matches!(phrase.as_str(), "turn off" | "turn of" | "switch off" | "turn off nova" | "turn of nova" | "turn nova off" | "switch off nova" | "disable nova" | "nova off" | "nova of") {
        return Some(DeterministicDecision::DisableNova);
    }
    let words = phrase.split_whitespace().collect::<Vec<_>>();
    if words.len() == 2
        && matches!(words[0], "close" | "cloze" | "clows" | "clothes" | "dismiss" | "hide")
        && matches!(words[1], "nova" | "nover" | "assistant") {
        return Some(DeterministicDecision::HideAssistant);
    }
    if matches!(phrase.as_str(), "nova" | "hey nova" | "close" | "cloze" | "turn" | "off") {
        return Some(DeterministicDecision::Unsupported("I didn't catch the full command. Please say Close NOVA or Turn off.".into()));
    }
    None
}

fn deterministic_request(input: &str) -> DeterministicDecision {
    let value = normalize_transcript(input);
    // Never let substring matching reverse a negated instruction.
    if value.split_whitespace().any(|word| matches!(word, "not" | "never" | "don" | "dont")) {
        return DeterministicDecision::Unsupported("I won't act on that negative instruction. Please state the action you want.".into());
    }
    if let Some(control) = assistant_control(&value) { return control; }
    if contains_any(
        &value,
        &[
            "turn on nova",
            "turn nova on",
            "enable nova",
            "switch on nova",
            "turn up nova",
            "nova up",
            "nova on",
        ],
    ) {
        return DeterministicDecision::EnableNova;
    }
    if contains_any(
        &value,
        &[
            "powershell",
            "cmd exe",
            "command prompt",
            "shell command",
            "run script",
            "delete",
            "shutdown",
            "restart",
            "registry",
            "wifi",
            "wi fi",
            "bluetooth",
        ],
    ) {
        return DeterministicDecision::Unsupported(
            "That request is outside NOVA's approved Phase 14 tool set.".into(),
        );
    }
    if contains_any(
        &value,
        &[
            "what apps are currently running",
            "what applications are running",
            "list running apps",
            "show running apps",
            "list running applications",
        ],
    ) {
        return DeterministicDecision::Request(tool_request("application.list_running", json!({})));
    }

    let open_action = contains_any(
        &value,
        &[
            "open ",
            "launch ",
            "start ",
            "run ",
            "go to ",
            "still not open",
        ],
    );
    if open_action && contains_any(&value, &["my project", "the project", "a project"]) {
        return DeterministicDecision::ProjectAmbiguity;
    }
    if open_action {
        let folders: &[(&str, &[&str])] = &[
            ("Downloads", &["downloads", "download folder"]),
            ("Documents", &["documents", "document folder"]),
            ("Desktop", &["desktop folder", "desktop"]),
            ("Videos", &["videos", "video folder"]),
            ("Music", &["music", "music folder"]),
            ("Projects", &["projects folder", "projects"]),
        ];
        for (folder, aliases) in folders {
            if aliases.iter().any(|alias| value.contains(alias)) {
                return DeterministicDecision::Request(tool_request(
                    "folder.open",
                    json!({ "folder": folder }),
                ));
            }
        }
        if contains_any(&value, &[" search ", " look up ", " browse for "]) {
            return DeterministicDecision::Unsupported(
                "Browser searching is not available yet. NOVA can open the browser as a separate command."
                    .into(),
            );
        }
        if let Some(application) =
            application_target(&value, &["open", "launch", "start", "run", "go"])
                .or_else(|| application_control::resolve_application_mention(&value))
        {
            return DeterministicDecision::Request(tool_request(
                "application.open",
                json!({ "application": application }),
            ));
        }
        if let Some(query) = file_query_after_action(&value, &["open", "launch", "start", "run"]) {
            return DeterministicDecision::Request(tool_request(
                "file.find_by_name",
                json!({ "query": query }),
            ));
        }
    }

    if contains_any(&value, &["close ", "exit ", "quit "]) {
        if contains_any(
            &value,
            &[
                "downloads",
                "download folder",
                "documents",
                "document folder",
                "desktop folder",
                "videos",
                "video folder",
                "music",
                "music folder",
                "file explorer",
                "explorer",
            ],
        ) {
            return DeterministicDecision::Request(tool_request(
                "application.close",
                json!({ "application": "File Explorer" }),
            ));
        }
        if let Some(application) = application_target(&value, &["close", "exit", "quit"])
            .or_else(|| application_control::resolve_application_mention(&value))
        {
            return DeterministicDecision::Request(tool_request(
                "application.close",
                json!({ "application": application }),
            ));
        }
    }

    if contains_any(
        &value,
        &["find ", "locate ", "search for ", "show ", "where "],
    ) {
        if let Some(query) =
            file_query_after_action(&value, &["find", "locate", "search", "show", "where"])
        {
            return DeterministicDecision::Request(tool_request(
                "file.find_by_name",
                json!({ "query": query }),
            ));
        }
    }

    if contains_any(&value, &["focus ", "switch to "]) {
        if let Some(application) = application_target(&value, &["focus", "to"]) {
            return DeterministicDecision::Request(tool_request(
                "application.focus",
                json!({ "application": application }),
            ));
        }
    }

    if contains_any(
        &value,
        &[
            "take a screenshot",
            "take screenshot",
            "capture my screen",
            "capture the screen",
        ],
    ) {
        return DeterministicDecision::Request(tool_request("system.take_screenshot", json!({})));
    }
    if value.contains("unmute") {
        return DeterministicDecision::Request(tool_request("system.unmute", json!({})));
    }
    if value == "mute"
        || contains_any(
            &value,
            &["mute volume", "mute my volume", "mute the volume"],
        )
    {
        return DeterministicDecision::Request(tool_request("system.mute", json!({})));
    }
    let mentions_volume = value.contains("volume") || value.contains("volumn");
    if mentions_volume
        && !value.contains(" to ")
        && contains_any(&value, &["increase", "raise", "decrease", "lower"])
    {
        return parse_percentage(&value).map_or_else(
            || {
                DeterministicDecision::Clarification(
                    "By what percentage should I adjust the volume?".into(),
                )
            },
            |amount| {
                let delta = if contains_any(&value, &["decrease", "lower"]) {
                    -(amount as i16)
                } else {
                    amount as i16
                };
                DeterministicDecision::Request(tool_request(
                    "system.adjust_volume",
                    json!({ "delta": delta }),
                ))
            },
        );
    }
    if mentions_volume && contains_any(&value, &["set", "change", "turn", "raise", "lower"]) {
        return parse_percentage(&value).map_or_else(
            || DeterministicDecision::Clarification("What volume percentage should I use?".into()),
            |volume| {
                DeterministicDecision::Request(tool_request(
                    "system.set_volume",
                    json!({ "volume": volume }),
                ))
            },
        );
    }
    if contains_any(
        &value,
        &[
            "what is my volume",
            "get volume",
            "current volume",
            "volume level",
        ],
    ) {
        return DeterministicDecision::Request(tool_request("system.get_volume", json!({})));
    }
    if value.contains("battery") {
        return DeterministicDecision::Request(tool_request("system.get_battery", json!({})));
    }
    if contains_any(&value, &["cpu usage", "processor usage"]) {
        return DeterministicDecision::Request(tool_request("system.get_cpu_usage", json!({})));
    }
    if contains_any(&value, &["memory usage", "ram usage"]) {
        return DeterministicDecision::Request(tool_request("system.get_memory_usage", json!({})));
    }
    if contains_any(
        &value,
        &["list windows", "show windows", "what windows are open"],
    ) {
        return DeterministicDecision::Request(tool_request("window.list", json!({})));
    }
    DeterministicDecision::UseModel
}

enum DeterministicDecision {
    Request(Value),
    HideAssistant,
    EnableNova,
    DisableNova,
    Cancelled,
    Clarification(String),
    ProjectAmbiguity,
    Unsupported(String),
    UseModel,
}

fn registered_project_request(app: &AppHandle, input: &str) -> Option<Value> {
    crate::developer::match_request(&normalize_transcript(input), &file_control::list_projects(app).ok()?, &crate::developer::workflows(app).ok()?)
}

fn model_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "decision": { "type": "string", "enum": ["tool", "sequence", "clarification", "unsupported"] },
            "tool": {
                "type": "string",
                "enum": SUPPORTED_TOOLS.iter().copied()
                    .chain(std::iter::once("none")).collect::<Vec<_>>()
            },
            "arguments": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "minLength": 1, "maxLength": 100 },
                    "application": { "type": "string", "minLength": 1, "maxLength": 120 },
                    "folder": { "type": "string", "minLength": 1, "maxLength": 260 },
                    "query": { "type": "string", "minLength": 1, "maxLength": 160 },
                    "root": { "type": "string", "minLength": 1, "maxLength": 260 },
                    "extension": { "type": "string", "minLength": 1, "maxLength": 20 },
                    "exact": { "type": "boolean" },
                    "modifiedWithinDays": { "type": "integer", "minimum": 0, "maximum": 3650 },
                    "volume": { "type": "integer", "minimum": 0, "maximum": 100 },
                    "delta": { "type": "integer", "minimum": -100, "maximum": 100 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                    "windowId": { "type": "string", "minLength": 1, "maxLength": 80 }
                },
                "additionalProperties": false
            },
            "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
            "message": { "type": "string", "maxLength": 300 }
            ,"steps": {
                "type": "array",
                "maxItems": 4,
                "items": {
                    "type": "object",
                    "properties": {
                        "tool": { "type": "string", "enum": SUPPORTED_TOOLS },
                        "arguments": { "type": "object" }
                    },
                    "required": ["tool", "arguments"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["decision", "tool", "arguments", "confidence", "message"],
        "additionalProperties": false
    })
}

#[allow(dead_code)]
fn model_prompt(projects: &[file_control::ProjectRecord]) -> String {
    let project_names = projects
        .iter()
        .map(|project| project.name.as_str())
        .collect::<Vec<_>>();
    let application_names = application_control::known_application_names();
    let recent_context = with_fresh_context(|context| {
        json!({
            "lastApplication": context.last_application,
            "recentCandidates": context.candidates.iter().filter_map(|candidate| {
                candidate.get("name").and_then(Value::as_str)
            }).take(MAX_CONTEXT_CANDIDATES).collect::<Vec<_>>()
        })
        .to_string()
    });
    format!(
        "You are NOVA's intent classifier for noisy speech transcripts. Return only the supplied JSON schema. Never produce shell, PowerShell, cmd, scripts, code, or raw filesystem operations. Select only an allowed tool. Correct obvious phonetic transcription errors to a known application only when the match is clear; otherwise request clarification. Examples: Notebagged or note pad means Notepad; Visual Studio coat means Visual Studio Code; power point means PowerPoint; cap cut means CapCut. The arguments field contains tool fields DIRECTLY and must never contain another arguments field. Example: application.open uses arguments={{\"application\":\"Notepad\"}}. Use decision=clarification and tool=none when a required target is ambiguous or missing. Use decision=unsupported and tool=none for unavailable or unsafe requests. Exact shapes: application.open/close/focus/is_running {{application:string}}; application.list_running {{}}; folder.open {{folder:string}}; folder.find/file.find_by_name {{query:string, optional root:string, extension:string, exact:boolean, modifiedWithinDays:integer}}; system.set_volume {{volume:integer 0..100}}; system.get_volume/mute/unmute/get_battery/get_cpu_usage/get_memory_usage/take_screenshot {{}}; window.list {{}}; window.focus {{title:string, optional exact:boolean}}. Known applications: {:?}. Known project names: {:?}.",
        application_names,
        format!("{project_names:?}. The user speaks naturally; grammar may be imperfect, Filipino-accented English is normal, transcripts may contain minor recognition errors, and basic Taglish may appear. Infer intent from the whole utterance and bounded context. Preserve the requested action. Prefer known local entities only when evidence is strong; ask a concise clarification when ambiguity materially changes an action. For a compound request, use decision=sequence, tool=none, and 2-4 ordered steps; never place dependent steps after an unsupported operation. Bounded recent context: {recent_context}")
    )
}

fn conversational_model_prompt(projects: &[file_control::ProjectRecord]) -> String {
    let projects = projects
        .iter()
        .map(|project| project.name.as_str())
        .collect::<Vec<_>>();
    let applications = application_control::known_application_names();
    let context = with_fresh_context(|context| {
        json!({
            "lastApplication": context.last_application,
            "recentCandidates": context.candidates.iter().filter_map(|candidate| {
                candidate.get("name").and_then(Value::as_str)
            }).take(MAX_CONTEXT_CANDIDATES).collect::<Vec<_>>()
        })
        .to_string()
    });
    format!(
        "You are NOVA's local conversational intent planner. The user speaks naturally; grammar may be imperfect, Filipino-accented English is normal, minor speech-recognition errors may appear, and basic Taglish may appear. Understand paraphrases, corrections, indirect wording, and bounded follow-up references from the whole utterance. Preserve the requested action and target. Prefer a known local entity only when evidence is strong. Ask a concise clarification when ambiguity materially changes an action, especially close or sensitive actions. Return only the supplied JSON schema. Never invent tools. Never output shell, PowerShell, cmd, scripts, code, or raw filesystem operations. Tool arguments must be direct fields, not nested arguments. Use clarification with tool none when a required target is missing or ambiguous, and unsupported with tool none for unavailable or unsafe requests. For a compound request use sequence with tool none and 2-4 ordered steps. Allowed tools and their exact arguments: developer.open_project, developer.start_project, developer.stop_project, developer.open_editor and developer.open_dev_url use name string for a known saved project; workflow.run uses name string for a saved workflow. These tools never accept commands or paths. application.open, close, focus, is_running use application string; application.list_running uses no arguments; folder.open uses folder string; folder.find and file.find_by_name use query with optional root, extension, exact, modifiedWithinDays; volume set uses volume 0-100; volume adjust uses nonzero delta -100 to 100; mute, unmute, volume query, battery, CPU, memory, screenshot, and window list use no arguments; window focus uses title and optional exact. Known applications: {applications:?}. Known projects: {projects:?}. Bounded recent local context: {context}."
    )
}

fn ollama_output_at(
    address: &str,
    input: &str,
    projects: &[file_control::ProjectRecord],
) -> Result<String, String> {
    let body = json!({
        "model": LOCAL_MODEL,
        "stream": false,
        "think": false,
        "keep_alive": "30s",
        "format": model_schema(),
        "options": { "temperature": 0, "seed": 42, "num_ctx": 2048, "num_predict": 128 },
        "messages": [
            { "role": "system", "content": conversational_model_prompt(projects) },
            { "role": "user", "content": "Open Notebagged" },
            { "role": "assistant", "content": "{\"decision\":\"tool\",\"tool\":\"application.open\",\"arguments\":{\"application\":\"Notepad\"},\"confidence\":0.86,\"message\":\"Open Notepad.\"}" },
            { "role": "user", "content": "Launch Google Chrome" },
            { "role": "assistant", "content": "{\"decision\":\"tool\",\"tool\":\"application.open\",\"arguments\":{\"application\":\"Google Chrome\"},\"confidence\":0.99,\"message\":\"Open Google Chrome.\"}" },
            { "role": "user", "content": "Open my project" },
            { "role": "assistant", "content": "{\"decision\":\"clarification\",\"tool\":\"none\",\"arguments\":{},\"confidence\":0.95,\"message\":\"Which project should NOVA open?\"}" },
            { "role": "user", "content": input }
        ]
    });
    let encoded = serde_json::to_vec(&body)
        .map_err(|error| format!("Could not encode the local-model request: {error}"))?;
    let response = http_request_at(
        address,
        "POST",
        "/api/chat",
        Some(&encoded),
        Duration::from_secs(8),
    )?;
    let response: OllamaChatResponse = serde_json::from_slice(&response)
        .map_err(|error| format!("Ollama returned an invalid response envelope: {error}"))?;
    Ok(response.message.content)
}

fn parse_model_output(content: &str) -> Result<ModelOutput, String> {
    let mut output: ModelOutput = serde_json::from_str(content)
        .map_err(|error| format!("The model returned malformed structured output: {error}"))?;
    if output.arguments.to_string().contains("/no_think") {
        return Err(
            "The model returned an internal control token instead of a valid tool argument.".into(),
        );
    }
    if !output.confidence.is_finite() || !(0.0..=1.0).contains(&output.confidence) {
        return Err("The model confidence must be between 0 and 1.".into());
    }
    if output.message.len() > 300 {
        return Err("The model message exceeds the 300-character limit.".into());
    }
    match output.decision {
        ModelDecision::Tool => {
            if !SUPPORTED_TOOLS.contains(&output.tool.as_str()) {
                return Err(format!(
                    "The model proposed unsupported tool '{}'.",
                    output.tool
                ));
            }
        }
        ModelDecision::Sequence => {
            if output.tool != "none" || !(2..=4).contains(&output.steps.len()) {
                return Err("A tool sequence must use tool=none and contain 2-4 steps.".into());
            }
            for step in &output.steps {
                if !SUPPORTED_TOOLS.contains(&step.tool.as_str()) {
                    return Err(format!(
                        "The model proposed unsupported tool '{}'.",
                        step.tool
                    ));
                }
                tool_router::validate_model_request(tool_request(
                    &step.tool,
                    step.arguments.clone(),
                ))
                .map_err(|_| {
                    format!("The model proposed invalid arguments for '{}'.", step.tool)
                })?;
            }
        }
        ModelDecision::Clarification | ModelDecision::Unsupported => {
            if output.tool != "none" {
                return Err("A non-tool decision must use tool=none.".into());
            }
            // Some small models populate schema-allowed argument fields even
            // when selecting no tool. Discarding them is safe because no tool
            // request is constructed for a non-tool decision.
            output.arguments = json!({});
        }
    }
    Ok(output)
}
fn route_request(
    app: &AppHandle,
    input: &str,
    request: Value,
    source: InterpretationSource,
    confidence: f32,
    message: String,
    started: Instant,
) -> NaturalCommandResult {
    if let Err(rejection) = tool_router::validate_model_request(request.clone()) {
        let rejection_message = rejection
            .error
            .as_ref()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "The proposed request failed native validation.".into());
        return NaturalCommandResult {
            status: NaturalCommandStatus::Rejected,
            source,
            model: LOCAL_MODEL,
            input: input.to_string(),
            normalized_input: normalize_transcript(input),
            confidence: Some(confidence),
            request: Some(request),
            routing: Some(rejection),
            message: rejection_message,
            candidates: Vec::new(),
            latency_millis: started.elapsed().as_millis(),
        };
    }

    let preferences = match settings::skill_preferences(app) {
        Ok(value) => value,
        Err(error) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Error,
                source,
                input,
                Some(confidence),
                error,
                Vec::new(),
                started,
            )
        }
    };
    let enabled = match settings::nova_enabled(app) {
        Ok(value) => value,
        Err(error) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Error,
                source,
                input,
                Some(confidence),
                error,
                Vec::new(),
                started,
            )
        }
    };
    let routing = tool_router::route(request.clone(), &preferences, enabled);
    if routing.status != ToolResultStatus::ConfirmationRequired {
        let tool = request
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or("natural.command");
        if let Err(error) = command_history::record(app, tool, input.to_string(), &routing) {
            eprintln!("Could not record natural-language command history: {error}");
        }
    }
    let routed_message = routing
        .error
        .as_ref()
        .map(|error| error.message.clone())
        .or_else(|| {
            routing
                .data
                .as_ref()?
                .get("message")?
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or(message);
    NaturalCommandResult {
        status: if matches!(
            routing.status,
            ToolResultStatus::Rejected | ToolResultStatus::Denied
        ) {
            NaturalCommandStatus::Rejected
        } else {
            NaturalCommandStatus::Ready
        },
        source,
        model: LOCAL_MODEL,
        input: input.to_string(),
        normalized_input: normalize_transcript(input),
        confidence: Some(confidence),
        request: Some(request),
        routing: Some(routing),
        message: routed_message,
        candidates: Vec::new(),
        latency_millis: started.elapsed().as_millis(),
    }
}

fn route_sequence(
    app: &AppHandle,
    input: &str,
    steps: Vec<ModelStep>,
    confidence: f32,
    message: String,
    started: Instant,
) -> NaturalCommandResult {
    let requests = steps
        .into_iter()
        .map(|step| tool_request(&step.tool, step.arguments))
        .collect::<Vec<_>>();
    if !(2..=4).contains(&requests.len()) {
        return NaturalCommandResult::terminal(
            NaturalCommandStatus::Rejected,
            InterpretationSource::Ollama,
            input,
            Some(confidence),
            "A tool sequence must contain 2-4 steps.",
            Vec::new(),
            started,
        );
    }
    let preferences = match settings::skill_preferences(app) {
        Ok(value) => value,
        Err(error) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Error,
                InterpretationSource::Ollama,
                input,
                Some(confidence),
                error,
                Vec::new(),
                started,
            )
        }
    };
    let enabled = settings::nova_enabled(app).unwrap_or(false);
    let mut preview_results = Vec::new();
    for request in &requests {
        if let Err(rejection) = tool_router::validate_model_request(request.clone()) {
            return NaturalCommandResult {
                status: NaturalCommandStatus::Rejected,
                source: InterpretationSource::Ollama,
                model: LOCAL_MODEL,
                input: input.to_string(),
                normalized_input: normalize_transcript(input),
                confidence: Some(confidence),
                request: Some(json!({ "type": "tool_sequence", "steps": requests })),
                routing: Some(rejection),
                message: "A sequence step failed native schema validation.".into(),
                candidates: Vec::new(),
                latency_millis: started.elapsed().as_millis(),
            };
        }
        let preview = tool_router::preflight(request.clone(), &preferences, enabled);
        if matches!(
            preview.status,
            ToolResultStatus::Denied | ToolResultStatus::Rejected
        ) {
            return NaturalCommandResult {
                status: NaturalCommandStatus::Rejected,
                source: InterpretationSource::Ollama,
                model: LOCAL_MODEL,
                input: input.to_string(),
                normalized_input: normalize_transcript(input),
                confidence: Some(confidence),
                request: Some(json!({ "type": "tool_sequence", "steps": requests })),
                message: preview
                    .error
                    .as_ref()
                    .map(|error| error.message.clone())
                    .unwrap_or_else(|| "A sequence step was denied.".into()),
                routing: Some(preview),
                candidates: Vec::new(),
                latency_millis: started.elapsed().as_millis(),
            };
        }
        preview_results.push(preview);
    }
    let waiting = preview_results
        .iter()
        .any(|result| result.status == ToolResultStatus::ConfirmationRequired);
    NaturalCommandResult {
        status: NaturalCommandStatus::ClarificationRequired,
        source: InterpretationSource::Ollama,
        model: LOCAL_MODEL,
        input: input.to_string(),
        normalized_input: normalize_transcript(input),
        confidence: Some(confidence),
        request: Some(json!({ "type": "tool_sequence", "steps": requests })),
        routing: Some(ToolResult {
            request_id: None,
            tool: None,
            category: Some(ToolCategory::Workflow),
            risk: Some(ToolRiskLevel::Sensitive),
            permission: Some(ToolPermission::Confirm),
            status: if waiting {
                ToolResultStatus::ConfirmationRequired
            } else {
                ToolResultStatus::Completed
            },
            data: Some(json!({ "stepCount": preview_results.len() })),
            error: None,
        }),
        message: format!(
            "{} The {}-step plan is validated but requires a dedicated whole-plan confirmation before execution.",
            message,
            preview_results.len()
        ),
        candidates: Vec::new(),
        latency_millis: started.elapsed().as_millis(),
    }
}

fn interpret_at(app: &AppHandle, input: &str, address: &str) -> NaturalCommandResult {
    let started = Instant::now();
    let input = input.trim();
    if input.is_empty() || input.len() > MAX_INPUT_LENGTH || input.chars().any(char::is_control) {
        return NaturalCommandResult::terminal(
            NaturalCommandStatus::Rejected,
            InterpretationSource::Deterministic,
            input,
            None,
            "Commands must contain 1-500 printable characters.",
            Vec::new(),
            started,
        );
    }

    if let Some(decision) = contextual_request(input) {
        match decision {
            DeterministicDecision::Request(request) => {
                return route_request(
                    app,
                    input,
                    request,
                    InterpretationSource::Deterministic,
                    1.0,
                    "Resolved from recent local conversation context.".into(),
                    started,
                )
            }
            DeterministicDecision::Cancelled => {
                return NaturalCommandResult::terminal(
                    NaturalCommandStatus::Ready,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    "Okay, cancelled.",
                    Vec::new(),
                    started,
                )
            }
            _ => {}
        }
    }

    if let Some(request) = registered_project_request(app, input) {
        return route_request(
            app,
            input,
            request,
            InterpretationSource::Deterministic,
            1.0,
            "Matched an approved project folder.".into(),
            started,
        );
    }

    match deterministic_request(input) {
        DeterministicDecision::HideAssistant => {
            return match assistant::hide_assistant_window(app) {
                Ok(()) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::AssistantHidden,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    "The floating assistant was hidden.",
                    Vec::new(),
                    started,
                ),
                Err(error) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::Error,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    error,
                    Vec::new(),
                    started,
                ),
            };
        }
        DeterministicDecision::EnableNova => {
            let service = app.state::<audio::AudioService>();
            return match settings::update_nova_enabled(app, &service, true) {
                Ok(_) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::NovaEnabled,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    "NOVA is now on. Wake detection, Alt+N, and enabled assistant tools are active.",
                    Vec::new(),
                    started,
                ),
                Err(error) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::Error,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    error,
                    Vec::new(),
                    started,
                ),
            };
        }
        DeterministicDecision::DisableNova => {
            let service = app.state::<audio::AudioService>();
            return match settings::update_nova_enabled(app, &service, false) {
                Ok(_) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::NovaDisabled,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    "NOVA is now off. Wake detection, Alt+N, and assistant tools are disabled.",
                    Vec::new(),
                    started,
                ),
                Err(error) => NaturalCommandResult::terminal(
                    NaturalCommandStatus::Error,
                    InterpretationSource::Deterministic,
                    input,
                    Some(1.0),
                    error,
                    Vec::new(),
                    started,
                ),
            };
        }
        DeterministicDecision::Cancelled => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Ready,
                InterpretationSource::Deterministic,
                input,
                Some(1.0),
                "Okay, cancelled.",
                Vec::new(),
                started,
            )
        }
        DeterministicDecision::Request(request) => {
            return route_request(
                app,
                input,
                request,
                InterpretationSource::Deterministic,
                1.0,
                "Matched a deterministic local command.".into(),
                started,
            )
        }
        DeterministicDecision::Clarification(message) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::ClarificationRequired,
                InterpretationSource::Deterministic,
                input,
                Some(1.0),
                message,
                Vec::new(),
                started,
            )
        }
        DeterministicDecision::ProjectAmbiguity => {
            let projects = file_control::list_projects(app).unwrap_or_default();
            if projects.len() == 1 {
                return route_request(
                    app,
                    input,
                    tool_request("folder.open", json!({ "folder": projects[0].name })),
                    InterpretationSource::Deterministic,
                    1.0,
                    "Resolved the only registered project.".into(),
                    started,
                );
            }
            let candidates = projects
                .into_iter()
                .map(|project| project.name)
                .collect::<Vec<_>>();
            let message = if candidates.is_empty() {
                "No known projects are registered. Add one from File & folder tools."
            } else {
                "Which project should NOVA open?"
            };
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::ClarificationRequired,
                InterpretationSource::Deterministic,
                input,
                Some(1.0),
                message,
                candidates,
                started,
            );
        }
        DeterministicDecision::Unsupported(message) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Unsupported,
                InterpretationSource::Deterministic,
                input,
                Some(1.0),
                message,
                Vec::new(),
                started,
            )
        }
        DeterministicDecision::UseModel => {}
    }

    let projects = file_control::list_projects(app).unwrap_or_default();
    let content = match ollama_output_at(address, input, &projects) {
        Ok(content) => content,
        Err(error) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::ModelUnavailable,
                InterpretationSource::Ollama,
                input,
                None,
                format!(
                "I couldn't understand that command in time. Please repeat it with the action and target. Manual tools still work. ({error})"
            ),
                Vec::new(),
                started,
            )
        }
    };
    let output = match parse_model_output(&content) {
        Ok(output) => output,
        Err(error) => {
            return NaturalCommandResult::terminal(
                NaturalCommandStatus::Rejected,
                InterpretationSource::Ollama,
                input,
                None,
                error,
                Vec::new(),
                started,
            )
        }
    };
    if output.confidence < MIN_MODEL_CONFIDENCE {
        return NaturalCommandResult::terminal(
            NaturalCommandStatus::ClarificationRequired,
            InterpretationSource::Ollama,
            input,
            Some(output.confidence),
            "I am not confident enough to choose a tool. Please be more specific.",
            Vec::new(),
            started,
        );
    }
    match output.decision {
        ModelDecision::Tool => route_request(
            app,
            input,
            tool_request(&output.tool, output.arguments),
            InterpretationSource::Ollama,
            output.confidence,
            output.message,
            started,
        ),
        ModelDecision::Sequence => route_sequence(
            app,
            input,
            output.steps,
            output.confidence,
            output.message,
            started,
        ),
        ModelDecision::Clarification => NaturalCommandResult::terminal(
            NaturalCommandStatus::ClarificationRequired,
            InterpretationSource::Ollama,
            input,
            Some(output.confidence),
            output.message,
            Vec::new(),
            started,
        ),
        ModelDecision::Unsupported => NaturalCommandResult::terminal(
            NaturalCommandStatus::Unsupported,
            InterpretationSource::Ollama,
            input,
            Some(output.confidence),
            output.message,
            Vec::new(),
            started,
        ),
    }
}

fn http_request_at(
    address: &str,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let address = SocketAddr::from_str(address)
        .map_err(|error| format!("Invalid local endpoint: {error}"))?;
    if !address.ip().is_loopback() {
        return Err("The language-model endpoint must be loopback-only.".into());
    }
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(1))
        .map_err(|error| format!("connection failed: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("Could not set model timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("Could not set model write timeout: {error}"))?;
    let body = body.unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost:11434\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("Could not send the local-model request: {error}"))?;
    // Bound the whole response, not each individual read: trickling bytes
    // must not extend an unclear command indefinitely.
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let remaining = deadline.checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or("Local understanding timed out. Please repeat a short command.")?;
        stream.set_read_timeout(Some(remaining)).map_err(|error| error.to_string())?;
        let count = stream.read(&mut chunk)
            .map_err(|error| format!("Could not read the local-model response: {error}"))?;
        if count == 0 { break; }
        if bytes.len() as u64 + count as u64 > MAX_HTTP_BYTES {
            return Err("The local-model response exceeded its size limit.".into());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("Ollama returned an invalid HTTP response.")?;
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let status = headers.lines().next().unwrap_or_default();
    if !status.contains(" 200 ") {
        let message = String::from_utf8_lossy(&bytes[header_end + 4..]);
        return Err(format!("Ollama returned {status}: {}", message.trim()));
    }
    let payload = &bytes[header_end + 4..];
    if headers
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(payload)
    } else {
        Ok(payload.to_vec())
    }
}

fn decode_chunked(payload: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = 0usize;
    let mut decoded = Vec::new();
    loop {
        let line_end = payload[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("Invalid chunked response.")?
            + cursor;
        let size_text = std::str::from_utf8(&payload[cursor..line_end])
            .map_err(|_| "Invalid chunk size encoding.")?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or_default(), 16)
            .map_err(|_| "Invalid chunk size.")?;
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let end = cursor
            .checked_add(size)
            .filter(|end| *end <= payload.len())
            .ok_or("Truncated chunked response.")?;
        decoded.extend_from_slice(&payload[cursor..end]);
        cursor = end
            .checked_add(2)
            .filter(|end| *end <= payload.len())
            .ok_or("Truncated chunk terminator.")?;
    }
    Ok(decoded)
}
#[tauri::command]
pub async fn get_local_model_info() -> LocalModelInfo {
    tauri::async_runtime::spawn_blocking(|| model_info_at(OLLAMA_ADDRESS))
        .await
        .unwrap_or_else(|error| LocalModelInfo {
            runtime: LOCAL_RUNTIME,
            model: LOCAL_MODEL,
            size: MODEL_SIZE,
            ram_requirement: RAM_REQUIREMENT,
            expected_latency: EXPECTED_LATENCY,
            status: LocalModelStatus::Error,
            message: format!("Could not inspect the local model: {error}"),
        })
}

#[tauri::command]
pub async fn interpret_natural_language(app: AppHandle, input: String) -> NaturalCommandResult {
    let fallback_input = input.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = Instant::now();
        let suppress = command_history::SuppressHistory::new();
        let result = interpret_at(&app, &input, OLLAMA_ADDRESS);
        drop(suppress);
        command_history::record_natural(&app,&result,started.elapsed().as_millis() as u64,"typed");
        result
    })
        .await
        .unwrap_or_else(|error| {
            NaturalCommandResult::terminal(
                NaturalCommandStatus::Error,
                InterpretationSource::Deterministic,
                &fallback_input,
                None,
                format!("The command interpreter stopped unexpectedly: {error}"),
                Vec::new(),
                Instant::now(),
            )
        })
}

fn voice_request_is_auto_confirmable(request: &Value) -> bool {
    matches!(
        request.get("tool").and_then(Value::as_str),
        Some(
            "application.open"
                | "application.close"
                | "application.focus"
                | "folder.open"
                | "file.open"
                | "file.find_by_name"
                | "file.reveal_in_explorer"
                | "system.set_volume"
                | "system.adjust_volume"
                | "system.mute"
                | "system.unmute"
                | "system.take_screenshot"
                | "window.focus"
        )
    )
}

fn single_file_candidate<'a>(candidates: &'a [Value], query: &str) -> Option<&'a Value> {
    if candidates.len() == 1 {
        return candidates.first();
    }
    let exact_stem_matches = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .get("name")
                .and_then(Value::as_str)
                .and_then(|name| Path::new(name).file_stem())
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.eq_ignore_ascii_case(query))
        })
        .collect::<Vec<_>>();
    (exact_stem_matches.len() == 1)
        .then(|| exact_stem_matches.first().copied())
        .flatten()
}

fn execute_safe_voice_confirmation(app: &AppHandle, result: &mut NaturalCommandResult) {
    let waiting = result
        .routing
        .as_ref()
        .is_some_and(|routing| routing.status == ToolResultStatus::ConfirmationRequired);
    let Some(request) = result.request.as_ref() else {
        return;
    };
    if waiting && request.get("tool").and_then(Value::as_str).is_some_and(|t| t.starts_with("developer.") || t == "workflow.run") {
        let routing = tool_router::execute_registered(app, request.clone(), tool_router::RegisteredOrigin::Voice);
        result.message = routing.error.as_ref().map(|e|e.message.clone()).or_else(||routing.data.as_ref()?.get("message")?.as_str().map(str::to_owned)).unwrap_or("Registered action finished.".into());
        result.status = if routing.status == ToolResultStatus::Completed { NaturalCommandStatus::Ready } else if routing.status == ToolResultStatus::ConfirmationRequired { NaturalCommandStatus::ClarificationRequired } else { NaturalCommandStatus::Rejected };
        result.routing = Some(routing);
        return;
    }
    if !waiting || !voice_request_is_auto_confirmable(request) {
        return;
    }

    // The spoken imperative is explicit confirmation only for the narrow
    // non-destructive allowlist above. Application close posts WM_CLOSE so the
    // application retains its own unsaved-work prompt. Destructive tools remain denied.
    let requested_tool = request
        .get("tool")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut routing = match requested_tool.as_deref() {
        Some(tool) if tool.starts_with("application.") => {
            tool_router::execute_confirmed_application_tool(app.clone(), request.clone(), true)
        }
        Some(tool) if tool.starts_with("file.") || tool.starts_with("folder.") => {
            tool_router::execute_confirmed_file_tool(app.clone(), request.clone(), true)
        }
        Some(tool) if tool.starts_with("system.") || tool.starts_with("window.") => {
            tool_router::execute_confirmed_system_tool(app.clone(), request.clone(), true)
        }
        _ => return,
    };

    if requested_tool.as_deref() == Some("file.find_by_name")
        && routing.status == ToolResultStatus::Completed
    {
        let candidates = routing
            .data
            .as_ref()
            .and_then(|data| data.get("candidates"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let query = request
            .get("arguments")
            .and_then(|arguments| arguments.get("query"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let normalized_input = normalize_transcript(&result.input);
        let candidate_action =
            if contains_any(&normalized_input, &["show", "where", "locate", "folder"]) {
                "file.reveal_in_explorer"
            } else {
                "file.open"
            };
        with_fresh_context(|context| {
            context.candidates = candidates
                .iter()
                .take(MAX_CONTEXT_CANDIDATES)
                .cloned()
                .collect();
            context.selected = None;
            context.candidate_action = Some(candidate_action.to_string());
            context.updated = Some(Instant::now());
        });
        let single_match = single_file_candidate(&candidates, query);
        if let Some(candidate) = single_match {
            let Some(result_id) = candidate.get("resultId").and_then(Value::as_str) else {
                result.status = NaturalCommandStatus::Rejected;
                result.message = "The file search returned an invalid result identifier.".into();
                result.routing = Some(routing);
                return;
            };
            with_fresh_context(|context| context.selected = Some(candidate.clone()));
            let explicit_open = contains_any(&normalized_input, &["open ", "launch ", "start "]);
            let explicit_reveal = candidate_action == "file.reveal_in_explorer";
            if explicit_open || explicit_reveal {
                let target_request =
                    tool_request(candidate_action, json!({ "resultId": result_id }));
                routing = tool_router::execute_confirmed_file_tool(
                    app.clone(),
                    target_request.clone(),
                    true,
                );
                result.request = Some(target_request);
            } else {
                result.message = candidate
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|path| format!("Found it. It’s in {path}."))
                    .unwrap_or_else(|| "Found one matching file.".into());
                result.status = NaturalCommandStatus::Ready;
                result.routing = Some(routing);
                return;
            }
        } else if candidates.is_empty() {
            result.status = NaturalCommandStatus::Rejected;
            result.message = "No matching file was found inside NOVA's approved folders.".into();
            result.routing = Some(routing);
            return;
        } else {
            result.status = NaturalCommandStatus::ClarificationRequired;
            result.candidates = candidates
                .iter()
                .filter_map(|candidate| candidate.get("name")?.as_str().map(str::to_owned))
                .collect();
            result.message = format!(
                "Found {} matching files. Please specify more of the file name.",
                candidates.len()
            );
            result.routing = Some(routing);
            return;
        }
    }
    result.message = routing
        .error
        .as_ref()
        .map(|error| error.message.clone())
        .or_else(|| {
            routing
                .data
                .as_ref()?
                .get("message")?
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "The application request completed.".into());
    result.status = if routing.status == ToolResultStatus::Completed {
        NaturalCommandStatus::Ready
    } else {
        NaturalCommandStatus::Rejected
    };
    if routing.status == ToolResultStatus::Completed {
        if let (Some(tool), Some(application)) = (
            result
                .request
                .as_ref()
                .and_then(|request| request.get("tool"))
                .and_then(Value::as_str),
            result
                .request
                .as_ref()
                .and_then(|request| request.get("arguments"))
                .and_then(|arguments| arguments.get("application"))
                .and_then(Value::as_str),
        ) {
            with_fresh_context(|context| {
                context.last_application = Some(application.to_string());
                context.last_application_tool = Some(tool.to_string());
                context.updated = Some(Instant::now());
            });
        }
    }
    result.routing = Some(routing);
}
pub fn process_voice_command(app: AppHandle, input: String) {
    let speech_generation = app.state::<crate::tts::TtsService>().generation();
    if let Err(error) = std::thread::Builder::new()
        .name("nova-natural-command".into())
        .spawn(move || {
            assistant::update_voice_state(
                &app,
                "thinking",
                Some(format!("Heard: “{input}”")),
                None,
            );
            let command_started = Instant::now();
            let history_suppression = command_history::SuppressHistory::new();
            let mut result = interpret_at(&app, &input, OLLAMA_ADDRESS);
            if result
                .routing
                .as_ref()
                .is_some_and(|routing| routing.status == ToolResultStatus::ConfirmationRequired)
                && result
                    .request
                    .as_ref()
                    .is_some_and(|request| request.get("tool").is_some())
            {
                assistant::update_voice_state(
                    &app,
                    "executing",
                    Some(result.message.clone()),
                    None,
                );
            }
            if !app.state::<crate::tts::TtsService>().current(speech_generation) {
                return;
            }
            execute_safe_voice_confirmation(&app, &mut result);
            drop(history_suppression);
            command_history::record_natural(&app,&result,command_started.elapsed().as_millis() as u64,"spoken");
            if !app.state::<crate::tts::TtsService>().current(speech_generation) {
                return;
            }
            // Publish text first; speech failure leaves it intact. KWS stays active while speaking.
            let response_state = if matches!(
                result.status,
                NaturalCommandStatus::Ready | NaturalCommandStatus::ClarificationRequired
            ) { "processing" } else { "error" };
            if !matches!(
                result.status,
                NaturalCommandStatus::AssistantHidden
                    | NaturalCommandStatus::NovaDisabled
                    | NaturalCommandStatus::NovaEnabled
            ) {
                assistant::update_voice_state(&app, response_state, Some(result.message.clone()), None);
                crate::tts::reply(&app, speech_generation, &result);
            }
            if !app.state::<crate::tts::TtsService>().current(speech_generation) {
                return;
            }
            let transcript = Some(format!("Heard: “{input}” — {}", result.message));
            match result.status {
                NaturalCommandStatus::Ready => {
                    let state = match result.routing.as_ref().map(|routing| routing.status) {
                        Some(ToolResultStatus::Completed) => "success",
                        Some(ToolResultStatus::Denied | ToolResultStatus::Rejected) => "error",
                        _ => "processing",
                    };
                    if state == "success"
                        && audio::begin_followup_capture(
                            &app.state::<audio::AudioService>(),
                            Some(speech_generation),
                        )
                            .is_ok()
                    {
                        let followup = transcript
                            .as_deref()
                            .map(|message| format!("{message} Listening for another command..."));
                        crate::assistant::update_voice_state(&app, "listening", followup, None);
                    } else if app.state::<crate::tts::TtsService>().current(speech_generation) {
                        crate::assistant::update_voice_state(&app, state, transcript, None);
                    }
                }
                NaturalCommandStatus::ClarificationRequired => {
                    let is_sequence = result
                        .request
                        .as_ref()
                        .and_then(|request| request.get("type"))
                        .and_then(Value::as_str)
                        == Some("tool_sequence");
                    if !is_sequence
                        && audio::begin_followup_capture(
                            &app.state::<audio::AudioService>(),
                            Some(speech_generation),
                        )
                            .is_ok()
                    {
                        let followup = transcript
                            .as_deref()
                            .map(|message| format!("{message} Listening for your answer..."));
                        crate::assistant::update_voice_state(&app, "listening", followup, None);
                    } else if app.state::<crate::tts::TtsService>().current(speech_generation) {
                        crate::assistant::update_voice_state(&app, "processing", transcript, None);
                    }
                }
                NaturalCommandStatus::AssistantHidden
                | NaturalCommandStatus::NovaEnabled
                | NaturalCommandStatus::NovaDisabled => {}
                _ => crate::assistant::update_voice_state(
                    &app,
                    "error",
                    Some(input),
                    Some(result.message),
                ),
            }
        })
    {
        eprintln!("Could not start natural-command interpretation: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trickling_model_response_cannot_extend_the_deadline() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = socket.read(&mut request);
            for _ in 0..100 {
                if socket.write_all(b"x").is_err() { break; }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let started = Instant::now();
        assert!(http_request_at(&address, "GET", "/", None, Duration::from_millis(100)).is_err());
        assert!(started.elapsed() < Duration::from_secs(1), "individual reads must not reset the total deadline");
        server.join().unwrap();
    }

    #[test]
    fn assistant_controls_accept_polite_and_bounded_speech_variants() {
        for input in ["Close Nova", "Close the nova, please", "Can you close the nova", "Please close the nova", "cloze nova", "clothes no va", "hide assistant"] {
            assert!(matches!(deterministic_request(input), DeterministicDecision::HideAssistant), "{input}");
        }
        for input in ["Turn off", "Turn off NOVA", "Please turn off the nova", "can you turn off", "turn of nova"] {
            assert!(matches!(deterministic_request(input), DeterministicDecision::DisableNova), "{input}");
        }
        for input in ["nova", "don't close nova", "do not turn off nova", "close nova project", "turn off chrome", "close notepad", "never close nova"] {
            assert!(!matches!(deterministic_request(input), DeterministicDecision::HideAssistant | DeterministicDecision::DisableNova), "{input}");
        }
    }

    #[test]
    fn paraphrased_commands_map_to_exact_typed_requests() {
        let cases = [
            (
                "Can you launch my code editor?",
                "application.open",
                json!({ "application": "Visual Studio Code" }),
            ),
            (
                "Open my Downloads folder.",
                "folder.open",
                json!({ "folder": "Downloads" }),
            ),
            (
                "Set my volume to thirty percent.",
                "system.set_volume",
                json!({ "volume": 30 }),
            ),
            ("Take a screenshot.", "system.take_screenshot", json!({})),
            (
                "What apps are currently running?",
                "application.list_running",
                json!({}),
            ),
        ];
        for (input, tool, arguments) in cases {
            let DeterministicDecision::Request(request) = deterministic_request(input) else {
                panic!("expected deterministic request for {input}");
            };
            assert_eq!(request["tool"], tool);
            assert_eq!(request["arguments"], arguments);
            assert!(tool_router::validate_model_request(request).is_ok());
        }
    }

    #[test]
    fn noisy_notepad_transcripts_resolve_without_model_guessing() {
        for input in ["Open Notebagged", "Open note. pad note. pad"] {
            let DeterministicDecision::Request(request) = deterministic_request(input) else {
                panic!("expected deterministic request for {input}");
            };
            assert_eq!(request["tool"], "application.open");
            assert_eq!(request["arguments"], json!({ "application": "Notepad" }));
            assert!(tool_router::validate_model_request(request).is_ok());
        }
    }

    #[test]
    fn voice_auto_confirmation_is_limited_to_explicit_non_destructive_actions() {
        assert!(voice_request_is_auto_confirmable(&tool_request(
            "application.open",
            json!({ "application": "Notepad" })
        )));
        assert!(voice_request_is_auto_confirmable(&tool_request(
            "application.focus",
            json!({ "application": "Notepad" })
        )));
        assert!(voice_request_is_auto_confirmable(&tool_request(
            "application.close",
            json!({ "application": "Notepad" })
        )));
        assert!(voice_request_is_auto_confirmable(&tool_request(
            "system.set_volume",
            json!({ "volume": 30 })
        )));
        assert!(voice_request_is_auto_confirmable(&tool_request(
            "folder.open",
            json!({ "folder": "Downloads" })
        )));
        assert!(!voice_request_is_auto_confirmable(&tool_request(
            "file.delete",
            json!({ "path": "C:\\\\important.txt" })
        )));
    }

    #[test]
    fn follow_up_application_close_is_resolved_deterministically() {
        let DeterministicDecision::Request(request) =
            deterministic_request("Close the Notepad now")
        else {
            panic!("expected a deterministic close request");
        };
        assert_eq!(request["tool"], "application.close");
        assert_eq!(request["arguments"], json!({ "application": "Notepad" }));
        assert!(tool_router::validate_model_request(request).is_ok());
    }

    #[test]
    fn nova_control_phrases_are_distinct_from_application_tools() {
        assert!(matches!(
            deterministic_request("Close NOVA"),
            DeterministicDecision::HideAssistant
        ));
        assert!(matches!(
            deterministic_request("Turn off NOVA"),
            DeterministicDecision::DisableNova
        ));
    }

    #[test]
    fn polite_and_imperfect_supported_phrases_stay_deterministic() {
        let cases = [
            (
                "Open my file explorer please",
                "application.open",
                json!({ "application": "File Explorer" }),
            ),
            (
                "Set my volumn to 46%",
                "system.set_volume",
                json!({ "volume": 46 }),
            ),
            (
                "Launch my VS Code",
                "application.open",
                json!({ "application": "Visual Studio Code" }),
            ),
            (
                "Open the File Explorer and go to downloads folder",
                "folder.open",
                json!({ "folder": "Downloads" }),
            ),
        ];
        for (input, tool, arguments) in cases {
            let DeterministicDecision::Request(request) = deterministic_request(input) else {
                panic!("expected deterministic request for {input}");
            };
            assert_eq!(request["tool"], tool);
            assert_eq!(request["arguments"], arguments);
        }
        assert!(matches!(
            deterministic_request("Open Chrome and search for Facebook"),
            DeterministicDecision::Unsupported(_)
        ));
    }

    #[test]
    fn reported_voice_phrases_map_without_model_guessing() {
        let cases = [
            (
                "Can you open Microsoft Edge",
                "application.open",
                json!({ "application": "Microsoft Edge" }),
            ),
            (
                "Go to my Videos folder",
                "folder.open",
                json!({ "folder": "Videos" }),
            ),
            (
                "Close Document folder",
                "application.close",
                json!({ "application": "File Explorer" }),
            ),
            (
                "Find my Chapter 1 file",
                "file.find_by_name",
                json!({ "query": "chapter 1" }),
            ),
            (
                "Open Chapter 1",
                "file.find_by_name",
                json!({ "query": "chapter 1" }),
            ),
            (
                "Set my volume to 30 percent",
                "system.set_volume",
                json!({ "volume": 30 }),
            ),
            (
                "Decrease my volume by 15 percent",
                "system.adjust_volume",
                json!({ "delta": -15 }),
            ),
            ("Mute", "system.mute", json!({})),
        ];
        for (input, tool, arguments) in cases {
            let DeterministicDecision::Request(request) = deterministic_request(input) else {
                panic!("expected deterministic request for {input}");
            };
            assert_eq!(request["tool"], tool, "input={input}");
            assert_eq!(request["arguments"], arguments, "input={input}");
            assert!(tool_router::validate_model_request(request).is_ok());
        }
    }

    #[test]
    fn wake_prefixes_and_basic_taglish_are_normalized_linguistically() {
        assert_eq!(
            normalize_transcript("Hey NOVA, open mo yung VS Code"),
            "open vs code"
        );
        assert_eq!(
            normalize_transcript("NOVA pakiclose yung File Explorer"),
            "close file explorer"
        );
        assert_eq!(
            normalize_transcript("Hanapin mo yung Chapter One ko"),
            "find chapter one"
        );
        assert_eq!(
            normalize_transcript("Nasaan yung thesis file ko?"),
            "where thesis file"
        );
    }

    #[test]
    fn semantic_show_and_taglish_variations_use_typed_tools() {
        let cases = [
            (
                "Show me where my thesis is",
                "file.find_by_name",
                json!({ "query": "thesis" }),
            ),
            (
                "Locate my resume",
                "file.find_by_name",
                json!({ "query": "resume" }),
            ),
            (
                "Open mo yung VS Code",
                "application.open",
                json!({ "application": "Visual Studio Code" }),
            ),
            (
                "Pakiclose yung File Explorer",
                "application.close",
                json!({ "application": "File Explorer" }),
            ),
            (
                "Hanapin mo yung Chapter One ko",
                "file.find_by_name",
                json!({ "query": "chapter one" }),
            ),
        ];
        for (input, expected_tool, expected_arguments) in cases {
            let DeterministicDecision::Request(request) = deterministic_request(input) else {
                panic!("expected typed request for {input}");
            };
            assert_eq!(request["tool"], expected_tool, "input={input}");
            assert_eq!(request["arguments"], expected_arguments, "input={input}");
            assert!(tool_router::validate_model_request(request).is_ok());
        }
    }

    #[test]
    fn bounded_context_resolves_ordinals_and_corrections() {
        with_fresh_context(|context| {
            *context = ConversationContext {
                updated: Some(Instant::now()),
                candidates: vec![
                    json!({ "name": "Chapter 1 old.docx", "resultId": "old", "modifiedAt": 1 }),
                    json!({ "name": "Chapter 1 new.docx", "resultId": "new", "modifiedAt": 2 }),
                ],
                selected: None,
                candidate_action: Some("file.reveal_in_explorer".into()),
                last_application: Some("Google Chrome".into()),
                last_application_tool: Some("application.open".into()),
            };
        });
        let Some(DeterministicDecision::Request(second)) =
            contextual_request("Show the second one")
        else {
            panic!("expected an ordinal follow-up");
        };
        assert_eq!(second["tool"], "file.reveal_in_explorer");
        assert_eq!(second["arguments"]["resultId"], "new");

        for (input, expected_tool, expected_id) in [
            ("Open it", "file.open", "new"),
            ("Show its folder", "file.reveal_in_explorer", "new"),
            ("Open the first one", "file.open", "old"),
            ("No, the other one", "file.reveal_in_explorer", "new"),
        ] {
            let Some(DeterministicDecision::Request(request)) = contextual_request(input) else {
                panic!("expected contextual request for {input}");
            };
            assert_eq!(request["tool"], expected_tool, "{input}");
            assert_eq!(request["arguments"]["resultId"], expected_id, "{input}");
        }
        with_fresh_context(|context| {
            context.candidates.push(json!({ "resultId": "third" }));
        });
        assert!(contextual_request("No, the other one").is_none());

        let Some(DeterministicDecision::Request(correction)) =
            contextual_request("No, I meant File Explorer")
        else {
            panic!("expected an application correction");
        };
        assert_eq!(correction["tool"], "application.open");
        assert_eq!(correction["arguments"]["application"], "File Explorer");
        let Some(DeterministicDecision::Request(replacement)) =
            contextual_request("Not Chrome, File Explorer")
        else {
            panic!("expected replacement, not the negated application");
        };
        assert_eq!(replacement["arguments"]["application"], "File Explorer");
        assert!(matches!(
            contextual_request("Cancel that"),
            Some(DeterministicDecision::Cancelled)
        ));
        assert!(contextual_request("Open it").is_none());
    }

    #[test]
    fn exact_file_stem_wins_over_partial_matches_without_random_selection() {
        let candidates = vec![
            json!({ "name": "Chapter 1-2 (1).docx", "resultId": "partial" }),
            json!({ "name": "Chapter 1.docx", "resultId": "exact" }),
        ];
        assert_eq!(
            single_file_candidate(&candidates, "chapter 1")
                .and_then(|candidate| candidate.get("resultId"))
                .and_then(Value::as_str),
            Some("exact")
        );
        assert!(single_file_candidate(&candidates, "chapter").is_none());
    }

    #[test]
    fn common_on_off_transcriptions_are_distinct_native_controls() {
        for input in ["NOVA on", "Turn up NOVA"] {
            assert!(matches!(
                deterministic_request(input),
                DeterministicDecision::EnableNova
            ));
        }
        for input in ["NOVA off", "NOVA of", "Turn of NOVA"] {
            assert!(matches!(
                deterministic_request(input),
                DeterministicDecision::DisableNova
            ));
        }
    }

    #[test]
    #[ignore = "requires the local qwen3:1.7b Ollama model"]
    fn installed_ollama_returns_direct_valid_arguments_for_known_apps() {
        for (input, expected) in [
            ("Open Notepad", "Notepad"),
            ("Launch Google Chrome", "Google Chrome"),
        ] {
            let content = ollama_output_at(OLLAMA_ADDRESS, input, &[])
                .expect("the installed local Ollama model should respond");
            let output = parse_model_output(&content)
                .unwrap_or_else(|error| panic!("{input}: {error}; output={content}"));
            assert_eq!(
                output.tool, "application.open",
                "input={input}; output={content}"
            );
            assert_eq!(
                output.arguments,
                json!({ "application": expected }),
                "input={input}; output={content}"
            );
            assert!(tool_router::validate_model_request(tool_request(
                &output.tool,
                output.arguments
            ))
            .is_ok());
        }
    }
    #[test]
    fn ambiguous_project_requests_never_guess() {
        assert!(matches!(
            deterministic_request("Open my project."),
            DeterministicDecision::ProjectAmbiguity
        ));
    }

    #[test]
    fn unsafe_and_unsupported_requests_never_reach_a_model_tool() {
        assert!(matches!(
            deterministic_request("Run PowerShell and delete my files"),
            DeterministicDecision::Unsupported(_)
        ));
    }

    #[test]
    fn malformed_or_unsupported_model_output_is_rejected() {
        assert!(parse_model_output("not json").is_err());
        assert!(parse_model_output(
            r#"{"decision":"tool","tool":"shell.execute","arguments":{},"confidence":0.99,"message":"run"}"#
        ).is_err());
        let structurally_valid = parse_model_output(
            r#"{"decision":"tool","tool":"system.set_volume","arguments":{"volume":999},"confidence":0.99,"message":"set"}"#
        ).unwrap();
        let request = tool_request(&structurally_valid.tool, structurally_valid.arguments);
        assert!(tool_router::validate_model_request(request).is_err());
        assert!(parse_model_output(
            r#"{"decision":"tool","tool":"folder.open","arguments":{"folder":"/no_think"},"confidence":0.99,"message":"open"}"#
        )
        .is_err());
    }

    #[test]
    fn compound_model_output_is_bounded_and_every_step_is_typed() {
        let output = parse_model_output(
            r#"{"decision":"sequence","tool":"none","arguments":{},"confidence":0.91,"message":"I prepared two actions.","steps":[{"tool":"application.open","arguments":{"application":"Visual Studio Code"}},{"tool":"folder.open","arguments":{"folder":"Downloads"}}]}"#,
        )
        .expect("valid bounded sequence");
        assert_eq!(output.decision, ModelDecision::Sequence);
        assert_eq!(output.steps.len(), 2);

        assert!(parse_model_output(
            r#"{"decision":"sequence","tool":"none","arguments":{},"confidence":0.9,"message":"bad","steps":[{"tool":"application.open","arguments":{"application":"Chrome"}},{"tool":"shell.execute","arguments":{}}]}"#
        )
        .is_err());
        assert!(parse_model_output(
            r#"{"decision":"sequence","tool":"none","arguments":{},"confidence":0.9,"message":"too short","steps":[{"tool":"application.open","arguments":{"application":"Chrome"}}]}"#
        )
        .is_err());
    }

    #[test]
    fn low_confidence_output_requires_clarification() {
        let output = parse_model_output(
            r#"{"decision":"tool","tool":"application.open","arguments":{"application":"Chrome"},"confidence":0.2,"message":"maybe"}"#
        ).unwrap();
        assert!(output.confidence < MIN_MODEL_CONFIDENCE);
    }

    #[test]
    fn http_client_rejects_non_loopback_endpoints() {
        let error =
            http_request_at("1.1.1.1:80", "GET", "/", None, Duration::from_millis(1)).unwrap_err();
        assert!(error.contains("loopback-only"));
    }

    #[test]
    fn readiness_distinguishes_missing_installed_and_loaded_models() {
        for (responses, expected) in [
            (vec![r#"{"models":[]}"#], LocalModelStatus::ModelMissing),
            (vec![r#"{"models":[{"name":"qwen3:1.7b"}]}"#, r#"{"models":[]}"#], LocalModelStatus::NotLoaded),
            (vec![r#"{"models":[{"name":"qwen3:1.7b"}]}"#, r#"{"models":[{"name":"qwen3:1.7b"}]}"#], LocalModelStatus::Loaded),
            (vec!["invalid"], LocalModelStatus::Error),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let server = std::thread::spawn(move || {
                for body in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                    let mut request = [0; 4096];
                    let _ = stream.read(&mut request).unwrap();
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
                }
            });
            assert_eq!(model_info_at(&address).status, expected);
            server.join().unwrap();
        }
    }

    #[test]
    fn offline_runtime_is_reported_without_retrying() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let info = model_info_at(&address.to_string());
        assert_eq!(info.status, LocalModelStatus::ServiceUnavailable);
    }

    #[test]
    fn chunked_http_payload_is_decoded_deterministically() {
        assert_eq!(
            decode_chunked(b"4\r\ntest\r\n3\r\n123\r\n0\r\n\r\n").unwrap(),
            b"test123"
        );
    }
}

pub fn clear_context() { if let Ok(mut context) = conversation_context().lock() { *context = ConversationContext::default(); } }
