import { afterEach, describe, it, expect, vi, beforeEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { GameLibrary } from "./GameLibrary";
import type { GameInfo } from "../types/game";

const mockUseGameList = vi.fn();

vi.mock("../hooks/useGameList", () => ({
  useGameList: () => mockUseGameList(),
}));

afterEach(cleanup);

const mockGames: GameInfo[] = [
  {
    appid: "3321460",
    name: "Crimson Desert",
    buildid: "22560074",
    installdir: "Crimson Desert",
    depots: [{ depot_id: "3321461", manifest: "744665017", size: "133575233011" }],
    install_path: "/steamapps/common/Crimson Desert",
  },
  {
    appid: "440",
    name: "Team Fortress 2",
    buildid: "12345",
    installdir: "Team Fortress 2",
    depots: [],
    install_path: "/steamapps/common/Team Fortress 2",
  },
];

describe("GameLibrary", () => {
  beforeEach(() => {
    mockUseGameList.mockReset();
  });

  it("shows loading indicator while fetching games", () => {
    mockUseGameList.mockReturnValue({
      games: [],
      loading: true,
      error: null,
      retry: vi.fn(),
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("shows error message with retry button when fetch fails", () => {
    const retry = vi.fn();
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: "Steam not found",
      retry,
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    expect(screen.getByText(/steam not found/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(retry).toHaveBeenCalledOnce();
  });

  it("shows empty state when no games are found", () => {
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    expect(screen.getByText(/no games found/i)).toBeInTheDocument();
  });

  it("renders game cards with name and build ID", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    expect(screen.getByText("Team Fortress 2")).toBeInTheDocument();
  });

  it("renders game header images from Steam CDN", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    const images = screen.getAllByRole("img");
    expect(images[0]).toHaveAttribute(
      "src",
      "https://cdn.akamai.steamstatic.com/steam/apps/3321460/header.jpg",
    );
    expect(images[1]).toHaveAttribute(
      "src",
      "https://cdn.akamai.steamstatic.com/steam/apps/440/header.jpg",
    );
  });

  it("calls onSelectGame when a game card is clicked", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    const onSelectGame = vi.fn();
    render(<GameLibrary username="testuser" onSelectGame={onSelectGame} onSignOut={vi.fn()} />);

    fireEvent.click(screen.getByText("Crimson Desert"));
    expect(onSelectGame).toHaveBeenCalledWith(mockGames[0]);
  });

  it("displays the signed-in username", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={vi.fn()} />);
    expect(screen.getByText(/testuser/)).toBeInTheDocument();
  });

  it("calls onSignOut when sign out is clicked", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    const onSignOut = vi.fn();
    render(<GameLibrary username="testuser" onSelectGame={vi.fn()} onSignOut={onSignOut} />);

    fireEvent.click(screen.getByText(/sign out/i));
    expect(onSignOut).toHaveBeenCalledOnce();
  });
});
