# NOVA manual voice test matrix

Run this matrix on each target microphone. Record wake success, false wake, raw transcript, normalized transcript, semantic intent, chosen tool/plan, execution result, and latency for wake-to-UI, speech-end-to-text, text-to-plan, and plan-to-result.

| Environment | Distance or condition | Phrases |
| --- | --- | --- |
| Quiet | close microphone | NOVA; Hey NOVA; Open VS Code |
| Quiet | normal seated laptop distance | NOVA, open Chrome; Hey NOVA, open Downloads |
| Quiet | slightly farther normal room distance | Hey NOVA; find my thesis |
| Fan | normal seated distance | NOVA; close File Explorer |
| Keyboard | intermittent typing | Hey NOVA; show my resume |
| Background audio | soft, non-speech where practical | NOVA; set volume to thirty percent |
| Voice level | normal then softer | NOVA; open Documents |
| Speaking rate | faster natural conversation | Hey NOVA, open File Explorer and go to Downloads |
| Pause | 300-700 ms mid-sentence | Open File Explorer ... and go to Downloads |
| Accent | natural Filipino-accented English | applications, files, projects, and system controls |
| Taglish | multilingual model only | Open mo yung VS Code; pakiclose yung File Explorer; hanapin mo yung Chapter 1 ko |
| Follow-up | same five-minute session | Find Chapter 1; show the second one; open it; show its folder |
| Correction | same session | Open Chrome; no, I meant File Explorer |
| Ambiguity | duplicate filenames | Find Chapter 1; verify NOVA asks which one |
| False wake | five minutes of ordinary work | conversation without NOVA, typing, fan, and soft playback |

Test Low, Medium, and High separately. Do not tune from one utterance. Compare at least 20 positive wake attempts per phrase and a meaningful ordinary-work false-wake period. Hardware-dependent tests are manual and must not be reported as passed until measured.
