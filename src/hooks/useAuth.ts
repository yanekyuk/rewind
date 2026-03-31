import { useState, useEffect, useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { extractErrorMessage } from "../utils/errors";

type InvokeFn = typeof tauriInvoke;

export interface UseAuthResult {
  /** Whether we are checking for a saved session on mount. */
  checking: boolean;
  /** Whether the sidecar has an active authenticated session. */
  authenticated: boolean;
  /** The Steam username of the authenticated user. */
  username: string | null;
  /** Whether a submission is in progress. */
  submitting: boolean;
  /** Error message from the last failed submission, if any. */
  error: string | null;
  /** Submit credentials to authenticate with Steam. */
  submit: (
    username: string,
    password: string,
    guardCode?: string,
  ) => Promise<void>;
  /** Sign out: clear the sidecar session and local auth state. */
  signOut: () => Promise<void>;
}

export function useAuth(invoke: InvokeFn = tauriInvoke): UseAuthResult {
  const [checking, setChecking] = useState(true);
  const [authenticated, setAuthenticated] = useState(false);
  const [username, setUsername] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // On mount, check for a saved sidecar session (RefreshToken on disk).
  // If found, the sidecar logs in silently and we skip the login screen.
  useEffect(() => {
    let cancelled = false;

    invoke<string | null>("check_session")
      .then((sessionUsername) => {
        if (!cancelled && sessionUsername) {
          setUsername(sessionUsername);
          setAuthenticated(true);
        }
      })
      .catch(() => {
        // No saved session -- user needs to log in
      })
      .finally(() => {
        if (!cancelled) {
          setChecking(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [invoke]);

  const submit = useCallback(
    async (username: string, password: string, guardCode?: string) => {
      setSubmitting(true);
      setError(null);

      try {
        await invoke("login", {
          username,
          password,
          guardCode: guardCode ?? null,
        });
        setUsername(username);
        setAuthenticated(true);
      } catch (err) {
        setError(extractErrorMessage(err));
      } finally {
        setSubmitting(false);
      }
    },
    [invoke],
  );

  const signOut = useCallback(async () => {
    await invoke("logout");
    setUsername(null);
    setAuthenticated(false);
  }, [invoke]);

  return { checking, authenticated, username, submitting, error, submit, signOut };
}
