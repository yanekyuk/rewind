import { afterEach, describe, it, expect, vi } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "./App";
import { STEPS } from "./steps";
import type { GameInfo } from "./types/game";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

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

afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("App", () => {
  it("renders the app title", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    expect(screen.getByText("Rewind")).toBeInTheDocument();
  });

  it("renders the step indicator with all steps", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    for (const step of STEPS) {
      const elements = screen.getAllByText(step.label);
      expect(elements.length).toBeGreaterThanOrEqual(1);
    }
  });

  it("shows the GameSelect component on the first step", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });
  });

  it("disables Next button when no game is selected on step 0", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    const nextButton = screen.getByRole("button", { name: /next/i });
    expect(nextButton).toBeDisabled();
  });

  it("enables Next button when a game is selected", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    const nextButton = screen.getByRole("button", { name: /next/i });
    expect(nextButton).not.toBeDisabled();
  });

  it("navigates to the auth step after selecting a game and clicking Next", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_games") return Promise.resolve(mockGames);
      if (cmd === "get_auth_state") return Promise.resolve(false);
      return Promise.resolve(null);
    });
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    const nextButton = screen.getByRole("button", { name: /next/i });
    fireEvent.click(nextButton);

    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: /steam authentication/i }),
      ).toBeInTheDocument();
    });
  });

  it("navigates back to GameSelect from step 1", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_games") return Promise.resolve(mockGames);
      if (cmd === "get_auth_state") return Promise.resolve(false);
      return Promise.resolve(null);
    });
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    fireEvent.click(screen.getByRole("button", { name: /next/i }));

    const backButton = screen.getByRole("button", { name: /back/i });
    fireEvent.click(backButton);

    // GameSelect remounts and fetches again
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });
  });

  it("disables Back button on the first step", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    const backButton = screen.getByRole("button", { name: /back/i });
    expect(backButton).toBeDisabled();
  });

  it("shows select-version step after auth step", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "list_games") return Promise.resolve(mockGames);
      if (cmd === "get_auth_state") return Promise.resolve(true);
      if (cmd === "list_manifests") return Promise.resolve([]);
      return Promise.resolve(null);
    });
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    // Navigate past auth (step 1) to select-version (step 2)
    fireEvent.click(screen.getByRole("button", { name: /next/i }));
    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: /steam authentication/i }),
      ).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /next/i }));

    // Step 2 (select-version) should show the ManifestSelect component
    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: STEPS[2].label }),
      ).toBeInTheDocument();
    });
  });
});
