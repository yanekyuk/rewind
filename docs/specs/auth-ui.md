---
title: "Authentication UI and IPC"
type: spec
tags: [auth, credentials, ipc, frontend, ui, depotdownloader, steam-guard]
created: 2026-03-30
updated: 2026-03-30
---

# Authentication UI and IPC

## Behavior

Rewind collects Steam credentials (username, password, and optional Steam Guard 2FA code) from the user via an in-app form and passes them to the Rust backend through a Tauri IPC command. The backend stores credentials in-memory for the duration of the session and injects them as command-line arguments when spawning DepotDownloader.

### Backend

- A `set_credentials` Tauri IPC command accepts `username`, `password`, and an optional `guard_code` (Steam Guard 2FA).
- Credentials are stored in Tauri-managed application state (`tauri::State`) using a `Mutex`-protected struct. They exist only in memory -- never written to disk.
- When `spawn_depot_downloader()` is called, it reads the stored credentials and appends `-username <user>`, `-password <pass>`, and `-remember-password` to the argument list.
- If no credentials are stored when a DepotDownloader operation is attempted, the command returns a typed error indicating authentication is required.

### Frontend

- An `AuthInput` component provides input fields for username, password, and Steam Guard code.
- The password field uses `type="password"` for masking.
- The Steam Guard code field is shown conditionally -- only when 2FA is required (initially hidden, shown after a 2FA-required error or user toggle).
- A `useAuth` hook encapsulates the `set_credentials` IPC call and manages auth state (idle, submitting, authenticated, error).
- Auth errors (invalid credentials, 2FA required, network issues) are displayed as user-friendly messages.

### Integration

- Auth gates the "Comparing Versions" step -- the user cannot proceed to manifest diff without stored credentials.
- DepotDownloader's `-remember-password` flag caches the session. If a session cache exists from a previous run, DepotDownloader reuses it without re-prompting for credentials.

## Constraints

- Credentials must never be persisted to disk by Rewind. Only DepotDownloader's own session cache (via `-remember-password`) is retained.
- Credentials must not appear in logs, debug output, or frontend state that could be inspected.
- The `set_credentials` IPC command must validate that username and password are non-empty before storing.
- The backend credential store must be thread-safe (protected by `Mutex` or equivalent).
- All Tauri IPC communication must use strongly-typed serde serialization.

## Acceptance Criteria

1. User can enter Steam username and password in the auth form and submit.
2. The `set_credentials` IPC command stores credentials in-memory and returns success.
3. When DepotDownloader is spawned, it receives `-username`, `-password`, and `-remember-password` arguments from stored credentials.
4. If credentials are not set when needed, the backend returns a typed `AuthRequired` error.
5. The password field is masked in the UI.
6. The Steam Guard code field is conditionally visible (shown only when needed).
7. Auth errors are displayed as clear, user-friendly messages.
8. Credentials are never written to disk by Rewind.
9. The auth step gates downstream operations -- manifest diff and download cannot proceed without credentials.
