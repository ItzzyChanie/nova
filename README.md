# NOVA

Local AI Desktop Assistant, built with Tauri 2, Rust, React, and TypeScript.

## Phase 02: floating assistant

The main window provides six navigable pages and interactive configuration previews.
Toggles and the sensitivity selector update typed React state only. No audio is
captured, no permissions are granted, and no commands are processed. Preferences
survive page navigation but reset on reload or restart. Clear all data remains disabled.
Test Assistant now shows the separate, reusable native assistant window.

The `assistant` window is created hidden at startup. It is a compact, opaque,
undecorated panel that stays above normal windows and is excluded from the taskbar.
Each activation recalculates its position at the top-center of the dashboard's
monitor work area, falling back to the primary monitor. Sizing and margins use
the target monitor's scale factor; no display resolution is assumed.

Dismiss, Escape, and the native close gesture hide the assistant rather than
destroying it. Closing the dashboard exits NOVA, including the hidden assistant.
There is no tray/background lifecycle yet. The assistant remains in `idle`; the
other typed UI states are reserved for future integrations. No audio or AI runs.

## Development

Install dependencies with `npm ci`, then run `npm run tauri dev` to launch the
Windows desktop app. This requires the Rust MSVC toolchain, Visual Studio C++
build tools, Node.js, and WebView2. Vite uses port 1420; run one dev session at a time.

Validate the frontend with `npm run build` and Rust with
`cargo check --locked --manifest-path src-tauri/Cargo.toml`.
Positioning tests: `cargo test --locked --lib --manifest-path src-tauri/Cargo.toml`.

## Frontend structure

- `src/App.tsx`: typed navigation and in-memory preview preferences.
- `src/main.tsx`: selects the dashboard or assistant root by native window label.
- `src/components/assistant/AssistantWindow.tsx`: compact idle assistant and dismissal.
- `src/services/assistantWindow.ts`: typed calls to the two native lifecycle commands.
- `src/components/layout/`: header-free shell, sidebar, and main landmark.
- `src/components/ui/`: shared page headings, cards, and keyboard-accessible toggles.
- `src/pages/`: separate Overview, CommandHistory, Skills, VoiceWakeWord, PrivacyData, and About page components.
- `src/types/navigation.ts`: navigation labels and page identifiers.
- `src/types/nova.ts`: typed frontend status contract.
- `src/types/assistant.ts`: idle, listening, thinking, executing, success, and error states.
- `src/types/settings.ts`: preview defaults, skill definitions, and preference types.
- `src/styles/tokens.css`: centralized color and typography tokens.
- `src/styles/app.css`: shared styles and responsive layout.
- `src/styles/assistant.css`: isolated assistant styling using the same design tokens.
- `src-tauri/src/assistant.rs`: positioning, show/hide commands, and close handling.

## Assistant permissions and smoke check

App commands are registered through Tauri's `AppManifest` so they participate in
the capability system. Only `main` gets `allow-show-assistant`; only `assistant`
gets `allow-hide-assistant`. Both commands also verify their calling window label.
No broad frontend window-mutation, shell, filesystem, or process permission is added.
The dashboard's existing capabilities remain scoped to `main`.
See [Tauri's capability documentation](https://v2.tauri.app/security/capabilities/).

Launch NOVA: only the dashboard should be visible. Click Test Assistant, dismiss
it, and repeat; the same assistant must reappear, without duplicates. Move the
dashboard to another monitor and repeat to check placement. Escape and Alt+F4 on
the assistant should hide it. Dashboard navigation should continue working.
Finally close the dashboard and confirm both native windows exit.

Hooks, services, and stores can be added when features need them. Tray behavior,
autostart, shortcuts, voice processing, AI, and desktop automation are not part
of this phase.

## UI direction

Use a compact developer-tool interface based on the supplied visual references:
navy/charcoal surfaces, thin borders, small radii, orange branding and selection,
mint for local/success indicators, and red only for errors or destructive actions.
Shared colors and typography live in `src/styles/tokens.css` as CSS custom properties.
Use sans-serif for primary UI and monospace selectively for technical metadata.

The default window is 900x650, with a 640x480 minimum. Keep the 220px sidebar on
normal desktop windows; smaller widths use a narrower left sidebar and one-column
cards. Do not switch to mobile navigation. Native window decorations provide the
title bar; do not add an inner title strip or decorative window-control dots.
Avoid hero layouts, excessive gradients, large icons, and invented metrics.

The assistant starts ON as a development preview, while no engine is running.
Apps/files and window-management skill previews start ON; other skill previews
start OFF. Voice reply, audio storage, and network access start OFF; command log
starts ON as a preference preview. Sensitivity starts at Medium. These states do
not represent live capabilities, installed engines, or native permissions.
