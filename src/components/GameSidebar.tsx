import { useGameList } from "../hooks/useGameList";
import type { GameInfo } from "../types/game";

interface GameSidebarProps {
  selectedAppId: string | null;
  onSelectGame: (game: GameInfo) => void;
}

export function GameSidebar({ selectedAppId, onSelectGame }: GameSidebarProps) {
  const { games, loading, error, retry } = useGameList();

  return (
    <div className="game-sidebar">
      <div className="game-sidebar__list">
        {loading && (
          <p className="game-sidebar__status">Loading games...</p>
        )}

        {error && (
          <div className="game-sidebar__status">
            <p>{error}</p>
            <button onClick={retry} type="button" className="game-sidebar__retry">
              Retry
            </button>
          </div>
        )}

        {!loading && !error && games.map((game) => (
          <button
            key={game.appid}
            className={`game-sidebar__item${selectedAppId === game.appid ? " game-sidebar__item--active" : ""}`}
            onClick={() => onSelectGame(game)}
            type="button"
          >
            <span className="game-sidebar__name">{game.name}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
