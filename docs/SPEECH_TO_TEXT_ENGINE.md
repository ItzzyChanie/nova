# NOVA local speech-to-text engine

> Current implementation note: the conversational voice update adds adaptive noise-floor VAD, DC blocking, bounded speech-only gain, generic compatible Whisper model discovery, multilingual auto-detection for non-English-only models, transcript normalization, and routing. The older Phase 13 description below is retained as historical design context; see CONVERSATIONAL_VOICE_ENGINEERING_REPORT.md for the current pipeline.

## Selection

Phase 13 uses OpenAI Whisper Tiny English exported to quantized ONNX and executed locally by `sherpa-onnx` 1.13.7 on the CPU.

This was selected because it:

- is fully offline after the model files are bundled;
- requires no cloud API, account, runtime download, or license key;
- supports Windows through the native runtime already used by NOVA's wake-word engine;
- avoids keeping a speech model loaded while NOVA is only waiting for the wake phrase;
- provides official INT8 Whisper encoder and decoder exports suitable for lower latency and memory use.

A direct `whisper.cpp` Rust wrapper was evaluated first. The currently tested wrappers generated bindings during every Windows build and required a separately installed `libclang.dll`; one wrapper's pre-generated-binding fallback was not ABI-compatible with MSVC. NOVA does not impose that developer-machine dependency. The selected backend still runs the Whisper model family locally, while sharing NOVA's proven sherpa-onnx runtime.

References:

- [sherpa-onnx Whisper Tiny English model instructions](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/tiny.en.html)
- [sherpa-onnx project](https://github.com/k2-fsa/sherpa-onnx)
- [OpenAI Whisper](https://github.com/openai/whisper)

## Runtime flow

1. The lightweight Zipformer keyword spotter listens while NOVA is enabled and not paused.
2. `Hey NOVA` reuses the single hidden assistant window and transitions it to `Listening`.
3. Audio is normalized to mono 16 kHz `f32` and held only in memory.
4. Capture waits up to 4 seconds for speech, ends after 0.9 seconds of trailing silence, and has a hard 10-second limit.
5. The wake stream is stopped during transcription so the microphone is not consumed twice.
6. A dedicated worker loads/caches Whisper only when needed and emits `Transcribing`, then `Processing` with `Heard: <text>`.
7. Audio samples are dropped after the worker completes. The lightweight wake listener is restored for the next command.

No transcript is sent to a model or tool router in Phase 13. AI reasoning remains intentionally disconnected.

## Error handling

The native pipeline returns typed states for no speech, low transcript quality, a missing/incomplete model directory, microphone loss, engine failure, and recording timeout. Failures are displayed in the floating assistant and logged for development. They do not crash NOVA, retry in a loop, or create another assistant window.

The sherpa Whisper API does not expose calibrated token probabilities. NOVA therefore uses a conservative deterministic quality gate for empty/silence markers, invalid text, empty tokens, and excessive unknown tokens. The UI deliberately does not display this internal score as model accuracy.

## Model configuration

The bundled model is `Whisper Tiny English INT8`. A custom absolute model directory can be saved locally from Voice & wake word. It must contain:

- `tiny.en-encoder.int8.onnx`
- `tiny.en-decoder.int8.onnx`
- `tiny.en-tokens.txt`

Use bundled model clears the override. The selected directory persists in `nova-settings.json` under `speechModelPath`.

## Privacy and resources

Audio storage stays unavailable/off. Command and microphone-test samples never leave the device and are never written to disk. Whisper is CPU-only, limited to at most four inference threads, and is not loaded merely for wake-word detection.
