import { useEffect, useState, type ReactNode } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { AppShell } from "./components/layout/AppShell";
import { OverviewPage } from "./pages/OverviewPage";
import { CommandHistoryPage } from "./pages/CommandHistoryPage";
import { SkillsPage } from "./pages/SkillsPage";
import { VoiceWakeWordPage } from "./pages/VoiceWakeWordPage";
import { PrivacyDataPage } from "./pages/PrivacyDataPage";
import { AboutPage } from "./pages/AboutPage";
import { DeveloperToolsPage } from "./pages/DeveloperToolsPage";
import { FileToolsPage } from "./pages/FileToolsPage";
import { SystemControlsPage } from "./pages/SystemControlsPage";
import {
  loadStartupSettings,
  saveNovaEnabled,
  saveSkillPreference,
  type StartupSettings,
} from "./services/settingsStore";
import type { NovaPage } from "./types/navigation";
import type { NovaStatus } from "./types/nova";
import {
  createDefaultSettings,
  type NovaSettings,
  type SkillId,
} from "./types/settings";
import "./styles/app.css";

function settingsMismatchMessage(settings: StartupSettings): string | null {
  if (settings.novaEnabled === settings.autostartEnabled) return null;
  return settings.novaEnabled
    ? "NOVA is on, but Start with Windows is disabled. Toggle NOVA off and on to retry."
    : "NOVA is off, but Windows still reports startup enabled. Toggle NOVA on and off to retry.";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function App() {
  const [page, setPage] = useState<NovaPage>("overview");
  const [settings, setSettings] = useState(createDefaultSettings);
  const [autostartEnabled, setAutostartEnabled] = useState<boolean | null>(
    null,
  );
  const [settingsReady, setSettingsReady] = useState(false);
  const [settingsSaving, setSettingsSaving] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [savingSkill, setSavingSkill] = useState<SkillId | null>(null);
  const [skillError, setSkillError] = useState<string | null>(null);
  const status: NovaStatus = {
    enabled: settings.novaEnabled,
    wakePhrase: "NOVA or Hey NOVA",
    localMode: true,
    version: "0.1.0",
    stage: "Foundation",
  };

  useEffect(() => {
    let active = true;
    void loadStartupSettings()
      .then((startupSettings) => {
        if (!active) return;
        setSettings((current) => ({
          ...current,
          novaEnabled: startupSettings.novaEnabled,
          skills: startupSettings.skillPreferences,
          sensitivity: startupSettings.wakeSensitivity,
          assistantPaused: startupSettings.assistantPaused,
        }));
        setAutostartEnabled(startupSettings.autostartEnabled);
        setSettingsError(settingsMismatchMessage(startupSettings));
      })
      .catch((error: unknown) => {
        console.error(
          "[NOVA] Could not load persistent startup settings.",
          error,
        );
        if (active) {
          setSettingsError(
            `NOVA could not read its startup settings: ${errorMessage(error)}`,
          );
        }
      })
      .finally(() => {
        if (active) setSettingsReady(true);
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: UnlistenFn | undefined;
    void listen<StartupSettings>("nova-settings-changed", (event) => {
      if (!active) return;
      const startupSettings = event.payload;
      setSettings((current) => ({
        ...current,
        novaEnabled: startupSettings.novaEnabled,
        skills: startupSettings.skillPreferences,
        sensitivity: startupSettings.wakeSensitivity,
        assistantPaused: startupSettings.assistantPaused,
      }));
      setAutostartEnabled(startupSettings.autostartEnabled);
      setSettingsError(settingsMismatchMessage(startupSettings));
    })
      .then((dispose) => {
        if (active) unlisten = dispose;
        else dispose();
      })
      .catch((error: unknown) => {
        console.error(
          "[NOVA] Could not subscribe to native settings changes.",
          error,
        );
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  function updateSettings(changes: Partial<NovaSettings>) {
    setSettings((current) => ({ ...current, ...changes }));
  }

  async function updateNovaEnabled(enabled: boolean) {
    if (settingsSaving) return;
    setSettingsSaving(true);
    setSettingsError(null);
    try {
      const startupSettings = await saveNovaEnabled(enabled);
      setSettings((current) => ({
        ...current,
        novaEnabled: startupSettings.novaEnabled,
        skills: startupSettings.skillPreferences,
        sensitivity: startupSettings.wakeSensitivity,
        assistantPaused: startupSettings.assistantPaused,
      }));
      setAutostartEnabled(startupSettings.autostartEnabled);
      setSettingsError(settingsMismatchMessage(startupSettings));
    } catch (error: unknown) {
      console.error("[NOVA] Could not update NOVA and Windows startup.", error);
      setSettingsError(`NOVA was not changed: ${errorMessage(error)}`);

      try {
        const startupSettings = await loadStartupSettings();
        setSettings((current) => ({
          ...current,
          novaEnabled: startupSettings.novaEnabled,
          skills: startupSettings.skillPreferences,
          sensitivity: startupSettings.wakeSensitivity,
          assistantPaused: startupSettings.assistantPaused,
        }));
        setAutostartEnabled(startupSettings.autostartEnabled);
      } catch (reloadError: unknown) {
        console.error(
          "[NOVA] Could not reload settings after the failed update.",
          reloadError,
        );
      }
    } finally {
      setSettingsSaving(false);
    }
  }

  async function updateSkillPreference(skill: SkillId, enabled: boolean) {
    if (savingSkill !== null) return;
    setSavingSkill(skill);
    setSkillError(null);
    try {
      const skillPreferences = await saveSkillPreference(skill, enabled);
      setSettings((current) => ({ ...current, skills: skillPreferences }));
    } catch (error: unknown) {
      console.error("[NOVA] Could not save the skill preference.", error);
      setSkillError(
        `The skill preference was not changed: ${errorMessage(error)}`,
      );
    } finally {
      setSavingSkill(null);
    }
  }

  const pages = {
    overview: (
      <OverviewPage
        status={status}
        autostartEnabled={autostartEnabled}
        settingsReady={settingsReady}
        settingsSaving={settingsSaving}
        settingsError={settingsError}
        onEnabledChange={(enabled) => void updateNovaEnabled(enabled)}
      />
    ),
    history: <CommandHistoryPage />,
    skills: (
      <SkillsPage
        settings={settings}
        settingsReady={settingsReady}
        savingSkill={savingSkill}
        error={skillError}
        onPreferenceChange={(skill, enabled) =>
          void updateSkillPreference(skill, enabled)
        }
      />
    ),
    voice: (
      <VoiceWakeWordPage
        wakePhrase={status.wakePhrase}
        settings={settings}
        onChange={updateSettings}
      />
    ),
    privacy: <PrivacyDataPage settings={settings} onChange={updateSettings} />,
    developer: <DeveloperToolsPage />,
    files: <FileToolsPage />,
    system: <SystemControlsPage />,
    about: <AboutPage version={status.version} />,
  } satisfies Record<NovaPage, ReactNode>;

  return (
    <AppShell status={status} page={page} onNavigate={setPage}>
      {pages[page]}
    </AppShell>
  );
}

export default App;
