import { useState, useEffect, useCallback } from "react";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { GameInfo } from "../types/game";

type InvokeFn = typeof tauriInvoke;

interface UseGameListResult {
  games: GameInfo[];
  loading: boolean;
  error: string | null;
  retry: () => void;
}

export function useGameList(invoke: InvokeFn = tauriInvoke): UseGameListResult {
  const [games, setGames] = useState<GameInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [fetchKey, setFetchKey] = useState(0);

  const retry = useCallback(() => {
    setFetchKey((k) => k + 1);
    setLoading(true);
    setError(null);
    setGames([]);
  }, []);

  useEffect(() => {
    let cancelled = false;

    invoke<GameInfo[]>("list_games")
      .then((result) => {
        if (!cancelled) {
          setGames(result);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load games");
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [fetchKey, invoke]);

  return { games, loading, error, retry };
}
