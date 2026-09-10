# NOVA wake-word engine

> Current implementation note: NOVA now recognizes both NOVA and Hey NOVA, conditions wake audio locally, and reports real threshold/noise/phrase/timestamp diagnostics. Sensitivity thresholds are Low 0.35, Medium 0.25, and High 0.18. See CONVERSATIONAL_VOICE_ENGINEERING_REPORT.md for the current pipeline.

## Selection

NOVA uses sherpa-onnx 1.13.7 with the English `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01` int8 model.

The engine performs open-vocabulary keyword spotting locally, so `Hey NOVA` is configured as a BPE token sequence and does not require phrase-specific cloud training. Inference uses one CPU thread and receives transient mono 16 kHz f32 PCM frames from NOVA's existing CPAL audio owner.

## Why this engine

- Fully offline inference; no account, API key, or network request at runtime.
- Custom phrases without retraining a phrase-specific classifier.
- Native Rust API and static Windows support.
- Small 3.3M-parameter KWS model with int8 encoder and joiner.
- Apache-2.0 engine and Apache-2.0 model metadata.

openWakeWord was not selected because `Hey NOVA` is not a bundled stock model and its current hosted custom-model workflow requires an account/credits and applies restrictive default model terms. Rustpotter was not selected because reference mode needs 3–8 user recordings and its own documentation does not position it as production-grade. Porcupine was excluded because its account/access-key and licensing model conflicts with NOVA's local-first requirements.

## Bundled model provenance

Official archive:
`https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2`

SHA-256:
`F170013B4716E41B62B9BFD809687C207CEF798EF9BC6534D524E17AF9B6561A`

NOVA bundles only the int8 encoder/decoder/joiner, `tokens.txt`, and `keywords.txt`. Audio is never written to disk by the wake engine.

## Sensitivity

- Low: threshold 0.35
- Medium: threshold 0.25
- High: threshold 0.18

A lower threshold is more responsive and can produce more false wakes. NOVA reports no accuracy metric until a representative evaluation dataset exists.
