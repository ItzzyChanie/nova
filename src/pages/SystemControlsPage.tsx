import { useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import { executeConfirmedSystemTool } from "../services/toolRouter";
import type {
  SystemOrWindowToolName,
  SystemOrWindowToolRequest,
  ToolResult,
} from "../types/tools";

interface ListedWindow {
  windowId: string;
  title: string;
  processId: number;
}

interface WindowListData {
  windows?: ListedWindow[];
}

function requestFor(
  tool: SystemOrWindowToolName,
  argumentsValue: SystemOrWindowToolRequest["arguments"],
): SystemOrWindowToolRequest {
  return {
    requestId: crypto.randomUUID(),
    tool,
    arguments: argumentsValue,
  } as SystemOrWindowToolRequest;
}

function resultMessage(result: ToolResult): string {
  return (
    result.error?.message ??
    (result.data as { message?: string } | undefined)?.message ??
    "Request completed."
  );
}

export function SystemControlsPage() {
  const [volume, setVolume] = useState(50);
  const [running, setRunning] = useState<SystemOrWindowToolName | null>(null);
  const [result, setResult] = useState<ToolResult | null>(null);
  const [windows, setWindows] = useState<ListedWindow[]>([]);
  const [selectedWindowId, setSelectedWindowId] = useState("");

  async function execute(request: SystemOrWindowToolRequest) {
    if (running) return;
    setRunning(request.tool);
    setResult(null);
    try {
      const next = await executeConfirmedSystemTool(request);
      setResult(next);
      if (request.tool === "window.list" && next.status === "completed") {
        const listed = (
          (next.data as WindowListData | undefined)?.windows ?? []
        ).filter((window) => window.windowId && window.title);
        setWindows(listed);
        setSelectedWindowId((current) => current || listed[0]?.windowId || "");
      }
    } catch (error: unknown) {
      setResult({
        tool: request.tool,
        status: "rejected",
        error: {
          code: "systemInfoUnavailable",
          message: error instanceof Error ? error.message : String(error),
        },
      });
    } finally {
      setRunning(null);
    }
  }

  const focusedWindow = windows.find(
    (window) => window.windowId === selectedWindowId,
  );

  return (
    <>
      <PageHeading
        title="System tools"
        description="Quick controls for your desktop and windows."
        preview="Direct deterministic controls remain available when the local model is offline."
      />
      <section
        className="panel system-tool-panel"
        aria-labelledby="system-controls-title"
      >
        <div>
          <h2 id="system-controls-title">Device controls</h2>
          <p>
            Supported actions are limited to volume, local telemetry,
            screenshots, and visible-window focus.
          </p>
        </div>
        <div className="system-action-grid">
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute(requestFor("system.get_volume", {}))}
          >
            Read volume
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute(requestFor("system.get_battery", {}))}
          >
            Battery
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute(requestFor("system.get_cpu_usage", {}))}
          >
            CPU usage
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() =>
              void execute(requestFor("system.get_memory_usage", {}))
            }
          >
            Memory usage
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() =>
              void execute(requestFor("system.take_screenshot", {}))
            }
          >
            Screenshot
          </button>
        </div>
        <label className="field-label volume-field">
          Volume
          <input
            type="number"
            min="0"
            max="100"
            value={volume}
            disabled={running !== null}
            onChange={(event) => setVolume(Number(event.target.value))}
          />
        </label>
        <div className="application-actions">
          <button
            className="button"
            type="button"
            disabled={running !== null || volume < 0 || volume > 100}
            onClick={() =>
              void execute(requestFor("system.set_volume", { volume }))
            }
          >
            Set volume
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute(requestFor("system.mute", {}))}
          >
            Mute
          </button>
          <button
            className="button"
            type="button"
            disabled={running !== null}
            onClick={() => void execute(requestFor("system.unmute", {}))}
          >
            Unmute
          </button>
        </div>
        <div className="window-tool-area">
          <div>
            <h2>Windows</h2>
            <p>
              Focus uses a listed window id when available, avoiding ambiguous
              title matches.
            </p>
          </div>
          <div className="application-actions">
            <button
              className="button"
              type="button"
              disabled={running !== null}
              onClick={() => void execute(requestFor("window.list", {}))}
            >
              List windows
            </button>
            <button
              className="button"
              type="button"
              disabled={running !== null || !selectedWindowId}
              onClick={() =>
                void execute(
                  requestFor("window.focus", { windowId: selectedWindowId }),
                )
              }
            >
              Focus selected
            </button>
          </div>
          {windows.length > 0 && (
            <label className="field-label">
              Visible window
              <select
                value={selectedWindowId}
                disabled={running !== null}
                onChange={(event) => setSelectedWindowId(event.target.value)}
              >
                {windows.map((window) => (
                  <option key={window.windowId} value={window.windowId}>
                    {window.title}
                  </option>
                ))}
              </select>
            </label>
          )}
          {focusedWindow && (
            <p className="supporting-note">Selected: {focusedWindow.title}</p>
          )}
        </div>
        {running && (
          <p className="supporting-note" role="status">
            Running {running}...
          </p>
        )}
        {result && (
          <div className="tool-result" aria-live="polite">
            <p className="technical">
              {result.tool ?? "system tool"} / {result.status}
            </p>
            <p>{resultMessage(result)}</p>
            {result.data !== undefined && (
              <pre>{JSON.stringify(result.data, null, 2)}</pre>
            )}
          </div>
        )}
      </section>
    </>
  );
}
