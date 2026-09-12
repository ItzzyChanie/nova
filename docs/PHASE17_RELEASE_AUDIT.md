# Phase 17: installed-build reliability and release audit

Date: 2026-09-12. Audited version: **1.0.1**, retained from the clean repository at audit start. Phase 16 described 1.0.0; the subsequent UI release already changed both HTML entry points and package versions. No unrelated feature work or version downgrade was performed.

**Recommendation: Beta, not Stable.** Installed human-audio, sign-in, installer/uninstaller and real-project acceptance remain release blockers until the worksheet is executed. Code inspection and build success are not installed-runtime acceptance.

## 1. Issues and fixes

| Finding | Change / disposition |
| --- | --- |
| Release wake, Whisper and VAD lookups could fall back to the build machine's source tree | Central resource resolver; source fallback compiled only into debug builds. Release always uses Tauri resource_dir. Missing assets produce explicit failures. |
| Persisted Whisper override could be relative despite UI validation | Native read path now rejects relative overrides. OFF/pause avoids resolving speech resources so bad overrides cannot prevent releasing capture. |
| Production stderr was not a usable local troubleshooting record | Added bounded local event logs and dashboard-only Open logs folder. |
| About showed generic model Error and green wake engine name independent of readiness | Replaced with on-demand diagnostic rows; checks also run on entering About. Distinguishes model missing, service unavailable, installed/on-demand and loaded. No downloads. |
| Duplicate launch searched any window titled NOVA and could miss cold startup | Session mutex still prevents workers; named auto-reset Windows event queues dashboard activation on the existing Tauri main thread. Background duplicates do not activate. |
| Tray failure could leave a hidden application without a usable dashboard | Show dashboard on failure; closing dashboard exits if no tray exists. |
| Tray had no pause action | Added Pause / resume using existing persisted pause behavior. |
| Shutdown cleanup concentrated in tray handler | Normal Tauri Exit stops SAPI and owned project jobs and records shutdown; process exit releases remaining audio/model threads and handles. |
| Store plugin ignores initial load failure | All native stores now use a serialized checked loader that validates existing JSON before initializing the plugin. Corrupt/unreadable files are preserved and rejected with recovery guidance; diagnostics checks on-disk JSON directly. |

## 2. Development versus installed runtime and paths

| Area | Audit result |
| --- | --- |
| Frontend | Development uses localhost Vite/HMR; installed app embeds dist/index.html and dist/assistant.html with separate CSS and production CSP. |
| Models | Installed resource_dir/resources/wake-word, resources/speech/sherpa-onnx-whisper-tiny.en and resources/vad/silero_vad.onnx. No release source-tree fallback. |
| Explicit speech override | Absolute user-selected directory remains supported. Missing/stale directories do not silently fall back to another model. Reset in Voice settings to use bundle. |
| Icons | Tauri packaged icons/default window icon; no runtime relative image lookup. |
| Settings/history/projects/workflows | Tauri store resolves filenames under app_data_dir, normally %APPDATA%/com.nova.assistant on Windows. Identifier preserved. |
| Screenshots | app_data_dir/screenshots; directory created on explicit screenshot action. Diagnostics never captures a screenshot. |
| Logs | app_local_data_dir/logs, normally %LOCALAPPDATA%/com.nova.assistant/logs. |
| Files and registered projects | Canonical paths checked against allowed roots. Commands use the approved canonical project working directory. |
| Applications | Known registry entries, Windows locations and PATH discovery. Missing applications return typed errors. Installation does not bundle editors or project runners. |
| Project runners | Absolute PATH entries resolve node.exe/dotnet.exe; npm CLI must exist beside Node. Sign-in PATH can differ from a development shell; guidance requires installing runner/restarting NOVA. |
| Processes | Owned project jobs start suspended, attach to kill-on-close Windows Job Objects, then resume with CREATE_NO_WINDOW. No repository current-directory dependency. |
| Autostart | Development can register the debug executable; installed startup resynchronizes the Run value to its own executable and --background. Verify after upgrades. |

Build-time absolute paths in generated NSIS File source entries are expected: destination entries put models under the installation resources directory. Test-only CARGO_MANIFEST_DIR fixtures are not release runtime fallback paths.

## 3. Readiness diagnostics and first run

About offers Run diagnostics and Open logs folder. Native checks execute off the UI thread. The assistant capability cannot invoke these commands; native code additionally requires window label main. Browser preview explicitly says Desktop only.

Checks cover desktop runtime, selected/default microphone enumeration, live wake snapshot, all required wake files, Whisper model files, VAD presence, unverified SAPI playback, loopback Ollama, qwen3:1.7b, on-disk JSON stores, actual autostart registration, shortcut registration, log writability and unsigned publisher status. Microphone detection is labelled capture-unverified; file presence is not engine inference proof. Actual permission/capture errors remain visible through Voice settings/wake status. No readiness check starts recording, runs project commands, pulls models or opens external sites.

