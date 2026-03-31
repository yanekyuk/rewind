import { useState, useEffect, useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { extractErrorMessage } from "../utils/errors";

type InvokeFn = typeof tauriInvoke;

export interface UseAuthResult {
  /** Whether we are checking existing auth state on mount. */
  checking: boolean;
  /** Whether credentials have been successfully stored. */
  authenticated: boolean;
  /** The Steam username of the authenticated user. */
  username: string | null;
  /** Whether full credentials (username + password) are stored in the OS keychain. */
  hasStoredCredentials: boolean;
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
  /** Resume a session using credentials already stored in the backend (from keychain). */
  resumeSession: () => Promise<void>;
  /** Clear credentials from memory and the OS keychain. */
  signOut: () => Promise<void>;
}

export function useAuth(invoke: InvokeFn = tauriInvoke): UseAuthResult {
  const [checking, setChecking] = useState(true);
  const [authenticated, setAuthenticated] = useState(false);
  const [username, setUsername] = useState<string | null>(null);
  const [hasStoredCredentials, setHasStoredCredentials] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Check if credentials are already stored (e.g. from earlier in the session)
  useEffect(() => {
    let cancelled = false;

    invoke<boolean>("get_auth_state")
      .then(async (isSet) => {
        if (!cancelled) {
          setAuthenticated(isSet);
          if (isSet) {
            const [name, hasCreds] = await Promise.all([
              invoke<string | null>("get_username"),
              invoke<boolean>("has_credentials"),
            ]);
            if (!cancelled) {
              setUsername(name);
              setHasStoredCredentials(hasCreds);
            }
          }
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
        setUsername(username);
        setHasStoredCredentials(true);
        setAuthenticated(true);
      } catch (err) {
        setError(extractErrorMessage(err));
      } finally {
        setSubmitting(false);
      }
    },
    [invoke],
  );

  const resumeSession = useCallback(async () => {
    setSubmitting(true);
    setError(null);

    try {
      await invoke("resume_session");
      setAuthenticated(true);
    } catch (err) {
      setError(extractErrorMessage(err));
    } finally {
      setSubmitting(false);
    }
  }, [invoke]);

  const signOut = useCallback(async () => {
    await invoke("clear_credentials");
    setUsername(null);
    setHasStoredCredentials(false);
    setAuthenticated(false);
  }, [invoke]);

  return { checking, authenticated, username, hasStoredCredentials, submitting, error, submit, resumeSession, signOut };
}
