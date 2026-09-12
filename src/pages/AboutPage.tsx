import { useEffect, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import { getRuntimeInfo, type RuntimeInfo } from "../services/privacy";
import { getWakeEngineStatus } from "../services/audio";
import { getLocalModelInfo } from "../services/languageModel";
import { getSpeechEngineInfo } from "../services/speech";
import type { LocalModelInfo } from "../types/languageModel";
import type { SpeechEngineInfo } from "../types/speech";

export function AboutPage({ version }: { version: string }) {
  const [runtime,setRuntime] = useState<RuntimeInfo|null>(null);
  const [error,setError] = useState("");
  const [speech, setSpeech] = useState<SpeechEngineInfo | null>(null);
  const [model, setModel] = useState<LocalModelInfo | null>(null);
  const [wakeEngine, setWakeEngine] = useState("Detecting...");

  useEffect(() => {
    let active = true;
    const failed = (value: unknown) => { if (active) setError(String(value)); };
    void getRuntimeInfo().then(value => { if (active) setRuntime(value); }).catch(failed);
    void getSpeechEngineInfo().then(value => { if (active) setSpeech(value); }).catch(failed);
    void getWakeEngineStatus().then(value => { if (active) setWakeEngine(value.engine); }).catch(value => { if (active) setWakeEngine("Unavailable"); failed(value); });
    void getLocalModelInfo().then(value => { if (active) setModel(value); }).catch(failed);
    return () => { active = false; };
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
        {error && <p role="alert" className="microphone-error">{error}</p>}
        <dl className="metadata">
          <div>
            <dt>Version</dt>
            <dd className="mint">{runtime?.version ?? version}</dd>
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
          <div><dt>Platform</dt><dd>{runtime ? `${runtime.platform} / ${runtime.architecture}` : 'Detecting...'}</dd></div>
          <div><dt>Voice reply engine</dt><dd>{runtime?.tts ?? 'Detecting...'}</dd></div>
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
