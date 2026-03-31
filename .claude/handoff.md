---
trigger: "Store encrypted passwords locally so users don't have to re-enter them on app restart, decrypt for sidecar calls"
type: feat
branch: feat/encrypted-credentials
base-branch: main
created: 2026-03-31
---

## Related Files
- src-tauri/src/application/auth.rs (AuthStore, save_username, load_username)
- src-tauri/src/domain/auth.rs (Credentials type)
- src-tauri/src/infrastructure/depot_downloader.rs (build_credential_args — uses Credentials)
- src-tauri/src/lib.rs (set_credentials, get_auth_state, clear_credentials commands)
- src/hooks/useAuth.ts (frontend auth flow)
- src/components/LoginView.tsx (login UI)
- src-tauri/Cargo.toml (dependencies)

## Relevant Docs
- docs/specs/auth-ui.md

## Related Issues
None — no related issues found.

## Scope
Currently only the username is persisted to disk. The password is only in-memory, so after app restart the user must re-enter it (or rely on the sidecar's session token which can expire). This feature adds encrypted local password storage.

Implementation approach:
- Use a platform keychain/keyring crate (e.g., `keyring` crate) for secure credential storage, OR
- Use OS-level encryption (DPAPI on Windows, Keychain on macOS, libsecret/kwallet on Linux) via the `keyring` crate
- Encrypt and store the password alongside the username on successful login
- On app startup, attempt to load and decrypt stored credentials into AuthStore
- If decryption succeeds, populate AuthStore with full credentials (username + password)
- This means `get_or_saved()` returns full credentials instead of empty-password fallback
- On sign-out (`clear_credentials`), remove stored password from keychain
- Frontend LoginView should show "Welcome back, <username>" when stored credentials exist, with option to sign in as different user

Key constraints:
- Must work cross-platform (Linux, macOS, Windows)
- Password must never be stored in plaintext on disk
- Graceful fallback if keychain is unavailable (degrade to current username-only behavior)
- The `keyring` crate (https://crates.io/crates/keyring) is the standard Rust solution for this
