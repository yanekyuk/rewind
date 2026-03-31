import type { ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, ArrowRight, Minus, Rewind, Square, User, X } from "lucide-react";

interface AppShellProps {
  username: string;
  canGoBack: boolean;
  onBack?: () => void;
  onLibrary: () => void;
  onSignOut: () => void;
  children: ReactNode;
}

export function AppShell({ username, canGoBack, onBack, onLibrary, onSignOut, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <header className="app-shell__header">
        <div className="app-shell__topbar" data-tauri-drag-region>
          <div className="app-shell__topbar-left">
            <Rewind size={14} />
            <h1 className="app-shell__title">Rewind</h1>
          </div>
          <div className="app-shell__topbar-right">
            <div className="app-shell__user-pill">
              <div className="app-shell__user-avatar">
                <User size={16} />
              </div>
              <span className="app-shell__username">{username}</span>
            </div>
            <div className="app-shell__window-controls">
              <button
                className="app-shell__window-btn"
                onClick={() => getCurrentWindow().minimize()}
                type="button"
                title="Minimize"
              >
                <Minus size={14} />
              </button>
              <button
                className="app-shell__window-btn"
                onClick={() => getCurrentWindow().toggleMaximize()}
                type="button"
                title="Maximize"
              >
                <Square size={12} />
              </button>
              <button
                className="app-shell__window-btn app-shell__window-btn--close"
                onClick={() => getCurrentWindow().close()}
                type="button"
                title="Close"
              >
                <X size={14} />
              </button>
            </div>
          </div>
        </div>
        <nav className="app-shell__nav">
          <div className="app-shell__nav-left">
            <button
              className="app-shell__nav-arrow"
              type="button"
              disabled={!canGoBack}
              title="Back"
              onClick={onBack}
            >
              <ArrowLeft size={18} />
            </button>
            <button className="app-shell__nav-arrow" type="button" disabled title="Forward">
              <ArrowRight size={18} />
            </button>
            <button className="app-shell__nav-tab app-shell__nav-tab--active" type="button" onClick={onLibrary}>
              Library
            </button>
          </div>
          <button
            className="app-shell__sign-out"
            onClick={onSignOut}
            type="button"
          >
            Sign Out
          </button>
        </nav>
      </header>
      <main className="app-shell__content">
        {children}
      </main>
    </div>
  );
}
