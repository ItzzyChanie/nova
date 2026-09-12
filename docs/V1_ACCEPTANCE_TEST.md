# NOVA v1 installed-build acceptance

Phase 17, 2026-09-12. Test the newly built **1.0.1** installer; the repository already used that version at audit start. This is an execution worksheet, not a declaration that manual testing passed.

## Evidence rules

Record installer SHA-256, Windows build, machine CPU/RAM, microphone, display scale/monitors, installed executable path, test date and operator. Use a disposable Windows user or VM for clean installation and destructive uninstall/data tests. Keep real project files unchanged. Mark a checkbox only after observing its expected result. Record failures, screenshots when useful, and exact recovery steps. Never infer acoustic accuracy from native fixtures or browser tests.

Automated results are recorded separately in [PHASE17_RELEASE_AUDIT.md](PHASE17_RELEASE_AUDIT.md). All boxes below initially require manual verification.

## Installation and first launch

- [ ] Verify artifact hash and unsigned signature status. Record SmartScreen/unknown-publisher behavior; do not disable Windows protection or bypass certificate checks.
- [ ] Fresh NSIS installation in a clean user account succeeds. Record chosen install directory, Start Menu entry and optional desktop shortcut.
- [ ] With WebView2 installed, launch works. Separately test a clean machine without WebView2 and document bootstrapper/network requirements.
- [ ] Launch from Start Menu and from a shell whose working directory is outside the repository. Dashboard opens once; assistant stays hidden.
- [ ] About shows 1.0.1, unsigned publisher status and actionable diagnostic states. Run diagnostics twice; no duplicate audio workers or downloads.
- [ ] Fresh-install NOVA defaults ON and registers Windows autostart. Turn OFF and verify the Run value disappears; turn ON and verify the installed executable plus --background is registered.

## Dependencies and failure recovery

- [ ] About reports actual selected microphone presence. Voice settings capture test verifies access separately.
- [ ] Deny Windows microphone permission, disconnect selected microphone, remove input while listening, then reconnect/reselect. Dashboard remains usable, errors are actionable, retry recovers without duplicate streams.
- [ ] In a disposable installation, temporarily move each bundled wake/Whisper/VAD resource aside. Diagnostics reports missing resources; voice failure does not crash the dashboard. Restore resources and restart.
- [ ] With source repository inaccessible, installed voice resources resolve under the installation resources directory.
- [ ] Configure an external absolute Whisper directory, restart, then move that directory. Model missing is reported. Reset override and verify bundled model recovery. OFF/pause still releases capture with invalid configuration.
- [ ] Voice reply succeeds with an available supported Windows SAPI voice; unavailable voice leaves the text result intact.
- [ ] Ollama absent: Not detected, model unverified, deterministic tools usable.
- [ ] Ollama installed but stopped: Service not running at 127.0.0.1:11434, no NOVA crash.
- [ ] Ollama running without qwen3:1.7b: Model missing with explicit install guidance. No automatic model download.
- [ ] Ollama and model installed: Installed/loads on demand or Loaded reflects real availability. Execute a model-assisted command.
- [ ] Stop Ollama during a request: bounded failure, later deterministic command still works. Restart Ollama and retry diagnostics/command.
- [ ] Missing editor/executable, invalid project path and permission-denied file access return useful failures and preserve dashboard responsiveness.

## Tray, shortcut and instances

- [ ] Exactly one tray icon. Open NOVA restores and focuses dashboard, including after minimization.
- [ ] X hides dashboard; Open restores it. Wake NOVA shows one assistant and begins listening while enabled.
- [ ] Pause/resume stops/restarts wake listening without changing NOVA ON/OFF or autostart.
- [ ] Launch a second manual instance with dashboard hidden and minimized. Existing dashboard is brought forward, second process exits, no extra tray/audio/shortcut registration.
- [ ] Launch twice in rapid succession during cold startup; activation is delivered after initialization.
- [ ] Launch a duplicate with --background; dashboard stays hidden.
- [ ] Exercise Alt+N with Chrome, VS Code, File Explorer and desktop focused, and dashboard hidden. One correctly positioned overlay; test high DPI, multiple monitors and taskbar positions.
- [ ] Reserve Alt+N in another app before launching NOVA. Diagnostics reports conflict; tray wake remains usable. Release conflict and restart NOVA.
- [ ] OFF denies wake/tools; ON restores them. Closing assistant cancels capture and leaves exactly one wake listener.

## Real audio matrix

Run both ?NOVA? and ?Hey NOVA? at laptop sitting distance and a farther desk distance. Repeat normal-volume, natural-speed Filipino-accented English in quiet, fan, keyboard and mild room noise. Record distance and noise rather than estimating acoustic quality. Taglish command trials use the current model; bundled Tiny English does not promise multilingual transcription. Repeat failed and successful samples enough to expose intermittent failures.

- [ ] Wake matrix completed with attempts, successes, false activations and latency.
- [ ] Command matrix completed for each phrase below after both wake and Alt+N.

