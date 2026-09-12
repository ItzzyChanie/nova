# Voice command recognition and response latency

## Changes

NOVA resolves complete assistant controls before calling the local model. Polite forms such as "Close the NOVA, please" and "Can you close the NOVA" dismiss the floating assistant. A bounded set of transcription variants (`cloze`, `clows`, `clothes`; `no va`, `nover`) is accepted only with an explicit assistant target. This does not enable fuzzy matching of other application names.

"Turn off" and "Turn off NOVA" use the existing NOVA OFF behavior, including disabling wake/shortcut actions and stopping managed projects. "Close NOVA" only hides the floating assistant. A lone "NOVA" never implies close or OFF. Negated requests are rejected before substring matching.

Command capture retains 500 ms before speech detection, increased from 200 ms, to preserve quiet initial consonants. End-of-speech silence is 700 ms rather than 900 ms. The model response deadline is 8 seconds rather than 75 seconds, enforced across all reads; a trickling response cannot reset it. Connection, speech transcription and tool execution have separate costs, so this is not an eight-second end-to-end guarantee.

Ambiguous speech now displays a short repeat prompt while listening resumes. A partial control phrase gets immediate deterministic feedback; model errors/timeouts return a concise retry instruction. No cloud engine or extra model was introduced.

## Validation scope

Tests cover polite control phrases, bounded transcription variants, standalone OFF, rejection of negation/unrelated targets, retention of a quiet onset, and a loopback server that trickles data beyond the response deadline. Existing local policy, speech endpoint and TTS regressions are also run.

These changes cannot reconstruct a word that was never captured or guarantee accent accuracy. Live pronunciation and room/microphone behavior require acceptance with the user's voice. No confidence score is fabricated. Avoid closing NOVA based only on recognizing its wake word.

Final native regression: `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline` passed with **85 passed, 0 failed, 10 ignored**. Hardware/accent acceptance was not run. This source change has not been packaged into a new installer; the existing 1.0.1 installer predates it.
