import { useState, type FormEvent } from "react";
import { useAuth } from "../hooks/useAuth";

export function AuthInput() {
  const { checking, authenticated, submitting, error, submit } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [guardCode, setGuardCode] = useState("");
  const [showGuardCode, setShowGuardCode] = useState(false);

  if (checking) {
    return (
      <section className="auth-input">
        <h2 className="step-view__title">Steam Authentication</h2>
        <p className="auth-input__checking">Checking authentication status...</p>
      </section>
    );
  }

  if (authenticated) {
    return (
      <section className="auth-input">
        <h2 className="step-view__title">Steam Authentication</h2>
        <p className="auth-input__authenticated">
          Authenticated. You can proceed to the next step.
        </p>
      </section>
    );
  }

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    submit(username, password, showGuardCode ? guardCode || undefined : undefined);
  };

  return (
    <section className="auth-input">
      <h2 className="step-view__title">Steam Authentication</h2>
      <p className="step-view__description">
        Enter your Steam account details. Your password is never stored by
        Rewind.
      </p>

      <form
        className="auth-input__form"
        onSubmit={handleSubmit}
        role="form"
        aria-label="Steam authentication"
      >
        <div className="auth-input__field">
          <label htmlFor="auth-username" className="auth-input__label">
            Username
          </label>
          <input
            id="auth-username"
            className="auth-input__input"
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoComplete="username"
            required
          />
        </div>

        <div className="auth-input__field">
          <label htmlFor="auth-password" className="auth-input__label">
            Password
          </label>
          <input
            id="auth-password"
            className="auth-input__input"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            required
          />
        </div>

        <button
          type="button"
          className="auth-input__guard-toggle"
          onClick={() => setShowGuardCode((v) => !v)}
        >
          {showGuardCode ? "Hide Steam Guard code" : "Enter Steam Guard code"}
        </button>

        {showGuardCode && (
          <div className="auth-input__field">
            <label htmlFor="auth-guard-code" className="auth-input__label">
              Steam Guard Code
            </label>
            <input
              id="auth-guard-code"
              className="auth-input__input"
              type="text"
              value={guardCode}
              onChange={(e) => setGuardCode(e.target.value)}
              autoComplete="one-time-code"
              placeholder="e.g. ABC12"
            />
          </div>
        )}

        {error && (
          <p className="auth-input__error" role="alert">
            {error}
          </p>
        )}

        <button
          type="submit"
          className="auth-input__submit"
          disabled={submitting}
        >
          {submitting ? "Signing in..." : "Sign In"}
        </button>
      </form>
    </section>
  );
}
