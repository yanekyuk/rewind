import { describe, it, expect, mock, beforeEach, afterEach } from "bun:test";
import { cleanup, renderHook, act, waitFor } from "@testing-library/react";
import { useDowngradeProgress } from "./useDowngradeProgress";

describe("useDowngradeProgress", () => {
  const mockListen = mock() as any;
  let mockUnlisten: ReturnType<typeof mock>;
  let capturedCallback: any;

  beforeEach(() => {
    mockUnlisten = mock();
    mockListen.mockReset();
    mockListen.mockImplementation((_event: string, callback: any) => {
      capturedCallback = callback;
      return Promise.resolve(mockUnlisten);
    });
  });

  afterEach(() => {
    cleanup();
    capturedCallback = undefined;
  });

  it("initializes with null phase", () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));
    expect(result.current.phase).toBeNull();
    expect(result.current.isActive).toBe(false);
  });

  it("sets up listener on mount", async () => {
    renderHook(() => useDowngradeProgress(mockListen));
    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("downgrade-progress", expect.any(Function));
    });
  });

  it("cleans up listener on unmount", async () => {
    const { unmount } = renderHook(() => useDowngradeProgress(mockListen));
    await waitFor(() => {
      expect(mockListen).toHaveBeenCalled();
    });
    unmount();
    expect(mockUnlisten).toHaveBeenCalled();
  });

  it("handles comparing phase", async () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));

    await waitFor(() => {
      expect(capturedCallback).toBeDefined();
    });

    act(() => {
      capturedCallback({
        payload: { phase: "comparing" },
      });
    });

    await waitFor(() => {
      expect(result.current.phase).toBe("comparing");
    });
    expect(result.current.isActive).toBe(true);
    expect(result.current.percent).toBeUndefined();
  });

  it("handles downloading phase with metrics", async () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));

    await waitFor(() => {
      expect(capturedCallback).toBeDefined();
    });

    act(() => {
      capturedCallback({
        payload: {
          phase: "downloading",
          percent: 50,
          bytes_downloaded: 5000000000,
          bytes_total: 10000000000,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.phase).toBe("downloading");
    });
    expect(result.current.percent).toBe(50);
    expect(result.current.bytesDownloaded).toBe(5000000000);
    expect(result.current.bytesTotal).toBe(10000000000);
    expect(result.current.isActive).toBe(true);
  });

  it("handles applying phase", async () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));

    await waitFor(() => {
      expect(capturedCallback).toBeDefined();
    });

    act(() => {
      capturedCallback({
        payload: { phase: "applying" },
      });
    });

    await waitFor(() => {
      expect(result.current.phase).toBe("applying");
    });
    expect(result.current.isActive).toBe(true);
  });

  it("handles complete phase", async () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));

    await waitFor(() => {
      expect(capturedCallback).toBeDefined();
    });

    act(() => {
      capturedCallback({
        payload: { phase: "complete" },
      });
    });

    await waitFor(() => {
      expect(result.current.phase).toBe("complete");
    });
    expect(result.current.isActive).toBe(false);
  });

  it("handles error phase with message", async () => {
    const { result } = renderHook(() => useDowngradeProgress(mockListen));

    await waitFor(() => {
      expect(capturedCallback).toBeDefined();
    });

    act(() => {
      capturedCallback({
        payload: {
          phase: "error",
          message: "Download failed",
        },
      });
    });

    await waitFor(() => {
      expect(result.current.phase).toBe("error");
    });
    expect(result.current.error).toBe("Download failed");
    expect(result.current.isActive).toBe(false);
  });
});
