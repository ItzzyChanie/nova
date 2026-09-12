import { useEffect, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import {
  loadKnownProjects,
  saveProjectProfile,
  deleteProjectProfile,
  loadProjectStatus,
  executeRegistered,
  type ProjectRecord,
} from "../services/projects";
const empty = (): ProjectRecord => ({
  id: crypto.randomUUID(),
  name: "",
  path: "",
  editor: "Visual Studio Code",
  frontendCommand: "",
  backendCommand: "",
  workingDirectory: "",
  developmentUrl: "",
  notes: "",
  voiceEnabled: false,
});
export function ProjectsPage() {
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [draft, setDraft] = useState<ProjectRecord | null>(null);
  const [approved, setApproved] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [status, setStatus] = useState<Record<string, string>>({});
  useEffect(() => {
    let active = true;
    void loadKnownProjects()
      .then((p) => {
        if (active) setProjects(p);
      })
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    const refresh = () => {
      if (!document.hidden)
        void loadProjectStatus()
          .then((s) => {
            if (active) setStatus(s);
          })
          .catch(() => {});
    };
    refresh();
    const timer = setInterval(refresh, 3000);
    return () => {
      active = false;
      clearInterval(timer);
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
  function edit(p: ProjectRecord) {
    setDraft({ ...empty(), ...p });
    setApproved(false);
  }
  function field(key: keyof ProjectRecord, value: string | boolean) {
    setDraft((d) => (d ? { ...d, [key]: value } : d));
    setApproved(false);
  }
  return (
    <>
      <PageHeading
        title="Projects"
        description="Your development environments, configured locally."
      />
      <p className="supporting-note">
        Enable Developer projects in Skills before running actions. Commands run
        with your Windows account; only approve projects and package scripts you
        trust.
      </p>
      <div className="application-actions">
        <button
          className="button"
          disabled={busy}
          onClick={() => edit(empty())}
        >
          New project
        </button>
      </div>
      {error && (
        <p className="microphone-error" role="alert">
          {error}
        </p>
      )}
      {message && (
        <p className="mint" role="status">
          {message}
        </p>
      )}
      {loading && <p role="status">Loading project profiles...</p>}
      {!loading && projects.length === 0 && (
        <section className="panel profile-panel">
          <h2>No projects yet</h2>
          <p>
            Add a folder, choose an editor, and approve the commands you want
            NOVA to run.
          </p>
        </section>
      )}
      {draft && (
        <form
          className="panel profile-panel"
          onSubmit={(e) => {
            e.preventDefault();
            if (!approved) return;
            void act(async () => {
              setProjects(await saveProjectProfile(draft));
              setDraft(null);
              setMessage("Project profile saved locally.");
            });
          }}
        >
          <h2>
            {projects.some((p) => p.id === draft.id)
              ? "Edit project"
              : "New project"}
          </h2>
          <fieldset disabled={busy} className="profile-fields">
            {(
              [
                ["name", "Name"],
                ["path", "Project path"],
                [
                  "workingDirectory",
                  "Working directory (blank uses project path)",
                ],
                ["frontendCommand", "Frontend command"],
                ["backendCommand", "Backend command (optional)"],
                ["developmentUrl", "Development URL (localhost)"],
              ] as const
            ).map(([key, label]) => (
              <label className="field-label" key={key}>
                {label}
                <input
                  required={key === "name" || key === "path"}
                  value={draft[key]}
                  onChange={(e) => field(key, e.target.value)}
                  spellCheck={false}
                />
              </label>
            ))}
            <label className="field-label">
              Editor
              <select
                value={draft.editor}
                onChange={(e) => field("editor", e.target.value)}
              >
                {[
                  "Visual Studio Code",
                  "Cursor",
                  "Visual Studio 2026",
                  "Android Studio",
                ].map((editor) => (
                  <option key={editor}>{editor}</option>
                ))}
              </select>
            </label>
            <label className="field-label">
              Notes
              <textarea
                maxLength={2000}
                value={draft.notes}
                onChange={(e) => field("notes", e.target.value)}
              />
            </label>
            <p className="supporting-note">
              Supported commands: <code>npm run &lt;script&gt;</code> and{" "}
              <code>dotnet run</code>. Both use the working directory. Shell
              operators and inline scripts are rejected. A package script may
              execute code and access the network independently of NOVA.
            </p>
            <label className="check-label">
              <input
                type="checkbox"
                checked={draft.voiceEnabled}
                onChange={(e) => field("voiceEnabled", e.target.checked)}
              />
              Allow voice execution of this saved project's actions
            </label>
            <label className="check-label">
              <input
                type="checkbox"
                checked={approved}
                onChange={(e) => setApproved(e.target.checked)}
              />
              I approve this folder, editor and these commands.{" "}
              {draft.voiceEnabled &&
                "Spoken commands may run these saved actions without another prompt."}
            </label>
            <div className="application-actions">
              <button className="button" disabled={!approved}>
                {busy ? "Saving..." : "Save approved profile"}
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
        {projects.map((p) => (
          <section className="panel profile-panel" key={p.id}>
            <div className="profile-heading">
              <h2>{p.name}</h2>
              <span className={status[p.id] === "running" ? "mint" : "muted"}>
                {status[p.id] ?? "Stopped"}
              </span>
            </div>
            <p className="technical wrap-text">{p.path}</p>
            <p className="supporting-note">
              {p.editor} &middot; Voice {p.voiceEnabled ? "approved" : "not approved"}
            </p>
            <p className="technical wrap-text">
              {[p.frontendCommand, p.backendCommand]
                .filter(Boolean)
                .join(" / ") || "No start commands configured"}
            </p>
            {p.notes && <p>{p.notes}</p>}
            <div className="application-actions">
              {(
                [
                  "open_project",
                  "start_project",
                  "stop_project",
                  "open_dev_url",
                ] as const
              ).map((action) => (
                <button
                  className="button"
                  key={action}
                  disabled={
                    busy || (action === "open_dev_url" && !p.developmentUrl)
                  }
                  onClick={() =>
                    void act(async () => {
                      const r = await executeRegistered(
                        `developer.${action}`,
                        p.id,
                      );
                      if (r.status !== "completed")
                        throw new Error(r.error?.message ?? "Action failed");
                      setMessage(
                        (
                          r.data as {
                            message?: string;
                          }
                        )?.message ?? "Done.",
                      );
                      setStatus(await loadProjectStatus());
                    })
                  }
                >
                  {
                    {
                      open_project: "Open editor",
                      start_project: "Start",
                      stop_project: "Stop",
                      open_dev_url: "Open URL",
                    }[action]
                  }
                </button>
              ))}
              <button
                className="button"
                disabled={busy}
                onClick={() => edit(p)}
              >
                Edit
              </button>
              <button
                className="button button-danger"
                disabled={busy}
                onClick={() => {
                  if (
                    window.confirm(
                      `Delete the ${p.name} profile? Project files will be kept.`,
                    )
                  )
                    void act(async () =>
                      setProjects(await deleteProjectProfile(p.id)),
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
