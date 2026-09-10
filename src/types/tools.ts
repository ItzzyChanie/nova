export const toolCategories = [
  "application",
  "file",
  "folder",
  "system",
  "browser",
  "window",
  "developer",
  "workflow",
] as const;

export type ToolCategory = (typeof toolCategories)[number];
export type ToolRiskLevel = "safe" | "sensitive" | "destructive";
export type ToolPermission = "allowed" | "denied" | "confirm";

export interface FileSearchArguments {
  query: string;
  root?: string;
  extension?: string;
  exact?: boolean;
  modifiedWithinDays?: number;
}

export interface FileTargetArguments {
  path?: string;
  resultId?: string;
}

export interface ToolArgumentMap {
  "application.open": { application: string };
  "application.close": { application: string };
  "application.focus": { application: string };
  "application.is_running": { application: string };
  "application.list_running": Record<string, never>;
  "folder.open": { folder: string };
  "folder.find": FileSearchArguments;
  "file.open": FileTargetArguments;
  "file.find_by_name": FileSearchArguments;
  "file.reveal_in_explorer": FileTargetArguments;
  "file.delete": { path: string };
  "folder.list": { path: string };
  "system.get_volume": Record<string, never>;
  "system.set_volume": { volume: number };
  "system.adjust_volume": { delta: number };
  "system.mute": Record<string, never>;
  "system.unmute": Record<string, never>;
  "system.get_battery": Record<string, never>;
  "system.get_cpu_usage": Record<string, never>;
  "system.get_memory_usage": Record<string, never>;
  "system.take_screenshot": Record<string, never>;
  "system.shutdown": Record<string, never>;
  "browser.search": { query: string };
  "window.list": Record<string, never>;
  "window.focus": { title?: string; windowId?: string; exact?: boolean };
  "developer.runScript": { scriptId: string };
  "workflow.preview": { steps: string[] };
}

export type ToolName = keyof ToolArgumentMap;
export type ApplicationToolName = Extract<ToolName, `application.${string}`>;
export type SystemOrWindowToolName = Extract<
  ToolName,
  `system.${string}` | `window.${string}`
>;
export type FileToolName =
  | "folder.open"
  | "folder.find"
  | "file.open"
  | "file.find_by_name"
  | "file.reveal_in_explorer";

export type ToolRequest = {
  [Name in ToolName]: {
    requestId?: string;
    tool: Name;
    arguments: ToolArgumentMap[Name];
  };
}[ToolName];

export type ApplicationToolRequest = Extract<
  ToolRequest,
  { tool: ApplicationToolName }
>;
export type FileToolRequest = Extract<ToolRequest, { tool: FileToolName }>;
export type SystemOrWindowToolRequest = Extract<
  ToolRequest,
  { tool: SystemOrWindowToolName }
>;

export type ToolResultStatus =
  | "completed"
  | "rejected"
  | "denied"
  | "confirmationRequired"
  | "unavailable";

export type ToolErrorCode =
  | "invalidTool"
  | "invalidArguments"
  | "permissionDenied"
  | "confirmationRequired"
  | "notImplemented"
  | "settingsUnavailable"
  | "unknownApplication"
  | "ambiguousApplication"
  | "applicationNotInstalled"
  | "applicationNotRunning"
  | "applicationLaunchFailed"
  | "applicationFocusFailed"
  | "applicationCloseFailed"
  | "unsupportedPlatform"
  | "unknownRoot"
  | "pathOutsideApprovedRoots"
  | "fileNotFound"
  | "folderNotFound"
  | "invalidResultId"
  | "invalidProject"
  | "fileSearchFailed"
  | "fileOpenFailed"
  | "fileRevealFailed"
  | "volumeUnavailable"
  | "systemInfoUnavailable"
  | "screenshotFailed"
  | "invalidWindow"
  | "windowNotFound"
  | "ambiguousWindow"
  | "windowFocusFailed";

export interface ToolError {
  code: ToolErrorCode;
  message: string;
}

export interface ToolResult {
  requestId?: string | null;
  tool?: ToolName | null;
  category?: ToolCategory | null;
  risk?: ToolRiskLevel | null;
  permission?: ToolPermission | null;
  status: ToolResultStatus;
  data?: unknown;
  error?: ToolError | null;
}
