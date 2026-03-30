import { useEffect } from "react";
import { useManifestList } from "../hooks/useManifestList";
import type { GameInfo } from "../types/game";

interface VersionSelectProps {
  game: GameInfo;
  selectedManifestId: string | null;
  onSelectManifest: (manifestId: string) => void;
}

export function VersionSelect({
  game,
  selectedManifestId,
  onSelectManifest,
}: VersionSelectProps) {
  const { manifests, loading, error, fetch } = useManifestList();

  const depot = game.depots[0];
  const depotId = depot?.depot_id ?? "";
  const currentManifestId = depot?.manifest ?? "Unknown";

  useEffect(() => {
    if (depotId) {
      fetch(game.appid, depotId);
    }
  }, [game.appid, depotId, fetch]);

  const handleRetry = () => {
    if (depotId) {
      fetch(game.appid, depotId);
    }
  };

  return (
    <div className="version-select">
      <h1 className="version-select__title">Change Version</h1>
      <p className="version-select__subtitle">{game.name}</p>

      <div className="version-select__current">
        <h2 className="version-select__section-title">Current Version</h2>
        <div className="version-select__info-grid">
          <div className="version-select__info-item">
            <span className="version-select__info-label">Build ID</span>
            <span className="version-select__info-value">{game.buildid}</span>
          </div>
          <div className="version-select__info-item">
            <span className="version-select__info-label">Manifest ID</span>
            <span className="version-select__info-value">
              {currentManifestId}
            </span>
          </div>
        </div>
      </div>

      <div className="version-select__available">
        <h2 className="version-select__section-title">Available Versions</h2>

        {loading && (
          <p className="version-select__loading">Loading available versions...</p>
        )}

        {error && (
          <div className="version-select__error">
            <p className="version-select__error-message">{error}</p>
            <button
              className="version-select__retry"
              onClick={handleRetry}
              type="button"
            >
              Retry
            </button>
          </div>
        )}

        {!loading && !error && manifests.length === 0 && (
          <p className="version-select__empty">No versions found.</p>
        )}

        {!loading && !error && manifests.length > 0 && (
          <div className="version-select__list">
            {manifests.map((entry) => (
              <button
                key={entry.manifest_id}
                className={`version-row${selectedManifestId === entry.manifest_id ? " version-row--selected" : ""}`}
                onClick={() => onSelectManifest(entry.manifest_id)}
                type="button"
              >
                <span className="version-row__date">{entry.date}</span>
                <span className="version-row__id">{entry.manifest_id}</span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
