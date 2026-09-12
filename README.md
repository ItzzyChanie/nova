<p align="center">
        <img src="src/assets/Nova-logo.png" alt="NOVA logo" width="180">
</p>

# NOVA 1.0.1 - local Windows development assistant

NOVA is a compact desktop assistant with a dark navy interface, orange actions, mint status indicators and monospace technical details. It combines local wake-word detection, speech recognition, optional voice replies, and permission-checked desktop tools. Development projects and reusable workflows are configured by the user, not generated and executed by an AI model.

This is a **local Windows development release**, not a signed public distribution. See [the Phase 16 engineering report](docs/PHASE16_ENGINEERING_REPORT.md) for measured validation and remaining manual checks.

## Current installed release

[Version 1.0.1 release notes](docs/RELEASE_1.0.1.md) describe the floating assistant and dashboard layout fixes. Each installed release has a new version: patch increments for fixes (1.0.1), minor increments for new features (1.1.0). Package, Cargo, and Tauri versions must match before packaging.

## Features

- Floating assistant, Alt+N wake shortcut, Hey NOVA/NOVA keyword detection, tray controls, and a persistent global OFF switch tied to Windows autostart.
- Local Whisper transcription through sherpa-onnx; optional Windows SAPI voice replies with wake interruption and text fallback.
- Deterministic natural-language routing for common commands, with an optional local Ollama `qwen3:1.7b` classifier.
- Native application, approved file/folder, system-volume, system-information and window tools.
- Persistent project profiles: ID, name, path, editor, frontend command, backend command, working directory, development URL, notes and explicit voice permission.
- `developer.open_project`, `developer.start_project`, `developer.stop_project`, `developer.open_editor`, `developer.open_dev_url` and `workflow.run`.
- Workflows with editable names, ordered steps, enable/disable, voice permission, and confirmed deletion. Each step uses a typed native NOVA tool; recursion and arbitrary scripts are rejected.
- Local history with timestamp, input source, input, interpreted action, result, status and duration. Logging can be disabled.
- Real privacy controls, confirmed clearing of NOVA data, detected runtime metadata, and single-instance protection.

## Privacy philosophy

Core wake detection, transcription, speech synthesis, intent classification and native tools require no cloud AI service. Microphone audio is processed in memory and not saved by NOVA. History and configurations are stored in the application's local data directory.

The Network access setting controls NOVA opening external HTTP(S) websites. Loopback Ollama and development URLs stay available. **It is not a firewall**: editors, browsers, npm scripts, .NET applications and other launched programs have their own permissions and network behavior.

Clear local data removes NOVA settings, history, project profiles and workflows, clears transient conversation/search context, stops managed project processes, and turns NOVA/autostart off. It keeps actual project files and bundled model assets. You must type `CLEAR NOVA DATA` to proceed.

Initial provisioning of Node, Rust, WebView2, Ollama and model files may require downloads. Once installed, core inference does not depend on a cloud endpoint.

## Architecture and technology

```text
React 19 / TypeScript / Vite
  Dashboard + assistant webview
          |
  Tauri 2 scoped IPC capabilities
          |
  Native Rust schema validation + skill permissions
          |
  Application / file / system / window adapters
  Saved project runner / saved workflow runner
          |
  Windows APIs + owned Job Objects

Microphone -> CPAL conditioning -> sherpa-onnx wake detector
           -> VAD + Whisper -> deterministic parser / local Ollama
           -> native tool router -> text + optional Windows SAPI
```

The frontend cannot run a shell or access the filesystem through a broad plugin. The assistant webview receives voice/state events and has only hide/resize permissions. Configuration, data clearing and execution controls belong to the dashboard.

- Rust, Tauri 2, React 19, TypeScript 6, Vite 8.
- CPAL, sherpa-onnx, ONNX models for keyword spotting, VAD and Whisper.
- Windows SAPI desktop voices, COM, registry discovery and Windows Job Objects.
- `tauri-plugin-store`, autostart and global-shortcut plugins.
- Optional loopback-only Ollama API with `qwen3:1.7b`.

## Requirements

- Windows desktop with an x64 MSVC Rust toolchain and Visual Studio C++ build tools/Windows SDK.
- Node.js compatible with the pinned Vite toolchain (development machine: Node 22.15.0) and npm.
- WebView2 runtime, an available microphone for voice input, and an output device for voice replies.
- Microsoft David or Zira desktop voice for current SAPI support; otherwise NOVA keeps text responses.
- Bundled wake/VAD/Whisper resources in `src-tauri/resources`.
- Optional Ollama and the `qwen3:1.7b` model for commands needing model interpretation.
- For project commands: a conventional Node.js installation containing npm, or the .NET SDK. A supported editor must be installed.

## Development setup

```powershell
npm ci
npm run tauri dev
```

For UI-only work, `npm run dev` starts Vite. A browser preview cannot exercise native IPC or desktop tools; those pages report unavailable backend operations.

Provision optional language inference separately:

```powershell
ollama pull qwen3:1.7b
```

Ollama must be serving on `127.0.0.1:11434`. The native client rejects non-loopback endpoints. Common deterministic commands remain usable if Ollama is unavailable.

