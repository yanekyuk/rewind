import { useCallback, useState } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

interface UseStartDowngradeParams {
  app_id: string;
  depot_id: string;
  target_manifest_id: string;
  current_manifest_id: string;
  latest_buildid: string;
  latest_manifest_id: string;
  latest_size: string;
  install_path: string;
  steamapps_path: string;
}

interface UseStartDowngradeResult {
  starting: boolean;
  error: string | null;
  start: (params: UseStartDowngradeParams) => Promise<void>;
}

/**
 * Hook to start a downgrade operation via the start_downgrade IPC command.
 *
 * Call the returned `start` function with the required parameters.
 * The hook will:
 * 1. Call start_downgrade via Tauri
 * 2. Automatically listen to downgrade-progress events (handled by useDowngradeProgress hook)
 * 3. Track loading and error states
 */
export function useStartDowngrade(): UseStartDowngradeResult {
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = useCallback(async (params: UseStartDowngradeParams) => {
    setStarting(true);
    setError(null);

    try {
      await tauriInvoke<void>("start_downgrade", params as unknown as Record<string, unknown>);
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStarting(false);
    }
  }, []);

  return {
    starting,
    error,
    start,
  };
}
