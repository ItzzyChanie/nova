# NOVA 1.0 local Windows development release - Phase 16 engineering report

## 1. Final architecture

NOVA has two React webviews hosted by Tauri: the dashboard owns configuration and explicit user actions; the floating assistant displays transcript, state and errors. Native Rust modules own settings, permissions, file boundaries, processes, audio and model access. Model output is data passed through the same typed validator as UI-origin requests.

Voice flows through CPAL input conditioning, sherpa-onnx keyword spotting, bounded VAD capture and local Whisper transcription. The deterministic interpreter handles known commands and registered names before optionally asking loopback Ollama for a constrained intent. The tool router validates the request, applies OFF/skill policy and checks confirmation. Results remain visible as text; optional local SAPI speech follows. Wake interruption invalidates old replies before recording resumes.

Projects are stored configurations, not model-produced programs. The model can identify a profile by name, while native code resolves the stored editor, directories and approved command. Workflow execution reads a saved, approved sequence, preflights every step, rechecks native permissions during execution and stops after the first failure.

## 2. Major features

- Project profiles include ID, name, path, editor, frontend/backend commands, working directory, development URL, notes and voice-execution approval. Legacy profiles migrate with empty commands and voice approval OFF.
- Project actions open the configured editor, start approved processes, wait for the configured loopback port, open the URL, and stop owned process trees. Duplicate starts are prevented. Missing folders, editors, runners, scripts, occupied ports and readiness timeouts return useful errors.
- Workflows support create, rename, edit, step reordering, enable/disable, execution and confirmed deletion. The UI builds steps from supported tools rather than accepting executable script text.
- Voice phrases resolve saved names deterministically, including Start ProctorX, Stop ProctorX, Start Coding Mode and a single saved project referenced as my project.
- Voice execution requires a separate saved approval flag. Without it, the native execution boundary returns confirmation-required. Neither a model request nor a workflow argument can set that flag.
- History stores timestamp, original natural input, spoken/typed source, selected action, result, status and duration. Old entries remain readable. Storage is bounded to 2,000 entries. Saved workflows produce one aggregate entry rather than flooding history with step entries.
- Privacy toggles now persist and act on real functionality. Audio storage remains unavailable and OFF; command logging gates writes; external website opening obeys the network preference. Clear local data requires a typed confirmation.
- About uses package/runtime metadata, actual speech/wake information and Ollama installed/loaded inspection. Version is 1.0.0; loaded-model state uses `/api/ps`, not merely the installed model list.
- Added single-instance protection, fixed tray wake to start listening, and released idle Whisper cache rather than retaining it indefinitely.

## 3. Technologies used

React 19, TypeScript 6, Vite 8, Rust, Tauri 2, CPAL, sherpa-onnx/ONNX, Windows SAPI/COM, Windows registry and shell APIs, Windows Job Objects, local JSON stores, global shortcuts and Windows autostart. Optional language classification uses Ollama's loopback HTTP API. No shell, unrestricted filesystem or cloud HTTP plugin is exposed to webviews.

Development environment observed: Windows x64, Node 22.15.0, MSVC toolchain. Exact crate versions are locked in `src-tauri/Cargo.lock`; frontend dependencies are locked in `package-lock.json`.

## 4. Files and modules

| Area | Primary files |
| --- | --- |
| Project/workflow storage, validation and execution | `src-tauri/src/developer.rs` |
| Runner resolution and owned process trees | `src-tauri/src/project_process.rs` |
| Native privacy controls and runtime metadata | `src-tauri/src/privacy.rs` |
| Duplicate instance prevention | `src-tauri/src/single_instance.rs` |
| Typed requests, policies and execution origins | `src-tauri/src/tool_router.rs` |
| Natural-language/voice integration | `src-tauri/src/language_model.rs` |
| Project root compatibility and editor launching | `src-tauri/src/file_control.rs`, `application_control.rs` |
| History and settings | `src-tauri/src/command_history.rs`, `settings.rs` |
| Voice lifecycle/resources | `audio.rs`, `speech.rs`, `tts.rs`, `assistant.rs` |
| Startup/tray registration | `lib.rs`, `tray.rs`, `wake_shortcut.rs` |
| Project/workflow UI | `src/pages/ProjectsPage.tsx`, `WorkflowsPage.tsx`, `src/services/projects.ts` |
| Privacy/About/history UI | `PrivacyDataPage.tsx`, `AboutPage.tsx`, `CommandHistoryPage.tsx`, `src/services/privacy.ts` |
| Navigation, styles and contracts | `src/App.tsx`, `src/types/*`, `src/styles/app.css`, `src/components/ui/Icon.tsx` |
| Capability registration and packaging | `src-tauri/build.rs`, `src-tauri/capabilities/*`, `src-tauri/tauri.conf.json`, package manifests |
| Browser regression | `scripts/phase16-ui-smoke.mjs` |
| Documentation | `README.md`, this report, existing engine/manual-test documents |

## 5. Permissions and security review

