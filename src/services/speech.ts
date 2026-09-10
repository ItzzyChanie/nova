import { invoke, isTauri } from "@tauri-apps/api/core";
import type { SpeechEngineInfo } from "../types/speech";

const browserFallback: SpeechEngineInfo = {
  engine: "Whisper Tiny English (sherpa-onnx 1.13.7)",
  model: "Whisper Tiny English INT8",
  modelPath: "Desktop app resource",
  modelAvailable: false,
  language: "English",
  sampleRate: 16_000,
};

export async function getSpeechEngineInfo(): Promise<SpeechEngineInfo> {
  if (!isTauri()) return browserFallback;
  return invoke<SpeechEngineInfo>("get_speech_engine_info");
}

export async function setSpeechModelPath(
  modelPath: string | null,
): Promise<SpeechEngineInfo> {
  if (!isTauri())
    throw new Error(
      "Speech-model configuration is available in the NOVA desktop app.",
    );
  return invoke<SpeechEngineInfo>("set_speech_model_path", { modelPath });
}
