import { describe, it, expect, mock, beforeEach } from "bun:test";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useGameList } from "./useGameList";
import type { GameInfo } from "../types/game";

const mockInvoke = mock() as any;

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

describe("useGameList", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("starts in loading state", () => {
    mockInvoke.mockReturnValue(new Promise(() => {})); // never resolves
    const { result } = renderHook(() => useGameList(mockInvoke));

    expect(result.current.loading).toBe(true);
    expect(result.current.games).toEqual([]);
    expect(result.current.error).toBeNull();
  });

  it("fetches games on mount via invoke('list_games')", async () => {
    mockInvoke.mockResolvedValue(mockGames);
    const { result } = renderHook(() => useGameList(mockInvoke));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockInvoke).toHaveBeenCalledWith("list_games");
    expect(result.current.games).toEqual(mockGames);
    expect(result.current.error).toBeNull();
  });

  it("sets error state when invoke fails", async () => {
    mockInvoke.mockRejectedValue(new Error("Steam not found"));
    const { result } = renderHook(() => useGameList(mockInvoke));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("Steam not found");
    expect(result.current.games).toEqual([]);
  });

  it("can retry after an error", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("Steam not found"));
    const { result } = renderHook(() => useGameList(mockInvoke));

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error).toBe("Steam not found");

    mockInvoke.mockResolvedValue(mockGames);
    act(() => {
      result.current.retry();
    });

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.games).toEqual(mockGames);
    expect(result.current.error).toBeNull();
  });

  it("handles non-Error rejection values", async () => {
    mockInvoke.mockRejectedValue("string error");
    const { result } = renderHook(() => useGameList(mockInvoke));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("Failed to load games");
    expect(result.current.games).toEqual([]);
  });
});