Ollama install detection uses absolute PATH and common per-user/Program Files locations without executing an installer or binary. An unreachable unusual installation is labelled Not detected, not conclusively Not installed. Model service requests use bounded existing loopback HTTP code; unavailable service does not disable deterministic routing. qwen3:1.7b is optional and only explicitly installed by the user.

## 4. Local logging

Two files: nova.log and nova.previous.log, each capped at 1 MiB. Rotation is serialized. Records contain Unix timestamp and fixed component/state labels, including startup/shutdown/panic, tray setup, shortcut registration/conflict, autostart changes, wake transitions, microphone errors, transcription/reply failures, diagnostic LLM availability and tool/workflow outcomes. No raw audio, transcript, file content, command arguments, names or error payloads are written by this logger. Detailed actionable errors remain in the UI; existing command history remains separately controlled by the history privacy toggle.

Logging failure must not crash the assistant; diagnostics tests append access. Abrupt process termination cannot guarantee a shutdown event. This is an event log, not exhaustive tracing of third-party native-library stderr. Production panic payloads are omitted to avoid accidental data leakage.

## 5. Single instance, tray, shortcut and autostart

The Windows session-local mutex is acquired before Tauri/audio services are created. Activation event exists before mutex acquisition so a rapid second launch can queue activation before window setup. A waiting worker blocks on a Windows event; it does not poll. A manual duplicate requests unminimize/show/focus on the primary dashboard. Windows focus restrictions and repeated cold-launch behavior require installed desktop acceptance. One instance per session is intended; multiple signed-in Windows sessions are separate.

Tray Open, Wake, Hide, Pause/resume and Quit reuse native lifecycle actions. X hides to tray when available; tray initialization failure keeps an accessible dashboard. Shortcut registers Alt+N once and records conflict; diagnostics reports registration failure with tray fallback guidance. OFF still denies shortcut wake by existing native policy.

NOVA defaults ON on first launch, and setup synchronizes Windows autostart to the saved ON/OFF setting. ON registers --background; OFF removes the Run value with verification/rollback. Hidden main/assistant defaults avoid a startup dashboard flash. Release binary uses Windows subsystem, not console subsystem. No sign-in/reboot, interactive tray quit or focus behavior is claimed as passed here.

## 6. Audio, Whisper and wake packaging

sherpa-onnx provides native in-process keyword spotting, Whisper and VAD; no runtime Python process or external Whisper CLI is needed. Wake transducer files, tokens and keyword definitions, Tiny English encoder/decoder/tokens and Silero VAD are included by tauri.conf.json resource patterns and generated NSIS destination entries. SAPI uses Windows installed desktop voices. Ollama and qwen3:1.7b are external, not bundled.

Missing assets yield guidance to reinstall or fix an absolute override. Tiny English is not a promise of Taglish transcription quality; multilingual model selection is separate. Natural speech, Filipino-accented English, distance/noise/false-positive behavior and device removal must be measured on real hardware. Fixture tests cannot certify them. Whisper's existing cache expires after 30 seconds without a transcription job; repeated live allocation behavior remains to be measured.

## 7. Real projects, workflows and security

Registered commands are limited to stored npm run <script> / dotnet run forms. Native execution resolves saved project names, working directories, editor and approval flags. Model output cannot supply command strings, filesystem paths or voice approval on these named actions. Workflow steps are validated before execution and native policy is checked at execution; failures stop later steps and retain prior effects. Process readiness is loopback port availability, not an application health check. User project end-to-end acceptance remains required; this audit does not silently start or alter a real project.

Reviewed Tauri CSP/capability lists, typed tool schemas/deny_unknown_fields, OFF/skill checks, confirmation paths, registered workflow restrictions, canonical file containment and owned process launching. No unrestricted LLM-to-shell execution path was found. developer.runScript remains denied. No shell/filesystem/cloud plugin permissions were added. Only two dashboard diagnostic commands were added; Open logs folder accepts no caller path and launches Windows Explorer with the fixed local log directory. This preserves the existing security model, not a claim of a formal penetration test.

Approved npm scripts can execute repository code and external programs may use the network. User approval trusts that project; NOVA is not a process sandbox. The privacy network preference governs NOVA's external website opening, not Windows networking or project processes.

## 8. Installer and uninstaller

Generated NSIS uses currentUser installation, normally %LOCALAPPDATA%/nova, an uninstall entry, Start Menu shortcut and optional desktop shortcut. Tauri's running-app check precedes replacement/removal. Generated uninstall removes bundled executable/resources and shortcuts, removes HKCU Software/Microsoft/Windows/CurrentVersion/Run product value when not updating, and preserves data by default. Windows registry value names are case-insensitive, so configured autostart name NOVA matches product nova cleanup.

The explicit delete-app-data checkbox removes both Roaming and Local com.nova.assistant directories only when selected and not in update mode. Default reinstall preserves identifier/data. No custom destructive hooks were needed. Review is based on this build's generated installer.nsi, not an observed install/uninstall cycle. In-use file, zombie process, reboot and same-version replacement behavior remain manual.

