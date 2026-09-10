# NOVA conversational voice engineering report

## Diagnosis

Previous pipeline:

Microphone through CPAL default input format -> channel averaging and nearest-sample 16 kHz conversion -> fixed Hey NOVA Zipformer keyword -> command capture -> fixed RMS gate -> 200 ms post-wake pre-roll -> 900 ms trailing-silence endpoint -> Whisper Tiny English INT8 greedy decode -> simple transcript checks -> deterministic phrase classifier or local qwen3:1.7b -> one typed request -> native validation -> permission policy -> native execution -> floating-window response.

Root causes found:

- Wake input had no DC removal, rumble filtering, gain control, or noise-floor diagnostics.
- Only Hey NOVA existed in the keyword file; NOVA alone could not match.
- Command VAD used one fixed 0.012 RMS threshold regardless of microphone or room noise.
- The bundled Tiny English model prioritizes speed and memory over accented-English accuracy and cannot transcribe Taglish reliably.
- The model prompt treated input mainly as noisy commands and included phrase-specific corrections rather than broad conversational guidance.
- File search sorted primarily by modification time after a token filter, allowing a recent partial match to outrank an exact stem.
- A successful file search automatically opened a single result even for show/where intent.
- No bounded candidate/application context existed for the second one, open it, show its folder, or corrections.
- The model schema supported one tool only.

## Updated pipeline

Microphone -> mono 16 kHz conversion -> DC blocker and clipping guard -> conservative speech-only gain -> local dual-phrase KWS -> wake event -> same-stream command capture with 200 ms pre-roll -> adaptive noise-floor energy VAD -> sustained-silence endpointing -> cached local Whisper -> conservative linguistic normalization -> bounded entity/candidate context -> deterministic resolver or local LLM -> one typed tool or validated 2-4 step plan -> schema validation -> permission preflight -> deterministic native execution -> concise assistant state/result.

Audio remains transient and in memory. No microphone data, transcripts, filenames, context, or history are sent outside the device. Ollama remains fixed to loopback.

## Implemented changes

- Added local keyword entries for NOVA and Hey NOVA.
- Low, Medium, and High use understood KWS thresholds 0.35, 0.25, and 0.18. Medium remains the recommended default. A three-second debounce and capture state prevent nested double wakes.
- Added a one-pole DC blocker, clipping cap, noise estimate, and smoothed gain capped at 2.0x for wake audio and 2.5x for detected command speech. Steady noise is not deliberately boosted.
- Replaced fixed command gating with an adaptive threshold: max of 0.008 RMS and 2.4 times estimated noise. Two voiced frames are required. Natural pauses shorter than 900 ms remain within one utterance.
- Kept 200 ms pre-roll and the same microphone stream across wake and command capture, avoiding device restart delay after wake.
- Added real microphone, level, noise floor, engine, phrase mode, sensitivity, threshold, last phrase, and timestamp diagnostics. Wake confidence is marked unavailable because sherpa-onnx does not expose a calibrated value.
- Kept bundled Whisper Tiny English INT8 as the fast low-memory option. Custom compatible Whisper directories may use model-prefixed or canonical encoder/decoder/token filenames. English-only .en exports force English; multilingual exports use local auto-detection for English, Filipino, and Taglish.
- Added conservative wake-prefix and grammatical Taglish normalization without filename/application-specific phonetic replacements.
- Expanded generic show, locate, and where-is file intent. Strong single results are revealed in Explorer for show/where intent and only opened for explicit open intent.
- File ranking now prioritizes exact filename/stem, then prefix/token relevance, then modification time. Searches remain bounded to approved roots, 10,000 visits, depth six, and 50 results.
- Added five-minute in-memory context capped at ten candidates, with ordinal/newer selection, selected-file references, application corrections, cancel, and never-mind clearing.
- The local prompt now describes natural grammar, Filipino-accented English, minor STT errors, Taglish, local entities, context, and clarification. Shell/script prohibition and native validation remain unchanged.
- Added a strict model schema for 2-4 step plans. Every step is independently schema-validated and permission-preflighted. Automatic multi-action execution is intentionally not enabled without an explicit plan-confirmation boundary.

## Performance and resources

DC blocking, RMS estimation, and bounded gain are linear, allocation-light operations on small audio frames. KWS remains one CPU thread. Whisper is cached after first use and uses at most four CPU threads. The default model has unchanged disk/RAM needs. A custom larger or multilingual model increases startup latency and memory according to that model.

Recommended defaults: Medium wake sensitivity. For lowest resource use, bundled Tiny English. For a Filipino user who needs Taglish or better accent robustness, validate a compatible multilingual base-class Whisper export on the actual laptop and use it if latency and RAM remain acceptable.

## Limitations

Software cannot recover speech that the physical microphone does not capture. Distance, microphone directionality, Windows enhancements, fan/keyboard noise, reverberation, speaker playback, and competing voices still matter. No universal accuracy claim is made. The runtime exposes no calibrated Whisper token confidence or KWS posterior. Full compound execution needs an explicit user-visible plan confirmation API. Opening a registered project inside a specific editor needs a dedicated typed project tool rather than shell arguments.

No dependencies were added or removed.

# Lightweight follow-up review

The subsequent review fixed explicit file-open follow-ups inheriting a previous
reveal action, restricted "the other one" to two candidates with a selection,
and made application corrections resolve the replacement rather than the
negated name. Regression cases cover reveal/open transitions, alternative
selection, ambiguous alternatives, correction, and cancellation.

The frontend now represents single requests and sequence plans as distinct
types and offers single-tool confirmation only for single requests. Sequence
plans no longer start an unsupported voice-confirmation loop. Whole-plan
confirmation and execution remain unfinished.

These follow-up changes received static review only; compilation and test
execution are deferred due to the machine's low-memory condition.
