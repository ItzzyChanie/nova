import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    const window = getCurrentWindow();
    let active = true;
    let dispose: UnlistenFn | undefined;
    const update = () => { void window.isMaximized().then(value => { if (active) setMaximized(value); }).catch(() => {}); };
    update();
    void window.onResized(update).then(unlisten => { if (active) dispose = unlisten; else unlisten(); });
    return () => { active = false; dispose?.(); };
  }, []);
  async function action(kind: "minimize" | "maximize" | "close") {
    if (!isTauri()) return;
    setError(null);
    try {
      const window = getCurrentWindow();
      if (kind === "minimize") await window.minimize();
      else if (kind === "maximize") { await window.toggleMaximize(); setMaximized(await window.isMaximized()); }
      else await window.close();
    } catch (reason) { setError(String(reason)); }
  }
  return <header className="title-bar">
    <div className="title-drag" data-tauri-drag-region onDoubleClick={() => void action("maximize")}><span data-tauri-drag-region>NOVA</span></div>
    {error && <span className="title-error" role="alert">{error}</span>}
    <div className="window-controls">
      <button aria-label="Minimize window" title="Minimize" onClick={() => void action("minimize")}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg></button>
      <button aria-label={maximized ? "Restore window" : "Maximize window"} title={maximized ? "Restore" : "Maximize"} onClick={() => void action("maximize")}><svg viewBox="0 0 16 16" aria-hidden="true"><path d={maximized ? "M5 5h7v7H5z M3 10V3h7" : "M4 4h8v8H4z"} /></svg></button>
      <button className="window-close" aria-label="Close to tray" title="Close to tray" onClick={() => void action("close")}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8 M12 4l-8 8" /></svg></button>
    </div>
  </header>;
}
