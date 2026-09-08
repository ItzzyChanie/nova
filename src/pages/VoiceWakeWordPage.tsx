import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import { sensitivities, type SettingsPageProps } from "../types/settings";

export function VoiceWakeWordPage({ wakePhrase, settings, onChange }: SettingsPageProps & { wakePhrase: string }) {
  return (
    <>
      <PageHeading title="Voice & wake word" description="Tune how NOVA listens." preview="Controls are visual preferences only. No microphone, wake-word engine, or speech engine is connected." />
      <div className="card-grid">
        <Card title="Wake phrase">
          <p className="technical value">&ldquo;{wakePhrase}&rdquo;</p>
        </Card>
        <Card title="Microphone" description="Microphone integration coming in the voice phase.">
          <p className="technical value">Not connected</p>
        </Card>
        <Card title="Sensitivity" description="Preview the future wake-word sensitivity preference.">
          <label className="sr-only" htmlFor="sensitivity">Wake-word sensitivity</label>
          <select
            id="sensitivity"
            value={settings.sensitivity}
            aria-describedby="preview-note"
            onChange={(event) => {
              const value = sensitivities.find((option) => option === event.currentTarget.value);
              if (value) onChange({ sensitivity: value });
            }}
          >
            {sensitivities.map((value) => <option key={value}>{value}</option>)}
          </select>
        </Card>
        <Card
          title="Voice reply"
          description="NOVA can speak responses back using a future local text-to-speech engine."
          control={<Toggle label="Voice reply" checked={settings.voiceReply} onChange={(checked) => onChange({ voiceReply: checked })} />}
        />
      </div>
    </>
  );
}
