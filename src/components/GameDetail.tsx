import { useEffect } from "react";
import { AlertCircle } from "lucide-react";
import { useDepotList } from "../hooks/useDepotList";
import type { GameInfo, SteamDepotInfo } from "../types/game";

function formatEpoch(epoch: string): string {
  const ts = Number(epoch);
  if (isNaN(ts) || ts === 0) return "Unknown";
  const d = new Date(ts * 1000);
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

interface GameDetailProps {
  game: GameInfo;
  onChangeVersion: (depotId: string) => void;
}

function steamHeroUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/library_hero.jpg`;
}

function steamLogoUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/logo.png`;
}

function stateLabel(flags: number): string {
  switch (flags) {
    case 4: return "Installed";
    case 1026: return "Update Required";
    case 6: return "Update Queued";
    default: return `State ${flags}`;
  }
}

export function GameDetail({ game, onChangeVersion }: GameDetailProps) {
  const { depots: steamDepots, fetch: fetchDepots } = useDepotList();

  useEffect(() => {
    fetchDepots(game.appid);
  }, [game.appid, fetchDepots]);

  const installedIds = new Set(game.depots.map((d) => d.depot_id));

  // Build merged depot list: installed depots enriched with Steam names,
  // then non-installed depots from Steam
  const mergedDepots = buildMergedDepots(game, steamDepots, installedIds);

  return (
    <div className="game-detail">
      <div className="game-detail__hero">
        <img
          className="game-detail__hero-bg"
          src={steamHeroUrl(game.appid)}
          alt=""
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />
        <div className="game-detail__hero-gradient" />
        <img
          className="game-detail__hero-logo"
          src={steamLogoUrl(game.appid)}
          alt={game.name}
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />

        <div className="game-detail__info-bar">
          <button
            className="game-detail__change-version"
            onClick={() => {
              const firstDepot = game.depots[0];
              if (firstDepot) onChangeVersion(firstDepot.depot_id);
            }}
            type="button"
          >
            Change Version
          </button>

          <div className="game-detail__meta">
            <div className="game-detail__meta-item">
              <span className="game-detail__meta-label">Status</span>
              <span className="game-detail__meta-value">{stateLabel(game.state_flags)}</span>
            </div>
            <div className="game-detail__meta-item">
              <span className="game-detail__meta-label">Build ID</span>
              <span className="game-detail__meta-value">{game.buildid}</span>
            </div>
            <div className="game-detail__meta-item">
              <span className="game-detail__meta-label">Size on Disk</span>
              <span className="game-detail__meta-value">{game.size_on_disk}</span>
            </div>
            {game.last_updated && (
              <div className="game-detail__meta-item">
                <span className="game-detail__meta-label">Last Updated</span>
                <span className="game-detail__meta-value">{formatEpoch(game.last_updated)}</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {game.update_pending && (
        <div className="game-detail__update-banner">
          <AlertCircle size={16} />
          <span>
            Update pending — target build <strong>{game.target_build_id}</strong>
            {game.bytes_to_download && game.bytes_to_download !== "0" && (
              <> ({game.bytes_to_download} bytes remaining)</>
            )}
          </span>
        </div>
      )}

      <div className="game-detail__body">
        <div className="game-detail__section">
          <h2 className="game-detail__section-title">Current Installation</h2>
          <div className="game-detail__info-grid">
            <div className="game-detail__info-item">
              <span className="game-detail__info-label">App ID</span>
              <span className="game-detail__info-value">{game.appid}</span>
            </div>
            <div className="game-detail__info-item">
              <span className="game-detail__info-label">Install Directory</span>
              <span className="game-detail__info-value">{game.install_path}</span>
            </div>
          </div>
        </div>

        {mergedDepots.length > 0 && (
          <div className="game-detail__section">
            <h2 className="game-detail__section-title">
              Depots
              <span className="game-detail__section-hint">
                Depots are content packages that make up the game. Each has its own manifest (version).
              </span>
            </h2>
            <div className="game-detail__depot-list">
              {mergedDepots.map((depot) => (
                <div
                  key={depot.depot_id}
                  className={`game-detail__depot${depot.installed ? "" : " game-detail__depot--not-installed"}`}
                >
                  <div className="game-detail__depot-header">
                    <span className="game-detail__depot-id">
                      Depot {depot.depot_id}
                      {depot.name && <> &mdash; {depot.name}</>}
                    </span>
                    {depot.installed && depot.size && (
                      <span className="game-detail__depot-size">{formatDepotSize(depot.size)}</span>
                    )}
                    {!depot.installed && (
                      <span className="game-detail__depot-status">Not installed</span>
                    )}
                  </div>
                  {depot.installed && depot.manifest && (
                    <div className="game-detail__depot-detail">
                      <span className="game-detail__depot-label">Manifest</span>
                      <span className="game-detail__depot-manifest">{depot.manifest}</span>
                    </div>
                  )}
                  <button
                    className="game-detail__depot-browse"
                    onClick={() => onChangeVersion(depot.depot_id)}
                    type="button"
                  >
                    Browse Versions
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

interface MergedDepot {
  depot_id: string;
  name: string | null;
  installed: boolean;
  manifest: string | null;
  size: string | null;
}

function buildMergedDepots(
  game: GameInfo,
  steamDepots: SteamDepotInfo[],
  installedIds: Set<string>,
): MergedDepot[] {
  const steamByDepotId = new Map(steamDepots.map((d) => [d.depot_id, d]));
  const result: MergedDepot[] = [];

  // Installed depots first, enriched with Steam names
  for (const depot of game.depots) {
    const steam = steamByDepotId.get(depot.depot_id);
    result.push({
      depot_id: depot.depot_id,
      name: steam?.name ?? null,
      installed: true,
      manifest: depot.manifest,
      size: depot.size,
    });
  }

  // Non-installed depots from Steam
  for (const steam of steamDepots) {
    if (!installedIds.has(steam.depot_id)) {
      result.push({
        depot_id: steam.depot_id,
        name: steam.name,
        installed: false,
        manifest: null,
        size: null,
      });
    }
  }

  return result;
}

function formatDepotSize(sizeStr: string): string {
  const bytes = Number(sizeStr);
  if (isNaN(bytes)) return sizeStr;
  const GB = 1_073_741_824;
  const MB = 1_048_576;
  const KB = 1_024;
  if (bytes >= GB) return `${(bytes / GB).toFixed(1)} GB`;
  if (bytes >= MB) return `${(bytes / MB).toFixed(1)} MB`;
  if (bytes >= KB) return `${Math.round(bytes / KB)} KB`;
  return `${bytes} B`;
}
