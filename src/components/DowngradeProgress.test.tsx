import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { DowngradeProgress } from "./DowngradeProgress";
import type { GameInfo } from "../types/game";

// Mock the hook
const mockUseDowngradeProgress = vi.fn();
vi.mock("../hooks/useDowngradeProgress", () => ({
  useDowngradeProgress: mockUseDowngradeProgress,
}));

import { useDowngradeProgress } from "../hooks/useDowngradeProgress";

const mockGame: GameInfo = {
  appid: "3321460",
  name: "Test Game",
  state_flags: 4,
  size_on_disk: "50000000000",
  buildid: "12345",
  last_updated: "1640000000",
  update_pending: false,
  install_path: "/path/to/game",
  target_build_id: "",
  bytes_to_download: "",
  depots: [
    {
      depot_id: "3321461",
      manifest: "9876543210",
      size: "50000000000",
    },
  ],
};

describe("DowngradeProgress", () => {
  const mockOnComplete = vi.fn();
  const mockOnError = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnComplete.mockClear();
    mockOnError.mockClear();
  });

  it("renders comparing phase", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "comparing",
      isActive: true,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Comparing manifests/)).toBeInTheDocument();
    expect(
      screen.getByText(/Fetching version information/)
    ).toBeInTheDocument();
  });

  it("renders downloading phase with metrics", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "downloading",
      percent: 45,
      bytesDownloaded: 4500000000,
      bytesTotal: 10000000000,
      speed: "12.5 MB/s",
      eta: "~5 min",
      isActive: true,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Downloading files/)).toBeInTheDocument();
    expect(screen.getByText(/45%/)).toBeInTheDocument();
    expect(screen.getByText(/12.5 MB\/s/)).toBeInTheDocument();
    expect(screen.getByText(/~5 min/)).toBeInTheDocument();
  });

  it("renders applying phase", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "applying",
      isActive: true,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Applying files/)).toBeInTheDocument();
    expect(
      screen.getByText(/Copying files, patching ACF/)
    ).toBeInTheDocument();
  });

  it("renders complete phase", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "complete",
      isActive: false,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Downgrade Complete/)).toBeInTheDocument();
    expect(screen.getByText(/Successfully downgraded/)).toBeInTheDocument();
    expect(screen.getByText(/Important/)).toBeInTheDocument();
  });

  it("calls onComplete when phase becomes complete", () => {
    const { rerender } = render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    mockUseDowngradeProgress.mockReturnValue({
      phase: "complete",
      isActive: false,
    });

    rerender(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(mockOnComplete).toHaveBeenCalled();
  });

  it("renders error phase with message", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "error",
      error: "Download failed due to network error",
      isActive: false,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
        onError={mockOnError}
      />
    );

    expect(screen.getByText(/Downgrade Failed/)).toBeInTheDocument();
    expect(
      screen.getByText(/Download failed due to network error/)
    ).toBeInTheDocument();
  });

  it("shows cancel button during active phases", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "downloading",
      percent: 50,
      isActive: true,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByRole("button", { name: /Cancel/ })).toBeInTheDocument();
  });

  it("shows return to game button in complete state", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "complete",
      isActive: false,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(
      screen.getByRole("button", { name: /Return to Game/ })
    ).toBeInTheDocument();
  });

  it("shows retry button in error state", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "error",
      error: "Failed",
      isActive: false,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    expect(
      screen.getByRole("button", { name: /Retry/ })
    ).toBeInTheDocument();
  });

  it("handles return to game button click", () => {
    mockUseDowngradeProgress.mockReturnValue({
      phase: "complete",
      isActive: false,
    });

    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        onComplete={mockOnComplete}
      />
    );

    const button = screen.getByRole("button", { name: /Return to Game/ });
    button.click();

    expect(mockOnComplete).toHaveBeenCalled();
  });
});
