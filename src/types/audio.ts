export interface AudioFormat {
  channels: number;
  sampleRate: number;
  sampleType: string;
}

export interface MicrophoneDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface MicrophoneDeviceList {
  devices: MicrophoneDevice[];
  selectedDeviceId: string | null;
  internalFormat: AudioFormat;
}

export type MicrophoneStatus =
  | "idle"
  | "listening"
  | "disconnected"
  | "permissionDenied"
  | "unavailable"
  | "error";

export interface MicrophoneTestSnapshot {
  status: MicrophoneStatus;
  level: number;
  deviceId: string | null;
  deviceName: string | null;
  message: string;
  internalFormat: AudioFormat;
}
export type WakeEngineStatus = "listening" | "paused" | "disabled" | "error";

export interface WakeEngineSnapshot {
  engine: string;
  phrase: string;
  status: WakeEngineStatus;
  message: string;
  deviceName: string | null;
  inputLevel: number;
  noiseFloor: number;
  sensitivity: "Low" | "Medium" | "High";
  threshold: number;
  lastDetectedPhrase: string | null;
  lastWakeConfidence: number | null;
  lastDetectionMillis: number | null;
}
