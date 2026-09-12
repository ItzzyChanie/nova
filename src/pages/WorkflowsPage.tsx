import { useEffect, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import {
  loadWorkflows,
  saveWorkflow,
  deleteWorkflow,
  executeRegistered,
  type WorkflowRecord,
} from "../services/projects";
const tools: Record<
  string,
  {
    label: string;
    field: string;
    hint: string;
  }
> = {
  "application.open": {
    label: "Open application",
    field: "application",
    hint: "Visual Studio Code",
  },
  "application.focus": {
    label: "Focus application",
    field: "application",
    hint: "Google Chrome",
  },
  "developer.open_project": {
    label: "Open project",
    field: "name",
    hint: "ProctorX",
  },
  "developer.start_project": {
    label: "Start project",
    field: "name",
    hint: "ProctorX",
  },
  "developer.stop_project": {
    label: "Stop project",
    field: "name",
    hint: "ProctorX",
  },
  "developer.open_dev_url": {
    label: "Open development URL",
    field: "name",
    hint: "ProctorX",
  },
  "browser.open_url": {
    label: "Open website",
    field: "url",
    hint: "https://github.com",
  },
  "folder.open": {
    label: "Open approved folder",
    field: "folder",
    hint: "Downloads",
  },
  "system.set_volume": { label: "Set volume", field: "volume", hint: "25" },
  "system.mute": { label: "Mute", field: "", hint: "" },
  "system.unmute": { label: "Unmute", field: "", hint: "" },
  "window.focus": {
    label: "Focus window",
    field: "title",
    hint: "Visual Studio Code",
  },
};
const empty = (): WorkflowRecord => ({
  id: crypto.randomUUID(),
  name: "",
  enabled: true,
  voiceEnabled: false,
  steps: [],
});
export function WorkflowsPage() {
  const [records, setRecords] = useState<WorkflowRecord[]>([]);
  const [draft, setDraft] = useState<WorkflowRecord | null>(null);
  const [approved, setApproved] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    let active = true;
    void loadWorkflows()
      .then((w) => {
        if (active) setRecords(w);
      })
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);
  async function act(operation: () => Promise<void>) {
    if (busy) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await operation();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  function change(value: WorkflowRecord) {
    setDraft(value);
    setApproved(false);
  }
  function move(index: number, offset: number) {
    if (!draft) return;
    const steps = [...draft.steps];
    [steps[index], steps[index + offset]] = [
      steps[index + offset],
      steps[index],
    ];
    change({ ...draft, steps });
  }
  return (
    <>
      <PageHeading
        title="Workflows"
        description="Build repeatable routines from approved NOVA actions."
      />
      <p className="supporting-note">
        Steps run in order and stop on the first failure. Enable each required
        skill first. Website steps also require Network access for external
        sites.
      </p>
      <div className="application-actions">
        <button
          className="button"
          disabled={busy}
          onClick={() => change(empty())}
        >
          New workflow
        </button>
      </div>
      {error && (
        <p role="alert" className="microphone-error">
          {error}
        </p>
      )}
      {message && (
        <p role="status" className="mint">
          {message}
        </p>
      )}
      {loading && <p role="status">Loading workflows...</p>}
      {!loading && !records.length && (
        <section className="panel profile-panel">
          <h2>No workflows yet</h2>
          <p>
            Create Coding Mode or School Mode using the actions you approve.
          </p>
        </section>
      )}
      {draft && (
        <form
          className="panel profile-panel"
          onSubmit={(e) => {
            e.preventDefault();
            if (approved)
              void act(async () => {
                setRecords(await saveWorkflow(draft));
                setDraft(null);
                setMessage("Workflow saved locally.");
              });
          }}
        >
          <h2>Workflow editor</h2>
          <fieldset disabled={busy} className="profile-fields">
            <label className="field-label">
              Name
              <input
                required
                maxLength={80}
                value={draft.name}
                onChange={(e) => change({ ...draft, name: e.target.value })}
              />
            </label>
            <label className="check-label">
              <input
                type="checkbox"
                checked={draft.enabled}
                onChange={(e) =>
                  change({ ...draft, enabled: e.target.checked })
                }
              />
              Enabled
            </label>
            <label className="check-label">
              <input
                type="checkbox"
                checked={draft.voiceEnabled}
                onChange={(e) =>
                  change({ ...draft, voiceEnabled: e.target.checked })
                }
              />
              Allow voice execution of these saved steps
            </label>
            <ol className="workflow-steps">
              {draft.steps.map((step, index) => {
                const schema = tools[step.tool];
                return (
                  <li key={index}>
                    <label className="field-label">
                      Step {index + 1}
                      <select
                        aria-label={`Step ${index + 1} action`}
                        value={step.tool}
                        onChange={(e) =>
                          change({
                            ...draft,
                            steps: draft.steps.map((s, i) =>
                              i === index
                                ? { tool: e.target.value, arguments: {} }
                                : s,
                            ),
                          })
                        }
                      >
                        {!schema && (
                          <option value={step.tool}>{step.tool}</option>
                        )}
                        {Object.entries(tools).map(([tool, s]) => (
                          <option key={tool} value={tool}>
                            {s.label}
                          </option>
                        ))}
                      </select>
                    </label>
                    {schema?.field && (
                      <label className="field-label">
                        {schema.field}
                        <input
                          required
                          type={schema.field === "volume" ? "number" : "text"}
                          min={schema.field === "volume" ? 0 : undefined}
                          max={schema.field === "volume" ? 100 : undefined}
                          placeholder={schema.hint}
                          value={String(step.arguments[schema.field] ?? "")}
                          onChange={(e) =>
                            change({
                              ...draft,
                              steps: draft.steps.map((s, i) =>
                                i === index
                                  ? {
                                      ...s,
                                      arguments: {
                                        [schema.field]:
                                          schema.field === "volume"
                                            ? Number(e.target.value)
                                            : e.target.value,
                                      },
                                    }
                                  : s,
                              ),
                            })
                          }
                        />
                      </label>
                    )}
                    <div className="application-actions">
                      <button
                        type="button"
                        className="button"
                        disabled={index === 0}
                        onClick={() => move(index, -1)}
                        aria-label={`Move step ${index + 1} up`}
                      >
                        Up
                      </button>
                      <button
                        type="button"
                        className="button"
                        disabled={index === draft.steps.length - 1}
                        onClick={() => move(index, 1)}
                        aria-label={`Move step ${index + 1} down`}
                      >
                        Down
                      </button>
                      <button
                        type="button"
                        className="button button-danger"
                        onClick={() =>
                          change({
                            ...draft,
                            steps: draft.steps.filter((_, i) => i !== index),
                          })
                        }
                      >
                        Remove
                      </button>
                    </div>
                  </li>
                );
              })}
            </ol>
            <button
              type="button"
              className="button"
              disabled={draft.steps.length >= 20}
              onClick={() =>
                change({
                  ...draft,
                  steps: [
                    ...draft.steps,
                    {
                      tool: "application.open",
                      arguments: { application: "" },
                    },
                  ],
                })
              }
            >
              Add step
            </button>
            <label className="check-label">
              <input
                type="checkbox"
                checked={approved}
                onChange={(e) => setApproved(e.target.checked)}
              />
              I approve these ordered actions.{" "}
              {draft.voiceEnabled &&
                "A spoken request may execute this workflow without another prompt."}
            </label>
            <div className="application-actions">
              <button
                className="button"
                disabled={!approved || !draft.steps.length}
              >
                {busy ? "Saving..." : "Save approved workflow"}
              </button>
              <button
                type="button"
                className="button"
                onClick={() => setDraft(null)}
              >
                Cancel
              </button>
            </div>
          </fieldset>
        </form>
      )}
      <div className="profile-list">
        {records.map((w) => (
          <section key={w.id} className="panel profile-panel">
            <div className="profile-heading">
              <h2>{w.name}</h2>
              <span className={w.enabled ? "mint" : "muted"}>
                {w.enabled ? "Enabled" : "Disabled"}
              </span>
            </div>
            <p className="supporting-note">
              {w.steps.length} steps ? Voice{" "}
              {w.voiceEnabled ? "approved" : "not approved"}
            </p>
            <ol>
              {w.steps.map((s, i) => (
                <li key={i} className="technical wrap-text">
                  {tools[s.tool]?.label ?? s.tool}{" "}
                  {Object.values(s.arguments).map(String).join(", ")}
                </li>
              ))}
            </ol>
            <div className="application-actions">
              <button
                className="button"
                disabled={busy || !w.enabled}
                onClick={() =>
                  void act(async () => {
                    const r = await executeRegistered("workflow.run", w.id);
                    if (r.status !== "completed")
                      throw new Error(r.error?.message ?? "Workflow failed");
                    setMessage(
                      (
                        r.data as {
                          message?: string;
                        }
                      )?.message ?? "Workflow completed.",
                    );
                  })
                }
              >
                {busy ? "Working..." : "Run workflow"}
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  change({
                    ...w,
                    steps: w.steps.map((s) => ({
                      ...s,
                      arguments: { ...s.arguments },
                    })),
                  })
                }
              >
                Edit / rename
              </button>
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  void act(async () =>
                    setRecords(
                      await saveWorkflow({ ...w, enabled: !w.enabled }),
                    ),
                  )
                }
              >
                {w.enabled ? "Disable" : "Enable"}
              </button>
              <button
                className="button button-danger"
                disabled={busy}
                onClick={() => {
                  if (window.confirm(`Delete workflow ${w.name}?`))
                    void act(async () =>
                      setRecords(await deleteWorkflow(w.id)),
                    );
                }}
              >
                Delete
              </button>
            </div>
          </section>
        ))}
      </div>
    </>
  );
}