| Phrase | Wake result | Raw transcript | Resolved intent | Tool result | Latency / environment |
| --- | --- | --- | --- | --- | --- |
| Open VS Code. | Unverified | | | | |
| Can you open my code editor? | Unverified | | | | |
| Close File Explorer. | Unverified | | | | |
| Open mo yung Downloads. | Unverified | | | | |
| Find my Chapter 1. | Unverified | | | | |
| Show me where my resume is. | Unverified | | | | |
| Open ProctorX. | Unverified | | | | |
| Open VS Code and open ProctorX. | Unverified | | | | |

Raw transcripts belong in this operator-controlled test record only; diagnostic logs intentionally omit them. Do not record microphone audio unless separately and explicitly authorized.

## Applications, files and system controls

- [ ] Open/focus supported installed apps; missing apps are identified. Confirm closing only intended test applications.
- [ ] Create disposable valid Chapter 1.docx, Chapter 1.pdf and Resume.pdf fixtures inside a permitted test folder. Add two matches for the same name in separate folders.
- [ ] ?Find Chapter 1?, ?Show Chapter 1?, ?Show me where my resume is? return/reveal the intended fixture. Multiple matches request clarification; no arbitrary opening.
- [ ] Explorer reveal selects the exact file; spaces and non-ASCII names work.
- [ ] Volume/mute read and approved change work; restore original value. Battery/CPU/RAM query works.
- [ ] An explicitly requested screenshot is written to application data/screenshots and opens correctly. No screenshot is taken by diagnostics.
- [ ] Disallowed paths, destructive commands and unapproved actions remain denied/confirmation-required.

## Projects, workflows and persistence

- [ ] Register one real existing project with its absolute directory and installed editor. Record initial file state.
- [ ] Save a supported npm run <script> or dotnet run command, explicitly approve execution and voice permission as appropriate.
- [ ] Restart NOVA; project profile and workflow order persist.
- [ ] developer.open_project opens the correct project/editor.
- [ ] developer.start_project launches from the approved working directory; developer.open_dev_url opens the configured loopback URL.
- [ ] Repeat start; no duplicate project worker. developer.stop_project ends NOVA-owned descendants only.
- [ ] Unapproved voice start remains confirmation-required. Model-supplied commands/paths/approval cannot override the saved profile.
- [ ] Execute a saved workflow. Force a middle step to fail; later steps do not run, earlier actions are reported and remain as documented.
- [ ] Settings, selected microphone, history preference, projects and workflows survive restart/reinstall. Disable history and confirm no new command-history entries.
- [ ] Verify project files were not unexpectedly edited by NOVA; approved package scripts may have their own documented effects.

## Quit, sign-in and reboot

- [ ] Tray Quit terminates NOVA, overlay, microphone access and owned project jobs. Ollama remains running independently if already running.
- [ ] ON followed by sign-out/sign-in: installed NOVA starts hidden, one tray, one wake listener, no dashboard/overlay/console.
- [ ] OFF followed by sign-out/sign-in: NOVA does not automatically start.
- [ ] Reboot with ON; verify the same behavior and readiness after dependencies become available.
- [ ] Run overnight idle/wake checks; no progressive CPU/RAM growth, duplicate workers or persistent microphone streams after OFF.

## Resource measurements

Use Task Manager/Performance Monitor with executable path and PID recorded. Include NOVA's WebView2 children and Ollama model runner separately. CPU means interval CPU-time delta / elapsed time / logical processors, not cumulative CPU seconds. Record sample duration and cold/warm state.

| Scenario | Duration | NOVA CPU | NOVA + WebView2 RAM | Ollama + runner RAM | Latency |
| --- | --- | --- | --- | --- | --- |
| OFF/idle dashboard hidden | Unmeasured | | | | |
| Wake listener active | Unmeasured | | | | |
| Whisper command, cold then warm | Unmeasured | | | | |
| Ollama model, cold then warm | Unmeasured | | | | |
| After OFF and after Quit | Unmeasured | | | | |

- [ ] Fill measurements; compare repeated commands and overnight samples for growth. Observe Whisper cache release after idle and owned child cleanup.

## Upgrade, uninstall and reinstall

- [ ] Quit through tray before upgrade; reinstall the newly hashed 1.0.1 artifact. Check About and resources, not version alone (same-version replacement).
- [ ] Uninstall while running: record NSIS running-process prompt/cleanup behavior; no zombie NOVA or owned project jobs.
- [ ] Default uninstall removes executable/resources/shortcuts and HKCU Run NOVA entry. Reboot has no broken startup registration.
- [ ] Default uninstall retains local settings/history/projects/logs. Verify retained data explicitly.
- [ ] Separately, only in disposable account, select the delete-app-data checkbox: verify documented Roaming and Local com.nova.assistant directories are removed.
- [ ] Reinstall after default uninstall restores retained settings and refreshes autostart to the new installed path. Reinstall after data deletion behaves like first launch.

## Acceptance decision

- [ ] Attach observed evidence for every required check; distinguish untested, pass and fail.
- [ ] Resolve every BLOCKER in the audit before declaring Stable. Record remaining MAJOR, MINOR and KNOWN LIMITATION issues.
