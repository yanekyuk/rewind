import { useCallback, useEffect, useState } from "react";
import { CheckCircle, AlertCircle, RotateCcw } from "lucide-react";
import { useDowngradeProgress } from "../hooks/useDowngradeProgress";
import type { GameInfo } from "../types/game";
import "./DowngradeProgress.css";

interface DowngradeProgressProps {
  game: GameInfo;
  targetManifestId: string;
  onComplete: () => void;
  onError?: (error: string) => void;
}

/**
 * Displays real-time progress of the downgrade operation.
 * Listens to downgrade-progress Tauri events and updates UI accordingly.
 */
export function DowngradeProgress({
  game,
  targetManifestId,
  onComplete,
  onError,
}: DowngradeProgressProps) {
  const progress = useDowngradeProgress();
  const [retrying, setRetrying] = useState(false);

  // Notify parent on completion
  useEffect(() => {
    if (progress.phase === "complete") {
      onComplete();
    }
  }, [progress.phase, onComplete]);

  // Notify parent on error
  useEffect(() => {
    if (progress.phase === "error" && progress.error && onError) {
      onError(progress.error);
    }
  }, [progress.phase, progress.error, onError]);

  const handleRetry = useCallback(() => {
    setRetrying(true);
    // In a real implementation, this would trigger a re-call to start_downgrade
    // For now, just reset the UI
    window.location.reload();
  }, []);

  const handleBack = useCallback(() => {
    onComplete();
  }, [onComplete]);

  const formatBytes = (bytes: number): string => {
    const GB = 1073741824;
    const MB = 1048576;
    if (bytes >= GB) return `${(bytes / GB).toFixed(1)} GB`;
    if (bytes >= MB) return `${(bytes / MB).toFixed(1)} MB`;
    return `${bytes} B`;
  };

  return (
    <div className="downgrade-progress">
      <h1 className="downgrade-progress__title">
        Downgrading {game.name}
      </h1>

      <div className="downgrade-progress__container">
        {progress.phase === "comparing" && (
          <>
            <div className="downgrade-progress__icon downgrade-progress__icon--spinner" />
            <h2 className="downgrade-progress__phase-title">
              Comparing manifests...
            </h2>
            <p className="downgrade-progress__phase-description">
              Fetching version information and calculating differences.
            </p>
          </>
        )}

        {progress.phase === "downloading" && (
          <>
            <div className="downgrade-progress__icon downgrade-progress__icon--download" />
            <h2 className="downgrade-progress__phase-title">
              Downloading files ({progress.percent ?? 0}%)
            </h2>

            <div className="downgrade-progress__progress-bar">
              <div
                className="downgrade-progress__progress-fill"
                style={{
                  width: `${progress.percent ?? 0}%`,
                }}
              />
            </div>

            <div className="downgrade-progress__metrics">
              {progress.bytesDownloaded !== undefined &&
                progress.bytesTotal !== undefined && (
                  <p className="downgrade-progress__metric">
                    {formatBytes(progress.bytesDownloaded)} /{" "}
                    {formatBytes(progress.bytesTotal)}
                  </p>
                )}

              {progress.speed && (
                <p className="downgrade-progress__metric">
                  Speed: {progress.speed}
                </p>
              )}

              {progress.eta && (
                <p className="downgrade-progress__metric">ETA: {progress.eta}</p>
              )}
            </div>
          </>
        )}

        {progress.phase === "applying" && (
          <>
            <div className="downgrade-progress__icon downgrade-progress__icon--spinner" />
            <h2 className="downgrade-progress__phase-title">
              Applying files...
            </h2>
            <p className="downgrade-progress__phase-description">
              Copying files, patching ACF, and updating manifest lock.
            </p>
          </>
        )}

        {progress.phase === "complete" && (
          <>
            <div className="downgrade-progress__icon downgrade-progress__icon--success">
              <CheckCircle size={64} />
            </div>
            <h2 className="downgrade-progress__phase-title">
              Downgrade Complete
            </h2>
            <p className="downgrade-progress__phase-description">
              Successfully downgraded {game.name} to manifest {targetManifestId}
            </p>

            <div className="downgrade-progress__warning">
              <AlertCircle size={20} />
              <div className="downgrade-progress__warning-content">
                <strong>Important:</strong>
                <p>
                  Set Steam's update preference to "Only update when I launch"
                  to prevent automatic updates.
                </p>
              </div>
            </div>
          </>
        )}

        {progress.phase === "error" && (
          <>
            <div className="downgrade-progress__icon downgrade-progress__icon--error">
              <AlertCircle size={64} />
            </div>
            <h2 className="downgrade-progress__phase-title">
              Downgrade Failed
            </h2>
            {progress.error && (
              <p className="downgrade-progress__error-message">
                {progress.error}
              </p>
            )}
          </>
        )}
      </div>

      <div className="downgrade-progress__actions">
        {progress.phase === "comparing" || progress.phase === "downloading" || progress.phase === "applying" ? (
          <button
            className="downgrade-progress__button downgrade-progress__button--cancel"
            onClick={handleBack}
            type="button"
          >
            Cancel
          </button>
        ) : progress.phase === "complete" ? (
          <>
            <button
              className="downgrade-progress__button downgrade-progress__button--primary"
              onClick={handleBack}
              type="button"
            >
              Return to Game
            </button>
            <button
              className="downgrade-progress__button downgrade-progress__button--secondary"
              onClick={handleBack}
              type="button"
            >
              Back
            </button>
          </>
        ) : progress.phase === "error" ? (
          <>
            <button
              className="downgrade-progress__button downgrade-progress__button--primary"
              onClick={handleRetry}
              disabled={retrying}
              type="button"
            >
              <RotateCcw size={16} /> Retry
            </button>
            <button
              className="downgrade-progress__button downgrade-progress__button--secondary"
              onClick={handleBack}
              type="button"
            >
              Back
            </button>
          </>
        ) : null}
      </div>
    </div>
  );
}
