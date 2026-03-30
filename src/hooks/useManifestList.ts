import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ManifestListEntry } from "../types/manifest";

interface UseManifestListResult {
  manifests: ManifestListEntry[];
  loading: boolean;
  error: string | null;
  fetch: (appId: string, depotId: string) => void;
}

export function useManifestList(): UseManifestListResult {
  const [manifests, setManifests] = useState<ManifestListEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [params, setParams] = useState<{
    appId: string;
    depotId: string;
  } | null>(null);

  const fetch = useCallback((appId: string, depotId: string) => {
    setParams({ appId, depotId });
    setLoading(true);
    setError(null);
    setManifests([]);
  }, []);

  useEffect(() => {
    if (!params) return;

    let cancelled = false;

    invoke<ManifestListEntry[]>("list_manifests", {
      appId: params.appId,
      depotId: params.depotId,
    })
      .then((result) => {
        if (!cancelled) {
          setManifests(result);
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
  }, [params]);

  return { manifests, loading, error, fetch };
}
