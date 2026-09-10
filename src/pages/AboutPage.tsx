import { useEffect, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import { getWakeEngineStatus } from "../services/audio";
import { getLocalModelInfo } from "../services/languageModel";
import { getSpeechEngineInfo } from "../services/speech";
import type { LocalModelInfo } from "../types/languageModel";
import type { SpeechEngineInfo } from "../types/speech";

export function AboutPage({ version }: { version: string }) {
  const [speech, setSpeech] = useState<SpeechEngineInfo | null>(null);
  const [model, setModel] = useState<LocalModelInfo | null>(null);
  const [wakeEngine, setWakeEngine] = useState("sherpa-onnx Zipformer KWS");

  useEffect(() => {
    void getSpeechEngineInfo()
      .then(setSpeech)
      .catch((error: unknown) =>
        console.error(
          "[NOVA] Could not read speech-engine information.",
          error,
        ),
      );
    void getWakeEngineStatus()
      .then((status) => setWakeEngine(status.engine))
      .catch((error: unknown) =>
        console.error("[NOVA] Could not read wake-engine information.", error),
      );
    void getLocalModelInfo()
      .then(setModel)
      .catch((error: unknown) =>
        console.error("[NOVA] Could not read local-model information.", error),
      );
  }, []);

  const modelClass =
    model?.status === "loaded"
      ? "mint"
      : model?.status === "error"
        ? "microphone-error"
        : "muted";
  const modelStatus =
    model?.status === "loaded"
      ? "Loaded"
      : model?.status === "notLoaded"
        ? "Not loaded"
        : "Error";

  return (
    <>
      <PageHeading
        title="About NOVA"
        description="Your desktop assistant. Powered locally."
      />
      <section
        className="panel about-panel"
        aria-labelledby="technical-heading"
      >
        <h2 id="technical-heading">Technical information</h2>
        <dl className="metadata">
          <div>
            <dt>Version</dt>
            <dd className="mint">{version}</dd>
          </div>
          <div>
            <dt>Local model</dt>
            <dd className={modelClass}>
              {model
                ? `${model.runtime} · ${model.model} · ${modelStatus}`
                : "Detecting..."}
            </dd>
          </div>
          <div>
            <dt>Speech engine</dt>
            <dd
              className={speech?.modelAvailable ? "mint" : "microphone-error"}
            >
              {speech
                ? `${speech.engine} · ${speech.modelAvailable ? "Available" : "Model missing"}`
                : "Detecting..."}
            </dd>
          </div>
          <div>
            <dt>Wake-word engine</dt>
            <dd className="mint">{wakeEngine}</dd>
          </div>
        </dl>
        {model && (
          <p className="supporting-note">
            {model.message} Model size: {model.size}. Expected memory:{" "}
            {model.ramRequirement}. Expected latency: {model.expectedLatency}.
          </p>
        )}
        <p className="supporting-note">
          Wake-word detection, speech transcription, and optional language
          classification run locally. Every selected action still passes native
          schema validation and permissions.
        </p>
      </section>
    </>
  );
}
