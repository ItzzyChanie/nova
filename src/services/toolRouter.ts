import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  ApplicationToolRequest,
  FileToolRequest,
  ToolRequest,
  SystemOrWindowToolRequest,
  ToolResult,
} from "../types/tools";

function unavailable(request: ToolRequest): ToolResult {
  return {
    requestId: request.requestId,
    tool: request.tool,
    status: "unavailable",
    error: {
      code: "notImplemented",
      message:
        "Native tool routing is available only inside the NOVA desktop application.",
    },
  };
}

export async function routeToolRequest(
  request: ToolRequest,
): Promise<ToolResult> {
  if (!isTauri()) return unavailable(request);
  return invoke<ToolResult>("route_tool_request", { request });
}

export async function executeConfirmedApplicationTool(
  request: ApplicationToolRequest,
): Promise<ToolResult> {
  if (!isTauri()) return unavailable(request);
  return invoke<ToolResult>("execute_confirmed_application_tool", {
    request,
    confirmed: true,
  });
}

export async function executeConfirmedFileTool(
  request: FileToolRequest,
): Promise<ToolResult> {
  if (!isTauri()) return unavailable(request);
  return invoke<ToolResult>("execute_confirmed_file_tool", {
    request,
    confirmed: true,
  });
}

export async function executeConfirmedSystemTool(
  request: SystemOrWindowToolRequest,
): Promise<ToolResult> {
  if (!isTauri()) return unavailable(request);
  return invoke<ToolResult>("execute_confirmed_system_tool", {
    request,
    confirmed: true,
  });
}
