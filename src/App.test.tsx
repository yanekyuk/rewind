import { describe, it, expect, mock, beforeEach, afterEach, afterAll } from "bun:test";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import type { GameInfo } from "./types/game";

// Mock all hooks
const mockUseAuth = mock();
const mockUseGameList = mock();
const mockUseManifestList = mock();
const mockUseDepotList = mock();
mock.module("./hooks/useAuth", () => ({
  useAuth: () => mockUseAuth(),
}));

mock.module("./hooks/useGameList", () => ({
  useGameList: () => mockUseGameList(),
}));

mock.module("./hooks/useManifestList", () => ({
  useManifestList: () => mockUseManifestList(),
}));

mock.module("./hooks/useDepotList", () => ({
  useDepotList: () => mockUseDepotList(),
}));

mock.module("@tauri-apps/api/core", () => ({
  Channel: class {},
  PluginListener: class {},
  Resource: class {},
  SERIALIZE_TO_IPC_FN: Symbol("SERIALIZE_TO_IPC_FN"),
  addPluginListener: mock(),
  checkPermissions: mock(),
  convertFileSrc: mock(),
  invoke: mock(),
  isTauri: mock(),
  requestPermissions: mock(),
  transformCallback: mock(),
}));

mock.module("@tauri-apps/api/event", () => ({
  TauriEvent: {},
  emit: mock(),
  emitTo: mock(),
  listen: mock(),
  once: mock(),
}));

const { default: App } = await import("./App");

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
    state_flags: 4,
    update_pending: false,
    target_build_id: null,
    bytes_to_download: null,
    size_on_disk: "133575233011",
    last_updated: null,
  },
];

describe("App", () => {
  beforeEach(() => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: null,
      submit: mock(),
      signOut: mock(),
    });
    mockUseGameList.mockReturnValue({
      games: [],
      loading: false,
      error: null,
      retry: mock(),
    });
    mockUseManifestList.mockReturnValue({
      manifests: [],
      loading: false,
      error: null,
      fetch: mock(),
    });
    mockUseDepotList.mockReturnValue({
      depots: [],
      loading: false,
      error: null,
      fetch: mock(),
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
      submit: mock(),
      signOut: mock(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<App />);
    expect(screen.getAllByText("Crimson Desert").length).toBeGreaterThan(0);
  });

  it("navigates to game detail when a game is clicked", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: mock(),
      signOut: mock(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    const { container } = render(<App />);
    const gameCard = container.querySelector(".game-card")!;
    fireEvent.click(gameCard);

    expect(screen.getByRole("button", { name: /change version/i })).toBeInTheDocument();
  });

  it("navigates to version select when Change Version is clicked", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: mock(),
      signOut: mock(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    const { container } = render(<App />);
    fireEvent.click(container.querySelector(".game-card")!);
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
      submit: mock(),
      signOut: mock(),
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    const { container } = render(<App />);
    fireEvent.click(container.querySelector(".game-card")!);
    fireEvent.click(screen.getByRole("button", { name: /back/i }));

    // Should be back at library - game card visible, no Change Version button
    expect(screen.getAllByText("Crimson Desert").length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: /change version/i })).not.toBeInTheDocument();
  });

  it("calls signOut on sign out click", () => {
    const mockSignOut = mock();
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: mock(),
      signOut: mockSignOut,
    });
    mockUseGameList.mockReturnValue({
      games: mockGames,
      loading: false,
      error: null,
      retry: mock(),
    });

    render(<App />);
    fireEvent.click(screen.getByText(/sign out/i));

    expect(mockSignOut).toHaveBeenCalled();
  });
});
