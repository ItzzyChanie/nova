import React, { lazy, Suspense } from "react";
import ReactDOM from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Load only the selected window's UI and CSS; both use the same Vite project.
const label = isTauri() ? getCurrentWindow().label : "main";
const WindowRoot = lazy(() =>
  label === "assistant" ? import("./components/assistant/AssistantWindow") : import("./App"),
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Suspense fallback={null}>
      <WindowRoot />
    </Suspense>
  </React.StrictMode>,
);
