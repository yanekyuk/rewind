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
Replace the 5-mechanism auth system with a two-tier approach: **SteamKit RefreshToken persistence (primary)** with **localStorage username+password as fallback**.

#### Primary: RefreshToken + GuardData persistence
SteamKit's auth API (see `SteamKit/Samples/000_Authentication`) supports persistent sessions:
- On successful login, SteamKit returns a `RefreshToken` (JWT) and `NewGuardData` (skips Steam Guard on next login)
- Store these in the sidecar's data directory (e.g., `~/.config/rewind/tokens/<username>.json`)
- On subsequent app starts, the sidecar loads the RefreshToken and uses it for `LogOnDetails.AccessToken` — no password or Steam Guard needed
- Use `GenerateAccessTokenForAppAsync` (see `SteamKit/Samples/002_WebCookie`) to renew tokens before they expire during long sessions
- Set `IsPersistentSession = true` and `ShouldRememberPassword = true` in `AuthSessionDetails`

#### Fallback: localStorage credentials
- If RefreshToken is missing or expired beyond renewal, fall back to username + password stored in frontend localStorage
- "Remember me" checkbox in LoginView controls whether credentials persist in localStorage
- Frontend reads localStorage on mount → if credentials exist, sends them to the sidecar `login` command for re-authentication

#### Login flow:
1. App starts → sidecar starts → sidecar checks for saved RefreshToken
2. If valid RefreshToken exists → auto-login silently, frontend skips login screen
3. If no token → frontend shows login form
4. User enters username + password (+ optional Steam Guard code)
5. Frontend sends `login` IPC command to sidecar
6. Sidecar authenticates, stores RefreshToken + GuardData to disk
7. If "remember me" checked, frontend also saves username + password to localStorage as fallback
8. All subsequent sidecar commands use the authenticated session

#### Sidecar commands:
The sidecar daemon's `login` command is called once after app start. All subsequent commands (list_depots, list_manifests, etc.) use the already-authenticated session — no credentials needed per-command. The sidecar must be logged in before any other command is accepted.

**What to remove:**
- `keyring` crate dependency from Cargo.toml
- `save_to_keychain`, `load_from_keychain`, `delete_from_keychain` functions
- `save_username`, `load_username`, `clear_saved_username` (plaintext file on disk)
- `AuthStore` struct entirely (or simplify to just track "is sidecar logged in" boolean)
- `resume_session` IPC command
- `has_credentials` IPC command
- `get_or_saved()` method
- The "Welcome back" flow in LoginView (replace with auto-login via RefreshToken)

**What to add/change:**
- **Sidecar:** Implement `IsPersistentSession`/`ShouldRememberPassword` in SteamSession.cs login flow
- **Sidecar:** Store RefreshToken + GuardData as JSON file per account in config dir
- **Sidecar:** Add `check-session` command that checks if a valid RefreshToken exists and attempts silent login
- **Sidecar:** On login success, return `{ logged_in: true, username: "..." }` so frontend knows auth state
- **Frontend:** localStorage read on mount → if RefreshToken login fails and localStorage has credentials, auto-retry with password
- **Frontend:** "Remember me" checkbox in LoginView
- **Frontend:** Ensure sidecar `login` (or `check-session`) is called before navigating past auth gate
- **IPC surface:** Simplify to `login`, `check_session`, `logout`, `get_auth_state`

#### SteamKit reference (from Samples/000_Authentication):
```csharp
var authSession = await client.Authentication.BeginAuthSessionViaCredentialsAsync(new AuthSessionDetails {
    Username = username,
    Password = password,
    IsPersistentSession = true,
    GuardData = previouslyStoredGuardData, // skip Steam Guard if available
    Authenticator = new UserConsoleAuthenticator(), // or custom for phone approval
});
// After polling:
authSession.RefreshToken  // store this — JWT, use for future logins
authSession.NewGuardData  // store this — skips Steam Guard next time
```
