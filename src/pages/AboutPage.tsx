import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { PageHeading } from "../components/ui/PageHeading";
interface Check { name: string; status: string; guidance: string }
export function AboutPage({ version }: { version: string }) {
  const [checks, setChecks] = useState<Check[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setChecks([{ name: "Desktop runtime", status: "Desktop only", guidance: "Run diagnostics in the installed NOVA application." }]);
      return;
    }
    setBusy(true);
    setError("");
    void invoke<Check[]>("run_diagnostics").then(value => {
      if (active) setChecks(value);
    }).catch(value => {
      if (active) setError(String(value));
    }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [revision]);
  return <>
    <PageHeading title="About NOVA" description={`NOVA ${version} - Local desktop assistant`} />
    <section className="panel about-panel" aria-labelledby="diagnostics-heading">
      <h2 id="diagnostics-heading">Readiness diagnostics</h2>
      <p className="supporting-note">Checks stay local and do not download models. Device detection and resource presence do not replace a real voice test.</p>
      <button disabled={busy || !isTauri()} onClick={() => setRevision(value => value + 1)}>{busy ? "Checking..." : "Run diagnostics"}</button>{" "}
      <button disabled={!isTauri()} onClick={() => { void invoke("open_logs_folder").catch(value => setError(String(value))); }}>Open logs folder</button>
      {error && <p role="alert" className="microphone-error">{error}</p>}
      <dl className="metadata" aria-live="polite" aria-busy={busy}>
        {checks.map(check => <div key={check.name}>
          <dt>{check.name}</dt>
          <dd>{check.status}<p className="supporting-note">{check.guidance}</p></dd>
        </div>)}
      </dl>
      <p className="supporting-note">Publisher signing is not configured for this release. The installer is unsigned.</p>
    </section>
  </>;
}
