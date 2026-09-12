import type { ToolRequest, ToolResult } from "./tools";

export type LocalModelStatus = "loaded" | "notLoaded" | "modelMissing" | "serviceUnavailable" | "error";

export interface LocalModelInfo {
  runtime: string;
  model: string;
  size: string;
  ramRequirement: string;
  expectedLatency: string;
  status: LocalModelStatus;
  message: string;
}

export type NaturalCommandStatus =
  | "ready"
  | "assistantHidden"
  | "novaEnabled"
  | "novaDisabled"
  | "clarificationRequired"
  | "unsupported"
  | "rejected"
  | "modelUnavailable"
  | "error";

export interface ToolSequence {
  type: "tool_sequence";
  steps: ToolRequest[];
}

export interface NaturalCommandResult {
  status: NaturalCommandStatus;
  source: "deterministic" | "ollama";
  model: string;
  input: string;
  normalizedInput?: string;
  confidence?: number | null;
  request?: ToolRequest | ToolSequence | null;
  routing?: ToolResult | null;
  message: string;
  candidates: string[];
  latencyMillis: number;
}
