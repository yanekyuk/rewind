import { afterEach, describe, it, expect, vi, beforeEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { GameSelect } from "./GameSelect";
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

describe("GameSelect", () => {
  beforeEach(() => {
    mockUseGameList.mockReset();
  });

  it("shows a loading indicator while fetching", () => {
    mockUseGameList.mockReturnValue({
      games: [],
      loading: true,
      error: null,
      retry: vi.fn(),
    });

    render(<GameSelect selectedGame={null} onSelectGame={vi.fn()} />);
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("shows an error message with retry button when fetch fails", () => {
    const retry = vi.fn();
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: "Steam not found",
      retry,
    });

    render(<GameSelect selectedGame={null} onSelectGame={vi.fn()} />);
    expect(screen.getByText(/steam not found/i)).toBeInTheDocument();

    const retryButton = screen.getByRole("button", { name: /retry/i });
    fireEvent.click(retryButton);
    expect(retry).toHaveBeenCalledOnce();
  });

  it("shows empty state when no games are found", () => {
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameSelect selectedGame={null} onSelectGame={vi.fn()} />);
    expect(screen.getByText(/no games found/i)).toBeInTheDocument();
  });

  it("renders the game list with name, appid, and buildid", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<GameSelect selectedGame={null} onSelectGame={vi.fn()} />);
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    expect(screen.getByText("3321460")).toBeInTheDocument();
    expect(screen.getByText("22560074")).toBeInTheDocument();
    expect(screen.getByText("Team Fortress 2")).toBeInTheDocument();
    expect(screen.getByText("440")).toBeInTheDocument();
  });

  it("calls onSelectGame when a game row is clicked", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    const onSelectGame = vi.fn();
    render(<GameSelect selectedGame={null} onSelectGame={onSelectGame} />);

    fireEvent.click(screen.getByText("Crimson Desert"));
    expect(onSelectGame).toHaveBeenCalledWith(mockGames[0]);
  });

  it("highlights the selected game row", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    const { container } = render(
      <GameSelect selectedGame={mockGames[0]} onSelectGame={vi.fn()} />,
    );

    const selectedRow = container.querySelector(".game-row--selected");
    expect(selectedRow).not.toBeNull();
    expect(selectedRow!.textContent).toContain("Crimson Desert");
  });

  it("does not highlight unselected rows", () => {
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    const { container } = render(
      <GameSelect selectedGame={mockGames[0]} onSelectGame={vi.fn()} />,
    );

    const allRows = container.querySelectorAll(".game-row");
    const selectedRows = container.querySelectorAll(".game-row--selected");
    expect(allRows.length).toBe(2);
    expect(selectedRows.length).toBe(1);
  });
});
