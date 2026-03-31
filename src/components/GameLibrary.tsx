import { useGameList } from "../hooks/useGameList";
import type { GameInfo } from "../types/game";

interface GameLibraryProps {
  onSelectGame: (game: GameInfo) => void;
}

function sampleEdgeColor(img: HTMLImageElement) {
  try {
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.drawImage(img, 0, 0);

    const w = canvas.width;
    const h = canvas.height;
    let r = 0, g = 0, b = 0, count = 0;
    const sample = (x: number, y: number) => {
      const d = ctx.getImageData(x, y, 1, 1).data;
      r += d[0]; g += d[1]; b += d[2]; count++;
    };
    for (let x = 0; x < w; x += Math.max(1, Math.floor(w / 20))) {
      sample(x, 0);
      sample(x, h - 1);
    }
    for (let y = 0; y < h; y += Math.max(1, Math.floor(h / 20))) {
      sample(0, y);
      sample(w - 1, y);
    }

    if (count > 0) {
      const wrap = img.closest(".game-card__image-wrap") as HTMLElement | null;
      if (wrap) {
        wrap.style.backgroundColor = `rgb(${Math.round(r / count)}, ${Math.round(g / count)}, ${Math.round(b / count)})`;
      }
    }
  } catch {
    // CORS or other issue — keep default background
  }
}

function steamHeaderUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/library_600x900.jpg`;
}

function steamHeaderFallbackUrl(appid: string): string {
  return `https://cdn.akamai.steamstatic.com/steam/apps/${appid}/header.jpg`;
}

export function GameLibrary({ onSelectGame }: GameLibraryProps) {
  const { games, loading, error, retry } = useGameList();

  return (
    <div className="game-library">
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
              <div className="game-card__image-wrap">
                <img
                  className="game-card__image"
                  src={steamHeaderUrl(game.appid)}
                  alt={game.name}
                  loading="lazy"
                  crossOrigin="anonymous"
                  onLoad={(e) => {
                    const img = e.currentTarget;
                    if (img.classList.contains("game-card__image--landscape")) {
                      sampleEdgeColor(img);
                    }
                  }}
                  onError={(e) => {
                    const img = e.currentTarget;
                    const fallback = steamHeaderFallbackUrl(game.appid);
                    if (!img.src.includes("header.jpg")) {
                      img.src = fallback;
                      img.classList.add("game-card__image--landscape");
                      img.closest(".game-card")!.classList.add("game-card--landscape");
                    } else {
                      img.style.display = "none";
                      img.closest(".game-card")?.classList.add("game-card--no-image");
                    }
                  }}
                />
              </div>
              <div className="game-card__info">
                <span className="game-card__name">{game.name}</span>
                <span className="game-card__build">Build {game.buildid}</span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
