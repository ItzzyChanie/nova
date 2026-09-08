import { PageHeading } from "../components/ui/PageHeading";

export function AboutPage({ version }: { version: string }) {
  return (
    <>
      <PageHeading title="About NOVA" description="A local AI desktop agent designed to perform approved tasks on your computer without requiring cloud AI services for core functionality." />
      <section className="panel about-panel" aria-labelledby="technical-heading">
        <h2 id="technical-heading">Technical information</h2>
        <dl className="metadata">
          <div><dt>Version</dt><dd className="mint">{version}</dd></div>
          <div><dt>Local model</dt><dd className="muted">Not installed</dd></div>
          <div><dt>Speech engine</dt><dd className="muted">Not installed</dd></div>
          <div><dt>Wake-word engine</dt><dd className="muted">Not installed</dd></div>
        </dl>
        <p className="supporting-note">Foundation build. No assistant engine or automation is connected.</p>
      </section>
    </>
  );
}
