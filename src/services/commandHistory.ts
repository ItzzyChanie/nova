import { invoke, isTauri } from "@tauri-apps/api/core";

export interface CommandHistoryEntry {
  timestamp: number;
  durationMillis: number;
  inputSource: string;
  tool: string;
  displayCommand: string;
  result: string;
  status: string;
  error?: string | null;
}

export async function loadCommandHistory(): Promise<CommandHistoryEntry[]> {
  if (!isTauri()) return [];
  return invoke<CommandHistoryEntry[]>("get_command_history");
}

export interface HistoryDay {
  start: number;
  end: number;
}
export function localHistoryDay(now = new Date()): HistoryDay {
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  return { start: start.getTime(), end: end.getTime() };
}
export function historyForDay(entries: CommandHistoryEntry[], day: HistoryDay) {
  return entries.filter(
    ({ timestamp }) => timestamp >= day.start && timestamp < day.end,
  );
}
export async function clearCommandHistoryDay(day: HistoryDay): Promise<void> {
  if (!isTauri()) return;
  await invoke("clear_command_history_day", { start: day.start, end: day.end });
}
