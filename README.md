# NOVA

Local AI Desktop Assistant, built with Tauri 2, Rust, React, and TypeScript.

## Phase 1: foundation

The main window provides six navigable pages and interactive configuration previews.
Toggles and the sensitivity selector update typed React state only. No audio is
captured, no permissions are granted, and no commands are processed. Preferences
survive page navigation but reset on reload or restart. Test Assistant and Clear
all data remain disabled until their underlying features exist.

## Development

Install dependencies with `npm ci`, then run `npm run tauri dev` to launch the
Windows desktop app. This requires the Rust MSVC toolchain, Visual Studio C++
build tools, Node.js, and WebView2. Vite uses port 1420; run one dev session at a time.

Validate the frontend with `npm run build` and Rust with
`cargo check --locked --manifest-path src-tauri/Cargo.toml`.

## Frontend structure

- `src/App.tsx`: typed navigation and in-memory preview preferences.
- `src/components/layout/`: header-free shell, sidebar, and main landmark.
- `src/components/ui/`: shared page headings, cards, and keyboard-accessible toggles.
- `src/pages/`: separate Overview, CommandHistory, Skills, VoiceWakeWord, PrivacyData, and About page components.
- `src/types/navigation.ts`: navigation labels and page identifiers.
- `src/types/nova.ts`: typed frontend status contract.
- `src/types/settings.ts`: preview defaults, skill definitions, and preference types.
- `src/styles/tokens.css`: centralized color and typography tokens.
- `src/styles/app.css`: shared styles and responsive layout.

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
