import { useEffect, useRef, useState } from "react";
import { PageHeading } from "../components/ui/PageHeading";
import { useCommandHistory } from "../hooks/useCommandHistory";
import {
  clearCommandHistoryDay,
  type HistoryDay,
} from "../services/commandHistory";

export function CommandHistoryPage() {
  const { today: entries, day, loading, error, refresh } = useCommandHistory();
  const [reviewDay, setReviewDay] = useState<HistoryDay | null>(null);
  const [clearing, setClearing] = useState(false);
  const [clearError, setClearError] = useState<string | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const pending = useRef(false);

  useEffect(() => {
    if (reviewDay) dialog.current?.showModal();
    else dialog.current?.close();
  }, [reviewDay]);

  async function confirmClear() {
    if (!reviewDay || pending.current) return;
    pending.current = true;
    setClearing(true);
    setClearError(null);
    try {
      await clearCommandHistoryDay(reviewDay);
      refresh();
      setReviewDay(null);
    } catch (reason) {
      setClearError(String(reason));
    } finally {
      pending.current = false;
      setClearing(false);
    }
  }

  return (
    <>
      <PageHeading
        title="Command history"
        description="Today's commands. A fresh view at midnight."
      />
      <div className="history-toolbar">
        <span className="muted">
          {new Date(day.start).toLocaleDateString(undefined, {
            dateStyle: "long",
          })}{" "}
          &middot; {entries.length} commands
        </span>
        <button
          className="button button-danger"
          disabled={loading || !!error || !entries.length}
          onClick={() => {
            setClearError(null);
            setReviewDay({ ...day });
          }}
        >
          Clear today
        </button>
      </div>
      {error && (
        <p className="assistant-launch-error" role="alert">
          {error}
        </p>
      )}
      {loading ? (
        <section className="panel empty-state">
          <h2>Loading history...</h2>
        </section>
      ) : !entries.length ? (
        <section className="panel empty-state">
          <h2>No commands today.</h2>
          <p>Your next command will appear here.</p>
        </section>
      ) : (
        <div className="history-list">
          {entries.map((entry, index) => (
            <article
              className="panel history-entry"
              key={`${entry.timestamp}-${index}`}
            >
              <div className="history-entry-heading">
                <h2>{entry.displayCommand}</h2>
                <span
                  className={`history-status history-status-${entry.status}`}
                >
                  {entry.status}
                </span>
              </div>
              <p className="technical">
                {entry.tool} &middot;{" "}
                {new Date(entry.timestamp).toLocaleTimeString()}
              </p>
              <p>{entry.error ?? entry.result}</p>
            </article>
          ))}
        </div>
      )}
      <dialog
        ref={dialog}
        className="confirm-dialog"
        aria-labelledby="clear-title"
        aria-describedby="clear-description"
        onCancel={(event) => {
          event.preventDefault();
          if (!clearing) setReviewDay(null);
        }}
      >
        <h2 id="clear-title">Clear this day's commands?</h2>
        <p id="clear-description">
          Delete command history for{" "}
          {reviewDay &&
            new Date(reviewDay.start).toLocaleDateString(undefined, {
              dateStyle: "long",
            })}
          . This also clears that day's Overview activity. This cannot be
          undone.
        </p>
        <p className="supporting-note">
          Settings, models, and your files are kept.
        </p>
        {clearError && (
          <p className="assistant-launch-error" role="alert">
            {clearError}
          </p>
        )}
        <div className="dialog-actions">
          <button
            className="button"
            autoFocus
            disabled={clearing}
            onClick={() => setReviewDay(null)}
          >
            Cancel
          </button>
          <button
            className="button button-danger"
            disabled={clearing}
            onClick={() => void confirmClear()}
          >
            {clearing ? "Clearing..." : "Delete commands"}
          </button>
        </div>
      </dialog>
    </>
  );
}
