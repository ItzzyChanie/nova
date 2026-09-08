export const navigationItems = [
  { id: "overview", label: "Overview" },
  { id: "history", label: "Command history" },
  { id: "skills", label: "Skills" },
  { id: "voice", label: "Voice & wake word" },
  { id: "privacy", label: "Privacy & data" },
  { id: "about", label: "About" },
] as const;

export type NovaPage = (typeof navigationItems)[number]["id"];
