import { describe, it, expect, mock, beforeEach } from "bun:test";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useAuth } from "./useAuth";

const mockInvoke = mock() as any;

describe("useAuth", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    // Default: check_session returns null (no saved session)
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      return Promise.resolve();
    });
  });

  it("starts in checking state", () => {
    const { result } = renderHook(() => useAuth(mockInvoke));
    expect(result.current.checking).toBe(true);
  });

  it("finishes checking with no auth when no saved session", async () => {
    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(result.current.authenticated).toBe(false);
    expect(result.current.username).toBeNull();
    expect(result.current.submitting).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("checks for saved session on mount via check_session", async () => {
    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(mockInvoke).toHaveBeenCalledWith("check_session");
  });

  it("auto-authenticates when check_session returns a username", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve("saveduser");
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(result.current.authenticated).toBe(true);
    expect(result.current.username).toBe("saveduser");
  });

  it("handles check_session rejection gracefully", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.reject("No session");
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(result.current.authenticated).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("submits auth via login IPC command", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.submit("testuser", "testpass");
    });

    expect(mockInvoke).toHaveBeenCalledWith("login", {
      username: "testuser",
      password: "testpass",
      guardCode: null,
    });
    expect(result.current.authenticated).toBe(true);
    expect(result.current.username).toBe("testuser");
    expect(result.current.error).toBeNull();
  });

  it("submits auth with guard code when provided", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.submit("testuser", "testpass", "ABC123");
    });

    expect(mockInvoke).toHaveBeenCalledWith("login", {
      username: "testuser",
      password: "testpass",
      guardCode: "ABC123",
    });
    expect(result.current.authenticated).toBe(true);
  });

  it("sets error state when submit fails", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return Promise.reject("Invalid credentials");
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.submit("testuser", "wrongpass");
    });

    expect(result.current.authenticated).toBe(false);
    expect(result.current.error).toBe("Invalid credentials");
  });

  it("sets submitting state during submission", async () => {
    let resolveSubmit: () => void;
    const submitPromise = new Promise<void>((resolve) => {
      resolveSubmit = resolve;
    });

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return submitPromise;
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    let submitDone: Promise<void>;
    act(() => {
      submitDone = result.current.submit("testuser", "testpass");
    });

    expect(result.current.submitting).toBe(true);

    await act(async () => {
      resolveSubmit!();
      await submitDone!;
    });

    expect(result.current.submitting).toBe(false);
    expect(result.current.authenticated).toBe(true);
  });

  it("clears previous error on new submission", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return Promise.reject("Invalid credentials");
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    // First attempt fails
    await act(async () => {
      await result.current.submit("testuser", "wrongpass");
    });
    expect(result.current.error).toBe("Invalid credentials");

    // Second attempt succeeds
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve(null);
      if (cmd === "login") return Promise.resolve();
      return Promise.resolve();
    });

    await act(async () => {
      await result.current.submit("testuser", "rightpass");
    });
    expect(result.current.error).toBeNull();
    expect(result.current.authenticated).toBe(true);
  });

  it("signOut calls logout IPC and clears state", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "check_session") return Promise.resolve("testuser");
      if (cmd === "logout") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.authenticated).toBe(true));

    await act(async () => {
      await result.current.signOut();
    });

    expect(mockInvoke).toHaveBeenCalledWith("logout");
    expect(result.current.authenticated).toBe(false);
    expect(result.current.username).toBeNull();
  });
});
