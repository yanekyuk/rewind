import { afterEach, describe, it, expect, vi, beforeEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { VersionSelect } from "./VersionSelect";
import type { GameInfo } from "../types/game";

const mockUseManifestList = vi.fn();

vi.mock("../hooks/useManifestList", () => ({
  useManifestList: () => mockUseManifestList(),
}));

afterEach(cleanup);

const mockGame: GameInfo = {
  appid: "3321460",
  name: "Crimson Desert",
  buildid: "22560074",
  installdir: "Crimson Desert",
  depots: [{ depot_id: "3321461", manifest: "744665017", size: "133575233011" }],
  install_path: "/steamapps/common/Crimson Desert",
};

const mockManifests = [
  { manifest_id: "7446650175280810671", date: "2026-03-22 16:01:45" },
  { manifest_id: "7446500175280810670", date: "2026-03-15 14:30:20" },
];

describe("VersionSelect", () => {
  beforeEach(() => {
    mockUseManifestList.mockReset();
  });

  it("displays current version info (build ID, manifest ID)", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: vi.fn(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    // Current build ID
    expect(screen.getByText("22560074")).toBeInTheDocument();
    // Current manifest ID from depot
    expect(screen.getByText("744665017")).toBeInTheDocument();
  });

  it("shows loading state while fetching manifests", () => {
    mockUseManifestList.mockReturnValue({
      manifests: [],
      loading: true,
      error: null,
      fetch: vi.fn(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("shows error state with retry", () => {
    const fetchFn = vi.fn();
    mockUseManifestList.mockReturnValue({
      manifests: [],
      loading: false,
      error: "Auth required",
      fetch: fetchFn,
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByText(/auth required/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(fetchFn).toHaveBeenCalled();
  });

  it("renders available versions with manifest ID and date", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: vi.fn(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
        onBack={vi.fn()}
      />,
    );
    expect(screen.getByText("2026-03-22 16:01:45")).toBeInTheDocument();
    expect(screen.getByText("7446650175280810671")).toBeInTheDocument();
    expect(screen.getByText("2026-03-15 14:30:20")).toBeInTheDocument();
    expect(screen.getByText("7446500175280810670")).toBeInTheDocument();
  });

  it("calls onSelectManifest when a version row is clicked", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: vi.fn(),
    });

    const onSelectManifest = vi.fn();
    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={onSelectManifest}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByText("2026-03-22 16:01:45"));
    expect(onSelectManifest).toHaveBeenCalledWith("7446650175280810671");
  });

  it("highlights the selected version", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: vi.fn(),
    });

    const { container } = render(
      <VersionSelect
        game={mockGame}
        selectedManifestId="7446650175280810671"
        onSelectManifest={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    const selectedRows = container.querySelectorAll(".version-row--selected");
    expect(selectedRows.length).toBe(1);
  });

  it("calls onBack when back button is clicked", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: vi.fn(),
    });

    const onBack = vi.fn();
    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={vi.fn()}
        onBack={onBack}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onBack).toHaveBeenCalledOnce();
  });
});
