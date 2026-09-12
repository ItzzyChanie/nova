//! Offline Windows desktop speech. COM objects live only for one reply.
use crate::{
    language_model::{NaturalCommandResult, NaturalCommandStatus},
    settings,
    tool_router::{ToolErrorCode, ToolName, ToolResultStatus},
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct TtsService {
    generation: AtomicU64,
    playback: Mutex<()>,
}

impl TtsService {
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
    pub fn current(&self, generation: u64) -> bool {
        self.generation() == generation
    }
    // Invalidate pending replies, then wait for the active renderer to purge.
    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        drop(self.playback.lock().unwrap_or_else(|e| e.into_inner()));
    }
    fn render(
        &self,
        generation: u64,
        enabled: impl FnOnce() -> bool,
        text: &str,
        engine: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<(), String>,
    ) -> Result<(), String> {
        let _guard = self
            .playback
            .lock()
            .map_err(|_| "Speech playback lock failed.")?;
        if !self.current(generation) || !enabled() || text.is_empty() {
            return Ok(());
        }
        engine(text, &|| !self.current(generation))
    }
}

pub fn stop(app: &AppHandle) {
    app.state::<TtsService>().stop();
}

/// Text is already displayed by the caller. Failure must never replace that result.
pub fn reply(app: &AppHandle, generation: u64, result: &NaturalCommandResult) {
    let text = concise_response(result);
    let service = app.state::<TtsService>();
    if let Err(error) = service.render(
        generation,
        || {
            settings::voice_reply(app).unwrap_or(false)
                && settings::nova_enabled(app).unwrap_or(false)
        },
        &text,
        platform_speak,
    ) {
        crate::local_log::event("speech", "voice_reply_failed");
        eprintln!("Local voice reply unavailable; keeping the text response: {error}");
    }
}

fn short_text(text: &str) -> Option<String> {
    let text = text.trim();
    // Never read paths, URLs, markup, JSON, stack traces, or lengthy diagnostics.
    if text.is_empty()
        || text.chars().count() > 160
        || text.split_whitespace().count() > 28
        || text
            .chars()
            .any(|c| c.is_control() || "<>`{}[]\\/".contains(c))
    {
        return None;
    }
    Some(text.to_string())
}

