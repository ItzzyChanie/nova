import { invoke, isTauri } from "@tauri-apps/api/core";

export interface ProjectRecord {
  id: string;
  name: string;
  path: string;
  editor: string;
  frontendCommand: string;
  backendCommand: string;
  workingDirectory: string;
  developmentUrl: string;
  notes: string;
  voiceEnabled: boolean;
}

export async function loadKnownProjects(): Promise<ProjectRecord[]> {
  if (!isTauri()) return [];
  return invoke<ProjectRecord[]>("get_known_projects");
}

export async function addKnownProject(
  name: string,
  path: string,
  editor: string,
): Promise<ProjectRecord[]> {
  if (!isTauri()) {
    throw new Error(
      "Project roots can only be approved inside the NOVA desktop application.",
    );
  }
  return invoke<ProjectRecord[]>("add_known_project", {
    name,
    path,
    editor,
    confirmed: true,
  });
}

export interface WorkflowRecord {
  id: string;
  name: string;
  enabled: boolean;
  voiceEnabled: boolean;
  steps: { tool: string; arguments: Record<string, unknown> }[];
}
export const saveProjectProfile = (project: ProjectRecord) => invoke<ProjectRecord[]>("save_project_profile", { project, confirmed: true });
export const deleteProjectProfile = (id: string) => invoke<ProjectRecord[]>("delete_project_profile", { id, confirmed: true });
export const loadProjectStatus = () => invoke<Record<string, string>>("get_project_status");
export const loadWorkflows = () => invoke<WorkflowRecord[]>("get_workflows");
export const saveWorkflow = (workflow: WorkflowRecord) => invoke<WorkflowRecord[]>("save_workflow", { workflow, confirmed: true });
export const deleteWorkflow = (id: string) => invoke<WorkflowRecord[]>("delete_workflow", { id, confirmed: true });
export const executeRegistered = (tool: string, name: string) => invoke<import("../types/tools").ToolResult>("execute_registered_tool", { request: { tool, arguments: { name } }, confirmed: true });
