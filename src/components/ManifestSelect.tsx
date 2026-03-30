import { useState, useEffect } from "react";
import { useManifestList } from "../hooks/useManifestList";
import type { GameInfo } from "../types/game";

interface ManifestSelectProps {
  selectedGame: GameInfo;
  selectedManifestId: string | null;
  onSelectManifest: (manifestId: string) => void;
}

export function ManifestSelect({
  selectedGame,
  selectedManifestId,
  onSelectManifest,
}: ManifestSelectProps) {
  const { manifests, loading, error, fetch } = useManifestList();
  const [manualId, setManualId] = useState("");
  const [useManual, setUseManual] = useState(false);

  const depot = selectedGame.depots[0];
  const depotId = depot?.depot_id ?? "";

  // Auto-fetch manifests on mount (credentials are already in AuthStore from step 2)
  useEffect(() => {
    if (depotId) {
      fetch(selectedGame.appid, depotId);
    }
  }, [selectedGame.appid, depotId, fetch]);

  const handleRetry = () => {
    if (depotId) {
      fetch(selectedGame.appid, depotId);
    }
  };

  const handleManualSubmit = () => {
    const trimmed = manualId.trim();
    if (trimmed) {
      onSelectManifest(trimmed);
    }
  };

  // Manual input mode
  if (useManual) {
    return (
      <section className="manifest-select">
        <h2 className="step-view__title">Select Version</h2>
        <p className="step-view__description">
          Enter the target manifest ID from SteamDB for{" "}
          <strong>{selectedGame.name}</strong>.
        </p>

        <div className="manifest-select__manual">
          <div className="manifest-select__field">
            <label className="manifest-select__label" htmlFor="manual-manifest">
              Manifest ID
            </label>
            <input
              id="manual-manifest"
              className="manifest-select__input"
              type="text"
              value={manualId}
              onChange={(e) => setManualId(e.target.value)}
              placeholder="e.g. 7446650175280810671"
            />
          </div>
          <div className="manifest-select__actions">
            <button
              className="manifest-select__button"
              onClick={handleManualSubmit}
              disabled={!manualId.trim()}
            >
              Use This Manifest
            </button>
            <button
              className="manifest-select__link-button"
              onClick={() => setUseManual(false)}
            >
              Browse available versions instead
            </button>
          </div>
        </div>
      </section>
    );
  }

  // Loading state
  if (loading) {
    return (
      <section className="manifest-select">
        <h2 className="step-view__title">Select Version</h2>
        <p className="manifest-select__loading">
          Fetching available versions for {selectedGame.name}...
        </p>
      </section>
    );
  }

  // Error state
  if (error) {
    return (
      <section className="manifest-select">
        <h2 className="step-view__title">Select Version</h2>
        <div className="manifest-select__error">
          <p className="manifest-select__error-message">{error}</p>
          <div className="manifest-select__actions">
            <button
              className="manifest-select__retry-button"
              onClick={handleRetry}
            >
              Retry
            </button>
            <button
              className="manifest-select__link-button"
              onClick={() => setUseManual(true)}
            >
              Enter manifest ID manually instead
            </button>
          </div>
        </div>
      </section>
    );
  }

  // Empty state
  if (manifests.length === 0) {
    return (
      <section className="manifest-select">
        <h2 className="step-view__title">Select Version</h2>
        <p className="manifest-select__empty">
          No manifests found for depot {depotId}.
        </p>
        <div className="manifest-select__actions">
          <button
            className="manifest-select__link-button"
            onClick={() => setUseManual(true)}
          >
            Enter manifest ID manually instead
          </button>
        </div>
      </section>
    );
  }

  // Manifest list
  return (
    <section className="manifest-select">
      <h2 className="step-view__title">Select Version</h2>
      <p className="step-view__description">
        Choose a target version for <strong>{selectedGame.name}</strong> (depot{" "}
        {depotId}).
      </p>
      <div className="manifest-list">
        {manifests.map((entry) => (
          <button
            key={entry.manifest_id}
            className={`manifest-row${selectedManifestId === entry.manifest_id ? " manifest-row--selected" : ""}`}
            onClick={() => onSelectManifest(entry.manifest_id)}
            type="button"
          >
            <span className="manifest-row__date">{entry.date}</span>
            <span className="manifest-row__id">{entry.manifest_id}</span>
          </button>
        ))}
      </div>
      <div className="manifest-select__actions">
        <button
          className="manifest-select__link-button"
          onClick={() => setUseManual(true)}
        >
          Enter manifest ID manually instead
        </button>
      </div>
    </section>
  );
}
