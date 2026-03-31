import { describe, it, expect, mock, afterEach } from "bun:test";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { LoginView } from "./LoginView";
import type { UseAuthResult } from "../hooks/useAuth";

function makeAuth(overrides: Partial<UseAuthResult> = {}): UseAuthResult {
  return {
    checking: false,
    authenticated: false,
    username: null,
    hasStoredCredentials: false,
    submitting: false,
    error: null,
    submit: mock(),
    resumeSession: mock(),
    signOut: mock(),
    ...overrides,
  };
}

describe("LoginView", () => {
  afterEach(cleanup);

  it("renders the Rewind branding", () => {
    render(<LoginView auth={makeAuth()} />);
    expect(screen.getByText("Rewind")).toBeInTheDocument();
  });

  it("renders username and password fields", () => {
    render(<LoginView auth={makeAuth()} />);
    expect(screen.getByLabelText(/account name/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it("renders password field with type password", () => {
    render(<LoginView auth={makeAuth()} />);
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "password");
  });

  it("shows Steam Guard waiting indicator while submitting", () => {
    render(<LoginView auth={makeAuth({ submitting: true })} />);
    expect(screen.getByText(/waiting for steam guard/i)).toBeInTheDocument();
  });

  it("calls submit with username and password on form submit", async () => {
    const submitMock = mock();
    render(<LoginView auth={makeAuth({ submit: submitMock })} />);

    fireEvent.change(screen.getByLabelText(/account name/i), {
      target: { value: "testuser" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "testpass" },
    });
    fireEvent.submit(screen.getByRole("form", { name: /steam authentication/i }));

    await waitFor(() => {
      expect(submitMock).toHaveBeenCalledWith("testuser", "testpass", undefined);
    });
  });

  it("displays error message when auth fails", () => {
    render(<LoginView auth={makeAuth({ error: "Invalid credentials" })} />);
    expect(screen.getByText("Invalid credentials")).toBeInTheDocument();
  });

  it("shows checking state while verifying existing session", () => {
    render(<LoginView auth={makeAuth({ checking: true })} />);
    expect(screen.getByText(/checking/i)).toBeInTheDocument();
  });

  it("shows Welcome back UI when stored credentials exist", () => {
    render(
      <LoginView
        auth={makeAuth({
          hasStoredCredentials: true,
          username: "steamuser",
        })}
      />,
    );
    expect(screen.getByText(/welcome back/i)).toBeInTheDocument();
    expect(screen.getByText("steamuser")).toBeInTheDocument();
  });

  it("calls resumeSession (not submit) when Welcome back form is submitted", async () => {
    const resumeMock = mock();
    const submitMock = mock();
    render(
      <LoginView
        auth={makeAuth({
          hasStoredCredentials: true,
          username: "steamuser",
          resumeSession: resumeMock,
          submit: submitMock,
        })}
      />,
    );

    fireEvent.submit(screen.getByRole("form", { name: /resume session/i }));

    await waitFor(() => {
      expect(resumeMock).toHaveBeenCalled();
    });
    // submit should NOT have been called — resumeSession handles the Welcome back flow
    expect(submitMock).not.toHaveBeenCalled();
  });

  it("shows Sign in with different account option on Welcome back", () => {
    const signOutMock = mock();
    render(
      <LoginView
        auth={makeAuth({
          hasStoredCredentials: true,
          username: "steamuser",
          signOut: signOutMock,
        })}
      />,
    );
    expect(screen.getByText(/different account/i)).toBeInTheDocument();
  });

  it("displays error on Welcome back UI when resumeSession fails", () => {
    render(
      <LoginView
        auth={makeAuth({
          hasStoredCredentials: true,
          username: "steamuser",
          error: "Session expired",
        })}
      />,
    );
    expect(screen.getByText("Session expired")).toBeInTheDocument();
  });
});
