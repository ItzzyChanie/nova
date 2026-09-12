import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { hideAssistant } from "../../services/assistantWindow";
import type { AssistantState } from "../../types/assistant";
import novaLogo from "../../assets/Nova-logo.png";
import { Icon } from "../ui/Icon";
import "../../styles/assistant.css";

const stateCopy = {
  idle: { label: "Idle", heading: "How can I help?" },
  listening: { label: "Listening", heading: "Listening..." },
  transcribing: { label: "Transcribing", heading: "Transcribing..." },
  processing: { label: "Processing", heading: "Processing your command..." },
  thinking: { label: "Understanding", heading: "Understanding..." },
  executing: { label: "Executing", heading: "Running your approved action..." },
  success: { label: "Success", heading: "All done." },
  error: { label: "Error", heading: "Something went wrong." },
} satisfies Record<AssistantState, { label: string; heading: string }>;

export default function AssistantWindow() {
  const [state, setState] = useState<AssistantState>("idle");
  const [transcript, setTranscript] = useState<string | null>(null);
  const [voiceError, setVoiceError] = useState<string | null>(null);
  const [dismissError, setDismissError] = useState<string | null>(null);
  const [hiding, setHiding] = useState(false);
  const [responsePage, setResponsePage] = useState(0);
  const panelRef = useRef<HTMLElement>(null);
  const pending = useRef(false);
  const [voiceLevel, setVoiceLevel] = useState(0);
  const voiceTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const processing = [
    "transcribing",
    "processing",
    "thinking",
    "executing",
  ].includes(state);
  const speaking = state === "listening" && voiceLevel > 0;
  const copy = stateCopy[state];

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel || !isTauri()) return;
    let active = true;
    let frame = 0;
    let lastHeight = 0;
    let requestedHeight = 0;
    let resizing = false;
    const resize = async () => {
      if (resizing) return;
      resizing = true;
      while (active && lastHeight !== requestedHeight) {
        lastHeight = requestedHeight;
        try {
          await invoke("resize_assistant", { height: lastHeight });
        } catch (error) {
          console.error("[NOVA] Could not expand assistant.", error);
          break;
        }
      }
      resizing = false;
    };
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        requestedHeight = Math.ceil(panel.getBoundingClientRect().height);
        void resize();
      });
    });
    observer.observe(panel);
    return () => {
      active = false;
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);

  const dismiss = useCallback(async () => {
    if (pending.current) return;
    pending.current = true;
    setHiding(true);
    setDismissError(null);
    try {
      await hideAssistant();
    } catch {
      setDismissError("Could not hide NOVA. Please try Dismiss again.");
    } finally {
      pending.current = false;
      setHiding(false);
    }
  }, []);

  useEffect(() => {
    let active = true;
    const disposers: UnlistenFn[] = [];

    const subscribe = async () => {
      try {
        const stateUnlisten = await listen<AssistantState>(
          "assistant-state",
          (event) => {
            if (active && event.payload in stateCopy) {
              setState(event.payload);
              if (event.payload !== "listening") setVoiceLevel(0);
            }
          },
        );
        if (!active) stateUnlisten();
        else disposers.push(stateUnlisten);

        const voiceUnlisten = await listen<number>(
          "assistant-voice-level",
          (event) => {
            if (!active) return;
            const level = Number.isFinite(event.payload)
              ? Math.max(0, Math.min(1, event.payload))
              : 0;
            setVoiceLevel(level);
            if (voiceTimeout.current) clearTimeout(voiceTimeout.current);
            voiceTimeout.current = setTimeout(() => {
              if (active) setVoiceLevel(0);
            }, 180);
          },
        );
        if (!active) voiceUnlisten();
        else disposers.push(voiceUnlisten);

        const transcriptUnlisten = await listen<string | null>(
          "assistant-transcript",
          (event) => {
            if (active) setTranscript(event.payload);
          },
        );
        if (!active) transcriptUnlisten();
        else disposers.push(transcriptUnlisten);

        const errorUnlisten = await listen<string | null>(
          "assistant-error",
          (event) => {
            if (active) setVoiceError(event.payload);
          },
        );
        if (!active) errorUnlisten();
        else disposers.push(errorUnlisten);
      } catch (listenError: unknown) {
        console.error(
          "[NOVA] Could not subscribe to assistant events.",
          listenError,
        );
      }
    };
    void subscribe();

    return () => {
      active = false;
      disposers.forEach((dispose) => dispose());
      if (voiceTimeout.current) clearTimeout(voiceTimeout.current);
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void dismiss();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dismiss]);

  const response = voiceError ?? transcript ?? copy.heading;
  // Very long responses use pages instead of overflowing the monitor or scrolling.
  const pageLength = Math.max(
    120,
    Math.min(600, Math.floor((window.screen.availHeight - 200) / 24) * 28),
  );
  const responsePages: string[] = [];
  let remaining = response;
  while (remaining.length > pageLength) {
    const boundary = remaining.lastIndexOf(" ", pageLength);
    const cut = boundary > pageLength / 2 ? boundary : pageLength;
    responsePages.push(remaining.slice(0, cut));
    remaining = remaining.slice(cut).trimStart();
  }
  responsePages.push(remaining);
  const currentPage = Math.min(responsePage, responsePages.length - 1);
  const heading = voiceError ? copy.heading : responsePages[currentPage];
  useEffect(() => {
    setResponsePage(0);
  }, [response]);
  const note =
    state === "idle"
      ? "Say Hey NOVA · Alt + N"
      : state === "listening"
        ? speaking
          ? "I hear you"
          : "Go ahead, I'm listening"
        : processing
          ? "Working on it, on your device"
          : state === "success"
            ? "Ready for what's next"
            : "Try your command again";

  return (
    <main
      ref={panelRef}
      className="assistant-panel"
      data-state={state}
      data-processing={processing}
      data-speaking={speaking}
      aria-labelledby="assistant-heading"
    >
      <header className="overlay-header">
        <div className="overlay-brand">
          <img src={novaLogo} alt="" /> NOVA
        </div>
        <span className="overlay-status" role="status">
          <span aria-hidden="true" />
          {copy.label}
        </span>
        <button
          type="button"
          onClick={() => void dismiss()}
          disabled={hiding}
          aria-label="Dismiss assistant"
          title="Dismiss (Esc)"
        >
          &times;
        </button>
      </header>
      <div className="overlay-body">
        <div
          className="voice-orb"
          aria-hidden="true"
          style={
            { "--voice-level": speaking ? voiceLevel : 0 } as CSSProperties
          }
        >
          <div className="voice-core">
            {state === "listening" ? (
              <div className="voice-equalizer">
                {[0, 1, 2, 3].map((bar) => (
                  <span key={bar} style={{ "--bar": bar } as CSSProperties} />
                ))}
              </div>
            ) : (
              <Icon name="skills" />
            )}
          </div>
        </div>
        <div className="overlay-copy">
          <h1 id="assistant-heading">{heading}</h1>
          <p className="overlay-note">{note}</p>
        </div>
      </div>
      {(voiceError || dismissError) && (
        <p className="overlay-error" role="alert">
          {voiceError ? responsePages[currentPage] : dismissError}
        </p>
      )}
      {responsePages.length > 1 && (
        <nav className="response-pages" aria-label="Response pages">
          <button
            disabled={currentPage === 0}
            onClick={() => setResponsePage(currentPage - 1)}
            aria-label="Previous response page"
          >
            Previous
          </button>
          <span aria-live="polite">
            {currentPage + 1} / {responsePages.length}
          </span>
          <button
            disabled={currentPage === responsePages.length - 1}
            onClick={() => setResponsePage(currentPage + 1)}
            aria-label="Next response page"
          >
            Next
          </button>
        </nav>
      )}
    </main>
  );
}
