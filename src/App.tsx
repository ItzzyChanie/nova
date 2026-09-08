import { useState, type ReactNode } from "react";
import { AppShell } from "./components/layout/AppShell";
import { OverviewPage } from "./pages/OverviewPage";
import { CommandHistoryPage } from "./pages/CommandHistoryPage";
import { SkillsPage } from "./pages/SkillsPage";
import { VoiceWakeWordPage } from "./pages/VoiceWakeWordPage";
import { PrivacyDataPage } from "./pages/PrivacyDataPage";
import { AboutPage } from "./pages/AboutPage";
import type { NovaPage } from "./types/navigation";
import type { NovaStatus } from "./types/nova";
import { createPreviewSettings, type PreviewSettings } from "./types/settings";
import "./styles/app.css";

function App() {
  const [page, setPage] = useState<NovaPage>("overview");
  const [settings, setSettings] = useState(createPreviewSettings);
  const status: NovaStatus = {
    enabled: settings.assistantEnabled,
    wakePhrase: "Hey NOVA",
    localMode: true,
    version: "0.1.0",
    stage: "Foundation",
  };

  function updateSettings(changes: Partial<PreviewSettings>) {
    setSettings((current) => ({ ...current, ...changes }));
  }

  const pages = {
    overview: <OverviewPage status={status} onEnabledChange={(enabled) => updateSettings({ assistantEnabled: enabled })} />,
    history: <CommandHistoryPage />,
    skills: <SkillsPage settings={settings} onChange={updateSettings} />,
    voice: <VoiceWakeWordPage wakePhrase={status.wakePhrase} settings={settings} onChange={updateSettings} />,
    privacy: <PrivacyDataPage settings={settings} onChange={updateSettings} />,
    about: <AboutPage version={status.version} />,
  } satisfies Record<NovaPage, ReactNode>;

  return (
    <AppShell status={status} page={page} onNavigate={setPage}>
      {pages[page]}
    </AppShell>
  );
}

export default App;
