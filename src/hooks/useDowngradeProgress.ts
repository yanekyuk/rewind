import { useEffect, useRef, useState } from "react";
import { listen as tauriListen } from "@tauri-apps/api/event";
import type { DowngradeProgressEvent } from "../types/downgrade";

type ListenFn = typeof tauriListen;

interface UseDowngradeProgressResult {
  phase: "comparing" | "downloading" | "applying" | "complete" | "error" | null;
  percent?: number;
  bytesDownloaded?: number;
  bytesTotal?: number;
  eta?: string;
  speed?: string;
  error?: string;
  isActive: boolean;
}

export function useDowngradeProgress(listenOverride?: ListenFn): UseDowngradeProgressResult {
  const listenFn = listenOverride ?? tauriListen;
  const [phase, setPhase] = useState<UseDowngradeProgressResult["phase"]>(null);
  const [percent, setPercent] = useState<number>();
  const [bytesDownloaded, setBytesDownloaded] = useState<number>();
  const [bytesTotal, setBytesTotal] = useState<number>();
  const [eta, setEta] = useState<string>();
  const [speed, setSpeed] = useState<string>();
  const [error, setError] = useState<string>();

  const lastUpdateRef = useRef<{ timestamp: number; bytes: number } | null>(null);

  useEffect(() => {
    let unlistener: (() => void) | null = null;

    (async () => {
      unlistener = await listenFn<DowngradeProgressEvent>(
        "downgrade-progress",
        (event) => {
          const { phase: newPhase, percent: newPercent, bytes_downloaded, bytes_total, message } = event.payload;

          setPhase(newPhase);

          if (newPhase === "comparing") {
            setPercent(undefined);
            setBytesDownloaded(undefined);
            setBytesTotal(undefined);
            setSpeed(undefined);
            setEta(undefined);
            setError(undefined);
          } else if (newPhase === "downloading") {
            setPercent(newPercent);
            setBytesDownloaded(bytes_downloaded);
            setBytesTotal(bytes_total);

            // Calculate speed and ETA
            const now = Date.now();
            if (lastUpdateRef.current) {
              const timeDeltaSec = (now - lastUpdateRef.current.timestamp) / 1000;
              const bytesDelta = (bytes_downloaded ?? 0) - lastUpdateRef.current.bytes;
              if (timeDeltaSec > 0.5) {
                // Only calculate if > 0.5s has passed
                const speedMBs = bytesDelta / (1024 * 1024) / timeDeltaSec;
                setSpeed(`${speedMBs.toFixed(1)} MB/s`);

                // Calculate ETA
                if (speedMBs > 0) {
                  const remainingBytes = (bytes_total ?? 0) - (bytes_downloaded ?? 0);
                  const remainingSec = remainingBytes / (speedMBs * 1024 * 1024);
                  const remainingMin = Math.ceil(remainingSec / 60);
                  if (remainingMin > 0) {
                    setEta(`~${remainingMin} min`);
                  }
                }

                lastUpdateRef.current = { timestamp: now, bytes: bytes_downloaded ?? 0 };
              }
            } else {
              lastUpdateRef.current = { timestamp: now, bytes: bytes_downloaded ?? 0 };
            }

            setError(undefined);
          } else if (newPhase === "applying") {
            setPercent(undefined);
            setBytesDownloaded(undefined);
            setBytesTotal(undefined);
            setSpeed(undefined);
            setEta(undefined);
            setError(undefined);
          } else if (newPhase === "complete") {
            setPercent(undefined);
            setBytesDownloaded(undefined);
            setBytesTotal(undefined);
            setSpeed(undefined);
            setEta(undefined);
            setError(undefined);
          } else if (newPhase === "error") {
            setPercent(undefined);
            setBytesDownloaded(undefined);
            setBytesTotal(undefined);
            setSpeed(undefined);
            setEta(undefined);
            setError(message);
          }
        }
      );
    })();

    return () => {
      if (unlistener) {
        unlistener();
      }
    };
  }, []);

  return {
    phase,
    percent,
    bytesDownloaded,
    bytesTotal,
    eta,
    speed,
    error,
    isActive: phase !== null && phase !== "complete" && phase !== "error",
  };
}
