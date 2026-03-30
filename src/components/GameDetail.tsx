import type { GameInfo } from "../types/game";

interface GameDetailProps {
  game: GameInfo;
  onChangeVersion: () => void;
}

function steamHeroUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/library_hero.jpg`;
}

function steamLogoUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/logo.png`;
}

export function GameDetail({ game, onChangeVersion }: GameDetailProps) {
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
            // If no logo, hide it — the name will show in the info bar
            e.currentTarget.style.display = "none";
          }}
        />
      </div>

      <div className="game-detail__info-bar">
        <button
          className="game-detail__change-version"
          onClick={onChangeVersion}
          type="button"
        >
          Change Version
        </button>

        <div className="game-detail__meta">
          <div className="game-detail__meta-item">
            <span className="game-detail__meta-label">App ID</span>
            <span className="game-detail__meta-value">{game.appid}</span>
          </div>
          <div className="game-detail__meta-item">
            <span className="game-detail__meta-label">Build ID</span>
            <span className="game-detail__meta-value">{game.buildid}</span>
          </div>
        </div>
      </div>

      <div className="game-detail__body">
        <div className="game-detail__section">
          <h2 className="game-detail__section-title">Install Info</h2>
          <div className="game-detail__info-grid">
            <div className="game-detail__info-item">
              <span className="game-detail__info-label">Install Directory</span>
              <span className="game-detail__info-value">{game.installdir}</span>
            </div>
            <div className="game-detail__info-item">
              <span className="game-detail__info-label">Install Path</span>
              <span className="game-detail__info-value">{game.install_path}</span>
            </div>
          </div>
        </div>

        {game.depots.length > 0 && (
          <div className="game-detail__section">
            <h2 className="game-detail__section-title">Depots</h2>
            <div className="game-detail__depot-list">
              {game.depots.map((depot) => (
                <div key={depot.depot_id} className="game-detail__depot">
                  <span className="game-detail__depot-id">Depot {depot.depot_id}</span>
                  <span className="game-detail__depot-manifest">Manifest: {depot.manifest}</span>
                  <span className="game-detail__depot-size">{depot.size}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
