import { afterEach, describe, it, expect, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { ManifestSelect } from "./ManifestSelect";
import type { GameInfo } from "../types/game";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const mockGame: GameInfo = {
  appid: "3321460",
  name: "Crimson Desert",
  buildid: "22560074",
  installdir: "Crimson Desert",
  depots: [{ depot_id: "3321461", manifest: "744665017", size: "133575233011" }],
  install_path: "/steamapps/common/Crimson Desert",
};

afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("ManifestSelect", () => {
  it("auto-fetches manifests on mount without credential inputs", async () => {
    mockInvoke.mockResolvedValue([]);

    render(
      <ManifestSelect
        selectedGame={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
      />,
    );

    // Should show loading state immediately (auto-fetch on mount)
    expect(
      screen.getByText(/fetching available versions/i),
    ).toBeInTheDocument();

    // Verify IPC call has no username/password
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("list_manifests", {
        appId: "3321460",
        depotId: "3321461",
      });
    });
  });

  it("does not render username or password inputs", async () => {
    mockInvoke.mockResolvedValue([]);

    render(
      <ManifestSelect
        selectedGame={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
      />,
    );

    // No credential inputs should exist
    expect(screen.queryByLabelText(/username/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/password/i)).not.toBeInTheDocument();
  });

  it("displays manifests after successful fetch", async () => {
    const manifests = [
      { manifest_id: "111222333", date: "2025-01-15 12:00:00" },
      { manifest_id: "444555666", date: "2025-01-10 08:30:00" },
    ];
    mockInvoke.mockResolvedValue(manifests);

    render(
      <ManifestSelect
        selectedGame={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("111222333")).toBeInTheDocument();
      expect(screen.getByText("444555666")).toBeInTheDocument();
    });
  });

  it("displays error state on fetch failure", async () => {
    mockInvoke.mockRejectedValue("Authentication required");

    render(
      <ManifestSelect
        selectedGame={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText("Authentication required")).toBeInTheDocument();
    });

    // Should have a retry button
    expect(screen.getByText(/retry/i)).toBeInTheDocument();
  });
});
