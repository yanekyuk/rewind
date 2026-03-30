import { describe, it, expect, mock, beforeEach, afterEach, afterAll } from "bun:test";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";

const mockSubmit = mock();
const mockUseAuth = mock();

mock.module("../hooks/useAuth", () => ({
  useAuth: () => mockUseAuth(),
}));

const { LoginView } = await import("./LoginView");

afterAll(() => mock.restore());

describe("LoginView", () => {
  afterEach(cleanup);

  beforeEach(() => {
    mockSubmit.mockReset();
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: null,
      submit: mockSubmit,
      signOut: mock(),
    });
  });

  it("renders the Rewind branding", () => {
    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByText("Rewind")).toBeInTheDocument();
  });

  it("renders username and password fields", () => {
    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByLabelText("Username")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it("renders password field with type password", () => {
    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByLabelText("Password")).toHaveAttribute("type", "password");
  });

  it("shows Steam Guard waiting indicator while submitting", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: true,
      error: null,
      submit: mockSubmit,
      signOut: mock(),
    });

    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByText(/waiting for steam guard/i)).toBeInTheDocument();
  });

  it("calls submit with username and password on form submit", async () => {
    mockSubmit.mockResolvedValue(undefined);
    render(<LoginView onAuthenticated={mock()} />);

    fireEvent.change(screen.getByLabelText("Username"), {
      target: { value: "testuser" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "testpass" },
    });
    fireEvent.submit(screen.getByRole("form"));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith("testuser", "testpass", undefined);
    });
  });

  it("displays error message when auth fails", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: "Invalid credentials",
      submit: mockSubmit,
      signOut: mock(),
    });

    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByText("Invalid credentials")).toBeInTheDocument();
  });

  it("calls onAuthenticated when auth succeeds", () => {
    const onAuthenticated = mock();
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: mockSubmit,
      signOut: mock(),
    });

    render(<LoginView onAuthenticated={onAuthenticated} />);
    expect(onAuthenticated).toHaveBeenCalled();
  });

  it("shows checking state while verifying existing session", () => {
    mockUseAuth.mockReturnValue({
      checking: true,
      authenticated: false,
      submitting: false,
      error: null,
      submit: mockSubmit,
      signOut: mock(),
    });

    render(<LoginView onAuthenticated={mock()} />);
    expect(screen.getByText(/checking/i)).toBeInTheDocument();
  });
});
