import { navigationItems, type NovaPage } from "../../types/navigation";
import type { NovaStatus } from "../../types/nova";
import novaLogo from "../../assets/Nova-logo.png";

import { Icon } from "../ui/Icon";

interface SidebarProps {
  status: NovaStatus;
  page: NovaPage;
  onNavigate: (page: NovaPage) => void;
}

export function Sidebar({ status, page, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar" aria-label="NOVA workspace">
      <div className="brand">
        <img className="brand-mark" src={novaLogo} alt="" />
        <span>NOVA</span>
      </div>
      <nav aria-label="Main navigation">
        {navigationItems.map((item) => (
          <button
            key={item.id}
            type="button"
            className="nav-item"
            aria-current={page === item.id ? "page" : undefined}
            onClick={() => onNavigate(item.id)}
          >
            <span className="nav-icon">
              <Icon name={item.id} />
            </span>
            {item.label}
          </button>
        ))}
      </nav>
      <footer className="sidebar-footer">
        <span className="local-status">
          <span className="status-dot" aria-hidden="true" />
          on-device only
        </span>
        <span className="version">v{status.version}</span>
      </footer>
    </aside>
  );
}