The dashboard capability lists explicit commands; broad `core:default` was removed and necessary event/window permissions are listed individually. The assistant retains listen/unlisten and its own hide/resize commands. Project/workflow mutation, privacy changes and registered execution additionally check the dashboard window in native code. Production CSP restricts script, connection, frame and object sources.

New actions use native confirmation policies and skill checks. Developer projects use the existing persisted scripts skill flag, now clearly labeled Developer projects; `developer.runScript` itself remains denied. Website actions use the web skill and an independent external-network preference. Workflow steps cannot recursively invoke workflows, run arbitrary scripts, delete files or shut down Windows. Model output rejects extra fields, including command strings, paths and voice-approval overrides on a named-project request.

During approval review, an initial unconditional voice-confirmation approach was rejected. The accepted implementation enforces the saved per-profile/per-workflow voice flag **inside the native execution boundary**, distinguished from dashboard-confirmed and approved-workflow origins. This is not a frontend-only check.

Commands accepted by project profiles are `npm run <script>` and `dotnet run`. NOVA resolves an absolute executable and launches it without a shell command line. npm's CLI and package scripts can themselves execute arbitrary repository code; user approval trusts that repository, and the UI/README say so. This is not a process sandbox. Commands or package contents changed outside NOVA are a local trust concern.

Windows processes start suspended, are assigned to a job with kill-on-close, and only then resume. Handles are owned and closed on stop/exit; unowned browser/editor processes are not terminated. Working directories must canonicalize inside the approved project directory. File tools continue using their established canonical containment checks.

HTTP(S) URLs reject embedded credentials and executable/file schemes. Development URLs are loopback-only. The network toggle governs NOVA-initiated external website opening, not a system-wide firewall or project process network access.

## 6. Models

- Wake engine: bundled sherpa-onnx Zipformer keyword spotting for NOVA/Hey NOVA.
- STT: bundled Whisper Tiny English INT8, with compatible user-selected Whisper exports supported by the existing settings page.
- VAD: bundled local VAD model used during command capture.
- Language model: optional local `qwen3:1.7b`, served by Ollama at `127.0.0.1:11434`. It is not bundled in the installer.
- TTS: Windows SAPI with explicitly selected installed David/Zira desktop voices; no additional neural TTS weights or cloud endpoint.

Initial environment/model provisioning can require downloads. Installed core inference stays local. A missing optional language model does not disable deterministic tools or the dashboard.

## 7. Resource behavior

Only one NOVA instance is admitted per Windows session. Background native workers block on channels while idle. Wake spotting runs only while NOVA is enabled and not paused; OFF drops its active stream/detector. The global shortcut remains registered but cannot wake an OFF assistant by design.

Whisper cache is released after 30 seconds without a transcription job. Ollama requests use a 30-second keep-alive; NOVA neither creates a second Ollama server nor kills an external server. SAPI objects are scoped to each reply and released immediately afterward. Voice status/project polling is bounded and skips hidden documents.

Started projects remain intentionally active until stopped, NOVA is turned off or NOVA exits. Stop affects only NOVA-owned jobs. A failed workflow preserves earlier successful actions and reports the failing step. There is no automatic rollback of arbitrary desktop actions.

No numeric idle CPU/RAM claim is made without an instrumented measurement. Hardware echo behavior remains a manual acceptance concern.

## 8. Validation results

### Automated regression

- Final native suite: **82 passed, 0 failed, 10 ignored**. Seven environment-dependent tests were then explicitly run, as recorded below.
- Frontend production build: passed.
- Headless Edge: **11 navigation pages passed**, project approval form passed, workflow add/reorder/remove passed, no uncaught UI exceptions. This uses a real browser preview and does **not** mock or claim native IPC coverage.
- Initial focused Windows tests: wake model on locally synthesized audio, live microphone stream, owned Node process termination, same-value volume write, and SAPI playback/cancellation passed.
- The initial Whisper fixture assertion failed on `night fall` versus `nightfall`; the recognized sentence otherwise matched. The assertion was adjusted to tolerate word spacing, with final rerun recorded below. This was a test-text normalization change, not a speech-engine workaround.
- `git diff --check`: passed at review.

Final validation on September 11, 2026:

- `cargo check --manifest-path src-tauri/Cargo.toml --offline`: passed.
- `npm run build`: passed, including the frontend build invoked by Tauri.
- Focused Windows regression: **7 passed, 0 failed**. This includes live microphone capture, synthesized Hey NOVA detection, Whisper transcription, SAPI replies/cancellation, same-value Core Audio write, owned Node termination, and an isolated configured npm server start/readiness/descendant-stop test.
- The corrected Whisper fixture passed on rerun. Three other environment-dependent tests remain ignored; this is not a claim that every ignored test was exercised.

