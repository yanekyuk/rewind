import { useGameList } from "../hooks/useGameList";
import type { GameInfo } from "../types/game";

interface GameLibraryProps {
  username: string;
  onSelectGame: (game: GameInfo) => void;
  onSignOut: () => void;
}

function steamHeaderUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/header.jpg`;
}

export function GameLibrary({ username, onSelectGame, onSignOut }: GameLibraryProps) {
  const { games, loading, error, retry } = useGameList();

  return (
    <div className="game-library">
      <header className="game-library__header">
        <div className="game-library__brand">
          <h1 className="game-library__title">Rewind</h1>
          <span className="game-library__subtitle">Library</span>
        </div>
        <div className="game-library__user">
          <span className="game-library__username">{username}</span>
          <button
            className="game-library__sign-out"
            onClick={onSignOut}
            type="button"
          >
            Sign Out
          </button>
        </div>
      </header>

      <main className="game-library__content">
        {loading && (
          <p className="game-library__loading">Loading installed games...</p>
        )}

        {error && (
          <div className="game-library__error">
            <p className="game-library__error-message">{error}</p>
            <button
              className="game-library__retry-button"
              onClick={retry}
              type="button"
            >
              Retry
            </button>
          </div>
        )}

        {!loading && !error && games.length === 0 && (
          <p className="game-library__empty">
            No games found. Make sure Steam is installed and you have games in
            your library.
          </p>
        )}

        {!loading && !error && games.length > 0 && (
          <div className="game-library__grid">
            {games.map((game) => (
              <button
                key={game.appid}
                className="game-card"
                onClick={() => onSelectGame(game)}
                type="button"
              >
                <img
                  className="game-card__image"
                  src={steamHeaderUrl(game.appid)}
                  alt={game.name}
                  loading="lazy"
                />
                <div className="game-card__info">
                  <span className="game-card__name">{game.name}</span>
                  <span className="game-card__build">Build {game.buildid}</span>
                </div>
              </button>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
