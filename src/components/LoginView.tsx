import { useState, type FormEvent } from "react";
import type { UseAuthResult } from "../hooks/useAuth";

interface LoginViewProps {
  auth: UseAuthResult;
}

export function LoginView({ auth }: LoginViewProps) {
  const { checking, submitting, error, submit } = auth;
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");

  if (checking) {
    return (
      <div className="login-view">
        <div className="login-view__card">
          <h1 className="login-view__brand">Rewind</h1>
          <p className="login-view__checking">Checking authentication status...</p>
        </div>
      </div>
    );
  }

  if (submitting) {
    return (
      <div className="login-view">
        <div className="login-view__card">
          <h1 className="login-view__brand">Rewind</h1>
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

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    submit(username, password, undefined);
  };

  return (
    <div className="login-view">
      <div className="login-view__card">
        <h1 className="login-view__brand">Rewind</h1>
        <p className="login-view__subtitle">Steam Game Downgrader</p>

        <form
          className="login-view__form"
          onSubmit={handleSubmit}
          role="form"
          aria-label="Steam authentication"
        >
          <div className="login-view__field">
            <label htmlFor="login-username" className="login-view__label">
              Username
            </label>
            <input
              id="login-username"
              className="login-view__input"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
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
              value={password}
              onChange={(e) => setPassword(e.target.value)}
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
            Sign In
          </button>
        </form>
      </div>
    </div>
  );
}
