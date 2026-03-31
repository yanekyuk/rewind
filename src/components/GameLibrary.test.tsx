import { afterEach, afterAll, describe, it, expect, mock, beforeEach } from "bun:test";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import type { GameInfo } from "../types/game";

const mockUseGameList = mock();

mock.module("../hooks/useGameList", () => ({
  useGameList: () => mockUseGameList(),
}));

const { GameLibrary } = await import("./GameLibrary");

afterEach(cleanup);
afterAll(() => mock.restore());

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
      retry: mock(),
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("shows error message with retry button when fetch fails", () => {
    const retry = mock();
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: "Steam not found",
      retry,
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
    expect(screen.getByText(/steam not found/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it("shows empty state when no games are found", () => {
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
    expect(screen.getByText(/no games found/i)).toBeInTheDocument();
  });

  it("renders game cards with name and build ID", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    expect(screen.getByText("Team Fortress 2")).toBeInTheDocument();
  });

  it("renders game header images from Steam CDN", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
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
      retry: mock(),
    });

    const onSelectGame = mock();
    render(<GameLibrary username="testuser" onSelectGame={onSelectGame} onSignOut={mock()} />);

    fireEvent.click(screen.getByText("Crimson Desert"));
    expect(onSelectGame).toHaveBeenCalledWith(mockGames[0]);
  });

  it("displays the signed-in username", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={mock()} />);
    expect(screen.getByText(/testuser/)).toBeInTheDocument();
  });

  it("calls onSignOut when sign out is clicked", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    const onSignOut = mock();
    render(<GameLibrary username="testuser" onSelectGame={mock()} onSignOut={onSignOut} />);

    fireEvent.click(screen.getByText(/sign out/i));
    expect(onSignOut).toHaveBeenCalledTimes(1);
  });
});
