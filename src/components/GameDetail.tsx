import type { GameInfo } from "../types/game";

interface GameDetailProps {
  game: GameInfo;
  onChangeVersion: () => void;
}

function steamHeaderUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/header.jpg`;
}

export function GameDetail({ game, onChangeVersion }: GameDetailProps) {
  return (
    <div className="game-detail">
      <div className="game-detail__hero">
        <img
          className="game-detail__image"
          src={steamHeaderUrl(game.appid)}
          alt={game.name}
        />
      </div>

      <div className="game-detail__body">
        <h1 className="game-detail__name">{game.name}</h1>

        <div className="game-detail__meta">
          <div className="game-detail__meta-item">
            <span className="game-detail__meta-label">App ID</span>
            <span className="game-detail__meta-value">{game.appid}</span>
          </div>
          <div className="game-detail__meta-item">
            <span className="game-detail__meta-label">Build ID</span>
            <span className="game-detail__meta-value">{game.buildid}</span>
          </div>
          <div className="game-detail__meta-item">
            <span className="game-detail__meta-label">Install Path</span>
            <span className="game-detail__meta-value">{game.install_path}</span>
          </div>
        </div>

        <div className="game-detail__actions">
          <button
            className="game-detail__change-version"
            onClick={onChangeVersion}
            type="button"
          >
            Change Version
          </button>
        </div>
      </div>
    </div>
  );
}
