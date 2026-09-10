import { invoke, isTauri } from "@tauri-apps/api/core";

export async function showAssistant(): Promise<void> {
  if (!isTauri()) {
    throw new Error(
      "Open NOVA in the desktop app to test the assistant window.",
    );
  }
  await invoke<void>("show_assistant");
}

export async function hideAssistant(): Promise<void> {
  await invoke<void>("hide_assistant");
}