pub fn concise_response(result: &NaturalCommandResult) -> String {
    if matches!(
        result.status,
        NaturalCommandStatus::AssistantHidden
            | NaturalCommandStatus::NovaDisabled
            | NaturalCommandStatus::NovaEnabled
    ) {
        return String::new();
    }
    if let Some(routing) = &result.routing {
        if let Some(error) = &routing.error {
            return match error.code {
                ToolErrorCode::UnknownApplication | ToolErrorCode::ApplicationNotInstalled => {
                    "I couldn't find that application."
                }
                ToolErrorCode::ApplicationNotRunning => "That application isn't running.",
                ToolErrorCode::FileNotFound | ToolErrorCode::FolderNotFound => {
                    "I couldn't find a match."
                }
                _ => "I couldn't complete that command. Please check the details on screen.",
            }
            .into();
        }
        if routing.status == ToolResultStatus::Completed {
            if let Some(data) = &routing.data {
                let name = data
                    .get("application")
                    .and_then(|v| v.as_str())
                    .and_then(short_text)
                    .filter(|v| v.chars().count() <= 60);
                match routing.tool {
                    Some(ToolName::ApplicationOpen) => {
                        return name
                            .map(|n| {
                                if data.get("alreadyRunning").and_then(|v| v.as_bool())
                                    == Some(true)
                                {
                                    format!("{n} is already open.")
                                } else {
                                    format!("Opening {n}.")
                                }
                            })
                            .unwrap_or("The application is open.".into())
                    }
                    Some(ToolName::ApplicationIsRunning) => {
                        return name
                            .map(|n| {
                                format!(
                                    "{n} is {}.",
                                    if data.get("running").and_then(|v| v.as_bool()) == Some(true) {
                                        "open"
                                    } else {
                                        "not running"
                                    }
                                )
                            })
                            .unwrap_or("Check the application status on screen.".into())
                    }
                    Some(ToolName::FileFindByName | ToolName::FolderFind) => {
                        if let Some(count) = data.get("count").and_then(|v| v.as_u64()) {
                            return format!(
                                "I found {count} matching {}.",
                                if count == 1 { "item" } else { "items" }
                            );
                        }
                    }
                    _ => {}
                }
                if let Some(message) = data
                    .get("message")
                    .and_then(|v| v.as_str())
                    .and_then(short_text)
                {
                    return message;
                }
            }
            return "Done. The result is on screen.".into();
        }
        if routing.status == ToolResultStatus::ConfirmationRequired {
            return "Please confirm the action on screen.".into();
        }
    }
    match result.status {
        NaturalCommandStatus::ClarificationRequired => short_text(&result.message)
            .unwrap_or("I found several possibilities. Please check the choices on screen.".into()),
        NaturalCommandStatus::Ready => {
            short_text(&result.message).unwrap_or("The result is on screen.".into())
        }
        NaturalCommandStatus::Unsupported => "I can't perform that command yet.".into(),
        _ => "I couldn't complete that command. Please check the details on screen.".into(),
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_speak(_: &str, _: &dyn Fn() -> bool) -> Result<(), String> {
    Err("Local voice replies require Windows.".into())
}

#[cfg(target_os = "windows")]
fn platform_speak(text: &str, cancelled: &dyn Fn() -> bool) -> Result<(), String> {
    use windows::{
        core::{HSTRING, PCWSTR},
        Win32::{
            Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
            Media::Speech::{
                ISpObjectToken, ISpVoice, SpObjectToken, SpVoice, SPF_ASYNC, SPF_IS_NOT_XML,
                SPF_PURGEBEFORESPEAK,
            },
            System::{
                Com::{
                    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                    COINIT_MULTITHREADED,
                },
                Threading::WaitForSingleObject,
            },
        },
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
        let _apartment = Apartment;
        let operation = || -> windows::core::Result<()> {
            let voice: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_INPROC_SERVER)?;
            // Explicit built-in offline voices: never use an arbitrary default SAPI provider.
            let token: ISpObjectToken =
                CoCreateInstance(&SpObjectToken, None, CLSCTX_INPROC_SERVER)?;
            let mut selected = false;
            for name in ["TTS_MS_EN-US_DAVID_11.0", "TTS_MS_EN-US_ZIRA_11.0"] {
                let id = HSTRING::from(format!(
                    "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Speech\\Voices\\Tokens\\{name}"
                ));
                if token.SetId(PCWSTR::null(), &id, false).is_ok() && voice.SetVoice(&token).is_ok()
                {
                    selected = true;
                    break;
                }
            }
            if !selected {
                return Err(windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    "No supported offline Microsoft desktop voice is installed.",
                ));
            }
            if cancelled() {
                return Ok(());
            }
            voice.Speak(
                &HSTRING::from(text),
                (SPF_ASYNC.0 | SPF_IS_NOT_XML.0) as u32,
                None,
            )?;
            let started = std::time::Instant::now();
            let outcome = loop {
                if cancelled() {
                    break Ok(());
                }
                if started.elapsed() > std::time::Duration::from_secs(20) {
                    break Err(windows::core::Error::new(
                        windows::core::HRESULT(0x80004005u32 as i32),
                        "Speech timed out.",
                    ));
                }
                match WaitForSingleObject(voice.SpeakCompleteEvent(), 20) {
                    WAIT_OBJECT_0 => break Ok(()),
                    WAIT_TIMEOUT => {}
                    _ => break Err(windows::core::Error::from_win32()),
                }
            };
            // Purge before releasing the serialization lock, including on cancellation/failure.
            voice.Speak(PCWSTR::null(), SPF_PURGEBEFORESPEAK.0 as u32, None)?;
            outcome
        };
        operation().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_model::InterpretationSource;
    use crate::tool_router::{ToolError, ToolResult};
    use serde_json::json;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    fn result(tool: ToolName, data: serde_json::Value) -> NaturalCommandResult {
        NaturalCommandResult {
            status: NaturalCommandStatus::Ready,
            source: InterpretationSource::Deterministic,
            model: "test",
            input: String::new(),
            normalized_input: String::new(),
            confidence: None,
            request: None,
            message: "Technical details remain in the text result.".into(),
            candidates: vec![],
            latency_millis: 0,
            routing: Some(ToolResult {
                request_id: None,
                tool: Some(tool),
                category: None,
                risk: None,
                permission: None,
                status: ToolResultStatus::Completed,
                data: Some(data),
                error: None,
            }),
        }
    }
    #[test]
    fn app_open_and_already_open_are_concise_and_based_on_actual_results() {
        assert_eq!(
            concise_response(&result(
                ToolName::ApplicationOpen,
                json!({"application":"Visual Studio Code","alreadyRunning":false})
            )),
            "Opening Visual Studio Code."
        );
        assert_eq!(
            concise_response(&result(
                ToolName::ApplicationIsRunning,
                json!({"application":"ProctorX","running":true})
            )),
            "ProctorX is open."
        );
        assert_eq!(
            concise_response(&result(
                ToolName::ApplicationOpen,
                json!({"application":"ProctorX","alreadyRunning":true})
            )),
            "ProctorX is already open."
        );
        assert_eq!(
            concise_response(&result(ToolName::FileFindByName, json!({"count":3}))),
            "I found 3 matching items."
        );
    }
    #[test]
    fn errors_never_speak_technical_logs() {
        let mut result = result(ToolName::ApplicationOpen, json!({}));
        let routing = result.routing.as_mut().unwrap();
        routing.status = ToolResultStatus::Rejected;
        routing.error = Some(ToolError {
            code: ToolErrorCode::UnknownApplication,
            message: "technical log".repeat(100),
        });
        assert_eq!(
            concise_response(&result),
            "I couldn't find that application."
        );
        result
            .routing
            .as_mut()
            .unwrap()
            .error
            .as_mut()
            .unwrap()
            .code = ToolErrorCode::ApplicationLaunchFailed;
        assert_eq!(
            concise_response(&result),
            "I couldn't complete that command. Please check the details on screen."
        );
        assert!(short_text(&"log ".repeat(100)).is_none());
        assert!(short_text("C:/private/file.txt").is_none());
        assert!(short_text("<speak>hello</speak>").is_none());
        assert!(short_text("Error\nstack trace").is_none());
    }
    #[test]
    fn off_never_initializes_engine_on_speaks_once_and_failure_preserves_text() {
        let service = TtsService::default();
        service
            .render(
                0,
                || false,
                "Opening Notepad.",
                |_, _| panic!("OFF must not load speech"),
            )
            .unwrap();
        let result = result(ToolName::ApplicationOpen, json!({"application":"Notepad"}));
        let text = result.message.clone();
        let spoken = concise_response(&result);
        let mut count = 0;
        service
            .render(
                0,
                || true,
                &spoken,
                |text, _| {
                    assert_eq!(text, "Opening Notepad.");
                    count += 1;
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(count, 1);
        assert!(service
            .render(0, || true, &spoken, |_, _| Err("device failure".into()))
            .is_err());
        assert_eq!(result.message, text);
        service.render(0, || true, &spoken, |_, _| Ok(())).unwrap();
    }
    #[test]
    fn interruption_purges_before_return_and_discards_stale_replies() {
        let service = Arc::new(TtsService::default());
        let worker_service = service.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            worker_service.render(
                0,
                || true,
                "Long reply",
                |_, cancelled| {
                    started_tx.send(()).unwrap();
                    while !cancelled() {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    stopped_tx.send(()).unwrap();
                    Ok(())
                },
            )
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        service.stop();
        stopped_rx
            .try_recv()
            .expect("playback must stop before listening starts");
        worker.join().unwrap().unwrap();
        service
            .render(
                0,
                || true,
                "Stale reply",
                |_, _| panic!("stale reply played"),
            )
            .unwrap();
        assert!(!service.current(0));
        service
            .render(service.generation(), || true, "New reply", |_, _| Ok(()))
            .unwrap();
    }
    #[test]
    fn multiple_commands_never_overlap_playback() {
        let service = Arc::new(TtsService::default());
        let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let workers: Vec<_> = (0..3)
            .map(|_| {
                let service = service.clone();
                let active = active.clone();
                std::thread::spawn(move || {
                    service
                        .render(
                            0,
                            || true,
                            "Done.",
                            |_, _| {
                                assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
                                std::thread::sleep(Duration::from_millis(5));
                                assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
                                Ok(())
                            },
                        )
                        .unwrap()
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    }
    #[test]
    #[cfg(target_os = "windows")]
    #[ignore = "plays audio on the installed Windows desktop voice"]
    fn installed_windows_voice_speaks_and_cancels() {
        platform_speak("Opening Visual Studio Code.", &|| false).unwrap();
        platform_speak("I couldn't find that application.", &|| false).unwrap();
        let start = std::time::Instant::now();
        platform_speak("This reply must stop when the user wakes the assistant, before this sentence finishes.", &|| start.elapsed() >= Duration::from_millis(300)).unwrap();
        assert!(start.elapsed() < Duration::from_secs(3));
        platform_speak("Ready for another command.", &|| false).unwrap();
    }
}