Reproduction commands (hardware tests require the two local 16 kHz mono fixture WAVs):

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
node scripts/phase16-ui-smoke.mjs
$env:NOVA_SPEECH_TEST_WAV = (Resolve-Path .tmp/phase16-stt.wav).Path
$env:NOVA_WAKE_TEST_WAV = (Resolve-Path .tmp/phase16-wake.wav).Path
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline -- --ignored --test-threads=1 project_process::tests::owned_process_tree_stops project_process::tests::configured_npm_script_starts_and_stops_server audio::tests::live_microphone_capture_smoke_test tts::tests::installed_windows_voice_speaks_and_cancels system_control::tests::live_core_audio_endpoint_accepts_a_same_value_write audio::tests::bundled_model_detects_hey_nova_audio speech::tests::local_whisper_transcribes_known_english_audio
npm run tauri build
```

- Production executable background startup: passed the bounded process-liveness smoke test.
- Duplicate launch: exited with code 0; exactly one NOVA process remained alive. The test then terminated only its owned process. Dashboard foreground visibility was not confirmed (the process window title remained empty); tray quit was not exercised by this check.

- `npm run tauri build`: **passed**, exit code 0; optimized Rust release compilation completed and NSIS reported one finished bundle.
- Installer: [`nova_1.0.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/nova_1.0.0_x64-setup.exe), **60,896,481 bytes**, detected product version **1.0.0**.
- SHA-256: `41CA169C7C073FEA7B59BB64B72F65E33A2D96579E9E29644ADC4CBC81D5A5F3`.
- This is the newly generated 1.0.0 bundle; the older 0.1.0 artifact was excluded. Installer installation/uninstallation and signing were not performed. The build validates bundle generation, not the full installer acceptance cycle.

### Full regression coverage and practical limits

| Requested area | Coverage / remaining acceptance |
| --- | --- |
| Manual/background startup | Native startup and instance guard reviewed; final runtime smoke status recorded above if exercised. Windows sign-in/autostart remains manual. |
| All navigation pages | All 11 pages tested in Edge; visual Projects capture inspected. Native IPC is separate from browser coverage. |
| Alt+N / Hey NOVA | Shortcut and OFF guard reviewed; KWS model exercised on a local synthesized sample. Live acoustic invocation and shortcut interaction remain manual. |
| Microphone / STT / TTS | Real microphone stream and SAPI smoke tests; bundled model fixture tests. Natural human speech/accent/room accuracy is not inferred from these. |
| App / files / system / windows | Native routing and containment regression tests; real same-value audio endpoint test. Interactive app/file/window actions remain manual unless explicitly recorded above. |
| Natural-language selection | Deterministic and schema-validation tests, offline-runtime recovery tests; arbitrary conversational accuracy is not asserted. |
| Project open/start/stop | Named routing, command validation, path boundary and owned process tests. User-project/editor end-to-end acceptance remains manual. |
| Workflows | Step schema/permission validation and frontend editing covered; actual desktop multi-step routines remain manual unless explicitly recorded above. |
| Settings persistence | Real native stores implemented, legacy profile serialization covered. End-to-end restart/clear-data acceptance on user settings remains manual. |
| Tray hide/open/wake/quit | Code review including wake-listening fix and owned-process quit cleanup; interactive tray checks remain manual. |
| OFF | Native policy tests deny tools while OFF; wake/shortcut/autostart/project cleanup paths reviewed. Physical shortcut/sign-in acceptance remains manual. |
| Privacy/security | No core cloud call path; strict model schema, project command rejection, URL/path boundaries, explicit approval and narrow capability checks covered. External program network access is not constrained. |

Local detailed logs and browser screenshots are under ignored `.tmp/phase16-*`. The source test commands and this report are the reproducible record; browser/hardware checks are not represented as broader end-to-end passes.

## 9. Known limitations

This is a Windows development release. The installer is unsigned; there is no update service. Current runner support is npm scripts and `dotnet run`, with a shared frontend/backend working directory. Terminal output is not embedded; project process failures report exit/readiness information rather than captured build logs. Loopback readiness tests port availability, not an application-specific health endpoint.

Workflow execution is serial, stops on failure and leaves earlier steps in place. Configurations are editable only when the relevant project/workflow is not active. Voice actions require explicit saved approval; model-generated command or approval fields never grant permission. Only installed supported editors and desktop SAPI voices can be used.

The network toggle is intentionally scoped to NOVA opening external websites. Project code, editors and browsers are not sandboxed. Acoustic echo cancellation, extensive accented/Taglish testing, overnight idle measurements, Windows sign-in/autostart and installer installation/uninstallation require hands-on acceptance before public distribution. Clear local data was not tested by deleting a user's real settings or project registrations.

## 10. Future improvements

Add an isolated native end-to-end test data directory, signed installers and release automation, broader hardware/accent tests and acoustic echo cancellation, richer runner adapters with explicit argument schemas, per-service working directories, bounded process logs, application-specific health checks, and opt-in compensating workflow actions. These are future work, not part of the Phase 16 implementation.

Phase 16 stops after the recorded validation and report; no cloud AI, unrestricted script runner or public deployment is introduced.
