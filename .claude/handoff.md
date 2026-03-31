---
trigger: "Build the frontend downgrade UI — embedded SteamDB webview for version discovery, progress view for the downgrade pipeline, and completion state."
type: feat
branch: feat/downgrade-ui
base-branch: main
created: 2026-03-31
---

## Related Files
- src/App.tsx — main navigation, view routing
- src/types/navigation.ts — ViewId type (needs "downgrade" added)
- src/components/VersionSelect.tsx — current version selection (will be reworked or replaced)
- src-tauri/src/lib.rs — start_downgrade IPC command (already exists)
- src-tauri/src/application/downgrade.rs — downgrade orchestration (already exists)
- src-tauri/src/domain/downgrade.rs — DowngradeParams, DowngradeProgress types
- src/components/GameDetail.tsx — "Change Version" button entry point
- src/App.css — styling

## Relevant Docs
- docs/specs/downgrade-pipeline.md — backend pipeline phases and progress events
- docs/specs/steam-ui-overhaul.md — navigation model, Steam theming, view structure
- docs/specs/mvp-scope.md — MVP feature set and version discovery approach
- docs/decisions/progress-ui.md — embedded progress UI design (progress bar, cancel, ETA, notifications)
- docs/domain/downgrade-process.md — the 9-step manual downgrade process being automated

## Related Issues
None — no related issues found.

## Scope

### 1. SteamDB Webview for Version Discovery
- Open SteamDB's depot manifest page (`https://steamdb.info/depot/<depotId>/manifests/`) in a Tauri webview
- User browses SteamDB naturally (including login if needed for older manifests)
- Inject JavaScript into the webview to extract the manifest history table from the DOM
- Parse extracted data: manifest ID, date, branch/version labels
- Present the extracted versions in the app's native UI for selection

### 2. Downgrade Progress View
- After user selects a target manifest, call `start_downgrade` IPC command
- Listen to `downgrade-progress` Tauri events on the frontend
- Display progress for each phase:
  - **Comparing**: spinner/indeterminate progress while manifests are fetched and diffed
  - **Downloading**: progress bar with percent, bytes downloaded/total, speed, ETA
  - **Applying**: spinner while files are copied, ACF patched, and manifest locked
- Cancel button to abort the download
- Error state with message and retry option

### 3. Completion State
- Success message after downgrade completes
- Reminder to set Steam update preference to "Only update when I launch"
- Option to return to game detail

### Key Backend Types (already exist)
- `DowngradeProgress` enum: `Comparing`, `Downloading { percent, bytes_downloaded, bytes_total }`, `Applying`, `Complete`, `Error { message }`
- `DowngradeParams`: `app_id`, `depot_id`, `target_manifest_id`, `current_manifest_id`, `latest_buildid`, `latest_manifest_id`, `latest_size`, `install_path`, `steamapps_path`
- `start_downgrade` IPC command in lib.rs
