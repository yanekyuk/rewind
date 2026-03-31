import { afterEach, afterAll, describe, it, expect, mock, beforeEach } from "bun:test";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import type { GameInfo } from "../types/game";

const mockUseManifestList = mock();

mock.module("../hooks/useManifestList", () => ({
  useManifestList: () => mockUseManifestList(),
}));

const { VersionSelect } = await import("./VersionSelect");

afterEach(cleanup);
afterAll(() => mock.restore());

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
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={mock()}
        onBack={mock()}
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
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={mock()}
        onBack={mock()}
      />,
    );
    expect(screen.getByText(/loading/i)).toBeInTheDocument();
  });

  it("shows error state with retry", () => {
    const fetchFn = mock();
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
        onSelectManifest={mock()}
        onBack={mock()}
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
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={mock()}
        onBack={mock()}
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
      fetch: mock(),
    });

    const onSelectManifest = mock();
    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={onSelectManifest}
        onBack={mock()}
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
      fetch: mock(),
    });

    const { container } = render(
      <VersionSelect
        game={mockGame}
        selectedManifestId="7446650175280810671"
        onSelectManifest={mock()}
        onBack={mock()}
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
      fetch: mock(),
    });

    const onBack = mock();
    render(
      <VersionSelect
        game={mockGame}
        selectedManifestId={null}
        onSelectManifest={mock()}
        onBack={onBack}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
