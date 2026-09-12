import { useEffect, useMemo, useState } from "react";
import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import {
  getMicrophoneTestState,
  getWakeEngineStatus,
  listMicrophoneDevices,
  setSelectedMicrophone,
  startMicrophoneTest,
  stopMicrophoneTest,
} from "../services/audio";
import { getSpeechEngineInfo, setSpeechModelPath } from "../services/speech";
import type { SpeechEngineInfo } from "../types/speech";
import type {
  AudioFormat,
  MicrophoneDevice,
  MicrophoneStatus,
  MicrophoneTestSnapshot,
  WakeEngineSnapshot,
} from "../types/audio";
import {
  saveAssistantPaused,
  saveVoiceReply,
  saveWakeSensitivity,
} from "../services/settingsStore";
import {
  sensitivities,
  type SettingsPageProps,
  type Sensitivity,
} from "../types/settings";

const DEFAULT_FORMAT: AudioFormat = {
  channels: 1,
  sampleRate: 16_000,
  sampleType: "f32 PCM",
};

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function statusLabel(status: MicrophoneStatus): string {
  switch (status) {
    case "listening":
      return "Listening";
    case "disconnected":
      return "Disconnected";
    case "permissionDenied":
      return "Permission denied";
    case "unavailable":
      return "Unavailable";
    case "error":
      return "Error";
    default:
      return "Ready";
  }
}

