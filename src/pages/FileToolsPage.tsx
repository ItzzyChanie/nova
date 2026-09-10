import { useEffect, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import {
  addKnownProject,
  loadKnownProjects,
  type ProjectRecord,
} from "../services/projects";
import { executeConfirmedFileTool } from "../services/toolRouter";
import type {
  FileSearchArguments,
  FileTargetArguments,
  FileToolName,
  FileToolRequest,
  ToolResult,
} from "../types/tools";

interface SearchCandidate {
  resultId: string;
  name: string;
  path: string;
  kind: "file" | "folder";
  extension?: string | null;
  modifiedAt?: number | null;
}

interface SearchData {
  candidates: SearchCandidate[];
  count: number;
  ambiguous: boolean;
  truncated: boolean;
  message: string;
}

function requestFor(
  tool: FileToolName,
  argumentsValue:
    | FileSearchArguments
    | FileTargetArguments
    | { folder: string },
): FileToolRequest {
  return {
    requestId: crypto.randomUUID(),
    tool,
    arguments: argumentsValue,
  } as FileToolRequest;
}

export function FileToolsPage() {
  const [root, setRoot] = useState("Downloads");
  const [query, setQuery] = useState("");
  const [extension, setExtension] = useState("");
  const [exact, setExact] = useState(false);
  const [recentDays, setRecentDays] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<ToolResult | null>(null);
  const [candidates, setCandidates] = useState<SearchCandidate[]>([]);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [projectName, setProjectName] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [projectEditor, setProjectEditor] = useState("Visual Studio Code");
  const [projectError, setProjectError] = useState<string | null>(null);

  useEffect(() => {
    void loadKnownProjects()
      .then(setProjects)
      .catch((error: unknown) =>
        setProjectError(error instanceof Error ? error.message : String(error)),
      );
  }, []);

  async function execute(request: FileToolRequest) {
    if (running) return;
    setRunning(true);
    setResult(null);
    try {
      const next = await executeConfirmedFileTool(request);
      setResult(next);
      const data = next.data as SearchData | undefined;
      setCandidates(Array.isArray(data?.candidates) ? data.candidates : []);
    } catch (error: unknown) {
      setResult({
        tool: request.tool,
        status: "rejected",
        error: {
          code: "fileOpenFailed",
          message: error instanceof Error ? error.message : String(error),
        },
      });
    } finally {
      setRunning(false);
    }
  }

  function searchArguments(): FileSearchArguments {
    const modifiedWithinDays = Number.parseInt(recentDays, 10);
    return {
      query,
      root,
      extension: extension || undefined,
      exact,
      modifiedWithinDays: Number.isFinite(modifiedWithinDays)
        ? modifiedWithinDays
        : undefined,
    };
  }

  async function registerProject() {
    if (!projectName.trim() || !projectPath.trim()) {
      setProjectError("Project name and folder path are required.");
      return;
    }
    if (!window.confirm(`Approve ${projectPath} as a NOVA project root?`))
      return;
    setProjectError(null);
    try {
      const records = await addKnownProject(
        projectName,
        projectPath,
        projectEditor,
      );
      setProjects(records);
      setRoot(projectName.trim());
      setProjectName("");
      setProjectPath("");
    } catch (error: unknown) {
      setProjectError(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <>
      <PageHeading
        title="File & folder tools"
        description="Find and open files in your approved folders."
        preview="Development panel — traversal is bounded and delete, move, and rename are not implemented."
      />
      <section
        className="panel file-tool-panel"
        aria-labelledby="file-tools-title"
      >
        <h2 id="file-tools-title">Approved-root tools</h2>
        <div className="file-tool-fields">
          <label className="field-label">
            Root
            <select
              value={root}
              disabled={running}
              onChange={(event) => setRoot(event.target.value)}
            >
              {["Desktop", "Documents", "Downloads", "Projects"].map((name) => (
                <option key={name}>{name}</option>
              ))}
              {projects.map((project) => (
                <option key={project.id} value={project.name}>
                  {project.name} — project
                </option>
              ))}
            </select>
          </label>
          <label className="field-label">
            Name query
            <input
              value={query}
              disabled={running}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="e.g. Automata"
            />
          </label>
          <label className="field-label">
            Extension
            <input
              value={extension}
              disabled={running}
              onChange={(event) => setExtension(event.target.value)}
              placeholder="e.g. pptx"
            />
          </label>
          <label className="field-label">
            Modified within days
            <input
              type="number"
              min="1"
              max="3650"
              value={recentDays}
              disabled={running}
              onChange={(event) => setRecentDays(event.target.value)}
              placeholder="Any time"
            />
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={exact}
              disabled={running}
              onChange={(event) => setExact(event.target.checked)}
            />
            Exact name
          </label>
        </div>
        <div className="application-actions">
          <button
            className="button"
            type="button"
            disabled={running}
            onClick={() =>
              void execute(requestFor("folder.open", { folder: root }))
            }
          >
            Open root
          </button>
          <button
            className="button"
            type="button"
            disabled={running || !query.trim()}
            onClick={() =>
              void execute(requestFor("file.find_by_name", searchArguments()))
            }
          >
            Find files
          </button>
          <button
            className="button"
            type="button"
            disabled={running || !query.trim()}
            onClick={() =>
              void execute(requestFor("folder.find", searchArguments()))
            }
          >
            Find folders
          </button>
        </div>
        {result && (
          <div className="tool-result" aria-live="polite">
            <p className="technical">
              {result.tool} · {result.status}
            </p>
            <p>
              {result.error?.message ??
                (result.data as { message?: string } | undefined)?.message ??
                "Request completed."}
            </p>
          </div>
        )}
        {candidates.length > 0 && (
          <div className="candidate-list">
            {candidates.map((candidate, index) => (
              <article key={candidate.resultId} className="candidate-row">
                <div>
                  <strong>
                    {index + 1}. {candidate.name}
                  </strong>
                  <p className="technical">{candidate.path}</p>
                  <p className="technical">Result ID: {candidate.resultId}</p>
                </div>
                <div className="candidate-actions">
                  <button
                    className="button"
                    type="button"
                    disabled={running}
                    onClick={() =>
                      void execute(
                        requestFor(
                          candidate.kind === "folder"
                            ? "folder.open"
                            : "file.open",
                          candidate.kind === "folder"
                            ? { folder: candidate.path }
                            : { resultId: candidate.resultId },
                        ),
                      )
                    }
                  >
                    Open
                  </button>
                  {candidate.kind === "file" && (
                    <button
                      className="button"
                      type="button"
                      disabled={running}
                      onClick={() =>
                        void execute(
                          requestFor("file.reveal_in_explorer", {
                            resultId: candidate.resultId,
                          }),
                        )
                      }
                    >
                      Reveal
                    </button>
                  )}
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
      <section className="panel project-panel" aria-labelledby="project-title">
        <h2 id="project-title">Known development projects</h2>
        <p>
          Explicitly approving a project adds its folder as a searchable root.
        </p>
        <div className="file-tool-fields">
          <label className="field-label">
            Project name
            <input
              value={projectName}
              onChange={(event) => setProjectName(event.target.value)}
              placeholder="ProctorX"
            />
          </label>
          <label className="field-label">
            Folder path
            <input
              value={projectPath}
              onChange={(event) => setProjectPath(event.target.value)}
              placeholder="D:\Projects\ProctorX"
            />
          </label>
          <label className="field-label">
            Editor
            <select
              value={projectEditor}
              onChange={(event) => setProjectEditor(event.target.value)}
            >
              <option>Visual Studio Code</option>
            </select>
          </label>
        </div>
        <button
          className="button"
          type="button"
          onClick={() => void registerProject()}
        >
          Approve project root
        </button>
        {projectError && (
          <p className="assistant-launch-error" role="alert">
            {projectError}
          </p>
        )}
        {projects.length > 0 && (
          <ul className="project-list">
            {projects.map((project) => (
              <li key={project.id}>
                <strong>{project.name}</strong>
                <span>{project.path}</span>
                <span>{project.editor}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </>
  );
}
