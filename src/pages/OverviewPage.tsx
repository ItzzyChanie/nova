import { useRef, useState } from "react";
import { Icon } from "../components/ui/Icon";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import type { NovaStatus } from "../types/nova";
import { showAssistant } from "../services/assistantWindow";
import { useCommandHistory } from "../hooks/useCommandHistory";

interface OverviewPageProps {
  status: NovaStatus;
  autostartEnabled: boolean | null;
  settingsReady: boolean;
  settingsSaving: boolean;
  settingsError: string | null;
  onEnabledChange: (enabled: boolean) => void;
}

export function OverviewPage({
  status,
  autostartEnabled,
  settingsReady,
  settingsSaving,
  settingsError,
  onEnabledChange,
}: OverviewPageProps) {
  const [showing, setShowing] = useState(false);
  const [assistantError, setAssistantError] = useState<string | null>(null);
  const {
    today: history,
    loading: historyLoading,
    error: historyError,
  } = useCommandHistory();
  const pending = useRef(false);

  async function testAssistant() {
    if (!status.enabled || pending.current) return;
    pending.current = true;
    setShowing(true);
    setAssistantError(null);
    try {
      await showAssistant();
    } catch (error: unknown) {
      setAssistantError(error instanceof Error ? error.message : String(error));
    } finally {
      pending.current = false;
      setShowing(false);
    }
  }

  const startupStatus =
    autostartEnabled === null
      ? "Checking..."
      : autostartEnabled
        ? "Enabled"
        : "Disabled";
  const commandsToday = history.length;
  const recent = history.slice(0, 3);

  return (
    <>
      <PageHeading
        title="Overview"
        description="A little less effort. A little more done."
      />
      <section
        className="panel overview-hero"
        aria-labelledby="assistant-title"
      >
        <div className="hero-top">
          <span className="eyebrow">
            <span
              className={status.enabled ? "status-dot mint" : "status-dot"}
            />{" "}
            {status.enabled ? "Assistant enabled" : "Assistant off"}
          </span>
          <Toggle
            label="Enable NOVA assistant"
            checked={status.enabled}
            disabled={!settingsReady || settingsSaving}
            onChange={onEnabledChange}
            describedBy="assistant-availability"
          />
        </div>
        <div className="hero-content">
          <div className="hero-copy">
            <h2 id="assistant-title">
              {status.enabled ? "Your day, simplified." : "Ready when you are."}
            </h2>
            <p>Say &ldquo;{status.wakePhrase}&rdquo; to get started.</p>
            <button
              type="button"
              className="button button-primary"
              disabled={!settingsReady || !status.enabled || showing}
              onClick={() => void testAssistant()}
              aria-busy={showing}
            >
              <Icon name="skills" /> {showing ? "Opening..." : "Open assistant"}{" "}
              <Icon name="arrow" />
            </button>
          </div>
          <div className="hero-orb" aria-hidden="true">
            <div>
              <Icon name="skills" />
            </div>
          </div>
        </div>
        <div className="hero-bottom">
          <span>
            <Icon name="privacy" /> On-device processing
          </span>
          <span>
            <kbd>Alt</kbd> + <kbd>N</kbd>
          </span>
        </div>
      </section>
      <p
        id="assistant-availability"
        className="supporting-note"
        aria-live="polite"
      >
        {!settingsReady
          ? "Loading settings..."
          : settingsSaving
            ? "Saving..."
            : "Enabling NOVA also enables Start with Windows."}
      </p>
      {settingsError && (
        <p className="assistant-launch-error" role="alert">
          {settingsError}
        </p>
      )}
      {assistantError && (
        <p className="assistant-launch-error" role="alert">
          {assistantError}
        </p>
      )}
      <dl className="overview-metrics">
        <div className="panel">
          <span className="metric-icon">
            <Icon name="history" />
          </span>
          <dt>Commands today</dt>
          <dd>{historyLoading ? "…" : historyError ? "—" : commandsToday}</dd>
        </div>
        <div className="panel">
          <span className="metric-icon mint">
            <Icon name="voice" />
          </span>
          <dt>Wake shortcut</dt>
          <dd>{status.enabled ? "Alt + N" : "Off"}</dd>
        </div>
        <div className="panel">
          <span className="metric-icon">
            <Icon name="power" />
          </span>
          <dt>Windows startup</dt>
          <dd>{startupStatus}</dd>
        </div>
      </dl>
      <section
        className="panel overview-activity"
        aria-labelledby="recent-heading"
      >
        <div className="activity-heading">
          <h2 id="recent-heading">
            <Icon name="history" /> Recent activity
          </h2>
          <span className="muted">Latest {recent.length}</span>
        </div>
        {historyError ? (
          <p className="assistant-launch-error" role="alert">
            {historyError}
          </p>
        ) : recent.length === 0 ? (
          <div className="overview-empty">
            <span className="empty-icon">
              <Icon name="skills" />
            </span>
            <h3>
              {historyLoading
                ? "Loading activity..."
                : "Your next idea starts here."}
            </h3>
            <p>
              {historyLoading
                ? "Checking local history."
                : "Ask NOVA to open an app or find a file."}
            </p>
          </div>
        ) : (
          <div className="activity-list">
            {recent.map((entry, index) => (
              <article
                className="activity-row"
                key={`${entry.timestamp}-${index}`}
              >
                <span className="activity-icon">
                  <Icon name="arrow" />
                </span>
                <div>
                  <h3>{entry.displayCommand}</h3>
                  <time dateTime={new Date(entry.timestamp).toISOString()}>
                    {new Date(entry.timestamp).toLocaleString([], {
                      month: "short",
                      day: "numeric",
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </time>
                </div>
                <span
                  className={`history-status history-status-${entry.status}`}
                >
                  {entry.status}
                </span>
              </article>
            ))}
          </div>
        )}
      </section>
    </>
  );
}
