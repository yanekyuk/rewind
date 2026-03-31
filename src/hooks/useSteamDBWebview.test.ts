import { describe, it, expect, mock, beforeEach, afterEach } from "bun:test";
import { cleanup, renderHook, act, waitFor } from "@testing-library/react";

const mockInvoke = mock();
const mockListen = mock();
let listeners: Record<string, (event: { payload: unknown }) => void> = {};

mock.module("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));
mock.module("@tauri-apps/api/event", () => ({
  listen: (event: string, callback: (event: { payload: unknown }) => void) => {
    listeners[event] = callback;
    return Promise.resolve(() => {
      delete listeners[event];
    });
  },
}));

const { useSteamDBWebview } = await import("./useSteamDBWebview");

describe("useSteamDBWebview", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
    listeners = {};
  });

  afterEach(cleanup);

  it("starts with empty state", () => {
    const { result } = renderHook(() => useSteamDBWebview());
    expect(result.current.manifests).toEqual([]);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("sets loading state when opening webview", async () => {
    const { result } = renderHook(() => useSteamDBWebview());

    await act(async () => {
      await result.current.open("12345");
    });

    expect(mockInvoke).toHaveBeenCalledWith("open_steamdb_webview", {
      depotId: "12345",
    });
  });

  it("sets error when open_steamdb_webview fails", async () => {
    mockInvoke.mockRejectedValue("Webview creation failed");
    const { result } = renderHook(() => useSteamDBWebview());

    await act(async () => {
      await result.current.open("12345");
    });

    expect(result.current.error).toBe("Webview creation failed");
    expect(result.current.loading).toBe(false);
  });

  it("receives manifests from steamdb-manifests event", async () => {
    const { result } = renderHook(() => useSteamDBWebview());

    // Wait for listeners to be registered
    await waitFor(() => {
      expect(listeners["steamdb-manifests"]).toBeDefined();
    });

    // Open to set loading state
    await act(async () => {
      await result.current.open("12345");
    });
    expect(result.current.loading).toBe(true);

    // Simulate event from webview
    const manifests = [
      { manifest_id: "111", date: "15 March 2026", branch: "public" },
      { manifest_id: "222", date: "10 March 2026", branch: null },
    ];
    act(() => {
      listeners["steamdb-manifests"]({ payload: manifests });
    });

    expect(result.current.manifests).toEqual(manifests);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("receives error from steamdb-manifests-error event", async () => {
    const { result } = renderHook(() => useSteamDBWebview());

    await waitFor(() => {
      expect(listeners["steamdb-manifests-error"]).toBeDefined();
    });

    await act(async () => {
      await result.current.open("12345");
    });

    act(() => {
      listeners["steamdb-manifests-error"]({ payload: "Extraction failed" });
    });

    expect(result.current.error).toBe("Extraction failed");
    expect(result.current.loading).toBe(false);
  });

  it("calls close_steamdb_webview on close", async () => {
    const { result } = renderHook(() => useSteamDBWebview());

    await act(async () => {
      await result.current.close("12345");
    });

    expect(mockInvoke).toHaveBeenCalledWith("close_steamdb_webview", {
      depotId: "12345",
    });
  });

  it("ignores close errors gracefully", async () => {
    mockInvoke.mockRejectedValue("Window not found");
    const { result } = renderHook(() => useSteamDBWebview());

    // Should not throw
    await act(async () => {
      await result.current.close("12345");
    });

    expect(result.current.error).toBeNull();
  });

  it("cleans up listeners on unmount", async () => {
    const { unmount } = renderHook(() => useSteamDBWebview());

    await waitFor(() => {
      expect(listeners["steamdb-manifests"]).toBeDefined();
    });

    unmount();

    // Cleanup runs via .then() so wait for microtasks to flush
    await waitFor(() => {
      expect(listeners["steamdb-manifests"]).toBeUndefined();
    });
    expect(listeners["steamdb-manifests-error"]).toBeUndefined();
  });
});
