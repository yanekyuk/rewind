import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { AuthInput } from "./AuthInput";

// Mock the useAuth hook
const mockSubmit = vi.fn();
const mockUseAuth = vi.fn();

vi.mock("../hooks/useAuth", () => ({
  useAuth: () => mockUseAuth(),
}));

describe("AuthInput", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockSubmit.mockReset();
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: null,
      submit: mockSubmit,
    });
  });

  it("renders username and password fields", () => {
    render(<AuthInput />);

    expect(screen.getByLabelText("Username")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
  });

  it("renders password field with type password for masking", () => {
    render(<AuthInput />);

    const passwordInput = screen.getByLabelText("Password");
    expect(passwordInput).toHaveAttribute("type", "password");
  });

  it("does not show Steam Guard field by default", () => {
    render(<AuthInput />);

    expect(screen.queryByLabelText("Steam Guard Code")).not.toBeInTheDocument();
  });

  it("shows Steam Guard field when toggle is clicked", () => {
    render(<AuthInput />);

    const toggle = screen.getByRole("button", {
      name: /steam guard/i,
    });
    fireEvent.click(toggle);

    expect(screen.getByLabelText("Steam Guard Code")).toBeInTheDocument();
  });

  it("calls submit with username and password on form submit", async () => {
    mockSubmit.mockResolvedValue(undefined);
    render(<AuthInput />);

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

  it("calls submit with guard code when provided", async () => {
    mockSubmit.mockResolvedValue(undefined);
    render(<AuthInput />);

    fireEvent.change(screen.getByLabelText("Username"), {
      target: { value: "testuser" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "testpass" },
    });

    // Show and fill Steam Guard field
    fireEvent.click(screen.getByRole("button", { name: /steam guard/i }));
    fireEvent.change(screen.getByLabelText("Steam Guard Code"), {
      target: { value: "ABC123" },
    });
    fireEvent.submit(screen.getByRole("form"));

    await waitFor(() => {
      expect(mockSubmit).toHaveBeenCalledWith("testuser", "testpass", "ABC123");
    });
  });

  it("displays error message when auth fails", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: false,
      error: "Invalid credentials",
      submit: mockSubmit,
    });

    render(<AuthInput />);

    expect(screen.getByText("Invalid credentials")).toBeInTheDocument();
  });

  it("disables submit button while submitting", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: false,
      submitting: true,
      error: null,
      submit: mockSubmit,
    });

    render(<AuthInput />);

    const submitButton = screen.getByRole("button", { name: /signing in/i });
    expect(submitButton).toBeDisabled();
  });

  it("shows authenticated state when already authenticated", () => {
    mockUseAuth.mockReturnValue({
      checking: false,
      authenticated: true,
      submitting: false,
      error: null,
      submit: mockSubmit,
    });

    render(<AuthInput />);

    expect(screen.getByText(/authenticated/i)).toBeInTheDocument();
  });

  it("shows loading state while checking auth", () => {
    mockUseAuth.mockReturnValue({
      checking: true,
      authenticated: false,
      submitting: false,
      error: null,
      submit: mockSubmit,
    });

    render(<AuthInput />);

    expect(screen.getByText(/checking/i)).toBeInTheDocument();
  });
});