WebView2 mode is downloadBootstrapper: a fresh machine without WebView2 may require network access. Core speech model resources are bundled and do not require first-run downloads. The installer is unsigned and About discloses signing is not configured. Windows may display SmartScreen/unknown-publisher prompts. No certificate bypass was added. For later Authenticode, use Tauri's Windows signing configuration and protected CI certificate credentials, verify both executable and installer signatures and publish the signed artifact hash.

Reference: [Tauri Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/) describes supported NSIS hooks and Windows packaging configuration. The generated local script is the evidence for this release's actual removal logic.

## 9. Resource measurements

No installed NOVA CPU/RAM, active wake, Whisper, loaded Ollama model or command-latency measurement was made. NOVA was not running during the read-only process inventory. An existing Ollama server was present; its single-process working set is not a model or total inference-memory measurement. Historical model memory/latency estimates are not measured acceptance evidence. The worksheet defines interval CPU measurement, child-process inclusion, cold/warm trials and overnight leak checks.

## 10. Automated validation

Baseline before code changes: native library **85 passed, 0 failed, 10 ignored**; npm run build passed. Current results and artifact identity are appended below after final validation.

The first two unrestricted full cargo test attempts exhausted host allocation during concurrent compilation, with compiler aborts; one also reported a missing sherpa_onnx_sys rlib. These were build failures, not passing test runs. Remaining validation is serialized with one Cargo build job. Existing Windows LNK4098 runtime-library warning is retained for follow-up; no certificate or linker-safety bypass was introduced.

Browser checks exercise real production frontend navigation/editing without native IPC. Hardware tests are deliberately not inferred from those results.

## 11. Manual acceptance and remaining issues

Execute [V1_ACCEPTANCE_TEST.md](V1_ACCEPTANCE_TEST.md) against the exact final installer hash. It covers installation/first run/dependencies/tray/shortcut/autostart/wake/speech/LLM/apps/files/system/projects/workflows/persistence/quit/reboot/uninstall and records raw transcript, resolved intent, tool result and latency only in the operator's explicit test worksheet.

| Severity | Open issue / required evidence |
| --- | --- |
| BLOCKER for Stable | Fresh installation, upgrade/reinstall, uninstall cleanup and no broken startup registration have not been exercised for this artifact. |
| BLOCKER for Stable | Real sign-in/background startup, rapid duplicate activation, tray quit, microphone release and cross-application Alt+N focus have not been accepted interactively. |
| BLOCKER for Stable | Natural wake/speech/accent/noise, file ambiguity/reveal and one real approved project/workflow require installed acceptance. |
| MAJOR | Overnight resource/leak and cold/warm command measurements are absent. |
| KNOWN LIMITATION | Corrupt JSON is preserved and rejected at initial load; there is no automatic backup/recovery UI. Restore a backup while NOVA is closed. External changes to already-loaded stores are not watched. |
| MINOR | Unsigned publisher / SmartScreen UX; document for test distribution and configure signing before broad public distribution. |
| MINOR | Existing debug linker LNK4098 warning and host memory pressure during parallel builds require reproducible release CI follow-up. |
| KNOWN LIMITATION | Windows session scope, common-location Ollama detection, external Node/.NET/editor installation and sign-in PATH differences. |
| KNOWN LIMITATION | Tiny English accuracy for Taglish is unverified; no acoustic echo cancellation guarantee. |
| KNOWN LIMITATION | First-machine WebView2 provisioning may require internet; no updater, model download manager or project process sandbox. |

Recommendation remains **Beta** until these acceptance gates are resolved. The audit/fixes/documentation do not convert unchecked manual tests into pass states.

## 12. Final validation record

- `npm run build`: PASS, including production frontend rebuilt by Tauri.
- `cargo check --manifest-path src-tauri/Cargo.toml --offline -j 1`: PASS on the final source.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline -j 1`: **88 passed, 0 failed, 10 ignored**. Includes new readiness HTTP-state, log-rotation and corrupt-store-preservation regressions.
- Explicit isolated process tests: **2 passed**, configured npm server start/readiness/stop and owned process-tree cleanup. These two were among the 10 ignored above; the other eight were not explicitly run in this phase.
- `node scripts/test-history.mjs`: PASS, millisecond boundaries/midnight/year rollover/DST.
- `node scripts/release101-ui-smoke.mjs`: PASS, all 11 navigation pages, project approval form and workflow editing. Native IPC not tested by browser checks.
- `git diff --check`: PASS apart from a line-ending normalization notice.

Process reproduction:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline -j 1 -- --ignored --test-threads=1 project_process::tests::owned_process_tree_stops project_process::tests::configured_npm_script_starts_and_stops_server
$env:CARGO_BUILD_JOBS = '1'
npm run tauri build
```

The earlier installer run overlapped source changes and is not the final acceptance artifact. Final installer completion and hash are recorded below after the last build finishes. No live human-audio, sign-in, installation/uninstallation, reboot or real-user-project result was fabricated.
