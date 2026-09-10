import { useEffect, useRef, type ReactNode } from "react";
import { TitleBar } from "./TitleBar";
import { Sidebar } from "./Sidebar";
import type { NovaPage } from "../../types/navigation";
import type { NovaStatus } from "../../types/nova";

interface AppShellProps {
  children: ReactNode;
  status: NovaStatus;
  page: NovaPage;
  onNavigate: (page: NovaPage) => void;
}

export function AppShell({ children, status, page, onNavigate }: AppShellProps) {
  const mainRef = useRef<HTMLElement>(null);
  const previousPage = useRef(page);

  useEffect(() => {
    if (previousPage.current !== page) {
      mainRef.current?.scrollTo({ top: 0 });
      mainRef.current?.querySelector<HTMLHeadingElement>("h1")?.focus({ preventScroll: true });
      previousPage.current = page;
    }
  }, [page]);

  return (
    <div className="app-shell">
      <TitleBar />
      <a className="skip-link" href="#page-title">Skip to content</a>
      <div className="workspace">
        <Sidebar status={status} page={page} onNavigate={onNavigate} />
        <main id="main-content" ref={mainRef} aria-labelledby="page-title">
          <div className="page-content" key={page}>{children}</div>
        </main>
      </div>
    </div>
  );
}
