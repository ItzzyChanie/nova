import { useCallback, useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  historyForDay,
  loadCommandHistory,
  localHistoryDay,
  type CommandHistoryEntry,
} from "../services/commandHistory";

export function useCommandHistory() {
  const [entries, setEntries] = useState<CommandHistoryEntry[]>([]);
  const [day, setDay] = useState(() => localHistoryDay());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const refresh = useCallback(() => setRevision((value) => value + 1), []);
  useEffect(() => {
    let active = true;
    let sequence = 0;
    let unlisten: UnlistenFn | undefined;
    let midnight: ReturnType<typeof setTimeout>;
    const updateDay = () => {
      const next = localHistoryDay();
      setDay(next);
      clearTimeout(midnight);
      midnight = setTimeout(updateDay, Math.max(1, next.end - Date.now() + 20));
    };
    const load = async () => {
      const request = ++sequence;
      updateDay();
      try {
        const history = await loadCommandHistory();
        if (active && request === sequence) {
          setEntries(history);
          setError(null);
        }
      } catch (reason) {
        if (active && request === sequence) setError(String(reason));
      } finally {
        if (active && request === sequence) setLoading(false);
      }
    };
    const onFocus = () => {
      void load();
    };
    const onVisible = () => {
      if (!document.hidden) void load();
    };
    if (isTauri()) {
      void listen("command-history-changed", onFocus)
        .then((dispose) => {
          if (!active) dispose();
          else {
            unlisten = dispose;
            void load();
          }
        })
        .catch(() => {
          /* Focus and periodic refresh remain available. */
        });
    }
    void load();
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisible);
    const interval = setInterval(() => {
      if (!document.hidden) void load();
    }, 15000);
    return () => {
      active = false;
      unlisten?.();
      clearTimeout(midnight);
      clearInterval(interval);
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [revision]);
  return {
    entries,
    today: historyForDay(entries, day),
    day,
    loading,
    error,
    refresh,
  };
}
