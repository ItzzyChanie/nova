export const skillDefinitions = [
  { id: "apps", title: "Open apps & files", description: "Launch installed apps and open approved files or folders." },
  { id: "system", title: "System controls", description: "Control supported system functions such as volume and device actions." },
  { id: "web", title: "Web search", description: "Open browser searches from spoken commands." },
  { id: "calendar", title: "Calendar & reminders", description: "Future local productivity integration." },
  { id: "files", title: "File search", description: "Find approved files by name or indexed content." },
  { id: "drafting", title: "Text & email drafting", description: "Generate local text using the future local language model." },
  { id: "windows", title: "Window management", description: "Switch, focus, and manage supported application windows." },
  { id: "scripts", title: "Custom scripts", description: "Allow explicitly registered local automation scripts." },
] as const;

export type SkillId = (typeof skillDefinitions)[number]["id"];
export const sensitivities = ["Low", "Medium", "High"] as const;
export type Sensitivity = (typeof sensitivities)[number];

export interface PreviewSettings {
  assistantEnabled: boolean;
  skills: Record<SkillId, boolean>;
  sensitivity: Sensitivity;
  voiceReply: boolean;
  audioStorage: boolean;
  commandLog: boolean;
  networkAccess: boolean;
}

export interface SettingsPageProps {
  settings: PreviewSettings;
  onChange: (changes: Partial<PreviewSettings>) => void;
}

// UI preferences only. These values never authorize native capabilities.
export function createPreviewSettings(): PreviewSettings {
  return {
    assistantEnabled: true,
    skills: { apps: true, system: false, web: false, calendar: false, files: false, drafting: false, windows: true, scripts: false },
    sensitivity: "Medium",
    voiceReply: false,
    audioStorage: false,
    commandLog: true,
    networkAccess: false,
  };
}
