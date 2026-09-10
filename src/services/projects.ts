import { invoke, isTauri } from "@tauri-apps/api/core";

export interface ProjectRecord {
  id: string;
  name: string;
  path: string;
  editor: string;
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
