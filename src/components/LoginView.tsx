import { useState, type FormEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Rewind, X } from "lucide-react";
import type { UseAuthResult } from "../hooks/useAuth";

interface LoginViewProps {
  auth: UseAuthResult;
}

export function LoginView({ auth }: LoginViewProps) {
  const { checking, submitting, error, submit, username, hasStoredCredentials, signOut } = auth;
  const [formUsername, setFormUsername] = useState("");
  const [formPassword, setFormPassword] = useState("");
  const [showFullForm, setShowFullForm] = useState(false);

  if (checking) {
    return (
      <div className="login-view" data-tauri-drag-region>
        <div className="login-view__card">
          <h1 className="login-view__brand"><Rewind size={32} /> Rewind</h1>
          <p className="login-view__checking">Checking authentication status...</p>
        </div>
      </div>
    );
  }

  if (submitting) {
    return (
      <div className="login-view" data-tauri-drag-region>
        <button
          className="login-view__close"
          onClick={() => getCurrentWindow().close()}
          type="button"
          title="Close"
        >
          <X size={20} />
        </button>
        <div className="login-view__card">
          <h1 className="login-view__brand"><Rewind size={32} /> Rewind</h1>
          <div className="login-view__guard">
            <div className="login-view__guard-icon" aria-hidden="true">
              <svg
                width="48"
                height="48"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
            </div>
            <p className="login-view__guard-text">
              Waiting for Steam Guard approval...
            </p>
            <p className="login-view__guard-hint">
              Check your Steam mobile app to approve the sign-in request.
            </p>
          </div>
        </div>
      </div>
    );
  }

  // "Welcome back" view: stored credentials found, no need to re-enter password
  if (hasStoredCredentials && username && !showFullForm) {
    const handleResumeSession = (e: FormEvent) => {
      e.preventDefault();
      // Submit with stored credentials — the backend already has them loaded
      // from the keychain. Pass empty password to signal session reuse via sidecar.
      submit(username, "", undefined);
    };

    const handleSignInAsDifferent = () => {
      signOut();
      setShowFullForm(true);
    };

    return (
      <div className="login-view" data-tauri-drag-region>
        <button
          className="login-view__close"
          onClick={() => getCurrentWindow().close()}
          type="button"
          title="Close"
        >
          <X size={20} />
        </button>
        <div className="login-view__card">
          <h1 className="login-view__brand"><Rewind size={32} /> Rewind</h1>

          <form
            className="login-view__form"
            onSubmit={handleResumeSession}
            role="form"
            aria-label="Resume session"
          >
            <p className="login-view__welcome">
              Welcome back, <strong>{username}</strong>
            </p>

            {error && (
              <p className="login-view__error" role="alert">
                {error}
              </p>
            )}

            <button type="submit" className="login-view__submit">
              Sign in
            </button>

            <button
              type="button"
              className="login-view__alt-action"
              onClick={handleSignInAsDifferent}
            >
              Sign in with a different account
            </button>
          </form>
        </div>
      </div>
    );
  }

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    submit(formUsername, formPassword, undefined);
  };

  return (
    <div className="login-view" data-tauri-drag-region>
      <button
        className="login-view__close"
        onClick={() => getCurrentWindow().close()}
        type="button"
        title="Close"
      >
        <X size={20} />
      </button>
      <div className="login-view__card">
        <h1 className="login-view__brand"><Rewind size={32} /> Rewind</h1>

        <form
          className="login-view__form"
          onSubmit={handleSubmit}
          role="form"
          aria-label="Steam authentication"
        >
          <div className="login-view__field">
            <label htmlFor="login-username" className="login-view__label">
              Sign in with account name
            </label>
            <input
              id="login-username"
              className="login-view__input"
              type="text"
              value={formUsername}
              onChange={(e) => setFormUsername(e.target.value)}
              autoComplete="username"
              required
            />
          </div>

          <div className="login-view__field">
            <label htmlFor="login-password" className="login-view__label">
              Password
            </label>
            <input
              id="login-password"
              className="login-view__input"
              type="password"
              value={formPassword}
              onChange={(e) => setFormPassword(e.target.value)}
              autoComplete="current-password"
              required
            />
          </div>

          {error && (
            <p className="login-view__error" role="alert">
              {error}
            </p>
          )}

          <button type="submit" className="login-view__submit">
            Sign in
          </button>
        </form>
      </div>
    </div>
  );
}
