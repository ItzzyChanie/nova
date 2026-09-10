import { useEffect, useState } from "react";
import {
  executeConfirmedNaturalRequest,
  getLocalModelInfo,
  interpretNaturalLanguage,
} from "../services/languageModel";
import type {
  LocalModelInfo,
  NaturalCommandResult,
} from "../types/languageModel";

const examples = [
  "Can you launch my code editor?",
  "Open my Downloads folder.",
  "Set my volume to thirty percent.",
  "Take a screenshot.",
  "What apps are currently running?",
] as const;

export function NaturalCommandPanel() {
  const [input, setInput] = useState<string>(examples[0]);
  const [model, setModel] = useState<LocalModelInfo | null>(null);
  const [result, setResult] = useState<NaturalCommandResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    void getLocalModelInfo()
      .then(setModel)
      .catch((error: unknown) => {
        setModel({
          runtime: "Ollama",
          model: "qwen3:1.7b",
          size: "1.4 GB (Q4_K_M)",
          ramRequirement: "Approximately 2.5-3.5 GB available RAM",
          expectedLatency: "Hardware dependent",
          status: "error",
          message: error instanceof Error ? error.message : String(error),
        });
      });
  }, []);

  async function interpret() {
    if (busy || !input.trim()) return;
    setBusy(true);
    setResult(null);
    try {
      setResult(await interpretNaturalLanguage(input.trim()));
    } catch (error: unknown) {
      setResult({
        status: "error",
        source: "deterministic",
        model: model?.model ?? "qwen3:1.7b",
        input,
        message: error instanceof Error ? error.message : String(error),
        candidates: [],
        latencyMillis: 0,
      });
    } finally {
      setBusy(false);
    }
  }

  async function confirm() {
    if (!result?.request || !("tool" in result.request) || confirming) return;
    setConfirming(true);
    try {
      const routing = await executeConfirmedNaturalRequest(result.request);
      setResult({
        ...result,
        routing,
        status: routing.status === "completed" ? "ready" : "rejected",
        message: routing.error?.message ?? "Confirmed tool request completed.",
      });
    } finally {
      setConfirming(false);
    }
  }

  const singleRequest =
    result?.request && "tool" in result.request ? result.request : null;
  const needsConfirmation =
    Boolean(singleRequest) &&
    result?.routing?.status === "confirmationRequired";

  return (
    <section
      className="panel application-test-panel"
      aria-labelledby="natural-command-title"
    >
      <div>
        <h2 id="natural-command-title">Natural command router</h2>
        <p>
          Language only selects a typed tool request. Native validation and
          permissions always run before execution.
        </p>
      </div>
      <p className="technical">
        {model
          ? `${model.runtime} · ${model.model} · ${model.status}`
          : "Checking local model..."}
      </p>
      {model && <p className="supporting-note">{model.message}</p>}
      <label className="field-label">
        Command
        <input
          value={input}
          disabled={busy || confirming}
          maxLength={500}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void interpret();
          }}
        />
      </label>
      <div className="application-actions">
        <button
          className="button"
          type="button"
          disabled={busy || confirming || !input.trim()}
          onClick={() => void interpret()}
        >
          {busy ? "Interpreting..." : "Interpret and route"}
        </button>
        {examples.map((example) => (
          <button
            className="button"
            type="button"
            key={example}
            disabled={busy || confirming}
            onClick={() => setInput(example)}
          >
            {example}
          </button>
        ))}
      </div>
      {result && (
        <div className="tool-result" aria-live="polite">
          <p className="technical">
            {result.source} · {result.status} · {result.latencyMillis} ms
            {result.confidence != null
              ? ` · ${Math.round(result.confidence * 100)}% confidence`
              : ""}
          </p>
          <p>{result.message}</p>
          <dl className="diagnostic-list">
            <div>
              <dt>Raw transcript</dt>
              <dd>{result.input}</dd>
            </div>
            <div>
              <dt>Normalized</dt>
              <dd>{result.normalizedInput ?? result.input}</dd>
            </div>
            <div>
              <dt>Interpretation</dt>
              <dd>{result.source}</dd>
            </div>
            <div>
              <dt>Tool plan</dt>
              <dd>
                {singleRequest?.tool ??
                  (result.request ? "tool sequence" : "None")}
              </dd>
            </div>
            <div>
              <dt>Execution</dt>
              <dd>{result.routing?.status ?? "Not executed"}</dd>
            </div>
            <div>
              <dt>Latency</dt>
              <dd>{result.latencyMillis} ms</dd>
            </div>
          </dl>
          {result.candidates.length > 0 && (
            <p>Candidates: {result.candidates.join(", ")}</p>
          )}
          {result.request && (
            <pre>{JSON.stringify(result.request, null, 2)}</pre>
          )}
          {result.routing?.data !== undefined && (
            <pre>{JSON.stringify(result.routing.data, null, 2)}</pre>
          )}
          {needsConfirmation && (
            <button
              className="button"
              type="button"
              disabled={confirming}
              onClick={() => void confirm()}
            >
              {confirming
                ? "Executing..."
                : `Confirm ${singleRequest?.tool ?? "tool"}`}
            </button>
          )}
        </div>
      )}
    </section>
  );
}
