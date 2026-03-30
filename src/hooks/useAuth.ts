import { useState, useEffect, useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

type InvokeFn = typeof tauriInvoke;

export interface UseAuthResult {
  /** Whether we are checking existing auth state on mount. */
  checking: boolean;
  /** Whether credentials have been successfully stored. */
  authenticated: boolean;
  /** Whether a submission is in progress. */
  submitting: boolean;
  /** Error message from the last failed submission, if any. */
  error: string | null;
  /** Submit credentials to the backend. */
  submit: (
    username: string,
    password: string,
    guardCode?: string,
  ) => Promise<void>;
  /** Clear credentials from memory and the OS keychain. */
  signOut: () => Promise<void>;
}

export function useAuth(invoke: InvokeFn = tauriInvoke): UseAuthResult {
  const [checking, setChecking] = useState(true);
  const [authenticated, setAuthenticated] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check if credentials are already stored (e.g. from earlier in the session)
  useEffect(() => {
    let cancelled = false;

    invoke<boolean>("get_auth_state")
      .then((isSet) => {
        if (!cancelled) {
          setAuthenticated(isSet);
          setChecking(false);
        }
      })
      .catch(() => {
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
        await invoke("set_credentials", {
          username,
          password,
          guardCode: guardCode ?? null,
        });
        setAuthenticated(true);
      } catch (err) {
        const message =
          err instanceof Error
            ? err.message
            : typeof err === "string"
              ? err
              : typeof err === "object" && err !== null
                ? String(Object.values(err as Record<string, unknown>)[0] ?? JSON.stringify(err))
                : String(err);
        setError(message);
      } finally {
        setSubmitting(false);
      }
    },
    [invoke],
  );

  const signOut = useCallback(async () => {
    await invoke("clear_credentials");
    setAuthenticated(false);
  }, [invoke]);

  return { checking, authenticated, submitting, error, submit, signOut };
}
