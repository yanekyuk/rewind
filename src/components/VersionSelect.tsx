import { useEffect, useState, useCallback } from "react";
import { useManifestList } from "../hooks/useManifestList";
import { useStartDowngrade } from "../hooks/useStartDowngrade";
import type { GameInfo } from "../types/game";
import { Lock } from "lucide-react";

interface VersionSelectProps {
  game: GameInfo;
  depotId: string | null;
  selectedManifestId: string | null;
  onSelectManifest: (manifestId: string) => void;
  onAuthRequired?: () => void;
}

function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/** Derive the steamapps directory from a game's install_path (steamapps/common/<dir>). */
function getSteamappsPath(installPath: string): string {
  const idx = installPath.toLowerCase().indexOf("steamapps");
  if (idx === -1) return installPath;
  return installPath.slice(0, idx + "steamapps".length);
}

export function VersionSelect({
  game,
  depotId: depotIdProp,
  selectedManifestId,
  onSelectManifest,
  onAuthRequired,
}: VersionSelectProps) {
  const { manifests, loading, error, fetch } = useManifestList(undefined, {
    onAuthRequired,
  });
  const { starting: startingDowngrade, start: startDowngrade } = useStartDowngrade();
  const [manualId, setManualId] = useState("");

  // Use the provided depot ID, falling back to the first installed depot
  const depotId = depotIdProp ?? game.depots[0]?.depot_id ?? "";
  const installedDepot = game.depots.find((d) => d.depot_id === depotId);
  const currentManifestId = installedDepot?.manifest ?? "Unknown";

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

  const buildDowngradeParams = useCallback((targetManifestId: string) => ({
    app_id: game.appid,
    depot_id: depotId,
    target_manifest_id: targetManifestId,
    current_manifest_id: currentManifestId,
    latest_buildid: game.buildid,
    latest_manifest_id: installedDepot?.manifest ?? "",
    latest_size: installedDepot?.size ?? "0",
    install_path: game.install_path,
    steamapps_path: getSteamappsPath(game.install_path),
  }), [game, depotId, currentManifestId, installedDepot]);

  const handleManualSubmit = useCallback(async () => {
    const trimmed = manualId.trim();
    if (trimmed) {
      await startDowngrade(buildDowngradeParams(trimmed));
      onSelectManifest(trimmed);
    }
  }, [manualId, buildDowngradeParams, onSelectManifest, startDowngrade]);

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
            {manifests.map((entry) => {
              const isCurrent = entry.manifest_id === currentManifestId;
              const isSelected = selectedManifestId === entry.manifest_id;
              const classes = [
                "version-row",
                isSelected ? "version-row--selected" : "",
                isCurrent ? "version-row--current" : "",
              ]
                .filter(Boolean)
                .join(" ");

              const handleRowClick = async () => {
                await startDowngrade(buildDowngradeParams(entry.manifest_id));
                onSelectManifest(entry.manifest_id);
              };

              return (
                <button
                  key={entry.manifest_id}
                  className={classes}
                  onClick={handleRowClick}
                  type="button"
                  disabled={startingDowngrade}
                >
                  <div className="version-row__info">
                    <span className="version-row__branch">
                      {entry.branch ?? "unknown"}
                      {isCurrent && (
                        <span className="version-row__current-badge">
                          current
                        </span>
                      )}
                      {entry.pwd_required && (
                        <span className="version-row__pwd">
                          <Lock size={12} />
                          <span>Password required</span>
                        </span>
                      )}
                    </span>
                    {entry.time_updated != null && (
                      <span className="version-row__time">
                        {formatTimestamp(entry.time_updated)}
                      </span>
                    )}
                  </div>
                  <span className="version-row__id">{entry.manifest_id}</span>
                </button>
              );
            })}
          </div>
        )}

        <div className="version-select__manual">
          <h2 className="version-select__section-title">Manual Entry</h2>
          <p className="version-select__manual-hint">
            Enter a manifest ID directly if you know it from SteamDB or a
            community guide.
          </p>
          <div className="version-select__manual-row">
            <input
              className="version-select__manual-input"
              type="text"
              placeholder="Enter manifest ID"
              value={manualId}
              onChange={(e) => setManualId(e.target.value)}
            />
            <button
              className="version-select__manual-button"
              type="button"
              onClick={handleManualSubmit}
              disabled={!manualId.trim()}
            >
              Use
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
