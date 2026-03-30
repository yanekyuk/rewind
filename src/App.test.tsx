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

  it("navigates to the next step after selecting a game and clicking Next", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    const nextButton = screen.getByRole("button", { name: /next/i });
    fireEvent.click(nextButton);

    expect(
      screen.getByRole("heading", { name: STEPS[1].label }),
    ).toBeInTheDocument();
  });

  it("navigates back to GameSelect from step 1", async () => {
    mockInvoke.mockResolvedValue(mockGames);
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

  it("shows StepView placeholder for non-game-select steps", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    render(<App />);
    await waitFor(() => {
      expect(screen.getByText("Crimson Desert")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Crimson Desert"));
    fireEvent.click(screen.getByRole("button", { name: /next/i }));

    // Step 1 should show the StepView placeholder
    expect(
      screen.getByRole("heading", { name: STEPS[1].label }),
    ).toBeInTheDocument();
    expect(screen.getByText(STEPS[1].description)).toBeInTheDocument();
  });
});
