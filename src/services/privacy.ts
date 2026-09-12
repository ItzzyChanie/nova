import { invoke, isTauri } from "@tauri-apps/api/core";
export interface PrivacySettings {
  audioStorage: boolean;
  commandLog: boolean;
  networkAccess: boolean;
}
export interface RuntimeInfo {
  version: string;
  platform: string;
  architecture: string;
  tts: string;
}
export const getPrivacySettings = () =>
  invoke<PrivacySettings>("get_privacy_settings");
export const setPrivacySetting = (
  key: "commandLog" | "networkAccess",
  enabled: boolean,
) => invoke<PrivacySettings>("set_privacy_setting", { key, enabled });
export const clearLocalData = (confirmation: string) =>
  invoke<void>("clear_local_data", { confirmation });
export async function getRuntimeInfo(): Promise<RuntimeInfo> {
  if (!isTauri())
    return {
      version: "Desktop only",
      platform: "Browser preview",
      architecture: "Unavailable",
      tts: "Desktop only",
    };
  return invoke<RuntimeInfo>("get_runtime_info");
}
