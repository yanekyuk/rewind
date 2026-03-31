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

  // Hypothesis: The bug occurs because Tauri IPC serializes RewindError::AuthRequired
  // as { AuthRequired: "msg" } (serde enum). String() on this object produces
  // "[object Object]" instead of a readable message. Additionally, AuthRequired errors
  // should trigger the onAuthRequired callback to redirect to login.

  it("extracts message from serde-serialized error objects instead of [object Object]", async () => {
    // Tauri IPC serializes RewindError::AuthRequired as { AuthRequired: "msg" }
    mockInvoke.mockRejectedValue({ Infrastructure: "Network error" });

    const { result } = renderHook(() => useManifestList(mockInvoke));

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    await waitFor(() => expect(result.current.loading).toBe(false));

    // Should extract the message, not show "[object Object]"
    expect(result.current.error).toBe("Network error");
  });

  it("calls onAuthRequired when an AuthRequired error is received", async () => {
    // Simulate Tauri IPC error for RewindError::AuthRequired
    mockInvoke.mockRejectedValue({
      AuthRequired: "No credentials available. Please sign in.",
    });

    const onAuthRequired = mock() as any;
    const { result } = renderHook(() =>
      useManifestList(mockInvoke, { onAuthRequired }),
    );

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    await waitFor(() => expect(onAuthRequired).toHaveBeenCalledTimes(1));
  });

  it("does not call onAuthRequired for non-auth errors", async () => {
    mockInvoke.mockRejectedValue({ Infrastructure: "Network error" });

    const onAuthRequired = mock() as any;
    const { result } = renderHook(() =>
      useManifestList(mockInvoke, { onAuthRequired }),
    );

    act(() => {
      result.current.fetch("3321460", "3321461");
    });

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(onAuthRequired).not.toHaveBeenCalled();
    expect(result.current.error).toBe("Network error");
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
