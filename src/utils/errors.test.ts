import { describe, it, expect } from "bun:test";
import { isAuthRequiredError, extractErrorMessage } from "./errors";

describe("isAuthRequiredError", () => {
  it("detects serde-serialized AuthRequired object", () => {
    const err = { AuthRequired: "No credentials available. Please sign in." };
    expect(isAuthRequiredError(err)).toBe(true);
  });

  it("detects AuthRequired string", () => {
    expect(isAuthRequiredError("AuthRequired")).toBe(true);
  });

  it("detects string containing AuthRequired", () => {
    expect(isAuthRequiredError("Authentication required: AuthRequired")).toBe(true);
  });

  it("returns false for other error objects", () => {
    const err = { Infrastructure: "Network error" };
    expect(isAuthRequiredError(err)).toBe(false);
  });

  it("returns false for plain strings without AuthRequired", () => {
    expect(isAuthRequiredError("Something went wrong")).toBe(false);
  });

  it("returns false for Error instances without AuthRequired", () => {
    expect(isAuthRequiredError(new Error("timeout"))).toBe(false);
  });

  it("returns false for null", () => {
    expect(isAuthRequiredError(null)).toBe(false);
  });

  it("returns false for undefined", () => {
    expect(isAuthRequiredError(undefined)).toBe(false);
  });
});

describe("extractErrorMessage", () => {
  it("extracts message from serde-serialized error object", () => {
    const err = { AuthRequired: "No credentials available. Please sign in." };
    expect(extractErrorMessage(err)).toBe(
      "No credentials available. Please sign in.",
    );
  });

  it("extracts message from Error instance", () => {
    expect(extractErrorMessage(new Error("something broke"))).toBe(
      "something broke",
    );
  });

  it("returns string errors as-is", () => {
    expect(extractErrorMessage("plain error")).toBe("plain error");
  });

  it("extracts first value from generic serde-serialized error object", () => {
    const err = { Infrastructure: "File not found" };
    expect(extractErrorMessage(err)).toBe("File not found");
  });

  it("JSON-stringifies objects with no string values", () => {
    const err = { code: 42 };
    expect(extractErrorMessage(err)).toBe('{"code":42}');
  });

  it("handles null", () => {
    expect(extractErrorMessage(null)).toBe("null");
  });

  it("handles undefined", () => {
    expect(extractErrorMessage(undefined)).toBe("undefined");
  });
});
