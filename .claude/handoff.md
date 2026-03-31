---
trigger: "Password persistence is broken — app asks for password and Steam Guard on every launch despite encrypted credentials feature. Keychain save/load not working correctly on Linux."
type: fix
branch: fix/credential-persistence
base-branch: main
created: 2026-03-31
---

## Related Files
- src-tauri/src/application/auth.rs — AuthStore, keychain helpers (save/load/delete)
- src-tauri/src/lib.rs — set_credentials IPC handler, app startup (run())
- src/hooks/useAuth.ts — frontend auth hook, submit flow
- src/components/LoginView.tsx — "Welcome back" UI, handleResumeSession
- docs/specs/credential-storage.md — spec for the feature

## Relevant Docs
- docs/specs/credential-storage.md — encrypted credential storage spec
- docs/specs/auth-ui.md — auth UI spec

## Related Issues
None — no related issues found.

## Scope

### Bug
The "Welcome back" flow is broken. When the user restarts the app:

1. App startup loads username from disk + password from OS keychain → `AuthStore` has full credentials
2. Frontend sees `has_credentials=true`, shows "Welcome back" UI
3. User clicks "Sign in" → `handleResumeSession` calls `submit(username, "", undefined)`
4. This calls `set_credentials` IPC with **empty password**
5. `set_credentials` calls `depot_downloader::login()` with empty password
6. If sidecar session token expired, login fails → user must re-enter password + Steam Guard

### Root Cause
The "Welcome back" submit passes empty password to signal "reuse session", but if the session expired, there's no fallback to the keychain-stored password. The backend already has the real password in `AuthStore` (loaded at startup), but `set_credentials` overwrites it with the empty-password credentials from the frontend.

### Fix Strategy
Two approaches (implementer should choose the cleanest):

**Option A — Backend: new `resume_session` IPC command**
- Add a `resume_session` command that uses the credentials already in `AuthStore` (loaded from keychain at startup) to call `depot_downloader::login()`
- No password crosses the IPC boundary — backend uses what it already has
- Frontend "Welcome back" calls `resume_session` instead of `set_credentials`

**Option B — Backend: fix `set_credentials` to use keychain fallback**
- When `set_credentials` is called with an empty password, check if `AuthStore` already has a stored password (from keychain load at startup)
- If so, use that password for the sidecar login call instead of the empty one
- Preserve the existing save_to_keychain guard (`if !password.is_empty()`)

Either way, the fix must ensure that when the sidecar session expires between launches, the app can re-authenticate using the keychain-stored password without requiring user re-entry.