## Configure a project

1. Open **Projects > New project**.
2. Set its name, existing absolute folder and installed editor.
3. Configure `npm run <script>` and/or `dotnet run`. Both commands use the configured working directory, which must be within the project root. A blank working directory uses the root.
4. Optionally set a localhost development URL, such as `http://localhost:5173`.
5. Review and approve the configuration. Enable **Allow voice execution** only if spoken requests may run these saved actions.
6. Enable **Developer projects** in Skills, then use Open editor, Start, Stop or Open URL.

Start validates the saved runner and working directory, opens the editor, starts configured processes, waits up to 20 seconds for the configured loopback port, and opens the URL when reachable. A port already in use is reported instead of launching a duplicate server. NOVA's process Job Objects include descendants; Stop terminates only process trees it owns. Quitting NOVA or turning it off also stops these processes. Editor windows are not forcibly closed.

Commands are deliberately limited to `npm run <script>` and `dotnet run`. Arbitrary shell text, command chaining, inline scripts and model-provided command/path overrides are rejected. npm package scripts are still executable code: approve only repositories you trust, and review changes to their scripts.

## Workflows and voice

In **Workflows**, create a routine, add supported actions, reorder steps with Up/Down, review its full sequence and save it. Actions include applications, projects, approved folders, HTTP(S) websites, volume and window focus. Skills and network preferences are checked at execution time. A failure stops subsequent steps; earlier successful actions remain in place.

Examples after configuring and approving the corresponding profiles:

- "Hey NOVA, start ProctorX."
- "Hey NOVA, start coding mode."
- "Open my project in VS Code." (one saved project; uses its configured editor)
- "Stop ProctorX."

Voice execution is OFF by default per project/workflow, including migrated profiles. Without this explicit approval, NOVA presents a confirmation-required response. A model cannot set the approval flag, create a profile or change its commands.

## Build and validation

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
npm run tauri build
```

The configured Windows bundle target is NSIS. A successful production build places the executable under `src-tauri/target/release` and the installer under `src-tauri/target/release/bundle/nsis`. Building may download installer tooling on its first run. The actual packaging result is recorded in the engineering report; do not infer installer success from `cargo check` alone.

Local browser regression, after `npm run build`:

```powershell
node scripts/phase16-ui-smoke.mjs
```

This uses an isolated headless Edge profile, checks navigation and editor UI, and writes results/screenshots under ignored `.tmp`. It does not mock or validate native IPC.

Hardware/model smoke tests are explicitly ignored in the default Rust suite. See the engineering report and [manual voice matrix](docs/MANUAL_VOICE_TEST_MATRIX.md) for their prerequisites and measured outcomes.

## Resource behavior

- One NOVA instance per Windows session; subsequent launches bring the existing dashboard forward when available.
- Wake detection runs continuously only when NOVA is enabled and not paused. OFF drops its active stream/detector. The shortcut remains registered but is gated by the native OFF check.
- Whisper is loaded on demand and its cache is released after 30 seconds idle.
- Ollama receives a 30-second keep-alive; NOVA does not start duplicate Ollama servers or kill an externally managed Ollama process.
- SAPI objects are released after each reply. Wake interruption purges current output before recording resumes.
- Background worker threads block on channels while idle. Project status polling is limited to the open Projects page and skips hidden documents.

## Security model

Trust is placed in the local user, approved project contents and installed applications. Model output has no authority. Models produce typed tool requests only. Native code validates names, argument shapes, bounds, skills and OFF state again before execution. Unknown keys are rejected, including attempts to attach command strings or approval flags to project requests.

A saved workflow can call only its approved step allowlist; it cannot recursively run workflows or execute `developer.runScript`, file deletion or shutdown. Arbitrary custom scripts remain unsupported and denied. Project commands bypass a shell at the NOVA process boundary, and Windows suspended-start/job assignment ensures ownership before code runs. Project scripts themselves are not sandboxed.

Capabilities are window-scoped. Production CSP restricts script and connection sources; no broad shell, filesystem or HTTP plugin is exposed. External URLs use HTTP(S) only, reject embedded credentials, and obey the network preference. File tools retain their existing canonical-path containment checks.

## Current limitations

- Windows-only development release; unsigned installer, with no updater or signing infrastructure.
- English-oriented bundled STT and David/Zira TTS. Multilingual recognition requires a compatible user-selected Whisper export.
- Acoustic wake accuracy, echo-induced wakes and microphone behavior depend on hardware. There is no full acoustic echo cancellation.
- Project runners currently support npm scripts and `dotnet run`; frontend/backend share one working directory. Output logs and terminal tabs are not embedded in the UI.
- URL readiness checks loopback port reachability, not application-specific health or HTTPS certificate validity.
- Workflow failures do not roll back earlier steps. Editing/deleting an active profile requires stopping it first.
- Browser navigation tests do not replace native Windows interaction, autostart sign-in, tray, microphone and installer acceptance testing.

## Screenshots

Placeholder: add reviewed screenshots of the dashboard, floating assistant, Projects, Workflows, and Privacy & data before public distribution. Local automated captures are written to `.tmp/phase16-projects.png`.
