import { useGameList } from "../hooks/useGameList";
import type { GameInfo } from "../types/game";

interface GameSelectProps {
  selectedGame: GameInfo | null;
  onSelectGame: (game: GameInfo) => void;
}

export function GameSelect({ selectedGame, onSelectGame }: GameSelectProps) {
  const { games, loading, error, retry } = useGameList();

  if (loading) {
    return (
      <section className="game-select">
        <h2 className="step-view__title">Select Game</h2>
        <p className="game-select__loading">Loading installed games...</p>
      </section>
    );
  }

  if (error) {
    return (
      <section className="game-select">
        <h2 className="step-view__title">Select Game</h2>
        <div className="game-select__error">
          <p className="game-select__error-message">{error}</p>
          <button className="game-select__retry-button" onClick={retry}>
            Retry
          </button>
        </div>
      </section>
    );
  }

  if (games.length === 0) {
    return (
      <section className="game-select">
        <h2 className="step-view__title">Select Game</h2>
        <p className="game-select__empty">
          No games found. Make sure Steam is installed and you have games in your library.
        </p>
      </section>
    );
  }

  return (
    <section className="game-select">
      <h2 className="step-view__title">Select Game</h2>
      <p className="step-view__description">
        Choose an installed Steam game to downgrade.
      </p>
      <div className="game-list">
        {games.map((game) => (
          <button
            key={game.appid}
            className={`game-row${selectedGame?.appid === game.appid ? " game-row--selected" : ""}`}
            onClick={() => onSelectGame(game)}
            type="button"
          >
            <span className="game-row__name">{game.name}</span>
            <span className="game-row__meta">
              <span className="game-row__appid">{game.appid}</span>
              <span className="game-row__buildid">{game.buildid}</span>
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}
