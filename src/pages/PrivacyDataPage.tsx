import { useEffect, useState } from "react";
import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import { getPrivacySettings, setPrivacySetting, clearLocalData } from "../services/privacy";
import type { SettingsPageProps } from "../types/settings";
export function PrivacyDataPage({ settings, onChange }: SettingsPageProps) {
    const [loading, setLoading] = useState(true);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState('');
    const [confirm, setConfirm] = useState<string | null>(null);
    useEffect(() => { let active = true; void getPrivacySettings().then(s => { if (active)
        onChange(s); }).catch(e => { if (active)
        setError(String(e)); }).finally(() => { if (active)
        setLoading(false); }); return () => { active = false; }; }, []);
    async function update(key: 'commandLog' | 'networkAccess', value: boolean) { setBusy(true); setError(''); try {
        onChange(await setPrivacySetting(key, value));
    }
    catch (e) {
        setError(String(e));
    }
    finally {
        setBusy(false);
    } }
    return <><PageHeading title="Privacy & data" description="Local data, explicit permissions."/>
    {loading && <p role="status">Loading privacy settings...</p>}{error && <p className="microphone-error" role="alert">{error}</p>}
    <div className="card-grid">
      <Card title="Audio storage" description="Off. Microphone audio is processed in memory for wake detection and transcription, then discarded. NOVA does not record audio files." control={<Toggle label="Audio storage" checked={false} disabled onChange={() => { }}/>}/>
      <Card title="Command log" description="Save command input, selected action, result, status and duration locally. Turning this off stops future entries; existing entries remain until cleared." control={<Toggle label="Command log" checked={settings.commandLog} disabled={loading || busy} onChange={v => void update('commandLog', v)}/>}/>
      <Card title="Network access" description="Allow NOVA to open external HTTP(S) websites from approved workflows. Localhost AI and development URLs stay available. This does not firewall browsers, editors, or project commands; those programs control their own network activity." control={<Toggle label="Network access" checked={settings.networkAccess} disabled={loading || busy} onChange={v => void update('networkAccess', v)}/>}/>
      <Card title="Clear local data" description="Remove NOVA settings, history, project profiles and workflows. Stop managed project processes and turn NOVA and autostart off. Your project files and bundled models are kept.">
        <button className="button button-danger" disabled={busy} onClick={() => setConfirm('')}>Clear local data</button>
        {confirm !== null && <form className="profile-fields" onSubmit={e => { e.preventDefault(); if (confirm !== 'CLEAR NOVA DATA')
            return; setBusy(true); setError(''); void clearLocalData(confirm).then(() => window.location.reload()).catch(e => { setError(String(e)); setBusy(false); }); }}>
          <label className="field-label">Type CLEAR NOVA DATA to confirm<input autoFocus value={confirm} disabled={busy} onChange={e => setConfirm(e.target.value)}/></label>
          <div className="application-actions"><button className="button button-danger" disabled={busy || confirm !== 'CLEAR NOVA DATA'}>{busy ? 'Clearing...' : 'Confirm clear'}</button><button type="button" className="button" disabled={busy} onClick={() => setConfirm(null)}>Cancel</button></div>
        </form>}
      </Card>
    </div>
  </>;
}
