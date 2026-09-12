# NOVA 1.0.1 - installed UI fixes

## Changes

- Dashboard and floating assistant now use separate HTML entry points (`index.html` and `assistant.html`) with independent production stylesheets. The previous conditional lazy import caused production preloading to mix their global CSS rules.
- Restored the floating assistant's orb, input equalizer and processing gradient border. The ring animates while listening or processing, while equalizer motion follows input. Reduced-motion preferences remain respected.
- Floating assistant starts at 520 by 180 logical pixels and expands vertically with response content. Very long responses retain pagination. Transparent window roots have no dashboard minimum width or scrollbars.
- Dashboard defaults to 1120 by 820 logical pixels, with a 900 by 680 minimum. Compact sidebar spacing keeps all 11 navigation items accessible without sidebar scrolling at these tested sizes.
- Dashboard root is bounded to the viewport. Positioned accessibility labels are contained by the page scroll region, removing the extra outer scrollbar and blank background. Long settings pages retain their one intended content scrollbar.
- Package, Cargo and Tauri versions are 1.0.1. The application identifier remains `com.nova.assistant`, preserving the existing local data location.

## Validation

- `npm run build`: passed with separate dashboard and assistant HTML/CSS outputs.
- `cargo check --manifest-path src-tauri/Cargo.toml --offline`: passed.
- `node scripts/release101-ui-smoke.mjs`: passed all 11 navigation pages, project approval form and workflow editing.
- Production browser layout tests: no document or sidebar overflow at 1120x820 and 900x680. Assistant has only its own stylesheet, transparent root and no overflow at 520x180.
- Computed animation names verified: `voice-ring`, `equalize`, `processing-border`. A long response expanded the panel to about 275 pixels; resizing the viewport to fit preserved zero document overflow. Screenshots were visually inspected.
- These browser checks supply presentation states and viewport resizing; they do not claim live microphone events, native resize IPC, or installation acceptance were exercised. Previous speech/tool regression evidence remains in the Phase 16 report.
- Detailed local evidence: ignored `.tmp/release101-*` logs, JSON results and screenshots.

## Packaged artifact

Production build completed successfully on September 12, 2026: `npm run tauri build` exited with code 0 and generated one NSIS bundle. The build emitted a non-blocking Vite plugin timing warning.

- [nova_1.0.1_x64-setup.exe](../src-tauri/target/release/bundle/nsis/nova_1.0.1_x64-setup.exe)
- Detected product version: **1.0.1**
- Size: **60,893,467 bytes**
- SHA-256: `3735E6404426AAE8110B4181899C9698D18AE65BAA64D07735E1D65365837026`

The final build's `dist/index.html` and `dist/assistant.html` were checked for separate stylesheet references. Tauri explicitly loads `assistant.html` in the assistant window. Installer installation and native interaction acceptance were not repeated for this fix release.

## Upgrade

Quit the existing NOVA instance through its tray menu, run the 1.0.1 installer, and launch NOVA again. Check About for version 1.0.1. The same application identifier and stores retain local settings and profiles. Installer upgrade and high-DPI/multiple-monitor visual acceptance should be checked on the target device; this release remains unsigned.
