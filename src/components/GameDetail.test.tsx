import { afterEach, afterAll, describe, it, expect, mock, beforeEach } from "bun:test";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import type { GameInfo } from "../types/game";

const mockUseDepotList = mock();

mock.module("../hooks/useDepotList", () => ({
  useDepotList: () => mockUseDepotList(),
}));

const { GameDetail } = await import("./GameDetail");

afterEach(cleanup);
afterAll(() => mock.restore());

const mockGame: GameInfo = {
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
};

const mockSteamDepots = [
  { depot_id: "3321461", name: "Crimson Desert Content", max_size: 133575233011, dlc_app_id: null },
  { depot_id: "3321462", name: "Crimson Desert DLC", max_size: 5000000000, dlc_app_id: "3321470" },
  { depot_id: "3321463", name: "Crimson Desert Soundtrack", max_size: 800000000, dlc_app_id: null },
];

function depotListLoaded() {
  return {
    depots: mockSteamDepots,
    loading: false,
    error: null,
    fetch: mock(),
  };
}

function depotListLoading() {
  return {
    depots: [],
    loading: true,
    error: null,
    fetch: mock(),
  };
}

function depotListError() {
  return {
    depots: [],
    loading: false,
    error: "Network error",
    fetch: mock(),
  };
}

describe("GameDetail", () => {
  beforeEach(() => {
    mockUseDepotList.mockReset();
    mockUseDepotList.mockReturnValue(depotListLoaded());
  });

  it("displays game metadata (app ID, build ID)", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    expect(screen.getByText(/3321460/)).toBeInTheDocument();
    expect(screen.getByText(/22560074/)).toBeInTheDocument();
  });

  it("has a Change Version button", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    expect(
      screen.getByRole("button", { name: /change version/i }),
    ).toBeInTheDocument();
  });

  it("calls onChangeVersion with depot ID when a depot's change version button is clicked", () => {
    const onChangeVersion = mock();
    render(
      <GameDetail
        game={mockGame}
        onChangeVersion={onChangeVersion}
      />,
    );

    // Click the main Change Version button (uses first installed depot)
    fireEvent.click(screen.getByRole("button", { name: /change version/i }));
    expect(onChangeVersion).toHaveBeenCalledTimes(1);
    expect(onChangeVersion).toHaveBeenCalledWith("3321461");
  });

  it("displays installed depots with manifest info", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    // Installed depot should show the depot ID
    expect(screen.getByText(/3321461/)).toBeInTheDocument();
    // Installed depot should show the manifest
    expect(screen.getByText("744665017")).toBeInTheDocument();
  });

  it("displays non-installed depots from Steam with distinct styling", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    // Non-installed depot should be shown
    expect(screen.getByText(/3321462/)).toBeInTheDocument();
    expect(screen.getByText(/Crimson Desert DLC/)).toBeInTheDocument();

    // Non-installed depot should show "Not installed" label
    expect(screen.getAllByText(/not installed/i).length).toBeGreaterThan(0);
  });

  it("shows depot name from Steam when available", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    expect(screen.getByText(/Crimson Desert Content/)).toBeInTheDocument();
    expect(screen.getByText(/Crimson Desert DLC/)).toBeInTheDocument();
    expect(screen.getByText(/Crimson Desert Soundtrack/)).toBeInTheDocument();
  });

  it("gracefully shows only installed depots when list_depots fails", () => {
    mockUseDepotList.mockReturnValue(depotListError());

    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );

    // Installed depot should still show
    expect(screen.getByText(/3321461/)).toBeInTheDocument();
    expect(screen.getByText("744665017")).toBeInTheDocument();

    // Non-installed depots should not appear
    expect(screen.queryByText(/3321462/)).not.toBeInTheDocument();
  });

  it("shows loading state while fetching depots", () => {
    mockUseDepotList.mockReturnValue(depotListLoading());

    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );

    // Installed depots should still show
    expect(screen.getByText(/3321461/)).toBeInTheDocument();
  });

  it("marks non-installed depots with the not-installed modifier class", () => {
    const { container } = render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );

    const notInstalledDepots = container.querySelectorAll(
      ".game-detail__depot--not-installed",
    );
    // depot 3321462 and 3321463 are not installed
    expect(notInstalledDepots.length).toBe(2);
  });

  it("makes all depots selectable for version browsing", () => {
    const onChangeVersion = mock();
    const { container } = render(
      <GameDetail game={mockGame} onChangeVersion={onChangeVersion} />,
    );

    // Each depot should have a "Browse Versions" button
    const browseButtons = container.querySelectorAll(
      ".game-detail__depot-browse",
    );
    expect(browseButtons.length).toBe(3);

    // Click browse on a non-installed depot
    fireEvent.click(browseButtons[1]); // 3321462
    expect(onChangeVersion).toHaveBeenCalledWith("3321462");
  });

  it("shows max_size for non-installed depots", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    // depot 3321462 has max_size 5000000000 (~4.7 GB)
    expect(screen.getByText(/4\.7 GB/)).toBeInTheDocument();
    // depot 3321463 has max_size 800000000 (~762.9 MB)
    expect(screen.getByText(/762\.9 MB/)).toBeInTheDocument();
  });

  it("shows DLC badge when depot belongs to a DLC", () => {
    render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );
    // depot 3321462 has dlc_app_id "3321470"
    // The DLC badge shows "DLC 3321470" in a .game-detail__depot-dlc element
    expect(screen.getByText(/DLC 3321470/)).toBeInTheDocument();
  });

  it("depot cards have proper CSS class structure for styling", () => {
    const { container } = render(
      <GameDetail game={mockGame} onChangeVersion={mock()} />,
    );

    // All depots have the depot card class
    const depotCards = container.querySelectorAll(".game-detail__depot");
    expect(depotCards.length).toBe(3);

    // Each depot has a badges container
    const badgeContainers = container.querySelectorAll(".game-detail__depot-badges");
    expect(badgeContainers.length).toBe(3);

    // DLC badge has its own class
    const dlcBadges = container.querySelectorAll(".game-detail__depot-dlc");
    expect(dlcBadges.length).toBe(1);
  });
});
