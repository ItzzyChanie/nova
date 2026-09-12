import type { ToolCategory } from "./tools";

export const skillDefinitions = [
  {
    id: "apps",
    title: "Open applications",
    description:
      "Request opening or closing a named application through a future allowlisted native adapter.",
    implemented: true,
    toolCategories: ["application"],
    enabledPolicy: "Allowed or confirmation per tool",
  },
  {
    id: "system",
    title: "System controls",
    description:
      "Request supported system functions such as volume or power actions.",
    implemented: true,
    toolCategories: ["system"],
    enabledPolicy: "Confirmation required",
  },
  {
    id: "web",
    title: "Website actions",
    description:
      "Open approved workflow websites. External sites also require Network access.",
    implemented: true,
    toolCategories: ["browser"],
    enabledPolicy: "Confirmation required",
  },
  {
    id: "calendar",
    title: "Calendar & reminders",
    description: "Future local productivity integration.",
    implemented: false,
    toolCategories: [],
    enabledPolicy: "No tools mapped yet",
  },
  {
    id: "files",
    title: "Files & folders",
    description:
      "Search and open files or folders only inside approved native roots.",
    implemented: true,
    toolCategories: ["file", "folder"],
    enabledPolicy: "Confirm or deny per tool",
  },
  {
    id: "drafting",
    title: "Text & email drafting",
    description: "Generate local text using the future local language model.",
    implemented: false,
    toolCategories: [],
    enabledPolicy: "No tools mapped yet",
  },
  {
    id: "windows",
    title: "Window management",
    description:
      "List visible windows and request focus for a selected window.",
    implemented: true,
    toolCategories: ["window"],
    enabledPolicy: "Confirmation required",
  },
  {
    id: "scripts",
    title: "Developer projects",
    description:
      "Open and run explicitly approved project commands. Arbitrary model-generated scripts remain denied.",
    implemented: true,
    toolCategories: ["developer"],
    enabledPolicy: "Approved project profiles only",
  },
] as const satisfies readonly {
  id: string;
  title: string;
  description: string;
  implemented: boolean;
  toolCategories: readonly ToolCategory[];
  enabledPolicy: string;
}[];

export type SkillId = (typeof skillDefinitions)[number]["id"];
export type SkillPreferences = Record<SkillId, boolean>;

export const DEFAULT_SKILL_PREFERENCES: SkillPreferences = {
  apps: true,
  system: false,
  web: false,
  calendar: false,
  files: false,
  drafting: false,
  windows: true,
  scripts: false,
};

export const sensitivities = ["Low", "Medium", "High"] as const;
export type Sensitivity = (typeof sensitivities)[number];

export interface NovaSettings {
  /** Persisted by the native settings store and enforced by assistant entry points. */
  novaEnabled: boolean;
  /** Persisted preferences interpreted by the native permission engine; never grants implementation. */
  skills: SkillPreferences;
  sensitivity: Sensitivity;
  assistantPaused: boolean;
  voiceReply: boolean;
  audioStorage: boolean;
  commandLog: boolean;
  networkAccess: boolean;
}

export interface SettingsPageProps {
  settings: NovaSettings;
  onChange: (changes: Partial<NovaSettings>) => void;
}

export function createDefaultSettings(): NovaSettings {
  return {
    novaEnabled: true,
    skills: { ...DEFAULT_SKILL_PREFERENCES },
    sensitivity: "Medium",
    assistantPaused: false,
    voiceReply: false,
    audioStorage: false,
    commandLog: true,
    networkAccess: false,
  };
}
