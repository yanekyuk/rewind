import { useState, useEffect, useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { SteamDepotInfo } from "../types/game";

type InvokeFn = typeof tauriInvoke;

interface UseDepotListResult {
  depots: SteamDepotInfo[];
  loading: boolean;
  error: string | null;
  fetch: (appId: string) => void;
}

export function useDepotList(invoke: InvokeFn = tauriInvoke): UseDepotListResult {
  const [depots, setDepots] = useState<SteamDepotInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [appId, setAppId] = useState<string | null>(null);

  const fetch = useCallback((id: string) => {
    setAppId(id);
    setLoading(true);
    setError(null);
    setDepots([]);
  }, []);

  useEffect(() => {
    if (!appId) return;

    let cancelled = false;

    invoke<SteamDepotInfo[]>("list_depots", { appId })
      .then((result) => {
        if (!cancelled) {
          setDepots(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          const message =
            err instanceof Error ? err.message : String(err);
          setError(message);
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [appId, invoke]);

  return { depots, loading, error, fetch };
}
