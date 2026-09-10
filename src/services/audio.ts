import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  MicrophoneDeviceList,
  MicrophoneTestSnapshot,
  WakeEngineSnapshot,
} from "../types/audio";

const INTERNAL_FORMAT = {
  channels: 1,
  sampleRate: 16_000,
  sampleType: "f32 PCM",
} as const;

export async function listMicrophoneDevices(): Promise<MicrophoneDeviceList> {
  if (!isTauri()) {
    return {
      devices: [],
      selectedDeviceId: null,
      internalFormat: INTERNAL_FORMAT,
    };
  }
  return invoke<MicrophoneDeviceList>("list_microphone_devices");
}

export async function setSelectedMicrophone(deviceId: string): Promise<string> {
  if (!isTauri()) return deviceId;
  return invoke<string>("set_selected_microphone", {
    arguments: { deviceId },
  });
}

export async function startMicrophoneTest(
  deviceId: string,
): Promise<MicrophoneTestSnapshot> {
  if (!isTauri()) {
    throw new Error("Microphone testing is available in the NOVA desktop app.");
  }
  return invoke<MicrophoneTestSnapshot>("start_microphone_test", { deviceId });
}

export async function getMicrophoneTestState(): Promise<MicrophoneTestSnapshot> {
  if (!isTauri()) {
    return {
      status: "unavailable",
      level: 0,
      deviceId: null,
      deviceName: null,
      message: "Microphone testing is available in the NOVA desktop app.",
      internalFormat: INTERNAL_FORMAT,
    };
  }
  return invoke<MicrophoneTestSnapshot>("get_microphone_test_state");
}

export async function stopMicrophoneTest(): Promise<MicrophoneTestSnapshot> {
  if (!isTauri()) {
    return {
      status: "idle",
      level: 0,
      deviceId: null,
      deviceName: null,
      message: "Microphone test is stopped.",
      internalFormat: INTERNAL_FORMAT,
    };
  }
  return invoke<MicrophoneTestSnapshot>("stop_microphone_test");
}
export async function getWakeEngineStatus(): Promise<WakeEngineSnapshot> {
  if (!isTauri()) {
    return {
      engine: "sherpa-onnx Zipformer KWS",
      phrase: "NOVA or Hey NOVA",
      status: "disabled",
      message: "Wake detection is available in the NOVA desktop app.",
      deviceName: null,
      inputLevel: 0,
      noiseFloor: 0,
      sensitivity: "Medium",
      threshold: 0.25,
      lastDetectedPhrase: null,
      lastWakeConfidence: null,
      lastDetectionMillis: null,
    };
  }
  return invoke<WakeEngineSnapshot>("get_wake_engine_status");
}
