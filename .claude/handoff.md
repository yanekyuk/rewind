---
trigger: "Replace manual manifest ID input with a version picker. Fetch available manifests for the selected game's depots using DepotDownloader and present them in the UI so users can pick a target version instead of copy-pasting from SteamDB."
type: feat
branch: feat/manifest-select
base-branch: main
created: 2026-03-30
version-bump: patch
---

## Related Files
- src/steps.ts (step 1 "enter-manifest" — may need label/description update)
- src/components/StepView.tsx (placeholder for step 1 — will be replaced with manifest selector)
- src/App.tsx (needs to pass selected game info to step 1, hold selected manifest state)
- src-tauri/src/lib.rs (needs new IPC command for listing manifests)
- src-tauri/src/infrastructure/sidecar.rs (spawn_depot_downloader helper)
- src-tauri/src/domain/manifest/mod.rs (DepotManifest types)

## Relevant Docs
- docs/decisions/manual-manifest-input.md (current decision: manual input for MVP; mentions SteamKit2 manifest listing as post-MVP improvement)
- docs/domain/depotdownloader.md (DepotDownloader CLI flags, -manifest-only output format)
- docs/specs/mvp-scope.md (MVP scope — manual manifest input; version discovery section)
- docs/domain/downgrade-process.md (full downgrade workflow)
- docs/domain/steam-internals.md (depots, manifests, ACF format)

## Related Issues
None — no related issues found.

## Scope
Rework the "Enter Manifest ID" step into a "Select Version" step. Instead of requiring users to manually find and paste manifest IDs from SteamDB, use DepotDownloader to fetch available manifests for the selected game's depots and present them in a selectable list.

### What to do

1. **Backend: Add `list_manifests` IPC command**
   - Takes an app ID and depot ID (from the selected game)
   - Spawns DepotDownloader with appropriate flags to list available manifests
   - Note: This requires Steam authentication — the user will need to provide credentials. Consider whether auth should be a separate step or integrated into this one.
   - Parses the output and returns a list of manifest entries (manifest ID, date, size)
   - Returns the list to the frontend

2. **Frontend: Update step definition**
   - Update step 1 in steps.ts: change label to "Select Version" (or similar), update description
   - The step ID can remain "enter-manifest" or be changed to "select-version"

3. **Frontend: Add ManifestSelect component**
   - Replaces the placeholder StepView for step 1
   - Receives the selected game from App state
   - Calls `list_manifests` IPC with the game's depot info
   - Shows loading/error/empty states (similar pattern to GameSelect)
   - Renders manifest entries with date, manifest ID, and size
   - User selects a target manifest
   - Also allow manual manifest ID input as a fallback

4. **Frontend: Wire into App.tsx**
   - Pass selected game to ManifestSelect
   - Hold selected manifest state
   - Next button disabled until manifest is selected

### Constraints
- Authentication is required for manifest listing — this may need a preceding auth step or inline auth UI
- DepotDownloader must be available (sidecar configured in previous PR)
- Keep manual input as fallback for users who already know the manifest ID
- Must handle games with multiple depots (show depot selector or list all)
- The decision doc (manual-manifest-input.md) should be updated to reflect this change
