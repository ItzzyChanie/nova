import { PageHeading } from "../components/ui/PageHeading";

export function CommandHistoryPage() {
  return (
    <>
      <PageHeading title="Command history" description="Every command NOVA has run can be reviewed here. History will remain on this device." />
      <section className="panel empty-state" aria-labelledby="history-empty">
        <h2 id="history-empty">No command history yet.</h2>
        <p>Once NOVA begins executing commands, completed actions will appear here.</p>
        <p className="supporting-note">Execution and local persistence are not implemented. Nothing is being recorded.</p>
      </section>
    </>
  );
}
