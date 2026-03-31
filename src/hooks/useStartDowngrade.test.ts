import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useStartDowngrade } from "./useStartDowngrade";

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

describe("useStartDowngrade", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockClear();
  });

  it("initializes with starting=false and error=null", () => {
    const { result } = renderHook(() => useStartDowngrade());
    expect(result.current.starting).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("calls start_downgrade IPC command with params", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const { result } = renderHook(() => useStartDowngrade());

    const params = {
      app_id: "3321460",
      depot_id: "3321461",
      target_manifest_id: "1234567890",
      current_manifest_id: "9876543210",
      latest_buildid: "22560074",
      latest_manifest_id: "9876543210",
      latest_size: "133575233011",
      install_path: "/path/to/game",
      steamapps_path: "/path/to/steamapps",
    };

    await result.current.start(params);

    expect(mockInvoke).toHaveBeenCalledWith(
      "start_downgrade",
      expect.objectContaining({
        app_id: "3321460",
        depot_id: "3321461",
      })
    );
  });

  it("handles IPC command success", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const { result } = renderHook(() => useStartDowngrade());

    const params = {
      app_id: "3321460",
      depot_id: "3321461",
      target_manifest_id: "1234567890",
      current_manifest_id: "9876543210",
      latest_buildid: "22560074",
      latest_manifest_id: "9876543210",
      latest_size: "133575233011",
      install_path: "/path/to/game",
      steamapps_path: "/path/to/steamapps",
    };

    await result.current.start(params);

    expect(result.current.starting).toBe(false);
  });

  it("handles IPC command error", async () => {
    const errorMsg = "Authentication required";
    mockInvoke.mockRejectedValueOnce(new Error(errorMsg));
    const { result } = renderHook(() => useStartDowngrade());

    const params = {
      app_id: "3321460",
      depot_id: "3321461",
      target_manifest_id: "1234567890",
      current_manifest_id: "9876543210",
      latest_buildid: "22560074",
      latest_manifest_id: "9876543210",
      latest_size: "133575233011",
      install_path: "/path/to/game",
      steamapps_path: "/path/to/steamapps",
    };

    await result.current.start(params);

    await waitFor(() => {
      expect(result.current.error).toBe(errorMsg);
    });
    expect(result.current.starting).toBe(false);
  });
});
