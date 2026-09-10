import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  DEFAULT_SKILL_PREFERENCES,
  type SkillId,
  type SkillPreferences,
  type Sensitivity,
} from "../types/settings";

export const DEFAULT_NOVA_ENABLED = true;

export interface StartupSettings {
  novaEnabled: boolean;
  autostartEnabled: boolean;
  skillPreferences: SkillPreferences;
  wakeSensitivity: Sensitivity;
  assistantPaused: boolean;
}

export const DEFAULT_STARTUP_SETTINGS: StartupSettings = {
  novaEnabled: DEFAULT_NOVA_ENABLED,
  autostartEnabled: DEFAULT_NOVA_ENABLED,
  skillPreferences: { ...DEFAULT_SKILL_PREFERENCES },
  wakeSensitivity: "Medium",
  assistantPaused: false,
};

export async function loadStartupSettings(): Promise<StartupSettings> {
  if (!isTauri()) return DEFAULT_STARTUP_SETTINGS;
  return invoke<StartupSettings>("get_startup_settings");
}

export async function saveNovaEnabled(
  enabled: boolean,
): Promise<StartupSettings> {
  if (!isTauri()) {
    return {
      novaEnabled: enabled,
      autostartEnabled: enabled,
      skillPreferences: { ...DEFAULT_SKILL_PREFERENCES },
      wakeSensitivity: "Medium",
      assistantPaused: false,
    };
  }
  return invoke<StartupSettings>("set_nova_enabled", { enabled });
}

export async function saveSkillPreference(
  skill: SkillId,
  enabled: boolean,
): Promise<SkillPreferences> {
  if (!isTauri()) {
    return { ...DEFAULT_SKILL_PREFERENCES, [skill]: enabled };
  }
  return invoke<SkillPreferences>("set_skill_preference", { skill, enabled });
}

export interface VoiceSettings {
  wakeSensitivity: Sensitivity;
  assistantPaused: boolean;
}

export async function saveWakeSensitivity(
  sensitivity: Sensitivity,
): Promise<VoiceSettings> {
  if (!isTauri())
    return { wakeSensitivity: sensitivity, assistantPaused: false };
  return invoke<VoiceSettings>("set_wake_sensitivity", { sensitivity });
}

export async function saveAssistantPaused(
  paused: boolean,
): Promise<VoiceSettings> {
  if (!isTauri()) return { wakeSensitivity: "Medium", assistantPaused: paused };
  return invoke<VoiceSettings>("set_assistant_paused", { paused });
}
