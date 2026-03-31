import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SteamDBManifest } from "../types/steamdb";

interface UseSteamDBWebviewResult {
  manifests: SteamDBManifest[];
  loading: boolean;
  error: string | null;
  open: (depotId: string) => Promise<void>;
  close: (depotId: string) => Promise<void>;
}

/**
 * Hook to manage a SteamDB webview window and receive extracted manifests.
 *
 * Opens a separate Tauri window pointing to SteamDB's depot manifests page.
 * Injected JavaScript extracts the manifest history table and emits data
 * back via Tauri events.
 */
export function useSteamDBWebview(): UseSteamDBWebviewResult {
  const [manifests, setManifests] = useState<SteamDBManifest[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unlistenManifests = listen<SteamDBManifest[]>(
      "steamdb-manifests",
      (event) => {
        setManifests(event.payload);
        setLoading(false);
        setError(null);
      },
    );

    const unlistenError = listen<string>("steamdb-manifests-error", (event) => {
      setError(event.payload);
      setLoading(false);
    });

    return () => {
      unlistenManifests.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  const open = useCallback(async (depotId: string) => {
    setLoading(true);
    setError(null);
    setManifests([]);
    try {
      await invoke("open_steamdb_webview", { depotId });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setLoading(false);
    }
  }, []);

  const close = useCallback(async (depotId: string) => {
    try {
      await invoke("close_steamdb_webview", { depotId });
    } catch {
      // Ignore close errors — window may already be gone
    }
  }, []);

  return { manifests, loading, error, open, close };
}
