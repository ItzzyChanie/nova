import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  executeConfirmedApplicationTool,
  executeConfirmedFileTool,
  executeConfirmedSystemTool,
} from "./toolRouter";
import type {
  LocalModelInfo,
  NaturalCommandResult,
} from "../types/languageModel";
import type {
  ApplicationToolRequest,
  FileToolRequest,
  SystemOrWindowToolRequest,
  ToolRequest,
  ToolResult,
} from "../types/tools";

const offlineModel: LocalModelInfo = {
  runtime: "Ollama",
  model: "qwen3:1.7b",
  size: "1.4 GB (Q4_K_M)",
  ramRequirement: "Approximately 2.5-3.5 GB available RAM",
  expectedLatency: "About 0.5-3 s warm; 2-10 s cold on a modern CPU",
  status: "notLoaded",
  message: "Model status is available inside the NOVA desktop application.",
};

export async function getLocalModelInfo(): Promise<LocalModelInfo> {
  if (!isTauri()) return offlineModel;
  return invoke<LocalModelInfo>("get_local_model_info");
}

export async function interpretNaturalLanguage(
  input: string,
): Promise<NaturalCommandResult> {
  if (!isTauri()) {
    return {
      status: "modelUnavailable",
      source: "deterministic",
      model: offlineModel.model,
      input,
      message:
        "Natural-command routing is available inside the NOVA desktop application.",
      candidates: [],
      latencyMillis: 0,
    };
  }
  return invoke<NaturalCommandResult>("interpret_natural_language", { input });
}

export async function executeConfirmedNaturalRequest(
  request: ToolRequest,
): Promise<ToolResult> {
  if (request.tool.startsWith("developer.") || request.tool === "workflow.run" || request.tool === "browser.open_url") {
    return invoke<ToolResult>("execute_registered_tool", { request, confirmed: true });
  }
  if (request.tool.startsWith("application.")) {
    return executeConfirmedApplicationTool(request as ApplicationToolRequest);
  }
  if (request.tool.startsWith("file.") || request.tool.startsWith("folder.")) {
    return executeConfirmedFileTool(request as FileToolRequest);
  }
  if (
    request.tool.startsWith("system.") ||
    request.tool.startsWith("window.")
  ) {
    return executeConfirmedSystemTool(request as SystemOrWindowToolRequest);
  }
  return {
    tool: request.tool,
    status: "rejected",
    error: {
      code: "invalidTool",
      message: "This tool cannot be confirmed from the natural-command panel.",
    },
  };
}
