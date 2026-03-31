---
title: "Encrypted Credential Storage"
type: spec
tags: [auth, credentials, keyring, keychain, encryption, persistence]
created: 2026-03-31
updated: 2026-03-31
---

# Encrypted Credential Storage

## Behavior

Store the user's Steam password securely in the OS keychain so they do not have to re-enter it on every app restart. The `keyring` Rust crate provides cross-platform access to the OS credential store (DPAPI on Windows, Keychain on macOS, libsecret/kwallet on Linux).

### Backend

- On successful `set_credentials`, save the password to the OS keychain via the `keyring` crate (service: `rewind`, user: the Steam username). Continue to save the username to the plaintext file as before.
- On app startup (`run()`), attempt to load the saved username from disk. If a username exists, attempt to load the password from the OS keychain. If both succeed, populate the `AuthStore` with full `Credentials` (username + password) so that `get_or_saved()` returns usable credentials with a real password instead of an empty-password fallback.
- On `clear_credentials`, delete the password from the OS keychain in addition to clearing the in-memory store and the saved username file.
- If keychain access fails at any point (e.g., no keyring daemon on Linux, access denied), log a warning and fall back to the current username-only behavior. The app must never crash or block due to keychain errors.

### Frontend

- When `get_auth_state` returns `true` and `get_username` returns a username, the `LoginView` shows a "Welcome back, \<username\>" message with a "Sign in" button (no password re-entry needed) and a "Sign in as different user" link.
- The "Sign in" button triggers `resume_session`, a dedicated IPC command that uses the credentials already loaded in the backend `AuthStore` (from the OS keychain at startup). No password crosses the IPC boundary. If the sidecar session is still valid, the user proceeds without re-authenticating. If the session has expired, the backend re-authenticates using the stored password automatically.
- The "Sign in as different user" link clears credentials and shows the full login form.
- A `has_credentials` IPC command returns whether full credentials (username + password) are stored, distinguishing between "saved session only" and "full credentials available."

### Integration

- The `keyring` crate is added as a dependency in `Cargo.toml`.
- The keyring service name is `rewind` and the keyring user is the Steam username.
- All keyring operations are wrapped in error handling that degrades gracefully.

## Constraints

- Password must never be stored in plaintext on disk. The OS keychain handles encryption.
- Must work cross-platform: Linux (libsecret/kwallet), macOS (Keychain), Windows (DPAPI/Credential Manager).
- Graceful fallback: if the keychain is unavailable, degrade to current username-only behavior.
- Credentials must not appear in logs, debug output, or frontend state.
- Thread-safe access: all keychain operations happen outside the `Mutex` lock to avoid potential deadlocks with OS-level keychain dialogs.

## Acceptance Criteria

1. On successful login, the password is saved to the OS keychain.
2. On app restart with a saved username, the password is loaded from the keychain and `AuthStore` is pre-populated with full credentials.
3. `get_or_saved()` returns credentials with a real password when keychain credentials exist.
4. On sign-out, the password is deleted from the OS keychain.
5. If the keychain is unavailable, the app falls back to username-only behavior (no crash, no blocking).
6. The frontend shows "Welcome back, \<username\>" when stored credentials exist.
7. The user can choose "Sign in as different user" to clear stored credentials and show the full login form.
8. All existing tests continue to pass.
9. New tests cover keychain save/load/delete operations (using mocked or trait-based keychain access for unit testing).
