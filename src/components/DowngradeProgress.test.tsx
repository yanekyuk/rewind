import { afterEach, describe, it, expect, mock, beforeEach } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import { DowngradeProgress } from "./DowngradeProgress";
import type { DowngradeProgressState } from "./DowngradeProgress";
import type { GameInfo } from "../types/game";

const mockGame: GameInfo = {
  appid: "3321460",
  name: "Test Game",
  buildid: "12345",
  depots: [
    {
      depot_id: "3321461",
      manifest: "9876543210",
      size: "50000000000",
    },
  ],
  install_path: "/home/user/.steam/steamapps/common/Test Game",
};

const nullProgress: DowngradeProgressState = { phase: null, isActive: false };

describe("DowngradeProgress", () => {
  const mockOnComplete = mock();
  const mockOnRetry = mock();
  const mockOnError = mock();

  afterEach(cleanup);

  beforeEach(() => {
    mockOnComplete.mockClear();
    mockOnRetry.mockClear();
    mockOnError.mockClear();
  });

  it("renders comparing phase", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "comparing", isActive: true }}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Comparing manifests/)).toBeInTheDocument();
    expect(
      screen.getByText(/Fetching version information/)
    ).toBeInTheDocument();
  });

  it("renders downloading phase with metrics", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{
          phase: "downloading",
          percent: 45,
          bytesDownloaded: 4500000000,
          bytesTotal: 10000000000,
          speed: "12.5 MB/s",
          eta: "~5 min",
          isActive: true,
        }}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Downloading files/)).toBeInTheDocument();
    expect(screen.getByText(/45%/)).toBeInTheDocument();
    expect(screen.getByText(/12.5 MB\/s/)).toBeInTheDocument();
    expect(screen.getByText(/~5 min/)).toBeInTheDocument();
  });

  it("renders applying phase", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "applying", isActive: true }}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Applying files/)).toBeInTheDocument();
    expect(
      screen.getByText(/Copying files, patching ACF/)
    ).toBeInTheDocument();
  });

  it("renders complete phase", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "complete", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByText(/Downgrade Complete/)).toBeInTheDocument();
    expect(screen.getByText(/Successfully downgraded/)).toBeInTheDocument();
    expect(screen.getByText(/Important/)).toBeInTheDocument();
  });

  it("calls onComplete when phase is complete", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "complete", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    expect(mockOnComplete).toHaveBeenCalled();
  });

  it("renders error phase with message", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "error", error: "Download failed due to network error", isActive: false }}
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
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "downloading", percent: 50, isActive: true }}
        onComplete={mockOnComplete}
      />
    );

    expect(screen.getByRole("button", { name: /Cancel/ })).toBeInTheDocument();
  });

  it("shows return to game button in complete state", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "complete", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    expect(
      screen.getByRole("button", { name: /Return to Game/ })
    ).toBeInTheDocument();
  });

  it("shows retry button in error state", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "error", error: "Failed", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    expect(
      screen.getByRole("button", { name: /Retry/ })
    ).toBeInTheDocument();
  });

  it("calls onRetry when retry button is clicked", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "error", error: "Failed", isActive: false }}
        onComplete={mockOnComplete}
        onRetry={mockOnRetry}
      />
    );

    screen.getByRole("button", { name: /Retry/ }).click();
    expect(mockOnRetry).toHaveBeenCalled();
  });

  it("falls back to onComplete when retry clicked without onRetry prop", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "error", error: "Failed", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    screen.getByRole("button", { name: /Retry/ }).click();
    expect(mockOnComplete).toHaveBeenCalled();
  });

  it("handles return to game button click", () => {
    render(
      <DowngradeProgress
        game={mockGame}
        targetManifestId="1234567890"
        progress={{ phase: "complete", isActive: false }}
        onComplete={mockOnComplete}
      />
    );

    const button = screen.getByRole("button", { name: /Return to Game/ });
    button.click();

    expect(mockOnComplete).toHaveBeenCalled();
  });
});
