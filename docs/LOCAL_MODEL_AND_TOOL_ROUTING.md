# NOVA Phase 14: Local Model and Controlled Tool Routing

> Current implementation note: the active prompt now treats input as natural conversation, supports Filipino-accented English and basic Taglish when transcription permits, uses five-minute bounded local candidate/application context, and accepts schema-validated 2-4 step plans. Sequence planning performs validation and permission preflight without execution; a dedicated whole-plan confirmation boundary remains required before multi-action execution.

## Selected runtime and model

NOVA uses the loopback-only Ollama API with `qwen3:1.7b` for commands that are not covered by its deterministic offline classifier.

- Runtime: Ollama, fixed to `127.0.0.1:11434`
- Model: `qwen3:1.7b`
- Published model artifact: 1.4 GB, Q4_K_M quantization, 2.03B parameters
- Estimated available RAM: approximately 2.5-3.5 GB, depending on Ollama/runtime overhead and context
- Expected latency: approximately 0.5-3 seconds warm or 2-10 seconds cold on a modern CPU; actual latency is hardware dependent
- Network behavior: NOVA does not download a model, contact a hosted inference API, or accept a non-loopback model endpoint

The model is small enough for classification and argument extraction while supporting tool-oriented prompts and structured output. It is not used as an executor.

Official references:

- https://ollama.com/library/qwen3:1.7b
- https://docs.ollama.com/capabilities/structured-outputs
- https://docs.ollama.com/api/chat

## Security boundary

The execution path is:

`voice or typed text -> deterministic classifier or local Ollama -> JSON output -> native typed request parser -> native permission engine -> deterministic native tool implementation`

The model allowlist contains only implemented read/action tools for applications, approved file/folder access, safe system controls, screenshots, and window management. The prompt forbids shell, PowerShell, cmd, scripts, code, and raw filesystem operations. More importantly, model output is treated as untrusted regardless of its prompt:

1. Ollama receives a strict JSON schema with a closed tool enum.
2. NOVA deserializes with unknown top-level fields denied.
3. Confidence must be finite and between 0 and 1; values below 0.65 require clarification.
4. The native Tool Router validates the exact argument structure and constraints for the selected tool.
5. Saved NOVA enabled state and skill permissions are checked.
6. Confirmation-required actions remain pending until explicit UI confirmation.
7. Only the existing typed native implementation executes an approved request.

No model-produced string is passed to PowerShell, cmd.exe, a shell, a script runtime, or a generic process launcher.

## Offline behavior

The required common commands have a deterministic local classifier, including application launch aliases, opening approved standard folders, setting/querying volume, taking a screenshot, listing running applications, and basic system queries. Existing manual development panels remain operational without Ollama.

If Ollama is unavailable, NOVA performs one loopback connection attempt and returns `modelUnavailable`; there is no retry loop. If Ollama is running but `qwen3:1.7b` is absent, About reports `Not loaded`. Invalid or malformed model output is rejected without tool execution.

At the end of Phase 14 development on this workstation, the Ollama CLI was not installed. NOVA therefore truthfully reports `Error` for the local-model runtime while its deterministic/manual fallback remains available. To activate the optional classifier later, install Ollama locally and run:

```text
ollama pull qwen3:1.7b
```

NOVA will detect the model through Ollama's local `/api/tags` endpoint; no NOVA configuration change is required.

## Ambiguity and history

`Open my project` is resolved only when exactly one known project exists. With multiple known projects, NOVA returns their names as clarification candidates; with none, it asks the user to register one. It never selects a candidate randomly.

Completed, denied, rejected, and failed routed commands are recorded in the existing local command-history store. A request waiting for confirmation is logged by the confirmed execution command, preventing a pending request from being recorded as executed.
