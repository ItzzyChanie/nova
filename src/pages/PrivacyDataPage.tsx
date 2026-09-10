import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import type { SettingsPageProps } from "../types/settings";

export function PrivacyDataPage({ settings, onChange }: SettingsPageProps) {
  return (
    <>
      <PageHeading
        title="Privacy & data"
        description="Your data stays close. Your controls stay here."
        preview="Microphone tests are processed transiently. Audio storage remains unavailable and off."
      />
      <div className="card-grid">
        <Card
          title="Audio storage"
          description="Unavailable in this phase. Microphone test samples are discarded immediately and never written to disk."
          control={
            <Toggle
              label="Audio storage"
              checked={false}
              disabled
              onChange={() => undefined}
            />
          }
        />
        <Card
          title="Command log"
          description="Commands are saved locally. Clear today's entries from Command history."
          control={
            <Toggle
              label="Command log"
              checked={settings.commandLog}
              onChange={(checked) => onChange({ commandLog: checked })}
            />
          }
        />
        <Card
          title="Network access"
          description="Core NOVA functionality is intended to operate without cloud AI services. This preview does not control network access."
          control={
            <Toggle
              label="Network access"
              checked={settings.networkAccess}
              onChange={(checked) => onChange({ networkAccess: checked })}
            />
          }
        />
        <Card
          title="Clear all data"
          description="Not implemented. This button does not delete anything. Use Clear today in Command history to delete that day's entries; your settings, models, and files are kept."
        >
          <button type="button" className="button button-danger" disabled>
            Clear all data
          </button>
        </Card>
      </div>
    </>
  );
}
