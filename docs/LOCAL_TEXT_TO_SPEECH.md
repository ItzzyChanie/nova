# Phase 15: local text-to-speech

## Compatibility decision

NOVA is a Tauri 2 / Rust Windows desktop app. It already uses windows 0.54 for native APIs. This phase uses native Windows SAPI with an explicitly selected Microsoft David or Zira desktop voice. Both voice tokens were found on the development machine. No Python process, cloud API, HTTP request, model download, or additional inference model is involved in TTS. A machine without either supported voice keeps text responses.

| Candidate | Windows integration | Resource and packaging implications | Decision |
| --- | --- | --- | --- |
| Windows SAPI | Native COM through the existing windows crate | Installed desktop voices; per-reply COM objects, released immediately afterward | Selected for Phase 15 |
| Piper | Local neural engine with Python and C/C++ interfaces | Additional engine/voice packaging; current upstream uses GPL-3.0 | Viable later if neural voice quality is needed |
| Kokoro | Local 82M-parameter model; Python/PyTorch reference implementation | Additional weights, inference runtime and phonemization dependencies | More packaging than needed for short desktop acknowledgements |

Sources checked: [Microsoft ISpVoice::Speak](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee125024(v=vs.85)), [Piper upstream](https://github.com/OHF-Voice/piper1-gpl), [Kokoro upstream](https://github.com/hexgrad/kokoro). SAPI supports asynchronous speech and purging pending output. NOVA forces plain text, never filename or XML interpretation, and does not select arbitrary third-party/default voice providers.

## Behavior

- Voice & wake word > Voice reply persists `voiceReply` in the native `nova-settings.json` store. Existing installations default to OFF. A failed save restores the previous in-memory setting and reports the error in the settings page.
- Completed spoken commands display their text first and then speak a concise acknowledgement when enabled. App names and status come from actual tool results. Errors use short fixed phrases. Long responses, file paths, markup and diagnostic logs stay on screen.
- The microphone continues keyword spotting during playback. Follow-up command recording begins only after playback finishes or fails, so it does not transcribe the reply itself.
- Waking NOVA invalidates pending replies and purges active speech before recording. Follow-up requests carry the reply generation and are checked again inside the audio worker. Older command completions cannot restart a superseded listening session.
- Manual wake, dismissal and Voice reply OFF also cancel playback. NOVA OFF dismisses the assistant and cancels speech.
- Playback is serialized. COM and speech objects are scoped to one reply; no persistent TTS model or worker thread remains idle. Playback has a 20-second limit and polls cancellation every 20 ms while rendering. Engine initialization time and driver behavior can add latency.
- Engine errors are logged locally without the spoken text, leave the visible result intact, and do not prevent later commands or follow-up listening.

## Validation

Recorded on the Windows development machine:

- `npm run build`: passed.
- Native library suite: 73 passed, 0 failed, 8 ignored (hardware/model-dependent tests).
- Explicit Windows SAPI playback/cancellation smoke test: 1 passed, 0 failed, completed in 9.28 seconds. This exercises real desktop speech, cancellation, and a subsequent reply.
- `git diff --check`: passed.
- The native linker reports LNK4098 (LIBCMT default-library conflict); it did not prevent compilation or either test run.
- Restart persistence and acoustic microphone/speaker acceptance remain unmeasured; see the manual matrix below.

Automated cases in `src-tauri/src/tts.rs` cover ON/OFF engine gating, completed app-open and running-status summaries, search counts, concise errors, diagnostic suppression, injected engine failure and recovery, cancellation before listening, stale reply suppression, and serialization of multiple replies.

Commands:

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline tts::tests::installed_windows_voice_speaks_and_cancels -- --ignored --exact --nocapture
```

The ignored Windows smoke test sends actual speech to the installed output device, cancels a sentence after 300 ms, then speaks again. It does not verify subjective audibility or acoustic wake detection.

Manual acceptance on the target speaker/microphone remains necessary:

| Case | Action | Expected |
| --- | --- | --- |
| ON and restart | Enable Voice reply, restart NOVA, speak an app-open command | Toggle remains ON; text plus one short acknowledgement |
| OFF and restart | Disable Voice reply, restart, repeat the command | Toggle remains OFF; text only |
| App open | Say Open Visual Studio Code, then repeat | Opening acknowledgement, then already-open acknowledgement if detected |
| Error | Request an unknown application | Concise error plus full text details |
| Wake interruption | Say Hey NOVA while a reply is playing, then give a command | Old reply stops, new command is captured; old follow-up does not restart |
| Multiple commands | Give three commands with follow-ups | Replies do not overlap and speech is not captured as the next command |
| Failure fallback | Use a target with no supported desktop voice or unavailable output | Text remains usable; subsequent commands still work |
| Acoustic feedback | Repeat at normal speaker volume and microphone distance | No self-wake; user wake is detected during playback |

Do not report the manual matrix as passed without microphone measurements. Phase 15 ends here.
