import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import App from "./App";
import type { GameInfo } from "./types/game";

// Mock all hooks
const mockUseAuth = vi.fn();
const mockUseGameList = vi.fn();
const mockUseManifestList = vi.fn();

vi.mock("./hooks/useAuth", () => ({
  useAuth: () => mockUseAuth(),
}));

vi.mock("./hooks/useGameList", () => ({
  useGameList: () => mockUseGameList(),
}));

vi.mock("./hooks/useManifestList", () => ({
  useManifestList: () => mockUseManifestList(),
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
];

describe("App", () => {
  beforeEach(() => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: vi.fn(),
    });
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: null,
      retry: vi.fn(),
    });
    mockUseManifestList.mockReturnValue({
      manifests: [],
      loading: false,
      error: null,
      fetch: vi.fn(),
    });
  });

  it("shows login view when not authenticated", () => {
    render(<App />);
    expect(screen.getByRole("form", { name: /steam authentication/i })).toBeInTheDocument();
  });

  it("shows game library when authenticated", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: vi.fn(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<App />);
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
  });

  it("navigates to game detail when a game is clicked", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: vi.fn(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<App />);
    fireEvent.click(screen.getByText("Crimson Desert"));

    expect(screen.getByRole("button", { name: /change version/i })).toBeInTheDocument();
  });

  it("navigates to version select when Change Version is clicked", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: vi.fn(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<App />);
    fireEvent.click(screen.getByText("Crimson Desert"));
    fireEvent.click(screen.getByRole("button", { name: /change version/i }));

    expect(screen.getByText("Current Version")).toBeInTheDocument();
    expect(screen.getByText("Available Versions")).toBeInTheDocument();
  });

  it("returns to game library via back from game detail", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: vi.fn(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<App />);
    fireEvent.click(screen.getByText("Crimson Desert"));
    fireEvent.click(screen.getByRole("button", { name: /back/i }));

    // Should be back at library - game card visible, no Change Version button
    expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /change version/i })).not.toBeInTheDocument();
  });

  it("calls signOut on sign out click", () => {
    const mockSignOut = vi.fn();
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: vi.fn(),
      signOut: mockSignOut,
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: vi.fn(),
    });

    render(<App />);
    fireEvent.click(screen.getByText(/sign out/i));

    expect(mockSignOut).toHaveBeenCalled();
  });
});
