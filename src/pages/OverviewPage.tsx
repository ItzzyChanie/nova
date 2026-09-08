import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import type { NovaStatus } from "../types/nova";

interface OverviewPageProps {
  status: NovaStatus;
  onEnabledChange: (enabled: boolean) => void;
}

export function OverviewPage({ status, onEnabledChange }: OverviewPageProps) {
  return (
    <>
      <PageHeading
        title="Overview"
        description={'NOVA is designed to listen for "Hey NOVA" and run approved commands locally. Voice and command execution are not connected yet.'}
      />
      <section className="panel assistant-status" aria-labelledby="assistant-title">
        <span className="assistant-mark" aria-hidden="true">N</span>
        <div className="assistant-copy">
          <h2 id="assistant-title">{status.enabled ? "NOVA is on" : "NOVA is off"} <span className="preview-label">Development preview</span></h2>
          <p className="technical">Wake phrase: &ldquo;{status.wakePhrase}&rdquo; &middot; Not listening</p>
        </div>
        <Toggle label="Enable NOVA preview" checked={status.enabled} onChange={onEnabledChange} describedBy="assistant-availability" />
      </section>
      <p id="assistant-availability" className="supporting-note">UI state only. Wake-word engine not connected. Changes reset when the app reloads.</p>
      <dl className="summary-grid">
        <div className="panel summary-card">
          <dt>Commands today</dt>
          <dd className="metric">0</dd>
        </div>
        <div className="panel summary-card">
          <dt>Uptime this session</dt>
          <dd>Not tracked yet</dd>
        </div>
        <div className="panel summary-card">
          <dt>Wake-word accuracy</dt>
          <dd>Unavailable</dd>
        </div>
      </dl>
      <section aria-labelledby="recent-heading">
        <h2 id="recent-heading" className="section-label">Recent commands</h2>
        <div className="panel empty-state">
          <h3>No commands yet.</h3>
          <p>Commands you run with NOVA will appear here.</p>
        </div>
      </section>
      <div className="test-assistant">
        <button type="button" className="button" disabled aria-describedby="test-help">Test Assistant</button>
        <p id="test-help">Assistant window coming in the next phase.</p>
      </div>
    </>
  );
}
