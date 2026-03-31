import { describe, it, expect, mock, beforeEach } from "bun:test";
import { renderHook, waitFor, act } from "@testing-library/react";
import { useAuth } from "./useAuth";

const mockInvoke = mock() as any;

describe("useAuth", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    // Default: get_auth_state returns false
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(false);
      return Promise.resolve();
    });
  });

  it("starts in idle state with no error", async () => {
    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(result.current.authenticated).toBe(false);
    expect(result.current.submitting).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("checks auth state on mount via get_auth_state", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(true);
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));

    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(mockInvoke).toHaveBeenCalledWith("get_auth_state");
    expect(result.current.authenticated).toBe(true);
  });

  it("submits auth via set_credentials IPC command", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(false);
      if (cmd === "set_credentials") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.submit("testuser", "testpass");
    });

    expect(mockInvoke).toHaveBeenCalledWith("set_credentials", {
      username: "testuser",
      password: "testpass",
      guardCode: null,
    });
    expect(result.current.authenticated).toBe(true);
    expect(result.current.error).toBeNull();
  });

  it("submits auth with guard code when provided", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(false);
      if (cmd === "set_credentials") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.submit("testuser", "testpass", "ABC123");
    });

    expect(mockInvoke).toHaveBeenCalledWith("set_credentials", {
      username: "testuser",
      password: "testpass",
      guardCode: "ABC123",
    });
    expect(result.current.authenticated).toBe(true);
  });

  it("sets error state when submit fails", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(false);
      if (cmd === "set_credentials")
        return Promise.reject("Invalid credentials");
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
      if (cmd === "get_auth_state") return Promise.resolve(false);
      if (cmd === "set_credentials") return submitPromise;
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

  // Hypothesis: The bug occurs because set_credentials ignores the keychain-loaded
  // password in AuthStore, always using the password from IPC arguments. When the
  // sidecar session expires, the empty password causes login failure. The fix adds a
  // dedicated resume_session IPC command that uses the credentials already in AuthStore.
  it("resumeSession calls resume_session IPC instead of set_credentials", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(true);
      if (cmd === "get_username") return Promise.resolve("saveduser");
      if (cmd === "has_credentials") return Promise.resolve(true);
      if (cmd === "resume_session") return Promise.resolve();
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    expect(result.current.hasStoredCredentials).toBe(true);

    await act(async () => {
      await result.current.resumeSession();
    });

    // Should call resume_session, NOT set_credentials
    expect(mockInvoke).toHaveBeenCalledWith("resume_session");
    expect(result.current.authenticated).toBe(true);
    expect(result.current.error).toBeNull();
  });

  it("resumeSession sets error state on failure", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_auth_state") return Promise.resolve(true);
      if (cmd === "get_username") return Promise.resolve("saveduser");
      if (cmd === "has_credentials") return Promise.resolve(true);
      if (cmd === "resume_session") return Promise.reject("Session expired");
      return Promise.resolve();
    });

    const { result } = renderHook(() => useAuth(mockInvoke));
    await waitFor(() => expect(result.current.checking).toBe(false));

    await act(async () => {
      await result.current.resumeSession();
    });

    // authenticated stays true (credentials still exist), but error is set
    // so the UI can show the error message on the "Welcome back" screen
    expect(result.current.error).toBe("Session expired");
  });

  it("clears previous error on new submission", async () => {
    mockInvoke
      .mockImplementation((cmd: string) => {
        if (cmd === "get_auth_state") return Promise.resolve(false);
        if (cmd === "set_credentials")
          return Promise.reject("Invalid credentials");
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
      if (cmd === "get_auth_state") return Promise.resolve(false);
      if (cmd === "set_credentials") return Promise.resolve();
      return Promise.resolve();
    });

    await act(async () => {
      await result.current.submit("testuser", "rightpass");
    });
    expect(result.current.error).toBeNull();
    expect(result.current.authenticated).toBe(true);
  });
});
