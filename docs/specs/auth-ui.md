---
title: "Authentication UI and IPC"
type: spec
tags: [auth, credentials, ipc, frontend, ui, steamkit, steam-guard]
created: 2026-03-30
updated: 2026-03-31
---

# Authentication UI and IPC

## Behavior

Rewind collects Steam credentials (username, password, and optional Steam Guard 2FA code) from the user via an in-app form and passes them to the Rust backend through a Tauri IPC command. The backend authenticates with Steam via the SteamKit sidecar and stores credentials in-memory for subsequent operations.

### Backend

- A `set_credentials` Tauri IPC command accepts `username`, `password`, and an optional `guard_code` (Steam Guard 2FA).
- On submit, the backend spawns the SteamKit sidecar `login` command to perform actual Steam authentication. This is where phone approval / Steam Guard 2FA occurs.
- On successful authentication, the sidecar saves a session token to disk (`~/.local/share/rewind/sessions/<username>.json`). Subsequent sidecar commands reuse this token without re-authenticating.
- Credentials are also stored in Tauri-managed application state (`tauri::State`) using a `Mutex`-protected struct for use in subsequent sidecar invocations.
- If no credentials are stored when a sidecar operation is attempted, the command returns a typed `AuthRequired` error.

### Frontend

- An `AuthInput` component provides input fields for username, password, and Steam Guard code.
- The password field uses `type="password"` for masking.
- The Steam Guard code field is shown conditionally -- only when 2FA is required (initially hidden, shown after a 2FA-required error or user toggle).
- A `useAuth` hook encapsulates the `set_credentials` IPC call and manages auth state (idle, submitting, authenticated, error).
- Auth errors (invalid credentials, 2FA required, network issues) are displayed as user-friendly messages.

### Integration

- Auth gates the "Comparing Versions" step -- the user cannot proceed to manifest diff without stored credentials.
- The SteamKit sidecar persists session tokens to disk. If a valid session exists from a previous run, the sidecar reuses it without re-prompting for 2FA.
- If a saved session expires, the sidecar deletes it, reconnects, and performs fresh credential authentication.

## Constraints

- Credentials (username/password) are stored in-memory only. The sidecar's session token file contains a refresh token, not the raw password.
- Credentials must not appear in logs, debug output, or frontend state that could be inspected.
- The `set_credentials` IPC command must validate that username and password are non-empty before storing.
- The backend credential store must be thread-safe (protected by `Mutex` or equivalent).
- All Tauri IPC communication must use strongly-typed serde serialization.

## Acceptance Criteria

1. User can enter Steam username and password in the auth form and submit.
2. The `set_credentials` IPC command spawns the SteamKit sidecar `login` command and authenticates with Steam.
3. Phone approval / Steam Guard 2FA occurs during sign-in, not during subsequent operations.
4. On success, credentials are stored in-memory and a session token is saved by the sidecar.
5. If credentials are not set when needed, the backend returns a typed `AuthRequired` error.
6. The password field is masked in the UI.
7. The Steam Guard code field is conditionally visible (shown only when needed).
8. Auth errors are displayed as clear, user-friendly messages.
9. The auth step gates downstream operations -- manifest diff and download cannot proceed without credentials.
