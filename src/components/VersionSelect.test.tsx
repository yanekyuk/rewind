import { afterEach, afterAll, describe, it, expect, mock, beforeEach } from "bun:test";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import type { GameInfo } from "../types/game";

const mockUseManifestList = mock();
const mockInvoke = mock();

mock.module("../hooks/useManifestList", () => ({
  useManifestList: () => mockUseManifestList(),
}));
mock.module("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));
mock.module("@tauri-apps/api/event", () => ({
  listen: mock(() => Promise.resolve(() => {})),
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
  { manifest_id: "7446650175280810671", branch: "public", time_updated: 1774387305, pwd_required: false },
  { manifest_id: "7446500175280810670", branch: "beta", time_updated: 1773782220, pwd_required: true },
];

describe("VersionSelect", () => {
  beforeEach(() => {
    mockUseManifestList.mockReset();
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
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
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
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
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
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
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
      />,
    );
    expect(screen.getByText(/auth required/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(fetchFn).toHaveBeenCalled();
  });

  it("renders available versions with branch name and manifest ID", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
      />,
    );
    expect(screen.getByText("public")).toBeInTheDocument();
    expect(screen.getByText("7446650175280810671")).toBeInTheDocument();
    expect(screen.getByText("beta")).toBeInTheDocument();
    expect(screen.getByText("7446500175280810670")).toBeInTheDocument();
  });

  it("calls onSelectManifest when a version row is clicked", async () => {
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
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={onSelectManifest}
      />,
    );

    fireEvent.click(screen.getByText("public"));
    await waitFor(() => {
      expect(onSelectManifest).toHaveBeenCalledWith("7446650175280810671");
    });
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
        depotId={null}
        selectedManifestId="7446650175280810671"
        onSelectManifest={mock()}
      />,
    );

    const selectedRows = container.querySelectorAll(".version-row--selected");
    expect(selectedRows.length).toBe(1);
  });

  it("highlights the currently installed manifest", () => {
    const manifestsWithCurrent = [
      { manifest_id: "744665017", branch: "public", time_updated: 1774387305 },
      { manifest_id: "7446500175280810670", branch: "beta", time_updated: 1773782220 },
    ];
    mockUseManifestList.mockReturnValue({
      manifests: manifestsWithCurrent,
      loading: false,
      error: null,
      fetch: mock(),
    });

    const { container } = render(
      <VersionSelect
        game={mockGame}
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
      />,
    );

    const currentRows = container.querySelectorAll(".version-row--current");
    expect(currentRows.length).toBe(1);
  });

  it("shows password required indicator for locked branches", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
      />,
    );

    // The beta branch has pwd_required: true
    expect(screen.getByText(/password required/i)).toBeInTheDocument();
  });

  it("provides manual manifest ID input field", () => {
    mockUseManifestList.mockReturnValue({
      manifests: mockManifests,
      loading: false,
      error: null,
      fetch: mock(),
    });

    render(
      <VersionSelect
        game={mockGame}
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={mock()}
      />,
    );

    const input = screen.getByPlaceholderText(/manifest id/i);
    expect(input).toBeInTheDocument();
  });

  it("calls onSelectManifest when manual manifest ID is submitted", async () => {
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
        depotId={null}
        selectedManifestId={null}
        onSelectManifest={onSelectManifest}
      />,
    );

    const input = screen.getByPlaceholderText(/manifest id/i);
    fireEvent.change(input, { target: { value: "9999999999" } });
    fireEvent.click(screen.getByRole("button", { name: /use/i }));
    await waitFor(() => {
      expect(onSelectManifest).toHaveBeenCalledWith("9999999999");
    });
  });
});
