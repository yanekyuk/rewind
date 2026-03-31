---
trigger: "Refactor the entire authentication system. Current auth is broken — keychain doesn't work on Linux, resume_session is unreliable, sidecar requires login before commands but app doesn't send it. Simplify to a single auth path: store username and password in localStorage (Tauri built-in storage), always send credentials with sidecar commands, add remember-me checkbox. Remove keychain integration, remove resume_session, remove complex AuthStore."
type: refactor
branch: refactor/simplify-auth
base-branch: main
created: 2026-03-31
version-bump: patch
---

## Related Files

### Rust backend (auth system to gut)
- src-tauri/src/application/auth.rs — AuthStore, keychain save/load/delete, username file persistence
- src-tauri/src/domain/auth.rs — Credentials struct, validate()
- src-tauri/src/lib.rs — IPC commands: set_credentials, resume_session, get_auth_state, get_username, has_credentials, clear_credentials; app startup keychain loading
- src-tauri/src/error.rs — RewindError::AuthRequired, AuthFailed variants
- src-tauri/src/infrastructure/sidecar.rs — SidecarState, SidecarHandle
- src-tauri/src/infrastructure/depot_downloader.rs — login(), list_manifests(), list_depots() functions

### Frontend (auth UI and hooks)
- src/hooks/useAuth.ts — useAuth hook with checking, authenticated, hasStoredCredentials, submit, resumeSession, signOut
- src/hooks/useAuth.test.ts — tests for useAuth
- src/components/LoginView.tsx — "Welcome back" flow, full login form, Steam Guard waiting
- src/components/LoginView.test.tsx — tests
- src/App.tsx — auth gate, onAuthRequired callback
- src/types/navigation.ts — ViewId union with "auth-gate"

### Sidecar
- sidecar/SteamKitSidecar/SteamSession.cs — Steam connection and login
- sidecar/SteamKitSidecar/Program.cs — daemon command dispatch, login command handling

## Relevant Docs
- docs/specs/credential-storage.md — Current keychain-based credential storage spec (to be superseded)
- docs/specs/auth-ui.md — Original auth UI spec (already superseded by steam-ui-overhaul)
- docs/specs/steam-ui-overhaul.md — Current UI spec including auth views
- docs/domain/steamkit-sidecar.md — Sidecar domain knowledge including auth flow

## Related Issues
None — no related issues found.

## Scope

### Problem
The auth system has 5 overlapping mechanisms that don't work together:
1. OS keychain via `keyring` crate — broken on Linux (DBus "name not activatable" error)
2. Plaintext username file on disk (`~/.config/rewind/username`)
3. In-memory AuthStore with Mutex (complex, fragile state)
4. `resume_session` IPC command (tries to use keychain credentials)
5. `get_or_saved()` empty-password fallback (sidecar rejects empty passwords)

The persistent sidecar (PR #25) requires a `login` command before any other command, but the app navigates to game detail and fires `list_depots`/`list_manifests` without logging in first. The "SteamClient instance must be connected" error in the screenshot confirms this.

### Solution
Replace everything with a single auth path:

**Storage:** Use Tauri's localStorage (frontend `window.localStorage`) to persist username and password. Add a "Remember me" checkbox — when checked, credentials persist across restarts; when unchecked, they're session-only.

**Login flow:**
1. User enters username + password (+ optional Steam Guard code)
2. Frontend sends `login` IPC command to sidecar
3. On success, if "remember me" is checked, save to localStorage
4. Frontend stores credentials in React state for the session

**Sidecar commands:** The sidecar daemon's `login` command is called once after app start (or after credentials are entered). All subsequent commands (list_depots, list_manifests, etc.) use the already-authenticated sidecar session — no credentials needed per-command.

**What to remove:**
- `keyring` crate dependency from Cargo.toml
- `save_to_keychain`, `load_from_keychain`, `delete_from_keychain` functions
- `save_username`, `load_username`, `clear_saved_username` (plaintext file)
- `AuthStore` struct entirely (or simplify to just hold session state)
- `resume_session` IPC command
- `has_credentials` IPC command
- `get_or_saved()` method
- The "Welcome back" flow in LoginView (replace with auto-login if localStorage has credentials)

**What to add/change:**
- Frontend localStorage read on mount → if credentials exist, auto-login to sidecar
- "Remember me" checkbox in LoginView
- Ensure sidecar `login` is called before any depot/manifest commands
- Simplify IPC surface: `login`, `logout`, `get_auth_state` (just checks if sidecar is authenticated)
