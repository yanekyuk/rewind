---
trigger: "Session expired error on list_manifests despite sidecar having a valid saved session token"
type: fix
branch: fix/session-reuse
base-branch: main
created: 2026-03-31
---

## Related Files
- src-tauri/src/lib.rs (list_manifests command blocks on missing credentials at line 114)
- src-tauri/src/application/auth.rs (AuthStore — only username persisted, credentials in-memory only)
- src-tauri/src/infrastructure/depot_downloader.rs (all sidecar calls require full Credentials with password)
- src-tauri/src/domain/auth.rs (Credentials struct)
- sidecar/SteamKitSidecar/Program.cs (--password is GetRequired in all commands)
- sidecar/SteamKitSidecar/SteamSession.cs (ConnectAndLoginAsync tries saved session before credentials)
- src/hooks/useAuth.ts (frontend auth hook — checks get_auth_state on mount)

## Relevant Docs
- docs/specs/auth-ui.md (auth flow spec, notes sidecar session persistence)
- docs/domain/steamkit-sidecar.md (sidecar architecture)

## Related Issues
None — no related issues found.

## Scope
The sidecar persists session tokens to `~/.local/share/rewind/sessions/<username>.json` and can authenticate using just a saved refresh token (no password needed). However, the Rust backend requires full credentials (username + password) in the in-memory `AuthStore` before attempting any sidecar call. On app restart, only the username is restored from disk — so `list_manifests` immediately returns `AuthRequired("Session expired")` without ever giving the sidecar a chance to use its saved session.

### Fix plan:
1. **Sidecar (C#)**: Make `--password` optional in all commands. If omitted and a saved session exists, use it. If the saved session is expired or missing, emit a structured `AUTH_REQUIRED` error code so the Rust backend can distinguish it from other failures.
2. **Rust backend**: Add a `get_or_saved()` method to `AuthStore` that returns `Credentials` with an empty password when only a saved username exists. Update `list_manifests` (and other sidecar-calling commands) to use this — attempt the call, and if the sidecar returns `AUTH_REQUIRED`, surface `RewindError::AuthRequired` to the frontend.
3. **Frontend**: Already handles `AuthRequired` by showing the login form — no changes needed.
