import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import type { SettingsPageProps } from "../types/settings";

export function PrivacyDataPage({ settings, onChange }: SettingsPageProps) {
  return (
    <>
      <PageHeading title="Privacy & data" description="NOVA is designed so core assistant processing can remain on this device." preview="Preferences only. No audio capture, data storage, or network policy is implemented. Changes reset on reload." />
      <div className="card-grid">
        <Card
          title="Audio storage"
          description="Audio storage is disabled by default. Future voice audio should be processed transiently unless explicitly enabled."
          control={<Toggle label="Audio storage" checked={settings.audioStorage} onChange={(checked) => onChange({ audioStorage: checked })} />}
        />
        <Card
          title="Command log"
          description="Command history will be stored locally once persistence is implemented."
          control={<Toggle label="Command log" checked={settings.commandLog} onChange={(checked) => onChange({ commandLog: checked })} />}
        />
        <Card
          title="Network access"
          description="Core NOVA functionality is intended to operate without cloud AI services. This preview does not control network access."
          control={<Toggle label="Network access" checked={settings.networkAccess} onChange={(checked) => onChange({ networkAccess: checked })} />}
        />
        <Card title="Clear all data" description="Available after local persistence is implemented.">
          <button type="button" className="button button-danger" disabled>Clear all data</button>
        </Card>
      </div>
    </>
  );
}
