import { useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import { NaturalCommandPanel } from "../components/NaturalCommandPanel";
import { executeConfirmedApplicationTool } from "../services/toolRouter";
import type {
  ApplicationToolName,
  ApplicationToolRequest,
  ToolResult,
} from "../types/tools";

const knownApplications = [
  "Visual Studio Code",
  "Google Chrome",
  "Discord",
  "Spotify",
  "File Explorer",
  "Notepad",
  "Docker Desktop",
  "Microsoft Word",
  "Visual Studio 2026",
  "Capcut",
  "PowerPoint",
  "Cursor",
  "Android Studio",
  "Microsoft Edge",
] as const;

function requestFor(
  tool: ApplicationToolName,
  application: string,
): ApplicationToolRequest {
  const requestId = crypto.randomUUID();
  if (tool === "application.list_running") {
    return { requestId, tool, arguments: {} };
  }
  return { requestId, tool, arguments: { application } };
}

export function DeveloperToolsPage() {
  const [application, setApplication] = useState<string>(knownApplications[0]);
  const [running, setRunning] = useState<ApplicationToolName | null>(null);
  const [result, setResult] = useState<ToolResult | null>(null);

  async function execute(tool: ApplicationToolName) {
    if (running) return;
    if (
      tool === "application.close" &&
      !window.confirm(`Close ${application}? Unsaved work may be lost.`)
    ) {
      return;
    }

    setRunning(tool);
    setResult(null);
    try {
      setResult(
        await executeConfirmedApplicationTool(requestFor(tool, application)),
      );
    } catch (error: unknown) {
      setResult({
        tool,
        status: "rejected",
        error: {
          code: "applicationLaunchFailed",
          message: error instanceof Error ? error.message : String(error),
        },
      });
    } finally {
      setRunning(null);
    }
  }

  return (
    <>
      <PageHeading
        title="Application tools"
        description="Open apps and run approved actions."
        preview="Development panel with native permission checks."
      />
      <NaturalCommandPanel />
      <section
        className="panel application-test-panel"
        aria-labelledby="application-test-title"
      >
        <div>
          <h2 id="application-test-title">Known application</h2>
          <p>
            Only registry entries listed here can be resolved or controlled.
          </p>
        </div>
        <label className="field-label">
          Application
          <select
            value={application}
            disabled={running !== null}
            onChange={(event) => setApplication(event.target.value)}
          >
            {knownApplications.map((name) => (
              <option key={name}>{name}</option>
            ))}
          </select>
        </label>
        <div className="application-actions">
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute("application.open")}
          >
            Open
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute("application.focus")}
          >
            Focus
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute("application.is_running")}
          >
            Is running?
          </button>
          <button
            className="button button-danger"
            type="button"
            disabled={running !== null}
            onClick={() => void execute("application.close")}
          >
            Close
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute("application.list_running")}
          >
            List running
          </button>
        </div>
        {running && (
          <p className="supporting-note" role="status">
            Running {running}...
          </p>
        )}
        {result && (
          <div className="tool-result" aria-live="polite">
            <p className="technical">
              {result.tool ?? "application tool"} · {result.status}
            </p>
            <p>
              {result.error?.message ??
                (result.data as { message?: string } | undefined)?.message ??
                "Request completed."}
            </p>
            {result.data !== undefined && (
              <pre>{JSON.stringify(result.data, null, 2)}</pre>
            )}
          </div>
        )}
      </section>
    </>
  );
}