export function VoiceWakeWordPage({
  wakePhrase,
  settings,
  onChange,
}: SettingsPageProps & { wakePhrase: string }) {
  const [devices, setDevices] = useState<MicrophoneDevice[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState("");
  const [format, setFormat] = useState<AudioFormat>(DEFAULT_FORMAT);
  const [snapshot, setSnapshot] = useState<MicrophoneTestSnapshot | null>(null);
  const [wakeStatus, setWakeStatus] = useState<WakeEngineSnapshot | null>(null);
  const [speechInfo, setSpeechInfo] = useState<SpeechEngineInfo | null>(null);
  const [speechModelInput, setSpeechModelInput] = useState("");
  const [modelSaving, setModelSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [voiceSaving, setVoiceSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedDevice = useMemo(
    () => devices.find((device) => device.id === selectedDeviceId) ?? null,
    [devices, selectedDeviceId],
  );

  async function refreshWakeStatus() {
    try {
      setWakeStatus(await getWakeEngineStatus());
    } catch (statusError: unknown) {
      console.error("[NOVA] Could not read wake-engine status.", statusError);
      setWakeStatus({
        engine: "sherpa-onnx Zipformer KWS",
        phrase: "NOVA or Hey NOVA",
        status: "error",
        message: errorMessage(statusError),
        deviceName: null,
        inputLevel: 0,
        noiseFloor: 0,
        sensitivity: settings.sensitivity,
        threshold:
          settings.sensitivity === "Low"
            ? 0.35
            : settings.sensitivity === "High"
              ? 0.18
              : 0.25,
        lastDetectedPhrase: null,
        lastWakeConfidence: null,
        lastDetectionMillis: null,
      });
    }
  }

  async function refreshSpeechInfo() {
    try {
      const info = await getSpeechEngineInfo();
      setSpeechInfo(info);
      setSpeechModelInput(info.modelPath);
    } catch (speechError: unknown) {
      console.error(
        "[NOVA] Could not read speech-engine information.",
        speechError,
      );
      setError(errorMessage(speechError));
    }
  }
  async function refreshDevices() {
    setLoading(true);
    setError(null);
    try {
      const response = await listMicrophoneDevices();
      setDevices(response.devices);
      setFormat(response.internalFormat);

      const savedExists = response.devices.some(
        (device) => device.id === response.selectedDeviceId,
      );
      const fallback =
        response.devices.find((device) => device.isDefault) ??
        response.devices[0];
      setSelectedDeviceId(
        savedExists ? (response.selectedDeviceId ?? "") : (fallback?.id ?? ""),
      );

      if (response.selectedDeviceId && !savedExists) {
        setError(
          "The saved microphone is disconnected. Choose an available device.",
        );
      } else if (response.devices.length === 0) {
        setError("No microphone input devices are available.");
      }
    } catch (loadError: unknown) {
      console.error(
        "[NOVA] Could not enumerate microphone devices.",
        loadError,
      );
      setDevices([]);
      setSelectedDeviceId("");
      setError(errorMessage(loadError));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshDevices();
    void refreshWakeStatus();
    void refreshSpeechInfo();
    const statusTimer = window.setInterval(() => { if (!document.hidden) void refreshWakeStatus(); }, 500);
    return () => {
      window.clearInterval(statusTimer);
      void stopMicrophoneTest().catch((stopError: unknown) => {
        console.error("[NOVA] Could not stop the microphone test.", stopError);
      });
    };
  }, []);

  useEffect(() => {
    if (!testing) return;

    const timer = window.setInterval(() => {
      void getMicrophoneTestState()
        .then((state) => {
          setSnapshot(state);
          if (state.status !== "listening") setTesting(false);
        })
        .catch((stateError: unknown) => {
          console.error(
            "[NOVA] Could not read the microphone level.",
            stateError,
          );
          setTesting(false);
          setError(errorMessage(stateError));
        });
    }, 100);

    return () => window.clearInterval(timer);
  }, [testing]);

  async function chooseDevice(deviceId: string) {
    if (!deviceId || saving) return;
    setSaving(true);
    setError(null);
    try {
      if (testing) {
        await stopMicrophoneTest();
        setTesting(false);
        setSnapshot(null);
      }
      const persisted = await setSelectedMicrophone(deviceId);
      setSelectedDeviceId(persisted);
      await refreshWakeStatus();
    } catch (saveError: unknown) {
      console.error(
        "[NOVA] Could not save the selected microphone.",
        saveError,
      );
      setError(errorMessage(saveError));
      await refreshDevices();
    } finally {
      setSaving(false);
    }
  }

  async function updateSensitivity(sensitivity: Sensitivity) {
    if (voiceSaving) return;
    setVoiceSaving(true);
    setError(null);
    try {
      const saved = await saveWakeSensitivity(sensitivity);
      onChange({
        sensitivity: saved.wakeSensitivity,
        assistantPaused: saved.assistantPaused,
      });
      await refreshWakeStatus();
    } catch (saveError: unknown) {
      console.error("[NOVA] Could not save wake sensitivity.", saveError);
      setError(errorMessage(saveError));
    } finally {
      setVoiceSaving(false);
    }
  }

  async function updateVoiceReply(enabled: boolean) {
    if (voiceSaving) return;
    setVoiceSaving(true);
    setError(null);
    try {
      onChange({ voiceReply: await saveVoiceReply(enabled) });
    } catch (saveError: unknown) {
      setError(errorMessage(saveError));
    } finally {
      setVoiceSaving(false);
    }
  }

  async function updatePaused(paused: boolean) {
    if (voiceSaving) return;
    setVoiceSaving(true);
    setError(null);
    try {
      const saved = await saveAssistantPaused(paused);
      onChange({
        sensitivity: saved.wakeSensitivity,
        assistantPaused: saved.assistantPaused,
      });
      await refreshWakeStatus();
    } catch (saveError: unknown) {
      console.error("[NOVA] Could not update Pause Assistant.", saveError);
      setError(errorMessage(saveError));
    } finally {
      setVoiceSaving(false);
    }
  }

  async function saveSpeechModel(modelPath: string | null) {
    if (modelSaving) return;
    setModelSaving(true);
    setError(null);
    try {
      const info = await setSpeechModelPath(modelPath);
      setSpeechInfo(info);
      setSpeechModelInput(info.modelPath);
      await refreshWakeStatus();
    } catch (modelError: unknown) {
      console.error(
        "[NOVA] Could not save the speech-model directory.",
        modelError,
      );
      setError(errorMessage(modelError));
    } finally {
      setModelSaving(false);
    }
  }
  async function toggleTest() {
    if (testing) {
      try {
        const stopped = await stopMicrophoneTest();
        setSnapshot(stopped);
        await refreshWakeStatus();
      } catch (stopError: unknown) {
        console.error("[NOVA] Could not stop the microphone test.", stopError);
        setError(errorMessage(stopError));
      } finally {
        setTesting(false);
      }
      return;
    }

    if (!selectedDeviceId) return;
    setError(null);
    try {
      const started = await startMicrophoneTest(selectedDeviceId);
      setSnapshot(started);
      setTesting(true);
      await refreshWakeStatus();
    } catch (startError: unknown) {
      console.error("[NOVA] Could not start the microphone test.", startError);
      setError(errorMessage(startError));
      try {
        setSnapshot(await getMicrophoneTestState());
      } catch {
        // The original start error is the useful error to retain.
      }
    }
  }

  const level = Math.round((snapshot?.level ?? 0) * 100);
  const status =
    snapshot?.status ?? (devices.length > 0 ? "idle" : "unavailable");

  return (
    <>
      <PageHeading
        title="Voice & wake word"
        description="Make NOVA sound and listen your way."
        preview="Wake detection, command capture, and Whisper transcription run entirely on this device."
      />
      <div className="card-grid">
        <Card title="Wake phrase">
          <p className="technical value">&ldquo;{wakePhrase}&rdquo;</p>
        </Card>

        <Card
          title="Wake engine"
          description="Offline open-vocabulary keyword spotting on one CPU thread."
        >
          <p className="technical value">
            {wakeStatus?.engine ?? "sherpa-onnx Zipformer KWS"}
          </p>
          <div className="wake-engine-status">
            <span>Status</span>
            <strong
              className={
                wakeStatus?.status === "listening"
                  ? "mint"
                  : wakeStatus?.status === "error"
                    ? "microphone-error"
                    : ""
              }
            >
              {wakeStatus?.status === "listening"
                ? "Listening"
                : wakeStatus?.status === "paused"
                  ? "Paused"
                  : wakeStatus?.status === "error"
                    ? "Error"
                    : "Disabled"}
            </strong>
          </div>
          <p className="supporting-note">
            {wakeStatus?.message ?? "Reading wake-engine status..."}
          </p>
          {wakeStatus?.deviceName && (
            <p className="supporting-note technical">{wakeStatus.deviceName}</p>
          )}
          <div
            className="level-meter"
            role="meter"
            aria-label="Background wake listener input level"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round((wakeStatus?.inputLevel ?? 0) * 100)}
          >
            <span
              style={{
                width: `${Math.round((wakeStatus?.inputLevel ?? 0) * 100)}%`,
              }}
            />
          </div>
          <p className="supporting-note">
            Speak near the selected microphone. Meter movement confirms the
            background listener is receiving audio.
          </p>
          <dl className="diagnostic-list">
            <div>
              <dt>Sensitivity</dt>
              <dd>
                {wakeStatus?.sensitivity ?? settings.sensitivity} (threshold{" "}
                {wakeStatus?.threshold?.toFixed(2) ?? "—"})
              </dd>
            </div>
            <div>
              <dt>Noise floor</dt>
              <dd>
                {Math.round((wakeStatus?.noiseFloor ?? 0) * 1000) / 10}% RMS
              </dd>
            </div>
            <div>
              <dt>Last phrase</dt>
              <dd>{wakeStatus?.lastDetectedPhrase ?? "None yet"}</dd>
            </div>
            <div>
              <dt>Last wake</dt>
              <dd>
                {wakeStatus?.lastDetectionMillis
                  ? new Date(
                      wakeStatus.lastDetectionMillis,
                    ).toLocaleTimeString()
                  : "None yet"}
              </dd>
            </div>
            <div>
              <dt>Confidence</dt>
              <dd>
                {wakeStatus?.lastWakeConfidence == null
                  ? "Not exposed by engine"
                  : wakeStatus.lastWakeConfidence.toFixed(2)}
              </dd>
            </div>
          </dl>
        </Card>

        <Card
          title="Speech engine"
          description="Offline Whisper transcription. The model loads on demand after wake detection."
        >
          <p className="technical value">
            {speechInfo?.engine ?? "Reading local engine..."}
          </p>
          <div className="wake-engine-status">
            <span>Model</span>
            <strong
              className={
                speechInfo?.modelAvailable ? "mint" : "microphone-error"
              }
            >
              {speechInfo?.modelAvailable ? "Available" : "Missing"}
            </strong>
          </div>
          <p className="supporting-note">
            {speechInfo?.model ?? "Whisper Tiny English INT8"} ·{" "}
            {speechInfo?.language ?? "English"} · 16 kHz mono
          </p>
          <label className="field-label" htmlFor="speech-model-directory">
            Model directory
            <input
              id="speech-model-directory"
              className="technical"
              value={speechModelInput}
              disabled={modelSaving}
              spellCheck={false}
              onChange={(event) =>
                setSpeechModelInput(event.currentTarget.value)
              }
            />
          </label>
          <div className="microphone-actions">
            <button
              type="button"
              className="button"
              disabled={modelSaving || !speechModelInput.trim()}
              onClick={() => void saveSpeechModel(speechModelInput)}
            >
              Save model directory
            </button>
            <button
              type="button"
              className="button"
              disabled={modelSaving}
              onClick={() => void saveSpeechModel(null)}
            >
              Use bundled model
            </button>
          </div>
          <p className="supporting-note">
            The bundled Tiny English model is the fast, low-memory option. For
            better accented-English accuracy or Taglish, select a compatible
            multilingual Whisper export with encoder, decoder, and tokens files;
            multilingual models use local language auto-detection.
          </p>
        </Card>
        <Card
          title="Pause assistant"
          description="Temporarily stop background wake detection without turning NOVA off."
          control={
            <Toggle
              label="Pause Assistant"
              checked={settings.assistantPaused}
              disabled={voiceSaving}
              onChange={(paused) => void updatePaused(paused)}
            />
          }
        />

        <Card
          title="Microphone"
          description="Real input devices reported by Windows. Selection is saved locally."
        >
          <label className="field-label" htmlFor="microphone-device">
            Input device
            <select
              id="microphone-device"
              value={selectedDeviceId}
              disabled={loading || saving || devices.length === 0}
              onChange={(event) => void chooseDevice(event.currentTarget.value)}
            >
              {devices.length === 0 && (
                <option value="">
                  {loading
                    ? "Detecting microphones..."
                    : "No microphones found"}
                </option>
              )}
              {devices.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.name}
                  {device.isDefault ? " (Default)" : ""}
                </option>
              ))}
            </select>
          </label>
          <div className="microphone-actions">
            <button
              type="button"
              className="button"
              disabled={loading || saving}
              onClick={() => void refreshDevices()}
            >
              Refresh devices
            </button>
            <button
              type="button"
              className="button"
              disabled={!selectedDevice || saving}
              onClick={() => void toggleTest()}
            >
              {testing ? "Stop test" : "Test microphone"}
            </button>
          </div>
          {error && (
            <p className="assistant-launch-error" role="alert">
              {error}
            </p>
          )}
        </Card>

        <Card
          title="Microphone test"
          description="The meter uses transient mono samples and stores no recording."
        >
          <div className="microphone-status">
            <span
              className={
                status === "listening"
                  ? "mint"
                  : status === "idle"
                    ? "muted"
                    : "microphone-error"
              }
            >
              {statusLabel(status)}
            </span>
            <span className="technical">{level}%</span>
          </div>
          <div
            className="level-meter"
            role="meter"
            aria-label="Live microphone input level"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={level}
          >
            <span style={{ width: `${level}%` }} />
          </div>
          <p className="supporting-note">
            {snapshot?.message ?? "Start the test to view a live input level."}
          </p>
        </Card>

        <Card
          title="Internal audio format"
          description="Conditioned for local wake-word and speech processing."
        >
          <p className="technical value">
            {format.channels === 1 ? "Mono" : `${format.channels} channels`} /{" "}
            {format.sampleRate / 1000} kHz / {format.sampleType}
          </p>
          <p className="supporting-note">
            Wake and command audio remains in memory only and is discarded after
            detection or transcription.
          </p>
        </Card>

        <Card
          title="Sensitivity"
          description="Low (0.35) reduces false wakes, Medium (0.25) is balanced, and High (0.18) accepts weaker matches with a higher false-wake risk."
        >
          <label className="sr-only" htmlFor="sensitivity">
            Wake-word sensitivity
          </label>
          <select
            id="sensitivity"
            value={settings.sensitivity}
            disabled={voiceSaving}
            aria-describedby="preview-note"
            onChange={(event) => {
              const value = sensitivities.find(
                (option) => option === event.currentTarget.value,
              );
              if (value) void updateSensitivity(value);
            }}
          >
            {sensitivities.map((value) => (
              <option key={value}>{value}</option>
            ))}
          </select>
        </Card>

        <Card
          title="Voice reply"
          description="Speak short responses using an installed Windows desktop voice. Everything stays on this device. Wake NOVA to interrupt."
          control={
            <Toggle
              label="Voice reply"
              checked={settings.voiceReply}
              disabled={voiceSaving}
              onChange={(checked) => void updateVoiceReply(checked)}
            />
          }
        />
      </div>
    </>
  );
}
