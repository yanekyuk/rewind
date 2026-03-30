---
trigger: "Authentication UI and IPC — add credential input (username, password, Steam Guard 2FA code) to the frontend and a Tauri IPC command that passes credentials to DepotDownloader. Credentials must never be persisted by Rewind. DepotDownloader's -remember-password flag handles session caching."
type: feat
branch: feat/auth-ui
base-branch: main
created: 2026-03-30
---

## Related Files
- src-tauri/src/infrastructure/sidecar.rs — spawn_depot_downloader() helper, will need auth args passed through
- src-tauri/src/lib.rs — Tauri IPC command registration
- src/App.tsx — main app component with step navigation
- src/steps.ts — step definitions (auth needs to integrate before download steps)
- src/components/GameSelect.tsx — reference for component patterns (loading/error states)
- src/hooks/useGameList.ts — reference for IPC hook patterns

## Relevant Docs
- docs/domain/depotdownloader.md — DepotDownloader CLI interface, auth flags (-username, -remember-password)
- docs/domain/downgrade-process.md — steps 5-6 require authentication
- docs/specs/mvp-scope.md — "Authentication Flow" section defines the 5-step auth flow
- docs/decisions/depotdownloader-sidecar.md — sidecar integration approach

## Related Issues
None — no related issues found.

## Scope

### Backend (Rust)
- Add an `authenticate` or `set_credentials` Tauri IPC command that accepts username, password, and optional Steam Guard code
- Store credentials in-memory only (app state managed by Tauri) for the duration of the session — never persist to disk
- When spawning DepotDownloader, inject `-username`, `-password`, and `-remember-password` args from stored credentials
- If Steam Guard 2FA is needed, DepotDownloader will prompt via stdout — parse this and relay to frontend
- Handle auth errors (invalid credentials, expired 2FA) and return typed error responses

### Frontend (React/TypeScript)
- Add an `AuthInput` component with fields for username, password, and Steam Guard code
- Steam Guard code field should be conditionally shown (only when 2FA is required)
- Add a `useAuth` hook that calls the `set_credentials` IPC command
- Wire auth into the app flow — auth should be prompted before any DepotDownloader operation that requires it (manifest fetch, download)
- Credentials must not be logged or displayed after entry (password field masking)
- Show clear error messages for auth failures

### Integration
- Auth state should gate the "Comparing Versions" step — user cannot proceed to manifest diff without valid credentials
- DepotDownloader's `-remember-password` flag caches the session, so subsequent operations in the same session should not re-prompt
- If session cache exists from a previous run, detect this and skip auth prompt
