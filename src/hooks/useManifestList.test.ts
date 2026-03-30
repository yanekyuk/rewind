import { describe, it, expect, mock, beforeEach } from "bun:test";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useManifestList } from "./useManifestList";

const mockInvoke = mock() as any;

describe("useManifestList", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("starts with empty state", () => {
    const { result } = renderHook(() => useManifestList(mockInvoke));

    expect(result.current.manifests).toEqual([]);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("fetches manifests with only appId and depotId (no credentials)", async () => {
    const mockManifests = [
      { manifest_id: "111", date: "2025-01-01" },
      { manifest_id: "222", date: "2025-01-02" },
    ];
    mockInvoke.mockResolvedValue(mockManifests);

    const { result } = renderHook(() => useManifestList(mockInvoke));

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    // Verify IPC call has only appId and depotId -- no username/password
    expect(mockInvoke).toHaveBeenCalledWith("list_manifests", {
      appId: "3321460",
      depotId: "3321461",
    });
    expect(result.current.manifests).toEqual(mockManifests);
    expect(result.current.error).toBeNull();
  });

  it("sets error state on failure", async () => {
    mockInvoke.mockRejectedValue("Auth required");

    const { result } = renderHook(() => useManifestList(mockInvoke));

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBe("Auth required");
    expect(result.current.manifests).toEqual([]);
  });

  it("clears previous state on new fetch", async () => {
    const mockManifests = [{ manifest_id: "111", date: "2025-01-01" }];
    mockInvoke.mockResolvedValue(mockManifests);

    const { result } = renderHook(() => useManifestList(mockInvoke));

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.manifests).toHaveLength(1);

    // Fetch again
    const newManifests = [{ manifest_id: "333", date: "2025-02-01" }];
    mockInvoke.mockResolvedValue(newManifests);

    act(() => {
      result.current.fetch("440", "441");
    });

    // Should reset manifests during loading
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.manifests).toEqual(newManifests);
  });
});
