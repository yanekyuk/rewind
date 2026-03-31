/**
 * Shared error utilities for handling Tauri IPC errors.
 *
 * Tauri serializes Rust enum errors (e.g. RewindError::AuthRequired("msg"))
 * as JSON objects like { AuthRequired: "msg" }. These utilities detect and
 * extract meaningful messages from such errors.
 */

/**
 * Check whether an error represents a RewindError::AuthRequired variant.
 *
 * Handles both serde-serialized objects ({ AuthRequired: "..." }) and
 * string errors containing "AuthRequired".
 */
export function isAuthRequiredError(err: unknown): boolean {
  if (err == null) return false;

  if (typeof err === "string") {
    return err.includes("AuthRequired");
  }

  if (err instanceof Error) {
    return err.message.includes("AuthRequired");
  }

  if (typeof err === "object") {
    return "AuthRequired" in (err as Record<string, unknown>);
  }

  return false;
}

/**
 * Extract a human-readable message from a Tauri IPC error.
 *
 * Handles:
 * - Error instances (uses .message)
 * - Plain strings (returned as-is)
 * - Serde-serialized enum variants like { AuthRequired: "msg" } (extracts first string value)
 * - Other objects (JSON-stringified)
 */
export function extractErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }

  if (typeof err === "string") {
    return err;
  }

  if (typeof err === "object" && err !== null) {
    const values = Object.values(err as Record<string, unknown>);
    const firstString = values.find((v) => typeof v === "string");
    if (typeof firstString === "string") {
      return firstString;
    }
    return JSON.stringify(err);
  }

  return String(err);
}
